#![allow(dead_code)]
//! fsl-llm::facilitator — SPATIAL ROLE: OFF-AXIS ENGINE (applied). The rule-based
//! Facilitator cognition that turns a free-form problem into a structured job by driving
//! the FSL primitives: it CLASSIFIES intent into presets, detects which required
//! variables (slots) are present, and — for every missing slot — CALLS the inverted
//! Oracle on a freshly-assembled context to mint the UNK that The Listener will ask
//! about. It holds NO state across calls (identical input ⇒ identical output) and
//! returns only element arrays; the persistent truth lives in the FSL World (the Ledger).
//!
//! Mapping to the three documents + spatial methodology:
//!   • a job request is a CABLE (root node); the platform "sells solutions, not tasks".
//!   • each required slot is a STRAND: present → an OBS (pointable); missing → an UNK
//!     (a Listener question) awaiting a topic — resolved later via the Onion Shell.
//!   • a missing slot's UNK is produced by the Oracle exactly per the inverted interface
//!     (Where the Water Is Loud, F4 Unknowns()); pointability follows I-A1.
//!   • once every slot is anchored, the free-form request has crossed its Delta Bridge
//!     and standardizes into a one-tap PRESET (data-driven standardization).

use fsl_core::*;

// ─────────────────────────── presets (intent classes) ───────────────────────────
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset { Delivery, Roadside, Moving, Errand, Labor, FreeForm }

impl Preset {
    pub fn label(self) -> &'static str {
        match self {
            Preset::Delivery => "Delivery",
            Preset::Roadside => "Roadside assistance",
            Preset::Moving => "Moving help",
            Preset::Errand => "Errand",
            Preset::Labor => "Labor / muscle",
            Preset::FreeForm => "Custom request",
        }
    }
    /// machine key (stable; used by the web layer to evolve free-form → preset buttons).
    pub fn key(self) -> &'static str {
        match self {
            Preset::Delivery => "delivery",
            Preset::Roadside => "roadside",
            Preset::Moving => "moving",
            Preset::Errand => "errand",
            Preset::Labor => "labor",
            Preset::FreeForm => "free-form",
        }
    }
    /// The variables that must be fully defined before the job is computable. While any
    /// remain undefined the request stays free-form (an open UNK list blocks crossing).
    pub fn required_slots(self) -> &'static [&'static str] {
        match self {
            Preset::Delivery => &["pickup", "dropoff", "item", "when"],
            Preset::Roadside => &["location", "vehicle", "problem"],
            Preset::Moving => &["origin", "destination", "items", "when"],
            Preset::Errand => &["store", "items", "when"],
            Preset::Labor => &["location", "task", "when"],
            Preset::FreeForm => &["details"],
        }
    }
    pub fn price_band(self) -> &'static str {
        match self {
            Preset::Delivery => "$8–15",
            Preset::Roadside => "$40–90",
            Preset::Moving => "$60–200",
            Preset::Errand => "$12–25",
            Preset::Labor => "$25–60/hr",
            Preset::FreeForm => "quote after clarification",
        }
    }
}

/// Structural intent classification — token/substring membership only (NO regex).
pub fn classify(text: &str) -> Preset {
    let t = text.to_lowercase();
    let any = |ks: &[&str]| ks.iter().any(|k| t.contains(k));
    if any(&["tow", "roadside", "flat", "jump", "dead battery", "stuck", "locked out", "out of gas"]) {
        Preset::Roadside
    } else if any(&["move", "moving", "haul", "furniture", "couch"]) {
        Preset::Moving
    } else if any(&["deliver", "delivery", "food", "pickup", "pick up", "drop off", "dropoff"]) {
        Preset::Delivery
    } else if any(&["errand", "grocery", "groceries", "store run", "pharmacy", "shopping"]) {
        Preset::Errand
    } else if any(&["help", "labor", "lift", "assemble", "load", "unload", "yard", "clean"]) {
        Preset::Labor
    } else {
        Preset::FreeForm
    }
}

// ─────────────────────────── slot detection (structural cues) ───────────────────────────
fn detect(slot: &str, text: &str, toks: &[String]) -> Option<String> {
    let t = text.to_lowercase();
    let any = |ks: &[&str]| ks.iter().any(|k| t.contains(k));
    let has_num = toks.iter().any(|w| w.chars().any(|c| c.is_ascii_digit()));
    let yes = |s: &str| Some(s.to_string());
    match slot {
        "pickup" | "origin" => if any(&["from ", "pick up", "pickup", " at "]) { yes("specified") } else { None },
        "dropoff" | "destination" => if any(&[" to ", "drop off", "dropoff", "deliver"]) { yes("specified") } else { None },
        "item" | "items" => if any(&["food", "package", "box", "boxes", "couch", "furniture", "grocery", "groceries", "bag", "bags", "parcel", "meal", "order"]) { yes("noted") } else { None },
        "details" => if toks.len() >= 6 { yes("described") } else { None },
        "when" => if has_num || any(&["today", "tonight", "tomorrow", "now", "asap", " am", " pm", ":", "morning", "afternoon", "evening"]) { yes("scheduled") } else { None },
        "location" => if (has_num && any(&["st", "street", "ave", "avenue", "rd", "road", "blvd", "hwy", "highway", "lane", "dr", "drive"])) || any(&["on ", "near ", "exit", "mile"]) { yes("located") } else { None },
        "vehicle" => if any(&["car", "truck", "suv", "sedan", "van", "toyota", "honda", "ford", "chevy", "tesla", "bmw", "jeep"]) { yes("identified") } else { None },
        "problem" => if any(&["flat", "dead", "battery", "won't start", "wont start", "stuck", "tow", "jump", "overheat", "locked", "gas", "fuel"]) { yes("described") } else { None },
        "store" => if any(&["store", "walmart", "target", "costco", "grocery", "pharmacy", "restaurant", "market", "shop"]) { yes("named") } else { None },
        "task" => if any(&["lift", "move", "assemble", "clean", "load", "unload", "help", "carry", "mount", "install", "yard", "mow"]) { yes("described") } else { None },
        _ => None,
    }
}

