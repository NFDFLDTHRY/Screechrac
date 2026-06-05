#![allow(dead_code)]
//! fsl-flow::membrane — SPATIAL ROLE: the ONLY channel across cables (Flow). SYSTEM A: The Proof Ledger, the canonical F-functions, and the MEMBRANE.
//! SPATIAL ROLE: FLOWS BETWEEN CABLES — the Proof membrane is the only channel across cables (only structure crosses).
//!
//! Where the Water Is Loud: "It is a crossing manual."
//! Per EW: the Proof Ledger "is the membrane that realizes 'only structure crosses
//! between minds.'" The membrane CALLS the Oracle (consult-only) to classify, then
//! APPLIES the returned element arrays. I-A1 pointability is enforced. INVALID is
//! convertible and actually converts when an anchor arrives later.

use std::collections::HashMap;
use fsl_core::*;

// ───────────────────────────── OBS ─────────────────────────────
/// OBS — Observations. "Something that can be pointed at." "If it can't be pointed
/// at, it is not OBS." The pointer is stored on the hot path as `Box<dyn Pointer>`
/// (the open format seam), NOT flattened to a String.
#[derive(Clone, Debug)]
pub struct Obs {
    pub id: ObsId,
    pub claim: ClaimId,
    pub pointer: Box<dyn Pointer>,
    pub inv: Inventory,
    pub topic: Option<TopicId>,
    pub stance: Option<i8>,
}

/// An expectation a single mind holds; an OBS may mismatch it → the second DELTA form.
#[derive(Clone, Debug)]
pub struct ExpectedRule { pub id: RuleId, pub topic: TopicId, pub expected_stance: i8, pub description: String }

// ───────────────────────────── DELTA ─────────────────────────────
/// DELTA — Differences. "Where two observations don't match."
/// Specs A2: "mismatch between two OBS inventories (or between OBS and an expected
/// rule)." BOTH forms are constructed by the membrane.
#[derive(Clone, Debug)]
pub enum Delta {
    /// "My OBS: 'You said A.' Your OBS: 'I said B.' DELTA: A ≠ B." (two OBS nodes)
    ObsVsObs { id: DeltaId, a: ObsId, b: ObsId, topic: Option<TopicId> },
    /// "...or between OBS and an expected rule."
    ObsVsRule { id: DeltaId, obs: ObsId, rule: RuleId },
}
impl Delta {
    pub fn id(&self) -> DeltaId { match self { Delta::ObsVsObs { id, .. } => *id, Delta::ObsVsRule { id, .. } => *id } }
}

// ───────────────────────────── UNK ─────────────────────────────
/// UNK — Unknowns. "Something required for understanding that is missing."
/// Specs: "Must specify what would become OBS and what DELTAs would be resolved by
/// that OBS." `resolves` is populated when the UNK is raised against specific DELTAs.
#[derive(Clone, Debug)]
pub struct Unk {
    pub id: UnkId,
    pub about_claim: Option<ClaimId>,
    pub need: Vec<RequiredElement>,
    pub would_become_obs: ObsSpec,
    pub resolves: Vec<DeltaId>,
    pub origin: Anchor,
    /// the topic an arriving pointer must match to RESOLVE this UNK (across ticks).
    pub awaiting_topic: Option<TopicId>,
    pub resolved: bool,
}
#[derive(Clone, Debug)] pub struct RequiredElement { pub describes: String }
#[derive(Clone, Debug)] pub struct ObsSpec { pub describes: String }

// ───────────────────────────── INVALID ─────────────────────────────
/// INVALID — Non-Computable Input. "Statements with no usable meaning." "These are
/// not wrong. They are meaningless UNTIL converted into OBS, DELTA, or UNK." → NOT
/// terminal: it carries a route-back AND it is stored so it can convert later.
#[derive(Clone, Debug)]
pub struct Invalid {
    pub claim: ClaimId,
    pub from: MindId,
    pub text: String,
    pub reason: String,
    pub route_back: RouteBack,
    /// the topic whose pointer would convert this INVALID into OBS.
    pub awaiting_topic: Option<TopicId>,
}
#[derive(Clone, Copy, Debug)] pub enum RouteBack { ProvideObs, ProvideDelta, ProvideUnk }

