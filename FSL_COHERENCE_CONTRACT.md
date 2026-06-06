# FSL COHERENCE CONTRACT

## Source of Truth

The single, **immutable** source of truth for this entire project is the verbatim
content of three canonical conceptual reference documents, held read-only under
[`references/conceptual/`](./references/conceptual/):

1. **[Human Cognitive Compiler Architecture.txt](./references/conceptual/Human%20Cognitive%20Compiler%20Architecture.txt)** — HCC-A.
2. **[Where the Water Is Loud.txt](./references/conceptual/Where%20the%20Water%20Is%20Loud.txt)** — the Proof Ledger / Delta-Bridge crossing manual.
3. **[The Coffee Cup (1).txt](./references/conceptual/The%20Coffee%20Cup%20(1).txt)** — the five-stage deformation arc.

These three documents are **sacred and read-only**. They are never to be altered,
renamed, reformatted, or summarized. **Every change to the codebase must be audited
against their verbatim content** — they are the sole authority for all fidelity audits
and merciless reviews. Their byte-for-byte integrity is enforced by the
`reference_docs_immutable` check in `src/hologram_compliance.rs` (exact length + content
hash); any modification, even a single byte, fails the compliance scan.

## First *functional* application: Facilitator

**Facilitator** (the driver-first local problem-solving marketplace —
[`references/facilitator/FACILITATOR_Project_Manifest_v1.1.md`](./references/facilitator/FACILITATOR_Project_Manifest_v1.1.md))
is the first application built on top of FSL, and it is now **functionally real, powered
end-to-end by the FSL cognitive engine**. It is bound by this contract, and the mapping
is exact and *live*:

- A posted **job** is a **root node** — a *cable* ingested into the persistent FSL `World`.
- Each required variable is a **strand**: anchored → an OBS; missing → an UNK that **The
  Listener** asks about, resolved across ticks by the **Onion Shell**.
- The **Coffee Cup** arc advances on every job pass; once all UNKs resolve, the
  free-form request crosses its **Delta Bridge** and standardizes into a **preset**.
- The Listener / Customer-Mode escalation is a **flow** — the sole connective primitive —
  and runs through the FSL cognitive system.
- The FSL **Ledger stays the source of truth**; Facilitator only navigates and presents it.

The custom rule-based engine lives in `fsl-llm` (the commissioned Oracle work); the
application/presentation lives in the detached [`web/`](./web) crate (its own
`[workspace]`), which consumes the FSL crates **as a library**. **No FSL core crate, the
scene graph, this contract, or the three conceptual references may be changed** — those
remain immutable, and `reference_docs_immutable` still passes.

**Durability mirrors the contract at the app layer.** Facilitator is durable and
authenticated on a single node: an embedded **SQLite** database is the application's
*truth* (accounts, jobs, clarifications, preset stats), and the in-memory FSL `World` is a
**projection rebuilt by replaying stored jobs on boot** — the same "truth vs. navigation"
discipline this contract demands of the Ledger and the Graph. Persistence and
**lightweight auth** (argon2 + signed HTTP-only session cookie + role identity) are
*infrastructure, not cognition*: no intelligence leaves FSL. A clean `PaymentProvider`
seam awaits the later GoDaddy hookup. Single-node is the present boundary; horizontal
scaling and the remaining vision (Hive Calls, Guardian Mode, full QR) are explicitly
future work, never faked.

---

The cable/strand/bulge spatial methodology is the **literal structural organizing
principle** of this codebase — not a visualization, not comments, not post-processing.
Crates, modules, types, and navigation embody it so the codebase is both
human-walkable and machine-navigable exactly like the 3D scene graph.

