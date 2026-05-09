**Date:** 2026-05-09
**Status:** active
**Subject:** Yjs — YATA-based collaborative-shared-types library; pure-JS core; single-maintainer (Kevin Jahns)

## What it is

Yjs is a CRDT framework that exposes its replicated state as *shared types* (`Y.Map`, `Y.Array`, `Y.Text`, `Y.XmlElement`, `Y.XmlText`) backed by the YATA list-CRDT algorithm. It is the most-starred CRDT repository on GitHub (21,791 stars) and ships in production at AFFiNE, Evernote, Proton Docs, JupyterLab, GitBook, and Tiptap-via-Hocuspocus (Linear is listed in the README but production-stack details are unverified — see [ecosystem.md](ecosystem.md)). The core is a single npm package `yjs` written in pure JavaScript with one runtime dependency (`lib0`); a separate Rust port (`yrs`) lives in its own org.

| Fact | Value |
|---|---|
| npm `yjs` (stable) | `13.6.30` (dist-tag `latest`) |
| npm `yjs` (RC track) | `14.0.0-8` (dist-tag `next`); `14.0.0-16` (dist-tag `beta`); GitHub release-tag `v14.0.0-rc.13` 2026-04-14 has no matching npm publish — **not yet GA** |
| License | MIT (Kevin Jahns + RWTH Aachen i5) |
| Repo | github.com/yjs/yjs — 21,791 stars, 767 forks, created 2014-07-29 |
| Maintainer | Kevin Jahns (`@dmonad`) — single maintainer, GitHub Sponsors–funded |
| Algorithm | YATA (Nicolaescu, Jahns, Derntl, Klamma, 2016) — list CRDT with `origin`/`originRight` pointers |
| Rust port | `yrs` 0.26.0 (crates.io, 2026-05-04) — separate org `y-crdt/y-crdt`, MIT |
| Core deps (yjs) | `lib0` (encoding lib by same author) |

## Architecture

Yjs ships as **one package** containing all shared types, the YATA engine, the binary update encoder, and the document object model. There is no Rust core under the JS — the production JS implementation is written by hand in pure JS for the JS runtime; the Rust port (`yrs`) is a separate, independently maintained reimplementation that aims for binary protocol compatibility but is not used by `yjs` itself.

A `Y.Doc` is the root container. It holds:

- A 53-bit random `clientID` assigned per client on first insert (fits JS safe-integer range).
- A `StructStore` indexing every inserted item by `(clientID, clock)`.
- A tree of doubly-linked lists in *document order*; each item has `left`/`right`/`parent` pointers.
- A search-marker cache (~80 most-recently-used insert positions) that reduces position-to-ID lookup from O(n) to amortized O(log n) for typical editing patterns.
- Top-level shared types accessed via `doc.getMap('name')`, `doc.getArray('name')`, etc. — the *name* keys the type to its slot in the doc.

Why pure JS rather than wasm-wrapped Rust: Kevin's design philosophy prioritizes minimal cold-start cost, no wasm boundary marshalling, and tight integration with editor bindings (ProseMirror/Quill/CodeMirror) that already live in JS. The cost is duplicated effort vs. `yrs` and weaker performance ceilings — but Yjs benchmarks consistently win on real-world editing workloads anyway because the algorithmic improvements (run-length compression of `Item`, GC tombstones) dominate.

## YATA algorithm

YATA = *Yet Another Transformation Approach* (Nicolaescu, Jahns, Derntl, Klamma — RWTH Aachen, 2016). Key insight: every insert records a pair of pointers — `origin` (ID of the element to its left at insert time) and `originRight` (ID of the element to its right) — plus its own `(clientID, clock)` Lamport ID. Concurrent inserts at the same anchor are deterministically ordered by:

1. comparing `originRight` to detect concurrency,
2. then breaking ties by Lamport `clientID` comparison.

This differs from RGA (used by Automerge): RGA tracks only a single causal-predecessor pointer and resolves concurrent siblings purely by clientID. YATA's two-pointer representation gives strictly better intention preservation for the case "two users each insert a run after the same character" — the runs interleave less and stay grouped per user. Yjs further adds `originRight` as an optimization on the original paper (the paper used only `origin`); see `INTERNALS.md`.

A formal mechanized proof of YATA's preservation and commutativity properties exists in Lean as `iasakura/lean-yjs`, which also flagged errors in the original paper's pseudocode. Treat the paper as the conceptual reference and Yjs's actual code (or `lean-yjs`) as the operational definition.

## Shared types

