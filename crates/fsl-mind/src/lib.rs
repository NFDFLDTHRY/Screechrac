#![allow(dead_code)]
//! fsl-mind — the self-referential World that wires the three documents into one loop.
//! SPATIAL ROLE: SCENE-GRAPH ASSEMBLER + WALKTHROUGH — places every grain in space; parallel cables/strands; Onion Shell; bulge explosion (scene math in fsl-scene).
//! A `Mind` is a complete HCC-A instance whose `run_pass` threads the WHOLE parameter
//! space through every stage S0–S7 (both loops). The `World` owns the Proof membrane
//! as the SOLE channel between minds ("only structure crosses"), drives the Delta
//! Bridge FSM from the membrane's signals, advances a long-lived `CupArc` per pair,
//! calls `apply_aftermath` to recirculate, decomposes UNKs across ticks via the cable,
//! and resolves them when a pointer arrives later. Minds can spawn minds.

use std::collections::HashMap;
use fsl_core::*;
use fsl_flow::membrane::{ProofMembrane, ExpectedRule, CrossingReport};
use fsl_flow::bridge::{BridgeState, detect_b};
use fsl_bulge::{CupArc, CupTickInput, Aftermath};
use fsl_actor::*;

// ─────────────────────────── A single mind ───────────────────────────
pub struct Mind {
    pub id: MindId,
    pub parent: Option<MindId>,
    pub interface: Interface,
    pub compiler: Compiler,
    pub meaning: MeaningEngine,
    pub isl: Isl,
    pub ledger: StoryLedger,
    pub params: Params,
    pub expected_rules: Vec<ExpectedRule>,
    /// "repeated emotional patterns" — the history the ISL integrates over.
    pub emotion_history: Vec<Settle>,
    pub identity: IdentitySnapshot,
    next_id: u64,
}

/// One full HCC-A pass, fully traced.
#[derive(Debug)]
pub struct PassResult {
    pub behavior: Behavior,
    pub last_settle: Settle,
    pub template_update: TemplateUpdate,
    pub perceptually_pruned: u64,
    pub prefilter_pruned: u64,
    pub postcompile_pruned: u64,
    pub identity: String,
    pub acts: Vec<Act>,
}

impl Mind {
    pub fn new(id: u64, params: Params, expected_rules: Vec<ExpectedRule>) -> Self {
        Mind {
            id: MindId(id), parent: None, interface: Interface::default(), compiler: Compiler,
            meaning: MeaningEngine, isl: Isl, ledger: StoryLedger::default(), params, expected_rules,
            emotion_history: vec![], identity: IdentitySnapshot::default(), next_id: 10_000 + id * 1000,
        }
    }

    /// Deliver crossed structure into this mind's interface (membrane → S0).
    pub fn receive(&mut self, s: StructurePayload) { self.interface.inject(s); }

