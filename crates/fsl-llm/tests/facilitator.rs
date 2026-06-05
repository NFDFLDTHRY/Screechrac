//! SPATIAL ROLE: OFF-AXIS ENGINE tests — the Facilitator cognition classifies intent,
//! raises clarifying UNKs for missing slots via the inverted Oracle, and holds no state.

use fsl_llm::facilitator::{classify, slot_topic, structure_job, Preset};
use fsl_llm::DeterministicOracle;

#[test]
fn classifies_intent_structurally() {
    assert_eq!(classify("my car has a flat tire on the highway"), Preset::Roadside);
    assert_eq!(classify("need a couch moved to my new place"), Preset::Moving);
    assert_eq!(classify("deliver food from the diner to my office"), Preset::Delivery);
    assert_eq!(classify("can someone do a grocery store run"), Preset::Errand);
    assert_eq!(classify("ffffff zzz qqq"), Preset::FreeForm);
}

#[test]
fn vague_request_raises_clarifying_unks() {
    let oracle = DeterministicOracle::default();
    let p = structure_job(&oracle, "I need some help");
    // Labor preset with under-specified slots → The Listener must ask.
    assert!(!p.missing.is_empty(), "a vague request must produce clarifying questions");
    // every clarify carries an Oracle-minted would_become_obs (the UNK spec).
    assert!(p.missing.iter().all(|c| !c.would_become_obs.is_empty()));
}

#[test]
fn well_specified_request_needs_less_clarification() {
    let oracle = DeterministicOracle::default();
    let vague = structure_job(&oracle, "move stuff");
    let precise = structure_job(
        &oracle,
        "move a couch from 5th street to 12 oak road today, two boxes too",
    );
    assert!(
        precise.missing.len() <= vague.missing.len(),
        "more anchors ⇒ fewer open UNKs (precise={}, vague={})",
        precise.missing.len(),
        vague.missing.len()
    );
}

#[test]
fn slot_topics_are_stable_and_distinct() {
    assert_eq!(slot_topic("pickup"), slot_topic("pickup"));
    assert_ne!(slot_topic("pickup"), slot_topic("dropoff"));
}

#[test]
fn engine_is_stateless_and_deterministic() {
    let oracle = DeterministicOracle::default();
    let a = structure_job(&oracle, "deliver a package to 200 main st today");
    let b = structure_job(&oracle, "deliver a package to 200 main st today");
    assert_eq!(a.preset, b.preset);
    assert_eq!(a.missing.len(), b.missing.len());
    assert_eq!(a.pointable, b.pointable);
}
