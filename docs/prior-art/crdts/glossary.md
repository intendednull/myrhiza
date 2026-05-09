**Date:** 2026-05-09
**Status:** active
**Subject:** Glossary of CRDT terms used across this folder

# Glossary

Terms used across [automerge.md](automerge.md), [yjs.md](yjs.md), [loro.md](loro.md), and the cross-cutting files. Cross-references point to the file with the deepest treatment.

## CRDT theory

- **CRDT** — Conflict-Free Replicated Data Type. A data structure with merge operations satisfying associativity, commutativity, and idempotence so that concurrent updates from multiple replicas converge without coordination. See [crdt-theory.md](crdt-theory.md).
- **CvRDT** — Convergent / state-based CRDT. Replicas exchange full state; merge function is a join in a join-semilattice. See [crdt-theory.md](crdt-theory.md).
- **CmRDT** — Commutative / operation-based CRDT. Replicas exchange operations; operations must commute. The three libraries here are all CmRDT-flavored. See [crdt-theory.md](crdt-theory.md).
- **Strong Eventual Consistency (SEC)** — replicas that have seen the same set of updates have the same state. Looser than linearizability; the consistency guarantee CRDTs provide. See [crdt-theory.md](crdt-theory.md).
- **Interleaving anomaly** — concurrent insertions of two text passages at the same anchor produce a character-by-character interleave (`HHEELLLLOO` instead of `HELLOHELLO`). The flagship correctness criterion separating modern text CRDTs (Fugue) from older ones (Logoot, Treedoc). See [crdt-theory.md](crdt-theory.md), [comparisons.md](comparisons.md).
- **Maximal non-interleaving** — Fugue's formal correctness property: concurrent insertions of contiguous runs are kept contiguous in the merged output. Proven for FugueMax in Weidner & Kleppmann 2023. See [crdt-theory.md](crdt-theory.md).
- **Tombstone** — a marker for a deleted element kept in the data structure to preserve causal context. All three libraries keep tombstones; Yjs has the most aggressive GC story. See [open-problems.md](open-problems.md).

## Algorithms

- **YATA** — Yet Another Transformation Approach. List CRDT used by Yjs. Each insert records `(origin, originRight)` pointers + Lamport ID; concurrent inserts at the same anchor break ties on `clientID` comparison. See [yjs.md](yjs.md), [crdt-theory.md](crdt-theory.md).
- **RGA** — Replicated Growable Array. List CRDT used by Automerge. Each element has a unique ID; new elements link to a left-anchor; concurrent inserts at same anchor ordered by ID. See [automerge.md](automerge.md), [crdt-theory.md](crdt-theory.md).
- **Fugue** — list CRDT used by Loro for text. Tree of positions; designed for maximal non-interleaving. Weidner & Kleppmann 2023. See [loro.md](loro.md), [crdt-theory.md](crdt-theory.md).
- **Peritext** — span-based CRDT for rich-text formatting (bold, italic, links). Used by Automerge (built-in marks) and Loro (via `crdt-richtext`). Litt, Lim, Kleppmann, van Hardenberg 2021 (Ink & Switch essay) / 2022 (PACMHCI 6 CSCW2 art. 531). See [automerge.md](automerge.md), [loro.md](loro.md).
- **Moveable Tree (Kleppmann 2021)** — algorithm for concurrent move operations on a tree without producing cycles. IEEE TPDS, Isabelle/HOL-verified. Loro's tree container uses it; Yjs has no native move; Automerge has limited move support. See [loro.md](loro.md), [open-problems.md](open-problems.md).
- **Eg-walker** — Joseph Gentle & Martin Kleppmann 2024 (arXiv 2409.14252; EuroSys 2025). Replicated-data structure that achieves CRDT correctness via local OT-like state computation rather than a CRDT proper; outperforms CRDTs on size and speed in benchmarks. Loro's op-log walking is influenced by it (via Gentle's diamond-types). See [crdt-theory.md](crdt-theory.md), [critiques.md](critiques.md).
- **OT** — Operational Transformation. Pre-CRDT lineage (Google Docs, ShareJS). Operates on operations rather than commutative state; requires a central coordinator. The architectural alternative CRDTs aimed to replace. See [comparisons.md](comparisons.md), [critiques.md](critiques.md).

## Identity / clocks

- **Lamport timestamp** — `(counter, replicaID)` pair providing partial order over events. Used by all three libraries.
- **Vector clock / state vector** — per-replica counter map encoding causal context. Yjs uses a state vector for delta sync. Loro uses version vectors. See [yjs.md](yjs.md), [comparisons.md](comparisons.md).
- **Actor ID (Automerge)** — 16-byte random identifier per replica session. See [automerge.md](automerge.md).
- **clientID (Yjs)** — 53-bit random integer (JS safe-integer range) assigned on first insert. See [yjs.md](yjs.md).
- **Peer ID (Loro)** — `u64` per-replica identifier used in change graph. See [loro.md](loro.md).
- **Frontier** — set of latest change IDs known to a replica; used as `version` argument to time-travel queries. Loro term. See [loro.md](loro.md).

## Sync / on-wire

