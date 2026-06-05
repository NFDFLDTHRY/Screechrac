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

## Facilitator — the first *production-grade* application of FSL (single-node)

**[Facilitator](./references/facilitator/FACILITATOR_Project_Manifest_v1.1.md)** is the
first real-world application built on top of the FSL cognitive engine: a **driver-first
local problem-solving marketplace** — *"You have a problem. We fix it."* It ships as the
single installable PWA under [`web/`](./web) (Leptos SSR + Axum) and serves
**customers** (post free-form/voice jobs), **drivers** (accept and complete jobs), and a
platform **Engine** view.

It is **durable and authenticated** (single-node, production-grade): an embedded **SQLite**
database is the application's source of truth (accounts, jobs, clarifications, preset
stats); the FSL `World` is an in-memory **projection rebuilt by replaying stored jobs on
boot**, so data survives restarts. Accounts use **argon2** hashing + a **signed, HTTP-only
session cookie**, and routes are **role-gated** (customer / driver / admin) — real
identity, ready for payments (GoDaddy) to hook into a clean `PaymentProvider` seam.
*Hive Calls, Guardian Mode, and the full QR pipeline are deferred to later PRs (not faked).*

It is **powered end-to-end by FSL** (the web layer stays thin — all intelligence lives in FSL):

- The custom **fsl-llm engine** ([`crates/fsl-llm/src/facilitator.rs`](./crates/fsl-llm/src/facilitator.rs))
  turns a free-form request into a structured job through the **inverted Oracle** — it
  classifies intent into presets and, for every missing variable, calls the Oracle to
  mint an **UNK** (a Listener question). It holds no state.
- Each posted job is a **cable** (root node) ingested into a persistent FSL `World`;
  missing slots become **strands** (UNKs); **The Listener** answers resolve them across
  ticks via the **Onion Shell**; the **Coffee Cup** arc advances each pass; and once
  every slot is anchored the free-form request **standardizes into a one-tap preset**.
- **Customer Mode** escalation routes through the FSL cognitive system; the **driver
  dashboard** shows live shadow-intelligence hints pulled from FSL state.

- **Data-driven preset evolution:** once a kind reaches a threshold of fully-specified
  jobs, it is promoted to a **one-tap preset** that posts a job already `ready`.
- **Deeper Shadow Intelligence** (`/api/shadow`): open vs resolved UNK ratios, Coffee-Cup
  stage, stalled-clarification detection, and live scene-graph growth.

**No FSL core crate is changed**; persistence and auth are application infrastructure in
the detached [`web/`](./web) crate (its own `[workspace]`). The scene graph, the Coherence
Contract, and the three conceptual references remain immutable. Single-node is the current
boundary (horizontal scaling is future). Deploy the whole thing with [`setup.sh`](./setup.sh).

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

## Testing — local only (Rust's native test system)

This project uses **only Rust's native test runner** — there is **no GitHub Actions / CI**.
Run the full test suite locally before pushing:

```sh
cargo test --workspace                       # all FSL core crates (bridge / cup / scene / pipeline / source)
cargo test --manifest-path web/Cargo.toml    # the Facilitator web crate (detached workspace)
cargo run --quiet                            # walk the 3D scene graph + hologram-compliance scan
cargo check --workspace --all-targets        # fast type/lint pass
```

The contract guardrails are enforced by these tests plus the runtime scan in
[`src/hologram_compliance.rs`](./src/hologram_compliance.rs) and the source-level checks in
`crates/fsl-mind/tests/hologram_source.rs` — all run via `cargo test`, on your machine.

## Invariants the build enforces

- **FLOW is the sole connective primitive** — nothing else may connect cables or strands.
- **The explosion lens never mutates truth** — the ledger is identical before/after `explode`.
- **The five CupEdges stay distinct** — each `propagate()` runs on traversal; never one generic arrow.
- **Directives are legitimate only in S3 Crossing / S4 Bank-Rebuild.**
- **Radial distance equals degree-of-separation** — `radius == RADIAL_BASE * (1 + degree)`.
- **The Oracle holds no context** — every call receives a freshly assembled context. No regex anywhere.

These are checked at runtime by [`src/hologram_compliance.rs`](./src/hologram_compliance.rs)
and at the source level by `crates/fsl-mind/tests/hologram_source.rs`.
