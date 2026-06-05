#![allow(dead_code)]
//! fsl-cable — the chain / cable topology for UNK decomposition across ticks.
//! SPATIAL ROLE: RADIAL STRANDS — UNK decomposition strands attached radially to a cable (the chain/cable topology).
//! Per EW: "each tick is a root node"; an unresolved UNK "becomes cable strands
//! linked like a chain to their origin (data pointers)" — a chain/cable hybrid. The
//! decomposition is governed by the UNK-resolution SEAM (`UnkPolicy`) and is actually
//! exercised by the Mind each tick (no dead code).

use fsl_core::*;
use fsl_flow::membrane::{Unk, RequiredElement, ObsSpec};

/// A back-linked sub-UNK: a strand whose `anchor` points to its origin (parent UNK +
/// tick + node), so the cable can always be walked back to the root.
#[derive(Clone, Debug)]
pub struct Strand {
    pub id: StrandId,
    pub unk: Unk,
    pub anchor: Anchor,
    pub depth: usize,
}

/// One tick's ROOT node plus the strands that hang off it.
#[derive(Clone, Debug)]
pub struct Link { pub node: NodeId, pub tick: TickId, pub strands: Vec<Strand> }

/// The chain of per-tick roots — the cable.
#[derive(Clone, Debug, Default)]
pub struct Cable { pub links: Vec<Link>, next_strand: u64, next_node: u64 }

impl Cable {
    /// "each tick is a root node": open (or reuse) this tick's root link.
    pub fn root(&mut self, tick: TickId) -> NodeId {
        if let Some(l) = self.links.iter().find(|l| l.tick == tick) { return l.node; }
        let node = NodeId(self.next_node); self.next_node += 1;
        self.links.push(Link { node, tick, strands: vec![] });
        node
    }

    /// Decompose a parent UNK into sub-UNK strands by CALLING the Oracle (consult-only)
    /// with a freshly-assembled context, honoring the `UnkPolicy` seam. Each strand is
    /// back-linked to the parent (data pointer) and INHERITS the parent's `resolves`
    /// (the DELTAs the eventual OBS would resolve), populating that field rather than
    /// leaving it empty. Returns the new sub-UNKs (also recorded as strands).
    pub fn decompose_unk(
        &mut self,
        parent: &Unk,
        oracle: &dyn Oracle,
        policy: &UnkPolicy,
        tick: TickId,
        next_unk: &mut u64,
        root: RootCtx,
    ) -> Vec<Unk> {
        let node = self.root(tick);
        let parent_depth = self.links.iter().find(|l| l.tick == tick)
            .and_then(|l| l.strands.iter().find(|s| s.unk.id == parent.id))
            .map(|s| s.depth).unwrap_or(0);

        // Policy gate (the seam): HaltOnAnyUnk decomposes nothing further; BoundedBudget
        // caps both open count and strand depth.
        let (max_open, max_depth) = match *policy {
            UnkPolicy::HaltOnAnyUnk => (0usize, 0usize),
            UnkPolicy::BoundedBudget { max_open_unk, max_strand_depth } => (max_open_unk, max_strand_depth),
            // The seam is #[non_exhaustive]: any future resolution rule is treated
            // conservatively as "halt" until it is explicitly given semantics here.
            _ => (0usize, 0usize),
        };
        if parent_depth + 1 > max_depth { return vec![]; }

        let needs: Vec<String> = parent.need.iter().map(|r| r.describes.clone()).collect();
        let dctx = DecomposeContext { need: &needs, policy, root };
        let drafts = oracle.decompose(&dctx);

        let mut out = vec![];
        for d in drafts.into_iter().take(max_open) {
            let id = UnkId(*next_unk); *next_unk += 1;
            let anchor = Anchor { tick, node, parent_unk: Some(parent.id) };
            // strands inherit which DELTAs resolution would close (resolves populated).
            let mut resolves = parent.resolves.clone();
            resolves.extend(d.resolves.iter().copied());
            let sub = Unk {
                id,
                about_claim: d.about_claim.or(parent.about_claim),
                need: d.needs.iter().map(|s| RequiredElement { describes: s.clone() }).collect(),
                would_become_obs: ObsSpec { describes: d.would_become_obs.clone() },
                resolves,
                origin: anchor,
                awaiting_topic: parent.awaiting_topic,
                resolved: false,
            };
            let sid = StrandId(self.next_strand); self.next_strand += 1;
            if let Some(l) = self.links.iter_mut().find(|l| l.tick == tick) {
                l.strands.push(Strand { id: sid, unk: sub.clone(), anchor, depth: parent_depth + 1 });
            }
            out.push(sub);
        }
        out
    }

    /// Walk a strand back to its origin tick (proves the chain is connected, not flat).
    pub fn origin_of(&self, strand: StrandId) -> Option<Anchor> {
        self.links.iter().flat_map(|l| l.strands.iter()).find(|s| s.id == strand).map(|s| s.anchor)
    }
    pub fn strand_count(&self) -> usize { self.links.iter().map(|l| l.strands.len()).sum() }
}
