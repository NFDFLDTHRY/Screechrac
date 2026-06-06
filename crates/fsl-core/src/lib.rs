#![allow(dead_code)]
//! fsl-core — shared primitives, the three forging seams, and the INVERTED ORACLE.
//! SPATIAL ROLE: THE SPACE — coordinates substrate (scene graph now lives in fsl-scene), the append-only Ledger (truth), the navigational Graph, the three seams, and the inverted Oracle.
//!
//! HCC-A, Stage Assumptions: "Only structure crosses between minds."
//!
//! Control inversion (per EW): the internal LLM NEVER calls a function. Functions
//! CALL the LLM. The LLM returns ONLY element arrays (OBS/DELTA/UNK/Transition/...
//! drafts). All applicable context is assembled FRESH at the start of every call
//! (the `*Context` structs below borrow current state and are rebuilt each call),
//! so the system stays as deterministic as possible. The Oracle holds no long-term
//! context: every method is a pure function of its freshly-assembled context.

use std::time::Duration;

// ───────────────────────────── Identifiers ─────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct MindId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct CrossingId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct ClaimId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct ObsId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct DeltaId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct UnkId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct RuleId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct NodeId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct TickId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct StrandId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct TemplateId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct TransitionId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct TopicId(pub u64);

// ───────────────── UNRESOLVED SEAM #1 — OBS pointer format ─────────────────
/// Where the Water Is Loud, Section 6 (1): "Pointer format for OBS (quote snippet,
/// timestamp, message ID, or 'the sentence you said: X')."
/// LEFT OPEN as a trait. It lives on the membrane's hot path as `Box<dyn Pointer>`,
/// so the format is a runtime swap, not a rewrite. No single format is baked in.
pub trait Pointer: std::fmt::Debug + Send {
    fn anchor(&self) -> &str;
    fn clone_box(&self) -> Box<dyn Pointer>;
}
impl Clone for Box<dyn Pointer> {
    fn clone(&self) -> Self { self.clone_box() }
}
/// `Span` is the format the HARNESS chooses to instantiate. The seam is the trait;
/// `Span` is merely one implementation — swap it for any other `Pointer`.
#[derive(Clone, Debug)]
pub struct Span { pub quote: String, pub context: String }
impl Pointer for Span {
    fn anchor(&self) -> &str { &self.quote }
    fn clone_box(&self) -> Box<dyn Pointer> { Box::new(self.clone()) }
}

/// A claim's raw text plus the STRUCTURED fields a real cradle-built LLM would
/// populate from that text (and which the deterministic Oracle reads instead of
/// pattern-matching — there is NO regex anywhere in this system).
/// `pointer = None` means "cannot be pointed at" → by I-A1 it cannot become OBS.
#[derive(Clone, Debug)]
pub struct Claim {
    pub id: ClaimId,
    pub text: String,
    /// "Something that can be pointed at." Absent ⇒ not pointable.
    pub pointer: Option<Box<dyn Pointer>>,
    /// Structured topic tag — lets DELTA detection be structural, not textual.
    pub topic: Option<TopicId>,
    /// -1 / 0 / +1 stance on the topic. Opposed stances on one topic ⇒ a DELTA.
    pub stance: Option<i8>,
}
impl Claim {
    pub fn plain(id: u64, text: &str) -> Self {
        Claim { id: ClaimId(id), text: text.into(), pointer: None, topic: None, stance: None }
    }
    /// Build a pointable claim — an OBS-eligible claim (I-A1: "something that can be
    /// pointed at"). `plain` builds a non-pointable claim that can only become an UNK.
    ///
    /// ```
    /// use fsl_core::{Claim, Span};
    /// let obs = Claim::pointable(1, "shipped Friday",
    ///     Span { quote: "Friday".into(), context: "thread".into() }, 7, 1);
    /// assert!(obs.is_pointable());
    /// assert!(!Claim::plain(2, "something feels off").is_pointable());
    /// ```
    pub fn pointable(id: u64, text: &str, p: impl Pointer + 'static, topic: u64, stance: i8) -> Self {
        Claim { id: ClaimId(id), text: text.into(), pointer: Some(Box::new(p)),
                topic: Some(TopicId(topic)), stance: Some(stance) }
    }
    pub fn is_pointable(&self) -> bool { self.pointer.is_some() }
}

