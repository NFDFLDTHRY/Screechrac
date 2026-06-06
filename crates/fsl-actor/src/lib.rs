#![allow(dead_code)]
//! fsl-actor — ACTOR STRANDS: Human Cognitive Compiler Architecture, OPERATIONAL (placed on each exploded plane).
//! SPATIAL ROLE: ACTOR-STRAND STRUCTURES — HCC-A nodes laid as Cubes/Spheres on each exploded stage plane.
//! Every named node is a type with its source quote. RIC ∥ PFC are parallel and BOTH
//! feed the Compiler under 4.4 arbitration. The Story Ledger is NOT narrative form.
//! Emotion is concurrent "meaning-in-flight" WITH an explicit stall path. Both loops
//! exist (outer Behavior→Reality, inner ISL→template entrench/weaken/split). The full
//! parameter space 4.1–4.7 is read and STEERS behavior in every stage.

use fsl_core::*;
use fsl_flow::bridge::{BridgeState, repair_sentence};

// ─────────────────────── Pruning levels (4.3) ───────────────────────
/// Three loci of pruning; "where most data loss happens." Levels map to drop
/// fractions (illustrative magnitudes, not a decided parameter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PruneLevel { None, Low, Medium, High }
impl PruneLevel {
    pub fn fraction(self) -> f32 { match self { PruneLevel::None => 0.0, PruneLevel::Low => 0.1, PruneLevel::Medium => 0.34, PruneLevel::High => 0.66 } }
    /// keep ceil((1-frac)*n) items, deterministically (drop from the tail).
    fn keep_count(self, n: usize) -> usize { ((1.0 - self.fraction()) * n as f32).ceil() as usize }
}

// ─────────────────────── Reality and Interface ───────────────────────
/// Reality (R): "Objective, constraint-bearing environment. Exists independent of any
/// observer. Not directly accessible by any mind."
#[derive(Clone, Debug, Default)]
pub struct Reality { pub events: Vec<Claim> }

/// Interface (I): "Biological and perceptual system ... Converts Reality → raw
/// internal signals." Perceptual pruning (4.3, S0) is applied HERE — the boundary is
/// lossy, not an identity copy.
#[derive(Clone, Debug, Default)]
pub struct Interface { inbox: Vec<Claim> }
#[derive(Clone, Debug, Default)]
pub struct RawSignals { pub claims: Vec<Claim>, pub perceptually_pruned: u64 }
impl Interface {
    pub fn convert(&self, r: &Reality, perceptual: PruneLevel) -> RawSignals {
        let keep = perceptual.keep_count(r.events.len());
        RawSignals { claims: r.events.iter().take(keep).cloned().collect(),
                     perceptually_pruned: (r.events.len() - keep) as u64 }
    }
    pub fn inject(&mut self, s: StructurePayload) { self.inbox.extend(s.claims); }
    pub fn drain(&mut self) -> Vec<Claim> { std::mem::take(&mut self.inbox) }
}

// ─────────────────────── Dual input channels (PARALLEL) ───────────────────────
/// RIC – Raw Input Channel: "preserve external structure as faithfully as possible."
#[derive(Clone, Debug, Default)]
pub struct Ric { pub structured: Vec<Claim> }
/// PFC – Pre-Filter Channel: "apply pre-existing schemas and assign probable
/// interpretations before full compilation." Reads the 4.1 Template Library.
#[derive(Clone, Debug, Default)]
pub struct Pfc { pub interpretations: Vec<InterpretationDraft> }
/// §2.1: "RIC and PFC run in parallel, not serially. Both feed the compiler."
#[derive(Clone, Debug, Default)]
pub struct Intake { pub ric: Ric, pub pfc: Pfc, pub prefilter_pruned: u64 }

/// PFC consults the Oracle (consult-only) with the schema library as fresh context.
pub fn pfc_interpret(oracle: &dyn Oracle, claims: &[Claim], templates: &TemplateLibrary, root: RootCtx) -> Pfc {
    let schemas: Vec<String> = templates.schemas.iter().map(|s| s.name.clone()).collect();
    let ctx = PfcContext { claims, schemas: &schemas, root };
    Pfc { interpretations: oracle.interpret(&ctx) }
}
/// Prefilter pruning (4.3, S1): "PFC discards inputs that don't fit templates."
pub fn prefilter_prune(intake: &mut Intake, level: PruneLevel) {
    let n = intake.ric.structured.len();
    let keep = level.keep_count(n);
    intake.prefilter_pruned = (n - keep) as u64;
    intake.ric.structured.truncate(keep);
}

