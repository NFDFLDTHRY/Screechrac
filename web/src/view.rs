//! SPATIAL ROLE: THE FACE — Leptos SSR pages + PWA assets + client glue. Pure
//! presentation: it renders shells and calls the JSON API; all cognition is server-side
//! in FSL. Auth state is read from /api/me; promoted presets and shadow signals are read
//! from the API. No business logic lives here.

use axum::{
    http::header,
    response::{Html, IntoResponse, Response},
};
use leptos::*;
use tokio::process::Command;

const CSS: &str = r##"
:root{--bg:#0b0f17;--panel:#121826;--ink:#e6edf3;--muted:#8aa0b6;--accent:#ffb020;--green:#36d399;--line:#1e2a3b}
*{box-sizing:border-box}html,body{margin:0;min-height:100%}
body{background:radial-gradient(1200px 600px at 70% -10%,#13203a 0,var(--bg) 55%);color:var(--ink);
 font:16px/1.55 ui-sans-serif,system-ui,-apple-system,Segoe UI,Roboto,Inter,sans-serif;-webkit-font-smoothing:antialiased;
 padding-top:calc(env(safe-area-inset-top) + 56px);padding-bottom:74px}
.appbar{position:fixed;top:0;left:0;right:0;height:56px;display:flex;align-items:center;gap:10px;padding:0 16px;
 background:rgba(11,15,23,.86);backdrop-filter:blur(8px);border-bottom:1px solid var(--line);z-index:20}
.logo{font-weight:800}.logo b{color:var(--accent)}.tag{color:var(--muted);font-size:12px}
.who{margin-left:auto;font-size:12px;color:var(--muted)}
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
textarea,input,select{width:100%;background:#060a12;border:1px solid var(--line);border-radius:12px;color:var(--ink);padding:12px;font:15px/1.5 inherit}
textarea{min-height:120px;resize:vertical}
.mic{flex:0 0 56px;border-radius:12px;border:1px solid var(--line);background:#060a12;color:var(--ink);font-size:20px}
.mic.rec{background:#3a1020;border-color:#ff5d73;color:#ff9db0}
.note{color:var(--muted);font-size:13px;margin-top:10px;min-height:18px}
.banner{background:#241016;border:1px solid #5b1f2f;color:#ff9db0;border-radius:12px;padding:10px 12px;margin:10px 0}
.banner a{color:var(--accent)}
.pill{display:inline-block;font-size:12px;font-weight:700;border-radius:999px;padding:4px 10px;border:1px solid var(--line)}
.pill.on{background:#0f2a1d;color:var(--green);border-color:#1f5b3f}.pill.off{background:#241016;color:#ff9db0}
.pill.amber{background:#2a210f;color:var(--accent);border-color:#5b481f}
.chips{display:flex;gap:8px;flex-wrap:wrap}
.chip{border:1px solid var(--line);background:#0a0f1a;border-radius:999px;padding:8px 12px;color:var(--ink);cursor:pointer;font-weight:700}
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
function me(){return fetch('/api/me').then(function(r){return r.ok?r.json():null;});}
function whoami(){var el=document.getElementById('who');if(!el)return;me().then(function(m){el.innerHTML=m?(esc(m.username)+' · '+esc(m.role)+' · <a href="#" onclick="logout()">log out</a>'):'<a href="/login">Log in / Register</a>';});}
function logout(){fetch('/api/logout',{method:'POST'}).then(function(){location.href='/';});return false;}
"##;

const HOME_JS: &str = r##"whoami();"##;

const LOGIN_JS: &str = r##"
(function(){
 var u=document.getElementById('u'),p=document.getElementById('p'),role=document.getElementById('role'),
     res=document.getElementById('res');
 function go(path,body){res.textContent='…';
   fetch(path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})
    .then(function(r){return r.ok?r.json():r.text().then(function(t){throw new Error(t);});})
    .then(function(m){location.href=(m.role==='driver')?'/driver':(m.role==='admin'?'/admin':'/customer');})
    .catch(function(e){res.textContent=''+e;});}
 document.getElementById('login').onclick=function(){go('/api/login',{username:u.value,password:p.value});};
 document.getElementById('register').onclick=function(){go('/api/register',{username:u.value,password:p.value,role:role.value});};
 whoami();
})();
"##;

const CUSTOMER_JS: &str = r##"
(function(){
 var t=document.getElementById('jobtext'),mic=document.getElementById('mic'),post=document.getElementById('post'),
     res=document.getElementById('result'),panel=document.getElementById('panel'),presetsEl=document.getElementById('presets'),
     guard=document.getElementById('guard');
 whoami();
 me().then(function(m){if(!m||m.role!=='customer'){guard.innerHTML='<div class="banner">You need a <b>customer</b> account to post jobs. <a href="/login">Log in / Register</a></div>';}});
 var SR=window.SpeechRecognition||window.webkitSpeechRecognition;
 if(mic){mic.addEventListener('click',function(){
   if(!SR){res.textContent='The Listener (voice) needs Chrome/Android. Type instead.';return;}
   var r=new SR();r.lang='en-US';r.interimResults=false;mic.classList.add('rec');res.textContent='The Listener is listening…';
   r.onresult=function(e){var s=e.results[0][0].transcript;t.value=(t.value?t.value+' ':'')+s;res.textContent='Captured: '+s;};
   r.onerror=function(e){res.textContent='Listener error: '+e.error;};r.onend=function(){mic.classList.remove('rec');};r.start();
 });}
 function render(j){
   var html='<div class="card"><div><b>'+esc(j.title)+'</b> <span class="pill amber">'+esc(j.preset)+'</span></div>'+
     '<div class="lede" style="margin:8px 0">Price band: '+esc(j.price_band)+'</div><div class="trace">'+trace(j)+'</div>';
   if(j.status==='ready'){html+='<p style="margin-top:12px">✅ <b>Job ready.</b> Drivers can accept it now.</p>';}
   else{html+='<h2>The Listener needs a few details</h2>';
     (j.open_questions||[]).forEach(function(q){
       html+='<div class="q"><div class="ask">'+esc(q.ask)+'</div><div class="row"><input id="a_'+q.slot+'" placeholder="Type or speak your answer"/>'+
         '<button class="btn green" style="flex:0 0 110px" onclick="answer('+j.id+',\''+q.slot+'\')">Answer</button></div></div>';});}
   html+='</div>';panel.innerHTML=html;
 }
 window.answer=function(id,slot){var el=document.getElementById('a_'+slot);var v=(el&&el.value||'').trim();if(!v)return;
   fetch('/api/clarify',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({job_id:id,slot:slot,answer:v})})
    .then(function(r){return r.json();}).then(render).catch(function(e){res.textContent='Error: '+e;});};
 if(post){post.addEventListener('click',function(){var text=(t.value||'').trim();if(!text){res.textContent='Describe the problem first.';return;}
   post.disabled=true;res.textContent='Routing through the FSL engine…';
   fetch('/api/job',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({text:text})})
    .then(function(r){return r.ok?r.json():r.text().then(function(t){throw new Error(t);});})
    .then(function(j){res.textContent='Posted job #'+j.id+'.';t.value='';render(j);})
    .catch(function(e){res.textContent='Error: '+e;}).finally(function(){post.disabled=false;});});}
 // one-tap promoted presets (data-driven standardization)
 window.tapPreset=function(kind){fetch('/api/presets').then(function(r){return r.json();}).then(function(list){
   var p=list.find(function(x){return x.kind===kind;});if(!p)return;
   var html='<div class="card"><b>'+esc(p.label)+'</b> <span class="pill amber">one-tap preset</span><div class="lede">Fill the known variables — posts as ready.</div>';
   p.slots.forEach(function(s){html+='<div class="q"><div class="ask">'+esc(s.ask)+'</div><input id="v_'+s.slot+'"/></div>';});
   html+='<button class="btn green" onclick="submitPreset(\''+kind+'\')">Post '+esc(p.label)+'</button></div>';
   panel.innerHTML=html;
 });};
 window.submitPreset=function(kind){fetch('/api/presets').then(function(r){return r.json();}).then(function(list){
   var p=list.find(function(x){return x.kind===kind;});var values=[];
   p.slots.forEach(function(s){var el=document.getElementById('v_'+s.slot);values.push([s.slot,(el&&el.value||'').trim()]);});
   fetch('/api/preset_job',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({kind:kind,values:values})})
    .then(function(r){return r.json();}).then(render).catch(function(e){res.textContent='Error: '+e;});});};
 function loadPresets(){fetch('/api/presets').then(function(r){return r.json();}).then(function(list){
   if(!list.length){presetsEl.innerHTML='';return;}
   presetsEl.innerHTML='<h2>One-tap presets</h2><div class="chips">'+list.map(function(p){return '<button class="chip" onclick="tapPreset(\''+p.kind+'\')">'+esc(p.label)+'</button>';}).join('')+'</div>';});}
 loadPresets();
})();
"##;

const DRIVER_JS: &str = r##"
(function(){
 var jobsEl=document.getElementById('jobs'),hintsEl=document.getElementById('hints'),fslEl=document.getElementById('fsl'),
     lbtn=document.getElementById('listener'),lstat=document.getElementById('lstat'),qr=document.getElementById('qr'),
     guard=document.getElementById('guard'),modal=document.getElementById('modal'),mbody=document.getElementById('mbody'),mclose=document.getElementById('mclose');
 whoami();
 me().then(function(m){if(!m||m.role!=='driver'){guard.innerHTML='<div class="banner">You need a <b>driver</b> account to take jobs. <a href="/login">Log in / Register</a></div>';}});
 function showModal(h){mbody.innerHTML=h;modal.style.display='flex';}
 if(mclose)mclose.addEventListener('click',function(){modal.style.display='none';});
 var online=localStorage.getItem('fsl_listener')==='1';
 function rl(){if(lstat){lstat.textContent=online?'ONLINE — The Listener is active':'OFFLINE';lstat.className='pill '+(online?'on':'off');}if(lbtn)lbtn.textContent=online?'Go offline':'Go online';}
 if(lbtn)lbtn.addEventListener('click',function(){online=!online;localStorage.setItem('fsl_listener',online?'1':'0');rl();});rl();
 if(qr)qr.addEventListener('click',function(){showModal('<h3>Restaurant QR pipeline</h3><p><b>(preview)</b> Scanning a restaurant QR will open a preset pickup job. Deferred to a later PR.</p>');});
 window.act=function(id,kind){var url=kind==='accept'?'/api/accept':(kind==='complete'?'/api/complete':'/api/escalate');
   fetch(url,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({job_id:id})})
    .then(function(r){return r.ok?r.json():r.text().then(function(t){throw new Error(t);});})
    .then(function(j){if(kind==='escalate'){showModal('<h3>Customer Mode</h3><p>'+esc(j.note)+'</p><div class="trace">FSL · bridge '+esc(j.bridge)+' · stage '+esc(j.stage)+' · behavior '+esc(j.behavior)+'</div>');}
      else if(kind==='complete'){showModal('<h3>Job #'+id+' completed</h3><div class="trace">payment: '+esc(j.payment)+'</div>');}
      load();})
    .catch(function(e){showModal('<p>Error: '+esc(e)+'</p>');});};
 function card(j){var a='';
   if(j.status==='ready')a='<button class="btn green" onclick="act('+j.id+',\'accept\')">Accept</button>';
   else if(j.status==='accepted')a='<div class="row"><button class="btn" onclick="act('+j.id+',\'escalate\')">Customer Mode</button><button class="btn green" onclick="act('+j.id+',\'complete\')">Complete</button></div>';
   else if(j.status==='completed')a='<span class="pill on">completed</span>';
   else a='<span class="pill amber">awaiting clarification</span>';
   return '<div class="job"><div class="jt">#'+j.id+' · '+esc(j.preset)+' · '+esc(j.price_band)+'</div><div class="jx">'+esc(j.title)+'</div><div class="jm">'+trace(j)+'</div>'+a+'</div>';}
 function load(){
   fetch('/api/jobs?limit=50').then(function(r){return r.json();}).then(function(jobs){
     var v=jobs.filter(function(j){return j.status!=='completed';});
     jobsEl.innerHTML=v.length?v.map(card).join(''):'<div class="empty">No live jobs yet. Stay online so The Listener can catch new ones.</div>';});
   fetch('/api/shadow').then(function(r){return r.json();}).then(function(s){
     hintsEl.innerHTML=(s.hints||[]).map(function(h){return '<li>'+esc(h)+'</li>';}).join('');
     if(fslEl)fslEl.textContent='FSL: '+s.fsl.cables+' cables · '+s.fsl.strands+' strands · '+s.fsl.flows+' flows · open '+s.fsl.open_unks+' · resolved '+s.fsl.resolved_unks+' · coherent '+s.fsl.coherent;});
 }
 load();setInterval(load,4000);
})();
"##;

const ADMIN_JS: &str = r##"
(function(){var b=document.getElementById('walk'),o=document.getElementById('out');whoami();if(!b||!o)return;
 b.addEventListener('click',function(){b.disabled=true;o.textContent='Running the FSL harness…';
  fetch('/api/walk').then(function(r){return r.ok?r.text():r.text().then(function(t){throw new Error(t);});}).then(function(t){o.textContent=t;})
   .catch(function(e){o.textContent='Error: '+e;}).finally(function(){b.disabled=false;});});})();
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
const CACHE='facilitator-v4';
const SHELL=['/', '/customer', '/driver', '/login', '/manifest.webmanifest', '/icons/icon-192.png', '/icons/icon-512.png'];
self.addEventListener('install',function(e){e.waitUntil(caches.open(CACHE).then(function(c){return c.addAll(SHELL);}).then(function(){return self.skipWaiting();}));});
self.addEventListener('activate',function(e){e.waitUntil(caches.keys().then(function(k){return Promise.all(k.filter(function(x){return x!==CACHE;}).map(function(x){return caches.delete(x);}));}).then(function(){return self.clients.claim();}));});
self.addEventListener('fetch',function(e){var u=new URL(e.request.url);
 if(u.pathname.indexOf('/api/')===0){e.respondWith(fetch(e.request).catch(function(){return new Response('offline',{status:503});}));return;}
 e.respondWith(caches.match(e.request).then(function(r){return r||fetch(e.request);}));});
"##;

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
<header class=\"appbar\"><span class=\"logo\">⚡ <b>Facilitator</b></span><span class=\"tag\">powered by FSL</span><span id=\"who\" class=\"who\"></span></header>\
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

pub async fn index() -> Html<String> {
    let body = leptos::ssr::render_to_string(|| {
        view! {
            <h1>"You have a problem. We fix it."</h1>
            <p class="lede">"A driver-first local problem-solving marketplace, powered by the FSL cognitive engine."</p>
            <a class="card role" href="/customer"><span class="emoji">"🧩"</span><span><span class="t">"I need something done"</span><br/><span class="d">"Describe a problem — The Listener + FSL turn it into a clear, priced job."</span></span></a>
            <a class="card role" href="/driver"><span class="emoji">"🚗"</span><span><span class="t">"I drive / I work"</span><br/><span class="d">"Live jobs, shadow-intelligence from FSL, The Listener."</span></span></a>
            <a class="card role" href="/admin"><span class="emoji">"⚙️"</span><span><span class="t">"Platform / Engine"</span><br/><span class="d">"Walk the FSL scene graph + compliance scan."</span></span></a>
        }
    })
    .to_string();
    Html(document("Facilitator — You have a problem. We fix it.", body, HOME_JS))
}

pub async fn login_page() -> Html<String> {
    let body = leptos::ssr::render_to_string(|| {
        view! {
            <h1>"Log in or create an account"</h1>
            <p class="lede">"Real accounts so we know who the customer and driver are (required for payments later)."</p>
            <div class="card">
                <input id="u" placeholder="username"/>
                <input id="p" type="password" placeholder="password" style="margin-top:8px"/>
                <select id="role" style="margin-top:8px"><option value="customer">"Customer (post jobs)"</option><option value="driver">"Driver (take jobs)"</option><option value="admin">"Admin"</option></select>
                <div class="row" style="margin-top:10px"><button id="login" class="btn">"Log in"</button><button id="register" class="btn green">"Register"</button></div>
                <div id="res" class="note"></div>
            </div>
        }
    })
    .to_string();
    Html(document("Facilitator — Sign in", body, LOGIN_JS))
}

pub async fn customer() -> Html<String> {
    let body = leptos::ssr::render_to_string(|| {
        view! {
            <h1>"Post a Job"</h1>
            <div id="guard"></div>
            <div id="presets"></div>
            <p class="lede">"Describe the problem in your own words — we sell solutions, not tasks."</p>
            <div class="card">
                <textarea id="jobtext" placeholder="e.g. “Need a couch moved from my 2nd-floor apartment to a truck this afternoon.”"></textarea>
                <div class="row" style="margin-top:10px"><button id="post" class="btn">"Post a Job"</button><button id="mic" class="mic" title="Speak with The Listener">"🎙"</button></div>
                <div id="result" class="note"></div>
            </div>
            <div id="panel"></div>
        }
    })
    .to_string();
    Html(document("Facilitator — Post a Job", body, CUSTOMER_JS))
}

pub async fn driver() -> Html<String> {
    let body = leptos::ssr::render_to_string(|| {
        view! {
            <h1>"Driver dashboard"</h1>
            <div id="guard"></div>
            <p class="lede">"Driver-first: no forced batches, no map hijacking, no hidden information."</p>
            <div class="card"><h2 style="margin-top:0">"The Listener"</h2><div><span id="lstat" class="pill off">"OFFLINE"</span></div>
                <p class="lede" style="margin:10px 0">"Always-on voice agent that wakes when you go online."</p><button id="listener" class="btn green">"Go online"</button></div>
            <div class="card"><h2 style="margin-top:0">"Shadow Intelligence"</h2><ul id="hints" class="hints"></ul><div id="fsl" class="trace"></div></div>
            <h2>"Available jobs"</h2><div id="jobs"></div>
            <button id="qr" class="btn ghost" style="margin-top:8px">"📷  Scan restaurant QR (preview)"</button>
        }
    })
    .to_string();
    Html(document("Facilitator — Drive", body, DRIVER_JS))
}

pub async fn admin() -> Html<String> {
    let body = leptos::ssr::render_to_string(|| {
        view! {
            <h1>"Platform / FSL Engine"</h1>
            <p class="lede">"Facilitator is a thin layer over the FSL cognitive engine. The core is unchanged."</p>
            <div class="card"><button id="walk" class="btn">"Walk the 3D scene graph"</button>
                <pre id="out" class="out">"Press “Walk” to run the FSL harness (scene tour + compliance)."</pre></div>
        }
    })
    .to_string();
    Html(document("Facilitator — Engine", body, ADMIN_JS))
}

pub async fn manifest() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/manifest+json")], MANIFEST)
}
pub async fn service_worker() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript"),
            (header::CACHE_CONTROL, "no-cache"),
            (header::HeaderName::from_static("service-worker-allowed"), "/"),
        ],
        SW,
    )
}
pub async fn health() -> &'static str { "ok" }

/// Platform/Engine view: run the FSL harness binary and return its output verbatim.
pub async fn walk() -> Response {
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
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("could not run the FSL harness ({bin}): {e}")).into_response(),
    }
}
