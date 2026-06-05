//! SPATIAL ROLE: THE STOREFRONT — Facilitator's multi-role PWA (Leptos SSR + Axum),
//! now functionally real. It drives the FSL cognitive engine AS A LIBRARY: every posted
//! job is a CABLE (root node) ingested into a persistent FSL `World`; the fsl-llm
//! Facilitator engine structures it and raises clarifying UNKs (The Listener); answers
//! flow back through the Onion Shell to resolve those UNKs across ticks; the Coffee Cup
//! arc advances each pass; and the Ledger stays the source of truth. The web layer is
//! thin — it only orchestrates FSL calls and presents results. No FSL core crate changes.

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

use fsl_actor::Params;
use fsl_core::{Claim, InvalidPolicy, Span, TopicId, UnkPolicy};
use fsl_llm::facilitator;
use fsl_llm::DeterministicOracle;
use fsl_mind::World;

// ─────────────────────────── application state ───────────────────────────
#[derive(Clone, Serialize)]
struct Question {
    slot: String,
    ask: String,
}
#[derive(Clone, Serialize)]
struct Job {
    id: u64,
    text: String,
    title: String,
    preset: String,      // display kind; "free-form" until standardized
    preset_kind: String, // machine key from the engine
    price_band: String,
    status: String,                  // needs_info | ready | accepted | completed
    open_questions: Vec<Question>,   // open UNKs (Listener questions)
    filled: Vec<(String, String)>,   // resolved slots → OBS
    // ── live FSL trace ──
    cable: u64,     // root-node id (the cable this job rides)
    strands: usize, // grains attached this pass (OBS/UNK/DELTA/…)
    stage: String,  // Coffee Cup stage
    coherent: bool, // sandbox coherence after processing
    accepted_by: Option<String>,
    created_unix: u64,
}

#[derive(Clone)]
struct AppState {
    world: Arc<Mutex<World>>,
    oracle: DeterministicOracle,
    jobs: Arc<Mutex<Vec<Job>>>,
    next_job: Arc<Mutex<u64>>,
    next_claim: Arc<Mutex<u64>>,
}
impl AppState {
    fn new() -> Self {
        // A persistent FSL World is the source of truth for all jobs. Two minds + a
        // default crossing give the membrane/bridge/Coffee-Cup a channel to run on.
        let policy = InvalidPolicy { stop_words: vec!["you never".into(), "obviously".into()] };
        let unk_policy = UnkPolicy::BoundedBudget { max_open_unk: 5, max_strand_depth: 3 };
        let mut world = World::new(policy, unk_policy);
        let a = world.spawn(None, Params::default(), vec![]);
        let b = world.spawn(None, Params::default(), vec![]);
        let x = world.open_pair(1, a, b, false);
        world.set_default_pair(x, a, b);
        AppState {
            world: Arc::new(Mutex::new(world)),
            oracle: DeterministicOracle::default(),
            jobs: Arc::new(Mutex::new(vec![])),
            next_job: Arc::new(Mutex::new(0)),
            next_claim: Arc::new(Mutex::new(1)),
        }
    }
    fn claim_id(&self) -> u64 {
        let mut n = self.next_claim.lock().unwrap();
        *n += 1;
        *n
    }
    fn job_id(&self) -> u64 {
        let mut n = self.next_job.lock().unwrap();
        *n += 1;
        *n
    }
}

fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// ─────────────────────────── FSL orchestration (thin) ───────────────────────────
/// Post a free-form request → structure it through the fsl-llm engine and ingest it as a
/// CABLE into the FSL World, raising an UNK per missing slot (the Listener's questions).
fn create_job(st: &AppState, text: &str) -> Job {
    let prop = facilitator::structure_job(&st.oracle, text);

    // Build claims OUTSIDE the world lock (only the id counter is touched here):
    //  • anchored slots → pointable OBS on the slot's topic,
    //  • missing slots  → non-pointable claims on the slot's topic → become UNKs.
    let mut claims: Vec<Claim> = vec![];
    for (slot, _v) in &prop.present {
        claims.push(Claim::pointable(
            st.claim_id(),
            &format!("{slot}: provided"),
            Span { quote: slot.clone(), context: "job".into() },
            facilitator::slot_topic(slot),
            1,
        ));
    }
    for c in &prop.missing {
        let mut cl = Claim::plain(st.claim_id(), &format!("missing {}", c.slot));
        cl.topic = Some(TopicId(facilitator::slot_topic(&c.slot)));
        claims.push(cl);
    }
    if claims.is_empty() {
        claims.push(Claim::plain(st.claim_id(), text));
    }

    let (cable, strands, stage, coherent) = {
        let mut w = st.world.lock().unwrap();
        let root = w.ingest_input(text); // the CABLE
        let tr = w.primary_loop(&st.oracle, root, claims, false);
        let coherent = w.coherence().2;
        let strands = tr.obs + tr.unks_raised + tr.deltas + tr.invalids + tr.converted;
        (root.0, strands, format!("{:?}", tr.cup_stage), coherent)
    };

    let standardized = prop.is_standardized();
    let job = Job {
        id: st.job_id(),
        text: text.to_string(),
        title: prop.title.clone(),
        preset: if standardized { prop.preset.label().to_string() } else { "free-form".to_string() },
        preset_kind: prop.preset.label().to_string(),
        price_band: prop.price_band.clone(),
        status: if prop.missing.is_empty() { "ready".into() } else { "needs_info".into() },
        open_questions: prop.missing.iter().map(|c| Question { slot: c.slot.clone(), ask: c.ask.clone() }).collect(),
        filled: prop.present.iter().map(|(s, v)| (s.clone(), v.clone())).collect(),
        cable,
        strands,
        stage,
        coherent,
        accepted_by: None,
        created_unix: now_unix(),
    };
    st.jobs.lock().unwrap().push(job.clone());
    job
}

/// Answer a clarifying question (The Listener) → feed a pointable OBS on the slot's topic
/// into the World and run the Onion Shell so the awaiting UNK resolves across ticks.
fn clarify_job(st: &AppState, job_id: u64, slot: &str, answer: &str) -> Option<Job> {
    // ── world phase (lock world only) ──
    let coherent = {
        let claim = Claim::pointable(
            st.claim_id(),
            &format!("{slot}: {answer}"),
            Span { quote: answer.to_string(), context: "clarify".into() },
            facilitator::slot_topic(slot),
            1,
        );
        let mut w = st.world.lock().unwrap();
        let r = w.ingest_input(answer);
        let _ = w.primary_loop(&st.oracle, r, vec![claim], false);
        w.detect_and_trigger(); // non-blocking trigger: the awaiting UNK is now eligible
        let _ = w.onion_resolve(&st.oracle, 16); // Onion Shell resolves it (cross-tick)
        w.coherence().2
    };

    // ── jobs phase (lock jobs only) ──
    let mut jobs = st.jobs.lock().unwrap();
    let job = jobs.iter_mut().find(|j| j.id == job_id)?;
    job.filled.push((slot.to_string(), answer.to_string()));
    job.open_questions.retain(|q| q.slot != slot);
    job.coherent = coherent;
    if job.open_questions.is_empty() {
        job.status = "ready".into();
        // free-form has crossed its Delta Bridge → standardize into a preset.
        if job.preset == "free-form" {
            job.preset = job.preset_kind.clone();
        }
    }
    Some(job.clone())
}

// ─────────────────────────── request / response bodies ───────────────────────────
#[derive(Deserialize)]
struct NewJob {
    text: String,
}
#[derive(Deserialize)]
struct Clarification {
    job_id: u64,
    slot: String,
    answer: String,
}
#[derive(Deserialize)]
struct JobRef {
    job_id: u64,
    #[serde(default)]
    driver: String,
}
#[derive(Serialize)]
struct FslStats {
    cables: usize,
    strands: usize,
    flows: usize,
    open_unks: usize,
    resolved_unks: usize,
    coherent: bool,
}
#[derive(Serialize)]
struct JobsView {
    jobs: Vec<Job>,
    hints: Vec<String>,
    fsl: FslStats,
}

