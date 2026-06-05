//! SPATIAL ROLE: THE ASSEMBLER/WALKER guardrail tests — drive the whole sandbox loop
//! end to end and assert every gap from the last audit is closed: real bridge stepping,
//! RIC∥PFC + 4.4 arbitration + emotion stall + Params steering, Onion Shell coherence,
//! non-blocking + cross-tick UNK resolution, Cup recirculation write-back, lens purity.

use std::collections::HashMap;

use fsl_actor::*;
use fsl_core::*;
use fsl_flow::bridge::BridgeState;
use fsl_flow::membrane::ExpectedRule;
use fsl_llm::DeterministicOracle;
use fsl_mind::World;

fn span(q: &str, c: &str) -> Span {
    Span { quote: q.into(), context: c.into() }
}
fn plain_topic(id: u64, text: &str, topic: u64) -> Claim {
    let mut c = Claim::plain(id, text);
    c.topic = Some(TopicId(topic));
    c
}
fn directive_params() -> Params {
    Params {
        meaning_style: MeaningStyle::Directive,
        identity_mode: IdentityMode::Sovereign,
        emotion_mode: EmotionMode::DecisionDriving,
        arbitration: Arbitration {
            raw_vs_template: RawVsTemplate::RawWins,
            identity_vs_reality: IdentityVsReality::RealityYields,
        },
        ..Params::default()
    }
}
fn descriptive_params() -> Params {
    Params {
        meaning_style: MeaningStyle::Descriptive,
        identity_mode: IdentityMode::Fragmented,
        emotion_mode: EmotionMode::Ignorable,
        arbitration: Arbitration {
            raw_vs_template: RawVsTemplate::TemplateWins,
            identity_vs_reality: IdentityVsReality::IdentityYields,
        },
        ..Params::default()
    }
}

fn base_world() -> (World, DeterministicOracle, MindId, MindId, CrossingId) {
    let oracle = DeterministicOracle::default();
    let policy = InvalidPolicy { stop_words: vec!["you never".into(), "obviously".into()] };
    let unk_policy = UnkPolicy::BoundedBudget { max_open_unk: 3, max_strand_depth: 2 };
    let mut world = World::new(policy, unk_policy);
    let b_rules = vec![ExpectedRule {
        id: RuleId(1),
        topic: TopicId(7),
        expected_stance: 1,
        description: "expects agreement on the deadline".into(),
    }];
    let a = world.spawn(None, directive_params(), vec![]);
    let b = world.spawn(None, descriptive_params(), b_rules);
    let x = world.open_pair(1, a, b, false);
    world.set_default_pair(x, a, b);
    (world, oracle, a, b, x)
}

#[test]
fn parallel_roots_nonblocking_trigger_then_onion_coherence() {
    let (mut world, oracle, _a, _b, _x) = base_world();
    let r0 = world.ingest_input("the deadline slipped and it matters");
    let r1 = world.ingest_input("here is the delivery record for the schedule");

    let mut inputs: HashMap<RootId, (Vec<Claim>, bool)> = HashMap::new();
    inputs.insert(
        r0,
        (
            vec![
                plain_topic(1001, "the deadline slipped and it matters", 7), // non-pointable → UNK awaiting topic 7
                plain_topic(1003, "you never deliver on time", 9),           // stop-word → INVALID awaiting topic 9
            ],
            false,
        ),
    );
    inputs.insert(
        r1,
        (
            vec![
                Claim::pointable(1004, "the deadline was Friday", span("the deadline was Friday", "schedule"), 7, 1),
                Claim::pointable(1005, "here is the delivery log", span("delivery log", "record"), 9, 1),
            ],
            false,
        ),
    );

    let traces = world.run_inputs(&oracle, &inputs);
    assert!(!traces.is_empty(), "the parallel scheduler must step both roots");

    // A different root produced the OBS the UNK was waiting on → non-blocking trigger.
    let triggered_unk_root = world
        .sandbox
        .roots
        .iter()
        .any(|r| matches!(r.kind, RootKind::Unk) && r.eligible);
    assert!(triggered_unk_root, "an UNK root must become eligible once its info appears elsewhere");

    let _onion = world.onion_resolve(&oracle, 32);
    let (_resolved, _open, coherent) = world.coherence();
    assert!(coherent, "the Onion Shell must drive the sandbox to coherence");
}

#[test]
fn bridge_reaches_bank_rebuild_through_real_steps() {
    let (mut world, oracle, _a, _b, x) = base_world();
    let r1 = world.ingest_input("first");
    world.primary_loop(
        &oracle,
        r1,
        vec![Claim::pointable(2001, "shipped on time", span("on time", "t"), 7, 1)],
        false,
    );
    let r2 = world.ingest_input("second");
    world.primary_loop(
        &oracle,
        r2,
        vec![Claim::pointable(2002, "shipped late", span("late", "t"), 7, -1)],
        false,
    );
    // an opposed OBS on the same topic builds a DELTA → the FSM steps into S3 Crossing.
    assert_eq!(world.bridge_of(x), BridgeState::S3Crossing);
    let s4 = world.resolve_crossing(x);
    assert_eq!(s4, BridgeState::S4BankRebuild, "resolving the DELTA completes the crossing");
    assert!(s4.directives_legitimate());
}

