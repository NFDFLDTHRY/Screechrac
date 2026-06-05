//! SPATIAL ROLE: THE ORCHESTRATOR — the ONLY module that talks to the FSL cognitive
//! engine. Every posted job is a CABLE ingested into the in-memory FSL `World`; missing
//! variables are UNK STRANDS; clarifications resolve them across ticks via the Onion
//! Shell; the Coffee Cup arc advances each pass. The durable SQLite store is the source
//! of truth; the `World` is a projection rebuilt by `replay_world`. All intelligence is
//! FSL's — this module only assembles claims and reads results.

use serde::Deserialize;
use serde_json::json;

use fsl_core::{Claim, Span, TopicId};
use fsl_llm::facilitator::{self, Preset};

use crate::auth::now;
use crate::payments::Charge;
use crate::store::JobRow;
use crate::AppState;

// ─────────────────────────── helpers ───────────────────────────
pub fn preset_by_key(k: &str) -> Preset {
    match k {
        "delivery" => Preset::Delivery,
        "roadside" => Preset::Roadside,
        "moving" => Preset::Moving,
        "errand" => Preset::Errand,
        "labor" => Preset::Labor,
        _ => Preset::FreeForm,
    }
}

fn anchored_claim(st: &AppState, slot: &str, value: &str) -> Claim {
    Claim::pointable(
        st.next_claim(),
        &format!("{slot}: {value}"),
        Span { quote: value.to_string(), context: "job".into() },
        facilitator::slot_topic(slot),
        1,
    )
}
fn missing_claim(st: &AppState, slot: &str) -> Claim {
    let mut c = Claim::plain(st.next_claim(), &format!("missing {slot}"));
    c.topic = Some(TopicId(facilitator::slot_topic(slot)));
    c
}

/// Ingest a job's text as a cable and run the full FSL pass. Returns (cable, strands, stage, coherent).
fn world_ingest(st: &AppState, text: &str, present: &[(String, String)], missing: &[String]) -> (i64, i64, String, bool) {
    let mut claims: Vec<Claim> = vec![];
    for (slot, val) in present {
        claims.push(anchored_claim(st, slot, val));
    }
    for slot in missing {
        claims.push(missing_claim(st, slot));
    }
    if claims.is_empty() {
        claims.push(Claim::plain(st.next_claim(), text));
    }
    let mut w = st.world.lock().unwrap();
    let root = w.ingest_input(text);
    let tr = w.primary_loop(&st.oracle, root, claims, false);
    let coherent = w.coherence().2;
    let strands = (tr.obs + tr.unks_raised + tr.deltas + tr.invalids + tr.converted) as i64;
    (root.0 as i64, strands, format!("{:?}", tr.cup_stage), coherent)
}

/// Feed a clarification answer back and run the Onion Shell. Returns (stage, coherent).
fn world_answer(st: &AppState, slot: &str, answer: &str) -> (String, bool) {
    let claim = anchored_claim(st, slot, answer);
    let mut w = st.world.lock().unwrap();
    let r = w.ingest_input(answer);
    let tr = w.primary_loop(&st.oracle, r, vec![claim], false);
    w.detect_and_trigger();
    let _ = w.onion_resolve(&st.oracle, 16);
    (format!("{:?}", tr.cup_stage), w.coherence().2)
}

/// Data-driven standardization: count a `ready` job of `kind`; promote to a one-tap preset
/// once the threshold of fully-specified jobs is reached.
fn record_ready(st: &AppState, kind: &str) {
    if let Ok(count) = st.store.bump_preset_stat(kind) {
        if count >= st.cfg.promote_threshold {
            let _ = st.store.ensure_preset(kind, now());
        }
    }
}

fn slot_questions(missing: &[facilitator::Clarify]) -> serde_json::Value {
    json!(missing.iter().map(|c| json!({"slot": c.slot, "ask": c.ask})).collect::<Vec<_>>())
}
fn filled_json(present: &[(String, String)]) -> serde_json::Value {
    json!(present.iter().map(|(s, v)| json!([s, v])).collect::<Vec<_>>())
}

// ─────────────────────────── public operations ───────────────────────────
/// Post a free-form request → structured job through FSL.
pub fn create_job(st: &AppState, customer_id: i64, text: &str) -> Result<JobRow, String> {
    let prop = facilitator::structure_job(&st.oracle, text);
    let present: Vec<(String, String)> = prop.present.iter().map(|(s, v)| (s.clone(), v.clone())).collect();
    let missing_slots: Vec<String> = prop.missing.iter().map(|c| c.slot.clone()).collect();
    let (cable, strands, stage, coherent) = world_ingest(st, text, &present, &missing_slots);

    let ready = prop.missing.is_empty();
    let kind = prop.preset.key().to_string();
    let display = if ready && prop.preset != Preset::FreeForm { prop.preset.label().to_string() } else { "free-form".to_string() };
    let mut row = JobRow {
        id: 0,
        customer_id,
        text: text.to_string(),
        title: prop.title.clone(),
        preset: display,
        preset_kind: kind.clone(),
        price_band: prop.price_band.clone(),
        status: if ready { "ready".into() } else { "needs_info".into() },
        open_questions: slot_questions(&prop.missing),
        filled: filled_json(&present),
        cable,
        strands,
        stage,
        coherent,
        accepted_by: None,
        created: now(),
    };
    row.id = st.store.insert_job(&row)?;
    if ready {
        record_ready(st, &kind);
    }
    Ok(row)
}

