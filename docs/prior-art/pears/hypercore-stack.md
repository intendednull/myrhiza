**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — the Hypercore data stack (Hypercore, Hyperdrive, Hyperbee, Autobase) as a P2P-state-replication design point

# The Hypercore Stack

Hypercore is the load-bearing data layer beneath everything Holepunch ships. Pears, Keet, the documented apps — they all sit on top of a small, opinionated set of primitives: Hypercore (signed append-only log), Hyperdrive (filesystem on Hypercore), Hyperbee (sorted KV B-tree on Hypercore), and Autobase (multi-writer linearization across N Hypercores). The stack predates the Pears runtime by years; if Pears went away tomorrow, the data layer would still be the more interesting prior art for Myrhiza.

This file covers the data layer in depth. The runtime side (Pear, Bare) and the network side (Hyperswarm, DHT, Holesail) live in companion files.

## Versions in scope (verified 2026-05-09)

| Package | npm latest | Released |
|---|---|---|
| `hypercore` | 11.30.1 | 2026-05-06 |
| `hyperdrive` | 13.3.2 | 2026-03-27 |
| `hyperbee` | 2.27.3 | 2026-01-27 |
| `autobase` | 7.28.0 | 2026-05-05 |
| `hyperblobs` | 2.12.0 | 2026-05-05 |
| `corestore` | 7.9.2 | 2026-03-21 |
| `hypercore-storage` | 2.9.0 | 2026-05-05 |
| `sodium-native` | 5.1.0 | — |

Two notes the README doesn't surface clearly:

- The `hypercore` README still says "the latest release is Hypercore 10". That's stale. Hypercore 11 has been on `latest` since January 2025 and is what every current Pears/Keet build uses. v10 is "LTS" only in the sense that the storage format and wire protocol are forward-compatible into v11 (per the v10 release notes).
- The `corestore` README has a footnote: the `latest` dist-tag is **Corestore 7**, not Corestore 11, "to avoid too much disruption" while v11 (RocksDB-backed) finishes validation. Corestore 11 is what binds to Hypercore 11's `hypercore-storage` and is the version actively in use inside Holepunch's own apps. If you `npm install corestore` today you get 7.x; if you read recent Holepunch source you see 11. Plan for both.

## Hypercore — the substrate

A Hypercore is one author's append-only log. Concretely:

- **Identity.** A 32-byte ed25519 public key identifies the core. The matching secret key signs every block (or, in v11, signs Merkle tree roots — see below). Anyone with the public key can verify; only the secret-key holder can append.
- **Content addressing.** Every block is committed into an incremental Merkle tree. The tree's root hash, plus the current length, plus a fork id, is what the author signs. Older versions signed each block; v10 collapsed this so a single signature in the "oplog" covers a whole batch (see `UPGRADE.md` v10 notes).
- **Sparse replication.** This is the property that distinguishes Hypercore from "sync the whole log" systems. A peer can request block N, receive block N along with the Merkle path to the signed root, and verify it without ever holding blocks 0..N-1. v11 made sparse-only: the `sparse: false` option is gone.
- **Wire protocol.** Binary, not JSON. Built on Noise for authenticated transport (the framing protocol is `protomux`). The Hypercore protocol is at major version 11; the major-version cuts have been semantic, not cosmetic (see below).
- **Truncate, fork, clear.** v10 added `truncate(newLength)` which rewinds the log and bumps a `fork` counter. Peers replicating an older fork are told to drop. `clear(start, end)` discards local block bytes (and gossips the loss to peers) without touching the Merkle tree — useful for storage reclaim.
- **Manifest.** v10/v11 introduced a "manifest" that describes how to verify the core: the hash function, the signers (an array, with quorum), the prologue, and optional `linked` references to associated cores. This is what makes multi-signer Hypercores possible without forking the protocol — the manifest just says "these N pubkeys, quorum of M". The core's `key` is a hash of its manifest, not the signer pubkey directly. (Single-signer cores still get a manifest; the key is just the hash of a one-signer manifest.)

### Hypercore protocol versions (verified from `UPGRADE.md`)

| Version | Year | Breaking change |
|---|---|---|
| 8 → 9 | 2020 | Signature format changed; Noise handshake changed; v8 and v9 cannot replicate together. |
| 9 → 10 | 2022 | All number encodings switched to little-endian; introduced an "oplog" for atomic local change tracking; Merkle format updated so a single signature covers a batch. |
| 10 → 11 | 2025 | Storage moved to RocksDB via `hypercore-storage` (no more flat-file `.metadata`/`.tree`/`.bitfield`/`.data`); sparse-only (the `sparse` option was removed); `encryptionKey` ctor option deprecated in favor of an `encryption` object; `core.indexedLength` renamed to `core.signedLength`. |

