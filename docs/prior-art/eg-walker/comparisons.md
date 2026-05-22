**Date:** 2026-05-22
**Status:** active
**Subject:** Eg-walker / diamond-types vs Automerge / Yjs / Loro / OT — head-to-head on the dimensions that matter for Myrhiza `state-apply`

# Comparisons

This file sits next to [`../crdts/comparisons.md`](../crdts/comparisons.md) and is best read together with it. The CRDT survey compares three libraries; this comparison adds eg-walker as a fourth datapoint with a different paradigm.

## Paradigm-level: what's stored vs what's computed

| System | On-disk form | Merge computation |
|---|---|---|
| **Automerge** | Columnar log of CRDT ops (RGA-flavoured + Peritext) | Merge by walking the op log under RGA rules |
| **Yjs** | Per-struct linked list with `originLeft`/`originRight` + tombstones (YATA) | Merge by inserting into the linked list per YATA rules |
| **Loro** | Per-container Fugue items for text, dedicated structures per type | Merge by computing Fugue positions on the persistent structure |
| **Eg-walker** | Append-only event graph of raw ops + causal parents | Merge by topologically walking the graph and transforming positions OT-style |
| **OT (Wave/ShareJS)** | Stream of operations transformed in real time | Merge by transforming concurrent operations server-side |

The eg-walker move: persist the operations, not the merged structure. The merged structure is computed when needed. CRDTs persist the merged structure (with metadata); OT persists nothing and re-transforms forever.

## Performance (per the paper's benchmarks)

The paper's benchmarks ran against seven editing traces (`datasets/raw/` in [`egwalker-paper`](https://github.com/josephg/egwalker-paper)). Comparators: diamond-types eg-walker, diamond-types CRDT (legacy), Automerge, Yjs, Yrs (Yjs Rust port), OT reference.

Headline (from abstract):

- **Steady-state memory:** "order of magnitude less memory" than existing CRDTs.
- **Load from disk:** "orders of magnitude faster" than existing CRDTs.
- **Long-running branch merge:** "orders of magnitude faster" than OT.
- **Worst case:** "merging performance of Eg-walker is comparable with existing CRDT algorithms" — the floor matches Yjs/Automerge, not better-than.

The headline numbers are *implementation-specific* — they hold for diamond-types vs Automerge 2.x / Yjs `13.x`. Automerge 3.0 (released after the paper benchmarks ran) closed much of the encoding gap. The relative ordering still holds qualitatively; the multipliers should be treated as paper-pinned, not eternal.

Note: the paper's benchmarks pre-date Loro `1.0+` and Yjs `14` prerelease (see [`../crdts/comparisons.md`](../crdts/comparisons.md) for current Loro/Yjs numbers). Treat the comparison as a 2024-snapshot, not a 2026-current state.

## Per-dimension comparison

### Memory at rest

| | Diamond-types (eg-walker) | Automerge 3.0 | Yjs 13.x | Loro 1.x |
|---|---|---|---|---|
| Per-op overhead | ~1-10 bytes (columnar+compression) | <1 byte/char (3.0) | ~10-20 bytes/struct | ~5-15 bytes |
| Tombstones | Implicit (deletes are ops in graph; no per-char tombstone) | Yes; columnar-compressed | Yes; struct-level | Yes; per-container |
| Snapshot size | Snapshot is the materialised state + frontier | Document state is the op log | Linked-list structure | Persistent CRDT structure |

Eg-walker wins on memory **at rest** when the document has been "settled" — the snapshot is just the materialised text + a small frontier. CRDTs always carry per-character metadata.

### Steady-state memory (active editing)

Eg-walker is comparable to Yjs/Loro in steady state — the snapshot + a small staging area for in-flight ops. The headline memory win is most pronounced for *loading from disk*, where the CRDT has to reconstruct full per-character metadata while eg-walker only has to deserialise the snapshot + the recent ops.

### Long-running offline branch merge

This is eg-walker's strongest result vs OT, and a moderate result vs CRDTs:

- **OT:** breaks down on long-running branches. The transformation function composes badly when the divergence is large; many production OT systems (Google Docs, ShareJS) require a server-mediated reconciliation.
- **CRDTs:** merge correctly regardless of divergence size, but pay the full cost of re-traversing the per-char metadata on both sides.
- **Eg-walker:** merges correctly, and only pays for the *concurrent subgraph* — operations causally after the divergence point. The non-concurrent operations don't participate in the merge step.

### Load from disk

This is the headline win:

