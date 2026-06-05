#!/usr/bin/env bash
#
# SPATIAL ROLE: THE LAUNCHPAD — deploys the scene graph as a live, installable host;
# a presentation/deployment layer only, never touching the cable/strand/bulge core.
#
# setup.sh — one command to turn a fresh Debian machine into a live FSL host.
#
# It is a DEPLOYMENT / UX layer only. It does NOT touch the FSL core logic: the web
# server is a thin presentation layer that runs the existing release binary
# (target/release/fsl) and streams its 3D scene-graph walk + hologram-compliance output.
# The cable/strand/bulge spatial methodology and the FSL Coherence Contract are untouched.
#
# What it does (idempotent, safe to re-run):
#   1. updates the system + installs build dependencies
#   2. installs Rust (rustup, stable) if absent
#   3. clones/updates the repo in /opt/fsl and builds the workspace in release mode
#   4. generates a standalone Leptos(SSR)+Axum web app under /opt/fsl/web (a PWA)
#   5. installs a systemd service (fsl.service) serving the PWA on :8080
#   6. (best effort) installs Caddy as a reverse proxy with automatic HTTPS
#
# Usage:  sudo bash setup.sh
# Tunables (env): FSL_REPO_URL FSL_HOME FSL_PORT FSL_BIND FSL_DOMAIN RUST_CHANNEL
#
set -Eeuo pipefail

# ─────────────────────────────── configuration ───────────────────────────────
FSL_REPO_URL="${FSL_REPO_URL:-https://github.com/NFDFLDTHRY/Screechrac.git}"
FSL_HOME="${FSL_HOME:-/opt/fsl}"
FSL_PORT="${FSL_PORT:-8080}"
FSL_BIND="${FSL_BIND:-0.0.0.0}"          # 0.0.0.0 → reachable directly on :PORT; set 127.0.0.1 to hide behind Caddy
FSL_DOMAIN="${FSL_DOMAIN:-}"             # set to a real domain to enable Caddy automatic HTTPS
RUST_CHANNEL="${RUST_CHANNEL:-stable}"
export CARGO_HOME="${CARGO_HOME:-/opt/rust/cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-/opt/rust/rustup}"
WEB_DIR="$FSL_HOME/web"

# ─────────────────────────────── logging / errors ───────────────────────────────
log()  { printf '\033[1;32m[fsl]\033[0m %s\n'      "$*"; }
warn() { printf '\033[1;33m[fsl:warn]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[fsl:err]\033[0m %s\n'  "$*" >&2; exit 1; }
trap 'die "failed at line $LINENO (command: $BASH_COMMAND)"' ERR

# ─────────────────────────────── must be root ───────────────────────────────
if [ "$(id -u)" -ne 0 ]; then
  if command -v sudo >/dev/null 2>&1; then exec sudo -E bash "$0" "$@"; fi
  die "please run as root (sudo bash setup.sh)"
fi

export DEBIAN_FRONTEND=noninteractive
export NEEDRESTART_MODE=a                 # Debian 12: auto-restart services, never prompt

# retry a command up to 4× with exponential backoff (network resilience)
retry() {
  local n=0
  until "$@"; do
    n=$((n + 1))
    [ "$n" -ge 4 ] && return 1
    warn "retry $n/3: $*"
    sleep "$((2 ** n))"
  done
}
apt_get() { retry apt-get -o Dpkg::Options::=--force-confold -o Dpkg::Options::=--force-confdef "$@"; }

# ═════════════════════════════ 1) system + dependencies ═════════════════════════════
log "updating the system…"
apt_get update
apt_get upgrade -y

log "installing build dependencies…"
apt_get install -y \
  build-essential pkg-config libssl-dev \
  curl git ca-certificates gnupg jq imagemagick \
  apt-transport-https debian-keyring debian-archive-keyring

# ═════════════════════════════ 2) Rust toolchain (rustup) ═════════════════════════════
mkdir -p "$CARGO_HOME" "$RUSTUP_HOME"
export PATH="$CARGO_HOME/bin:$PATH"
if ! command -v rustc >/dev/null 2>&1 && [ ! -x "$CARGO_HOME/bin/rustc" ]; then
  log "installing Rust (rustup, $RUST_CHANNEL)…"
  retry curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
  sh /tmp/rustup-init.sh -y --no-modify-path --profile minimal --default-toolchain "$RUST_CHANNEL"
