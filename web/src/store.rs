//! SPATIAL ROLE: THE DURABLE LEDGER — SQLite is the application's source of truth for
//! accounts, sessions, jobs, clarifications, and preset statistics. This mirrors the FSL
//! contract at the app layer: the database is TRUTH; the in-memory FSL `World` is a
//! navigable projection rebuilt by replaying stored jobs. No cognition lives here.

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

pub type Db = Pool<SqliteConnectionManager>;
type R<T> = Result<T, String>;
fn e<E: std::fmt::Display>(x: E) -> String { x.to_string() }

#[derive(Clone)]
pub struct Store {
    pub pool: Db,
}

#[derive(Clone, Serialize)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub payout: Option<String>,
}

/// A persisted job (the display snapshot + the durable facts needed for replay).
#[derive(Clone, Serialize)]
pub struct JobRow {
    pub id: i64,
    pub customer_id: i64,
    pub text: String,
    pub title: String,
    pub preset: String,
    pub preset_kind: String,
    pub price_band: String,
    pub status: String,
    pub open_questions: serde_json::Value,
    pub filled: serde_json::Value,
    pub cable: i64,
    pub strands: i64,
    pub stage: String,
    pub coherent: bool,
    pub accepted_by: Option<String>,
    pub created: i64,
}

/// Everything needed to deterministically replay a job into a fresh FSL `World`.
pub struct ReplayJob {
    pub text: String,
    pub clarifications: Vec<(String, String)>, // (slot, answer) in order
}

impl Store {
    pub fn open(path: &str) -> R<Store> {
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::builder().max_size(8).build(manager).map_err(e)?;
        let store = Store { pool };
        store.migrate()?;
        Ok(store)
    }