// ──────────────── Inventory: whose OBS this is (two-party DELTA) ────────────────
/// Where the Water Is Loud, I-A2: "Sharedness is optional at OBS, mandatory at DELTA
/// resolution." Inside one mind: RIC (raw) and PFC (interpretation). Across minds: a
/// Peer. Each party owns an OBS inventory.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Inventory { Ric, Pfc, Peer(MindId) }

// ─────────── The only thing that crosses the membrane: structure ───────────
/// HCC-A §2.5: "Emotions are not transmissible. Only structure is." There is no
/// meaning/emotion field here: "Others generate their own E from their M + L."
#[derive(Clone, Debug)]
pub struct StructurePayload { pub from: MindId, pub claims: Vec<Claim> }

/// What a mind hands the membrane when Behavior is aimed at another mind. May also
/// carry an `anchor_for` that supplies a previously-missing pointer — this is how a
/// convertible INVALID later becomes OBS.
#[derive(Clone, Debug)]
pub struct CrossingCandidate {
    pub from: MindId,
    pub to: MindId,
    pub claims: Vec<Claim>,
}

/// F1 Locate() routing tag. (WTR F1 output: "route to OBS request | INVALID | UNK
/// list | DELTA build".) Defined in core so both Proof and Bridge share it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProofRouting { Obs, Delta, Unk, Invalid }

/// Events that DRIVE the Delta-Bridge FSM. Produced by F1–F5; consumed by
/// `BridgeState::step`. In core so Proof (which runs F1–F5) and Bridge (which owns
/// the FSM) need no dependency on each other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BridgeSignal {
    /// nothing pointable shared yet → Banks
    NothingShared,
    /// "urgency/heat present + anchors missing" → Rapids
    HeatWithoutAnchors,
    /// "one party presents a pointable element; the other acknowledges" → Delta
    AnchorPresented,
    /// "DELTA(OBS_A, OBS_B) exists and UNKs are listed" → Crossing
    PairedDeltaExists,
    /// remaining unknowns block crossing
    UnksListed,
    /// "crossing completed → decisions/actions can occur safely" → Bank-Rebuild
    DeltasResolved,
}

// ───────────────── UNRESOLVED SEAM #2 — INVALID stop-word list ─────────────
/// Where the Water Is Loud, Section 6 (2): "Stop-word list for INVALID."
/// Injected, never baked in. Matching is exact token-window membership (NOT regex,
/// NOT substring search): both text and stop-phrase are tokenized and compared as
/// contiguous slices.
#[derive(Clone, Debug, Default)]
pub struct InvalidPolicy { pub stop_words: Vec<String> }
impl InvalidPolicy {
    pub fn flags(&self, text: &str) -> bool {
        let toks = tokenize(text);
        self.stop_words.iter().any(|p| phrase_in_tokens(&toks, &tokenize(p)))
    }
}
/// Whitespace split + per-token punctuation strip + lowercase. No regex.
///
/// ```
/// use fsl_core::tokenize;
/// assert_eq!(tokenize("The deadline, was Friday!"), vec!["the", "deadline", "was", "friday"]);
/// ```
pub fn tokenize(s: &str) -> Vec<String> {
    s.split_whitespace()
        .map(|w| w.chars().filter(|c| c.is_alphanumeric() || *c == '\'').collect::<String>().to_lowercase())
        .filter(|w| !w.is_empty())
        .collect()
}
fn phrase_in_tokens(toks: &[String], phrase: &[String]) -> bool {
    if phrase.is_empty() || phrase.len() > toks.len() { return false; }
    toks.windows(phrase.len()).any(|w| w == phrase)
}

// ───────────────── UNRESOLVED SEAM #3 — UNK resolution rule ────────────────
/// Where the Water Is Loud, Section 6 (3): "UNK resolution rule (do we halt at any
/// UNK, or can we proceed within a bounded UNK budget?)." Explicit enum; no default
/// blessed; `#[non_exhaustive]` so a future rule must be handled before use.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum UnkPolicy {
    HaltOnAnyUnk,
    BoundedBudget { max_open_unk: usize, max_strand_depth: usize },
}

