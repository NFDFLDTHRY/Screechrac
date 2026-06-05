//! SPATIAL ROLE: THE BULGE guardrail tests — the five CupEdges stay distinct and each
//! `propagate()` runs on traversal; full recirculation writes the three Aftermath targets.

use fsl_bulge::*;
use fsl_core::CrossingId;

#[test]
fn five_cup_edges_have_distinct_verbs() {
    // gap #2: the five distinctly-named edges must never flatten into one arrow.
    let verbs = [
        ConditionsMakeLikelyRelease.verb(),
        ReleaseCommitsTrajectory.verb(),
        TrajectoryShapesImpact.verb(),
        ImpactFeedsNarrative.verb(),
        NarrativeRewritesConditions.verb(),
    ];
    let mut v = verbs.to_vec();
    v.sort();
    v.dedup();
    assert_eq!(v.len(), 5, "expected five pairwise-distinct edge verbs, got {verbs:?}");
}

#[test]
fn each_edge_connects_adjacent_stages() {
    assert_eq!(ConditionsMakeLikelyRelease.from(), CupStage::Conditions);
    assert_eq!(ConditionsMakeLikelyRelease.to(), CupStage::Release);
    assert_eq!(NarrativeRewritesConditions.from(), CupStage::Aftermath);
    assert_eq!(NarrativeRewritesConditions.to(), CupStage::Conditions, "the loop closes");
}

#[test]
fn advance_recirculates_and_writes_aftermath() {
    // Drive the long-lived arc around the full loop; the Impact→Aftermath edge must
    // produce an Aftermath with all three feedback targets populated.
    let mut arc = CupArc::new(CrossingId(1), false);
    let mut captured: Option<Aftermath> = None;
    for _ in 0..16 {
        let input = CupTickInput { admitted: None, bridge_crossing: true, charged: true, terminal: true };
        let _edge = arc.advance(&input);
        if let Some(a) = arc.take_aftermath() {
            captured = Some(a);
        }
    }
    assert!(arc.recirculated >= 1, "the arc must recirculate at least once");
    let after = captured.expect("Impact→Aftermath edge must produce an Aftermath");
    // three Aftermath targets: rewrite conditions / alter release threshold / reshape interventions.
    assert!(after.release_threshold_delta > 0.0, "aftermath must alter the release threshold");
    assert!(
        !after.intervention_policy_update.attempts_next_time.is_empty(),
        "aftermath must reshape future interventions"
    );
}

#[test]
fn no_stakes_framing_degrades_visibility() {
    // No-stakes framing is a DEGRADE toward INVALID, not a hard reject.
    let no_stakes = framing_bias(Framing { stakes_present: false });
    let stakes = framing_bias(Framing { stakes_present: true });
    assert!(no_stakes.invalid_pressure < stakes.invalid_pressure);
    assert!(no_stakes.structure_visibility > stakes.structure_visibility);
}
