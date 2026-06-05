//! fsl-scene::primitives — SPATIAL ROLE: the 3D scene-graph primitives (Cable/Strand/Bulge/Flow/StagePlane).
//!
//! Spatial methodology (the central organizing principle of this codebase):
//!   • every ROOT NODE (user input, system event, tick) is a CABLE — the primary
//!     carrier axis;
//!   • every derived element / observation / delta / unknown / onion iteration /
//!     recursive strand is a STRAND attached RADIALLY to that cable, with radial
//!     distance representing degree-of-separation from the root;
//!   • the Coffee Cup arc is a BULGE / deformation along the cable that can be
//!     exploded/collapsed as a LENS operation WITHOUT changing truth;
//!   • explosion of a bulge reveals EXACTLY FIVE STAGE PLANES simultaneously
//!     (Conditions, Release, Trajectory, Impact, Aftermath) plus the full HCC-A
//!     structures on Actor Strands (as Cubes/Spheres on the X-axis of each plane);
//!   • FLOWS are the SOLE connective primitive — nothing else may connect cables or
//!     strands;
//!   • the entire sandbox is a navigable 3D scene graph where every grain of sand has
//!     an exact spatial position, radial anchoring, and deterministic traversal.

use fsl_core::{RootId, Seq, StrandId, Relationship};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }
impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self { Vec3 { x, y, z } }
    pub fn add(self, o: Vec3) -> Vec3 { Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z) }
    pub fn scale(self, s: f32) -> Vec3 { Vec3::new(self.x * s, self.y * s, self.z * s) }
}

pub const GOLDEN_ANGLE: f32 = 2.399963;   // radians — spreads strands deterministically around a cable
pub const RADIAL_BASE: f32 = 1.0;          // radial units per degree-of-separation
pub const ANCHOR_STEP: f32 = 0.5;          // spacing of strand anchors ALONG a cable axis
pub const CABLE_SPACING: f32 = 10.0;       // parallel cables offset along X

/// A pointer to any grain of sand in the scene: a cable or a strand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SceneRef { Cable(RootId), Strand(StrandId) }

/// What a strand carries (mirrors a Ledger grain).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrandKind { Obs, Delta, Unk, Invalid, Converted, Behavior, OnionStrand }

/// CABLE — a root node as a primary carrier axis in space.
#[derive(Clone, Debug)]
pub struct Cable {
    pub root: RootId,
    pub origin: Vec3,
    pub dir: Vec3,            // unit axis (+Z)
    pub length: f32,
    pub bulge: Option<Bulge>, // the Coffee Cup deformation, if this cable carries one
}
impl Cable {
    /// A point at distance `t` ALONG the cable axis.
    pub fn point_at(&self, t: f32) -> Vec3 { self.origin.add(self.dir.scale(t)) }
}

/// STRAND — a derived element attached RADIALLY to a cable. Radial distance == the
/// degree-of-separation from the root cable.
#[derive(Clone, Debug)]
pub struct Strand {
    pub id: StrandId,
    pub cable: RootId,
    pub kind: StrandKind,
    pub seq: Seq,             // pointer into the append-only Ledger (the source of truth)
    pub degree: u32,
    pub anchor_t: f32,        // position ALONG the cable axis (where it attaches)
    pub radius: f32,          // RADIAL distance = degree-of-separation
    pub angle: f32,           // radial angle around the axis
}

/// BULGE — the Coffee Cup arc as a deformation along a cable. Explodes into 5 planes.
#[derive(Clone, Debug)]
pub struct Bulge { pub cable: RootId, pub center_t: f32, pub amplitude: f32, pub exploded: bool }

/// The five Coffee Cup stage names as a core-level enum (verbatim stage titles).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneStage {
    /// "Stage One — Conditions Before Release"
    Conditions,
    /// "Stage Two — Release"
    Release,
    /// "Stage Three — Trajectory (The Control Window)"
    Trajectory,
    /// "Stage Four — Impact"
    Impact,
    /// "Stage Five — Aftermath"
    Aftermath,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Domain { Social, Language, Lived }
/// "as Cubes/Spheres on the X-axis of each plane."
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Glyph { Cube, Sphere }

#[derive(Clone, Debug)] pub struct DomainGlyph { pub domain: Domain, pub label: String, pub pos: Vec3 }
/// An HCC-A structure rendered as a Cube/Sphere on the X-axis of a stage plane (an
/// Actor Strand laid across the plane).
#[derive(Clone, Debug)] pub struct ActorGlyph { pub glyph: Glyph, pub label: String, pub x: f32 }
impl ActorGlyph { pub fn pos_on(&self, plane: &StagePlane) -> Vec3 { plane.origin.add(Vec3::new(self.x, 0.0, 0.0)) } }