    /// Run S0→S7 with the full parameter space steering EVERY stage. Both loops run:
    /// the inner loop here (ISL → template update applied to priors); the outer loop is
    /// closed by the World feeding behavior back as new structure.
    pub fn run_pass(&mut self, oracle: &dyn Oracle, bridge: BridgeState, invalid_pressure: f32, root: RootCtx) -> PassResult {
        let mut acts = vec![];

        // ── S0 Intake ── perceptual pruning (4.3 locus 1) at the lossy interface.
        acts.push(act_of(HccaStage::Intake));
        let incoming = self.interface.drain();
        let reality = Reality { events: incoming };
        let raw = self.interface.convert(&reality, self.params.pruning.perceptual);
        let perceptually_pruned = raw.perceptually_pruned;
        // RIC ∥ PFC — PFC reads the 4.1 template library (parallel, both feed C).
        let pfc = pfc_interpret(oracle, &raw.claims, &self.params.templates, root);
        let mut intake = Intake { ric: Ric { structured: raw.claims }, pfc, prefilter_pruned: 0 };

        // ── S1 Coexistence & Early Pruning ── prefilter pruning (4.3 locus 2).
        acts.push(act_of(HccaStage::CoexistPrune));
        prefilter_prune(&mut intake, self.params.pruning.prefilter);
        let prefilter_pruned = intake.prefilter_pruned;

        // ── S2 Compilation ── {RIC, PFC} → Ledger under 4.4 arbitration.
        acts.push(act_of(HccaStage::Compile));
        let mut next_id = self.next_id;
        self.compiler.compile(&intake, &mut self.ledger, self.params.arbitration, &mut next_id);
        self.next_id = next_id;

        // ── S3 Meaning ── style (4.5) + priority order (4.2) steer the transitions.
        acts.push(act_of(HccaStage::Meaning));
        let names: Vec<&str> = self.params.priority_stack.iter().map(|p| p.name()).collect();
        let transitions = self.meaning.run(oracle, &self.ledger, &names, self.params.meaning_style, root);
        let postcompile_pruned = postcompile_prune(&mut self.ledger, self.params.pruning.postcompile); // 4.3 locus 3

        // ── S4 Emotion ── meaning-in-flight WHILE applying each transition; stall path.
        acts.push(act_of(HccaStage::Emotion));
        let emode = self.params.emotion_mode;
        let mut emotion = EmotionState::idle(emode);
        let mut last = ApplyResult::default();
        let idmode = self.params.identity_mode;
        for t in &transitions {
            last = apply_transition(t, &mut emotion, idmode, &mut self.params.priority_stack);
            self.emotion_history.push(emotion.settle);
        }
        let last_settle = emotion.settle;

        // ── S5 Identity ── INNER LOOP: ISL picks entrench/weaken/split from the
        // repeated emotional patterns + identity mode, and the update is APPLIED to
        // the priors (template weights), not a bool.
        acts.push(act_of(HccaStage::Identity));
        let (snapshot, template_update) =
            self.isl.integrate(&self.ledger, &self.emotion_history, idmode, TemplateId(1));
        self.params.templates.apply_update(template_update);
        self.identity = snapshot.clone();

        // ── S6 Behavior ── full taxonomy; directive gated on the bridge (STATE 4);
        // emotion-mode consequence + 4.4 identity-vs-reality + framing pressure select.
        acts.push(act_of(HccaStage::Behavior));
        let directive = matches!(self.params.meaning_style, MeaningStyle::Directive) && !last.stalled;
        let intent = BehaviorIntent { text: "respond to the crossing".into(), directive };
        let behavior = emit(intent, bridge, last.mode_consequence.clone(), self.params.arbitration, invalid_pressure);

        // ── S7 Feedback ── (outer loop is closed by the World; see World::tick).
        acts.push(act_of(HccaStage::Feedback));

        PassResult { behavior, last_settle, template_update, perceptually_pruned, prefilter_pruned,
                     postcompile_pruned, identity: snapshot.summary, acts }
    }

    /// Behavior → outgoing structure for the next crossing (only pointable structure
    /// can later become OBS; non-pointable becomes the other mind's UNK).
    pub fn outgoing_from(&self, behavior: &Behavior, to: MindId, claims: Vec<Claim>) -> CrossingCandidate {
        let _ = behavior;
        CrossingCandidate { from: self.id, to, claims }
    }
}

// ─────────────────────── Per-crossing (per-pair) state ───────────────────────
/// The membrane's per-pair state that Aftermath recirculation writes back into.
pub struct PairState {
    pub crossing: CrossingId,
    pub a: MindId,
    pub b: MindId,
    pub bridge: BridgeState,
    pub ticks_in_state: u32,
    pub cup: CupArc,
    // ── the three Aftermath feedback targets live here ──
    pub conditions: Vec<ClaimId>,      // "They rewrite conditions."
    pub release_threshold: f32,        // "alter future release thresholds."
    pub intervention: Vec<String>,     // "reshape what interventions will be attempted next time."
}
impl PairState {
    fn new(crossing: u64, a: MindId, b: MindId, stakes: bool) -> Self {
        PairState {
            crossing: CrossingId(crossing), a, b, bridge: BridgeState::S0Banks, ticks_in_state: 0,
            cup: CupArc::new(CrossingId(crossing), stakes),
            conditions: vec![], release_threshold: 0.5, intervention: vec![],
        }
    }
}