- **Bloom-filter sync (Automerge)** — peers exchange Bloom filters of changes-they-have so each computes the changes-the-other-needs without round-tripping every hash. ~10 bits/commit, 7 hash probes. See [automerge.md](automerge.md), [comparisons.md](comparisons.md).
- **State-vector sync (Yjs)** — peer A sends `Y.encodeStateVector()`; peer B replies with `Y.encodeStateAsUpdate(doc, A_sv)` containing only operations A is missing. One round-trip. See [yjs.md](yjs.md).
- **Version-vector sync (Loro)** — peer A sends version vector; peer B replies with delta. One round-trip. See [loro.md](loro.md).
- **Update format** — binary encoding of one or more operations for over-the-wire transfer. Yjs has `update v1` and `update v2` formats (v2 is columnar, more compact). Automerge uses columnar binary chunks. Loro uses its own columnar encoding.
- **Document save format** — binary serialization of full document state (compressed change history + materialized state). Different from incremental update format on each library.

## Library-specific

### Automerge

- **`@automerge/automerge-wasm`** — wasm-bindgen wrapper around the Rust core; loaded by the JS package internally. See [automerge.md](automerge.md).
- **`@automerge/automerge-repo`** — sync protocol + storage layer above the core CRDT. Handles network adapters (WebSocket, BroadcastChannel) and storage adapters (IndexedDB, NodeFS). See [automerge.md](automerge.md), [ecosystem.md](ecosystem.md).
- **DocHandle** — automerge-repo's per-document handle abstraction.
- **Mark** — Peritext-derived inline formatting tuple `(start, end, name, value)` stored outside the character sequence. See [automerge.md](automerge.md).
- **Change** — atomic unit of work; SHA-256 hashed; references parent changes by hash, forming a DAG. See [automerge.md](automerge.md).

### Yjs

- **`Y.Doc`** — root container holding all top-level shared types.
- **Shared type** — `Y.Map`, `Y.Array`, `Y.Text`, `Y.XmlElement`, `Y.XmlText`. Application-facing CRDT abstractions.
- **`Y.awarenessProtocol`** — separate ephemeral state layer (cursor, presence, online status) — *not* part of the CRDT. Designed to be GC'd on disconnect. See [yjs.md](yjs.md).
- **StructStore** — internal Yjs index from `(clientID, clock)` to inserted Item.
- **`yrs`** — Rust port of Yjs at `y-crdt/y-crdt`, separate org. Binary protocol-compatible. See [yjs.md](yjs.md), [ecosystem.md](ecosystem.md).
- **Hocuspocus** — Tiptap's Yjs backend service. See [ecosystem.md](ecosystem.md).
- **Liveblocks** — commercial Yjs hosting platform. See [ecosystem.md](ecosystem.md).
- **Y-Sweet** — open-source Yjs server (Drifting in Space). See [ecosystem.md](ecosystem.md).
- **`y-websocket`, `y-webrtc`, `y-indexeddb`** — Yjs network/persistence providers. See [yjs.md](yjs.md).

### Loro

- **`LoroDoc`** — root container.
- **Container** — typed sub-document (`LoroText`, `LoroList`, `LoroMap`, `LoroTree`, `LoroMovableList`). Each container picks a different CRDT algorithm.
- **`crdt-richtext`** — sibling crate at `loro-dev/crdt-richtext` providing Peritext-flavored rich-text marks layered on Fugue. See [loro.md](loro.md).
- **`loro-ffi`** — separate repo housing Swift + JS/WASM bindings via UniFFI. See [loro.md](loro.md).
- **Shallow snapshot** — Git-shallow-clone-style truncation of history before a chosen "Critical Version." See [loro.md](loro.md).
- **Critical Version** — pinned point in history before which old changes can be discarded; the boundary is chosen so causal structure remains intact for reachable replicas.
- **diamond-types** — Joseph Gentle's pre-Eg-walker Rust list CRDT; Loro's op-log walking algorithm is adapted from it. See [loro.md](loro.md), [crdt-theory.md](crdt-theory.md).

## Cross-substrate (for comparison with neighbor folders)

- **Hypercore** ([Pears](../pears/)) — append-only signed log. Different convergence model: linearization of writer logs via Autobase rather than CRDT merge. See [`../pears/hypercore-stack.md`](../pears/hypercore-stack.md).
- **Source chain** ([Holochain](../holochain/)) — per-agent append-only signed log; convergence via deterministic validation in WASM zomes, not CRDT. See [`../holochain/`](../holochain/).
- **state-apply** (Myrhiza) — pure WASM Component Model function `(prior_state, event) → next_state`. CRDTs are one candidate for what `next_state` actually is; the choice has determinism + on-disk-format implications.
- **Vat** ([Agoric](../agoric-endo/)) — single-threaded ocap compute unit with snapshot-and-replay determinism. Different convergence model: deterministic re-execution from logged input rather than CRDT merge. See [`../agoric-endo/vat-model.md`](../agoric-endo/vat-model.md).

## Sources

- Shapiro et al. 2011: <https://hal.inria.fr/inria-00555588>
- YATA (Nicolaescu et al. 2016): <https://www.researchgate.net/publication/310212186>
- Fugue (Weidner & Kleppmann 2023): <https://arxiv.org/abs/2305.00583>
- Move-tree (Kleppmann et al. 2021): <https://martin.kleppmann.com/papers/move-op.pdf>
- Eg-walker (Kleppmann 2024): <https://arxiv.org/abs/2409.14252>
- Peritext (Litt, Lim, Kleppmann, van Hardenberg): <https://www.inkandswitch.com/peritext/>
- Per-library repos: see [README.md](README.md) and per-library files.