/// Deterministic, regex-free routing the default Oracle uses (a real model may
/// override). Decision is made on STRUCTURE: pointability, injected stop tokens,
/// and framing pressure. This is also where the no-stakes framing DEGRADE lives:
/// higher `invalid_pressure` pushes a non-pointable claim toward INVALID
/// ("you would stop looking at structure and start arguing about stakes") rather
/// than hard-rejecting anything.
///
/// ```
/// use fsl_core::{Claim, Span, InvalidPolicy, ProofRouting, structural_route};
/// let policy = InvalidPolicy::default();
/// // pointable claim → routed to OBS (I-A1 satisfied)
/// let obs = Claim::pointable(1, "x", Span { quote: "x".into(), context: "c".into() }, 1, 1);
/// assert_eq!(structural_route(&obs, &policy, 0.0), ProofRouting::Obs);
/// // non-pointable under low framing pressure → UNK (degrade, not gate)
/// assert_eq!(structural_route(&Claim::plain(2, "feels off"), &policy, 0.0), ProofRouting::Unk);
/// ```
pub fn structural_route(claim: &Claim, policy: &InvalidPolicy, invalid_pressure: f32) -> ProofRouting {
    if policy.flags(&claim.text) {
        return ProofRouting::Invalid;
    }
    if claim.is_pointable() {
        ProofRouting::Obs
    } else {
        // I-A1: a non-pointable claim cannot be OBS. Degrade, not gate.
        if invalid_pressure >= 0.5 { ProofRouting::Invalid } else { ProofRouting::Unk }
    }
}

// ─────────────── Doubling time horizons + per-stage ledgers ───────────────
/// Per EW: "HCC-A stages are 2x time horizons per step and each one gets its own
/// ledger of summations."
#[derive(Clone, Copy, Debug)]
pub struct Horizon(pub Duration);
/// horizon(stage_i) = base << i (doubling per step).
pub fn stage_horizon(base: Horizon, stage_index: u32) -> Horizon {
    Horizon(base.0 * 2u32.pow(stage_index))
}
#[derive(Clone, Debug, Default)]
pub struct Summations { pub events: u64, pub weight_sum: f64, pub pruned: u64 }
/// Each HCC-A stage owns its own summation ledger over its own (doubling) horizon.
#[derive(Clone, Debug)]
pub struct StageLedger { pub stage_index: u32, pub horizon: Horizon, pub sums: Summations }

/// Back-pointer from a (sub-)UNK to its origin: "this self talk becomes cable
/// strands linked like a chain to their origin (data pointers)." (EW)
#[derive(Clone, Copy, Debug)]
pub struct Anchor { pub tick: TickId, pub node: NodeId, pub parent_unk: Option<UnkId> }

// ═══════════════════════════ THE INVERTED ORACLE ═══════════════════════════
// The LLM returns ONLY element arrays (the *Draft types). It never mutates state
// and never calls a function. Functions assemble a fresh *Context, call the Oracle,
// then VALIDATE and APPLY the returned drafts.

/// Tag for 4.5 Meaning Engine Style, passed into the meaning call.
#[derive(Clone, Copy, Debug)]
pub enum MeaningStyleTag { Descriptive, Evaluative, Directive }

// —— element arrays the LLM may return ——
#[derive(Clone, Debug)] pub struct InterpretationDraft { pub claim: ClaimId, pub schema: String, pub probability: f32 }
#[derive(Clone, Debug)] pub struct RoutingDraft { pub claim: ClaimId, pub routing: ProofRouting }
#[derive(Clone, Debug)] pub struct ObsDraft { pub claim: ClaimId }
#[derive(Clone, Debug)] pub enum DeltaKindDraft { ObsVsObs { a: ClaimId, b: ClaimId }, ObsVsRule { obs: ClaimId, rule: RuleId } }
#[derive(Clone, Debug)] pub struct DeltaDraft { pub kind: DeltaKindDraft }
#[derive(Clone, Debug)] pub struct UnkDraft {
    pub about_claim: Option<ClaimId>,
    pub needs: Vec<String>,
    pub would_become_obs: String,
    pub resolves: Vec<DeltaId>,
}
#[derive(Clone, Debug)] pub struct TransitionDraft {
    pub beliefs: Vec<String>,
    pub priorities: Vec<String>,
    pub roles: Vec<String>,
    pub future_expectations: Vec<String>,
    pub allowed_behaviors: Vec<String>,
    pub weight: f32,
}