| Type | Purpose | Notes |
|---|---|---|
| `Y.Map` | Key→value store | Last-writer-wins per key by Lamport order; older values flagged deleted but kept until GC. |
| `Y.Array` | Ordered list | Direct list-CRDT, items integrated by YATA. Supports nested types. |
| `Y.Text` | Rich text with formatting | List of characters; runs of same-client sequential inserts compressed into one `ItemString`. Quill-Delta interop via `applyDelta`/`toDelta`. |
| `Y.XmlElement` | XML-flavored element node | Children + attributes; used by `y-prosemirror`. No XML-validation enforcement. |
| `Y.XmlFragment` | Container for `XmlElement`/`XmlText` | Document-fragment root. |
| `Y.XmlText` | Text node with attributes | XML-aware variant of `Y.Text`. |

Internally **everything is a list**: `Y.Map` entries are list items keyed by entry name, `Y.Text` is a character list with run-length compression. Reusing the YATA engine for all types is the central design simplification.

## State vectors

A *state vector* is `Map<clientID, nextExpectedClock>` — for each client this doc has heard from, the smallest clock that has *not* been observed yet. It is structurally a version vector, but Yjs uses it only to compute "what am I missing?" — **not for causality tracking**.

Two-roundtrip incremental sync:

```js
const sv1 = Y.encodeStateVector(ydoc1)              // ~tens of bytes per client
const sv2 = Y.encodeStateVector(ydoc2)
const diff1to2 = Y.encodeStateAsUpdate(ydoc1, sv2)  // only structs ydoc2 lacks
const diff2to1 = Y.encodeStateAsUpdate(ydoc2, sv1)
Y.applyUpdate(ydoc2, diff1to2)
Y.applyUpdate(ydoc1, diff2to1)
```

There is also an "offline" path: `Y.encodeStateVectorFromUpdate` and `Y.diffUpdate` operate directly on encoded update binaries without instantiating a `Y.Doc` — useful for relay servers that want to dedupe diffs without parsing CRDT state.

## Awareness protocol

`y-protocols/awareness` is **a separate module, not part of the CRDT**. It carries ephemeral per-peer state — cursor position, selection range, presence (name, color), typing indicators — using a simple last-write-wins map keyed by `clientID` with timestamp-based liveness (peers who go silent are reaped after a timeout).

The deliberate split: putting cursor data into the CRDT would make it grow unboundedly with every cursor move, defeat tombstone GC, and serialize ephemeral updates into the persistent log. Awareness sits next to the CRDT on the wire and is delivered through the same provider connection but stored separately. Every editor binding (y-prosemirror, y-codemirror, y-monaco) wires cursors through `Awareness`, never through the doc.

For Myrhiza this is a load-bearing pattern: any "presence" channel must be off the deterministic state-apply path or `state-apply` becomes non-pure.

## Update format

Updates are binary, produced by `Y.encodeStateAsUpdate` and consumed by `Y.applyUpdate`. They are **commutative and idempotent** — applying any subset, in any order, any number of times, converges to the same state.

There are **two on-the-wire formats**:

- **V1** — original. Default. All providers speak V1.
- **V2** — opt-in via `Y.applyUpdateV2`/`Y.encodeStateAsUpdateV2`/`'updateV2'` event. Better compression. Not used by default because most providers haven't migrated. Conversion helpers `convertUpdateFormatV1ToV2` and `convertUpdateFormatV2ToV1` exist.

V14 changes the TypeScript surface to `Uint8Array<ArrayBuffer>` and refactors the encoder around a unified delta representation, but does not (per the RC notes) introduce a V3 wire format. **V1 wire compatibility is preserved across v13 → v14**, which is why every provider can keep speaking V1.

`Y.obfuscateUpdate` produces a structurally-equivalent update with content scrambled — useful for bug reports without leaking content.

## Providers

A *provider* glues a `Y.Doc` to a transport and/or storage. They are pluggable and stackable.

| Provider | Role | Topology |
|---|---|---|
| `y-websocket` | WebSocket transport + simple Node server | Centralized hub-and-spoke |
| `y-webrtc` | WebRTC mesh w/ public signaling servers | Peer-to-peer (small rooms) |
| `y-indexeddb` | Persist full doc + incremental updates in browser | Local-first persistence |
| `Hocuspocus` | Tiptap-managed yjs server: SQLite, webhooks, auth | Centralized, managed |
| `@liveblocks/yjs` | Hosted WebSocket + persistence | Centralized, SaaS |
| `y-sweet` | Standalone Rust-based server, S3/FS persistence | Centralized, OSS or hosted |
| `teleportal` | DIY framework for building yjs sync servers | Any |
| `y-libp2p`, `y-atproto`, `nostr-crdt`, `matrix-crdt` | P2P/federated transports | Various |
| `y-redis`, `y-postgresql`, `y-mongodb-provider` | Server-side persistence backends | Pair with transport |

