#![allow(dead_code)]
//! fsl-flow::river — SPATIAL ROLE: the water the flows move through (Flow). SYSTEM B: The River Model. Three regions, four sayable Shared States,
//! SPATIAL ROLE: THE WATER — the River regions/states the flows move through.
//! invariants, and failure modes — distinct granularities (3 / 4 / and the bridge's
//! 5), never collapsed. Consumed by fsl-bridge's FSM and projections.


/// Specs B2 — three primitive regions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RiverRegion {
    /// The Banks: "You can see each other. You can shout. Nothing crosses. This is
    /// normal. This is not failure."
    Banks,
    /// The Rapids (Pressure Zone): "Loud does not mean transferable."
    Rapids,
    /// The River Delta (Crossing Zone): "the only place where crossing is possible."
    /// "No delta → no crossing → no understanding."
    DeltaCrossing,
}

/// Middle Layer — the four user-facing Shared States (== bridge FSM minus S4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SharedState {
    /// "We're on opposite banks" — different OBS; no shared reference.
    OppositeBanks,
    /// "We're in the rapids" — "slow down; do not decide anything here; ask for one anchor."
    Rapids,
    /// "We're at the delta" — "at least one shared, pointable thing exists."
    Delta,
    /// "We're crossing" — "Only here does: explanation work, decisions make sense,
    /// directives become legitimate."
    Crossing,
}

/// I-B* — detected by the bridge driver (see fsl-bridge).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvariantB {
    /// I-B1: "Pressure ≠ Transfer: loudness indicates energy, not comprehension."
    PressureNeqTransfer,
    /// I-B2: "Crossing requires delta conditions: if you can't cross, you can't resolve."
    CrossingRequiresDelta,
    /// I-B3: "Conversations often start in rapids: because pressure compels engagement."
    ConversationsStartInRapids,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureB {
    /// FM-B1: "Starting in rapids → confusion amplified; content used as weapons."
    StartingInRapids,
    /// FM-B2: "Staying on banks → positional warfare; no shared object."
    StayingOnBanks,
    /// FM-B3: "Mistaking motion for progress → 'we talked a lot' but zero crossing."
    MotionMistakenForProgress,
}
