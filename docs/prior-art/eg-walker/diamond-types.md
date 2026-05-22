**Date:** 2026-05-22
**Status:** active
**Subject:** Diamond-types — Joseph Gentle's Rust implementation of eg-walker; the optimisation lineage from "5000x faster CRDTs" (2021) through the paper

# Diamond-types

`josephg/diamond-types` is the **Rust reference implementation** of eg-walker. It is the artefact the paper benchmarks against and the artefact awarded EuroSys 2025's Gilles Muller Best Artifact Award. Don't confuse the *algorithm* (eg-walker, paper-defined) with the *implementation* (diamond-types, this file).

## Repo facts

| Item | Value |
|---|---|
| Repo | <https://github.com/josephg/diamond-types> |
| License | ISC (current 1.0.0+; earlier `0.1.0` was `ISC OR Apache-2.0` dual-licensed) |
| Stars | ~1,800 |
| Last published `crates.io` version | `1.0.0` published **2022-08-25** |
| Master `Cargo.toml` version | `2.0.0` (**unpublished** as of this writing — never released to crates.io) |
| Total crates.io downloads | 27,041 |
| Recent crates.io downloads | 3,065 |
| Funding | "made possible by funding from the Invisible College" (README) |
| Status | "WIP" (README) — "Cargo package is quite out of date, both in terms of API and performance" |

The unpublished-master situation is unusual. Diamond-types is *not* abandoned (active commits through 2025, paper artefacts updated 2024-11), but the published crate is **three years stale** relative to the in-tree code the paper benchmarks. If Myrhiza ever wants to depend on diamond-types, the choice is:

1. Pin a git revision (loses semver, gains current code).
2. Pin `crates.io = "1.0.0"` (loses the paper's optimisations, gains semver).
3. Fork. (Likely; the algorithm is published, the encoding is not stable, the project has one steward.)

## The optimisation lineage

Diamond-types' performance story arrived in stages. Reading them in order is the simplest way to understand why eg-walker is fast:

### Stage 1: "I was wrong. CRDTs are the future" (Gentle, 2020)

Gentle's [2020 blog post](https://josephg.com/blog/crdts-are-the-future/) reverses his earlier OT-defending position. He acknowledges Kleppmann/Automerge's argument and starts looking at CRDTs as a serious replacement for OT.

### Stage 2: "5000x faster CRDTs: An adventure in optimization" (Gentle, 2021-07-31)

The foundational post: [`josephg.com/blog/crdts-go-brrr/`](https://josephg.com/blog/crdts-go-brrr/). Verbatim claims:

> "most CRDTs you read about in academic papers are crazy slow"

> "I was reading papers which described the *behaviour* of different systems. And I assumed that meant we knew how the best way to *implement* those systems. And wow, I was super wrong."

> "[diamond-types is] processing the same editing trace in 56 milliseconds. Thats 0.056 seconds, which is over 5000x faster."

Compares to Automerge processing the same trace in nearly 5 minutes (this is pre-Automerge-3.0, which closed much of the gap). The post acknowledges Kevin Jahns (Yjs) and Martin Kleppmann (Automerge) as foundational:

> "Kevin's list representation + insertion approach I describe here makes everything so much faster and simpler."

Diamond-types at this stage was a *fast CRDT* — RGA-flavoured, with the Jahns-style packed-struct representation. The paper move (eg-walker) had not happened yet.

### Stage 3: Eg-walker paper (Gentle & Kleppmann, EuroSys 2025)

The conceptual move: **stop storing CRDT metadata at rest; store the operations themselves; replay on demand**. The implementation became diamond-types' master branch, which has been continuously refactored to match the paper.

The 2021 "5000x" claim no longer holds in the simple form — Automerge 3.0 closed much of the encoding gap, and Yjs was always faster than Automerge for steady-state. The paper's specific claim is **"order of magnitude less memory in the steady state… loading orders of magnitude faster"** (verbatim from the abstract), which is the modern post-Automerge-3.0 framing.

## What's in the repo

| Subdir | Contents | Notes |
|---|---|---|
| `crates/` | The Rust workspace | Multiple crates; `diamond-types` is the headline; sub-crates for content-tree / oplog primitives |
| `js/` | JavaScript bindings | Hand-written shim layer |
| `npm-pkg-isomorphic/` | WASM bindings for Node.js and browsers | Source for `diamond-types-web` + `diamond-types-node` (npm `1.0.2`, 2023-05-15) |
| `src/` | Core source code | The historical 1.0 release layout |
| `examples/` | Usage examples | |
| `test_data/` | Test datasets | Editing traces |
| `wiki/` | In-repo wiki notes | Design discussion |

## On-disk encoding

The paper's headline performance numbers come from diamond-types' encoding. The pieces:

- **Columnar oplog.** Each operation field (agent, sequence, parent_versions, type, position, content) is stored in its own column. Within a column, run-length and delta encoding compress per-field. This is the same idea as Automerge 3.0's columnar format (which Kleppmann's other work has championed) but tuned for eg-walker's op shape.
- **B-tree content tree.** Document positions are indexed by a B-tree that maps `(position) → (character run, authoring op)`. Lookups are O(log n); insertions split nodes lazily.
- **Packed agent IDs.** Each `(agent, sequence)` op-id is referenced by an interned small-int index into a per-document agent table. Agents that author many ops get a single-byte tag; rarely-seen agents pay a few extra bytes.

Net per-op overhead in the diamond-types format: **on the order of 1-10 bytes/op** after compression. Automerge 2.x had ~240 bytes/op (per Gentle 2021); Automerge 3.0 dropped to <1 byte/char (per Automerge 3 release notes). Diamond-types is in the same league as Automerge 3.0 for the steady state, with substantially less in-memory metadata during merges.

## Sync protocol

Diamond-types ships a delta-sync model: each peer announces its causal frontier (set of "tip" operation IDs); the other peer responds with operations the announcing peer doesn't have. The protocol is **op-set-difference computation**, not a CRDT-state diff.

This matches Hypercore/Iroh/Willow's gossip model: peers exchange operations filtered by causal frontier, not state diffs. Closer to Myrhiza's per-author Merkle DAG sync than to Yjs's update-vector model.

## WASM viability

Diamond-types compiles to WASM via `wasm-bindgen`, published as:

- `diamond-types-web` (npm `1.0.2`, 2023-05-15) — browser-targeted
- `diamond-types-node` (npm `1.0.2`, 2023-05-15) — Node.js-targeted

Both are dual-licensed `ISC OR Apache-2.0` (npm metadata). They are **not Component Model artefacts** — same as Automerge/Yjs/Loro per [`../crdts/open-problems.md` §10](../crdts/open-problems.md). For Myrhiza adoption, wrapping diamond-types as a Component is the runtime's job.

The 2023 publish date is consistent with the rest of the project: the npm wrapper has not been refreshed alongside the eg-walker paper work. The 1.0.2 wrapper consumes diamond-types-the-CRDT (Stage 2 above), not diamond-types-the-eg-walker (Stage 3).

## Conformance with the reference TS implementation

`eg-walker-reference` is the pedagogical TypeScript implementation. Per its README:

> "Designed to be fully compatible with the diamond-types library with conformance testing."

The TS impl is **~200x slower** but **~30x fewer lines of code** than diamond-types. This makes the algorithm legible: a reader who wants to understand eg-walker can read `eg-walker-reference` in a sitting, then read the paper for the formal treatment, then read diamond-types for the engineering. The conformance test ensures the simple version produces the same merge results as the optimised version.

The conformance posture is unusual for a research project and is part of why the paper won the Best Artifact Award.

## Implications for Myrhiza

- **Pinning strategy.** If Myrhiza ever depends on diamond-types, the published crate is unusable for the paper's claims. Pin a git revision or fork. The stewardship signal (one steward, three-year-stale crates.io publish) is bus-factor-1 — comparable to Yjs's Kevin-Jahns risk per [`../crdts/governance.md`](../crdts/governance.md).
- **The columnar oplog technique is portable.** Independent of the eg-walker algorithm, the columnar-oplog + B-tree-content-tree encoding is a directly applicable model for Myrhiza's state-component snapshot/replay layer. If a Myrhiza state component implements eg-walker-shape text editing, the encoding is what it should mimic.
- **WIT/Component bindings don't exist.** Same as every CRDT lib. Adopting diamond-types into Myrhiza means writing the WIT + the host-call mapping ourselves.
- **The reference TS implementation is the tractable starting point.** If a Myrhiza app wants eg-walker semantics with a small surface area, port `eg-walker-reference` (200 LOC class) to Rust as a state component, accept the ~200x slowdown until the encoding work matters. This is a cheaper experiment than wrapping diamond-types.

## Sources

- Diamond-types repo: <https://github.com/josephg/diamond-types>
- Crates.io: <https://crates.io/crates/diamond-types>
- npm `diamond-types-web`: <https://www.npmjs.com/package/diamond-types-web>
- npm `diamond-types-node`: <https://www.npmjs.com/package/diamond-types-node>
- `eg-walker-reference` repo: <https://github.com/josephg/eg-walker-reference>
- `egwalker-paper` artefacts: <https://github.com/josephg/egwalker-paper>
- Gentle, *5000x faster CRDTs*: <https://josephg.com/blog/crdts-go-brrr/>
- Gentle, *I was wrong. CRDTs are the future*: <https://josephg.com/blog/crdts-are-the-future/>
- Automerge 3.0 release notes (encoding comparison): <https://automerge.org/blog/automerge-3/>