else
  log "Rust already present — ensuring $RUST_CHANNEL is current…"
fi
rustup default "$RUST_CHANNEL" >/dev/null 2>&1 || true
retry rustup update "$RUST_CHANNEL"
log "using $(rustc --version)"

# ═════════════════════════════ 3) clone/update + build workspace ═════════════════════════════
if [ -d "$FSL_HOME/.git" ]; then
  log "updating existing checkout in $FSL_HOME…"
  git config --global --add safe.directory "$FSL_HOME" || true
  retry git -C "$FSL_HOME" fetch --all --prune
  git -C "$FSL_HOME" pull --ff-only || warn "could not fast-forward; keeping current checkout"
else
  log "cloning $FSL_REPO_URL → $FSL_HOME…"
  mkdir -p "$(dirname "$FSL_HOME")"
  retry git clone "$FSL_REPO_URL" "$FSL_HOME"
  git config --global --add safe.directory "$FSL_HOME" || true
fi

cd "$FSL_HOME"
log "building the FSL workspace in release mode (this can take a while)…"
cargo build --release
FSL_BIN="$FSL_HOME/target/release/fsl"
[ -x "$FSL_BIN" ] || die "release binary not found at $FSL_BIN"
log "FSL harness built: $FSL_BIN"

# ═════════════════════════════ 4) generate the Leptos/Axum PWA ═════════════════════════════
# A SELF-CONTAINED crate (its own [workspace], so the FSL workspace is untouched). It is a
# thin presentation layer: /api/walk runs the existing FSL binary and returns its output.
log "generating the web platform (Leptos SSR + Axum) under $WEB_DIR…"
mkdir -p "$WEB_DIR/src" "$WEB_DIR/static/icons"

cat > "$WEB_DIR/Cargo.toml" <<'CARGO_EOF'
# SPATIAL ROLE: THE STOREFRONT — a thin presentation layer over the FSL harness (no core changes).
[package]
name = "fsl-web"
version = "0.1.0"
edition = "2021"
description = "Leptos(SSR)+Axum PWA front end for the FSL harness. Presentation only."

# Detached from the FSL workspace on purpose — this crate never alters core logic.
[workspace]

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "process"] }
tower-http = { version = "0.5", features = ["fs"] }
leptos = { version = "0.6", features = ["ssr"] }

[profile.release]
opt-level = 2
CARGO_EOF

cat > "$WEB_DIR/src/main.rs" <<'RUST_EOF'
//! SPATIAL ROLE: THE STOREFRONT — Leptos(SSR)+Axum presentation layer for the FSL harness.
//! It renders an installable PWA shell and exposes /api/walk, which RUNS the existing
//! release binary (the cable/strand/bulge scene-graph walk + hologram compliance) and
//! streams its output verbatim. No FSL core logic lives here; truth stays in the harness.

