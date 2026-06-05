//! SPATIAL ROLE: THE PROOF — end-to-end integration tests that drive the Axum app
//! in-process against a temp SQLite file: auth gating, the full job lifecycle, preset
//! promotion, and restart-survival (durable truth + FSL World rebuild).

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;

const SECRET: &str = "integration-test-secret-please-keep-stable-0123456789";

fn app_on(db: &str) -> Router {
    let store = fsl_web::store::Store::open(db).unwrap();
    fsl_web::build_app(store, ".", Some(SECRET)).0
}

struct Res {
    status: StatusCode,
    cookie: Option<String>,
    body: Value,
}

async fn call(app: &Router, method: &str, uri: &str, cookie: Option<&str>, body: Option<Value>) -> Res {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        b = b.header("cookie", c);
    }
    let req = match body {
        Some(v) => b.header("content-type", "application/json").body(Body::from(v.to_string())).unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let cookie = res
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or("").to_string());
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Res { status, cookie, body }
}

async fn register(app: &Router, user: &str, role: &str) -> String {
    let r = call(app, "POST", "/api/register", None, Some(json!({"username": user, "password": "password1", "role": role}))).await;
    assert_eq!(r.status, StatusCode::OK, "register failed: {:?}", r.body);
    r.cookie.expect("register must set a session cookie")
}

#[tokio::test]
async fn auth_gating_blocks_unauthenticated_and_wrong_role() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let app = app_on(tmp.path().to_str().unwrap());

    // no cookie → 401
    let r = call(&app, "POST", "/api/job", None, Some(json!({"text": "x"}))).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);

    // a customer cannot accept jobs → 403
    let cust = register(&app, "cust_a", "customer").await;
    let r = call(&app, "POST", "/api/accept", Some(&cust), Some(json!({"job_id": 1}))).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // wrong password fails login (argon2 verify) → 401
    let r = call(&app, "POST", "/api/login", None, Some(json!({"username": "cust_a", "password": "wrong"}))).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn full_lifecycle_customer_listener_driver() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let app = app_on(tmp.path().to_str().unwrap());

    let cust = register(&app, "cust_b", "customer").await;

    // vague request → needs_info with clarifying questions (open UNKs)
    let r = call(&app, "POST", "/api/job", Some(&cust), Some(json!({"text": "I need a couch moved"}))).await;
    assert_eq!(r.status, StatusCode::OK);
    let job = &r.body;
    assert_eq!(job["status"], "needs_info");
    let id = job["id"].as_i64().unwrap();
    let questions = job["open_questions"].as_array().unwrap().clone();
    assert!(!questions.is_empty(), "vague request must raise clarifying UNKs");

    // The Listener: answer each open question → resolves UNKs across ticks (Onion Shell)
    let mut last = job.clone();
    for q in &questions {
        let slot = q["slot"].as_str().unwrap();
        let r = call(&app, "POST", "/api/clarify", Some(&cust), Some(json!({"job_id": id, "slot": slot, "answer": "5th street"}))).await;
        assert_eq!(r.status, StatusCode::OK);
        last = r.body;
    }
    assert_eq!(last["status"], "ready", "fully clarified job must be ready");
    assert_ne!(last["preset"], "free-form", "free-form must standardize into a preset");

    // driver lifecycle
    let drv = register(&app, "drv_b", "driver").await;
    let r = call(&app, "POST", "/api/accept", Some(&drv), Some(json!({"job_id": id}))).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["status"], "accepted");
    assert_eq!(r.body["accepted_by"], "drv_b", "driver identity comes from the session");

    // Customer Mode escalation routes through FSL
    let r = call(&app, "POST", "/api/escalate", Some(&drv), Some(json!({"job_id": id}))).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["recorded"], true);
    assert!(r.body["bridge"].is_string());

    // complete → payments seam returns "unconfigured" (no fake charge)
    let r = call(&app, "POST", "/api/complete", Some(&drv), Some(json!({"job_id": id}))).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["job"]["status"], "completed");
    assert_eq!(r.body["payment"], "unconfigured");
}

#[tokio::test]
async fn preset_promotion_and_one_tap() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let app = app_on(tmp.path().to_str().unwrap());
    let cust = register(&app, "cust_c", "customer").await;

    // three fully-specified delivery jobs (all slots present) → ready
    for _ in 0..3 {
        let r = call(
            &app,
            "POST",
            "/api/job",
            Some(&cust),
            Some(json!({"text": "deliver food from the diner to 200 main st today"})),
        )
        .await;
        assert_eq!(r.status, StatusCode::OK);
        assert_eq!(r.body["status"], "ready", "well-specified job should be ready: {:?}", r.body);
    }

    // delivery preset has been promoted (default threshold 3)
    let r = call(&app, "GET", "/api/presets", Some(&cust), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let kinds: Vec<String> = r.body.as_array().unwrap().iter().map(|p| p["kind"].as_str().unwrap().to_string()).collect();
    assert!(kinds.contains(&"delivery".to_string()), "delivery should be promoted, got {kinds:?}");

    // one-tap creation from the preset → immediately ready, no open questions
    let r = call(
        &app,
        "POST",
        "/api/preset_job",
        Some(&cust),
        Some(json!({"kind": "delivery", "values": [["pickup","diner"],["dropoff","office"],["item","burger"],["when","now"]]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["status"], "ready");
    assert_eq!(r.body["open_questions"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn restart_survival_and_world_rebuild() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    // session 1: create a customer + a job
    let id;
    let cookie;
    {
        let app = app_on(&path);
        cookie = register(&app, "cust_d", "customer").await;
        let r = call(&app, "POST", "/api/job", Some(&cookie), Some(json!({"text": "deliver food from the diner to 200 main st today"}))).await;
        assert_eq!(r.status, StatusCode::OK);
        id = r.body["id"].as_i64().unwrap();
    }

    // session 2: a NEW app over the SAME db file (simulated restart) + same secret
    let app2 = app_on(&path);

    // the session cookie still validates (durable sessions + stable signing key)
    let me = call(&app2, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["username"], "cust_d");

    // the job survived
    let r = call(&app2, "GET", "/api/jobs?limit=10", Some(&cookie), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let ids: Vec<i64> = r.body.as_array().unwrap().iter().map(|j| j["id"].as_i64().unwrap()).collect();
    assert!(ids.contains(&id), "job {id} must survive restart, got {ids:?}");

    // the FSL World was rebuilt by replay (cables exist again)
    let s = call(&app2, "GET", "/api/shadow", Some(&cookie), None).await;
    assert_eq!(s.status, StatusCode::OK);
    assert!(s.body["fsl"]["cables"].as_u64().unwrap() >= 1, "World must be rebuilt on boot");
}