Providers handle: connection lifecycle, awareness exchange, state-vector exchange, incremental update relay, and (for persistence variants) loading on startup. They do **not** modify CRDT semantics — every provider applies updates through the same `Y.applyUpdate` path.

## yrs (Rust port)

`yrs` is the Rust reimplementation of Yjs maintained at `y-crdt/y-crdt` by Bartosz Sypytkowski (`@Horusiath`), Kevin Jahns, and John Waidhofer. As of 2026-05-04 the published version is **0.26.0** on crates.io (~1.48M downloads cumulative), MIT-licensed, no MSRV pinned in metadata.

The repo also contains:

- `lib0` — Rust reimpl of the encoding lib.
- `yffi` — C FFI wrapper.
- `ywasm` — WASM/JS binding (note: its observer/document-event API is documented as *incompatible with yjs* — diverges from the JS reference).

Downstream language bindings ride on `yrs`: `pycrdt` (Python), `yrb` (Ruby), `ydotnet` (.NET), `yswift` (Swift), `ykt` (Kotlin), `yr` (R), `y_ex` (Elixir).

The y-crdt README's parity table compares `yrs` 0.21 with Yjs 13.6 and shows full feature parity for the core (Text/Map/Array/Xml/sub-docs/snapshots/sticky indexes/undo/awareness). YArray-move and YMap-weak-links land in `yrs` mainline before `yjs` mainline (yjs has them on feature branches). **Wire-format compatibility with Yjs V1 is the explicit project goal** — yrs and yjs can sync directly.

For Myrhiza: `yrs` is the realistic candidate for embedding YATA in a `state-apply` WASM component. Pure-JS Yjs is unsuitable for a WASM runtime; `ywasm` exists but is JS-targeted, not Component-Model-targeted. A `state-apply` component would link `yrs` directly via a Rust crate.

## Performance

Headline behaviors (per `dmonad/crdt-benchmarks` and the Yjs `INTERNALS.md`):

- **Run-length compression**: sequential inserts by the same client at the same position collapse into one `Item` with a contiguous content buffer. The `[a][b][c]` → `[abc]` example in the docs is the common case in real text editing.
- **Tombstone reduction**: deleted content's payload is dropped (replaced with `ItemDeleted` carrying only length); when a parent type is deleted, all descendants collapse to lightweight `GC` structs that further merge with adjacent GCs.
- **Search-marker cache**: ~80-entry MRU cache of recent insert positions makes typical editing O(log n)-ish rather than O(n).
- **Worst case**: paper-cited "edit a 5000-character document right-to-left" — Yjs still completes; benchmarks generally show Yjs winning on document-size and apply-time vs. older Automerge versions, though the gap narrowed substantially after Automerge's 2.x rewrite. **Always re-run `dmonad/crdt-benchmarks` against the current versions before citing numbers** — the lead changes by release.

Operations are at the character level (or run level when compressed). There is no operation-batching window required for correctness; transactions are a developer-ergonomics feature, not a CRDT requirement.

## Determinism

Yjs guarantees **state convergence** — two clients that have observed the same set of updates land on byte-identical materialized state, in the same `Item` graph order. This is what YATA's commutativity proof establishes.

Whether two clients producing the *same operation* generate the *byte-identical update binary* is a different question. The update wire format is deterministic given `(clientID, clock, content, origin, originRight)`, but `clientID` is a random 53-bit integer chosen at `Y.Doc` construction. Two clients given the same script and the same starting state but different `clientID`s produce different but semantically equivalent updates — and `Y.mergeUpdates` of them still converges. This is *strong eventual consistency*, not bit-deterministic operation hashing.

For Myrhiza's `state-apply`: applying a Yjs update to a Yjs doc *is* a deterministic pure function of `(prior state, update)` — same input bytes, same output bytes. The non-determinism lives in the *generation* side (when a client picks a random `clientID`), which is the `state-propose` side in our model. This maps cleanly onto the propose/apply split.

## v14 RC track

v14 has been in RC since 2025. npm `next` dist-tag is `14.0.0-8`; npm `beta` is `14.0.0-16`. GitHub release tag `v14.0.0-rc.13` (2026-04-14) was tagged but never published to npm; the npm and git release cadences have diverged. No GA date announced. Notable changes harvested from release notes (early prereleases onward):