// —— freshly-assembled contexts (rebuilt at each call site) ——
/// Light, core-only views so the Oracle trait needs no Proof types (no cycle).
#[derive(Clone, Debug)] pub struct ObsView { pub claim: ClaimId, pub topic: Option<TopicId>, pub stance: Option<i8>, pub from_peer: bool }
#[derive(Clone, Debug)] pub struct RuleView { pub rule: RuleId, pub topic: TopicId, pub expected_stance: i8 }

pub struct PfcContext<'a> { pub claims: &'a [Claim], pub schemas: &'a [String], pub root: RootCtx }
pub struct RoutingContext<'a> { pub claims: &'a [Claim], pub invalid_policy: &'a InvalidPolicy, pub invalid_pressure: f32, pub root: RootCtx }
pub struct ObsContext<'a> { pub claims: &'a [Claim], pub root: RootCtx }
pub struct DeltaContext<'a> { pub incoming: &'a [Claim], pub existing: &'a [ObsView], pub expected: &'a [RuleView], pub root: RootCtx }
pub struct UnkContext<'a> { pub claims: &'a [Claim], pub root: RootCtx }
pub struct DecomposeContext<'a> { pub need: &'a [String], pub policy: &'a UnkPolicy, pub root: RootCtx }
pub struct MeaningContext<'a> { pub ledger: &'a [Claim], pub priority_order: &'a [&'a str], pub style: MeaningStyleTag, pub root: RootCtx }

/// Inverted agent harness: "the harness calls an LLM ... the LLM can never call a
/// single function." Each method takes a FRESH context and returns an element array.
/// No `&mut`, no registry, no callbacks — the inversion is enforced by these
/// signatures. The Oracle is also stateless across calls (context is reassembled
/// every time), keeping the system deterministic.
pub trait Oracle {
    /// PFC: "apply pre-existing schemas and assign probable interpretations."
    fn interpret(&self, ctx: &PfcContext) -> Vec<InterpretationDraft>;
    /// F1 Locate: route each claim to OBS request | INVALID | UNK | DELTA.
    fn route(&self, ctx: &RoutingContext) -> Vec<RoutingDraft>;
    /// OBS: "Something that can be pointed at." Returns which claims become OBS.
    fn observe(&self, ctx: &ObsContext) -> Vec<ObsDraft>;
    /// DELTA: "Where two observations don't match" (or OBS vs an expected rule).
    fn diff(&self, ctx: &DeltaContext) -> Vec<DeltaDraft>;
    /// UNK: "Something required for understanding that is missing."
    fn unknowns(&self, ctx: &UnkContext) -> Vec<UnkDraft>;
    /// UNK decomposition into sub-UNKs ("what data elements are required").
    fn decompose(&self, ctx: &DecomposeContext) -> Vec<UnkDraft>;
    /// Meaning Engine: returns Transition arrays over the exact five-set.
    fn meaning(&self, ctx: &MeaningContext) -> Vec<TransitionDraft>;
}

// ═══════════════════════ SANDBOX / LEDGER / GRAPH / ROOTS ═══════════════════════
// Architecture: the Ledger is the append-only SOURCE OF TRUTH; the Graph is ONLY a
// navigational layer (pointers + relationships); the Sandbox holds ALL elements ever
// generated (the "grains of sand") plus every root node. Every user input or system
// event creates a root node. UNK root nodes run in PARALLEL to input root nodes.

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)] pub struct RootId(pub u64);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)] pub struct Seq(pub u64);

/// Degree-of-separation: distance (in Onion shells) from the original root node.
/// Higher degree ⇒ progressively lower onion priority.
pub type DegreeOfSeparation = u32;

/// Fresh per-call root context. Every LLM call receives the current root node and its
/// degree-of-separation tracking (assembled fresh, never held by the LLM).
#[derive(Clone, Copy, Debug)]
pub struct RootCtx { pub root: RootId, pub degree: DegreeOfSeparation }
impl Default for RootCtx { fn default() -> Self { RootCtx { root: RootId(0), degree: 0 } } }