// ─────────────────────────── The World ───────────────────────────
pub struct World {
    pub minds: HashMap<MindId, Mind>,
    pub membrane: ProofMembrane,
    pub pairs: HashMap<CrossingId, PairState>,
    pub cable: Cable_,
    pub policy: InvalidPolicy,
    pub unk_policy: UnkPolicy,
    // ── sandbox / scheduler / onion ──
    pub sandbox: Sandbox,
    pub ready: std::collections::VecDeque<RootId>,
    /// Onion priority queue: (degree, UnkId, unk-root). Lowest degree = highest priority.
    pub onion: Vec<(DegreeOfSeparation, UnkId, RootId)>,
    /// the conversational pair every root is processed against (the membrane channel).
    pub default_pair: Option<CrossingId>,
    pub default_from: Option<MindId>,
    pub default_to: Option<MindId>,
    next_mind: u64,
    next_unk: u64,
    tickn: u64,
}
// alias to the cable crate's type (kept distinct from any local name)
pub use fsl_cable::Cable as Cable_;

/// Everything one tick of a pair produced — for the harness trace.
pub struct TickReport {
    pub report: CrossingReport,
    pub bridge_before: BridgeState,
    pub bridge_after: BridgeState,
    pub crossed: bool,
    pub pass: Option<PassResult>,
    pub cup_stage: fsl_bulge::CupStage,
    pub recirculated: u32,
    pub resolved_unks: Vec<UnkId>,
    pub decomposed: usize,
    pub aftermath_applied: bool,
}

impl World {
    /// Open a fresh `World` — the self-referential sandbox that ingests root nodes
    /// (cables) and runs the full membrane → bridge → Coffee-Cup → HCC-A loop.
    ///
    /// ```
    /// use fsl_mind::World;
    /// use fsl_core::{InvalidPolicy, UnkPolicy};
    /// use fsl_actor::Params;
    /// use fsl_llm::DeterministicOracle;
    /// let oracle = DeterministicOracle::default();
    /// let mut world = World::new(InvalidPolicy::default(),
    ///     UnkPolicy::BoundedBudget { max_open_unk: 3, max_strand_depth: 2 });
    /// let a = world.spawn(None, Params::default(), vec![]);
    /// let b = world.spawn(None, Params::default(), vec![]);
    /// let x = world.open_pair(1, a, b, false);
    /// world.set_default_pair(x, a, b);
    /// let root = world.ingest_input("the deadline slipped"); // a CABLE
    /// let _trace = world.primary_loop(&oracle, root, vec![], false);
    /// assert!(world.sandbox.ledger.len() >= 1); // the Ledger is the append-only truth
    /// ```
    pub fn new(policy: InvalidPolicy, unk_policy: UnkPolicy) -> Self {
        World { minds: HashMap::new(), membrane: ProofMembrane::new(), pairs: HashMap::new(),
                cable: Cable_::default(), policy, unk_policy,
                sandbox: Sandbox::default(), ready: std::collections::VecDeque::new(), onion: vec![],
                default_pair: None, default_from: None, default_to: None,
                next_mind: 1, next_unk: 90_000, tickn: 0 }
    }

    /// Spawn a mind. Minds can spawn minds (self-referential): pass the parent's id.
    pub fn spawn(&mut self, parent: Option<MindId>, params: Params, rules: Vec<ExpectedRule>) -> MindId {
        let id = self.next_mind; self.next_mind += 1;
        let mut m = Mind::new(id, params, rules);
        m.parent = parent;
        let mid = m.id; self.minds.insert(mid, m); mid
    }

    pub fn open_pair(&mut self, crossing: u64, a: MindId, b: MindId, stakes: bool) -> CrossingId {
        let ps = PairState::new(crossing, a, b, stakes);
        let id = ps.crossing; self.pairs.insert(id, ps); id
    }