- **Single Yjs delta type**: refactor exposes one delta representation across all types via `lib0@v1` delta. `getContent` renamed to `toDelta`. Internal type system simplified.
- **`contentIds` / `contentMap` API**: new APIs for addressing and filtering content by ID ranges. `createContentIdsFromUpdate`, `intersectUpdateWithContentIds`, `excludeContentMap`.
- **AttributionManager**: `DiffAttributionManager`, `SnapshotAttributionManager`, `acceptAllChanges`/`rejectAllChanges`. Pluggable change-attribution renderer for diff/snapshot UIs.
- **Delta integration in nested types**: bug fixes in nested-delta diff with attribution.
- **Type rework**: updates are now typed as `Uint8Array<ArrayBuffer>`. ESM-only bundle (CJS path removed in rc.17).
- **`Y.mergeUpdates` optimization**: targeted at merging many updates efficiently (rc.1).
- **No V3 wire format**; V1/V2 encoders unchanged.

Migration concerns flagged: ESM-only bundle is a hard break for CommonJS consumers. The `Uint8Array<ArrayBuffer>` typing tightens generics in TS. The single-delta refactor is the largest internal change; bindings (`y-prosemirror` etc.) are tracking rc releases on their own beta tags.

Treat v14 as **not production-ready as of 2026-05-09**. New work that has to ship now should target 13.6.30. Greenfield work willing to track RCs can adopt v14 with the understanding that further breaks may land before GA.

## Implications for Myrhiza

- **YATA semantics are compatible with `state-apply` purity.** `Y.applyUpdate(prior_state_bytes, update_bytes) → next_state_bytes` is a pure function modulo `clientID` non-determinism, which is generation-side, not apply-side. The propose/apply split in Myrhiza maps onto Yjs's apply path cleanly. **(borrow)**
- **`yrs` is the realistic embedding target.** Pure-JS Yjs cannot run in a Component-Model `state-apply`. `yrs` 0.26.0 is Rust, MIT, wire-compatible with Yjs V1, and actively maintained — a `state-apply` component can link `yrs` and consume the Yjs binary update format directly. Verify wasm32 + Component Model build works; `yrs` is not currently advertised as having a `wasi-component-model` target (its WASM story is `ywasm` for JS hosts). **(verify before adopting)**
- **Awareness must live outside `state-apply`.** Yjs's deliberate split — ephemeral cursor/presence channel separate from the CRDT — is the correct shape for Myrhiza's interaction-profile components. Don't put presence in events. **(borrow)**
- **State-vector exchange is the right sync primitive for delta replay.** `(send state-vector, receive missing-since-vector)` is exactly the protocol Myrhiza event log replay wants between peers; no log-position pointers needed. **(borrow)**
- **Single-maintainer risk is real.** Yjs is Kevin Jahns alone. `yrs` has three maintainers. Building Myrhiza on `yrs` rather than `yjs` is *also* better on the bus-factor axis. **(avoid: leaning on the JS package as our reference impl)**
- **YATA conflict resolution depends on `clientID` tie-breaks.** If Myrhiza wants reproducible event ordering across re-runs of the same scenario (for testing/replay), it must record `clientID` choices in the event log. Random `clientID` per session means scenario-replays in a test harness need a deterministic seed. **(borrow with care)**

## Sources

- yjs repo: https://github.com/yjs/yjs
- yjs README + algorithm section + INTERNALS.md (above)
- yjs releases (v14 RC track): https://github.com/yjs/yjs/releases (verified via `gh api repos/yjs/yjs/releases`)
- yjs npm: https://www.npmjs.com/package/yjs (verified via registry.npmjs.org/yjs — `latest`: 13.6.30, `next`: 14.0.0-8, `beta`: 14.0.0-16)
- y-crdt (Rust port) repo: https://github.com/y-crdt/y-crdt
- yrs on crates.io: https://crates.io/crates/yrs (0.26.0, 2026-05-04)
- YATA paper (Nicolaescu, Jahns, Derntl, Klamma 2016): https://www.researchgate.net/publication/310212186_Near_Real-Time_Peer-to-Peer_Shared_Editing_on_Extensible_Data_Types
- Kevin Jahns — *Are CRDTs Suitable for Shared Editing?* https://blog.kevinjahns.de/are-crdts-suitable-for-shared-editing/
- Lean formal verification: https://github.com/iasakura/lean-yjs
- CRDT benchmarks: https://github.com/dmonad/crdt-benchmarks
- Awareness protocol: https://github.com/yjs/y-protocols/blob/master/awareness.js
- Yjs license: https://github.com/yjs/yjs/blob/main/LICENSE (MIT, Kevin Jahns + RWTH Aachen)

See siblings: [`glossary.md`](./glossary.md) for CRDT terminology, [`crdt-theory.md`](./crdt-theory.md) for algorithm-class background, [`comparisons.md`](./comparisons.md) for Automerge/Loro/Yjs feature matrix, [`ecosystem.md`](./ecosystem.md) for full provider/binding catalog, [`lessons.md`](./lessons.md) for Myrhiza-specific takeaways.
