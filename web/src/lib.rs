//! SPATIAL ROLE: THE STOREFRONT (library) — wires the durable store, auth, the FSL
//! `World` projection, and presentation into one Axum app. `build_app` rebuilds the
//! `World` by replaying stored jobs (deterministic) so the projection survives restarts.
//! The web layer stays thin: every cognition call lives in `jobs.rs`; FSL core is unchanged.

pub mod auth;
pub mod jobs;
pub mod payments;
pub mod store;
pub mod view;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::Key;
use serde::Deserialize;
use tower_http::services::ServeDir;

use fsl_actor::Params;
use fsl_core::{InvalidPolicy, UnkPolicy};
use fsl_llm::DeterministicOracle;
use fsl_mind::World;

use auth::CurrentUser;
use payments::{NoopProvider, PaymentProvider};
use store::Store;

#[derive(Clone)]
pub struct Config {
    pub promote_threshold: i64,
    pub stall_secs: i64,
    pub replay_cap: i64,
    pub web_dir: String,
}

#[derive(Clone)]
pub struct AppState {
    pub world: Arc<Mutex<World>>,
    pub oracle: DeterministicOracle,
    pub store: Store,
    pub key: Key,
    pub payments: Arc<dyn PaymentProvider>,
    pub cfg: Config,
    claims: Arc<AtomicU64>,
}
impl AppState {
    pub fn next_claim(&self) -> u64 {
        self.claims.fetch_add(1, Ordering::Relaxed)
    }
}
impl axum::extract::FromRef<AppState> for Key {
    fn from_ref(s: &AppState) -> Key {
        s.key.clone()
    }
}

/// A fresh FSL World with two minds + a default crossing (the membrane/bridge/Coffee-Cup channel).
pub fn fresh_world() -> World {
    let policy = InvalidPolicy { stop_words: vec!["you never".into(), "obviously".into()] };
    let unk_policy = UnkPolicy::BoundedBudget { max_open_unk: 5, max_strand_depth: 3 };
    let mut world = World::new(policy, unk_policy);
    let a = world.spawn(None, Params::default(), vec![]);
    let b = world.spawn(None, Params::default(), vec![]);
    let x = world.open_pair(1, a, b, false);
    world.set_default_pair(x, a, b);
    world
}