    /// One directed tick a→b on a crossing. This is where all three documents meet:
    /// membrane (Proof) → bridge step (River/Bridge) → HCC-A pass → Cup advance →
    /// Aftermath recirculation → cross-tick UNK resolution + decomposition.
    pub fn tick(&mut self, oracle: &dyn Oracle, crossing: CrossingId, from: MindId, to: MindId, claims: Vec<Claim>, tick: TickId, terminal: bool, root: RootCtx) -> TickReport {
        // (a) framing → invalid_pressure for this pair (no-stakes degrade, end-to-end).
        let invalid_pressure = {
            let ps = self.pairs.get(&crossing).unwrap();
            ps.cup.visibility().invalid_pressure
        };
        // clone receiver's expected rules before the mutable membrane borrow (no alias).
        let expected: Vec<ExpectedRule> = self.minds.get(&to).map(|m| m.expected_rules.clone()).unwrap_or_default();

        // (b) MEMBRANE: functions assemble fresh context and CALL the Oracle; apply.
        let cand = CrossingCandidate { from, to, claims };
        let report = self.membrane.cross(&cand, &self.policy, oracle, invalid_pressure, &expected, tick, root);

        // (c) BRIDGE: drive the FSM from the membrane's signal (REAL transition).
        let bridge_before;
        let bridge_after;
        {
            let ps = self.pairs.get_mut(&crossing).unwrap();
            bridge_before = ps.bridge;
            ps.bridge = ps.bridge.step(report.signal);
            ps.ticks_in_state = if ps.bridge == bridge_before { ps.ticks_in_state + 1 } else { 0 };
            bridge_after = ps.bridge;
            let _ = detect_b(bridge_before, bridge_after, ps.ticks_in_state); // I-B*/FM-B* available to trace
        }

        // (d) DELIVER crossed structure → receiver runs a full HCC-A pass (both loops).
        let mut crossed = false;
        let mut pass = None;
        let mut resolved_unks = vec![];
        if let Some(structure) = report.admitted.clone() {
            crossed = true;
            resolved_unks = self.membrane.resolve_unks_with(&structure); // UNK resolution across ticks
            if let Some(m) = self.minds.get_mut(&to) {
                m.receive(structure);
                pass = Some(m.run_pass(oracle, bridge_after, invalid_pressure, root));
            }
        }

        // (e) CABLE: decompose any still-open UNKs into back-linked strands (across ticks).
        let mut decomposed = 0;
        {
            let open: Vec<_> = self.membrane.open_unks.iter().filter(|u| !u.resolved).cloned().collect();
            let mut nu = self.next_unk;
            for u in &open {
                let subs = self.cable.decompose_unk(u, oracle, &self.unk_policy, tick, &mut nu, root);
                decomposed += subs.len();
            }
            self.next_unk = nu;
        }

        // (f) CUP: advance the long-lived arc by EVENTS this tick.
        let charged = report.admitted.as_ref().map(|s| s.claims.iter().any(|c| c.stance.unwrap_or(0) != 0)).unwrap_or(false);
        let bridge_crossing = bridge_after.directives_legitimate();
        let (cup_stage, recirculated, aftermath_opt) = {
            let ps = self.pairs.get_mut(&crossing).unwrap();
            let input = CupTickInput { admitted: report.admitted.as_ref(), bridge_crossing, charged, terminal };
            let _edge = ps.cup.advance(&input);
            (ps.cup.phase, ps.cup.recirculated, ps.cup.take_aftermath())
        };

        // (g) RECIRCULATION: Aftermath writes back into the per-pair state (loop closes).
        let mut aftermath_applied = false;
        if let Some(after) = aftermath_opt {
            self.apply_aftermath(crossing, &after);
            aftermath_applied = true;
        }

        TickReport {
            report, bridge_before, bridge_after, crossed, pass,
            cup_stage, recirculated, resolved_unks, decomposed, aftermath_applied,
        }
    }