// ─────────────────────── Invariants / Failure modes (System A) ───────────────────────
/// Where the Water Is Loud, A3 — ENFORCED (see `ProofMembrane::check_invariants`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvariantA {
    /// I-A1 Pointability: "No claim enters the system unless it can be pointed at or
    /// explicitly labeled UNK."
    Pointability,
    /// I-A2: "Sharedness is optional at OBS, mandatory at DELTA resolution."
    SharedAtDeltaResolution,
    /// I-A3: "unresolved UNK blocks coherent comprehension."
    UnkCannotBeIgnored,
}
/// Failure modes — DETECTED and emitted by the membrane, not just documented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureA {
    /// FM-A1: "Meta without anchors → becomes INVALID to directive thinkers."
    MetaWithoutAnchors,
    /// FM-A2: "Directive without premises → 'floating force vector'; 'can't speak here.'"
    DirectiveWithoutPremises,
    /// FM-A3: "Accusation language → INVALID; produces heat but no compute."
    AccusationLanguage,
}

// ─────────────────────────── canonical F-functions (element construction) ───────────────────────────
// These are the single source of truth for building Proof elements. fsl-bridge
// wraps them with River translations + drives the FSM; the membrane calls them.

/// F2 — Anchor(): a pointable claim → an OBS node (with its pointer). Non-pointable
/// ⇒ None (I-A1).
pub fn f2_anchor(claim: &Claim, inv: Inventory, id: ObsId) -> Option<Obs> {
    claim.pointer.as_ref().map(|p| Obs {
        id, claim: claim.id, pointer: p.clone(), inv, topic: claim.topic, stance: claim.stance,
    })
}
/// F3 — Pair(): two OBS nodes → a DELTA node (OBS-vs-OBS).
pub fn f3_pair(a: &Obs, b: &Obs, id: DeltaId) -> Delta {
    Delta::ObsVsObs { id, a: a.id, b: b.id, topic: a.topic }
}
/// F3′ — the second DELTA form: an OBS against an expected rule.
pub fn f3_pair_rule(obs: &Obs, rule: &ExpectedRule, id: DeltaId) -> Delta {
    Delta::ObsVsRule { id, obs: obs.id, rule: rule.id }
}
/// F4 — Unknowns(): a draft → a UNK entry specifying required OBS + resolved DELTAs.
pub fn f4_unknowns(d: &UnkDraft, id: UnkId, origin: Anchor, awaiting_topic: Option<TopicId>) -> Unk {
    Unk {
        id,
        about_claim: d.about_claim,
        need: d.needs.iter().map(|s| RequiredElement { describes: s.clone() }).collect(),
        would_become_obs: ObsSpec { describes: d.would_become_obs.clone() },
        resolves: d.resolves.clone(),
        origin,
        awaiting_topic,
        resolved: false,
    }
}
/// F5 — Reject(): a non-computable claim → INVALID + request for anchor/UNK.
pub fn f5_reject(claim: &Claim, from: MindId) -> Invalid {
    Invalid {
        claim: claim.id, from, text: claim.text.clone(),
        reason: "non-computable input (no usable meaning)".into(),
        route_back: RouteBack::ProvideObs,
        awaiting_topic: claim.topic,
    }
}

// ───────────────────────── Per-party inventory ─────────────────────────
#[derive(Clone, Default)]
pub struct ObsInventory { pub obs: Vec<Obs> }
impl ObsInventory {
    fn views(&self) -> Vec<ObsView> {
        self.obs.iter().map(|o| ObsView { claim: o.claim, topic: o.topic, stance: o.stance,
            from_peer: matches!(o.inv, Inventory::Peer(_)) }).collect()
    }
    fn find(&self, claim: ClaimId) -> Option<&Obs> { self.obs.iter().find(|o| o.claim == claim) }
}

// ───────────────────────────── THE MEMBRANE ─────────────────────────────
/// The Proof Ledger as the SOLE channel between minds.
#[derive(Default)]
pub struct ProofMembrane {
    pub inventories: HashMap<MindId, ObsInventory>,
    pub deltas: Vec<Delta>,
    pub open_unks: Vec<Unk>,
    pub pending_invalid: Vec<Invalid>,
    next_obs: u64,
    next_delta: u64,
    next_unk: u64,
}

