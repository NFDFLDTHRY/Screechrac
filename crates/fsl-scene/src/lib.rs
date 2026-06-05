//! fsl-scene — SPATIAL ROLE: THE SCENE — the entire navigable 3D scene graph.
//! Projection (sandbox → 3D), the bulge EXPLOSION lens (reveals exactly five stage
//! planes simultaneously + the full HCC-A actor row on each), and deterministic
//! traversal. The lens is PURE: `explode` takes `&` and mutates no truth.
//! (FSL Coherence Contract: Sandbox = the navigable scene graph; Ledger = truth.)

mod primitives;
pub use primitives::*;

use std::collections::HashMap;
use fsl_core::*;
use fsl_bulge::{CupStage, CupArc};

/// Which strand kind a Ledger grain projects to. Inputs/events ARE cables, not strands.
fn payload_kind(p: &LedgerPayload) -> Option<StrandKind> {
    Some(match p {
        LedgerPayload::Obs { .. } => StrandKind::Obs,
        LedgerPayload::Delta { .. } => StrandKind::Delta,
        LedgerPayload::Unk { .. } => StrandKind::Unk,
        LedgerPayload::UnkResolved { .. } => StrandKind::OnionStrand,
        LedgerPayload::Invalid { .. } => StrandKind::Invalid,
        LedgerPayload::InvalidConverted { .. } => StrandKind::Converted,
        LedgerPayload::Behavior { .. } | LedgerPayload::Transition { .. } => StrandKind::Behavior,
        LedgerPayload::Input { .. } | LedgerPayload::Event { .. } => return None,
    })
}
fn scene_stage(c: CupStage) -> SceneStage {
    match c {
        CupStage::Conditions => SceneStage::Conditions,
        CupStage::Release => SceneStage::Release,
        CupStage::Trajectory => SceneStage::Trajectory,
        CupStage::Impact => SceneStage::Impact,
        CupStage::Aftermath => SceneStage::Aftermath,
    }
}
/// The full HCC-A structure as an Actor Strand row across a plane's X-axis.
/// Cubes = structural/persistent nodes; Spheres = runtime "meaning-in-flight" states.
fn hcca_actor_row() -> Vec<ActorGlyph> {
    let items: &[(&str, Glyph)] = &[
        ("Interface", Glyph::Cube), ("RIC", Glyph::Cube), ("PFC", Glyph::Cube),
        ("Compiler", Glyph::Cube), ("StoryLedger", Glyph::Cube),
        ("MeaningEngine", Glyph::Sphere), ("EmotionEngine", Glyph::Sphere),
        ("ISL", Glyph::Cube), ("Behavior", Glyph::Cube),
    ];
    items.iter().enumerate().map(|(i, (l, g))| ActorGlyph { glyph: *g, label: (*l).into(), x: i as f32 }).collect()
}

/// Project the entire sandbox into the navigable 3D scene graph. Deterministic: cables
/// in root order; strands in ledger order (radius = degree-of-separation); flows mirror
/// the navigational Graph relationships (the SOLE connective primitive). `bulge` marks
/// the cable that carries the Coffee Cup deformation.
pub fn project(sandbox: &Sandbox, bulge: Option<(RootId, f32)>) -> SceneGraph {
    let mut sg = SceneGraph::default();
    let mut roots: Vec<&RootNode> = sandbox.roots.iter().collect();
    roots.sort_by_key(|r| r.id.0);
    for r in &roots { sg.place_cable(r.id); }

    let mut by_seq: HashMap<u64, SceneRef> = HashMap::new();
    for e in sandbox.ledger.entries() {
        match payload_kind(&e.payload) {
            None => { by_seq.insert(e.seq.0, SceneRef::Cable(e.root)); }
            Some(kind) => {
                let sid = sg.attach_strand(e.root, kind, e.degree, e.seq);
                by_seq.insert(e.seq.0, SceneRef::Strand(sid));
            }
        }
    }
    // FLOWS — the sole connective primitive — mirror the navigational Graph edges.
    for ed in &sandbox.graph.edges {
        let f = sandbox.graph.nodes.iter().find(|n| n.id == ed.from).map(|n| n.seq.0);
        let t = sandbox.graph.nodes.iter().find(|n| n.id == ed.to).map(|n| n.seq.0);
        if let (Some(f), Some(t)) = (f, t) {
            if let (Some(&fr), Some(&to)) = (by_seq.get(&f), by_seq.get(&t)) { sg.flow(fr, to, ed.rel); }
        }
    }
    if let Some((cable, amp)) = bulge { sg.place_bulge(cable, amp); }
    sg
}

/// EXPLODE the bulge: reveal EXACTLY FIVE STAGE PLANES simultaneously — Conditions,
/// Release, Trajectory, Impact, Aftermath — each carrying its three parallel domains
/// (verbatim) and the full HCC-A actor row as Cubes/Spheres on the plane's X-axis.
/// Pure LENS: reads the CupArc + the actor row; changes no truth.
pub fn explode(scene: &SceneGraph, cable: RootId, cup: &CupArc) -> Explosion {
    let center = scene.cable(cable).and_then(|c| c.bulge.as_ref().map(|b| b.center_t)).unwrap_or(1.0);
    let base = scene.cable(cable).map(|c| c.point_at(center)).unwrap_or_default();
    let stages = [CupStage::Conditions, CupStage::Release, CupStage::Trajectory, CupStage::Impact, CupStage::Aftermath];
    let actor_row = hcca_actor_row();
    let mut planes = Vec::with_capacity(5);
    for (i, st) in stages.iter().enumerate() {
        let dom = st.domains(); // verbatim three-domain text (social / language / lived)
        let origin = base.add(Vec3::new(0.0, 0.0, i as f32 * 2.0));
        let domains = vec![
            DomainGlyph { domain: Domain::Social,   label: dom.social.into(),   pos: origin },
            DomainGlyph { domain: Domain::Language, label: dom.language.into(), pos: origin.add(Vec3::new(0.0, 1.0, 0.0)) },
            DomainGlyph { domain: Domain::Lived,    label: dom.lived.into(),    pos: origin.add(Vec3::new(0.0, 2.0, 0.0)) },
        ];
        planes.push(StagePlane { stage: scene_stage(*st), origin, normal: Vec3::new(0.0, 0.0, 1.0), domains, actors: actor_row.clone() });
    }
    let _ = cup; // the arc state informs bulge amplitude; the planes ARE its explosion
    Explosion { cable, planes }
}
