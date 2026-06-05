//! SPATIAL ROLE: THE STOREFRONT — Facilitator's multi-role PWA (Leptos SSR + Axum).
//! A thin presentation + job-routing layer over the FSL cognitive engine. It serves ONE
//! installable PWA for two roles (customer / driver), plus the platform "Engine" view.
//! Posting a job RUNS the existing FSL release binary (no core change): each job is a
//! root node (a "cable"), facilitated and recorded by the FSL harness. Truth stays in FSL.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use leptos::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use tower_http::services::ServeDir;

// ─────────────────────────── shared (in-memory) job store ───────────────────────────
#[derive(Clone, Serialize)]
struct Job {
    id: u64,
    text: String,
    kind: String,   // "free-form" → standardizes into presets from real usage
    status: String,
    fsl_note: String,
    created_unix: u64,
}
#[derive(Deserialize)]
struct NewJob {
    text: String,
}
#[derive(Clone, Default)]
struct AppState {
    jobs: Arc<Mutex<Vec<Job>>>,
    next: Arc<Mutex<u64>>,
}

// ─────────────────────────── PWA assets (baked in) ───────────────────────────
const CSS: &str = r##"
:root{--bg:#0b0f17;--panel:#121826;--ink:#e6edf3;--muted:#8aa0b6;--accent:#ffb020;--green:#36d399;--line:#1e2a3b}
*{box-sizing:border-box}html,body{margin:0;min-height:100%}
body{background:radial-gradient(1200px 600px at 70% -10%,#13203a 0,var(--bg) 55%);color:var(--ink);
 font:16px/1.55 ui-sans-serif,system-ui,-apple-system,Segoe UI,Roboto,Inter,sans-serif;-webkit-font-smoothing:antialiased;
 padding-top:calc(env(safe-area-inset-top) + 56px);padding-bottom:74px}
.appbar{position:fixed;top:0;left:0;right:0;height:56px;display:flex;align-items:center;gap:10px;padding:0 16px;
 background:rgba(11,15,23,.86);backdrop-filter:blur(8px);border-bottom:1px solid var(--line);z-index:20}
.logo{font-weight:800;letter-spacing:.3px}.logo b{color:var(--accent)}
.tag{color:var(--muted);font-size:12px}
.content{max-width:680px;margin:0 auto;padding:16px}
h1{font-size:22px;margin:6px 0 2px}h2{font-size:16px;margin:18px 0 8px}
.lede{color:var(--muted);margin:0 0 14px}
.card{background:var(--panel);border:1px solid var(--line);border-radius:16px;padding:16px;margin:12px 0;box-shadow:0 10px 30px rgba(0,0,0,.35)}
.role{display:flex;gap:12px;align-items:center;text-decoration:none;color:var(--ink)}
.role .emoji{font-size:30px}.role .t{font-weight:700}.role .d{color:var(--muted);font-size:13px}
.btn{appearance:none;border:0;border-radius:12px;background:var(--accent);color:#241600;font-weight:800;font-size:16px;padding:14px 16px;width:100%;cursor:pointer}
.btn.green{background:var(--green);color:#04240f}
.btn.ghost{background:transparent;color:var(--ink);border:1px solid var(--line)}
.btn:disabled{opacity:.6;cursor:progress}
.row{display:flex;gap:10px}.row>*{flex:1}
textarea{width:100%;min-height:120px;background:#060a12;border:1px solid var(--line);border-radius:12px;color:var(--ink);padding:12px;font:15px/1.5 inherit;resize:vertical}
.mic{flex:0 0 56px;border-radius:12px;border:1px solid var(--line);background:#060a12;color:var(--ink);font-size:20px}
.mic.rec{background:#3a1020;border-color:#ff5d73;color:#ff9db0}
.note{color:var(--muted);font-size:13px;margin-top:10px;min-height:18px}
.pill{display:inline-block;font-size:12px;font-weight:700;border-radius:999px;padding:4px 10px;border:1px solid var(--line)}
.pill.on{background:#0f2a1d;color:var(--green);border-color:#1f5b3f}.pill.off{background:#241016;color:#ff9db0}
.hints{margin:8px 0 0;padding-left:18px;color:#cfe0ef}.hints li{margin:4px 0}
.job{border:1px solid var(--line);border-radius:12px;padding:12px;margin:10px 0;background:#0a0f1a}
.job .jt{font-weight:700;color:var(--accent);font-size:13px}.job .jx{margin:4px 0}.job .jm{color:var(--muted);font-size:12px;margin-bottom:8px}
.accept{appearance:none;border:0;border-radius:10px;background:var(--green);color:#04240f;font-weight:700;padding:8px 12px;cursor:pointer}
.empty{color:var(--muted);text-align:center;padding:18px}
.out{background:#060a12;border:1px solid var(--line);border-radius:12px;padding:12px;max-height:52vh;overflow:auto;white-space:pre;font:12px/1.5 ui-monospace,Menlo,monospace;color:#cfe8df}
.tabbar{position:fixed;bottom:0;left:0;right:0;height:62px;display:flex;background:rgba(11,15,23,.92);backdrop-filter:blur(8px);border-top:1px solid var(--line);z-index:20}
.tabbar a{flex:1;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:2px;text-decoration:none;color:var(--muted);font-size:11px}
.tabbar a .i{font-size:18px}
.modal{display:none;position:fixed;inset:0;background:rgba(0,0,0,.6);align-items:flex-end;justify-content:center;z-index:30}
.sheet{background:var(--panel);border:1px solid var(--line);border-radius:18px 18px 0 0;padding:18px;width:100%;max-width:680px;position:relative}
.sheet .x{position:absolute;top:10px;right:12px;background:none;border:0;color:var(--muted);font-size:22px;cursor:pointer}
"##;

const COMMON_JS: &str = r##"
if('serviceWorker' in navigator){ navigator.serviceWorker.register('/sw.js').catch(function(e){console.warn('sw',e);}); }
"##;

const CUSTOMER_JS: &str = r##"
(function(){
 var t=document.getElementById('jobtext'),mic=document.getElementById('mic'),post=document.getElementById('post'),res=document.getElementById('result');
 var SR=window.SpeechRecognition||window.webkitSpeechRecognition;
 if(mic){mic.addEventListener('click',function(){
   if(!SR){res.textContent='The Listener (voice) needs Chrome/Android. Type your request instead.';return;}
   var r=new SR();r.lang='en-US';r.interimResults=false;mic.classList.add('rec');res.textContent='The Listener is listening…';
   r.onresult=function(e){var s=e.results[0][0].transcript;t.value=(t.value?t.value+' ':'')+s;res.textContent='Captured: '+s;};
   r.onerror=function(e){res.textContent='Listener error: '+e.error;};
   r.onend=function(){mic.classList.remove('rec');};
   r.start();
 });}
 if(post){post.addEventListener('click',function(){
   var text=(t.value||'').trim();if(!text){res.textContent='Describe the problem first — you have a problem, we fix it.';return;}
   post.disabled=true;res.textContent='Routing through the FSL engine…';
   fetch('/api/job',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({text:text})})
    .then(function(r){return r.json();})
    .then(function(j){res.textContent='Job #'+j.id+' posted as '+j.kind+'. FSL: '+j.fsl_note;t.value='';})
    .catch(function(e){res.textContent='Error: '+e;})
    .finally(function(){post.disabled=false;});
 });}
})();
"##;

const DRIVER_JS: &str = r##"
(function(){
 var jobsEl=document.getElementById('jobs'),hintsEl=document.getElementById('hints'),
     lbtn=document.getElementById('listener'),lstat=document.getElementById('lstat'),
     cm=document.getElementById('custmode'),qr=document.getElementById('qr'),
     modal=document.getElementById('modal'),mbody=document.getElementById('mbody'),mclose=document.getElementById('mclose');
 function showModal(h){mbody.innerHTML=h;modal.style.display='flex';}
 if(mclose)mclose.addEventListener('click',function(){modal.style.display='none';});
 var online=localStorage.getItem('fsl_listener')==='1';
 function renderListener(){if(lstat){lstat.textContent=online?'ONLINE — The Listener is active':'OFFLINE';lstat.className='pill '+(online?'on':'off');}if(lbtn)lbtn.textContent=online?'Go offline':'Go online';}
 if(lbtn)lbtn.addEventListener('click',function(){online=!online;localStorage.setItem('fsl_listener',online?'1':'0');renderListener();});
 renderListener();
 if(cm)cm.addEventListener('click',function(){showModal('<h3>Customer Mode</h3><p>Trigger phrase: <b>&ldquo;connect me to a person&rdquo;</b>.</p><p>Hand the phone to the customer for direct voice escalation to a human representative — the FSL cognitive system facilitates and records the interaction. <i>(simulated)</i></p>');});
 if(qr)qr.addEventListener('click',function(){showModal('<h3>Restaurant QR pipeline</h3><p>Scanning&hellip; <b>(placeholder)</b></p><p>In the full build, scanning a restaurant QR opens a preset pickup job. Coming soon.</p>');});
 function esc(s){return s.replace(/[&<>"]/g,function(c){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c];});}
 function hints(jobs){var h=[];if(jobs.length===0){h.push('No live jobs — stay online so The Listener can catch new ones.');}else{h.push('Shadow Intelligence: '+jobs.length+' live job(s) in range.');if(jobs.length>=2)h.push('Batch suggestion: chain 2+ nearby errands for higher $/mile (never forced).');h.push('Recurring free-form jobs become one-tap presets over time.');}hintsEl.innerHTML=h.map(function(x){return '<li>'+x+'</li>';}).join('');}
 function render(jobs){if(jobs.length===0){jobsEl.innerHTML='<div class="empty">No available jobs yet.</div>';}else{jobsEl.innerHTML=jobs.slice().reverse().map(function(j){return '<div class="job"><div class="jt">#'+j.id+' · '+esc(j.kind)+'</div><div class="jx">'+esc(j.text)+'</div><div class="jm">'+esc(j.status)+' · '+esc(j.fsl_note)+'</div><button class="accept" data-id="'+j.id+'">Accept</button></div>';}).join('');Array.prototype.forEach.call(document.querySelectorAll('.accept'),function(b){b.addEventListener('click',function(){showModal('<h3>Job #'+b.dataset.id+' accepted</h3><p>Maximum control, minimum bullshit — no forced batches, no map hijacking. Navigate when ready.</p>');});});}hints(jobs);}
 function load(){fetch('/api/jobs').then(function(r){return r.json();}).then(render).catch(function(){jobsEl.innerHTML='<div class="empty">Could not load jobs.</div>';});}
 load();setInterval(load,4000);
})();
"##;

const ADMIN_JS: &str = r##"
(function(){
 var b=document.getElementById('walk'),o=document.getElementById('out');
 if(!b||!o)return;
 b.addEventListener('click',function(){b.disabled=true;o.textContent='Running the FSL harness…';
  fetch('/api/walk').then(function(r){return r.text();}).then(function(t){o.textContent=t;})
   .catch(function(e){o.textContent='Error: '+e;}).finally(function(){b.disabled=false;});});
})();
"##;

const MANIFEST: &str = r##"{
  "name": "Facilitator — powered by FSL",
  "short_name": "Facilitator",
  "description": "Driver-first local problem-solving marketplace. You have a problem. We fix it.",
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

const SW: &str = r##"
const CACHE='facilitator-v2';
const SHELL=['/', '/customer', '/driver', '/manifest.webmanifest', '/icons/icon-192.png', '/icons/icon-512.png'];
self.addEventListener('install',function(e){e.waitUntil(caches.open(CACHE).then(function(c){return c.addAll(SHELL);}).then(function(){return self.skipWaiting();}));});
self.addEventListener('activate',function(e){e.waitUntil(caches.keys().then(function(k){return Promise.all(k.filter(function(x){return x!==CACHE;}).map(function(x){return caches.delete(x);}));}).then(function(){return self.clients.claim();}));});
self.addEventListener('fetch',function(e){var u=new URL(e.request.url);
 if(u.pathname.indexOf('/api/')===0){e.respondWith(fetch(e.request).catch(function(){return new Response('offline',{status:503});}));return;}
 e.respondWith(caches.match(e.request).then(function(r){return r||fetch(e.request);}));});
"##;

// ─────────────────────────── document shell ───────────────────────────
fn document(title: &str, body: String, extra_js: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head>\
<meta charset=\"utf-8\"/>\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1, viewport-fit=cover\"/>\
<meta name=\"theme-color\" content=\"#0b0f17\"/>\
<meta name=\"description\" content=\"Facilitator — driver-first local problem-solving marketplace, powered by FSL\"/>\
<link rel=\"manifest\" href=\"/manifest.webmanifest\"/>\
<link rel=\"apple-touch-icon\" href=\"/icons/apple-touch-icon.png\"/>\
<meta name=\"mobile-web-app-capable\" content=\"yes\"/>\
<meta name=\"apple-mobile-web-app-capable\" content=\"yes\"/>\
<meta name=\"apple-mobile-web-app-status-bar-style\" content=\"black-translucent\"/>\
<meta name=\"apple-mobile-web-app-title\" content=\"Facilitator\"/>\
<title>{title}</title><style>{css}</style></head><body>\
<header class=\"appbar\"><span class=\"logo\">⚡ <b>Facilitator</b></span><span class=\"tag\">powered by FSL</span></header>\
<main class=\"content\">{body}</main>\
<nav class=\"tabbar\">\
<a href=\"/\"><span class=\"i\">🏠</span>Home</a>\
<a href=\"/customer\"><span class=\"i\">➕</span>Post</a>\
<a href=\"/driver\"><span class=\"i\">🚗</span>Drive</a>\
<a href=\"/admin\"><span class=\"i\">⚙️</span>Engine</a>\
</nav>\
<div id=\"modal\" class=\"modal\"><div class=\"sheet\"><button id=\"mclose\" class=\"x\">×</button><div id=\"mbody\"></div></div></div>\
<script>{common}{extra}</script></body></html>",
        title = title, css = CSS, body = body, common = COMMON_JS, extra = extra_js,
    )
}

// ─────────────────────────── pages (Leptos SSR bodies) ───────────────────────────
async fn index() -> Html<String> {
    let body = leptos::ssr::render_to_string(|| {
        view! {
            <h1>"You have a problem. We fix it."</h1>
            <p class="lede">"A driver-first local problem-solving marketplace, powered by the FSL cognitive engine. Choose how you’re here today."</p>
            <a class="card role" href="/customer">
                <span class="emoji">"🧩"</span>
                <span><span class="t">"I need something done"</span><br/><span class="d">"Post a job — food, roadside, moving, errands, labor. Free-form voice or text."</span></span>
            </a>
            <a class="card role" href="/driver">
                <span class="emoji">"🚗"</span>
                <span><span class="t">"I drive / I work"</span><br/><span class="d">"See live jobs, shadow-intelligence hints, and The Listener. Maximum control, minimum bullshit."</span></span>
            </a>
            <a class="card role" href="/admin">
                <span class="emoji">"⚙️"</span>
                <span><span class="t">"Platform / Engine"</span><br/><span class="d">"Walk the FSL scene graph + hologram-compliance scan."</span></span>
            </a>
        }
    }).to_string();
    Html(document("Facilitator — You have a problem. We fix it.", body, ""))
}

async fn customer() -> Html<String> {
    let body = leptos::ssr::render_to_string(|| {
        view! {
            <h1>"Post a Job"</h1>
            <p class="lede">"Describe the problem in your own words — we sell solutions, not tasks. The Listener + FSL turn it into a clear, priced job."</p>
            <div class="card">
                <textarea id="jobtext" placeholder="e.g. “Need a couch moved from my 2nd-floor apartment to a truck downstairs this afternoon.”"></textarea>
                <div class="row" style="margin-top:10px">
                    <button id="post" class="btn">"Post a Job"</button>
                    <button id="mic" class="mic" title="Speak with The Listener">"🎙"</button>
                </div>
                <div id="result" class="note"></div>
            </div>
            <p class="lede">"Tip: recurring free-form requests automatically standardize into one-tap preset job types over time."</p>
        }
    }).to_string();
    Html(document("Facilitator — Post a Job", body, CUSTOMER_JS))
}

async fn driver() -> Html<String> {
    let body = leptos::ssr::render_to_string(|| {
        view! {
            <h1>"Driver dashboard"</h1>
            <p class="lede">"Driver-first: no forced batches, no map hijacking, no hidden information."</p>
            <div class="card">
                <h2 style="margin-top:0">"The Listener"</h2>
                <div><span id="lstat" class="pill off">"OFFLINE"</span></div>
                <p class="lede" style="margin:10px 0">"Always-on voice agent that wakes when you go online — captures notes, access instructions, and odd delivery details."</p>
                <div class="row">
                    <button id="listener" class="btn green">"Go online"</button>
                    <button id="custmode" class="btn ghost">"Customer Mode"</button>
                </div>
            </div>
            <div class="card">
                <h2 style="margin-top:0">"Shadow Intelligence"</h2>
                <ul id="hints" class="hints"></ul>
            </div>
            <h2>"Available jobs"</h2>
            <div id="jobs"></div>
            <button id="qr" class="btn ghost" style="margin-top:8px">"📷  Scan restaurant QR (preview)"</button>
        }
    }).to_string();
    Html(document("Facilitator — Drive", body, DRIVER_JS))
}

async fn admin() -> Html<String> {
    let body = leptos::ssr::render_to_string(|| {
        view! {
            <h1>"Platform / FSL Engine"</h1>
            <p class="lede">"Facilitator is a thin layer over the FSL cognitive engine. Walk the 3D scene graph and run the hologram-compliance scan — the core is unchanged."</p>
            <div class="card">
                <button id="walk" class="btn">"Walk the 3D scene graph"</button>
                <pre id="out" class="out">"Press “Walk the 3D scene graph” to run the FSL harness (scene tour + compliance)."</pre>
            </div>
        }
    }).to_string();
    Html(document("Facilitator — Engine", body, ADMIN_JS))
}

// ─────────────────────────── API (thin job-routing over FSL) ───────────────────────────
fn classify(text: &str) -> String {
    let t = text.to_lowercase();
    let has = |k: &str| t.contains(k);
    if has("deliver") || has("food") || has("pickup") || has("pick up") { "delivery (preset)".into() }
    else if has("tow") || has("roadside") || has("jump") || has("flat") || has("stuck") { "roadside (preset)".into() }
    else if has("move") || has("moving") || has("haul") || has("furniture") || has("couch") { "moving (preset)".into() }
    else if has("errand") || has("store") || has("grocery") || has("buy") { "errand (preset)".into() }
    else if has("help") || has("labor") || has("lift") || has("muscle") { "labor (preset)".into() }
    else { "free-form".into() }
}

/// Talk to the FSL core: run the existing harness and surface its compliance verdict as
/// the job's facilitation note. No core change — the binary is the source of truth.
async fn fsl_note() -> String {
    let bin = std::env::var("FSL_BIN").unwrap_or_else(|_| "fsl".into());
    match Command::new(&bin).output().await {
        Ok(o) => {
            let out = String::from_utf8_lossy(&o.stdout);
            out.lines()
                .rev()
                .find(|l| l.contains("coherence contract") || l.contains("ALL CHECKS PASS"))
                .map(|l| l.trim().trim_matches('─').trim().to_string())
                .unwrap_or_else(|| "facilitated by the FSL engine".into())
        }
        Err(_) => "FSL engine unavailable (job queued)".into(),
    }
}

async fn post_job(State(st): State<AppState>, Json(req): Json<NewJob>) -> Response {
    let text = req.text.trim().to_string();
    if text.is_empty() {
        return (StatusCode::BAD_REQUEST, "empty request").into_response();
    }
    let note = fsl_note().await; // route through the FSL core
    let id = {
        let mut n = st.next.lock().unwrap();
        *n += 1;
        *n
    };
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let job = Job { id, kind: classify(&text), text, status: "open".into(), fsl_note: note, created_unix: now };
    st.jobs.lock().unwrap().push(job.clone());
    Json(job).into_response()
}

async fn list_jobs(State(st): State<AppState>) -> Json<Vec<Job>> {
    Json(st.jobs.lock().unwrap().clone())
}

async fn manifest() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/manifest+json")], MANIFEST)
}
async fn service_worker() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript"),
            (header::CACHE_CONTROL, "no-cache"),
            (header::HeaderName::from_static("service-worker-allowed"), "/"),
        ],
        SW,
    )
}
async fn health() -> &'static str { "ok" }

/// Platform/Engine view: run the FSL harness and return its full output verbatim.
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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("could not run the FSL harness ({bin}): {e}")).into_response(),
    }
}

#[tokio::main]
async fn main() {
    let web_dir = std::env::var("FSL_WEB_DIR").unwrap_or_else(|_| ".".into());
    let bind = std::env::var("FSL_BIND").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("FSL_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);

    let state = AppState::default();
    let app = Router::new()
        .route("/", get(index))
        .route("/customer", get(customer))
        .route("/driver", get(driver))
        .route("/admin", get(admin))
        .route("/manifest.webmanifest", get(manifest))
        .route("/sw.js", get(service_worker))
        .route("/api/health", get(health))
        .route("/api/walk", get(walk))
        .route("/api/job", post(post_job))
        .route("/api/jobs", get(list_jobs))
        .nest_service("/icons", ServeDir::new(format!("{web_dir}/static/icons")))
        .with_state(state);

    let addr: SocketAddr = format!("{bind}:{port}").parse().expect("invalid bind address");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind");
    println!("Facilitator PWA (Leptos SSR + Axum) listening on http://{addr}");
    axum::serve(listener, app).await.expect("server error");
}