#[test]
fn params_steer_divergent_behavior_same_crossing() {
    let (mut world, oracle, a, b, x) = base_world();
    // drive the crossing to a legitimate (directive-permitting) state first.
    let r1 = world.ingest_input("first");
    world.primary_loop(&oracle, r1, vec![Claim::pointable(2001, "on time", span("a", "t"), 7, 1)], false);
    let r2 = world.ingest_input("second");
    world.primary_loop(&oracle, r2, vec![Claim::pointable(2002, "late", span("b", "t"), 7, -1)], false);
    let bridge = world.resolve_crossing(x);
    assert!(bridge.directives_legitimate());

    let payload = StructurePayload {
        from: a,
        claims: vec![Claim::pointable(3001, "decision: hold the date", span("hold", "t"), 7, 1)],
    };
    let beh_a = {
        let m = world.minds.get_mut(&a).unwrap();
        m.receive(payload.clone());
        m.run_pass(&oracle, bridge, 0.0, RootCtx::default()).behavior
    };
    let beh_b = {
        let m = world.minds.get_mut(&b).unwrap();
        m.receive(payload.clone());
        m.run_pass(&oracle, bridge, 0.0, RootCtx::default()).behavior
    };
    assert_ne!(beh_a, beh_b, "different Params must steer different behavior on the same crossing");
}

#[test]
fn emotion_stall_path_is_reachable() {
    let (mut world, oracle, a, _b, _x) = base_world();
    // Coherent identity + Evaluative meaning (beliefs change, no behavior permitted) → STALL.
    let stall = world.spawn(
        None,
        Params {
            meaning_style: MeaningStyle::Evaluative,
            identity_mode: IdentityMode::Coherent,
            emotion_mode: EmotionMode::StoryRewriting,
            ..Params::default()
        },
        vec![],
    );
    let m = world.minds.get_mut(&stall).unwrap();
    m.receive(StructurePayload {
        from: a,
        claims: vec![Claim::pointable(1100, "a fact with no permitted action", span("fact", "stall"), 7, 1)],
    });
    let pr = m.run_pass(&oracle, BridgeState::S0Banks, 0.0, RootCtx::default());
    assert_eq!(pr.last_settle, Settle::Stalled, "the emotion stall path must be exercised");
}

#[test]
fn cup_recirculation_writes_back_to_pair_state() {
    let (mut world, oracle, _a, _b, x) = base_world();
    let mut aftermath_applied = false;
    for k in 1..=8u64 {
        let rk = world.ingest_input("follow-up exchange");
        let tr = world.primary_loop(
            &oracle,
            rk,
            vec![Claim::pointable(3000 + k, "follow-up, pointable", span("follow-up", "loop"), 7, 1)],
            k == 4,
        );
        aftermath_applied |= tr.aftermath_applied;
    }
    assert!(aftermath_applied, "the Cup must recirculate and apply Aftermath at least once");
    let ps = &world.pairs[&x];
    assert!(
        ps.release_threshold != 0.5 || !ps.intervention.is_empty(),
        "Aftermath must write back into the per-pair state"
    );
}

#[test]
fn explosion_lens_does_not_mutate_ledger_truth() {
    let (mut world, oracle, a, _b, x) = base_world();
    let r = world.ingest_input("seed");
    world.primary_loop(&oracle, r, vec![Claim::pointable(5001, "pointable", span("p", "t"), 7, 1)], true);
    let scene = world.project_scene();
    let before = world.sandbox.ledger.len();
    let exp = world.explode_bulge(&scene, x, a);
    let after = world.sandbox.ledger.len();
    assert_eq!(before, after, "the explosion lens must not change truth");
    assert_eq!(exp.planes.len(), 5);
}

#[test]
fn oracle_is_stateless_and_deterministic() {
    // The inverted Oracle holds no context: identical fresh context ⇒ identical output.
    let oracle = DeterministicOracle::default();
    let claims = vec![
        Claim::pointable(1, "pointable", span("p", "c"), 7, 1),
        Claim::plain(2, "non-pointable claim"),
    ];
    let policy = InvalidPolicy::default();
    let mk = || RoutingContext {
        claims: &claims,
        invalid_policy: &policy,
        invalid_pressure: 0.0,
        root: RootCtx::default(),
    };
    let first = oracle.route(&mk());
    let second = oracle.route(&mk());
    assert_eq!(first.len(), second.len());
    for (x, y) in first.iter().zip(second.iter()) {
        assert_eq!(x.claim, y.claim);
        assert_eq!(x.routing, y.routing);
    }
}