// ─────────────────────── Compiler and Story Ledger ───────────────────────
/// Compiler (C): "Mechanism that converts structure → story." Inputs {RIC, PFC}.
#[derive(Clone, Debug, Default)]
pub struct Compiler;
/// Story Ledger (L): "Ledger is not narrative form. It's the underlying data the mind
/// uses to LATER construct narrative." Operations addition / merging / re-indexing /
/// deletion are all real. Narrative is a later, separate construction over L.
#[derive(Clone, Debug, Default)]
pub struct StoryLedger { pub events: Vec<Claim>, pub clusters: Vec<Vec<ClaimId>> }
impl StoryLedger {
    pub fn add(&mut self, c: Claim) { self.events.push(c); }                              // addition
    pub fn merge(&mut self, topic: Option<TopicId>) {                                     // merging (grouping)
        let group: Vec<ClaimId> = self.events.iter().filter(|c| c.topic == topic).map(|c| c.id).collect();
        if group.len() > 1 { self.clusters.push(group); }
    }
    pub fn reindex(&mut self) { self.clusters.sort_by_key(|g| std::cmp::Reverse(g.len())); } // re-indexing
    pub fn delete(&mut self, id: ClaimId) { self.events.retain(|c| c.id != id); }          // deletion/suppression
}
impl Compiler {
    /// {RIC, PFC} → Story Ledger under 4.4 arbitration. PFC interpretations that the
    /// arbitration lets win are added as derived claims; conflicts resolved by axis.
    pub fn compile(&self, intake: &Intake, l: &mut StoryLedger, arb: Arbitration, next_id: &mut u64) {
        for c in &intake.ric.structured { l.add(c.clone()); }
        // PFC contributes interpretations; whether they override raw depends on 4.4.
        let pfc_wins = matches!(arb.raw_vs_template, RawVsTemplate::TemplateWins | RawVsTemplate::IdentityWins);
        if pfc_wins {
            for interp in &intake.pfc.interpretations {
                let id = ClaimId(*next_id); *next_id += 1;
                l.add(Claim { id, text: format!("[pfc:{}] p={:.2}", interp.schema, interp.probability),
                              pointer: None, topic: None, stance: None });
            }
        }
        // grouping + reindex are part of compilation's ledger maintenance.
        l.merge(None); l.reindex();
    }
}
/// Post-compile pruning (4.3, S3–S5): "revise or erase uncomfortable ledger entries."
pub fn postcompile_prune(l: &mut StoryLedger, level: PruneLevel) -> u64 {
    let n = l.events.len(); let keep = level.keep_count(n);
    let pruned = (n - keep) as u64; l.events.truncate(keep); pruned
}

// ─────────────────────── Meaning Engine ───────────────────────
/// Meaning Engine (M): "Weight system + transition logic operating on the ledger."
/// "Meaning is directive. It doesn't just label; it influences what happens next."
#[derive(Clone, Debug, Default)]
pub struct MeaningEngine;
/// "Transitions: what should change: beliefs, priorities, roles, future expectations,
/// allowed behaviors." (the exact five-set; all five carried.)
#[derive(Clone, Debug)]
pub struct Transition {
    pub id: TransitionId,
    pub beliefs: Vec<String>,
    pub priorities: Vec<String>,
    pub roles: Vec<String>,
    pub future_expectations: Vec<String>,
    pub allowed_behaviors: Vec<String>,
    pub weight: f32,
}
impl MeaningEngine {
    /// Calls the Oracle (consult-only) for Transition arrays; style (4.5) and the
    /// priority ORDER (4.2) are passed as fresh context and steer the result.
    pub fn run(&self, oracle: &dyn Oracle, l: &StoryLedger, priority_order: &[&str], style: MeaningStyle, root: RootCtx) -> Vec<Transition> {
        let ctx = MeaningContext { ledger: &l.events, priority_order, style: style.tag(), root };
        oracle.meaning(&ctx).into_iter().enumerate().map(|(i, d)| Transition {
            id: TransitionId(i as u64 + 1),
            beliefs: d.beliefs, priorities: d.priorities, roles: d.roles,
            future_expectations: d.future_expectations, allowed_behaviors: d.allowed_behaviors,
            weight: d.weight,
        }).collect()
    }
}