Each cut is wire-incompatible. There is no in-protocol version negotiation; replication just fails. Holepunch's pragmatic answer is "everyone upgrades together" — fine for a single vendor's app surface, painful for an open ecosystem.

### Encryption

A Hypercore can be created with an `encryption: { key: buffer }` block-encryption key. Block contents are encrypted with that key before storage; the public key alone is not enough to read. This is what Keet uses for end-to-end encryption: the room's "invite" carries both the Hypercore key (for verification + replication) and the encryption key (for decryption). v11 generalized this to a pluggable `HypercoreEncryption` interface (`hypercore-encryption` package), so applications can supply their own key-rotation scheme.

The interesting property: the *swarm* (peers gossiping discovery keys) cannot read your blocks even if they happily replicate them. Replication is over discovery key (a hash of the public key); decryption requires the encryption key, which is shared out-of-band.

## Hyperdrive — filesystem on Hypercore

A Hyperdrive looks like a POSIX-ish filesystem (paths, file content, symlinks, executable bit, arbitrary metadata) layered on Hypercore primitives. Internally it is *not* "two raw Hypercores" anymore — that was the v10 shape. v13 splits cleanly:

- **Metadata** is a Hyperbee (`drive.db`) over one Hypercore (`drive.core`). Keys are paths, values are entry records (`{ executable, linkname, blob, metadata }`). The `blob` field is a Hyperblobs locator: `{ blockOffset, blockLength, byteOffset, byteLength }`.
- **Content** is a Hyperblobs instance — a separate Hypercore (`drive.contentKey`) treated as a packed blob store. Files are split into blocks, addressed by byte ranges, and looked up via the locator stored in metadata.

So a `put('/a.txt', buf)` is: append `buf` (chunked) to the Hyperblobs core, then `db.put('/a.txt', { blob: <locator>, ... })` on the Hyperbee, which itself appends a tree-node block to the metadata Hypercore. A `del('/a.txt')` is a Hyperbee delete — which is itself a tombstone append (Hyperbee is also append-only-on-Hypercore; "delete" is a delete-marker block). The blob bytes stay on disk until the application chooses to `clear()` them.

Implication: **a Hyperdrive write is at minimum two append operations across two cores.** Both are signed by the same key (drive.key signs the metadata core; the content core has its own keypair, but the metadata entries embed locators verified through the metadata signature chain). You don't get atomic cross-core appends — if you crash between the blob append and the metadata append, you end up with a blob-core block that no metadata entry references. That's not a correctness problem (the blob is just unreferenced), but it's a fact about the data model.

## Hyperbee — sorted KV B-tree on Hypercore

A Hyperbee stores a B-tree across the blocks of one Hypercore. Every `put` or `del` appends a new tree-node block; nodes reference earlier nodes by block index, so the tree is implicitly versioned. `db.version` is the underlying core length.

Properties worth flagging:

- **Sparse reads compose with the tree structure.** A `get(key)` walks O(log N) blocks; you only download those blocks, plus the Merkle proofs to verify them. A peer holding the public key can read a single key without downloading the whole bee.
- **Range queries** (`createReadStream({ gte, lt })`) walk the tree and stream matching entries.
- **History stream** (`createHistoryStream()`) reads the underlying core in append order — this is the event-sourced view of "every put/del that ever happened".
- **CAS support.** `put(k, v, { cas })` only succeeds if the comparator returns true. Single-writer, so this is just optimistic-concurrency-of-one — but it's the right primitive for "increment-if-equal" patterns.
- **Encoding.** Keys and values pass through pluggable encodings (`utf-8`, `json`, `binary`, or any abstract-encoding). Read/diff streams sort on encoded bytes, not decoded values — non-obvious if you're using JSON keys.

Hyperbee is the workhorse view for almost everything. Hyperdrive uses it. Autobase views are typically Hyperbees. Keet's room state is a Hyperbee. If you're sketching how Myrhiza materializes state, Hyperbee is the closest analog to "indexed key-value materialized view over an event log".

## Autobase — multi-writer linearization

Autobase is the answer to "how do you have N people writing to one logical thing when each Hypercore only allows one writer?" The shape:

- Each writer has their own local Hypercore.
- Each block in a writer's core carries explicit references to "what I'd seen from other writers when I wrote this" — forming a causal DAG across all writer cores.
- A linearizer walks the DAG and produces a deterministic total order. New information can cause already-linearized nodes to be reordered; the framework handles "undo and reapply" against the view.
- The view is a Hyperbee or Hyperdrive (any data structure derivable from a Hypercore via the `apply` callback). The view is itself a Hypercore — once enough indexers sign a `signedLength`, that prefix is frozen and peers behind on the DAG can fast-forward to it without redoing all the apply work.

The README is explicit about determinism:

