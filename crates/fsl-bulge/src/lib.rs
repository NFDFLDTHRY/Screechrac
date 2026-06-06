#![allow(dead_code)]
//! fsl-bulge — THE BULGE: The Coffee Cup arc as a deformation along a cable. The Coffee Cup. A long-lived `CupArc` per `CrossingId` that ADVANCES
//! SPATIAL ROLE: THE BULGE — the Coffee Cup deformation along a cable; explodes into five stage planes.
//! across ticks (event-driven, not fixed-rate). Five stages, each answering its one
//! exact question and carrying three parallel domains (social / language / lived);
//! five distinctly-named edges with DISTINCT causal semantics; Trajectory as a real
//! visibility discipline; Release as option-collapse; no-stakes framing as a DEGRADE
//! (VisibilityBias); recirculation via Aftermath writing back into per-crossing state.

use fsl_core::*;

// ─────────────── No-stakes framing: a DEGRADE toward INVALID, not a reject ───────────────
/// Lines 5–8: "Nothing bad will happen if it falls. That matters. Because if
/// something bad could happen, you would stop looking at structure and start arguing
/// about stakes. This removes that escape." Modeled as visibility degradation.
#[derive(Clone, Copy, Debug)]
pub struct Framing { pub stakes_present: bool }
#[derive(Clone, Copy, Debug)]
pub struct VisibilityBias { pub structure_visibility: f32, pub invalid_pressure: f32 }
/// Feeds the membrane: higher stakes ⇒ lower structure-visibility ⇒ higher INVALID
/// pressure (more "arguing about stakes"). Magnitudes illustrative, not decided.
pub fn framing_bias(f: Framing) -> VisibilityBias {
    if f.stakes_present { VisibilityBias { structure_visibility: 0.3, invalid_pressure: 0.7 } }
    else { VisibilityBias { structure_visibility: 1.0, invalid_pressure: 0.0 } }
}

// ─────────────── The three parallel domains of EVERY stage ───────────────
/// Each stage is rendered three times: "In social interaction… / In language… / In
/// lived events…". All five stages carry — and populate — these three domains.
#[derive(Clone, Debug)]
pub struct Domains { pub social: &'static str, pub language: &'static str, pub lived: &'static str }

// ─────────────── The five stages, each answering ONE question ───────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CupStage {
    /// Stage One — Conditions Before Release. "None of this causes the fall. It makes
    /// the fall possible."
    Conditions,
    /// Stage Two — Release. "Release is the point where possibility collapses into
    /// motion." "It is not decisive. It is initiating."
    Release,
    /// Stage Three — Trajectory (The Control Window). "It is a visibility discipline."
    Trajectory,
    /// Stage Four — Impact. "Impact is where the process completes. Reality resolves."
    Impact,
    /// Stage Five — Aftermath. "Physics is finished. Causality is not."
    Aftermath,
}
impl CupStage {
    /// "This stage answers one question only" — verbatim (lines 39 / 64 / 100 / 122 / 145).
    pub fn question(self) -> &'static str {
        match self {
            CupStage::Conditions => "Why was a fall possible.",
            CupStage::Release    => "When did motion begin.",
            CupStage::Trajectory => "When could this outcome still be altered.",
            CupStage::Impact     => "What did reality produce.",
            CupStage::Aftermath  => "What this event becomes once it enters memory and system behavior.",
        }
    }
    /// The three domains, populated with the document's verbatim per-domain text.
    pub fn domains(self) -> Domains {
        match self {
            CupStage::Conditions => Domains {
                social: "everything that exists before a conversation turns",   // line 26
                language: "the loaded field before words are spoken",            // line 30
                lived: "structural vulnerability",                               // line 33
            },
            CupStage::Release => Domains {
                social: "the moment a boundary is crossed or refused",           // line 50
                language: "the first charged utterance",                         // line 53
                lived: "initiation",                                             // line 55
            },
            CupStage::Trajectory => Domains {
                social: "escalation",                                            // line 79
                language: "the message sequence",                                // line 82
                lived: "the interval after initiation but before severance",     // line 86
            },
            CupStage::Impact => Domains {
                social: "rupture",                                               // line 110
                language: "the final statement",                                 // line 113
                lived: "the outcome that cannot be undone",                      // line 115
            },
            CupStage::Aftermath => Domains {
                social: "how the event is retold",                               // line 131
                language: "the script that gets reused",                         // line 133
                lived: "how the outcome rewrites future conditions",             // line 135
            },
        }
    }
}