// ─────────────────────── Emotion Engine (meaning-in-flight + STALL) ───────────────────────
/// Emotion Engine (E): "Meaning-in-flight. The runtime state of the system WHILE the
/// Meaning Engine is applying transitions." "Emotions vanish or settle when
/// transitions complete OR stall." Both outcomes are modeled; E is the ambient state
/// held ACROSS the application steps, not a stage computed after M finishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settle { InFlight, Completed, Stalled }
#[derive(Clone, Copy, Debug)]
pub struct EmotionState { pub during: Option<TransitionId>, pub settle: Settle, pub mode: EmotionMode }
impl EmotionState {
    pub fn idle(mode: EmotionMode) -> Self { EmotionState { during: None, settle: Settle::Completed, mode } }
}

/// What applying a transition produced, including the emotion-mode consequence.
#[derive(Clone, Debug, Default)]
pub struct ApplyResult {
    pub applied_steps: u32,
    pub stalled: bool,
    pub priority_reorder: Vec<String>,   // applied to the priority stack (directive)
    pub mode_consequence: ModeConsequence,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ModeConsequence {
    #[default] None,
    MustAct,            // Mode D
    RewriteStory,       // Mode C
    ReweightImportance, // Mode B
}

/// Apply a Transition as a STEPPED process. Emotion is `InFlight` for the whole loop
/// (concurrent with application); it resolves to Completed or Stalled. The 4.6 mode
/// determines the consequence. A step STALLS when a belief change is demanded but no
/// behavior is permitted (the "floating" case) or identity is Coherent and resists.
pub fn apply_transition(t: &Transition, emotion: &mut EmotionState, identity_mode: IdentityMode, priorities: &mut Vec<Priority>) -> ApplyResult {
    emotion.during = Some(t.id);
    emotion.settle = Settle::InFlight; // meaning-in-flight WHILE applying
    let mut res = ApplyResult::default();
    // steps = the non-empty dimensions of the five-set.
    let steps: Vec<&str> = [
        (!t.beliefs.is_empty()).then_some("beliefs"),
        (!t.priorities.is_empty()).then_some("priorities"),
        (!t.roles.is_empty()).then_some("roles"),
        (!t.future_expectations.is_empty()).then_some("future_expectations"),
        (!t.allowed_behaviors.is_empty()).then_some("allowed_behaviors"),
    ].into_iter().flatten().collect();
    for step in steps {
        // stall condition: changing beliefs with nothing permitted, under Coherent identity.
        if step == "beliefs" && t.allowed_behaviors.is_empty() && matches!(identity_mode, IdentityMode::Coherent) {
            res.stalled = true; break;
        }
        if step == "priorities" {
            // directive: meaning REORDERS the priority stack (it "influences what happens next").
            res.priority_reorder = t.priorities.clone();
            apply_priority_reorder(priorities, &t.priorities);
        }
        res.applied_steps += 1;
    }
    emotion.settle = if res.stalled { Settle::Stalled } else { Settle::Completed };
    emotion.during = None;
    res.mode_consequence = match emotion.mode {
        EmotionMode::DecisionDriving => ModeConsequence::MustAct,
        EmotionMode::StoryRewriting => ModeConsequence::RewriteStory,
        EmotionMode::Reweighting => ModeConsequence::ReweightImportance,
        EmotionMode::Ignorable => ModeConsequence::None,
    };
    res
}
/// Move named priorities to the front of the stack, preserving the rest (directive 4.2).
fn apply_priority_reorder(stack: &mut Vec<Priority>, names: &[String]) {
    let mut front: Vec<Priority> = vec![];
    for n in names {
        if let Some(p) = Priority::from_name(n) {
            if let Some(pos) = stack.iter().position(|x| *x == p) { front.push(stack.remove(pos)); }
        }
    }
    front.extend(stack.drain(..));
    *stack = front;
}

// ─────────────────────── Identity Shaping Layer (inner loop) ───────────────────────
/// ISL: "Compression/synchronization layer where Story + Meaning + repeated emotional
/// patterns integrate into Identity." Output: identity snapshot AND "Updated
/// rules/priors for future compilation" (the INNER loop: entrench / weaken / split).
#[derive(Clone, Debug, Default)]
pub struct Isl;
#[derive(Clone, Debug, Default)]
pub struct IdentitySnapshot { pub summary: String }
/// ACT VI / inner loop: "templates entrenched / weakened / split."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateUpdate { Entrench(TemplateId), Weaken(TemplateId), Split(TemplateId) }
impl Isl {
    /// Chooses entrench/weaken/split from the REPEATED emotional pattern history and
    /// the 4.7 identity mode — not a constant.
    pub fn integrate(&self, l: &StoryLedger, emotion_history: &[Settle], identity_mode: IdentityMode, t: TemplateId) -> (IdentitySnapshot, TemplateUpdate) {
        let completed = emotion_history.iter().filter(|s| matches!(s, Settle::Completed)).count();
        let stalled = emotion_history.iter().filter(|s| matches!(s, Settle::Stalled)).count();
        let update = match identity_mode {
            // Sovereign/Coherent: confident consolidation when patterns complete.
            _ if stalled > completed => TemplateUpdate::Weaken(t),
            IdentityMode::Fragmented | IdentityMode::Scripted if stalled > 0 && completed > 0 => TemplateUpdate::Split(t),
            IdentityMode::Sovereign | IdentityMode::Coherent => TemplateUpdate::Entrench(t),
            _ => if completed >= stalled { TemplateUpdate::Entrench(t) } else { TemplateUpdate::Weaken(t) },
        };
        let snap = IdentitySnapshot { summary: format!("identity[{:?}] @{} events (compl {}, stall {})", identity_mode, l.events.len(), completed, stalled) };
        (snap, update)
    }
}

// ─────────────────────── Behavior ───────────────────────
/// Behavior Output (B): "External, observable actions ... words spoken or withheld,
/// physical actions, silence / freeze, jokes, escalation, retreat, attack, fawn."
/// "Behavior is the only directly observable part of the loop."
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Behavior {
    Directive(String), NonDirective(String),
    Withhold,          // "words ... withheld"
    SilenceFreeze,     // "silence / freeze"
    Joke,              // "jokes"
    Escalation,        // "escalation"
    Retreat,           // "retreat"
    Attack,            // "attack"
    Fawn,              // "fawn"
    Repair(String),    // route-back when a directive can't legitimately be pushed
}
pub struct BehaviorIntent { pub text: String, pub directive: bool }
/// Directive legitimacy is gated on the bridge (STATE 4). The full taxonomy is
/// SELECTED using emotion-mode consequence + 4.4 identity-vs-reality + bridge state —
/// so Withhold/Escalation/Fawn/etc. are reachable, not dead variants.
pub fn emit(intent: BehaviorIntent, bridge: BridgeState, mode_consequence: ModeConsequence, arb: Arbitration, invalid_pressure: f32) -> Behavior {
    if intent.directive {
        if bridge.directives_legitimate() { return Behavior::Directive(intent.text); }
        // FM-A2 "Directive without premises → floating force vector; 'can't speak here.'"
        return Behavior::Repair(repair_sentence().to_string());
    }
    match (mode_consequence, arb.identity_vs_reality) {
        (ModeConsequence::MustAct, _) if invalid_pressure >= 0.5 => Behavior::Escalation,
        (ModeConsequence::MustAct, IdentityVsReality::RealityYields) => Behavior::Attack,
        (_, IdentityVsReality::IdentityYields) => if invalid_pressure >= 0.5 { Behavior::Fawn } else { Behavior::Retreat },
        (ModeConsequence::ReweightImportance, _) => Behavior::NonDirective(intent.text),
        (ModeConsequence::None, _) if !bridge.directives_legitimate() => Behavior::Withhold,
        _ => Behavior::NonDirective(intent.text),
    }
}

// ─────────────────────── Feedback (outer loop) ───────────────────────
/// Feedback: "Behavior alters Reality / environment. New events become new structure.
/// Loop restarts."
#[derive(Clone, Debug, Default)]
pub struct Feedback { pub new_events: Vec<Claim> }

// ─────────────────────── State machine S0–S7 + Six-Act projection ───────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HccaStage {
    /// S0 Intake: "Interface receives signals from Reality. RIC gets raw structured
    /// events. PFC applies priors/templates to the same events."
    Intake,
    /// S1 Coexistence & Early Pruning: "RIC and PFC both active. Early filtering."
    CoexistPrune,
    /// S2 Compilation (C): "C takes {RIC, PFC} → updates the Story Ledger (L)."
    Compile,
    /// S3 Meaning (M): "What matters here? How much? What must change?"
    Meaning,
    /// S4 Emotion Runtime (E): "the felt 'meaning in motion' state."
    Emotion,
    /// S5 Identity Update (ISL): "'Who am I now, given this?'"
    Identity,
    /// S6 Behavior (B): "Action chosen based on: Identity, Meaning, Emotion, Active
    /// constraints/templates."
    Behavior,
    /// S7 Feedback: "Behavior changes outer structure. New conditions feed back into
    /// S0. Loop repeats."
    Feedback,
}
/// Six-Act stack — a NON-uniform projection of S0–S7 (Act III = S3+S4; Act VI =
/// template mutation). The projection is real, not a comment.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act { SceneAndStakes, StoryUpdate, MeaningAndEmotion, IdentityIntegration, BehaviorEmission, WorldModelAndTemplateUpdate }
pub fn act_of(stage: HccaStage) -> Act {
    match stage {
        HccaStage::Intake => Act::SceneAndStakes,                       // ACT I "What world am I in right now?"
        HccaStage::CoexistPrune | HccaStage::Compile => Act::StoryUpdate, // ACT II "What just got added to my story?"
        HccaStage::Meaning | HccaStage::Emotion => Act::MeaningAndEmotion, // ACT III "What does this mean for me?"
        HccaStage::Identity => Act::IdentityIntegration,                // ACT IV "What does this say about who I am?"
        HccaStage::Behavior => Act::BehaviorEmission,                   // ACT V "What do I do now?"
        HccaStage::Feedback => Act::WorldModelAndTemplateUpdate,        // ACT VI "What did this teach me about the world?"
    }
}