// ─────────────────────────── API handlers ───────────────────────────
async fn post_job(State(st): State<AppState>, Json(req): Json<NewJob>) -> Response {
    let text = req.text.trim().to_string();
    if text.is_empty() {
        return (StatusCode::BAD_REQUEST, "describe the problem first").into_response();
    }
    Json(create_job(&st, &text)).into_response()
}

async fn clarify(State(st): State<AppState>, Json(req): Json<Clarification>) -> Response {
    let ans = req.answer.trim();
    if ans.is_empty() {
        return (StatusCode::BAD_REQUEST, "empty answer").into_response();
    }
    match clarify_job(&st, req.job_id, &req.slot, ans) {
        Some(job) => Json(job).into_response(),
        None => (StatusCode::NOT_FOUND, "no such job").into_response(),
    }
}

async fn accept(State(st): State<AppState>, Json(req): Json<JobRef>) -> Response {
    let mut jobs = st.jobs.lock().unwrap();
    match jobs.iter_mut().find(|j| j.id == req.job_id) {
        Some(j) if j.status == "ready" => {
            j.status = "accepted".into();
            j.accepted_by = Some(if req.driver.is_empty() { "driver".into() } else { req.driver.clone() });
            Json(j.clone()).into_response()
        }
        Some(_) => (StatusCode::CONFLICT, "job is not in a ready state").into_response(),
        None => (StatusCode::NOT_FOUND, "no such job").into_response(),
    }
}

async fn complete(State(st): State<AppState>, Json(req): Json<JobRef>) -> Response {
    let mut jobs = st.jobs.lock().unwrap();
    match jobs.iter_mut().find(|j| j.id == req.job_id) {
        Some(j) => {
            j.status = "completed".into();
            Json(j.clone()).into_response()
        }
        None => (StatusCode::NOT_FOUND, "no such job").into_response(),
    }
}

/// Customer Mode escalation routed THROUGH the FSL cognitive system: the escalation is
/// ingested as structure that crosses the membrane; we return the live FSL record.
async fn escalate(State(st): State<AppState>, Json(req): Json<JobRef>) -> Response {
    let claim = Claim::pointable(
        st.claim_id(),
        "[Customer Mode] connect me to a person",
        Span { quote: "connect me to a person".into(), context: "escalation".into() },
        facilitator::slot_topic("escalation"),
        1,
    );
    let (stage, bridge, behavior) = {
        let mut w = st.world.lock().unwrap();
        let r = w.ingest_input("Customer Mode escalation");
        let tr = w.primary_loop(&st.oracle, r, vec![claim], false);
        (format!("{:?}", tr.cup_stage), format!("{:?}", tr.bridge), tr.behavior.unwrap_or_else(|| "facilitating".into()))
    };
    Json(serde_json::json!({
        "job_id": req.job_id,
        "recorded": true,
        "stage": stage,
        "bridge": bridge,
        "behavior": behavior,
        "note": "Live voice handed to a human representative — the FSL cognitive system is facilitating and recording the interaction."
    }))
    .into_response()
}

