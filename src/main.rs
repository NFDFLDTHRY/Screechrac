//! FSL harness — the SANDBOX model end to end. A realistic multi-tick run that:
//! SPATIAL ROLE: THE WALKER — walks the 3D scene graph end to end.
//!   • creates root nodes from user inputs (the grains of sand) and processes input
//!     root nodes in PARALLEL (neither blocks the other),
//!   • NON-BLOCKINGLY triggers an UNK root when its information appears anywhere in the
//!     sandbox (here: produced by a DIFFERENT root),
//!   • runs the ONION SHELL — selecting UNKs by degree-of-separation (closest first),
//!     feeding each back into the FULL Primary Operational Loop, peeling outward at
//!     degree+1 (progressively lower priority) until sandbox COHERENCE,
//!   • crosses the Delta Bridge (S0→…→S3 where directives become legitimate→S4),
//!     recirculates a long-lived CupArc via apply_aftermath, shows the inner+outer
//!     HCC-A loops, an emotion STALL, and two minds with different Params diverging.
//!
//! Ledger = append-only source of truth. Graph = navigation only. The internal LLM
//! never holds context and never calls a function; every call gets fresh context
//! (ledger/graph/sandbox + current root + degree-of-separation).

use std::collections::HashMap;
use fsl_core::*;
use fsl_scene::*;
use fsl_flow::membrane::ExpectedRule;
use fsl_flow::bridge::{BridgeState, ONE_RULE};
use fsl_actor::*;
use fsl_mind::{World, Mind, StepTrace};
use fsl_llm::DeterministicOracle;
mod hologram_compliance;

fn span(q: &str, c: &str) -> Span { Span { quote: q.into(), context: c.into() } }
fn plain_topic(id: u64, text: &str, topic: u64) -> Claim { let mut c = Claim::plain(id, text); c.topic = Some(TopicId(topic)); c }

fn directive_params() -> Params {
    Params { meaning_style: MeaningStyle::Directive, identity_mode: IdentityMode::Sovereign,
        emotion_mode: EmotionMode::DecisionDriving,
        arbitration: Arbitration { raw_vs_template: RawVsTemplate::RawWins, identity_vs_reality: IdentityVsReality::RealityYields },
        pruning: Pruning { perceptual: PruneLevel::None, prefilter: PruneLevel::Low, postcompile: PruneLevel::None },
        ..Params::default() }
}
fn descriptive_params() -> Params {
    Params { meaning_style: MeaningStyle::Descriptive, identity_mode: IdentityMode::Fragmented,
        emotion_mode: EmotionMode::Ignorable,
        arbitration: Arbitration { raw_vs_template: RawVsTemplate::TemplateWins, identity_vs_reality: IdentityVsReality::IdentityYields },
        pruning: Pruning { perceptual: PruneLevel::Medium, prefilter: PruneLevel::High, postcompile: PruneLevel::Low },
        ..Params::default() }
}

fn step_line(t: &StepTrace) {
    println!("   root {:?} [{:?}, deg {}] cross={} OBS={} Δ={} UNK+={} INV={} conv={} bridge={:?} cup={:?} beh={:?} {}",
        t.root, t.kind, t.degree, t.crossed, t.obs, t.deltas, t.unks_raised, t.invalids, t.converted,
        t.bridge, t.cup_stage, t.behavior, if t.note.is_empty() { "" } else { &t.note });
}

