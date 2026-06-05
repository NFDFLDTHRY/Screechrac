//! SPATIAL ROLE: FLOW guardrail tests — the Delta-Bridge FSM is the single source of
//! truth for crossing state, and directives are gated to S3/S4 only.

use fsl_core::BridgeSignal::*;
use fsl_flow::bridge::BridgeState::*;

#[test]
fn step_drives_full_crossing_banks_to_rebuild() {
    // BridgeState::step is the ONLY mover of crossing state (gap #1).
    let s = S0Banks;
    let s = s.step(AnchorPresented);
    assert_eq!(s, S2Delta, "a presented, acknowledged anchor reaches the delta");
    let s = s.step(PairedDeltaExists);
    assert_eq!(s, S3Crossing, "a paired DELTA + listed UNKs begins the crossing");
    let s = s.step(DeltasResolved);
    assert_eq!(s, S4BankRebuild, "resolving DELTAs completes the crossing");
}

#[test]
fn heat_without_anchors_always_drops_to_rapids() {
    for from in [S0Banks, S2Delta, S3Crossing, S4BankRebuild] {
        assert_eq!(from.step(HeatWithoutAnchors), S1Rapids);
    }
}

#[test]
fn nothing_shared_resets_to_banks() {
    assert_eq!(S4BankRebuild.step(NothingShared), S0Banks);
    assert_eq!(S1Rapids.step(NothingShared), S0Banks);
}

#[test]
fn directives_legitimate_only_in_s3_s4() {
    assert!(!S0Banks.directives_legitimate());
    assert!(!S1Rapids.directives_legitimate());
    assert!(!S2Delta.directives_legitimate());
    assert!(S3Crossing.directives_legitimate());
    assert!(S4BankRebuild.directives_legitimate());
}