## 1:1 mapping (non-negotiable)
| Concept   | Definition                                                                                          | Lives in |
|-----------|-----------------------------------------------------------------------------------------------------|----------|
| **Cable** | Every root node (user input, system event, tick) — the primary carrier axis.                        | `fsl-core` (RootNode/Sandbox), placed in space by `fsl-scene` |
| **Strand**| Every derived element (OBS, DELTA, UNK, INVALID, onion iteration, behavior…) attached radially; radial distance = degree-of-separation. | `fsl-cable` (radial UNK strands), `fsl-scene` (spatial strands) |
| **Bulge** | The long-lived `CupArc` (Coffee Cup deformation along a cable); explode/collapse as a pure lens.     | `fsl-bulge` |
| **Explosion** | The lens revealing exactly five stage planes simultaneously + the full HCC-A actor row on each. | `fsl-scene` |
| **Flow**  | The **SOLE** connective primitive; nothing else may connect cables or strands.                      | `fsl-flow` (membrane + bridge + river) + `fsl-scene::Flow` |
| **Sandbox** | The entire navigable 3D scene graph. **Ledger = truth (append-only). Graph = navigation only.**   | `fsl-core` (truth/nav), `fsl-scene` (3D) |
| **Actor** | The HCC-A structures placed on each exploded plane (Cubes = structural, Spheres = runtime).          | `fsl-actor` |
| **Mind**  | Only the assembler/walker that drives the loop and walks the scene.                                 | `fsl-mind` |
| **LLM**   | The inverted Oracle: functions CALL it; it returns only element arrays; fresh context every call.   | `fsl-llm` |

## Testing & documentation — local only (Rust's native tooling)
This project is **self-contained**: there is **no GitHub Actions / CI infrastructure**.
All testing, quality checks, and documentation use **only Rust's native tools**, run
locally — and only these four:
- `cargo test --workspace` — the FSL core crates' tests **and doctests**.
- `cargo test --manifest-path web/Cargo.toml` — the Facilitator web crate's tests and doctests.
- `cargo check --workspace --all-targets` — fast type/lint pass.
- `cargo doc --workspace --no-deps --open` — rustdoc for every public API.

Public entrypoints carry rustdoc with **runnable doctests** (executed by `cargo test`),
so documentation is verified rather than aspirational. The compliance checklist below is
additionally enforced through these `cargo test` runs and the runtime scan in
`hologram_compliance.rs`, on the engineer's machine. Doc edits never change FSL logic, and
the three conceptual references stay byte-for-byte immutable.

## Compliance checklist (enforced by `hologram_compliance.rs`)
- [ ] FLOW is the sole connective primitive — every Graph relationship becomes a `Flow`; no other connection type exists.
- [ ] The explosion LENS never mutates truth — ledger length is identical before/after `explode`.
- [ ] The five CupEdges are distinct — `make likely` / `commits` / `shapes` / `feeds` / `rewrites` are pairwise different and each `propagate()` is called on traversal.
- [ ] `directives_legitimate()` is true ONLY in S3 Crossing / S4 Bank-Rebuild.
- [ ] Radial anchoring holds — every strand's `radius == RADIAL_BASE * (1 + degree)`.
- [ ] The Oracle holds no context — every call receives a freshly assembled context (current root + degree-of-separation).
- [ ] The three forging seams stay open — Pointer (trait), InvalidPolicy (injected), UnkPolicy (`#[non_exhaustive]`). No regex anywhere.

## Crate layout (the backbone, walked outward from a cable)
```
fsl-core   THE SPACE        ids, seams, Oracle trait, Ledger(truth), Graph(nav), Sandbox/RootNode
fsl-flow   FLOWS            membrane (only structure crosses) + bridge (crossing FSM) + river (the water)
fsl-bulge  THE BULGE        CupArc + five distinct edges + three domains per stage
fsl-actor  ACTOR STRANDS    HCC-A structures placed on each exploded plane
fsl-cable  RADIAL STRANDS   UNK decomposition chain/cable across ticks
fsl-scene  THE SCENE        projection (sandbox→3D), explosion lens, deterministic traversal
fsl-llm    OFF-AXIS ENGINE  the inverted Oracle (rule-based; swappable for a real model)
fsl-mind   THE ASSEMBLER    World: parallel cables/strands, Onion Shell, the walker
fsl (bin)  THE WALK         the 3D scene tour + hologram compliance
```