fn env_i64(k: &str, default: i64) -> i64 {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// Build the full application (router + state). Exposed so integration tests can drive it.
///
/// On build it rebuilds the in-memory FSL `World` by replaying stored jobs (deterministic),
/// so the projection survives restarts. The returned `Router` is served with `axum::serve`.
///
/// ```no_run
/// use fsl_web::{build_app, store::Store};
/// let store = Store::open("facilitator.db").unwrap();
/// let (app, _state) = build_app(store, ".", Some("a-stable-signing-secret"));
/// // axum::serve(listener, app).await — see `main.rs`
/// let _ = app;
/// ```
pub fn build_app(store: Store, web_dir: &str, secret: Option<&str>) -> (Router, AppState) {
    let cfg = Config {
        promote_threshold: env_i64("FSL_PRESET_PROMOTE", 3),
        stall_secs: env_i64("FSL_STALL_SECS", 600),
        replay_cap: env_i64("FSL_REPLAY_CAP", 1000),
        web_dir: web_dir.to_string(),
    };
    let state = AppState {
        world: Arc::new(Mutex::new(fresh_world())),
        oracle: DeterministicOracle::default(),
        store,
        key: auth::key_from_secret(secret),
        payments: Arc::new(NoopProvider),
        cfg,
        claims: Arc::new(AtomicU64::new(1)),
    };
    // rebuild the FSL projection from durable truth.
    let _ = jobs::replay_world(&state);

    let app = Router::new()
        .route("/", get(view::index))
        .route("/login", get(view::login_page))
        .route("/customer", get(view::customer))
        .route("/driver", get(view::driver))
        .route("/admin", get(view::admin))
        .route("/manifest.webmanifest", get(view::manifest))
        .route("/sw.js", get(view::service_worker))
        .route("/api/health", get(view::health))
        .route("/api/register", post(auth::register))
        .route("/api/login", post(auth::login))
        .route("/api/logout", post(auth::logout))
        .route("/api/me", get(auth::me))
        .route("/api/job", post(post_job))
        .route("/api/preset_job", post(preset_job))
        .route("/api/clarify", post(clarify))
        .route("/api/accept", post(accept))
        .route("/api/complete", post(complete))
        .route("/api/escalate", post(escalate))
        .route("/api/jobs", get(list_jobs))
        .route("/api/presets", get(list_presets))
        .route("/api/shadow", get(shadow))
        .route("/api/walk", get(walk_guarded))
        .nest_service("/icons", ServeDir::new(format!("{web_dir}/static/icons")))
        .with_state(state.clone());
    (app, state)
}

// ─────────────────────────── role-gated API handlers ───────────────────────────
async fn post_job(State(st): State<AppState>, user: CurrentUser, Json(req): Json<jobs::NewJob>) -> Response {
    let acct = match auth::require(&user, "customer") { Ok(a) => a, Err(r) => return r };
    let text = req.text.trim();
    if text.is_empty() {
        return (StatusCode::BAD_REQUEST, "describe the problem first").into_response();
    }
    match jobs::create_job(&st, acct.id, text) {
        Ok(j) => Json(j).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn preset_job(State(st): State<AppState>, user: CurrentUser, Json(req): Json<jobs::PresetJob>) -> Response {
    let acct = match auth::require(&user, "customer") { Ok(a) => a, Err(r) => return r };
    match jobs::create_preset_job(&st, acct.id, &req.kind, req.values) {
        Ok(j) => Json(j).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn clarify(State(st): State<AppState>, user: CurrentUser, Json(req): Json<jobs::Clarification>) -> Response {
    if let Err(r) = auth::require(&user, "customer") { return r; }
    let ans = req.answer.trim();
    if ans.is_empty() {
        return (StatusCode::BAD_REQUEST, "empty answer").into_response();
    }
    match jobs::clarify(&st, req.job_id, req.slot.trim(), ans) {
        Ok(Some(j)) => Json(j).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "no such job").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn accept(State(st): State<AppState>, user: CurrentUser, Json(req): Json<jobs::JobRef>) -> Response {
    let acct = match auth::require(&user, "driver") { Ok(a) => a, Err(r) => return r };
    match jobs::accept(&st, req.job_id, &acct.username) {
        Ok(Some(j)) => Json(j).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "no such job").into_response(),
        Err(e) => (StatusCode::CONFLICT, e).into_response(),
    }
}

async fn complete(State(st): State<AppState>, user: CurrentUser, Json(req): Json<jobs::JobRef>) -> Response {
    if let Err(r) = auth::require(&user, "driver") { return r; }
    match jobs::complete(&st, req.job_id) {
        Ok(Some((job, payment))) => Json(serde_json::json!({ "job": job, "payment": payment })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "no such job").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn escalate(State(st): State<AppState>, user: CurrentUser, Json(req): Json<jobs::JobRef>) -> Response {
    if let Err(r) = auth::require(&user, "driver") { return r; }
    Json(jobs::escalate(&st, req.job_id)).into_response()
}

#[derive(Deserialize)]
struct Page {
    limit: Option<i64>,
    offset: Option<i64>,
}
async fn list_jobs(State(st): State<AppState>, user: CurrentUser, Query(p): Query<Page>) -> Response {
    if let Err(r) = auth::require_any(&user) { return r; }
    match jobs::list(&st, p.limit.unwrap_or(50), p.offset.unwrap_or(0)) {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn list_presets(State(st): State<AppState>, user: CurrentUser) -> Response {
    if let Err(r) = auth::require_any(&user) { return r; }
    match jobs::presets(&st) {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn shadow(State(st): State<AppState>, user: CurrentUser) -> Response {
    if let Err(r) = auth::require_any(&user) { return r; }
    match jobs::shadow(&st) {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn walk_guarded(user: CurrentUser) -> Response {
    if let Err(r) = auth::require(&user, "admin") { return r; }
    view::walk().await
}