> Autobase messages may be reordered as new data becomes available. Updates will be undone and reapplied internally. So it is important that the `open` handler returns a data structure only derived from its `store` object argument and that while updating the view in the `apply` function, the `view` argument is the only data structure being update[d] and that its fully deterministic. If any external data structures are used, these updates will not be correctly undone.

That paragraph is the entire correctness contract. It's load-bearing and informally stated.

### Is it a CRDT?

Holepunch does not call Autobase a CRDT. The README calls it a "multiwriter data structure" using the "event sourcing pattern". Mechanically it is CRDT-shaped: commutative, eventually consistent over the DAG of writer events. But it differs from the academic-CRDT line in two respects:

1. **The merge is not commutative at the value level — it's commutative at the linearization level.** Two writers can append "set X = 1" and "set X = 2" and the linearizer picks an order; whichever comes second in the linearization wins. That's last-writer-wins by linearization order, not by semantic merge of the values.
2. **The "undo + reapply" model assumes the view can be rebuilt cheaply.** Pure CRDTs converge without undo. Autobase converges *with* undo, paid for by the determinism of the apply function and the indexer-quorum checkpoint that bounds how far back reordering can reach.

So: CRDT-shaped enough that the comparison is fair (Automerge, Yjs, iroh-docs all live in adjacent territory), but the implementation is closer to a deterministic state-machine-replication scheme over a partial-order log than to a state-based or operation-based CRDT.

### The "view" pattern

The view abstraction is what makes Autobase usable. The `open(store)` callback creates one (or more) Hypercores within the store. The `apply(nodes, view, host)` callback consumes linearized writer nodes and updates the view. The framework guarantees:

- Same writer cores in → same view out (assuming `apply` is deterministic).
- Reordering is handled by reverting to a checkpoint and reapplying.
- `host.addWriter(key, { indexer })` lets the apply function dynamically grow the writer set — i.e. Autobase membership is itself a write to the base.

`signedLength` is the key piece for resilience. Once a quorum of indexers has signed off on a linearization prefix, that prefix is immutable. New peers fast-forward to it. The autoack mechanism (`ackInterval`) periodically appends null nodes that just reference current heads, advancing the signed length without requiring a real write.

### Optimistic appends

v7 added "optimistic" mode: non-writer peers can append blocks tagged optimistic, and the apply function decides per-block whether to call `host.ackWriter(key)` (promoting them) or drop the block. This is the mechanism for "anyone can submit, indexers decide what's accepted" without giving every speculative submitter write capability up front.

## Storage — what's actually on disk

In Hypercore 10 and earlier each core lived as four flat files (`.metadata`, `.tree`, `.bitfield`, `.data`) in a directory, accessed via `random-access-file` or `random-access-storage` adapters. This is the model older docs and most third-party tutorials describe.

In Hypercore 11 storage moved to RocksDB via the `hypercore-storage` package. Multiple cores share one RocksDB instance, keyed by discovery key, with column families for auth/head/sessions/blocks/tree-nodes/bitfield-pages/user-data. The motivation is atomic batches across cores (`createAtom()` lets you stage writes across cores and `flush()` them together) and the operational benefits of RocksDB (compression, snapshots, recovery). Corestore 11 is the layer that binds Hypercore 11 to `hypercore-storage`.

If you read older Hypercore literature ("each core is four files") and then look at a current Pears install, the disk layout will not match. Both are correct; they're different versions.

The Corestore layer adds:

- **Key derivation.** All writable cores in a corestore are derived from a single primary key + a user-provided name. You can recreate the whole tree from one secret.
- **Session reuse.** `store.get({ name })` returns a session on a shared underlying core; resources release when all sessions close.
- **Namespacing.** `store.namespace('app').get({ name: 'main' })` keeps name collisions out of multi-app deployments.
- **All-to-all replication.** `store.replicate(stream)` tries to replicate every loaded core that the remote peer has capability for. Capabilities are not exchanged — both sides must already know the keys.

## Bandwidth and replication profile

Hypercore replication is diff-not-snapshot. The protocol exchanges Merkle proofs of "what I have"; the receiver requests blocks they're missing along with the proofs needed to verify them. For a long-lived log this means the steady-state cost is proportional to new appends, not log size. For a sparse reader (e.g. someone who only fetched blocks 1000 and 2000) the cost of catching up is also bounded by what they actually need.

A consequence Myrhiza authors should internalize: there's no "snapshot" primitive in the data layer. State synchronization happens via "replicate the underlying log + materialize on the receiver side" or "fast-forward to a signed checkpoint". Autobase's `signedLength` is the snapshot equivalent — but it's a snapshot of the *system* core's linearization, not of any particular view. If a new peer wants the materialized view fast, the application has to ship the view's Hypercore separately.

## Native crypto, JS-only runtime

