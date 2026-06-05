//! fsl-flow — SPATIAL ROLE: FLOW is the SOLE connective primitive. Everything that
//! moves a flow between cables lives here: the Proof membrane (only structure crosses),
//! the Delta-Bridge crossing FSM, and the River water the flow moves through.
//! (FSL Coherence Contract: Flow = the sole connective primitive; nothing else connects.)
pub mod river;     // SPATIAL ROLE: the water the flows move through (System B — River Model)
pub mod membrane;  // SPATIAL ROLE: the only channel across cables (System A — Proof Ledger)
pub mod bridge;    // SPATIAL ROLE: the crossing FSM that lets a flow cross cable→cable