    /// "This is where the loop closes." The Aftermath's THREE targets are written back
    /// into the membrane's per-pair state.
    pub fn apply_aftermath(&mut self, crossing: CrossingId, a: &Aftermath) {
        if let Some(ps) = self.pairs.get_mut(&crossing) {
            ps.conditions = a.rewrites_conditions.clone();                 // rewrite conditions
            ps.release_threshold += a.release_threshold_delta;            // alter release thresholds
            ps.intervention = a.intervention_policy_update.attempts_next_time.clone(); // reshape interventions
        }
    }

    pub fn bridge_of(&self, crossing: CrossingId) -> BridgeState { self.pairs[&crossing].bridge }

    /// DELTA resolution at the crossing ("resolve DELTAs / promote UNKs to OBS"):
    /// the crossing's DELTAs are resolved and, if at S3 Crossing, the FSM completes to
    /// S4 Bank-Rebuild ("decisions/actions can occur safely").
    pub fn resolve_crossing(&mut self, crossing: CrossingId) -> BridgeState {
        self.membrane.deltas.clear();
        let ps = self.pairs.get_mut(&crossing).unwrap();
        if ps.bridge == BridgeState::S3Crossing {
            ps.bridge = ps.bridge.step(BridgeSignal::DeltasResolved);
            ps.ticks_in_state = 0;
        }
        ps.bridge
    }
}

// ═══════════════════════ SANDBOX / SCHEDULER / ONION ORCHESTRATION ═══════════════════════
// The Primary Operational Loop, the append-only Ledger recording, the navigational
// Graph wiring, parallel root-node scheduling, non-blocking UNK triggering, and the
// Onion Shell coherence engine. The internal LLM still never holds context: each step
// assembles a fresh RootCtx (current root + degree-of-separation) for every call.

/// One step of the sandbox, for the trace.
#[derive(Debug)]
pub struct StepTrace {
    pub root: RootId,
    pub kind: RootKind,
    pub degree: DegreeOfSeparation,
    pub crossed: bool,
    pub obs: usize, pub deltas: usize, pub unks_raised: usize, pub invalids: usize, pub converted: usize,
    pub bridge: BridgeState,
    pub behavior: Option<String>,
    pub cup_stage: fsl_bulge::CupStage,
    pub recirculated: u32,
    pub aftermath_applied: bool,
    pub note: String,
}

impl World {
    pub fn set_default_pair(&mut self, pair: CrossingId, from: MindId, to: MindId) {
        self.default_pair = Some(pair); self.default_from = Some(from); self.default_to = Some(to);
    }
    fn next_tick(&mut self) -> TickId { self.tickn += 1; TickId(self.tickn) }
    fn max_depth(&self) -> u32 { match self.unk_policy { UnkPolicy::BoundedBudget { max_strand_depth, .. } => max_strand_depth as u32, _ => 0 } }

    /// Every user input creates a ROOT NODE (degree 0), appends to the Ledger, adds a
    /// Graph anchor, and enqueues it on the scheduler.
    pub fn ingest_input(&mut self, text: &str) -> RootId { self.ingest(RootKind::Input, text) }
    /// A system event likewise creates a root node.
    pub fn ingest_event(&mut self, text: &str) -> RootId { self.ingest(RootKind::Event, text) }
    fn ingest(&mut self, kind: RootKind, text: &str) -> RootId {
        let r = self.sandbox.new_root(kind, 0, None, text);
        let seq = self.sandbox.ledger.append(r, 0, match kind {
            RootKind::Event => LedgerPayload::Event { text: text.into() },
            _ => LedgerPayload::Input { text: text.into() },
        });
        let n = self.sandbox.graph.add_node(seq);
        if let Some(rn) = self.sandbox.root_mut(r) { rn.anchor_node = Some(n); }
        self.ready.push_back(r);
        r
    }