// ─────────────── Stage Two — Release (option-set collapse) ───────────────
/// "Before this moment, the cup could be held, set down, or dropped. After this
/// moment, only one process remains available."
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Affordance { Held, SetDown, Dropped, Falling }
#[derive(Clone, Debug)]
pub struct Release { pub options_before: Vec<Affordance>, pub remaining_after: Affordance, pub committed: bool }
impl Default for Release {
    fn default() -> Self {
        Release { options_before: vec![Affordance::Held, Affordance::SetDown, Affordance::Dropped],
                  remaining_after: Affordance::Falling, committed: false }
    }
}

// ─────────────── Stage Three — Trajectory (visibility discipline) ───────────────
/// "Early in the fall, intervention is trivial. Later, it requires speed. Later
/// still, only partial mitigation remains. Eventually, nothing can be done."
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InterventionCost { Trivial, RequiresSpeed, PartialMitigationOnly, Impossible }
/// The four questions Trajectory forces (lines 91–94), populated as the fall proceeds.
#[derive(Clone, Debug)]
pub struct Signal {
    pub description: String,
    /// "When did this become obvious."
    pub became_obvious_at: Option<f32>,
    /// "What signals were visible but ignored."
    pub visible_but_ignored: bool,
    /// "What interventions were possible but socially costly."
    pub intervention_possible: bool,
    pub socially_costly: bool,
}
#[derive(Clone, Debug, Default)]
pub struct ControlWindow {
    pub position: f32,          // 0.0 Release .. 1.0 Impact
    pub cost: Option<InterventionCost>,
    /// "When did 'still time' quietly turn into 'too late.'"
    pub too_late_at: f32,
    pub too_late: bool,
    pub signals: Vec<Signal>,   // the visibility discipline — populated each tick
}
impl ControlWindow {
    pub fn cost_at(position: f32) -> InterventionCost {
        if position < 0.25 { InterventionCost::Trivial }
        else if position < 0.5 { InterventionCost::RequiresSpeed }
        else if position < 0.9 { InterventionCost::PartialMitigationOnly }
        else { InterventionCost::Impossible }
    }
}

// ─────────────── Stage Four — Impact (ground truth) ───────────────
/// "Impact matters because it provides ground truth. What actually happened, not
/// what might have happened."
#[derive(Clone, Debug, Default)]
pub struct Impact { pub ground_truth: Vec<ClaimId>, pub severity: f32 }

// ─────────────── Stage Five — Aftermath (THREE feedback targets) ───────────────
/// "Narratives alter future release thresholds. They reshape what interventions will
/// be attempted next time. They rewrite conditions. This is where the loop closes."
/// Also: "Stories form. Meaning is assigned. Lessons are drawn. Identities adjust."
/// (the written lexical bridge to HCC-A's Story Ledger / Meaning Engine / ISL.)
#[derive(Clone, Debug, Default)]
pub struct Aftermath {
    pub rewrites_conditions: Vec<ClaimId>,
    pub release_threshold_delta: f32,
    pub intervention_policy_update: InterventionPolicy,
}
#[derive(Clone, Debug, Default)]
pub struct InterventionPolicy { pub attempts_next_time: Vec<String> }

// ─────────────── The five DISTINCT edges (distinct TYPES + distinct SEMANTICS) ───────────────
/// "The Loop You Keep Missing." Five differently-named edges; each `propagate` body
/// is DIFFERENT (probabilistic vs irreversible-commit vs shaping vs feeding vs
/// rewriting) — never one generic arrow.
pub trait CupEdge {
    fn verb(&self) -> &'static str;
    fn from(&self) -> CupStage;
    fn to(&self) -> CupStage;
    /// The causal action this edge performs on the arc. Distinct per edge.
    fn propagate(&self, arc: &mut CupArc);
}

/// "Conditions make release likely." — PROBABILISTIC: raises likelihood, forces nothing.
pub struct ConditionsMakeLikelyRelease;
/// "Release commits trajectory." — IRREVERSIBLE latch: collapses options to one.
pub struct ReleaseCommitsTrajectory;
/// "Trajectory shapes impact." — SHAPES severity from window position/cost.
pub struct TrajectoryShapesImpact;
/// "Impact feeds narrative." — FEEDS ground truth forward into Aftermath material.
pub struct ImpactFeedsNarrative;
/// "Narrative rewrites conditions." — REWRITES Conditions + release threshold. ("It recirculates.")
pub struct NarrativeRewritesConditions;

