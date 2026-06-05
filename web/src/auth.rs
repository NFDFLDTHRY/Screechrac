//! SPATIAL ROLE: THE GATE — lightweight, self-contained authentication. Passwords are
//! hashed with argon2; a high-entropy session token rides a SIGNED, HTTP-only cookie and
//! is validated against the durable `sessions` table. Role identity (customer/driver/
//! admin) comes from the session, never the request body — which is exactly what the
//! later GoDaddy payments hookup needs (who is the customer, who is the driver). No
//! cognition here.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::{
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::{Cookie, Key, SameSite, SignedCookieJar};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Deserialize;

use crate::store::Account;
use crate::AppState;

const COOKIE: &str = "fsl_session";
const SESSION_SECS: i64 = 60 * 60 * 24 * 14; // 14 days

pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Derive a stable 64-byte signing key from `FSL_SECRET` (sha512). If unset, a random key
/// is generated (cookies then invalidate on restart — fine for dev/tests; setup.sh sets it).
pub fn key_from_secret(secret: Option<&str>) -> Key {
    match secret {
        Some(s) if !s.is_empty() => {
            use sha2::{Digest, Sha512};
            let digest = Sha512::digest(s.as_bytes());
            Key::from(&digest)
        }
        _ => Key::generate(),
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
fn new_token() -> String {
    let mut b = [0u8; 32];
    OsRng.fill_bytes(&mut b);
    hex(&b)
}

pub fn hash_password(pw: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}
pub fn verify_password(hash: &str, pw: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default().verify_password(pw.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

fn valid_role(role: &str) -> bool {
    matches!(role, "customer" | "driver" | "admin")
}

// ─────────────────────────── the CurrentUser extractor ───────────────────────────
/// Resolves the signed session cookie to an account. Never rejects — handlers decide what
/// to do with `None` (so we can return precise 401/403 with messages).
pub struct CurrentUser(pub Option<Account>);

#[axum::async_trait]
impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let jar = SignedCookieJar::from_request_parts(parts, state).await.unwrap_or_else(|_| SignedCookieJar::new(state.key.clone()));
        let acct = jar
            .get(COOKIE)
            .map(|c| c.value().to_string())
            .and_then(|tok| state.store.session_account(&tok, now()).ok().flatten());
        Ok(CurrentUser(acct))
    }
}

/// Require a logged-in account with `role`; otherwise an error response (401/403).
pub fn require(user: &CurrentUser, role: &str) -> Result<Account, Response> {
    match &user.0 {
        None => Err((StatusCode::UNAUTHORIZED, "login required").into_response()),
        Some(a) if a.role == role => Ok(a.clone()),
        Some(_) => Err((StatusCode::FORBIDDEN, format!("requires {role} role")).into_response()),
    }
}
/// Require any logged-in account.
pub fn require_any(user: &CurrentUser) -> Result<Account, Response> {
    user.0.clone().ok_or_else(|| (StatusCode::UNAUTHORIZED, "login required").into_response())
}

// ─────────────────────────── handlers ───────────────────────────
#[derive(Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub role: String,
}

pub async fn register(State(st): State<AppState>, jar: SignedCookieJar, Json(req): Json<Credentials>) -> Response {
    let username = req.username.trim();
    let role = if req.role.is_empty() { "customer" } else { req.role.trim() };
    if username.len() < 3 || req.password.len() < 6 {
        return (StatusCode::BAD_REQUEST, "username ≥ 3 chars and password ≥ 6 chars").into_response();
    }
    if !valid_role(role) {
        return (StatusCode::BAD_REQUEST, "role must be customer, driver, or admin").into_response();
    }
    if st.store.account_by_username(username).ok().flatten().is_some() {
        return (StatusCode::CONFLICT, "username taken").into_response();
    }
    let hash = match hash_password(&req.password) {
        Ok(h) => h,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "hash error").into_response(),
    };
    let acct = match st.store.create_account(username, &hash, role, now()) {
        Ok(a) => a,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };
    issue_session(&st, jar, &acct)
}

pub async fn login(State(st): State<AppState>, jar: SignedCookieJar, Json(req): Json<Credentials>) -> Response {
    let found = st.store.account_by_username(req.username.trim()).ok().flatten();
    match found {
        Some((acct, hash)) if verify_password(&hash, &req.password) => issue_session(&st, jar, &acct),
        _ => (StatusCode::UNAUTHORIZED, "invalid username or password").into_response(),
    }
}

pub async fn logout(State(st): State<AppState>, jar: SignedCookieJar) -> Response {
    if let Some(tok) = jar.get(COOKIE).map(|c| c.value().to_string()) {
        let _ = st.store.delete_session(&tok);
    }
    let jar = jar.remove(Cookie::from(COOKIE));
    (jar, Json(serde_json::json!({"ok": true}))).into_response()
}

pub async fn me(user: CurrentUser) -> Response {
    match user.0 {
        Some(a) => Json(serde_json::json!({"username": a.username, "role": a.role})).into_response(),
        None => (StatusCode::UNAUTHORIZED, "not logged in").into_response(),
    }
}

fn issue_session(st: &AppState, jar: SignedCookieJar, acct: &Account) -> Response {
    let token = new_token();
    if st.store.create_session(&token, acct.id, now() + SESSION_SECS).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "session error").into_response();
    }
    let cookie = Cookie::build((COOKIE, token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(SESSION_SECS))
        .build();
    let jar = jar.add(cookie);
    (jar, Json(serde_json::json!({"username": acct.username, "role": acct.role}))).into_response()
}
