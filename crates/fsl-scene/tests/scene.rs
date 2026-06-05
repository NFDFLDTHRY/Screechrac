//! SPATIAL ROLE: THE SCENE guardrail tests — radial distance equals degree-of-separation,
//! the explosion lens reveals exactly five planes, and the lens is pure (reads only).

use fsl_bulge::CupArc;
use fsl_core::{CrossingId, RootId, Seq};
use fsl_scene::*;

#[test]
fn strand_radius_equals_degree_of_separation() {
    let mut sg = SceneGraph::default();
    sg.place_cable(RootId(0));
    for deg in 0..5u32 {
        let id = sg.attach_strand(RootId(0), StrandKind::Obs, deg, Seq(deg as u64));
        let s = sg.strand(id).unwrap();
        assert!(
            (s.radius - RADIAL_BASE * (1.0 + deg as f32)).abs() < 1e-6,
            "radius {} != RADIAL_BASE*(1+{deg})",
            s.radius
        );
    }
}

#[test]
fn explosion_reveals_exactly_five_planes_with_domains_and_actor_row() {
    let mut sg = SceneGraph::default();
    sg.place_cable(RootId(0));
    sg.attach_strand(RootId(0), StrandKind::Obs, 0, Seq(0));
    sg.place_bulge(RootId(0), 1.0);
    let cup = CupArc::new(CrossingId(1), false);

    let exp = explode(&sg, RootId(0), &cup);
    assert_eq!(exp.planes.len(), 5, "explosion must reveal exactly five stage planes");
    let stages: Vec<SceneStage> = exp.planes.iter().map(|p| p.stage).collect();
    assert_eq!(
        stages,
        vec![
            SceneStage::Conditions,
            SceneStage::Release,
            SceneStage::Trajectory,
            SceneStage::Impact,
            SceneStage::Aftermath
        ]
    );
    for p in &exp.planes {
        assert_eq!(p.domains.len(), 3, "each plane carries three parallel domains");
        assert_eq!(p.actors.len(), 9, "each plane carries the full HCC-A actor row");
    }
}

#[test]
fn explosion_lens_mutates_no_scene_state() {
    let mut sg = SceneGraph::default();
    sg.place_cable(RootId(0));
    sg.attach_strand(RootId(0), StrandKind::Obs, 0, Seq(0));
    sg.place_bulge(RootId(0), 1.0);
    let cup = CupArc::new(CrossingId(1), false);

    let (c0, s0, f0) = sg.stats();
    let _ = explode(&sg, RootId(0), &cup); // `explode` takes &SceneGraph — cannot mutate.
    let (c1, s1, f1) = sg.stats();
    assert_eq!((c0, s0, f0), (c1, s1, f1), "the lens is pure: scene unchanged");
}