use axum::{
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use leptos::*;
use std::net::SocketAddr;
use tokio::process::Command;
use tower_http::services::ServeDir;

const CSS: &str = r#"
:root{--bg:#0b0f17;--panel:#121826;--ink:#e6edf3;--muted:#8aa0b6;--accent:#36d399;--line:#1e2a3b}
*{box-sizing:border-box}html,body{margin:0;height:100%}
body{background:radial-gradient(1200px 600px at 70% -10%,#13203a 0,var(--bg) 55%);color:var(--ink);
 font:16px/1.55 ui-sans-serif,system-ui,-apple-system,Segoe UI,Roboto,Inter,sans-serif;
 -webkit-font-smoothing:antialiased;padding:env(safe-area-inset-top) 0 0}
.wrap{max-width:920px;margin:0 auto;padding:22px 18px 60px}
.bar{display:flex;align-items:baseline;gap:12px;margin:8px 0 18px}
.brand{font-weight:800;letter-spacing:.5px;font-size:30px;color:var(--accent)}
.sub{color:var(--muted);font-size:14px}
.card{background:var(--panel);border:1px solid var(--line);border-radius:16px;padding:20px;
 box-shadow:0 10px 30px rgba(0,0,0,.35)}
.legend{display:flex;flex-wrap:wrap;gap:8px;margin:14px 0 6px}
.tag{font-size:12px;color:var(--muted);border:1px solid var(--line);border-radius:999px;padding:4px 10px}
.btn{appearance:none;border:0;border-radius:12px;background:var(--accent);color:#04240f;
 font-weight:700;font-size:16px;padding:12px 18px;margin-top:14px;cursor:pointer}
.btn:disabled{opacity:.6;cursor:progress}
.out{margin-top:16px;background:#060a12;border:1px solid var(--line);border-radius:12px;
 padding:14px;max-height:60vh;overflow:auto;white-space:pre;font:12.5px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace;color:#cfe8df}
.foot{color:var(--muted);font-size:12px;margin-top:18px;text-align:center}
"#;

const JS: &str = r#"
if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('/sw.js').catch(function(e){ console.warn('sw', e); });
}
(function(){
  var b=document.getElementById('walk'), o=document.getElementById('out');
  if(!b||!o) return;
  b.addEventListener('click', function(){
    b.disabled=true; o.textContent='Running the FSL harness…';
    fetch('/api/walk').then(function(r){ return r.text(); })
      .then(function(t){ o.textContent=t; })
      .catch(function(e){ o.textContent='Error: '+e; })
      .finally(function(){ b.disabled=false; });
  });
})();
"#;

const MANIFEST: &str = r##"{
  "name": "FSL — Foundational System Loop",
  "short_name": "FSL",
  "description": "Cable / strand / bulge spatial scene graph over the FSL harness.",
  "id": "/",
  "start_url": "/",
  "scope": "/",
  "display": "standalone",
  "orientation": "portrait-primary",
  "background_color": "#0b0f17",
  "theme_color": "#0b0f17",
  "icons": [
    { "src": "/icons/icon-192.png", "sizes": "192x192", "type": "image/png" },
    { "src": "/icons/icon-512.png", "sizes": "512x512", "type": "image/png" },
    { "src": "/icons/maskable-512.png", "sizes": "512x512", "type": "image/png", "purpose": "maskable" }
  ]
}"##;

const SW: &str = r#"
const CACHE = 'fsl-v1';
const SHELL = ['/', '/manifest.webmanifest', '/icons/icon-192.png', '/icons/icon-512.png'];
self.addEventListener('install', function(e){
  e.waitUntil(caches.open(CACHE).then(function(c){ return c.addAll(SHELL); }).then(function(){ return self.skipWaiting(); }));
});
self.addEventListener('activate', function(e){
  e.waitUntil(caches.keys().then(function(keys){
    return Promise.all(keys.filter(function(k){ return k !== CACHE; }).map(function(k){ return caches.delete(k); }));
  }).then(function(){ return self.clients.claim(); }));
});
self.addEventListener('fetch', function(e){
  var url = new URL(e.request.url);
  if (url.pathname.indexOf('/api/') === 0) {        // network-first for live harness output
    e.respondWith(fetch(e.request).catch(function(){ return new Response('offline', { status: 503 }); }));
    return;
  }
  e.respondWith(caches.match(e.request).then(function(r){ return r || fetch(e.request); }));  // cache-first shell
});
"#;

fn page() -> String {
    // Leptos SSR renders the interactive shell (full-stack Rust); the PWA <head> wraps it.
    let body = leptos::ssr::render_to_string(|| {
        view! {
            <div class="wrap">
                <div class="bar">
                    <div class="brand">"FSL"</div>
                    <div class="sub">"Foundational System Loop — cable · strand · bulge"</div>
                </div>
                <div class="card">
                    <p>"A human-walkable 3D scene graph. Every root node is a "<b>"cable"</b>", every derived element a "<b>"strand"</b>" anchored radially by degree-of-separation, the Coffee Cup arc a "<b>"bulge"</b>" that explodes into five stage planes — and "<b>"flow"</b>" is the sole connective primitive."</p>
                    <div class="legend">
                        <span class="tag">"Ledger = truth"</span>
                        <span class="tag">"Graph = navigation"</span>
                        <span class="tag">"Flow = sole connective primitive"</span>
                        <span class="tag">"Lens never mutates truth"</span>
                    </div>
                    <button id="walk" class="btn">"Walk the 3D scene graph"</button>
                    <pre id="out" class="out">"Press “Walk the 3D scene graph” to run the FSL harness and stream its output (scene tour + hologram-compliance scan)."</pre>
                </div>
                <div class="foot">"Presentation layer only — truth lives in the FSL harness, audited against the three canonical references."</div>
            </div>
        }
    })
    .to_string();

    format!(
        "<!doctype html><html lang=\"en\"><head>\
<meta charset=\"utf-8\"/>\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1, viewport-fit=cover\"/>\
<meta name=\"theme-color\" content=\"#0b0f17\"/>\
<meta name=\"description\" content=\"FSL — cable/strand/bulge spatial scene graph\"/>\
<link rel=\"manifest\" href=\"/manifest.webmanifest\"/>\
<link rel=\"apple-touch-icon\" href=\"/icons/apple-touch-icon.png\"/>\
<meta name=\"mobile-web-app-capable\" content=\"yes\"/>\
<meta name=\"apple-mobile-web-app-capable\" content=\"yes\"/>\
<meta name=\"apple-mobile-web-app-status-bar-style\" content=\"black-translucent\"/>\
<meta name=\"apple-mobile-web-app-title\" content=\"FSL\"/>\
<title>FSL — Foundational System Loop</title>\
<style>{css}</style></head><body>{body}<script>{js}</script></body></html>",
        css = CSS,
        body = body,
        js = JS,
    )
}

async fn index() -> Html<String> {
    Html(page())
}

async fn manifest() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/manifest+json")], MANIFEST)
}

async fn service_worker() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript"),
            (header::CACHE_CONTROL, "no-cache"),
            // allow the SW to control the whole origin, not just /sw.js
            (header::HeaderName::from_static("service-worker-allowed"), "/"),
        ],
        SW,
    )
}

