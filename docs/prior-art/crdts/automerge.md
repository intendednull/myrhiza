**Date:** 2026-05-09
**Status:** active
**Subject:** Automerge — JSON-like CRDT for collaborative apps; Rust core + JS/WASM FFI; Ink & Switch stewardship

## What it is

Automerge is a JSON-shaped CRDT library: a document is a tree of maps, lists, text, and scalars that can be edited concurrently on multiple peers and merged automatically. The original implementation by Martin Kleppmann (Cambridge) was TypeScript, ~2017; the current production codebase is a Rust core (rewritten ~2020-2021) exposed via FFI to JS/WASM, C, and other languages. Stewardship is at Ink & Switch, with Alex Good ([@alexjg](https://github.com/alexjg)) and Orion Henry ([@orionz](https://github.com/orionz)) as full-time maintainers. See [crdt-theory.md](crdt-theory.md) for the academic lineage and [comparisons.md](comparisons.md) for how Automerge stacks against Yjs and Loro.

| Field | Value |
| --- | --- |
| npm package | `@automerge/automerge` 3.2.6 (2026-04-22) |
| crates.io | `automerge` 0.9.0 (~79K recent downloads) |
| Companion | `@automerge/automerge-repo` 2.5.5 (2026-03-31) |
| License | MIT |
| Repo | [github.com/automerge/automerge](https://github.com/automerge/automerge) — 6,258 stars, 246 forks, created 2019-12-26 |
| Language mix | JavaScript 57.5%, Rust 31.0%, TypeScript 5.9%, C 4.9% |
| Stewards | Ink & Switch (Alex Good, Orion Henry) |

## Architecture: Rust core + FFI

The repo describes itself as "a core Rust implementation which is exposed via FFI in javascript+WASM, C, and soon other languages." Workspace layout:

- `./rust/` — Rust core plus platform wrappers
- `./rust/automerge-wasm` — wasm-bindgen layer used by the JS package
- `./rust/automerge-c` — C FFI bindings
- `./javascript/` — idiomatic JS surface that internally drives `automerge-wasm`

The choice not to maintain a parallel pure-JS core is deliberate: the 2.0 announcement frames it as "identical" CRDT logic across platforms so "everyone benefits from new features and optimizations together." For Myrhiza this is the right shape — a single Rust crate is what `state-apply` would import.

## Document model

Automerge documents are JSON-like, rooted in a map. Value types:

- **Map** — string keys to any Automerge value
- **List** — RGA-flavored ordered sequence of values
- **Text** — sequence of UTF-8 characters with inline marks (Peritext-derived)
- **Counter** — merges by adding all concurrent operations
- **Scalars** — bool, signed/unsigned int, float, string, timestamp, byte array

Per the docs, "Maps have string keys and any automerge type as a value." Marks are tuples `(start, end, name, value)` storing inline formatting outside the character sequence.

Operations carry an **OpId** = `(actor, counter)`. Actor IDs are 16-byte random identifiers; sequence numbers per actor identify changes, and OpId pairs identify individual ops. (See [glossary.md](glossary.md) for OpId / actorId semantics shared with Yjs and Loro.)

## Change graph

A **change** is the unit of replication: a batch of operations attributable to one actor at one logical step. Changes form a hash DAG, intentionally git-shaped. Each change carries the SHA-256 hashes of its causal predecessors as `dependencies`:

> "Dependencies are represented as an array of 64-digit lowercase hexadecimal strings containing the SHA-256 hashes of the binary encoding of the changes that causally precede this change. The array is empty for the first ever change, contains one hash in the case of a linear editing history, and multiple hashes in the case of a 'merge commit'." (Automerge binary format spec)

The change hash itself is the 32-byte SHA-256 of `(chunk type 0x01 ‖ chunk length ‖ chunk contents)`. The first 4 bytes are also stored inline as a checksum. "Like git, points in the history of a document are identified by hash. Unlike git there can be multiple hashes representing a particular point (because automerge supports concurrent changes)."

Document state at any moment is therefore `(set of head hashes) + (compressed history of all changes leading to them)`.

## Sync protocol

The peer-to-peer sync protocol is bloom-filter-based and is grounded in [Kleppmann & Howard, "Byzantine Eventual Consistency and the Fundamental Limits of Peer-to-Peer Databases" (arXiv:2012.00472)](https://arxiv.org/abs/2012.00472). The Rust docs note the protocol "assumes a reliable in-order stream between two peers who are synchronizing a document."

Each `Message` carries:

- The sender's current heads (change hashes)
- A `Have` summary: a `BloomFilter` of changes the sender already knows (`"A summary of the changes that the sender of the message already has. This is implicitly a request to the recipient to send all changes that the sender does not already know about."` — Rust API docs)
- Any changes the receiver is now believed to need

Round-trip count is the protocol's headline property. From Kleppmann's blog post on the algorithm: **"Almost all reconciliations complete in one round trip."** The probability of needing a second pass is roughly 1%, third 0.01%, etc., driven by bloom filter false positives. The filter sizing referenced is **10 bits (1.25 bytes) per commit**.

When a false positive bites — a receiver thinks a predecessor was filtered when it wasn't actually present — a second exchange resolves it via graph traversal of dependencies.

## automerge-repo

[`@automerge/automerge-repo`](https://github.com/automerge/automerge-repo) is the practical batteries-included layer. The README: *"Automerge Repo is a wrapper for the Automerge CRDT library which provides facilities to support working with many documents at once, as well as pluggable networking and storage."* Current version: **v2.5.5 (2026-03-31)**, written ~98% TypeScript.

**Storage adapters:**
- `automerge-repo-storage-indexeddb` — browser
- `automerge-repo-storage-nodefs` — Node filesystem
- (filesystem adapter for general FS)

**Network adapters:**
- `automerge-repo-network-websocket` — client/server
- `automerge-repo-network-messagechannel` — cross-tab via MessageChannel
- `automerge-repo-network-broadcastchannel` — tab-to-tab (experimental)
- Reference `automerge-repo-sync-server` as a sample relay

Frontend integrations: React hooks, Svelte stores, Solid primitives.

For Myrhiza, automerge-repo is the canonical example of separating the deterministic CRDT core from non-deterministic transport/storage — exactly the kernel boundary Myrhiza enforces. See [ecosystem.md](ecosystem.md).

## Text CRDT

The list/text core is **RGA** (Roh, Jeon, Kim, Lee, *Replicated abstract data types*, JPDC 2011), giving each insertion a sequence number set to one greater than any seen, with deletions handled via tombstones. List ordering uses an RGA tree linked through OpId references.

**Rich text** as of Automerge 2.2 (2024) uses a **Peritext-inspired** algorithm developed by Ink & Switch with Kleppmann ([Peritext: A CRDT for Collaborative Rich Text Editing, ACM CSCW 2022](https://dl.acm.org/doi/10.1145/3555644)). Formatting marks are stored *outside* the character sequence as spans `(start, end, name, value)` with an "expand on insert" flag. Block markers (paragraphs, list items) are inserted into the text sequence with optional parent links for tree structure. Automerge ships an official ProseMirror binding.

## Performance characteristics

The well-known Kleppmann editing trace is **260,000 keystrokes producing a ~100,000-character paper**. Joseph Gentle's ["CRDTs go brrr"](https://josephg.com/blog/crdts-go-brrr/) benchmarked it across implementations:

| Impl | Trace time | RAM |
| --- | --- | --- |
| Automerge (pre-Rust) | 291 s | 880 MB |
| Yjs | 0.97 s | 3.3 MB |
| diamond-types (Rust) | 0.056 s | 1.1 MB |

Automerge 2.0 closed much of that gap. Per Ink & Switch, processing the same trace dropped from **13,052 ms (1.0.1) to 1,816 ms (2.0.1)** with peak RAM from 184 MB to 44.5 MB; on-disk save shrank from 146 MB to 129 KB ("less than one additional byte per character"). Automerge 3.0 (2025) cut **runtime memory by ~10×** by using the compressed representation in memory, not just on disk: "pasting Moby Dick consumes 1.3MB in version 3.0 versus 700MB in version 2.0," and "a document which hadn't loaded after 17 hours [is now] loading in 9 seconds."

Document size grows (compressed) roughly linearly in op count; the columnar/RLE format keeps overhead near 30%.

## Storage format

`Automerge.save()` emits a sequence of length-delimited chunks. Each chunk:

```
magic    [0x85, 0x6f, 0x4a, 0x83]   (4 bytes)
checksum first 4 bytes of SHA-256 over rest
type     0x00 Document | 0x01 Change | 0x02 CompressedChange
length   uLEB128
body     columnar-encoded data
```

Column types include actor, sequence number, max op, time, message, dependencies, plus per-op columns for object id, key, op id, action (`makeMap`, `set`, `del`, ...), value, and successors (in document chunks) or predecessors (in change chunks). Compression: RLE pairs `(length, value)`, delta encoding, boolean run-length, actor-index dedup. The least-significant 3 bits of a column descriptor encode column type; the 4th LSB indicates DEFLATE.

Integers are uLEB / signed LEB128, 64-bit max, shortest-form required (matters for hash determinism).

## Determinism

Automerge is built around determinism in the senses Myrhiza cares about:

1. **Change hashes are canonical SHA-256 over the binary encoding.** Shortest-form LEB128 plus a fixed column ordering means independent implementations producing the same logical change must produce the same bytes and therefore the same hash. A change is identified by content, not by who computed it.
2. **`Automerge.merge(a, b)` converges by construction.** Both peers end at the same set of head hashes; the materialized JSON is determined by RGA ordering rules over `(actor, counter)` OpIds, which are total. Two peers given the same change set produce the same logical document. (Round-tripping through `save()` → `load()` should yield the same observable doc; whether the *bytes* are identical depends on chunk packing choices — for byte-identical state you compare head hashes, not save() output.)
3. **The sync protocol is non-deterministic at the transport layer** (bloom filter false positives, peer ordering) but its outcome — which changes each peer ends up holding — is convergent.

For Myrhiza state-apply, the load-bearing claim is (1) and (2): a deterministic function from `(prior change set, new change)` to `(new change set, new head hashes)` is what the kernel needs.

## Implications for Myrhiza

- **Validates the Rust-core + WASM-FFI shape.** Automerge already ships exactly the pattern Myrhiza assumes: deterministic core in Rust, embedded in WASM for app code, with non-deterministic I/O (network, storage) as separate adapters. `state-apply` could in principle be a thin wrapper over the `automerge` crate.
- **Borrow: hash-DAG of changes as the canonical state representation.** Heads are a small set of SHA-256 hashes; full state is reconstructible from the change log. This matches Myrhiza's event-sourced framing — each event is a change, state is `apply(prior_state, event)`, and convergence is structural.
- **Borrow: bloom-filter sync protocol.** One round-trip P2P reconciliation at ~10 bits/commit is hard to beat for the kernel's peer-sync surface. Even if Myrhiza writes its own protocol, the [Kleppmann & Howard 2020](https://arxiv.org/abs/2012.00472) algorithm is the obvious starting point. See [lessons.md](lessons.md).
- **Cost to consider: opinionated document model.** Automerge enforces a JSON-shape and bakes RGA/Peritext into the type system. If Myrhiza apps want richer or different schema (e.g. relational, graph, signed-event semantics), embedding Automerge means living inside its ontology or wrapping it awkwardly.
- **Cost to consider: change format = ABI.** SHA-256 hashes over a specific columnar binary encoding mean you cannot evolve the format without breaking hash compatibility across peers. Myrhiza's own event format would inherit the same constraint, so this is informative rather than disqualifying — but the lesson is to nail the binary spec early.
- **Avoid: assuming Automerge's text CRDT is plug-and-play for non-text sequences.** Lists are RGA; rich text is Peritext. Both are tuned for human editing; CRDTs for, say, append-only logs or set-of-edges work better with simpler designs.

## Sources

- [github.com/automerge/automerge](https://github.com/automerge/automerge) — repo README, workspace layout, language mix, license
- [automerge.org/docs/reference/documents/](https://automerge.org/docs/reference/documents/) — document model, types, marks
- [automerge.org/automerge/automerge/sync/index.html](https://automerge.org/automerge/automerge/sync/index.html) — sync protocol Rust API docs
- [automerge.org/automerge-binary-format-spec/](https://automerge.org/automerge-binary-format-spec/) — binary chunk format, columnar encoding, change hashes
- [automerge.org/blog/automerge-2/](https://automerge.org/blog/automerge-2/) — 2.0 release notes, Rust rewrite, benchmarks
- [automerge.org/blog/automerge-3/](https://automerge.org/blog/automerge-3/) — 3.0 release notes, 10× memory reduction
- [automerge.org/blog/rich-text/](https://automerge.org/blog/rich-text/) — 2.2 rich-text, Peritext-inspired marks
- [github.com/automerge/automerge-repo](https://github.com/automerge/automerge-repo) — repo wrapper, storage/network adapters
- [martin.kleppmann.com/2020/12/02/bloom-filter-hash-graph-sync.html](https://martin.kleppmann.com/2020/12/02/bloom-filter-hash-graph-sync.html) — bloom-filter sync algorithm, round-trip math
- [arxiv.org/abs/2012.00472](https://arxiv.org/abs/2012.00472) — Kleppmann & Howard, *Byzantine Eventual Consistency and the Fundamental Limits of Peer-to-Peer Databases*
- [josephg.com/blog/crdts-go-brrr/](https://josephg.com/blog/crdts-go-brrr/) — Kleppmann editing trace benchmark, Automerge vs Yjs vs diamond-types
- [liangrunda.com/posts/automerge-internal-2/](https://liangrunda.com/posts/automerge-internal-2/) — internals walkthrough: change graph, actor IDs, columnar encoding
- [inkandswitch.com/peritext/](https://www.inkandswitch.com/peritext/) — Peritext rich-text CRDT design
- [dl.acm.org/doi/10.1145/3555644](https://dl.acm.org/doi/10.1145/3555644) — Litt et al., *Peritext: A CRDT for Collaborative Rich Text Editing*, ACM CSCW 2022
- Roh, Jeon, Kim, Lee, *Replicated abstract data types: Building blocks for collaborative applications*, JPDC 71(3):354–368, 2011 — original RGA paper