    /// THE PRIMARY OPERATIONAL LOOP for one root: run the full membrane→bridge→cup→
    /// HCC-A pass on the root's claims, RECORD every generated grain to the append-only
    /// Ledger, wire the navigational Graph (relationships only), and spawn UNK ROOT
    /// NODES (parallel) at degree+1 for the Onion Shell.
    pub fn primary_loop(&mut self, oracle: &dyn Oracle, root_id: RootId, claims: Vec<Claim>, terminal: bool) -> StepTrace {
        let degree = self.sandbox.root(root_id).map(|r| r.degree).unwrap_or(0);
        let kind = self.sandbox.root(root_id).map(|r| r.kind).unwrap_or(RootKind::Input);
        let anchor = self.sandbox.root(root_id).and_then(|r| r.anchor_node);
        let rootctx = RootCtx { root: root_id, degree };
        let (pair, from, to) = (self.default_pair.unwrap(), self.default_from.unwrap(), self.default_to.unwrap());
        let tk = self.next_tick();

        let rep = self.tick(oracle, pair, from, to, claims, tk, terminal, rootctx);

        // ── record to Ledger (truth) + Graph (navigation) ──
        for oid in &rep.report.obs_admitted {
            let topic = self.membrane.obs_topic(*oid);
            let seq = self.sandbox.ledger.append(root_id, degree, LedgerPayload::Obs { id: *oid, topic });
            let n = self.sandbox.graph.add_node(seq);
            if let Some(a) = anchor { self.sandbox.graph.link(a, n, Relationship::Derives); }
        }
        for d in &rep.report.deltas_built {
            let seq = self.sandbox.ledger.append(root_id, degree, LedgerPayload::Delta { id: *d });
            let n = self.sandbox.graph.add_node(seq);
            if let Some(a) = anchor { self.sandbox.graph.link(a, n, Relationship::Pairs); }
        }
        for c in &rep.report.converted {
            let seq = self.sandbox.ledger.append(root_id, degree, LedgerPayload::InvalidConverted { claim: *c });
            let n = self.sandbox.graph.add_node(seq);
            if let Some(a) = anchor { self.sandbox.graph.link(a, n, Relationship::Converts); }
        }
        for iv in &rep.report.invalids {
            self.sandbox.ledger.append(root_id, degree, LedgerPayload::Invalid { claim: iv.claim });
        }
        if let Some(p) = &rep.pass {
            self.sandbox.ledger.append(root_id, degree, LedgerPayload::Behavior { summary: format!("{:?}", p.behavior) });
        }
        // ── UNKs raised → record + spawn PARALLEL UNK root at degree+1 + onion-queue ──
        let raised: Vec<UnkId> = rep.report.unks_raised.clone();
        for uid in &raised {
            let awaiting = self.membrane.open_unks.iter().find(|u| u.id == *uid).and_then(|u| u.awaiting_topic);
            let seq = self.sandbox.ledger.append(root_id, degree, LedgerPayload::Unk { id: *uid, awaiting_topic: awaiting });
            let un = self.sandbox.graph.add_node(seq);
            if let Some(a) = anchor { self.sandbox.graph.link(a, un, Relationship::Raises); }
            let ur = self.sandbox.new_root(RootKind::Unk, degree + 1, Some(root_id), &format!("UNK {:?}", uid));
            if let Some(rn) = self.sandbox.root_mut(ur) { rn.unk = Some(*uid); rn.awaiting_topic = awaiting; rn.anchor_node = Some(un); }
            self.onion.push((degree + 1, *uid, ur));
        }
        if let Some(rn) = self.sandbox.root_mut(root_id) { rn.done = true; }
        self.onion.sort_by_key(|(d, u, _)| (*d, u.0)); // priority: lowest degree first

        StepTrace {
            root: root_id, kind, degree, crossed: rep.crossed,
            obs: rep.report.obs_admitted.len(), deltas: rep.report.deltas_built.len(),
            unks_raised: raised.len(), invalids: rep.report.invalids.len(), converted: rep.report.converted.len(),
            bridge: rep.bridge_after, behavior: rep.pass.as_ref().map(|p| format!("{:?}", p.behavior)),
            cup_stage: rep.cup_stage, recirculated: rep.recirculated, aftermath_applied: rep.aftermath_applied,
            note: String::new(),
        }
    }