async fn list_jobs(State(st): State<AppState>) -> Json<JobsView> {
    let jobs = st.jobs.lock().unwrap().clone();
    let (cables, strands, flows, open_unks, resolved_unks, coherent) = {
        let w = st.world.lock().unwrap();
        let scene = w.project_scene();
        let (c, s, f) = scene.stats();
        let (res, open, coh) = w.coherence();
        (c, s, f, open, res, coh)
    };

    let needs = jobs.iter().filter(|j| j.status == "needs_info").count();
    let ready = jobs.iter().filter(|j| j.status == "ready").count();
    let mut hints = vec![];
    if needs > 0 {
        hints.push(format!("{needs} job(s) awaiting clarification — open UNKs in the FSL Onion Shell."));
    }
    if ready > 0 {
        hints.push(format!("{ready} job(s) ready to accept right now."));
    }
    // batching suggestion (never forced): 2+ ready jobs of the same kind.
    use std::collections::BTreeMap;
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    for j in jobs.iter().filter(|j| j.status == "ready") {
        *by_kind.entry(j.preset_kind.clone()).or_insert(0) += 1;
    }
    for (k, n) in by_kind.iter() {
        if *n >= 2 {
            hints.push(format!("Batch suggestion: {n} {k} jobs nearby could be chained (your call — never forced)."));
        }
    }
    hints.push(format!(
        "FSL graph: {cables} cables (jobs), {strands} strands, {flows} flows; coherent={coherent}."
    ));
    if needs == 0 && ready > 0 {
        hints.push("Fully-specified requests have standardized into one-tap presets.".into());
    }

    Json(JobsView { jobs, hints, fsl: FslStats { cables, strands, flows, open_unks, resolved_unks, coherent } })
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
.btn.green{background:var(--green);color:#04240f}.btn.ghost{background:transparent;color:var(--ink);border:1px solid var(--line)}
.btn:disabled{opacity:.6;cursor:progress}
.row{display:flex;gap:10px}.row>*{flex:1}
textarea,input{width:100%;background:#060a12;border:1px solid var(--line);border-radius:12px;color:var(--ink);padding:12px;font:15px/1.5 inherit}
textarea{min-height:120px;resize:vertical}
.mic{flex:0 0 56px;border-radius:12px;border:1px solid var(--line);background:#060a12;color:var(--ink);font-size:20px}
.mic.rec{background:#3a1020;border-color:#ff5d73;color:#ff9db0}
.note{color:var(--muted);font-size:13px;margin-top:10px;min-height:18px}
.pill{display:inline-block;font-size:12px;font-weight:700;border-radius:999px;padding:4px 10px;border:1px solid var(--line)}
.pill.on{background:#0f2a1d;color:var(--green);border-color:#1f5b3f}.pill.off{background:#241016;color:#ff9db0}
.pill.amber{background:#2a210f;color:var(--accent);border-color:#5b481f}
.trace{font:12px/1.5 ui-monospace,Menlo,monospace;color:#9fd9c6;margin-top:8px}
.q{border:1px solid var(--line);border-radius:12px;padding:10px;margin:8px 0;background:#0a0f1a}
.q .ask{font-weight:600;margin-bottom:6px}
.hints{margin:8px 0 0;padding-left:18px;color:#cfe0ef}.hints li{margin:4px 0}
.job{border:1px solid var(--line);border-radius:12px;padding:12px;margin:10px 0;background:#0a0f1a}
.job .jt{font-weight:700;color:var(--accent);font-size:13px}.job .jx{margin:4px 0}.job .jm{color:var(--muted);font-size:12px;margin-bottom:8px}
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
function esc(s){return (''+s).replace(/[&<>"]/g,function(c){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c];});}
function trace(j){return 'FSL · cable #'+j.cable+' · '+j.strands+' strands · stage '+esc(j.stage)+' · coherent='+j.coherent;}
"##;

const CUSTOMER_JS: &str = r##"
(function(){
 var t=document.getElementById('jobtext'),mic=document.getElementById('mic'),post=document.getElementById('post'),
     res=document.getElementById('result'),panel=document.getElementById('panel');
 var SR=window.SpeechRecognition||window.webkitSpeechRecognition;
 if(mic){mic.addEventListener('click',function(){
   if(!SR){res.textContent='The Listener (voice) needs Chrome/Android. Type your request instead.';return;}
   var r=new SR();r.lang='en-US';r.interimResults=false;mic.classList.add('rec');res.textContent='The Listener is listening…';
   r.onresult=function(e){var s=e.results[0][0].transcript;t.value=(t.value?t.value+' ':'')+s;res.textContent='Captured: '+s;};
   r.onerror=function(e){res.textContent='Listener error: '+e.error;};r.onend=function(){mic.classList.remove('rec');};r.start();
 });}
 function render(j){
   var html='<div class="card"><div><b>'+esc(j.title)+'</b> <span class="pill amber">'+esc(j.preset)+'</span></div>'+
     '<div class="lede" style="margin:8px 0">Price band: '+esc(j.price_band)+'</div>'+
     '<div class="trace">'+trace(j)+'</div>';
   if(j.status==='ready'){
     html+='<p style="margin-top:12px">✅ <b>Job ready.</b> The Listener + FSL fully structured it — drivers can accept it now.</p>';
   }else{
     html+='<h2>The Listener needs a few details</h2>';
     j.open_questions.forEach(function(q){
       html+='<div class="q"><div class="ask">'+esc(q.ask)+'</div>'+
         '<div class="row"><input id="a_'+q.slot+'" placeholder="Type or speak your answer"/>'+
         '<button class="btn green" style="flex:0 0 110px" onclick="answer('+j.id+',\''+q.slot+'\')">Answer</button></div></div>';
     });
   }
   html+='</div>';
   panel.innerHTML=html;
 }
 window.answer=function(id,slot){
   var el=document.getElementById('a_'+slot);var val=(el&&el.value||'').trim();
   if(!val){return;}
   fetch('/api/clarify',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({job_id:id,slot:slot,answer:val})})
    .then(function(r){return r.json();}).then(render).catch(function(e){res.textContent='Error: '+e;});
 };
 if(post){post.addEventListener('click',function(){
   var text=(t.value||'').trim();if(!text){res.textContent='Describe the problem — you have a problem, we fix it.';return;}
   post.disabled=true;res.textContent='Routing through the FSL engine…';
   fetch('/api/job',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({text:text})})
    .then(function(r){return r.json();}).then(function(j){res.textContent='Posted job #'+j.id+'.';t.value='';render(j);})
    .catch(function(e){res.textContent='Error: '+e;}).finally(function(){post.disabled=false;});
 });}
})();
"##;

const DRIVER_JS: &str = r##"
(function(){
 var jobsEl=document.getElementById('jobs'),hintsEl=document.getElementById('hints'),fslEl=document.getElementById('fsl'),
     lbtn=document.getElementById('listener'),lstat=document.getElementById('lstat'),
     qr=document.getElementById('qr'),modal=document.getElementById('modal'),mbody=document.getElementById('mbody'),mclose=document.getElementById('mclose');
 function showModal(h){mbody.innerHTML=h;modal.style.display='flex';}
 if(mclose)mclose.addEventListener('click',function(){modal.style.display='none';});
 var online=localStorage.getItem('fsl_listener')==='1';
 function renderListener(){if(lstat){lstat.textContent=online?'ONLINE — The Listener is active':'OFFLINE';lstat.className='pill '+(online?'on':'off');}if(lbtn)lbtn.textContent=online?'Go offline':'Go online';}
 if(lbtn)lbtn.addEventListener('click',function(){online=!online;localStorage.setItem('fsl_listener',online?'1':'0');renderListener();});
 renderListener();
 if(qr)qr.addEventListener('click',function(){showModal('<h3>Restaurant QR pipeline</h3><p>Scanning&hellip; <b>(placeholder)</b></p><p>In the full build, scanning a restaurant QR opens a preset pickup job. Coming soon.</p>');});
 window.act=function(id,kind){
   var url=kind==='accept'?'/api/accept':(kind==='complete'?'/api/complete':'/api/escalate');
   fetch(url,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({job_id:id,driver:'you'})})
    .then(function(r){return r.json();}).then(function(j){
       if(kind==='escalate'){showModal('<h3>Customer Mode</h3><p>'+esc(j.note)+'</p><div class="trace">FSL · bridge '+esc(j.bridge)+' · stage '+esc(j.stage)+' · behavior '+esc(j.behavior)+' · recorded='+j.recorded+'</div>');}
       load();
    }).catch(function(e){showModal('<p>Error: '+esc(e)+'</p>');});
 };
 function jobCard(j){
   var actions='';
   if(j.status==='ready')actions='<button class="btn green" onclick="act('+j.id+',\'accept\')">Accept</button>';
   else if(j.status==='accepted')actions='<div class="row"><button class="btn" onclick="act('+j.id+',\'escalate\')">Customer Mode</button><button class="btn green" onclick="act('+j.id+',\'complete\')">Complete</button></div>';
   else if(j.status==='completed')actions='<span class="pill on">completed</span>';
   else actions='<span class="pill amber">awaiting customer clarification</span>';
   return '<div class="job"><div class="jt">#'+j.id+' · '+esc(j.preset)+' · '+esc(j.price_band)+'</div>'+
     '<div class="jx">'+esc(j.title)+'</div>'+
     '<div class="jm">'+trace(j)+'</div>'+actions+'</div>';
 }
 function render(v){
   var visible=v.jobs.filter(function(j){return j.status!=='completed';});
   jobsEl.innerHTML=visible.length?visible.slice().reverse().map(jobCard).join(''):'<div class="empty">No live jobs yet. Stay online so The Listener can catch new ones.</div>';
   hintsEl.innerHTML=v.hints.map(function(h){return '<li>'+esc(h)+'</li>';}).join('');
   if(fslEl)fslEl.textContent='FSL: '+v.fsl.cables+' cables · '+v.fsl.strands+' strands · '+v.fsl.flows+' flows · open UNKs '+v.fsl.open_unks+' · resolved '+v.fsl.resolved_unks+' · coherent '+v.fsl.coherent;
 }
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
const CACHE='facilitator-v3';
const SHELL=['/', '/customer', '/driver', '/manifest.webmanifest', '/icons/icon-192.png', '/icons/icon-512.png'];
self.addEventListener('install',function(e){e.waitUntil(caches.open(CACHE).then(function(c){return c.addAll(SHELL);}).then(function(){return self.skipWaiting();}));});
self.addEventListener('activate',function(e){e.waitUntil(caches.keys().then(function(k){return Promise.all(k.filter(function(x){return x!==CACHE;}).map(function(x){return caches.delete(x);}));}).then(function(){return self.clients.claim();}));});
self.addEventListener('fetch',function(e){var u=new URL(e.request.url);
 if(u.pathname.indexOf('/api/')===0){e.respondWith(fetch(e.request).catch(function(){return new Response('offline',{status:503});}));return;}
 e.respondWith(caches.match(e.request).then(function(r){return r||fetch(e.request);}));});
"##;

// ─────────────────────────── document shell + pages ───────────────────────────
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

async fn index() -> Html<String> {
    let body = leptos::ssr::render_to_string(|| {
        view! {
            <h1>"You have a problem. We fix it."</h1>
            <p class="lede">"A driver-first local problem-solving marketplace, powered by the FSL cognitive engine. Choose how you’re here today."</p>
            <a class="card role" href="/customer">
                <span class="emoji">"🧩"</span>
                <span><span class="t">"I need something done"</span><br/><span class="d">"Describe a problem — The Listener + FSL turn it into a clear, priced job."</span></span>
            </a>
            <a class="card role" href="/driver">
                <span class="emoji">"🚗"</span>
                <span><span class="t">"I drive / I work"</span><br/><span class="d">"Live jobs, shadow-intelligence hints from FSL, and The Listener. Maximum control, minimum bullshit."</span></span>
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
            <p class="lede">"Describe the problem in your own words — we sell solutions, not tasks. The Listener + FSL turn it into a clear, priced job and ask only for what’s missing."</p>
            <div class="card">
                <textarea id="jobtext" placeholder="e.g. “Need a couch moved from my 2nd-floor apartment to a truck downstairs this afternoon.”"></textarea>
                <div class="row" style="margin-top:10px">
                    <button id="post" class="btn">"Post a Job"</button>
                    <button id="mic" class="mic" title="Speak with The Listener">"🎙"</button>
                </div>
                <div id="result" class="note"></div>
            </div>
            <div id="panel"></div>
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
                <button id="listener" class="btn green">"Go online"</button>
            </div>
            <div class="card">
                <h2 style="margin-top:0">"Shadow Intelligence"</h2>
                <ul id="hints" class="hints"></ul>
                <div id="fsl" class="trace"></div>
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

/// Platform/Engine view: run the FSL harness binary and return its output verbatim.
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

    let state = AppState::new();
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
        .route("/api/clarify", post(clarify))
        .route("/api/accept", post(accept))
        .route("/api/complete", post(complete))
        .route("/api/escalate", post(escalate))
        .nest_service("/icons", ServeDir::new(format!("{web_dir}/static/icons")))
        .with_state(state);

    let addr: SocketAddr = format!("{bind}:{port}").parse().expect("invalid bind address");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind");
    println!("Facilitator PWA (Leptos SSR + Axum + live FSL engine) listening on http://{addr}");
    axum::serve(listener, app).await.expect("server error");
}