- **Automerge 2.x:** loading a large document is "minutes" (Gentle's 2021 benchmarks). Automerge 3.0 closes much of this gap with columnar decoding.
- **Yjs:** loading is fast but rebuilds the full struct linked list in memory.
- **Loro:** loading is fast for the snapshot form; rebuilding the persistent CRDT structure is non-trivial for long histories.
- **Eg-walker:** loading is just `deserialise snapshot + frontier`. The operations only need to be decoded when concurrent merges happen.

### Sync protocol

| System | Sync model |
|---|---|
| **Automerge** | State-vector-based delta sync (`automerge-repo`) |
| **Yjs** | State-vector encoding + diff (custom binary protocol) |
| **Loro** | Snapshot + update format |
| **Eg-walker** | Op-set difference computation; gossip by causal frontier |
| **Hypercore** | Append-only log; gossip by Lamport range |
| **Willow / Myrhiza** | Per-author Merkle DAG; gossip by causal frontier |

Eg-walker's sync model is **structurally identical** to Hypercore's and to Willow/Myrhiza's. CRDTs need a state-vector layer; eg-walker doesn't.

### Determinism

| System | Logical determinism | Byte-equal at rest |
|---|---|---|
| **Automerge** | Yes (same op set → same doc) | Yes within a version; encoding can change across releases |
| **Yjs** | Yes | No — internal struct ordering can differ |
| **Loro** | Yes | No — internal Fugue layout can differ |
| **Eg-walker** | Yes (paper claims bit-identical convergence) | **Snapshot byte-equal is implementation-defined**; the graph is canonical, the snapshot is a cache |
| **OT** | No (depends on transformation order) | No |

Eg-walker's determinism is closer to Myrhiza's `state-apply` requirement than any of the CRDTs: the *graph* is canonical (bit-equal across peers because the op set is the same), the *snapshot* is a derived cache.

### Component Model integration

None of the four ship as Component Model artefacts. Diamond-types ships as raw wasm-bindgen (`diamond-types-web`/`-node`); Automerge 3.0 ships wasm-bindgen; Yjs ships pure-JS (no WASM needed in canonical form); Loro ships wasm-bindgen for browser + native Rust crate.

All four would need a Myrhiza-authored WIT wrapper. Same gap. ([`../crdts/open-problems.md` §10](../crdts/open-problems.md).)

### Schema migration / data shape

| System | Beyond text |
|---|---|
| **Automerge** | Full Automerge JSON tree (maps, lists, counters, text, marks) |
| **Yjs** | Y.Doc with Y.Map, Y.Array, Y.Text, Y.XmlFragment |
| **Loro** | LoroDoc with LoroMap, LoroList, LoroText, LoroTree (moveable) |
| **Eg-walker** | **Text only** in the paper; `more_types` branch experimental for JSON |

This is the cleanest argument *against* eg-walker as a general state-apply substrate: it's specialised for text. If Myrhiza wants a "general-purpose deterministic-merge runtime" Loro/Automerge cover more shapes today. The eg-walker *technique* may extend to other shapes; the published algorithm does not.

### Tree-shape concurrent move

| System | Concurrent move |
|---|---|
| **Loro** | Yes — Kleppmann 2021 move-op CRDT |
| **Automerge** | Limited |
| **Yjs** | No (apps simulate via delete + reinsert) |
| **Eg-walker** | Not addressed by paper |

For Myrhiza state with tree shape, Loro is still the only off-the-shelf answer. Eg-walker doesn't compete here yet.

### Maturity / production hardening

| System | Production users | Notes |
|---|---|---|
| **Automerge** | GoodNotes, Bowtie, Ink & Switch demos | Ink & Switch backs the project; healthy stewardship |
| **Yjs** | Proton Docs, JupyterLab, AFFiNE, Linear (in stack); via Tiptap/Hocuspocus most production editors | Largest editor ecosystem; bus-factor-1 (Kevin Jahns) |
| **Loro** | None at scale verified | "Production-ready" status disputed (per [`../crdts/critiques.md` §10](../crdts/critiques.md)) |
| **Eg-walker** | **None at scale.** Diamond-types crates.io: 27K total / 3K recent downloads. | Research-grade-but-shipping; algorithm is paper-vetted (Best Artifact Award) but no flagship app |

Eg-walker is **less production-hardened than Automerge or Yjs**. Comparable to Loro in production posture (no named at-scale users) but with stronger algorithmic credentials (peer-reviewed paper + award).

## Recommendation matrix for Myrhiza

| Need | Best fit | Why |
|---|---|---|
| Production text editing in app today, willing to wrap | **Yjs (via yrs)** | Largest ecosystem, real production users; bus-factor accepted |
| Production multi-type CRDT (maps/lists/text), willing to wrap | **Automerge** | Healthiest stewardship, broadest data model |
| Tree-shape with concurrent move | **Loro** | Only off-the-shelf answer; tolerate "no at-scale users" |
| Match Myrhiza's deterministic-event-log-replay paradigm | **Eg-walker (the technique)** | Algorithm shape lines up with `state-apply`; implementation maturity gap |
| Long-running peer offline merges with tiny snapshots | **Eg-walker** | Memory + load-time win is real and unique |
| Cross-app interop | **None** (see [`../crdts/open-problems.md` §9](../crdts/open-problems.md)) | No CRDT lib interoperates with another |

For Myrhiza specifically: **eg-walker is the algorithm that should inspire `state-apply`'s shape; diamond-types is not yet the implementation to vendor.** The right path is to build `state-apply` components around the event-log-replay pattern eg-walker validates, and consider diamond-types or a Myrhiza re-implementation when the first text-heavy app needs it.

## Sources

- Paper (benchmarks section): <https://arxiv.org/abs/2409.14252>
- Paper artefacts: <https://github.com/josephg/egwalker-paper>
- Diamond-types: <https://github.com/josephg/diamond-types>
- Automerge 3.0 release: <https://automerge.org/blog/automerge-3/>
- Yjs internals: <https://github.com/yjs/yjs/blob/main/INTERNALS.md>
- Loro version docs: <https://loro.dev/docs/advanced/version_deep_dive>
- Companion: [`../crdts/comparisons.md`](../crdts/comparisons.md)