Every package in this stack is JavaScript on Node, Bare, or the Pear runtime. The cryptographic heavy lifting (ed25519, blake2b, chacha20, x25519 for Noise) lives in `sodium-native`, a Node native binding to libsodium. There is no WebAssembly build of the stack. There is no Rust port. There are some abandoned ports (`datrust`, various Hyperdrive ports from the Dat era), all stale by years and targeting a pre-v10 protocol.

This is the single biggest gotcha for Myrhiza adoption: the stack assumes a host with native sodium-native or equivalent libsodium bindings. A WASM Component Model app can't link against it without going through host imports. Pear gets away with this because Pear apps run on Bare, which is Node-shaped and links sodium-native directly. Myrhiza's apps are WASM components in a CM runtime — fundamentally different sandbox.

## Implications for Myrhiza

The data-layer shape is genuinely close to what Myrhiza wants for the `state-apply` event log. Specifically:

- **Append-only, signed-by-author log** is exactly the shape of Myrhiza events keyed by `(peer, instance)`. A Hypercore-style log per instance, with the instance's secret key signing appends, would give us authority verification for free.
- **Sparse, verifiable replication** matches what we want for "newcomer joins a long-running app and only needs to materialize current state, not replay history". Any peer can hand them a checkpoint + Merkle proof; verification doesn't require the whole tail.
- **Manifest-based multi-signer** maps onto our group-authority case (an event accepted by quorum of N indexers). Hypercore 11's manifest shape is very close to what a "signers + quorum + namespace" verification rule looks like in our spec.
- **Autobase's "undo and reapply" determinism contract** is *exactly* the contract Myrhiza state-apply needs. Holepunch has run into the same problem — multi-writer convergence requires a pure function of `(prior view, ordered events)` — and arrived at the same answer informally. Their lessons (the contract is informal, the failure mode of breaking it is silent divergence, the indexer-quorum checkpoint is what makes the cost bounded) are directly applicable.

What doesn't transfer:

- **The whole stack is JS-on-native-libsodium.** No WASM build. No Rust impl current with v11. We can't just "import Hypercore" into a CM runtime; we'd need either a host capability that brokers a native Hypercore (loses determinism for state-apply because the host implementation is opaque to the WASM component) or a clean-room port of the protocol (large undertaking, and v11 is moving — the README is *already* stale on its own version).
- **Single-writer-per-core forces N-cores-per-instance for our identity model.** Myrhiza's `(peer, instance)` identity wants one append surface per writer per app instance. In Hypercore terms that's one core. Multi-writer Myrhiza apps would need an Autobase per app, with one writer-core per `(peer, instance)` participant, plus a system view core, plus per-view cores. The core count scales with participants × view count. Manageable but worth pricing in.
- **Autobase determinism is informal.** The contract lives in a paragraph of the README. There's no static analysis, no runtime guard, no test harness that catches non-deterministic apply functions. Myrhiza's WASM CM substrate could enforce this mechanically — components have no ambient I/O, so an apply component that compiles and runs cannot accidentally read the wall clock — and that's a real advantage over the JS-on-Node implementation.
- **No protocol-version negotiation.** Hypercore upgrades are flag-day. If Myrhiza ships an event-log substrate, version negotiation is something to design in from day one rather than patch around later.
- **Storage is being rewritten as we speak.** Hypercore 11 swapped flat-files for RocksDB. That is a useful reminder that storage is *not* a small detail — and that we should pick our storage abstraction with an upgrade path in mind.

The shortest summary: if Myrhiza had a Rust impl of the Hypercore-11 wire protocol + manifest + Autobase linearizer, we'd be most of the way to our event-log substrate. We don't, and the JS impl can't run inside a WASM CM sandbox, so the right move is "study the design, port the ideas, don't depend on the code".

## Cross-references

- Companion files in this folder: [`pear-runtime.md`](./pear-runtime.md), [`bare-runtime.md`](./bare-runtime.md), [`hyperswarm.md`](./hyperswarm.md), [`keet-and-apps.md`](./keet-and-apps.md), [`governance.md`](./governance.md), [`history.md`](./history.md), [`commercial.md`](./commercial.md), [`comparisons.md`](./comparisons.md), [`critiques.md`](./critiques.md), [`open-problems.md`](./open-problems.md), [`lessons.md`](./lessons.md), [`data-model.md`](./data-model.md).
- Adjacent prior art: [Iroh's iroh-docs](../iroh/) for an in-rust signed-log + view design that *did* get a clean-room implementation; [Holochain's source-chain + DHT](../holochain/) for the per-agent append-only-log + content-addressed-replication shape applied to a different runtime model; [Agoric's transcript-driven replay](../agoric-endo/persistence.md) for the deterministic-replay-of-events-into-state pattern at the VM level.