impl CupEdge for ConditionsMakeLikelyRelease {
    fn verb(&self) -> &'static str { "make likely" }
    fn from(&self) -> CupStage { CupStage::Conditions }
    fn to(&self) -> CupStage { CupStage::Release }
    fn propagate(&self, arc: &mut CupArc) {
        // probabilistic: nudges likelihood; does not commit.
        arc.release_likelihood = (arc.release_likelihood + 0.5).min(1.0);
    }
}
impl CupEdge for ReleaseCommitsTrajectory {
    fn verb(&self) -> &'static str { "commits" }
    fn from(&self) -> CupStage { CupStage::Release }
    fn to(&self) -> CupStage { CupStage::Trajectory }
    fn propagate(&self, arc: &mut CupArc) {
        // irreversible: the option set is now one process.
        let mut r = arc.release.take().unwrap_or_default();
        r.committed = true; r.remaining_after = Affordance::Falling;
        arc.release = Some(r);
        arc.window.position = 0.0;
        arc.window.too_late_at = 0.9;
        arc.window.cost = Some(ControlWindow::cost_at(0.0));
    }
}
impl CupEdge for TrajectoryShapesImpact {
    fn verb(&self) -> &'static str { "shapes" }
    fn from(&self) -> CupStage { CupStage::Trajectory }
    fn to(&self) -> CupStage { CupStage::Impact }
    fn propagate(&self, arc: &mut CupArc) {
        // severity is shaped by how far the fall went before resolution.
        arc.impact.severity = arc.window.position;
    }
}
impl CupEdge for ImpactFeedsNarrative {
    fn verb(&self) -> &'static str { "feeds" }
    fn from(&self) -> CupStage { CupStage::Impact }
    fn to(&self) -> CupStage { CupStage::Aftermath }
    fn propagate(&self, arc: &mut CupArc) {
        arc.aftermath = Some(Aftermath {
            rewrites_conditions: arc.impact.ground_truth.clone(),
            release_threshold_delta: if arc.impact.severity >= 0.9 { 0.2 } else { 0.05 },
            intervention_policy_update: InterventionPolicy {
                attempts_next_time: vec!["earlier anchor request".into()],
            },
        });
    }
}
impl CupEdge for NarrativeRewritesConditions {
    fn verb(&self) -> &'static str { "rewrites" }
    fn from(&self) -> CupStage { CupStage::Aftermath }
    fn to(&self) -> CupStage { CupStage::Conditions }
    fn propagate(&self, arc: &mut CupArc) {
        // the loop closes: narrative rewrites the conditions of the next cycle.
        arc.recirculated += 1;
        arc.release_likelihood = 0.0;
        arc.release = None;
        arc.window = ControlWindow::default();
    }
}

// ─────────────── Diagnostics: observer error by stage-fixation ───────────────
/// "What This Exposes About You" — each error keyed to where attention collapses.
#[derive(Clone, Copy, Debug)]
pub enum ObserverError {
    /// "relief at release → you live upstream and call it inevitability"
    UpstreamInevitability,
    /// "clarity at impact → you arrive late and call it judgment"
    LateJudgment,
    /// "discomfort at trajectory → responsibility exists there without permission"
    TrajectoryResponsibility,
    /// "wanted to argue about what should have been done → you are already in aftermath"
    AlreadyAftermath,
}

// ─────────────── Strategic theory: hold all five at once ───────────────
/// "Planning is not prediction. Planning is the ability to hold all five stages at
/// once while time still exists."
pub struct PlannerView { pub stages: [CupStage; 5] }
impl Default for PlannerView {
    fn default() -> Self {
        PlannerView { stages: [CupStage::Conditions, CupStage::Release, CupStage::Trajectory, CupStage::Impact, CupStage::Aftermath] }
    }
}

// ─────────────── The long-lived arc, advanced per tick (event-driven) ───────────────
/// Inputs the surrounding tick assembles for the arc (content + bridge-derived).
pub struct CupTickInput<'a> {
    pub admitted: Option<&'a StructurePayload>,
    pub bridge_crossing: bool,
    /// a charged/committing utterance is present (drives Conditions → Release).
    pub charged: bool,
    /// a rupture/severance event resolves the fall (drives Trajectory → Impact).
    pub terminal: bool,
}