    /// NON-BLOCKING TRIGGER: continuously detect when new information becomes available
    /// anywhere in the sandbox and mark eligible the UNK roots that can now proceed —
    /// without blocking any in-flight root. Returns the roots triggered this scan.
    pub fn detect_and_trigger(&mut self) -> Vec<RootId> {
        let mut triggered = vec![];
        let ids: Vec<RootId> = self.sandbox.roots.iter()
            .filter(|r| matches!(r.kind, RootKind::Unk) && !r.done && !r.eligible)
            .map(|r| r.id).collect();
        for id in ids {
            let awaiting = self.sandbox.root(id).and_then(|r| r.awaiting_topic);
            if let Some(t) = awaiting {
                if self.sandbox.ledger.obs_topic_present(t) {
                    if let Some(rn) = self.sandbox.root_mut(id) { rn.eligible = true; }
                    self.ready.push_back(id);
                    triggered.push(id);
                }
            }
        }
        triggered
    }

    /// PARALLEL ROOT-NODE PROCESSING: step every active input/event root in one
    /// scheduling pass (round-robin); UNK roots run in parallel and never block inputs.
    /// `inputs` maps each input root to the claims it carries.
    pub fn run_inputs(&mut self, oracle: &dyn Oracle, inputs: &std::collections::HashMap<RootId, (Vec<Claim>, bool)>) -> Vec<StepTrace> {
        let mut traces = vec![];
        loop {
            self.detect_and_trigger();
            let mut active: Vec<RootId> = self.sandbox.roots.iter()
                .filter(|r| !matches!(r.kind, RootKind::Unk) && !r.done && r.eligible)
                .map(|r| r.id).collect();
            active.sort();
            if active.is_empty() { break; }
            for r in active {
                let (claims, terminal) = inputs.get(&r).cloned().unwrap_or((vec![], false));
                let tr = self.primary_loop(oracle, r, claims, terminal);
                traces.push(tr);
            }
        }
        traces
    }

    fn onion_next(&self) -> Option<usize> {
        // lowest-degree, in-budget entry whose UNK is still open.
        let md = self.max_depth();
        self.onion.iter().position(|(d, u, _)| *d <= md && !self.membrane.open_unks.iter().any(|x| x.id == *u && x.resolved))
    }

