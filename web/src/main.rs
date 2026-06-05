//! SPATIAL ROLE: THE STOREFRONT (bootstrap binary) — reads env, opens the durable SQLite
//! store, builds the app (which replays stored jobs to rebuild the FSL `World`), and
//! serves it. All logic lives in the `fsl_web` library; this binary is a thin entrypoint.

use fsl_web::{build_app, store::Store};

#[tokio::main]
async fn main() {
    let web_dir = std::env::var("FSL_WEB_DIR").unwrap_or_else(|_| ".".into());
    let db = std::env::var("FSL_DB").unwrap_or_else(|_| "facilitator.db".into());
    let secret = std::env::var("FSL_SECRET").ok();
    let bind = std::env::var("FSL_BIND").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("FSL_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);

    let store = Store::open(&db).expect("failed to open SQLite store");
    let (app, _state) = build_app(store, &web_dir, secret.as_deref());

    let addr: std::net::SocketAddr = format!("{bind}:{port}").parse().expect("invalid bind address");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind");
    println!("Facilitator (durable + authenticated, Leptos SSR + Axum + live FSL) on http://{addr}  [db={db}]");
    axum::serve(listener, app).await.expect("server error");
}