    fn conn(&self) -> R<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool.get().map_err(e)
    }

    fn migrate(&self) -> R<()> {
        let c = self.conn()?;
        c.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS accounts(
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               username TEXT UNIQUE NOT NULL,
               pass_hash TEXT NOT NULL,
               role TEXT NOT NULL,
               payout TEXT,
               created INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS sessions(
               token TEXT PRIMARY KEY,
               account_id INTEGER NOT NULL,
               expires INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS jobs(
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               customer_id INTEGER NOT NULL,
               text TEXT NOT NULL,
               title TEXT NOT NULL,
               preset TEXT NOT NULL,
               preset_kind TEXT NOT NULL,
               price_band TEXT NOT NULL,
               status TEXT NOT NULL,
               open_questions TEXT NOT NULL,
               filled TEXT NOT NULL,
               cable INTEGER NOT NULL,
               strands INTEGER NOT NULL,
               stage TEXT NOT NULL,
               coherent INTEGER NOT NULL,
               accepted_by TEXT,
               created INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS job_clarifications(
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               job_id INTEGER NOT NULL,
               slot TEXT NOT NULL,
               answer TEXT NOT NULL,
               ord INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS preset_stats(
               kind TEXT PRIMARY KEY,
               ready_count INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS presets(
               kind TEXT PRIMARY KEY,
               created INTEGER NOT NULL);",
        )
        .map_err(e)
    }

    // ── accounts ──
    pub fn create_account(&self, username: &str, pass_hash: &str, role: &str, created: i64) -> R<Account> {
        let c = self.conn()?;
        c.execute(
            "INSERT INTO accounts(username, pass_hash, role, payout, created) VALUES(?1,?2,?3,NULL,?4)",
            params![username, pass_hash, role, created],
        )
        .map_err(e)?;
        Ok(Account { id: c.last_insert_rowid(), username: username.into(), role: role.into(), payout: None })
    }

    /// Returns (account, pass_hash) for login verification.
    pub fn account_by_username(&self, username: &str) -> R<Option<(Account, String)>> {
        let c = self.conn()?;
        c.query_row(
            "SELECT id, username, role, payout, pass_hash FROM accounts WHERE username=?1",
            params![username],
            |r| {
                Ok((
                    Account { id: r.get(0)?, username: r.get(1)?, role: r.get(2)?, payout: r.get(3)? },
                    r.get::<_, String>(4)?,
                ))
            },
        )
        .optional()
        .map_err(e)
    }

    pub fn account_by_id(&self, id: i64) -> R<Option<Account>> {
        let c = self.conn()?;
        c.query_row(
            "SELECT id, username, role, payout FROM accounts WHERE id=?1",
            params![id],
            |r| Ok(Account { id: r.get(0)?, username: r.get(1)?, role: r.get(2)?, payout: r.get(3)? }),
        )
        .optional()
        .map_err(e)
    }

    // ── sessions ──
    pub fn create_session(&self, token: &str, account_id: i64, expires: i64) -> R<()> {
        let c = self.conn()?;
        c.execute(
            "INSERT OR REPLACE INTO sessions(token, account_id, expires) VALUES(?1,?2,?3)",
            params![token, account_id, expires],
        )
        .map_err(e)?;
        Ok(())
    }

    /// Resolve a session token to its account, enforcing expiry.
    pub fn session_account(&self, token: &str, now: i64) -> R<Option<Account>> {
        let c = self.conn()?;
        let id: Option<i64> = c
            .query_row(
                "SELECT account_id FROM sessions WHERE token=?1 AND expires>?2",
                params![token, now],
                |r| r.get(0),
            )
            .optional()
            .map_err(e)?;
        match id {
            Some(id) => self.account_by_id(id),
            None => Ok(None),
        }
    }

    pub fn delete_session(&self, token: &str) -> R<()> {
        let c = self.conn()?;
        c.execute("DELETE FROM sessions WHERE token=?1", params![token]).map_err(e)?;
        Ok(())
    }

    // ── jobs ──
    #[allow(clippy::too_many_arguments)]
    pub fn insert_job(&self, j: &JobRow) -> R<i64> {
        let c = self.conn()?;
        c.execute(
            "INSERT INTO jobs(customer_id,text,title,preset,preset_kind,price_band,status,open_questions,filled,cable,strands,stage,coherent,accepted_by,created)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                j.customer_id, j.text, j.title, j.preset, j.preset_kind, j.price_band, j.status,
                j.open_questions.to_string(), j.filled.to_string(), j.cable, j.strands, j.stage,
                j.coherent as i64, j.accepted_by, j.created
            ],
        )
        .map_err(e)?;
        Ok(c.last_insert_rowid())
    }

    pub fn update_job_snapshot(&self, j: &JobRow) -> R<()> {
        let c = self.conn()?;
        c.execute(
            "UPDATE jobs SET preset=?2,status=?3,open_questions=?4,filled=?5,strands=?6,stage=?7,coherent=?8,accepted_by=?9 WHERE id=?1",
            params![
                j.id, j.preset, j.status, j.open_questions.to_string(), j.filled.to_string(),
                j.strands, j.stage, j.coherent as i64, j.accepted_by
            ],
        )
        .map_err(e)?;
        Ok(())
    }

    fn row_to_job(r: &rusqlite::Row) -> rusqlite::Result<JobRow> {
        let oq: String = r.get(8)?;
        let fl: String = r.get(9)?;
        Ok(JobRow {
            id: r.get(0)?,
            customer_id: r.get(1)?,
            text: r.get(2)?,
            title: r.get(3)?,
            preset: r.get(4)?,
            preset_kind: r.get(5)?,
            price_band: r.get(6)?,
            status: r.get(7)?,
            open_questions: serde_json::from_str(&oq).unwrap_or(serde_json::json!([])),
            filled: serde_json::from_str(&fl).unwrap_or(serde_json::json!([])),
            cable: r.get(10)?,
            strands: r.get(11)?,
            stage: r.get(12)?,
            coherent: r.get::<_, i64>(13)? != 0,
            accepted_by: r.get(14)?,
            created: r.get(15)?,
        })
    }

    const JOB_COLS: &'static str =
        "id,customer_id,text,title,preset,preset_kind,price_band,status,open_questions,filled,cable,strands,stage,coherent,accepted_by,created";

    pub fn job(&self, id: i64) -> R<Option<JobRow>> {
        let c = self.conn()?;
        let sql = format!("SELECT {} FROM jobs WHERE id=?1", Self::JOB_COLS);
        c.query_row(&sql, params![id], Self::row_to_job).optional().map_err(e)
    }

    pub fn list_jobs(&self, limit: i64, offset: i64) -> R<Vec<JobRow>> {
        let c = self.conn()?;
        let sql = format!("SELECT {} FROM jobs ORDER BY id DESC LIMIT ?1 OFFSET ?2", Self::JOB_COLS);
        let mut stmt = c.prepare(&sql).map_err(e)?;
        let rows = stmt.query_map(params![limit, offset], Self::row_to_job).map_err(e)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(e)
    }

    pub fn count_jobs(&self) -> R<i64> {
        let c = self.conn()?;
        c.query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0)).map_err(e)
    }

    pub fn add_clarification(&self, job_id: i64, slot: &str, answer: &str) -> R<()> {
        let c = self.conn()?;
        let ord: i64 = c
            .query_row("SELECT COALESCE(MAX(ord),0)+1 FROM job_clarifications WHERE job_id=?1", params![job_id], |r| r.get(0))
            .map_err(e)?;
        c.execute(
            "INSERT INTO job_clarifications(job_id,slot,answer,ord) VALUES(?1,?2,?3,?4)",
            params![job_id, slot, answer, ord],
        )
        .map_err(e)?;
        Ok(())
    }

    /// All jobs (oldest first) with ordered clarifications — the deterministic replay set.
    pub fn jobs_for_replay(&self, cap: i64) -> R<Vec<ReplayJob>> {
        let c = self.conn()?;
        let mut stmt = c.prepare("SELECT id,text FROM jobs ORDER BY id ASC LIMIT ?1").map_err(e)?;
        let base: Vec<(i64, String)> = stmt
            .query_map(params![cap], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(e)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(e)?;
        let mut out = vec![];
        for (id, text) in base {
            let mut cs = c.prepare("SELECT slot,answer FROM job_clarifications WHERE job_id=?1 ORDER BY ord ASC").map_err(e)?;
            let clar: Vec<(String, String)> = cs
                .query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(e)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(e)?;
            out.push(ReplayJob { text, clarifications: clar });
        }
        Ok(out)
    }

    // ── preset evolution (data-driven standardization) ──
    /// Record that a job of `kind` reached `ready`; returns the new count for that kind.
    pub fn bump_preset_stat(&self, kind: &str) -> R<i64> {
        let c = self.conn()?;
        c.execute(
            "INSERT INTO preset_stats(kind,ready_count) VALUES(?1,1)
             ON CONFLICT(kind) DO UPDATE SET ready_count=ready_count+1",
            params![kind],
        )
        .map_err(e)?;
        c.query_row("SELECT ready_count FROM preset_stats WHERE kind=?1", params![kind], |r| r.get(0)).map_err(e)
    }

    pub fn ensure_preset(&self, kind: &str, created: i64) -> R<()> {
        let c = self.conn()?;
        c.execute(
            "INSERT OR IGNORE INTO presets(kind,created) VALUES(?1,?2)",
            params![kind, created],
        )
        .map_err(e)?;
        Ok(())
    }

    pub fn promoted_presets(&self) -> R<Vec<String>> {
        let c = self.conn()?;
        let mut stmt = c.prepare("SELECT kind FROM presets ORDER BY created ASC").map_err(e)?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(e)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(e)
    }

    pub fn ready_count_for(&self, kind: &str) -> R<i64> {
        let c = self.conn()?;
        c.query_row("SELECT COALESCE(ready_count,0) FROM preset_stats WHERE kind=?1", params![kind], |r| r.get(0))
            .optional()
            .map_err(e)
            .map(|o| o.unwrap_or(0))
    }
}