/// One-tap creation from a promoted preset: all variables supplied → immediately ready.
pub fn create_preset_job(st: &AppState, customer_id: i64, kind: &str, values: Vec<(String, String)>) -> Result<JobRow, String> {
    let preset = preset_by_key(kind);
    let required: Vec<&str> = preset.required_slots().to_vec();
    let present: Vec<(String, String)> = required
        .iter()
        .filter_map(|s| values.iter().find(|(k, _)| k == s).map(|(k, v)| (k.clone(), v.clone())))
        .collect();
    let missing: Vec<String> = required.iter().filter(|s| !present.iter().any(|(k, _)| k == *s)).map(|s| s.to_string()).collect();

    let text = format!(
        "[{}] {}",
        preset.label(),
        present.iter().map(|(s, v)| format!("{s}: {v}")).collect::<Vec<_>>().join("; ")
    );
    let (cable, strands, stage, coherent) = world_ingest(st, &text, &present, &missing);
    let ready = missing.is_empty();
    let missing_clar: Vec<facilitator::Clarify> = missing
        .iter()
        .map(|s| facilitator::Clarify { slot: s.clone(), ask: facilitator::ask_for(s).to_string(), would_become_obs: String::new() })
        .collect();
    let mut row = JobRow {
        id: 0,
        customer_id,
        text,
        title: format!("{} (one-tap)", preset.label()),
        preset: if ready { preset.label().to_string() } else { "free-form".into() },
        preset_kind: kind.to_string(),
        price_band: preset.price_band().to_string(),
        status: if ready { "ready".into() } else { "needs_info".into() },
        open_questions: slot_questions(&missing_clar),
        filled: filled_json(&present),
        cable,
        strands,
        stage,
        coherent,
        accepted_by: None,
        created: now(),
    };
    row.id = st.store.insert_job(&row)?;
    // persist the supplied values as clarifications so a restart replays identically.
    for (s, v) in &present {
        let _ = st.store.add_clarification(row.id, s, v);
    }
    if ready {
        record_ready(st, kind);
    }
    Ok(row)
}

/// Answer a clarifying question (The Listener) → resolve the awaiting UNK via the Onion Shell.
pub fn clarify(st: &AppState, job_id: i64, slot: &str, answer: &str) -> Result<Option<JobRow>, String> {
    let Some(mut row) = st.store.job(job_id)? else { return Ok(None) };
    let (stage, coherent) = world_answer(st, slot, answer);
    st.store.add_clarification(job_id, slot, answer)?;

    // update display snapshot
    let mut filled: Vec<(String, String)> = serde_json::from_value(row.filled.clone()).unwrap_or_default();
    filled.push((slot.to_string(), answer.to_string()));
    let mut questions: Vec<serde_json::Value> = serde_json::from_value(row.open_questions.clone()).unwrap_or_default();
    questions.retain(|q| q.get("slot").and_then(|s| s.as_str()) != Some(slot));

    row.filled = json!(filled.iter().map(|(s, v)| json!([s, v])).collect::<Vec<_>>());
    row.open_questions = json!(questions);
    row.stage = stage;
    row.coherent = coherent;
    row.strands += 1;
    if questions.is_empty() && row.status == "needs_info" {
        row.status = "ready".into();
        if row.preset == "free-form" && row.preset_kind != "free-form" {
            row.preset = preset_by_key(&row.preset_kind).label().to_string();
        }
        record_ready(st, &row.preset_kind);
    }
    st.store.update_job_snapshot(&row)?;
    Ok(Some(row))
}

pub fn accept(st: &AppState, job_id: i64, driver_username: &str) -> Result<Option<JobRow>, String> {
    let Some(mut row) = st.store.job(job_id)? else { return Ok(None) };
    if row.status != "ready" {
        return Err("job is not in a ready state".into());
    }
    row.status = "accepted".into();
    row.accepted_by = Some(driver_username.to_string());
    st.store.update_job_snapshot(&row)?;
    Ok(Some(row))
}

/// Complete the job and route it through the payments seam (inert in this PR).
pub fn complete(st: &AppState, job_id: i64) -> Result<Option<(JobRow, String)>, String> {
    let Some(mut row) = st.store.job(job_id)? else { return Ok(None) };
    row.status = "completed".into();
    st.store.update_job_snapshot(&row)?;
    let driver_id = row
        .accepted_by
        .as_ref()
        .and_then(|u| st.store.account_by_username(u).ok().flatten())
        .map(|(a, _)| a.id)
        .unwrap_or(0);
    let charge = Charge {
        job_id: row.id,
        customer_id: row.customer_id,
        driver_id,
        amount_cents: 0, // a real provider derives this from an agreed quote
        memo: format!("{} ({})", row.title, row.price_band),
    };
    let settlement = st.payments.settle(&charge).to_string();
    Ok(Some((row, settlement)))
}

