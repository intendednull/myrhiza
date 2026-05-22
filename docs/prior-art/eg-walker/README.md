**Date:** 2026-05-22
**Status:** active
**Subject:** Eg-walker (Event Graph Walker) — the non-CRDT event-graph replication algorithm by Joseph Gentle and Martin Kleppmann; diamond-types is Gentle's Rust implementation

# Eg-walker / diamond-types

Eg-walker is a **collaborative-text-editing algorithm that replays an event graph instead of maintaining per-character CRDT metadata at rest**. Diamond-types (`josephg/diamond-types`) is Joseph Gentle's Rust implementation; `eg-walker-reference` is the pedagogical TypeScript implementation. The paper is *Collaborative Text Editing with Eg-walker: Better, Faster, Smaller* (Gentle & Kleppmann, EuroSys 2025, [arXiv:2409.14252](https://arxiv.org/abs/2409.14252)).

This folder exists because Eg-walker is **Kleppmann's own pivot away from CRDT orthodoxy** — the most-cited CRDT researcher of the last decade arguing that the per-character metadata CRDTs carry is mostly a cache, not a load-bearing structural property. For Myrhiza's `state-apply` design (already event-log-replay-shaped per [`willow/runtime-vision.md`](../willow/runtime-vision.md)), the algorithmic frame is closer than any of [`crdts/`](../crdts/).

## Key facts at a glance

