#![allow(dead_code)]
//! fsl-flow::bridge — SPATIAL ROLE: the crossing FSM that lets a flow cross cable→cable. The "Delta Bridge": the OPERATIONAL FSM between System A (Proof) and
//! SPATIAL ROLE: THE CROSSING FLOW — the Delta-Bridge FSM that lets a flow cross from cable to cable.
//! System B (River). Real transitions (`step`), per-state dual outputs (Proof +
//! River), the minimal function set F1–F5 (wrapping Proof's canonical element
//! constructors — single source of truth), the isomorphism, the One Rule, kernel /
//! repair sentences, and System-B failure detection.

use fsl_core::*;
use crate::membrane::{Obs, Delta, Unk, Invalid, ExpectedRule, f2_anchor, f3_pair, f3_pair_rule, f4_unknowns, f5_reject};
use crate::river::{RiverRegion, SharedState, InvariantB, FailureB};

// ─────────────────────── The FSM (S0–S4) ───────────────────────
/// Where the Water Is Loud, Specs 4.1 — five states (S4 is real, not dropped).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BridgeState {
    /// S0 — Banks (Isolated): "no shared reference objects yet."
    S0Banks,
    /// S1 — Rapids (Pressure): "urgency/heat present + anchors missing."
    S1Rapids,
    /// S2 — Delta (Crossing Setup): "one party presents a pointable element; the
    /// other acknowledges it as addressable."
    S2Delta,
    /// S3 — Crossing (Compute): "DELTA(OBS_A, OBS_B) exists and UNKs are listed."
    S3Crossing,
    /// S4 — Bank Rebuild (Action): "crossing completed → decisions/actions can occur
    /// safely."
    S4BankRebuild,
}

impl BridgeState {
    /// THE TRANSITION FUNCTION. Drives banks → rapids → delta → crossing → rebuild
    /// from the `BridgeSignal`s that F1–F5 / the membrane emit. This is what makes
    /// the bridge a state MACHINE and lets directives ever become legitimate.
    ///
    /// ```
    /// use fsl_flow::bridge::BridgeState;
    /// use fsl_core::BridgeSignal;
    /// let s = BridgeState::S0Banks.step(BridgeSignal::AnchorPresented);
    /// assert_eq!(s, BridgeState::S2Delta);
    /// // directives are legitimate ONLY at the crossing / bank-rebuild
    /// assert!(!BridgeState::S0Banks.directives_legitimate());
    /// assert!(BridgeState::S3Crossing.directives_legitimate());
    /// ```
    pub fn step(self, sig: BridgeSignal) -> BridgeState {
        use BridgeState::*;
        use BridgeSignal::*;
        match (self, sig) {
            // Heat with missing anchors always drops to the rapids ("crossing impossible").
            (_, HeatWithoutAnchors) => S1Rapids,
            // A presented, acknowledged anchor reaches the delta.
            (S0Banks, AnchorPresented) | (S1Rapids, AnchorPresented) => S2Delta,
            (S0Banks, NothingShared) => S0Banks,
            (S0Banks, _) => S0Banks,
            (S1Rapids, NothingShared) => S0Banks,
            (S1Rapids, _) => S1Rapids,
            // A paired DELTA + listed UNKs begins the crossing (compute).
            (S2Delta, PairedDeltaExists) => S3Crossing,
            (S2Delta, AnchorPresented) => S2Delta,
            (S2Delta, _) => S2Delta,
            // Resolving the DELTAs completes the crossing → rebuild banks (safe action).
            (S3Crossing, DeltasResolved) => S4BankRebuild,
            (S3Crossing, _) => S3Crossing,
            // After rebuild, a new topic resets to banks; fresh heat re-enters rapids.
            (S4BankRebuild, NothingShared) => S0Banks,
            (S4BankRebuild, _) => S4BankRebuild,
        }
    }

    /// STATE 4: "Only here does ... directives become legitimate." Legitimate at the
    /// crossing and the safe rebuild that follows it.
    pub fn directives_legitimate(self) -> bool {
        matches!(self, BridgeState::S3Crossing | BridgeState::S4BankRebuild)
    }

    /// Per-state outputs in BOTH systems, exactly as written (Specs 4.1).
    pub fn outputs(self) -> StateOutputs {
        match self {
            BridgeState::S0Banks => StateOutputs { proof: "UNK declarations + requests for OBS", river: "We're on opposite banks." },
            BridgeState::S1Rapids => StateOutputs { proof: "INVALID on accusations; demand OBS", river: "We're in rapids—crossing impossible." },
            BridgeState::S2Delta => StateOutputs { proof: "create OBS_A, OBS_B", river: "We reached the delta—crossing can start." },
            BridgeState::S3Crossing => StateOutputs { proof: "resolve DELTAs / promote UNKs to OBS", river: "We're crossing now." },
            BridgeState::S4BankRebuild => StateOutputs { proof: "Action list derived from resolved DELTAs", river: "Back to calm water; proceed." },
        }
    }

    /// Projection 5 → 3 (FSM state → River region).
    pub fn region(self) -> RiverRegion {
        match self {
            BridgeState::S0Banks => RiverRegion::Banks,
            BridgeState::S1Rapids => RiverRegion::Rapids,
            _ => RiverRegion::DeltaCrossing,
        }
    }
    /// Projection 5 → 4 (sayable Shared States = FSM minus S4 Bank-Rebuild).
    pub fn shared_state(self) -> Option<SharedState> {
        match self {
            BridgeState::S0Banks => Some(SharedState::OppositeBanks),
            BridgeState::S1Rapids => Some(SharedState::Rapids),
            BridgeState::S2Delta => Some(SharedState::Delta),
            BridgeState::S3Crossing => Some(SharedState::Crossing),
            BridgeState::S4BankRebuild => None,
        }
    }
}