/// Customer Mode escalation routed THROUGH the FSL cognitive system.
pub fn escalate(st: &AppState, job_id: i64) -> serde_json::Value {
    let claim = anchored_claim(st, "escalation", "connect me to a person");
    let (stage, bridge, behavior) = {
        let mut w = st.world.lock().unwrap();
        let r = w.ingest_input("Customer Mode escalation");
        let tr = w.primary_loop(&st.oracle, r, vec![claim], false);
        (format!("{:?}", tr.cup_stage), format!("{:?}", tr.bridge), tr.behavior.unwrap_or_else(|| "facilitating".into()))
    };
    json!({
        "job_id": job_id,
        "recorded": true,
        "stage": stage,
        "bridge": bridge,
        "behavior": behavior,
        "note": "Live voice handed to a human representative — the FSL cognitive system is facilitating and recording the interaction."
    })
}

pub fn list(st: &AppState, limit: i64, offset: i64) -> Result<Vec<JobRow>, String> {
    st.store.list_jobs(limit.clamp(1, 200), offset.max(0))
}

#[derive(serde::Serialize)]
pub struct PresetView {
    pub kind: String,
    pub label: String,
    pub price_band: String,
    pub slots: Vec<serde_json::Value>,
}
pub fn presets(st: &AppState) -> Result<Vec<PresetView>, String> {
    let kinds = st.store.promoted_presets()?;
    Ok(kinds
        .into_iter()
        .map(|k| {
            let p = preset_by_key(&k);
            PresetView {
                kind: k,
                label: p.label().to_string(),
                price_band: p.price_band().to_string(),
                slots: p
                    .required_slots()
                    .iter()
                    .map(|s| json!({"slot": s, "ask": facilitator::ask_for(s)}))
                    .collect(),
            }
        })
        .collect())
}

/// Deep, FSL-derived shadow intelligence.
pub fn shadow(st: &AppState) -> Result<serde_json::Value, String> {
    let jobs = st.store.list_jobs(100, 0)?;
    let now_s = now();
    let mut stalled = vec![];
    let mut needs = 0;
    let mut ready = 0;
    for j in &jobs {
        match j.status.as_str() {
            "needs_info" => {
                needs += 1;
                let open = j.open_questions.as_array().map(|a| a.len()).unwrap_or(0);
                if now_s - j.created > st.cfg.stall_secs {
                    stalled.push(json!({"id": j.id, "stage": j.stage, "open": open, "age_secs": now_s - j.created}));
                }
            }
            "ready" => ready += 1,
            _ => {}
        }
    }
    let (cables, strands, flows, open_unks, resolved_unks, coherent) = {
        let w = st.world.lock().unwrap();
        let s = w.project_scene();
        let (c, st_, f) = s.stats();
        let (res, open, coh) = w.coherence();
        (c, st_, f, open, res, coh)
    };
    let mut hints = vec![];
    if needs > 0 {
        hints.push(format!("{needs} job(s) awaiting clarification — open UNKs in the FSL Onion Shell."));
    }
    if !stalled.is_empty() {
        hints.push(format!("{} job(s) stalled mid-clarification (>{}s) — nudge the customer.", stalled.len(), st.cfg.stall_secs));
    }
    if ready > 0 {
        hints.push(format!("{ready} job(s) ready to accept right now."));
    }
    hints.push(format!("FSL graph: {cables} cables (jobs), {strands} strands, {flows} flows; coherent={coherent}."));
    if resolved_unks > 0 {
        hints.push(format!("{resolved_unks} clarification(s) resolved across ticks (Onion Shell)."));
    }
    Ok(json!({
        "hints": hints,
        "stalled": stalled,
        "fsl": {"cables": cables, "strands": strands, "flows": flows, "open_unks": open_unks, "resolved_unks": resolved_unks, "coherent": coherent}
    }))
}

/// Rebuild the in-memory FSL `World` from the durable store on boot — deterministic replay.
pub fn replay_world(st: &AppState) -> Result<usize, String> {
    let jobs = st.store.jobs_for_replay(st.cfg.replay_cap)?;
    let n = jobs.len();
    for job in jobs {
        let prop = facilitator::structure_job(&st.oracle, &job.text);
        let present: Vec<(String, String)> = prop.present.iter().map(|(s, v)| (s.clone(), v.clone())).collect();
        let missing: Vec<String> = prop.missing.iter().map(|c| c.slot.clone()).collect();
        let _ = world_ingest(st, &job.text, &present, &missing);
        for (slot, answer) in job.clarifications {
            let _ = world_answer(st, &slot, &answer);
        }
    }
    Ok(n)
}

// request bodies shared with handlers
#[derive(Deserialize)]
pub struct NewJob {
    pub text: String,
}
#[derive(Deserialize)]
pub struct PresetJob {
    pub kind: String,
    pub values: Vec<(String, String)>,
}
#[derive(Deserialize)]
pub struct Clarification {
    pub job_id: i64,
    pub slot: String,
    pub answer: String,
}
#[derive(Deserialize)]
pub struct JobRef {
    pub job_id: i64,
}