/// The append-only SOURCE OF TRUTH. Nothing is mutated or deleted: every grain of sand
/// the system ever generates is appended here, in order.
#[derive(Clone, Debug, Default)]
pub struct Ledger { entries: Vec<LedgerEntry> }
#[derive(Clone, Debug)]
pub struct LedgerEntry { pub seq: Seq, pub root: RootId, pub degree: DegreeOfSeparation, pub payload: LedgerPayload }
#[derive(Clone, Debug)]
pub enum LedgerPayload {
    Input { text: String },
    Event { text: String },
    Obs { id: ObsId, topic: Option<TopicId> },
    Delta { id: DeltaId },
    Unk { id: UnkId, awaiting_topic: Option<TopicId> },
    UnkResolved { id: UnkId },
    Invalid { claim: ClaimId },
    InvalidConverted { claim: ClaimId },
    Transition { id: TransitionId },
    Behavior { summary: String },
}
impl Ledger {
    pub fn append(&mut self, root: RootId, degree: DegreeOfSeparation, payload: LedgerPayload) -> Seq {
        let seq = Seq(self.entries.len() as u64);
        self.entries.push(LedgerEntry { seq, root, degree, payload });
        seq
    }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn entries(&self) -> &[LedgerEntry] { &self.entries }
    /// All ledger seqs belonging to one root's conversational context (preserved and
    /// re-assembled when an UNK from that root is fed back through the loop).
    pub fn context_of(&self, root: RootId) -> Vec<Seq> {
        self.entries.iter().filter(|e| e.root == root).map(|e| e.seq).collect()
    }
    /// Non-blocking "new information available" detector: has an OBS for this topic been
    /// recorded ANYWHERE in the sandbox yet?
    pub fn obs_topic_present(&self, topic: TopicId) -> bool {
        self.entries.iter().any(|e| matches!(&e.payload, LedgerPayload::Obs { topic: Some(t), .. } if *t == topic))
    }
}

/// The Graph is ONLY a navigational layer: pointers into the Ledger + relationships.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Relationship { Raises, Resolves, Pairs, Converts, Derives, FeedsBack, Spawns }
#[derive(Clone, Copy, Debug)] pub struct GraphNode { pub id: NodeId, pub seq: Seq }
#[derive(Clone, Copy, Debug)] pub struct GraphEdge { pub from: NodeId, pub to: NodeId, pub rel: Relationship }
#[derive(Clone, Debug, Default)]
pub struct Graph { pub nodes: Vec<GraphNode>, pub edges: Vec<GraphEdge>, next: u64 }
impl Graph {
    pub fn add_node(&mut self, seq: Seq) -> NodeId { let id = NodeId(self.next); self.next += 1; self.nodes.push(GraphNode { id, seq }); id }
    pub fn link(&mut self, from: NodeId, to: NodeId, rel: Relationship) { self.edges.push(GraphEdge { from, to, rel }); }
    pub fn edge_count(&self) -> usize { self.edges.len() }
}

/// Every user input or system event creates a ROOT NODE. UNK root nodes run in
/// PARALLEL to input root nodes; neither blocks the other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RootKind { Input, Event, Unk }
#[derive(Clone, Debug)]
pub struct RootNode {
    pub id: RootId,
    pub kind: RootKind,
    pub degree: DegreeOfSeparation,
    pub origin: Option<RootId>,
    pub eligible: bool,
    pub done: bool,
    pub unk: Option<UnkId>,
    pub awaiting_topic: Option<TopicId>,
    pub label: String,
    /// pointer into the Graph for this root's anchor node (navigational only).
    pub anchor_node: Option<NodeId>,
}
/// THE SANDBOX — all elements ever generated (the grains of sand): the append-only
/// Ledger (truth) + the navigational Graph + every root node.
#[derive(Clone, Debug, Default)]
pub struct Sandbox { pub ledger: Ledger, pub graph: Graph, pub roots: Vec<RootNode>, next_root: u64 }
impl Sandbox {
    pub fn new_root(&mut self, kind: RootKind, degree: DegreeOfSeparation, origin: Option<RootId>, label: &str) -> RootId {
        let id = RootId(self.next_root); self.next_root += 1;
        self.roots.push(RootNode {
            id, kind, degree, origin,
            eligible: !matches!(kind, RootKind::Unk), // UNK roots start ineligible until info appears
            done: false, unk: None, awaiting_topic: None, label: label.into(), anchor_node: None,
        });
        id
    }
    pub fn root(&self, id: RootId) -> Option<&RootNode> { self.roots.iter().find(|r| r.id == id) }
    pub fn root_mut(&mut self, id: RootId) -> Option<&mut RootNode> { self.roots.iter_mut().find(|r| r.id == id) }
    pub fn active_roots(&self) -> Vec<RootId> { self.roots.iter().filter(|r| !r.done && r.eligible).map(|r| r.id).collect() }
}