pub struct StateOutputs { pub proof: &'static str, pub river: &'static str }

// ─────────────────────── System-B detection (I-B*, FM-B*) ───────────────────────
/// "Conversations often start in rapids" (I-B3) and the failure of starting there
/// (FM-B1) / staying on banks (FM-B2) / motion without crossing (FM-B3).
pub fn detect_b(prev: BridgeState, next: BridgeState, ticks_in_state: u32) -> (Vec<InvariantB>, Vec<FailureB>) {
    let mut inv = vec![]; let mut fm = vec![];
    if matches!(next, BridgeState::S1Rapids) { inv.push(InvariantB::PressureNeqTransfer); }
    if matches!(prev, BridgeState::S0Banks) && matches!(next, BridgeState::S1Rapids) { fm.push(FailureB::StartingInRapids); }
    if matches!(next, BridgeState::S0Banks) && ticks_in_state >= 2 { fm.push(FailureB::StayingOnBanks); }
    if matches!(prev, BridgeState::S2Delta) && matches!(next, BridgeState::S2Delta) && ticks_in_state >= 2 {
        fm.push(FailureB::MotionMistakenForProgress);
    }
    if !matches!(next, BridgeState::S3Crossing | BridgeState::S4BankRebuild) { inv.push(InvariantB::CrossingRequiresDelta); }
    (inv, fm)
}

// ─────────────────────── Structural isomorphism ───────────────────────
/// The ONLY written cross-system mapping (River ≈ Proof). Active data: the bridge's
/// per-state `outputs()` are cross-checked against this in tests/traces.
pub struct Isomorphism { pub river: &'static str, pub proof: &'static str }
pub const ISOMORPHISM: [Isomorphism; 3] = [
    Isomorphism { river: "River Banks", proof: "separate OBS inventories" },
    Isomorphism { river: "River Rapids / Pressure", proof: "high-rate talk with missing anchors + unresolved UNK" },
    Isomorphism { river: "River Delta (Crossing)", proof: "the moment a paired DELTA exists (two OBS nodes both can point at) and UNKs are explicitly declared" },
];

// ─────────────────────── F1–F5 with DUAL outputs ───────────────────────
// Each wraps Proof's canonical element constructor (single source of truth) and adds
// the River translation. The membrane uses the Proof constructors directly; these
// dual wrappers are the named F-interface and what the trace/driver report.
pub struct DualOut<T> { pub value: T, pub river: &'static str }

/// F1 — Locate(): a routing → (its River translation). Routing itself is decided by
/// the Oracle (`core::structural_route` in the deterministic case); F1 names it and
/// gives the River side + the BridgeSignal it implies.
pub fn f1_locate_dual(routing: ProofRouting, pointable: bool) -> DualOut<BridgeSignal> {
    let (sig, river) = match routing {
        ProofRouting::Invalid => (BridgeSignal::HeatWithoutAnchors, "shouting across the rapids"),
        ProofRouting::Obs => (if pointable { BridgeSignal::AnchorPresented } else { BridgeSignal::UnksListed }, "located the conversation"),
        ProofRouting::Delta => (BridgeSignal::PairedDeltaExists, "two banks, one pointable thing"),
        ProofRouting::Unk => (BridgeSignal::UnksListed, "fog over the water"),
    };
    DualOut { value: sig, river }
}
/// F2 — Anchor().
pub fn f2_anchor_dual(claim: &Claim, inv: Inventory, id: ObsId) -> DualOut<Option<Obs>> {
    DualOut { value: f2_anchor(claim, inv, id), river: "we're at the delta now." }
}
/// F3 — Pair() (OBS vs OBS) and F3′ (OBS vs rule).
pub fn f3_pair_dual(a: &Obs, b: &Obs, id: DeltaId) -> DualOut<Delta> {
    DualOut { value: f3_pair(a, b, id), river: "crossing is possible; start crossing." }
}
pub fn f3_pair_rule_dual(o: &Obs, r: &ExpectedRule, id: DeltaId) -> DualOut<Delta> {
    DualOut { value: f3_pair_rule(o, r, id), river: "your bank vs the map you expected." }
}
/// F4 — Unknowns().
pub fn f4_unknowns_dual(d: &UnkDraft, id: UnkId, origin: Anchor, topic: Option<TopicId>) -> DualOut<Unk> {
    DualOut { value: f4_unknowns(d, id, origin, topic), river: "still too loud / too deep to cross; widen delta." }
}
/// F5 — Reject().
pub fn f5_reject_dual(claim: &Claim, from: MindId) -> DualOut<Invalid> {
    DualOut { value: f5_reject(claim, from), river: "shouting in rapids." }
}

// ─────────────────────── Governing rules / utterances ───────────────────────
/// PART III — The One Rule That Prevents Collapse.
pub const ONE_RULE: &str =
    "If you can't point at it, you can't push on it. Pressure without orientation creates resistance. Orientation without pressure creates drift.";
/// PART IV — The Universal Repair Sentence (emitted when the bridge is in rapids).
pub fn repair_sentence() -> &'static str {
    "We're in the rapids. Let's pick one thing we can point at so we can reach the delta."
}
/// Section 7 — Interface Kernel Sentence (The Bridge Call).
pub fn kernel_sentence() -> &'static str {
    "We're in rapids—pick one pointable thing (OBS) so we can reach the delta and compute the DELTA."
}