/// A stage plane revealed by exploding a bulge: its three parallel domains + the full
/// HCC-A actor row.
#[derive(Clone, Debug)]
pub struct StagePlane {
    pub stage: SceneStage,
    pub origin: Vec3,
    pub normal: Vec3,
    pub domains: Vec<DomainGlyph>,
    pub actors: Vec<ActorGlyph>,
}

/// The lens result: "Explosion of a bulge reveals exactly five stage planes
/// simultaneously." A pure projection — reading it changes no truth.
#[derive(Clone, Debug)]
pub struct Explosion { pub cable: RootId, pub planes: Vec<StagePlane> }

/// FLOW — the SOLE connective primitive. Nothing else may connect cables or strands.
#[derive(Clone, Copy, Debug)]
pub struct Flow { pub from: SceneRef, pub to: SceneRef, pub rel: Relationship }

/// THE NAVIGABLE 3D SCENE GRAPH: all cables, all strands, all flows.
#[derive(Clone, Debug, Default)]
pub struct SceneGraph { pub cables: Vec<Cable>, pub strands: Vec<Strand>, pub flows: Vec<Flow>, next_strand: u64 }
impl SceneGraph {
    /// Place a root node as a CABLE. Parallel cables are offset along X.
    pub fn place_cable(&mut self, root: RootId) -> usize {
        let idx = self.cables.len();
        self.cables.push(Cable {
            root, origin: Vec3::new(idx as f32 * CABLE_SPACING, 0.0, 0.0),
            dir: Vec3::new(0.0, 0.0, 1.0), length: 0.0, bulge: None,
        });
        idx
    }
    pub fn cable(&self, root: RootId) -> Option<&Cable> { self.cables.iter().find(|c| c.root == root) }
    fn cable_mut(&mut self, root: RootId) -> Option<&mut Cable> { self.cables.iter_mut().find(|c| c.root == root) }
    fn strand_ordinal(&self, root: RootId) -> usize { self.strands.iter().filter(|s| s.cable == root).count() }

    /// Attach a STRAND radially. radius = RADIAL_BASE*(1+degree), so radial distance IS
    /// the degree-of-separation from the root cable; angle spreads deterministically.
    pub fn attach_strand(&mut self, cable: RootId, kind: StrandKind, degree: u32, seq: Seq) -> StrandId {
        let ord = self.strand_ordinal(cable);
        let id = StrandId(self.next_strand); self.next_strand += 1;
        let anchor_t = ANCHOR_STEP * (ord as f32 + 1.0);
        let radius = RADIAL_BASE * (1.0 + degree as f32);
        let angle = (ord as f32 * GOLDEN_ANGLE) % (std::f32::consts::PI * 2.0);
        self.strands.push(Strand { id, cable, kind, seq, degree, anchor_t, radius, angle });
        if let Some(c) = self.cable_mut(cable) { if anchor_t + 1.0 > c.length { c.length = anchor_t + 1.0; } }
        id
    }
    pub fn strand(&self, id: StrandId) -> Option<&Strand> { self.strands.iter().find(|s| s.id == id) }
    /// Exact world position of a strand: along the axis by anchor_t, then radially out.
    pub fn strand_pos(&self, s: &Strand) -> Vec3 {
        let base = self.cable(s.cable).map(|c| c.point_at(s.anchor_t)).unwrap_or_default();
        base.add(Vec3::new(s.radius * s.angle.cos(), s.radius * s.angle.sin(), 0.0))
    }

    /// FLOWS ARE THE SOLE CONNECTIVE PRIMITIVE — this is the only method that connects
    /// any two grains of sand.
    pub fn flow(&mut self, from: SceneRef, to: SceneRef, rel: Relationship) {
        self.flows.push(Flow { from, to, rel });
    }

    /// Mark a cable as carrying a Coffee Cup BULGE (deformation) of given amplitude.
    pub fn place_bulge(&mut self, cable: RootId, amplitude: f32) {
        if let Some(c) = self.cable_mut(cable) {
            let center = (c.length * 0.5).max(1.0);
            c.bulge = Some(Bulge { cable, center_t: center, amplitude, exploded: false });
        }
    }

    /// Deterministic traversal from a cable: the cable, then its strands ordered by
    /// (radius, angle) — i.e., walking outward shell by shell.
    pub fn traverse(&self, cable: RootId) -> Vec<SceneRef> {
        let mut out = vec![SceneRef::Cable(cable)];
        let mut ss: Vec<&Strand> = self.strands.iter().filter(|s| s.cable == cable).collect();
        ss.sort_by(|a, b| a.radius.partial_cmp(&b.radius).unwrap().then(a.angle.partial_cmp(&b.angle).unwrap()));
        out.extend(ss.into_iter().map(|s| SceneRef::Strand(s.id)));
        out
    }
    pub fn stats(&self) -> (usize, usize, usize) { (self.cables.len(), self.strands.len(), self.flows.len()) }
}