#[derive(Clone, Debug)]
pub struct CupArc {
    pub interaction: CrossingId,
    pub phase: CupStage,
    pub framing: Framing,
    pub release_likelihood: f32,
    pub release: Option<Release>,
    pub window: ControlWindow,
    pub impact: Impact,
    pub aftermath: Option<Aftermath>,
    pub recirculated: u32,
}
impl CupArc {
    /// Open a fresh Coffee Cup arc for a crossing. The five edges that drive it are
    /// pairwise DISTINCT (never one generic arrow):
    ///
    /// ```
    /// use fsl_bulge::{CupArc, CupEdge, ConditionsMakeLikelyRelease, ReleaseCommitsTrajectory,
    ///     TrajectoryShapesImpact, ImpactFeedsNarrative, NarrativeRewritesConditions};
    /// use fsl_core::CrossingId;
    /// let arc = CupArc::new(CrossingId(1), false);
    /// assert_eq!(arc.recirculated, 0);
    /// let verbs = [ConditionsMakeLikelyRelease.verb(), ReleaseCommitsTrajectory.verb(),
    ///     TrajectoryShapesImpact.verb(), ImpactFeedsNarrative.verb(), NarrativeRewritesConditions.verb()];
    /// let mut v = verbs.to_vec(); v.sort(); v.dedup();
    /// assert_eq!(v.len(), 5);
    /// ```
    pub fn new(interaction: CrossingId, stakes_present: bool) -> Self {
        CupArc {
            interaction, phase: CupStage::Conditions, framing: Framing { stakes_present },
            release_likelihood: 0.0, release: Some(Release::default()),
            window: ControlWindow::default(), impact: Impact::default(),
            aftermath: None, recirculated: 0,
        }
    }
    pub fn visibility(&self) -> VisibilityBias { framing_bias(self.framing) }

    /// SINGLE SOURCE OF EDGE DISPATCH (FSL Coherence Contract: never flatten distinct
    /// edges). This is the ONLY place a CupEdge is ever traversed; it always calls the
    /// edge's distinct `propagate()` — never a generic arrow — then returns the edge.
    fn traverse_edge(&mut self, edge: Box<dyn CupEdge>) -> Box<dyn CupEdge> {
        edge.propagate(self);
        edge
    }

    /// Advance the arc by EVENTS this tick (not a fixed clock). Returns the edge
    /// traversed (whose distinct `propagate` was applied via `traverse_edge`), or None
    /// if the fall is still mid-Trajectory and only the window moved.
    pub fn advance(&mut self, input: &CupTickInput) -> Option<Box<dyn CupEdge>> {
        match self.phase {
            CupStage::Conditions => {
                let e = self.traverse_edge(Box::new(ConditionsMakeLikelyRelease)); // "make likely" (probabilistic)
                if input.charged || input.bridge_crossing || self.release_likelihood >= 1.0 {
                    self.phase = CupStage::Release;
                }
                Some(e)
            }
            CupStage::Release => {
                let e = self.traverse_edge(Box::new(ReleaseCommitsTrajectory)); // "commits" (irreversible)
                self.phase = CupStage::Trajectory;
                Some(e)
            }
            CupStage::Trajectory => {
                // visibility discipline: record a signal, advance the window, detect "too late".
                let before = self.window.position;
                self.window.position = (self.window.position + 0.5).min(1.0);
                self.window.cost = Some(ControlWindow::cost_at(self.window.position));
                let obvious = if before < 0.5 && self.window.position >= 0.5 { Some(self.window.position) } else { None };
                self.window.signals.push(Signal {
                    description: format!("trajectory@{:.2}", self.window.position),
                    became_obvious_at: obvious,
                    visible_but_ignored: !input.bridge_crossing,
                    intervention_possible: self.window.position < self.window.too_late_at,
                    socially_costly: true,
                });
                if self.window.position >= self.window.too_late_at { self.window.too_late = true; }
                if input.terminal || self.window.position >= 1.0 {
                    if let Some(s) = input.admitted { self.impact.ground_truth = s.claims.iter().map(|c| c.id).collect(); }
                    let e = self.traverse_edge(Box::new(TrajectoryShapesImpact)); // "shapes"
                    self.phase = CupStage::Impact;
                    return Some(e);
                }
                None // still falling; window moved, no stage edge yet
            }
            CupStage::Impact => {
                let e = self.traverse_edge(Box::new(ImpactFeedsNarrative)); // "feeds"
                self.phase = CupStage::Aftermath;
                Some(e)
            }
            CupStage::Aftermath => {
                let e = self.traverse_edge(Box::new(NarrativeRewritesConditions)); // "rewrites" — recirculates
                self.phase = CupStage::Conditions; // "It recirculates."
                Some(e)
            }
        }
    }
    /// The Aftermath produced at the Impact→Aftermath edge (consumed by the membrane
    /// per-pair write-back). Cleared once applied.
    pub fn take_aftermath(&mut self) -> Option<Aftermath> { self.aftermath.take() }
}