fn main() {
    let oracle = DeterministicOracle::default();
    let policy = InvalidPolicy { stop_words: vec!["you never".into(), "obviously".into(), "you always".into()] };
    let unk_policy = UnkPolicy::BoundedBudget { max_open_unk: 3, max_strand_depth: 2 };
    let mut world = World::new(policy, unk_policy);

    let b_rules = vec![ExpectedRule { id: RuleId(1), topic: TopicId(7), expected_stance: 1, description: "expects agreement on the deadline".into() }];
    let a = world.spawn(None, directive_params(), vec![]);
    let b = world.spawn(None, descriptive_params(), b_rules);
    let _c = world.spawn(Some(a), descriptive_params(), vec![]); // minds spawn minds (self-referential)
    let x = world.open_pair(1, a, b, /*stakes=*/false);
    world.set_default_pair(x, a, b);

    println!("ONE RULE: {ONE_RULE}\n");

    // ════════ 1) PARALLEL ROOT NODES + NON-BLOCKING UNK TRIGGER ════════
    // r0 (input) raises UNKs but does not block r1; r1 (input) produces the OBS that
    // r0's UNK is waiting on — a cross-root, non-blocking trigger.
    println!("════ 1) PARALLEL ROOT NODES + NON-BLOCKING TRIGGER ════");
    let r0 = world.ingest_input("the deadline slipped and it matters");
    let r1 = world.ingest_input("here is the delivery record for the schedule");
    let mut inputs: HashMap<RootId, (Vec<Claim>, bool)> = HashMap::new();
    inputs.insert(r0, (vec![
        plain_topic(1001, "the deadline slipped and it matters", 7),   // non-pointable → UNK awaiting topic 7
        plain_topic(1002, "something else feels off too", 5),          // non-pointable → UNK awaiting topic 5 (never supplied)
        plain_topic(1003, "you never deliver on time", 9),             // stop-word → INVALID awaiting topic 9
    ], false));
    inputs.insert(r1, (vec![
        Claim::pointable(1004, "the deadline was Friday per the email", span("the deadline was Friday", "thread: schedule"), 7, 1), // OBS topic 7
        Claim::pointable(1005, "here is the delivery log for last sprint", span("delivery log", "thread: record"), 9, 1),           // OBS topic 9 → converts the INVALID
    ], false));

    let traces = world.run_inputs(&oracle, &inputs); // steps r0 and r1 in one parallel pass
    for t in &traces { step_line(t); }
    println!("   ── non-blocking trigger result (UNK roots now eligible because their info appeared) ──");
    for r in &world.sandbox.roots {
        if matches!(r.kind, RootKind::Unk) {
            println!("      UNK root {:?} deg {} awaiting {:?} eligible={}", r.id, r.degree, r.awaiting_topic, r.eligible);
        }
    }

    // ════════ 2) ONION SHELL recursion + sandbox COHERENCE ════════
    println!("\n════ 2) ONION SHELL (degree-of-separation priority) → COHERENCE ════");
    let onion = world.onion_resolve(&oracle, 16);
    for t in &onion { step_line(t); }
    let (resolved, open, coherent) = world.coherence();
    let (grains, edges, inputs_n, unk_roots) = world.sandbox_stats();
    println!("   coherence: resolved UNKs={resolved}  open UNKs={open}  coherent={coherent}");
    println!("   sandbox: grains(ledger entries)={grains}  graph edges={edges}  input/event roots={inputs_n}  UNK roots={unk_roots}");

    // ════════ 3) DELTA-BRIDGE CROSSING (both DELTA forms) ════════
    println!("\n════ 3) BRIDGE CROSSING + both DELTA forms ════");
    let r2 = world.ingest_input("but we shipped Thursday, ahead of it");
    // pointable, OPPOSED stance on topic 7 → OBS-vs-OBS DELTA + OBS-vs-RULE DELTA (B expects +1).
    let t2 = world.primary_loop(&oracle, r2, vec![
        Claim::pointable(1006, "but we shipped Thursday, ahead of it", span("we shipped Thursday", "thread: schedule"), 7, -1),
    ], false);
    step_line(&t2);
    let s4 = world.resolve_crossing(x);
    println!("   resolved DELTAs → bridge {:?} (Bank-Rebuild: decisions/actions safe)", s4);

    // ════════ 4) DIVERGENCE — same crossing, different Params ════════
    println!("\n════ 4) DIVERGENCE (Params steer behavior) ════");
    let bridge_now = world.bridge_of(x);
    let payload = StructurePayload { from: a, claims: vec![
        Claim::pointable(1007, "decision: hold the ship date", span("hold the ship date", "thread: schedule"), 7, 1)] };
    let beh_a = { let m: &mut Mind = world.minds.get_mut(&a).unwrap(); m.receive(payload.clone()); m.run_pass(&oracle, bridge_now, 0.0, RootCtx::default()).behavior };
    let beh_b = { let m: &mut Mind = world.minds.get_mut(&b).unwrap(); m.receive(payload.clone()); m.run_pass(&oracle, bridge_now, 0.0, RootCtx::default()).behavior };
    println!("   bridge {:?} (directives legitimate={})", bridge_now, bridge_now.directives_legitimate());
    println!("   Mind A (directive/sovereign)   → {:?}", beh_a);
    println!("   Mind B (descriptive/fragmented) → {:?}", beh_b);

    // ════════ 5) EMOTION STALL PATH ════════
    println!("\n════ 5) EMOTION STALL PATH ════");
    let stall = world.spawn(None, Params { meaning_style: MeaningStyle::Evaluative, identity_mode: IdentityMode::Coherent,
        emotion_mode: EmotionMode::StoryRewriting, ..Params::default() }, vec![]);
    {
        let m = world.minds.get_mut(&stall).unwrap();
        m.receive(StructurePayload { from: a, claims: vec![Claim::pointable(1100, "a fact with no permitted action", span("a fact", "stall"), 7, 1)] });
        let pr = m.run_pass(&oracle, BridgeState::S0Banks, 0.0, RootCtx::default());
        println!("   Coherent+Evaluative mind: emotion(settle)={:?}  behavior={:?}", pr.last_settle, pr.behavior);
    }

    // ════════ 6) CUP RECIRCULATION (apply_aftermath writes back per-pair) ════════
    println!("\n════ 6) RECIRCULATION (CupArc around the loop; apply_aftermath) ════");
    for k in 1..=5u64 {
        let rk = world.ingest_input("follow-up exchange");
        let tr = world.primary_loop(&oracle, rk, vec![
            Claim::pointable(3000 + k, "follow-up, pointable", span("follow-up", "loop"), 7, 1)], k == 3);
        println!("   tick {k}: cup={:?} recirculated={} aftermath_applied={} bridge={:?}",
            tr.cup_stage, tr.recirculated, tr.aftermath_applied, tr.bridge);
    }
    let ps = &world.pairs[&x];
    println!("   per-pair write-back: conditions={:?} release_threshold={:.2} intervention={:?}",
        ps.conditions, ps.release_threshold, ps.intervention);

    // ════════ final sandbox snapshot ════════
    let (grains, edges, inputs_n, unk_roots) = world.sandbox_stats();
    let (resolved, open, coherent) = world.coherence();
    println!("\n════ SANDBOX SNAPSHOT ════");
    println!("   grains(ledger)={grains}  graph edges={edges}  input/event roots={inputs_n}  UNK roots={unk_roots}");
    println!("   UNKs resolved={resolved}  open={open}  coherent={coherent}  cable strands={}", world.cable.strand_count());

    // ════════ 7) 3D SCENE GRAPH — CABLES / STRANDS (radial = degree) / FLOWS ════════
    // Project the whole sandbox (truth) into the navigable scene graph: every root is a
    // CABLE; every grain is a STRAND anchored radially by degree-of-separation; every
    // relationship is a FLOW (the sole connective primitive).
    println!("\n════ 7) 3D SCENE GRAPH (cables / strands / bulge) ════");
    let scene = world.project_scene();
    let (n_cables, n_strands, n_flows) = scene.stats();
    println!("   cables={n_cables}  strands={n_strands}  flows={n_flows}  (flows are the sole connective primitive)");
    for c in scene.cables.iter().take(3) {
        let bulge = c.bulge.as_ref().map(|b| format!("BULGE@t={:.1} amp={:.1}", b.center_t, b.amplitude)).unwrap_or_else(|| "—".into());
        println!("   cable {:?}: origin=({:.0},{:.0},{:.0}) dir=(0,0,1) len={:.1} {}", c.root, c.origin.x, c.origin.y, c.origin.z, c.length, bulge);
    }
    // Deterministic traversal of the originating cable: walk outward shell by shell.
    let cable0 = scene.cables[0].root;
    println!("   deterministic traversal of {:?} (radius = degree-of-separation):", cable0);
    for r in scene.traverse(cable0).into_iter().take(6) {
        match r {
            SceneRef::Cable(rid) => println!("      ↳ CABLE {:?} (root carrier axis)", rid),
            SceneRef::Strand(sid) => {
                if let Some(s) = scene.strand(sid) {
                    let p = scene.strand_pos(s);
                    println!("      ↳ STRAND {:?} {:?} deg={} radius={:.1} pos=({:.2},{:.2},{:.2})", sid, s.kind, s.degree, s.radius, p.x, p.y, p.z);
                }
            }
        }
    }

    // ════════ 8) BULGE EXPLOSION — five stage planes + HCC-A actor glyphs ════════
    // A lens operation: explode the Coffee Cup bulge to reveal all five stage planes at
    // once, each with its three parallel domains and the HCC-A structure as Cubes/
    // Spheres on the plane's X-axis. Truth (the ledger) is unchanged by the lens.
    println!("\n════ 8) BULGE EXPLOSION (lens — truth unchanged) ════");
    let truth_before = world.sandbox.ledger.len();
    let exp = world.explode_bulge(&scene, x, a);
    println!("   exploded bulge on cable {:?} → {} stage planes revealed simultaneously:", exp.cable, exp.planes.len());
    for pl in &exp.planes {
        let q = match pl.stage {
            SceneStage::Conditions => "Why was a fall possible.",
            SceneStage::Release => "When did motion begin.",
            SceneStage::Trajectory => "When could this outcome still be altered.",
            SceneStage::Impact => "What did reality produce.",
            SceneStage::Aftermath => "What this event becomes once it enters memory and system behavior.",
        };
        println!("   ── plane {:?} @z={:.0}  Q: \"{}\"", pl.stage, pl.origin.z, q);
        for d in &pl.domains { println!("        domain {:?}: \"{}\"", d.domain, d.label); }
        let glyphs: Vec<String> = pl.actors.iter().map(|a| format!("{}{}", match a.glyph { Glyph::Cube => "▢", Glyph::Sphere => "○" }, a.label)).collect();
        println!("        HCC-A actor row (X-axis): {}", glyphs.join("  "));
    }
    let truth_after = world.sandbox.ledger.len();
    println!("   truth unchanged by lens: ledger {} == {} → {}", truth_before, truth_after, truth_before == truth_after);

    // ════════ 9) 3D SCENE TOUR — walk every cable, strand, bulge, flow ════════
    println!("\n════ 9) 3D SCENE TOUR (walk every cable / strand / bulge / flow) ════");
    for c in &scene.cables {
        let n_strands = scene.strands.iter().filter(|s| s.cable == c.root).count();
        let bulge = if c.bulge.is_some() { "  ◆BULGE" } else { "" };
        if n_strands > 0 || c.bulge.is_some() {
            println!("   CABLE {:?} @x={:.0} len={:.1} strands={}{}", c.root, c.origin.x, c.length, n_strands, bulge);
        }
    }
    // flows grouped by relationship (the sole connective primitive)
    use std::collections::BTreeMap;
    let mut by_rel: BTreeMap<String, usize> = BTreeMap::new();
    for f in &scene.flows { *by_rel.entry(format!("{:?}", f.rel)).or_insert(0) += 1; }
    let rels: Vec<String> = by_rel.iter().map(|(k, v)| format!("{k}×{v}")).collect();
    println!("   FLOWS (sole connective primitive) by relationship: {}", rels.join(", "));

    // ════════ 10) HOLOGRAM COMPLIANCE — banned-pattern scan ════════
    println!("\n════ 10) HOLOGRAM COMPLIANCE (Coherence Contract guardrails) ════");
    let checks = hologram_compliance::scan(&world, &scene, x);
    let mut all = true;
    for ch in &checks { all &= ch.pass; println!("   [{}] {} — {}", if ch.pass { "PASS" } else { "FAIL" }, ch.name, ch.detail); }
    println!("   ── coherence contract: {} ──", if all { "ALL CHECKS PASS" } else { "VIOLATION DETECTED" });
}