/// Everything one crossing produced — rich enough to drive the FSM and trace the tick.
pub struct CrossingReport {
    pub admitted: Option<StructurePayload>,
    pub obs_admitted: Vec<ObsId>,
    pub invalids: Vec<Invalid>,
    pub unks_raised: Vec<UnkId>,
    pub deltas_built: Vec<DeltaId>,
    pub converted: Vec<ClaimId>,
    pub signal: BridgeSignal,
    pub failures: Vec<FailureA>,
    pub invariant_violations: Vec<InvariantA>,
}

impl ProofMembrane {
    pub fn new() -> Self { Self::default() }

    /// Attempt a crossing. Functions assemble fresh context, CALL the Oracle for the
    /// routing/UNK/DELTA element arrays, then APPLY them (the Oracle mutates nothing).
    pub fn cross(
        &mut self,
        cand: &CrossingCandidate,
        policy: &InvalidPolicy,
        oracle: &dyn Oracle,
        invalid_pressure: f32,
        expected_rules: &[ExpectedRule],
        tick: TickId,
        root: RootCtx,
    ) -> CrossingReport {
        let mut report = CrossingReport {
            admitted: None, obs_admitted: vec![], invalids: vec![], unks_raised: vec![],
            deltas_built: vec![], converted: vec![], signal: BridgeSignal::NothingShared,
            failures: vec![], invariant_violations: vec![],
        };

        // (0) INVALID CONVERSION: a pointable incoming claim whose topic matches a
        // pending INVALID from this sender converts that INVALID into OBS later.
        for c in cand.claims.iter().filter(|c| c.is_pointable()) {
            if let Some(pos) = self.pending_invalid.iter().position(|iv|
                iv.from == cand.from && iv.awaiting_topic.is_some() && iv.awaiting_topic == c.topic)
            {
                let iv = self.pending_invalid.remove(pos);
                report.converted.push(iv.claim);
            }
        }

        // (1) FRESH routing context → call Oracle (F1 locate, inverted).
        let rctx = RoutingContext { claims: &cand.claims, invalid_policy: policy, invalid_pressure, root };
        let routings = oracle.route(&rctx);
        let route_of = |id: ClaimId| routings.iter().find(|r| r.claim == id).map(|r| r.routing);

        // (2) Apply routing per claim, enforcing I-A1.
        let mut admitted_claims: Vec<Claim> = vec![];
        let mut any_pointable = false;
        let mut any_heat = false;
        for c in &cand.claims {
            match route_of(c.id).unwrap_or(ProofRouting::Unk) {
                ProofRouting::Invalid => {
                    let iv = f5_reject(c, cand.from);
                    report.failures.push(FailureA::AccusationLanguage); // FM-A3
                    report.invalids.push(iv.clone());
                    self.pending_invalid.push(iv);
                    any_heat = true;
                }
                ProofRouting::Obs => {
                    if c.is_pointable() {
                        let oid = ObsId(self.next_obs); self.next_obs += 1;
                        let obs = f2_anchor(c, Inventory::Peer(cand.from), oid).unwrap();
                        self.inventories.entry(cand.from).or_default().obs.push(obs);
                        report.obs_admitted.push(oid);
                        admitted_claims.push(c.clone());
                        any_pointable = true;
                    } else {
                        // I-A1 guard: routed OBS but not pointable → degrade.
                        if invalid_pressure >= 0.5 {
                            let iv = f5_reject(c, cand.from);
                            report.invalids.push(iv.clone());
                            self.pending_invalid.push(iv);
                            any_heat = true;
                        } else {
                            self.raise_unk(c, oracle, tick, root, &mut report);
                        }
                    }
                }
                ProofRouting::Unk => { self.raise_unk(c, oracle, tick, root, &mut report); }
                ProofRouting::Delta => { admitted_claims.push(c.clone()); any_pointable |= c.is_pointable(); }
            }
        }

        // (3) DELTA detection on a FRESH context (both forms). Existing OBS of the
        // RECEIVER + the sender's peer inventory; plus expected rules.
        let existing: Vec<ObsView> = {
            let mut v = self.inventories.get(&cand.to).map(|i| i.views()).unwrap_or_default();
            v.extend(self.inventories.get(&cand.from).map(|i| i.views()).unwrap_or_default());
            v
        };
        let expected_views: Vec<RuleView> = expected_rules.iter()
            .map(|r| RuleView { rule: r.id, topic: r.topic, expected_stance: r.expected_stance }).collect();
        let dctx = DeltaContext { incoming: &admitted_claims, existing: &existing, expected: &expected_views, root };
        for dd in oracle.diff(&dctx) {
            match dd.kind {
                DeltaKindDraft::ObsVsObs { a, b } => {
                    if let (Some(oa), Some(ob)) = (self.find_obs(a), self.find_obs(b)) {
                        let id = DeltaId(self.next_delta); self.next_delta += 1;
                        let d = f3_pair(&oa, &ob, id);
                        self.deltas.push(d); report.deltas_built.push(id);
                    }
                }
                DeltaKindDraft::ObsVsRule { obs, rule } => {
                    if let (Some(o), Some(r)) = (self.find_obs(obs), expected_rules.iter().find(|r| r.id == rule)) {
                        let id = DeltaId(self.next_delta); self.next_delta += 1;
                        let d = f3_pair_rule(&o, r, id);
                        self.deltas.push(d); report.deltas_built.push(id);
                    }
                }
            }
        }

        // (4) Choose the BridgeSignal that drives the FSM this tick.
        report.signal =
            if !report.deltas_built.is_empty() { BridgeSignal::PairedDeltaExists }
            else if any_pointable { BridgeSignal::AnchorPresented }
            else if any_heat { BridgeSignal::HeatWithoutAnchors }
            else if !self.open_unks.iter().all(|u| u.resolved) { BridgeSignal::UnksListed }
            else { BridgeSignal::NothingShared };

        // (5) Only STRUCTURE crosses: admitted (pointable) claims become the payload.
        if !admitted_claims.is_empty() {
            report.admitted = Some(StructurePayload { from: cand.from, claims: admitted_claims });
        }

        // (6) Enforce invariants (I-A1 etc.) and record any violation.
        report.invariant_violations = self.check_invariants();
        report
    }