/// Human phrasing for a slot's clarifying question (The Listener's prompt).
pub fn ask_for(slot: &str) -> &'static str {
    match slot {
        "pickup" => "Where should the driver pick up?",
        "origin" => "Where are we moving from?",
        "dropoff" => "Where is the drop-off?",
        "destination" => "Where are we moving to?",
        "item" => "What exactly needs to be delivered?",
        "items" => "What items are involved (size / count)?",
        "when" => "When do you need this — now, today, or scheduled?",
        "location" => "What's the exact location or address?",
        "vehicle" => "What vehicle is it (make / model)?",
        "problem" => "What's wrong with the vehicle?",
        "store" => "Which store should we go to?",
        "task" => "What's the task, exactly?",
        "details" => "Tell me a bit more — what's the problem?",
        _ => "Can you add a detail we can point at?",
    }
}

fn has_anchor(toks: &[String], text: &str) -> bool {
    let t = text.to_lowercase();
    toks.iter().any(|w| w.chars().any(|c| c.is_ascii_digit()))
        || ["st", "street", "ave", "rd", "road", "today", "tomorrow", " am", " pm", "now", "asap", "near "]
            .iter()
            .any(|k| t.contains(k))
}

// ─────────────────────────── the proposal (element arrays) ───────────────────────────
/// A missing slot, surfaced as a clarifying UNK (The Listener question). `would_become_obs`
/// is minted by the Oracle (F4 Unknowns()) so it is genuinely engine-derived.
#[derive(Clone, Debug)]
pub struct Clarify { pub slot: String, pub ask: String, pub would_become_obs: String }

/// The structured proposal returned to the caller. The engine holds nothing; this is a
/// pure function of the (freshly-assembled) input — the inverted interface.
#[derive(Clone, Debug)]
pub struct JobProposal {
    pub preset: Preset,
    pub title: String,
    pub price_band: String,
    pub present: Vec<(String, String)>, // anchored slots → OBS
    pub missing: Vec<Clarify>,          // open UNKs → Listener questions
    pub pointable: bool,                // I-A1: can anything be pointed at yet?
}

impl JobProposal {
    /// "Sell solutions, not tasks": fully-anchored, non-free-form ⇒ a standardized preset.
    pub fn is_standardized(&self) -> bool { self.missing.is_empty() && self.preset != Preset::FreeForm }
}

/// Stable topic id for a slot, so a missing-slot UNK and the later answering OBS pair up
/// across ticks (Onion Shell resolution). FNV-1a — deterministic, no external crate.
pub fn slot_topic(slot: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in slot.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(0x0000_0100_0000_01b3); }
    h
}

fn make_title(p: Preset, text: &str) -> String {
    let snip: Vec<&str> = text.split_whitespace().take(6).collect();
    let snip = snip.join(" ");
    if snip.is_empty() { p.label().to_string() } else { format!("{}: {}", p.label(), snip) }
}

/// THE ENGINE ENTRYPOINT. The caller (a function) CALLS this with raw text; it returns
/// element arrays (present OBS-slots + missing UNK-clarifies). For every missing slot it
/// assembles a FRESH `UnkContext` and calls `oracle.unknowns(..)` (inverted interface),
/// and it routes the request through `oracle.route(..)` for I-A1 pointability — so the
/// rule-based cognition runs *through* the Oracle, not around it. No state is retained.
pub fn structure_job(oracle: &dyn Oracle, text: &str) -> JobProposal {
    let preset = classify(text);
    let toks = tokenize(text);

    let mut present = vec![];
    let mut missing_slots: Vec<&str> = vec![];
    for &slot in preset.required_slots() {
        match detect(slot, text, &toks) {
            Some(v) => present.push((slot.to_string(), v)),
            None => missing_slots.push(slot),
        }
    }

    // Mint one UNK per missing slot by CALLING the Oracle on a freshly-assembled context.
    let claims: Vec<Claim> = missing_slots
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut c = Claim::plain(900_000 + i as u64, &format!("missing {s} for {}", preset.label()));
            c.topic = Some(TopicId(slot_topic(s)));
            c
        })
        .collect();
    let drafts = oracle.unknowns(&UnkContext { claims: &claims, root: RootCtx::default() });
    let missing: Vec<Clarify> = missing_slots
        .iter()
        .enumerate()
        .map(|(i, s)| Clarify {
            slot: (*s).to_string(),
            ask: ask_for(s).to_string(),
            would_become_obs: drafts
                .get(i)
                .map(|d| d.would_become_obs.clone())
                .unwrap_or_else(|| format!("OBS once {s} can be pointed at")),
        })
        .collect();

    // I-A1 pointability, decided by the Oracle's structural route (fresh context).
    let probe = if has_anchor(&toks, text) {
        Claim::pointable(900_999, text, Span { quote: text.into(), context: "job".into() }, slot_topic("request"), 1)
    } else {
        Claim::plain(900_999, text)
    };
    let routed = oracle.route(&RoutingContext {
        claims: std::slice::from_ref(&probe),
        invalid_policy: &InvalidPolicy::default(),
        invalid_pressure: 0.0,
        root: RootCtx::default(),
    });
    let pointable = routed.first().map(|r| matches!(r.routing, ProofRouting::Obs)).unwrap_or(false);

    JobProposal {
        preset,
        title: make_title(preset, text),
        price_band: preset.price_band().to_string(),
        present,
        missing,
        pointable,
    }
}