// ─────────────────────── Parameter space (4.1–4.7) ───────────────────────
/// 4.1 Template / Schema Library (PFC content).
#[derive(Clone, Debug)] pub struct Schema { pub name: String, pub weight: f32 }
#[derive(Clone, Debug, Default)]
pub struct TemplateLibrary { pub schemas: Vec<Schema>, pub override_raw_input: bool }
impl TemplateLibrary {
    pub fn apply_update(&mut self, u: TemplateUpdate) {
        match u {
            // entrench/weaken adjust schema WEIGHTS (real priors), not a bare bool.
            TemplateUpdate::Entrench(_) => { self.override_raw_input = true;  for s in &mut self.schemas { s.weight = (s.weight + 0.1).min(1.0); } }
            TemplateUpdate::Weaken(_)   => { self.override_raw_input = false; for s in &mut self.schemas { s.weight = (s.weight - 0.1).max(0.0); } }
            TemplateUpdate::Split(t)    => { self.schemas.push(Schema { name: format!("split:{}", t.0), weight: 0.5 }); }
        }
    }
}
/// 4.2 Priority Stack — "internal ordering of these." All fourteen, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority { Coherence, Comfort, Belonging, Control, Safety, Novelty, Power, Harmony, Image, Autonomy, Truth, Sovereignty, Status, Peace }
impl Priority {
    /// Names and parses the 4.2 priority-stack entries (round-trips with `from_name`).
    ///
    /// ```
    /// use fsl_actor::Priority;
    /// assert_eq!(Priority::Coherence.name(), "coherence");
    /// assert_eq!(Priority::from_name("truth"), Some(Priority::Truth));
    /// assert_eq!(Priority::from_name("not-a-priority"), None);
    /// ```
    pub fn name(self) -> &'static str {
        match self {
            Priority::Coherence => "coherence", Priority::Comfort => "comfort", Priority::Belonging => "belonging",
            Priority::Control => "control", Priority::Safety => "safety", Priority::Novelty => "novelty",
            Priority::Power => "power", Priority::Harmony => "harmony", Priority::Image => "image",
            Priority::Autonomy => "autonomy", Priority::Truth => "truth", Priority::Sovereignty => "sovereignty",
            Priority::Status => "status", Priority::Peace => "peace",
        }
    }
    pub fn from_name(n: &str) -> Option<Priority> {
        Some(match n.to_lowercase().as_str() {
            "coherence" => Priority::Coherence, "comfort" => Priority::Comfort, "belonging" => Priority::Belonging,
            "control" => Priority::Control, "safety" => Priority::Safety, "novelty" => Priority::Novelty,
            "power" => Priority::Power, "harmony" => Priority::Harmony, "image" => Priority::Image,
            "autonomy" => Priority::Autonomy, "truth" => Priority::Truth, "sovereignty" => Priority::Sovereignty,
            "status" => Priority::Status, "peace" => Priority::Peace, _ => return None,
        })
    }
}
/// 4.3 Pruning Strategy — three loci.
#[derive(Clone, Copy, Debug)]
pub struct Pruning { pub perceptual: PruneLevel, pub prefilter: PruneLevel, pub postcompile: PruneLevel }
impl Default for Pruning { fn default() -> Self { Pruning { perceptual: PruneLevel::Low, prefilter: PruneLevel::Low, postcompile: PruneLevel::None } } }
/// 4.4 Arbitration Between RIC and PFC — TWO axes.
#[derive(Clone, Copy, Debug)]
pub struct Arbitration { pub raw_vs_template: RawVsTemplate, pub identity_vs_reality: IdentityVsReality }
#[derive(Clone, Copy, Debug)] pub enum RawVsTemplate { RawWins, TemplateWins, CompilerResolves, IdentityWins }
#[derive(Clone, Copy, Debug)] pub enum IdentityVsReality { IdentityYields, RealityYields, Split }
/// 4.5 Meaning Engine Style. "Directive meaning is where sovereignty and consistent
/// agency come from."
#[derive(Clone, Copy, Debug)] pub enum MeaningStyle { Descriptive, Evaluative, Directive }
impl MeaningStyle { pub fn tag(self) -> MeaningStyleTag { match self { MeaningStyle::Descriptive => MeaningStyleTag::Descriptive, MeaningStyle::Evaluative => MeaningStyleTag::Evaluative, MeaningStyle::Directive => MeaningStyleTag::Directive } } }
/// 4.6 Emotion Handling Style — Modes D / C / B / Ignorable.
#[derive(Clone, Copy, Debug)]
pub enum EmotionMode { DecisionDriving, StoryRewriting, Reweighting, Ignorable }
/// 4.7 Identity Mode (ISL Style).
#[derive(Clone, Copy, Debug)] pub enum IdentityMode { Coherent, Fragmented, Scripted, Aesthetic, Sovereign }