async fn health() -> &'static str {
    "ok"
}

/// THE THIN SEAM: run the existing FSL release binary and return its stdout verbatim.
async fn walk() -> Response {
    let bin = std::env::var("FSL_BIN").unwrap_or_else(|_| "fsl".into());
    match Command::new(&bin).output().await {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
            if !o.stderr.is_empty() {
                s.push_str("\n──── stderr ────\n");
                s.push_str(&String::from_utf8_lossy(&o.stderr));
            }
            ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], s).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("could not run the FSL harness ({bin}): {e}"),
        )
            .into_response(),
    }
}

#[tokio::main]
async fn main() {
    let web_dir = std::env::var("FSL_WEB_DIR").unwrap_or_else(|_| ".".into());
    let bind = std::env::var("FSL_BIND").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("FSL_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);

    let app = Router::new()
        .route("/", get(index))
        .route("/manifest.webmanifest", get(manifest))
        .route("/sw.js", get(service_worker))
        .route("/api/health", get(health))
        .route("/api/walk", get(walk))
        .nest_service("/icons", ServeDir::new(format!("{web_dir}/static/icons")));

    let addr: SocketAddr = format!("{bind}:{port}").parse().expect("invalid bind address");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind");
    println!("fsl-web (Leptos SSR + Axum) listening on http://{addr}");
    axum::serve(listener, app).await.expect("server error");
}
RUST_EOF

# ── PWA assets that must be real files: the icons (manifest + service worker are baked in) ──
gen_icons() {
  local I="$WEB_DIR/static/icons"
  convert -size 512x512 xc:'#0b0f17' -fill '#36d399' -gravity center -pointsize 210 \
    -annotate +0+0 'FSL' "$I/icon-512.png"
  convert "$I/icon-512.png" -resize 192x192 "$I/icon-192.png"
  convert "$I/icon-512.png" -resize 180x180 "$I/apple-touch-icon.png"
  # maskable: same art with extra safe-zone padding
  convert -size 512x512 xc:'#0b0f17' -fill '#36d399' -gravity center -pointsize 150 \
    -annotate +0+0 'FSL' "$I/maskable-512.png"
}
if gen_icons; then log "generated PWA icons"; else warn "icon generation failed (imagemagick); PWA still serves but icons may 404"; fi

log "building the web platform in release mode (downloads Leptos/Axum on first run)…"
cd "$WEB_DIR"
cargo build --release
WEB_BIN="$WEB_DIR/target/release/fsl-web"
[ -x "$WEB_BIN" ] || die "web binary not found at $WEB_BIN"
log "web platform built: $WEB_BIN"

# ═════════════════════════════ 5) systemd service ═════════════════════════════
log "installing systemd service fsl.service (port $FSL_PORT)…"
cat > /etc/systemd/system/fsl.service <<UNIT_EOF
[Unit]
Description=FSL — Foundational System Loop (Leptos/Axum PWA server)
Documentation=https://github.com/NFDFLDTHRY/Screechrac
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
Environment=FSL_BIN=$FSL_BIN
Environment=FSL_WEB_DIR=$WEB_DIR
Environment=FSL_BIND=$FSL_BIND
Environment=FSL_PORT=$FSL_PORT
WorkingDirectory=$WEB_DIR
ExecStart=$WEB_BIN
Restart=always
RestartSec=2
NoNewPrivileges=true
ProtectSystem=full
ProtectHome=read-only

[Install]
WantedBy=multi-user.target
UNIT_EOF

systemctl daemon-reload
systemctl enable fsl.service >/dev/null 2>&1 || true
systemctl restart fsl.service
sleep 2
systemctl is-active --quiet fsl.service && log "fsl.service is active" || warn "fsl.service did not become active — check: journalctl -u fsl -e"

# ═════════════════════════════ 6) Caddy reverse proxy (best effort) ═════════════════════════════
# Run in a failure-tolerant function: a Caddy hiccup must never fail the whole deploy.
setup_caddy() {
  if ! command -v caddy >/dev/null 2>&1; then
    log "installing Caddy (reverse proxy + automatic HTTPS)…"
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' \
      | gpg --batch --yes --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' \
      > /etc/apt/sources.list.d/caddy-stable.list
    apt_get update
    apt_get install -y caddy
  fi

  if [ -n "$FSL_DOMAIN" ]; then
    log "configuring Caddy for $FSL_DOMAIN (automatic HTTPS)…"
    cat > /etc/caddy/Caddyfile <<CADDY_EOF
$FSL_DOMAIN {
	encode zstd gzip
	reverse_proxy 127.0.0.1:$FSL_PORT
}
CADDY_EOF
  else
    log "configuring Caddy on :80 (no domain set → HTTP reverse proxy)…"
    cat > /etc/caddy/Caddyfile <<CADDY_EOF
:80 {
	encode zstd gzip
	reverse_proxy 127.0.0.1:$FSL_PORT
}
CADDY_EOF
  fi

  systemctl enable caddy >/dev/null 2>&1 || true
  systemctl reload caddy 2>/dev/null || systemctl restart caddy
}
if setup_caddy; then CADDY_OK=1; else CADDY_OK=0; warn "Caddy step skipped (optional) — direct access on :$FSL_PORT still works"; fi

# ═════════════════════════════ final instructions ═════════════════════════════
IP="$(hostname -I 2>/dev/null | awk '{print $1}')"; [ -n "$IP" ] || IP="<server-ip>"
echo
log "──────────────────────────────────────────────────────────────"
log "FSL is LIVE."
echo
echo "  Direct (backend) ........ http://$IP:$FSL_PORT"
echo "  Localhost ............... http://localhost:$FSL_PORT"
if [ "${CADDY_OK:-0}" = "1" ]; then
  if [ -n "$FSL_DOMAIN" ]; then
    echo "  Public (HTTPS) .......... https://$FSL_DOMAIN   (Caddy auto-TLS; point DNS A/AAAA at $IP)"
  else
    echo "  Public (HTTP via Caddy) . http://$IP            (set FSL_DOMAIN=example.com and re-run for HTTPS)"
  fi
fi
echo
echo "  Install as an Android app (PWA):"
echo "    1. Open the URL above in Chrome on Android."
echo "    2. Tap ⋮ menu → “Install app” / “Add to Home screen”."
echo "    3. Launch it from your home screen — it runs standalone, full-screen, offline-capable."
echo
echo "  Manage the service:"
echo "    systemctl status fsl       # health"
echo "    journalctl -u fsl -e -f    # logs"
echo "    systemctl restart fsl      # restart"
echo
echo "  Update to the latest code (idempotent):"
echo "    sudo bash $FSL_HOME/setup.sh"
echo
echo "  Layout:"
echo "    repo + harness ... $FSL_HOME            (cargo build --release → $FSL_BIN)"
echo "    web platform ..... $WEB_DIR             ($WEB_BIN)"
echo "    service .......... /etc/systemd/system/fsl.service"
[ "${CADDY_OK:-0}" = "1" ] && echo "    reverse proxy .... /etc/caddy/Caddyfile"
log "──────────────────────────────────────────────────────────────"
