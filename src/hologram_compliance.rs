//! src/hologram_compliance.rs — SPATIAL ROLE: the lens/flow/edge GUARDRAILS.
//! A banned-pattern scanner enforcing the FSL Coherence Contract at runtime (and via a
//! source scan): FLOW is the sole connective primitive; the explosion lens never
//! mutates truth; the five CupEdges stay distinct; directives are gated to S3/S4;
//! radial distance equals degree-of-separation; and the three CANONICAL CONCEPTUAL
//! REFERENCE DOCUMENTS (the immutable source of truth) exist and are byte-for-byte intact.

use std::path::Path;
use fsl_core::{CrossingId, MindId};
use fsl_scene::{SceneGraph, SceneRef, RADIAL_BASE};
use fsl_flow::bridge::BridgeState;
use fsl_bulge::{CupEdge, ConditionsMakeLikelyRelease, ReleaseCommitsTrajectory, TrajectoryShapesImpact, ImpactFeedsNarrative, NarrativeRewritesConditions};
use fsl_mind::World;

pub struct Check { pub name: &'static str, pub pass: bool, pub detail: String }

// ─────────── SOURCE OF TRUTH: the three immutable conceptual references ───────────
/// The canonical documents are sacred and read-only. Each is pinned by exact byte
/// length + a dependency-free FNV-1a-64 content hash, so ANY modification (even a
/// single byte) is detected. These files are NEVER to be altered, renamed, or
/// summarized — every fidelity audit is performed against their verbatim content.
const REFERENCE_DIR: &str = "references/conceptual";
struct ReferenceDoc { name: &'static str, len: u64, fnv1a: u64 }
const REFERENCE_DOCS: &[ReferenceDoc] = &[
    ReferenceDoc { name: "Human Cognitive Compiler Architecture.txt", len: 12084, fnv1a: 0xfc86ac50d82adcf1 },
    ReferenceDoc { name: "Where the Water Is Loud.txt",               len: 19862, fnv1a: 0x7e2f59cbc87ab999 },
    ReferenceDoc { name: "The Coffee Cup (1).txt",                    len: 7623,  fnv1a: 0x9200d5b6b79b84b1 },
];
/// FNV-1a 64-bit — a small, deterministic content fingerprint (no external crate).
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes { h ^= *b as u64; h = h.wrapping_mul(0x0000_0100_0000_01b3); }
    h
}

fn endpoint_ok(scene: &SceneGraph, r: SceneRef) -> bool {
    match r { SceneRef::Cable(rid) => scene.cable(rid).is_some(), SceneRef::Strand(sid) => scene.strand(sid).is_some() }
}

pub fn scan(world: &World, scene: &SceneGraph, crossing: CrossingId) -> Vec<Check> {
    let mut out = vec![];

    // 1) FLOW is the SOLE connective primitive: every flow endpoint resolves, and the
    //    scene primitives expose no other connective method (banned-pattern source scan).
    let endpoints_ok = scene.flows.iter().all(|f| endpoint_ok(scene, f.from) && endpoint_ok(scene, f.to));
    let src = include_str!("../crates/fsl-scene/src/primitives.rs");
    let banned = src.contains("pub fn connect(") || src.contains("pub fn join(") || src.contains("pub fn attach_to(");
    out.push(Check { name: "flow_is_sole_connective", pass: endpoints_ok && !banned,
        detail: format!("{} flows; all endpoints valid; banned-connective-method present={}", scene.flows.len(), banned) });

    // 2) The explosion LENS mutates no truth: ledger length identical before/after.
    let before = world.sandbox.ledger.len();
    let exp = world.explode_bulge(scene, crossing, MindId(0));
    let after = world.sandbox.ledger.len();
    out.push(Check { name: "lens_does_not_mutate_truth", pass: before == after && exp.planes.len() == 5,
        detail: format!("ledger {before}=={after}; planes={}", exp.planes.len()) });

    // 3) The five CupEdges are DISTINCT (no flattening into identical arrows).
    let verbs = [ConditionsMakeLikelyRelease.verb(), ReleaseCommitsTrajectory.verb(), TrajectoryShapesImpact.verb(), ImpactFeedsNarrative.verb(), NarrativeRewritesConditions.verb()];
    let distinct = { let mut v = verbs.to_vec(); v.sort(); v.dedup(); v.len() == 5 };
    out.push(Check { name: "five_distinct_cup_edges", pass: distinct, detail: format!("{:?}", verbs) });

    // 4) directives_legitimate() is true ONLY in S3 Crossing / S4 Bank-Rebuild.
    let g = |s: BridgeState| s.directives_legitimate();
    let gated = !g(BridgeState::S0Banks) && !g(BridgeState::S1Rapids) && !g(BridgeState::S2Delta) && g(BridgeState::S3Crossing) && g(BridgeState::S4BankRebuild);
    out.push(Check { name: "directives_only_S3_S4", pass: gated, detail: "S0/S1/S2=false; S3/S4=true".into() });

    // 5) RADIAL anchoring: radius == RADIAL_BASE*(1+degree) for every strand.
    let radial_ok = scene.strands.iter().all(|s| (s.radius - RADIAL_BASE * (1.0 + s.degree as f32)).abs() < 1e-3);
    out.push(Check { name: "radial_distance_equals_degree", pass: radial_ok, detail: format!("{} strands checked", scene.strands.len()) });

    // 6) SOURCE OF TRUTH: the three canonical reference documents EXIST in
    //    references/conceptual/ and are byte-for-byte UNMODIFIED (length + content hash).
    out.push(reference_docs_intact());

    out
}

/// Verify the three immutable conceptual references are present and unaltered. The
/// directory is resolved relative to the workspace root (compiled-in), so the check is
/// correct regardless of the current working directory.
pub fn reference_docs_intact() -> Check {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(REFERENCE_DIR);
    let mut intact = 0usize;
    let mut problems: Vec<String> = vec![];
    for doc in REFERENCE_DOCS {
        let path = root.join(doc.name);
        match std::fs::read(&path) {
            Err(_) => problems.push(format!("MISSING {}", doc.name)),
            Ok(bytes) => {
                if bytes.len() as u64 != doc.len {
                    problems.push(format!("LENGTH-CHANGED {} ({} != {})", doc.name, bytes.len(), doc.len));
                } else if fnv1a64(&bytes) != doc.fnv1a {
                    problems.push(format!("CONTENT-MODIFIED {}", doc.name));
                } else {
                    intact += 1;
                }
            }
        }
    }
    Check {
        name: "reference_docs_immutable",
        pass: problems.is_empty(),
        detail: if problems.is_empty() {
            format!("{intact}/3 canonical references present and byte-for-byte intact")
        } else {
            problems.join("; ")
        },
    }
}