    /// THE ONION SHELL — the coherence guarantee engine. Recursively selects UNKs by
    /// degree-of-separation (closest to the original root first), feeds each back into
    /// the FULL Primary Operational Loop while preserving the original conversational
    /// context around its root, and pushes any new UNKs at degree+1 (progressively
    /// lower priority). Bounded by the UNK-resolution seam (max_strand_depth).
    pub fn onion_resolve(&mut self, oracle: &dyn Oracle, max_iterations: usize) -> Vec<StepTrace> {
        let mut traces = vec![];
        let mut iters = 0usize;
        while let Some(idx) = self.onion_next() {
            if iters >= max_iterations { break; }
            iters += 1;
            let (degree, uid, uroot) = self.onion.remove(idx);
            // preserve the original conversational context around the root node
            // (re-assembled fresh from the append-only Ledger for the LLM call).
            let origin = self.sandbox.root(uroot).and_then(|r| r.origin).unwrap_or(uroot);
            let _preserved_context: Vec<Seq> = self.sandbox.ledger.context_of(origin);
            let awaiting = self.sandbox.root(uroot).and_then(|r| r.awaiting_topic);
            let resolvable = awaiting.map(|t| self.sandbox.ledger.obs_topic_present(t)).unwrap_or(false);

            if resolvable {
                // the needed information is now available somewhere in the sandbox:
                // resolve the UNK against the source of truth and wire Resolves.
                if let Some(u) = self.membrane.open_unks.iter_mut().find(|x| x.id == uid) { u.resolved = true; }
                let seq = self.sandbox.ledger.append(uroot, degree, LedgerPayload::UnkResolved { id: uid });
                let rn = self.sandbox.graph.add_node(seq);
                if let Some(a) = self.sandbox.root(uroot).and_then(|r| r.anchor_node) { self.sandbox.graph.link(a, rn, Relationship::Resolves); }
                if let Some(r) = self.sandbox.root_mut(uroot) { r.done = true; }
                traces.push(StepTrace {
                    root: uroot, kind: RootKind::Unk, degree, crossed: false,
                    obs: 0, deltas: 0, unks_raised: 0, invalids: 0, converted: 0,
                    bridge: self.bridge_of(self.default_pair.unwrap()), behavior: None,
                    cup_stage: self.pairs[&self.default_pair.unwrap()].cup.phase,
                    recirculated: self.pairs[&self.default_pair.unwrap()].cup.recirculated, aftermath_applied: false,
                    note: format!("shell {degree}: RESOLVED (information available in sandbox)"),
                });
            } else {
                // feed the UNK back into the FULL loop → peel a shell: decompose into
                // sub-UNKs at degree+1 (lower priority). A non-pointable probe carries
                // the awaiting topic so the loop raises the sub-UNK.
                let mut probe = Claim::plain(700_000 + uid.0, "onion probe: required element still missing");
                probe.topic = awaiting;
                let mut tr = self.primary_loop(oracle, uroot, vec![probe], false);
                tr.note = format!("shell {degree}: peeled → sub-UNK(s) at degree {}", degree + 1);
                traces.push(tr);
            }
            self.onion.sort_by_key(|(d, u, _)| (*d, u.0));
        }
        traces
    }

    /// Sandbox coherence: (resolved UNKs, still-open UNKs, coherent?). Coherent when no
    /// in-budget UNK remains unresolved (everything reachable has been resolved or the
    /// bounded UNK budget was reached).
    pub fn coherence(&self) -> (usize, usize, bool) {
        let resolved = self.membrane.open_unks.iter().filter(|u| u.resolved).count();
        let open = self.membrane.open_unks.iter().filter(|u| !u.resolved).count();
        let coherent = self.onion_next().is_none();
        (resolved, open, coherent)
    }

    pub fn sandbox_stats(&self) -> (usize, usize, usize, usize) {
        let grains = self.sandbox.ledger.len();
        let edges = self.sandbox.graph.edge_count();
        let inputs = self.sandbox.roots.iter().filter(|r| !matches!(r.kind, RootKind::Unk)).count();
        let unk_roots = self.sandbox.roots.iter().filter(|r| matches!(r.kind, RootKind::Unk)).count();
        (grains, edges, inputs, unk_roots)
    }
}
// ═══════════════════════ THE WALKER — scene wrappers (delegate to fsl-scene) ═══════════════════════
// SPATIAL ROLE: fsl-mind only ASSEMBLES/WALKS; the scene math lives in fsl-scene.
impl World {
    /// Project the sandbox (truth) into the navigable 3D scene graph. The Coffee Cup
    /// BULGE sits on the originating conversational cable (lowest root id).
    pub fn project_scene(&self) -> fsl_scene::SceneGraph {
        let bulge = self.pairs.values().next().map(|ps| {
            let amp = ps.cup.window.position.max(0.2) + ps.cup.recirculated as f32;
            let cable = self.sandbox.roots.iter().map(|r| r.id).min_by_key(|r| r.0).unwrap_or(RootId(0));
            (cable, amp)
        });
        fsl_scene::project(&self.sandbox, bulge)
    }
    /// Explode the Coffee Cup bulge as a pure lens (truth unchanged).
    pub fn explode_bulge(&self, scene: &fsl_scene::SceneGraph, crossing: CrossingId, _actor: MindId) -> fsl_scene::Explosion {
        let cable = scene.cables.first().map(|c| c.root).unwrap_or(RootId(0));
        fsl_scene::explode(scene, cable, &self.pairs[&crossing].cup)
    }
}