| Item | Value | Notes |
|---|---|---|
| Paper | *Collaborative Text Editing with Eg-walker: Better, Faster, Smaller* | arXiv:2409.14252, [DOI 10.1145/3689031.3696076](https://doi.org/10.1145/3689031.3696076) |
| Authors | Joseph Gentle, Martin Kleppmann | Gentle is first author, Kleppmann last |
| arXiv submission | 2024-09-21 (v1) | Pre-print before EuroSys |
| Venue | EuroSys 2025 (20th European Conference on Computer Systems), Rotterdam, co-located with ASPLOS 2025 | Presented March/April 2025 |
| Award | **Gilles Muller Best Artifact Award (EuroSys 2025)** | Per Kleppmann's homepage publications listing |
| Paper license | CC-BY 4.0 (text); ISC (artefact code/data) | `egwalker-paper` repo |
| Rust impl | `diamond-types` — crates.io `1.0.0` (2022-08-25); master `Cargo.toml` shows `2.0.0` but **2.0.0 is unpublished** | crates.io has 2 published versions only (`0.1.0`, `1.0.0`); 27,041 total / 3,065 recent downloads |
| JS/WASM wrappers | `diamond-types-web` and `diamond-types-node` — npm `1.0.2` (2023-05-15) latest | Wrap the Rust impl via wasm-bindgen |
| TS reference impl | `josephg/eg-walker-reference` — 174 stars, ~200x slower than diamond-types but ~30x fewer LOC, conformance-tested | The pedagogical artefact; the paper appendix gestures at it |
| Crate license | ISC (1.0.0+; earlier 0.1.0 was `ISC OR Apache-2.0`) | crates.io versions endpoint |
| Funding | Per `diamond-types` README: "made possible by funding from the Invisible College" | No Rocicorp affiliation verified |
| Status | Research-grade-but-shipping. No flagship app at scale. Diamond-types `crates.io` last published 2022-08-25; repo development active but unreleased. | See `## Honest scale disclosure` below |

## How to use

Read in this order:

1. **[algorithm.md](algorithm.md)** — what eg-walker *is*: append-only event log, snapshots, Fugue-based sequencing for tie-breaks, replay-on-demand. Paper §3-4 cited.
2. **[diamond-types.md](diamond-types.md)** — Gentle's Rust implementation: B-tree-backed content tree, columnar oplog format, packed agent IDs, the optimisation lineage (`5000x faster CRDTs` 2021 → diamond-types → eg-walker).
3. **[comparisons.md](comparisons.md)** — vs Automerge / Yjs / Loro / OT. Memory, load time, document size, sync protocol, determinism. Cross-link to [`../crdts/comparisons.md`](../crdts/comparisons.md).
4. **[critiques.md](critiques.md)** — third-party voices. The Kleppmann pivot quoted verbatim. HN thread highlights. What the paper itself acknowledges as not solved.
5. **[open-problems.md](open-problems.md)** — what eg-walker structurally doesn't solve: garbage collection at scale, schema evolution, authority/Byzantine, offline-merge cost when full history isn't available, partial replication.
6. **[history.md](history.md)** — Gentle's lineage (ShareJS 2011 → CRDTs go brrr 2021 → diamond-types → eg-walker paper 2024 → EuroSys 2025); Kleppmann's pivot from Automerge.
7. **[lessons.md](lessons.md)** — *the decision file.* validates / avoid / borrow synthesis for Myrhiza `state-apply`.
8. **[glossary.md](glossary.md)** — system-specific terms.

If you only have time for two files: read **lessons.md** + **algorithm.md**.

## What makes eg-walker different from CRDTs

The paper's framing (Gentle & Kleppmann, abstract):

> "Compared to existing CRDTs, [Eg-walker] consumes an order of magnitude less memory in the steady state, and loading a document from disk is orders of magnitude faster."

The core move:

- **CRDTs maintain per-character metadata at rest** (Yjs structs, Automerge ops as a columnar log, Loro Fugue items). The metadata is the algorithm — operations are merged via that metadata.
- **Eg-walker stores the operations themselves in a causal graph** (an *event graph*) — `(agent, seq, parents, type, position, content)`. To query the document state, the algorithm *walks the graph in causal order*, transforming positions OT-style, producing a content snapshot. The snapshot can be cached; the metadata only re-materialises when needed (e.g. when merging a concurrent branch).
- For tie-breaking concurrent inserts at the same position, eg-walker uses **Fugue ordering** (Weidner & Kleppmann 2023, [arXiv:2305.00583](https://arxiv.org/abs/2305.00583)) — agent-ID tie-break that minimises interleaving.

Net: eg-walker is **technically still a CRDT** (it converges deterministically from the same op set), but it splits the algorithm from the storage layout. The stored form is closer to an event-sourcing log; CRDT-merge semantics are computed on demand.

## Why this folder exists for Myrhiza

[`/home/user/myrhiza/docs/README.md:101`](../../README.md) self-flagged Eg-walker as a future prior-art candidate. [`references/local-first.md` §4.2](../../references/local-first.md) frames it as *"Kleppmann's argument that replicated data structures need not be CRDTs"* and the single most important convergence-paradigm anchor since the 2011 Shapiro survey. [`crdts/critiques.md` §1](../crdts/critiques.md) quotes Kleppmann's own pivot.

For Myrhiza specifically:

- `state-apply` is already a **deterministic pure fn of (prior state, event)** (CLAUDE.md). That is closer to eg-walker's *replay the graph* posture than to Automerge/Yjs's *carry merge metadata in the value type* posture.
- Per-author Merkle DAG event-sourcing (lifted from Willow PR #636) is **structurally an event graph**. Eg-walker's algorithm is one possible state-apply implementation for shared text in a Myrhiza app.
- The "two-entry-point" component shape (`apply` + `propose`, see [`willow/runtime-vision.md`](../willow/runtime-vision.md) §"Apps as bundles") matches eg-walker's split between *appending to the event graph* and *materialising state from the graph*.

## Honest scale disclosure

Be explicit about where eg-walker / diamond-types actually sit:

- **No flagship app at scale.** No Linear, no Notion, no Proton Docs, no JupyterLab. Diamond-types' `crates.io` page has 27,041 total downloads / 3,065 recent (vs Automerge's six-digit and Yjs's seven-digit numbers).
- **Diamond-types `crates.io` last published 2022-08-25.** The repo is actively developed (master is at `2.0.0` in Cargo.toml, with optimisation work continuing through 2025) but the **published artefact is three years stale**. Anyone building on `cargo add diamond-types` today is consuming 2022 code; the paper benchmarks ran a much later in-tree build.
- **No production-grade WASM Component Model artefact.** Diamond-types ships as raw wasm-bindgen modules (`diamond-types-web`, `diamond-types-node`); not a CM artefact with a WIT interface. Same gap as Automerge/Yjs/Loro per [`../crdts/open-problems.md` §10](../crdts/open-problems.md).
- **No `crate-level` API stability commitment.** The README explicitly says the published cargo package is "quite out of date, both in terms of API and performance."
- **Pedagogical TS impl runs ~200x slower than diamond-types.** Per `eg-walker-reference` README: "approximately 200x slower than diamond-types and 30x fewer lines of code." Don't confuse the paper's benchmarks (run against the optimised Rust) with what a naive port would deliver.

Treat eg-walker as **research-grade-but-shipping**. Closer to that than [Loro's](../crdts/loro.md) "no at-scale users" position because Gentle's blog post lineage and the paper's reproducibility artefact (Best Artifact Award) signal engineering rigor, but the production-readiness gap is real.

**Framing disclosure.** These docs are written from the **Myrhiza-as-deterministic-event-log-replay-runtime** stance — the "Implications for Myrhiza" sub-sections frame eg-walker's choices through that lens. A reader auditing whether deterministic-event-log-replay is itself the right primitive for `state-apply` should weigh the corpus accordingly: it is a *learn-from-eg-walker-into-Myrhiza-state-apply* artefact, not a neutral catalog. Where this folder and [`../crdts/`](../crdts/) disagree (eg-walker is the better paradigm match; CRDT libs have more production hardening), the crdts/ folder is more representative of *what's currently shipping*; this folder is more representative of *what the algorithmic frame should be*. The corpus also tilts toward the [Willow/PR #636 → Myrhiza `state-apply`](../willow/runtime-vision.md) reading: where eg-walker's `(agent, seq, parents)` event identity rhymes with Willow's `Event { author, prev, deps }`, the lessons file picks up the rhyme; where it diverges (eg-walker has no signature, no authority predicate), the open-problems file calls the gap.

## Sources

- Paper: <https://arxiv.org/abs/2409.14252> — *Collaborative Text Editing with Eg-walker: Better, Faster, Smaller*, Gentle & Kleppmann, EuroSys 2025
- Paper DOI: <https://doi.org/10.1145/3689031.3696076>
- Diamond-types: <https://github.com/josephg/diamond-types>
- Eg-walker reference (TS): <https://github.com/josephg/eg-walker-reference>
- Eg-walker paper artefacts: <https://github.com/josephg/egwalker-paper>
- Crates.io: <https://crates.io/crates/diamond-types>
- npm `diamond-types-web`: <https://www.npmjs.com/package/diamond-types-web>
- npm `diamond-types-node`: <https://www.npmjs.com/package/diamond-types-node>
- Joseph Gentle blog: <https://josephg.com/blog/>
- Martin Kleppmann homepage: <https://martin.kleppmann.com/>
- Fugue paper (Weidner & Kleppmann 2023): <https://arxiv.org/abs/2305.00583>