/// The complete, MUTABLE parameter space. Read in every stage (see fsl-mind).
#[derive(Clone, Debug)]
pub struct Params {
    pub templates: TemplateLibrary,    // 4.1
    pub priority_stack: Vec<Priority>, // 4.2
    pub pruning: Pruning,              // 4.3
    pub arbitration: Arbitration,      // 4.4
    pub meaning_style: MeaningStyle,   // 4.5
    pub emotion_mode: EmotionMode,     // 4.6
    pub identity_mode: IdentityMode,   // 4.7
}
impl Default for Params {
    fn default() -> Self {
        Params {
            templates: TemplateLibrary { schemas: vec![
                Schema { name: "conflict".into(), weight: 0.5 }, Schema { name: "authority".into(), weight: 0.5 },
            ], override_raw_input: false },
            priority_stack: vec![
                Priority::Coherence, Priority::Comfort, Priority::Belonging, Priority::Control,
                Priority::Safety, Priority::Novelty, Priority::Power, Priority::Harmony,
                Priority::Image, Priority::Autonomy, Priority::Truth, Priority::Sovereignty,
                Priority::Status, Priority::Peace,
            ],
            pruning: Pruning::default(),
            arbitration: Arbitration { raw_vs_template: RawVsTemplate::CompilerResolves, identity_vs_reality: IdentityVsReality::Split },
            meaning_style: MeaningStyle::Directive,
            emotion_mode: EmotionMode::Reweighting,
            identity_mode: IdentityMode::Sovereign,
        }
    }
}
