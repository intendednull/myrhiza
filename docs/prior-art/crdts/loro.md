**Date:** 2026-05-09
**Status:** active
**Subject:** Loro — Rust-native CRDT framework using Fugue text + Moveable Tree; loro-dev org; started 2022

## What it is

Loro is a CRDT library for "JSON data" — a `LoroDoc` is a tree of typed sub-containers (text, list, map, tree) each backed by an algorithm chosen for its data shape, with a single op-log binding them. It is the youngest of the Rust CRDT lineage (first commit 2022-07-12) and the only one designed Rust-native from day one with WASM as a first-class target. The project pitches itself on three differentiators: Fugue text (no interleaving), full version-control semantics (time travel, shallow snapshots), and Rust-native performance for the browser.

| Field | Value |
|---|---|
| License | MIT |
| Repo | [github.com/loro-dev/loro](https://github.com/loro-dev/loro) (5,594 stars, 142 forks, created 2022-07-12) |
| Stewardship | `loro-dev` GitHub org — no public corporate sponsor; GitHub Sponsors only |
| Founder / lead | Zixuan Chen ([@zxch3n](https://github.com/zxch3n), zx@loro.dev) — 1,584 of ~1,950 contributions (~81% solo authorship) |
| Co-maintainer | [@Leeeon233](https://github.com/Leeeon233) — 350 contributions |
| Latest crate | `loro` 1.12.0 (crates.io), `loro-crdt` 1.12.1 (npm, 2026-04-29) |
| Core | Rust (89.9% of repo), TypeScript bindings (4.8%) |
| FFI | Swift + JS/WASM via separate `loro-ffi` repo |
| Homepage | [loro.dev](https://loro.dev) |

## Architecture

Single Rust workspace. The core crate is `loro` (re-export surface over `loro-internal`). WASM bindings ship as `loro-crdt` on npm. Bundle size has grown across Loro's lifetime: the 0.10.x line shipped ~1.05 MB raw / ~399 KB gzipped (the figure Kevin Jahns cited in the 2024 critique quoted in [critiques.md](critiques.md)); the current 1.12.1 release ships **~3.16 MB raw / ~1 MB gzipped** per WASM file (verified via tarball inspection of `loro-crdt@1.12.1`, `package/web/loro_wasm_bg.wasm` = 3,165,306 bytes). For comparison, Yjs's pure-JS bundle is ~69 KB / 20 KB gzipped. Bundle size is a known criticism. Swift bindings live in a separate `loro-ffi` repo via UniFFI. There is no published `no_std` build and no WASM Component Model story (only `wasm-bindgen`-style JS interop); both are gaps for embedding in a Component Model runtime like Myrhiza.

## CRDT algorithms

Loro mixes a different algorithm per container type. This per-container choice is the most architecturally distinctive thing about Loro versus Automerge (uniform RGA-like) and Yjs (uniform YATA).

- **Fugue (LoroText, LoroList).** Weidner & Kleppmann, [_The Art of the Fugue_](https://arxiv.org/abs/2305.00583), 2023. Fugue solves the *interleaving anomaly*: when two replicas concurrently insert text at the same position, naive RGA/YATA can interleave the two passages character-by-character, producing unreadable garbage. Fugue introduces *maximal non-interleaving* as a correctness property and the paper proves FugueMax satisfies it. Loro picked Fugue over RGA (Automerge ≤2.x) and YATA (Yjs) because Fugue is the only published list CRDT with a formal interleaving guarantee, and its tree-of-positions structure encodes well columnar.
- **Rich text CRDT (LoroText with marks).** Loro's rich-text formatting layer is implemented in the sibling [loro-dev/crdt-richtext](https://github.com/loro-dev/crdt-richtext) crate, which combines Peritext (Litt et al. — span-based mark CRDT) for formatting with Fugue for text. So bold/italic spans converge under Peritext's interval-CRDT semantics while character insertions converge under Fugue.
- **Moveable Tree.** Implements [Kleppmann, Mulligan, Gomes, Beresford 2021](https://martin.kleppmann.com/papers/move-op.pdf), "A Highly-Available Move Operation for Replicated Trees" (IEEE TPDS 33(7), Isabelle/HOL-verified). The algorithm survives concurrent move operations without producing cycles by maintaining a per-replica log and replaying with deterministic conflict resolution. Sibling order within a tree node uses fractional indexing.
- **Moveable List.** Combines Fugue with [Kleppmann's 2020 PaPoC paper](https://martin.kleppmann.com/2020/04/27/papoc-list-move.html) "Moving Elements in List CRDTs". Carries roughly 50% more memory and ~80% slower encode/decode than plain `LoroList` per Loro's own docs — the move op is not free.
- **LWW Map.** Last-write-wins on Lamport timestamps. Standard, no novelty.

Internally the op-log uses an *event graph* (DAG of changes) walked via the [Eg-walker](https://github.com/josephg/diamond-types) algorithm adapted from Joseph Gentle's diamond-types — it lets state computation visit only the operations relevant to a given query rather than the full op log.

## Document model

`LoroDoc` is the root. It owns:

- A peer ID (random `u64` per replica).
- An op-log (DAG of `Change`s, each containing a run of `Op`s with Lamport timestamps).
- A *frontiers* set — the set of latest op IDs from each peer that the doc has observed; this is Loro's version vector analogue and is the argument to `checkout`.
- Containers reachable by ID or path: `LoroText`, `LoroList`, `LoroMovableList`, `LoroMap`, `LoroTree`, plus `LoroCounter`.

Operations on containers produce events that flow to subscribers (`doc.subscribe`).

## Time travel

`doc.checkout(frontiers)` rewinds (or fast-forwards) the materialized state to any point in the op-log DAG. The doc enters a *detached* state where edits are blocked; `doc.attach()` returns to the latest version. Cost is O(ops between current and target frontier) using the Eg-walker — bounded by op-log length, not state size. This makes `state-apply`-style replay viable: a kernel can redrive a doc to verify deterministic convergence.

## Shallow snapshots

A *shallow snapshot* is Loro's `git clone --depth=1` analogue: the export contains the materialized state plus only history newer than a chosen *Critical Version*, discarding older ops. A Critical Version is a frontier through which every causal path from the current state must pass — i.e. a graph cut where dropping earlier ops cannot affect future merges. Loro publishes a 6.2x-size example: full snapshot 5,421 bytes, shallow snapshot 869 bytes; v1.6 cut shallow snapshot import to 82.82 µs from a v1.0 baseline of 466.425 µs. The required precondition — Critical Version must be observed by every peer — is the catch: shallow snapshots only work after the network has converged on the cut point.

## Performance

Numbers below are from [dmonad/crdt-benchmarks](https://github.com/dmonad/crdt-benchmarks) (community-maintained, the canonical CRDT shootout). Loro 0.10.1 vs Yjs 13.6.11 vs Automerge 2.1.10:

| Benchmark | Yjs | Loro | Automerge |
|---|---|---|---|
| B1 append 6,000 chars (time / doc size) | 188 ms / 6,031 B | **120 ms** / 6,162 B | 365 ms / 3,992 B |
| B2 concurrent inserts | **65 ms** / 33,444 B | 83 ms / 35,554 B | 287 ms / 27,476 B |
| B3 many-client conflicts | **86 ms** / 7.8 MB | 116 ms / 7.9 MB | 2,335 ms / 8.06 MB |
| B4 LaTeX trace (259,778 ops) | 5,714 ms / 159,929 B | **3,089 ms** / 258,228 B | 14,326 ms / 129,116 B |
| Bundle (raw / gzip) | **69 KB / 20 KB** | ~360 KB / ~120 KB (Automerge 3.x WASM) | ~3.16 MB / ~1 MB (loro-crdt@1.12.1, per-WASM-file) |

Read: Loro is the fastest on real-world traces (B4) and append-heavy workloads, neck-and-neck with Yjs on synthetic conflict tests, and ~3-5x faster than Automerge across the board. The cost is binary size — Loro's WASM bundle is ~15x larger than Yjs's. dmonad has publicly questioned earlier Loro benchmarks for non-reproducibility (see [discuss.yjs.dev/t/2567](https://discuss.yjs.dev/t/yjs-vs-loro-new-crdt-lib/2567)); the current numbers are from his own harness. *(unverified: Loro's published per-op encoding ratio claims at loro.dev/docs/performance — site returns 403 to scripted fetches.)*

## Sync protocol

Three export modes, all returning `Uint8Array` blobs:

- `mode: "snapshot"` — full state plus full op-log.
- `mode: "update", from: Frontiers` — delta of ops since the supplied frontier; the standard incremental sync wire format.
- `mode: "shallow-snapshot", frontiers: CriticalVersion` — state plus history newer than the cut.

A two-peer sync is symmetric: each peer calls `doc.export({mode: "update", from: peer.oplogVersion()})` with the other's frontier and `doc.import(bytes)`. There is no built-in transport — the wire format is just bytes; binding into iroh, libp2p, WebRTC, or a Myrhiza kernel channel is the integrator's job.

## Determinism

Loro 1.0 froze the on-disk format and commits to no breaking changes within 1.x. Within a single Rust build, `state-apply` over the same op set produces the same materialized state and the same exported snapshot — this is required for the merge model to work. **However, byte-identical parity between the Rust core and the WASM-via-`wasm-bindgen` build is not explicitly guaranteed in published docs (unverified)**, and the encoding format has changed multiple times pre-1.0 (the format-stability commitment only attaches from 1.0 onward, late 2024). For Myrhiza this matters: if `state-apply` runs as a WASM Component on every peer, the runtime needs the same Loro-compiled-to-WASM artifact on every peer to guarantee bit-identical convergence. Cross-language byte parity (Swift FFI vs JS WASM vs native Rust) is a question to verify empirically before betting on it.

## Stewardship reality

Solo founder + one regular co-maintainer. Zixuan Chen (zxch3n) wrote ~81% of commits; Leeeon233 ~18%. Twenty-five other contributors have ≤9 commits each, mostly typo fixes. No GitHub Sponsors page total disclosed; no Crunchbase entry; no Y Combinator association found. Funding model appears to be founder time + GitHub Sponsors + (presumably) consulting. Activity is steady — bi-weekly to monthly minor releases throughout 2025-2026, latest `loro-crdt@1.12.1` on 2026-04-29, last commit 2026-05-07. This is a healthy *bus factor: 1* project: the code quality is high but if zxch3n moves on the project stalls.

## Adoption

No high-profile production users surfaced in research. The Hacker News [Show HN thread](https://news.ycombinator.com/item?id=38248900) and the rich-text discussion ([HN 39102577](https://news.ycombinator.com/item?id=39102577)) are largely positive but commenter-driven, not customer-driven. Some integration in the muni-town/weird project ([issue #264](https://github.com/muni-town/weird/issues/264)). Compared to Yjs (powering BlockNote, Linear, Evernote, Tiptap collab) and Automerge (Ink & Switch, several local-first prototypes), Loro has not yet had its first big public production deployment that I can verify. It's a 2022 project that hit 1.0 in late 2024; the adoption window has been short.

## Implications for Myrhiza

- **Best Rust+WASM technical fit of the three.** Rust-native, 1.0-stable wire format, fastest on real traces, container-per-algorithm matches the variety of state shapes a Myrhiza app needs. If you wanted to drop a CRDT into a `state-apply` component today, Loro is the path of least friction.
- **Determinism story is only partial.** Byte-parity between Rust-native and WASM-compiled Loro is not advertised. Until verified empirically, a Myrhiza `state-apply` component using Loro must compile Loro to WASM and ship the same WASM artifact to every peer — no native fast path, or convergence breaks. This rules out a "Loro running natively in the kernel, called by WASM apps" design.
- **No `no_std` / no Component Model bindings.** Loro assumes a heap and JS-style WASM imports. Embedding into a Component Model world requires either writing a WIT facade by hand or living with `wasm-bindgen` shape (which is incompatible with the Component Model ABI). Non-trivial integration work.
- **Bundle weight.** 1 MB WASM is fine for a kernel module but heavy for per-app code. A Myrhiza app that imports a Loro state-apply component pays that cost once per app instance.
- **Maturity gap.** Yjs has 5+ years of production scar tissue; Automerge has academic provenance and the Ink & Switch ecosystem; Loro has neither. Choosing Loro is choosing the better algorithm story (Fugue, verified move-tree) over the better operational story (years in production).
- **Stewardship risk.** Bus factor 1. If Myrhiza takes a hard dependency on Loro for `state-apply`, that's load-bearing on one person's continued involvement. The MIT license + clean Rust code mean a fork is feasible; the depth of the algorithm work means a fork would be expensive to maintain.

## Sources

- [github.com/loro-dev/loro](https://github.com/loro-dev/loro) — primary repo
- [github.com/loro-dev/crdt-richtext](https://github.com/loro-dev/crdt-richtext) — Peritext + Fugue rich text impl
- [loro.dev](https://loro.dev) — homepage (returns 403 to scripted fetches; content via Google index)
- [www.loro.dev/llms-full.txt](https://www.loro.dev/llms-full.txt) — LLM-friendly docs export
- [crates.io/crates/loro](https://crates.io/crates/loro) — `loro` 1.12.0
- [npmjs.com/package/loro-crdt](https://www.npmjs.com/package/loro-crdt) — `loro-crdt` 1.12.1 (2026-04-29)
- [docs.rs/loro/latest/loro/struct.LoroDoc.html](https://docs.rs/loro/latest/loro/struct.LoroDoc.html) — Rust API reference
- [arxiv.org/abs/2305.00583](https://arxiv.org/abs/2305.00583) — Weidner & Kleppmann 2023, _The Art of the Fugue_
- [martin.kleppmann.com/papers/move-op.pdf](https://martin.kleppmann.com/papers/move-op.pdf) — Kleppmann et al. 2021, "A Highly-Available Move Operation for Replicated Trees" (IEEE TPDS 33(7))
- [martin.kleppmann.com/2020/04/27/papoc-list-move.html](https://martin.kleppmann.com/2020/04/27/papoc-list-move.html) — Kleppmann 2020, "Moving Elements in List CRDTs"
- [github.com/josephg/diamond-types](https://github.com/josephg/diamond-types) — Eg-walker origin
- [github.com/dmonad/crdt-benchmarks](https://github.com/dmonad/crdt-benchmarks) — community CRDT benchmark harness
- [discuss.yjs.dev/t/yjs-vs-loro-new-crdt-lib/2567](https://discuss.yjs.dev/t/yjs-vs-loro-new-crdt-lib/2567) — dmonad/zxch3n debate on benchmark methodology
- [news.ycombinator.com/item?id=38248900](https://news.ycombinator.com/item?id=38248900) — Loro Show HN
- [news.ycombinator.com/item?id=39102577](https://news.ycombinator.com/item?id=39102577) — Loro rich text HN thread

See sibling files: [automerge.md](automerge.md), [yjs.md](yjs.md). Eg-walker / diamond-types is mentioned but not given its own file in this folder — see [crdt-theory.md](crdt-theory.md) §10 and [history.md](history.md) for treatment.
