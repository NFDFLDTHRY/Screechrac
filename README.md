# FSL — Foundational System Loop

## Canonical Reference Documents — the single, immutable source of truth

The entire project is authored against three canonical conceptual documents, held
**read-only and immutable** under [`references/conceptual/`](./references/conceptual/):

1. **[Human Cognitive Compiler Architecture.txt](./references/conceptual/Human%20Cognitive%20Compiler%20Architecture.txt)** — HCC-A.
2. **[Where the Water Is Loud.txt](./references/conceptual/Where%20the%20Water%20Is%20Loud.txt)** — the Proof Ledger / Delta-Bridge crossing manual.
3. **[The Coffee Cup (1).txt](./references/conceptual/The%20Coffee%20Cup%20(1).txt)** — the five-stage deformation arc.

These three files are the **single, immutable source of truth** for all fidelity audits
and merciless reviews. They are sacred and read-only — never altered, renamed, or
summarized. Every change to this codebase must be audited against their verbatim
content. Their byte-for-byte integrity is enforced by the `reference_docs_immutable`
guardrail in [`src/hologram_compliance.rs`](./src/hologram_compliance.rs).

## Facilitator — the first real-world application of FSL

**[Facilitator](./references/facilitator/FACILITATOR_Project_Manifest_v1.1.md)** is the
first real-world application built on top of the FSL cognitive engine: a **driver-first
local problem-solving marketplace** — *"You have a problem. We fix it."* It ships as the
single installable PWA under [`web/`](./web) (Leptos SSR + Axum) and serves two roles —
**customers** who post free-form/voice jobs and **drivers** who accept them — plus the
platform **Engine** view that walks the FSL scene graph.

Facilitator is a **thin presentation + job-routing layer only**: every posted job is a
root node (a *cable*) routed through the existing FSL harness, and **no FSL core crate is
changed**. See the manifest and `web/src/main.rs` for the mapping onto the
cable/strand/bulge methodology. Deploy the whole thing with [`setup.sh`](./setup.sh).

---

A Rust workspace where the **cable / strand / bulge** spatial methodology is the
*literal structural organizing principle* of the codebase — not a visualization, not
comments, not a post-processing layer. Crates, modules, types, and traversal embody it,
so the system is both **human-walkable** (as a 3D scene graph) and **machine-buildable**.

FSL unifies three conceptual documents into one operational loop:

- **HCC-A** — the Human Cognitive Compiler Architecture (Interface → RIC ∥ PFC →
  Compiler → Story Ledger → Meaning → Emotion → ISL → Behavior → Feedback).
- **Where the Water Is Loud** — the Proof Ledger / Delta-Bridge crossing manual
  (OBS / DELTA / UNK / INVALID, the five-state bridge, the River model).
- **The Coffee Cup** — the five-stage deformation arc (Conditions → Release →
  Trajectory → Impact → Aftermath) and its five distinct recirculating edges.

See **[`FSL_COHERENCE_CONTRACT.md`](./FSL_COHERENCE_CONTRACT.md)** for the
non-negotiable 1:1 mapping that every crate, type, and file obeys.

## The backbone (walked outward from a cable)

| Crate       | Spatial role  | Contents |
|-------------|---------------|----------|
| `fsl-core`  | THE SPACE     | ids, the three forging seams, the inverted `Oracle` trait, `Ledger` (truth), `Graph` (nav), `Sandbox`/`RootNode` |
| `fsl-flow`  | FLOWS         | the Proof membrane (only structure crosses) + the Delta-Bridge crossing FSM + the River |
| `fsl-bulge` | THE BULGE     | the long-lived `CupArc` + five distinctly-named `CupEdge`s + three domains per stage |
| `fsl-actor` | ACTOR STRANDS | the HCC-A structures placed on each exploded plane (Cubes / Spheres) |
| `fsl-cable` | RADIAL STRANDS| the UNK decomposition chain/cable across ticks |
| `fsl-scene` | THE SCENE     | projection (sandbox → 3D), the explosion lens, deterministic traversal |
| `fsl-llm`   | OFF-AXIS ENGINE | the inverted Oracle (rule-based now; swappable for a real model) |
| `fsl-mind`  | THE ASSEMBLER | the `World`: parallel cables/strands, Onion Shell, the walker |
| `fsl` (bin) | THE WALK      | the end-to-end scene tour + the hologram compliance scan |

## Run it

```sh
cargo run            # walk the whole 3D scene graph end to end + compliance scan
cargo test           # the contract guardrails (bridge / cup / scene / pipeline / source)
cargo check --workspace --all-targets
```

## Invariants the build enforces

- **FLOW is the sole connective primitive** — nothing else may connect cables or strands.
- **The explosion lens never mutates truth** — the ledger is identical before/after `explode`.
- **The five CupEdges stay distinct** — each `propagate()` runs on traversal; never one generic arrow.
- **Directives are legitimate only in S3 Crossing / S4 Bank-Rebuild.**
- **Radial distance equals degree-of-separation** — `radius == RADIAL_BASE * (1 + degree)`.
- **The Oracle holds no context** — every call receives a freshly assembled context. No regex anywhere.

These are checked at runtime by [`src/hologram_compliance.rs`](./src/hologram_compliance.rs)
and at the source level by `crates/fsl-mind/tests/hologram_source.rs`.