    fn raise_unk(&mut self, c: &Claim, oracle: &dyn Oracle, tick: TickId, root: RootCtx, report: &mut CrossingReport) {
        let uctx = UnkContext { claims: std::slice::from_ref(c), root };
        let drafts = oracle.unknowns(&uctx);
        for d in drafts {
            let id = UnkId(self.next_unk); self.next_unk += 1;
            let origin = Anchor { tick, node: NodeId(0), parent_unk: None };
            let unk = f4_unknowns(&d, id, origin, c.topic);
            self.open_unks.push(unk);
            report.unks_raised.push(id);
        }
    }

    fn find_obs(&self, claim: ClaimId) -> Option<Obs> {
        self.inventories.values().find_map(|inv| inv.find(claim).cloned())
    }
    /// Topic of an admitted OBS (so the Ledger/Graph can record it for non-blocking
    /// UNK triggering).
    pub fn obs_topic(&self, id: ObsId) -> Option<TopicId> {
        self.inventories.values().flat_map(|i| i.obs.iter()).find(|o| o.id == id).and_then(|o| o.topic)
    }

    /// Resolve any open UNK whose `awaiting_topic` is now satisfied by an admitted,
    /// pointable claim — UNK resolution ACROSS TICKS. Returns resolved UNK ids.
    pub fn resolve_unks_with(&mut self, structure: &StructurePayload) -> Vec<UnkId> {
        let topics: Vec<TopicId> = structure.claims.iter().filter_map(|c| if c.is_pointable() { c.topic } else { None }).collect();
        let mut resolved = vec![];
        for u in self.open_unks.iter_mut().filter(|u| !u.resolved) {
            if let Some(t) = u.awaiting_topic {
                if topics.contains(&t) { u.resolved = true; resolved.push(u.id); }
            }
        }
        resolved
    }

    /// I-A1 / I-A3 enforcement. Every stored OBS must have a pointer (I-A1). An UNK
    /// that stays unresolved is recorded against I-A3 (it "blocks comprehension").
    pub fn check_invariants(&self) -> Vec<InvariantA> {
        let mut v = vec![];
        let i_a1_ok = self.inventories.values().all(|inv| inv.obs.iter().all(|o| !o.pointer.anchor().is_empty()));
        if !i_a1_ok { v.push(InvariantA::Pointability); }
        if self.open_unks.iter().any(|u| !u.resolved) { v.push(InvariantA::UnkCannotBeIgnored); }
        v
    }
}
