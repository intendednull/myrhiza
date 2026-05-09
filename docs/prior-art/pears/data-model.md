**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — how Hypercore-shaped data interacts with application code (mutation model, consistency, ordering, performance)

# The Hypercore Data Model

This file is the application-facing companion to [`hypercore-stack.md`](./hypercore-stack.md). The other file describes the primitives; this one describes how the primitives constrain (and serve) application code. The audience is a Myrhiza spec author asking "if we adopted this shape for state-apply, what does it actually mean for app developers and for our determinism contract?"

## Append-only as the primary mutation

Every state-changing operation in the Hypercore stack is a block append to some Hypercore. There is no in-place update primitive at any layer:

- A Hypercore append produces a new block; the previous block is immutable. Even `truncate(newLength)` doesn't mutate prior blocks — it bumps a `fork` counter and tells peers to drop the truncated tail.
- A Hyperbee `put(k, v)` appends a new B-tree node block. The old node is still on disk and still referenceable by `db.checkout(version)`. Old keys aren't physically erased — they're just no longer reachable from the tree's current root.
- A Hyperbee `del(k)` appends a *delete-marker block*. The key disappears from `get(k)` but the deletion is itself an event in the underlying core's history.
- A Hyperdrive `put('/path', buf)` is two appends: blob bytes to the content core, then a metadata-Hypercore append (via Hyperbee) recording the locator. `del('/path')` is a Hyperbee delete-marker on the metadata core; the blob bytes stay until explicit `clear()`.
- An Autobase `append(value)` writes to the local writer's own Hypercore with explicit causal references to other writers' heads as known at write time.

This is *event sourcing as the only mutation primitive*. There is no "current state" stored separately from the log — current state is a derived projection. "Update" and "delete" are events with semantic meaning to the projector, not destructive operations on storage.

## Strong eventual consistency via deterministic merge

For a single Hypercore, consistency is trivial: there's one writer, one sequence of appends, one truth. Replicas converge by replicating new blocks and verifying signatures.

For Autobase (multi-writer), the convergence story is **strong eventual consistency conditional on apply determinism**:

> Given the same set of writer Hypercores at the same lengths, every replica that runs the same `apply` function deterministically derives the same view.

The conditionality matters. The Autobase README states the contract once, informally:

> it is important that the `open` handler returns a data structure only derived from its `store` object argument and that while updating the view in the `apply` function, the `view` argument is the only data structure being update[d] and that its fully deterministic.

Violations (reading wall-clock time, hitting a network, depending on map iteration order) cause silent divergence. Two peers process the same writer events and produce different views. There is no runtime check. There is no test harness in the framework that detects this. The contract is the developer's responsibility, full stop.

This is the single most important fact about the Hypercore data model, and it's the place where Myrhiza's WASM Component Model substrate has a genuine advantage over the Node/Bare implementation: a WASM component has no ambient I/O. An `apply` written as a CM component literally cannot read the wall clock or open a socket — those imports either aren't declared or aren't granted. Determinism stops being a developer-discipline contract and becomes a sandbox property.

## Single-writer-per-Hypercore

This is the constraint the rest of the data model is built around. Each Hypercore has exactly one keypair, and only the secret-key holder can append. v11 generalized this slightly via the "manifest": a core can have N signers with quorum M, so multi-signer cores are possible — but in practice the multi-signer path is used for things like the Autobase system core (signed by quorum of indexers), not for "give two users a shared writable log".

For multi-writer state, the canonical answer is N cores + Autobase merging them. Costs:

- Each writer needs their own keypair, their own core, their own `storage`.
- Adding a writer is itself an event written to the base (`apply` calls `host.addWriter(key)` when it sees an add-writer block).
- Removing a writer is similarly modeled as an event, though the writer's prior contributions stay in the history forever (you can't retroactively un-append blocks they signed).
- Causal references in each writer's blocks pin the linearization, so writer cores aren't independent — the DAG ties them together.

For Myrhiza's `(peer, instance)` identity, this maps as: each peer running an instance gets one Hypercore-shaped writer-log per app per peer. An app with K participants has K writer cores plus one system view core plus per-view cores. That's manageable for chat-shaped apps (Keet rooms scale to a few hundred members) but worth pricing in for "every peer co-owns this document" cases — N writers means N cores worth of disk and replication topology.

## On-disk layout

The on-disk layout has changed across protocol versions. Both shapes are still in the wild.

**Hypercore 10 and earlier** (still common in older apps and most third-party tutorials):

```
storage-dir/
  <discovery-key-hex>/
    metadata     ← oplog (signature, length, fork) and core header
    tree         ← Merkle tree nodes
    bitfield     ← which blocks we have
    data         ← block bytes
```

Each core was four flat files, accessed through `random-access-storage` adapters (file, memory, web). One core per directory; multiple cores in a corestore = many directories under the corestore root.

**Hypercore 11** (current latest, what Pears + Keet use today):

Storage moved to RocksDB via the `hypercore-storage` package. The whole corestore lives in one RocksDB instance, with cores keyed by discovery key and the same logical state (auth, head, sessions, blocks, tree nodes, bitfield pages, user data) split across column families. The benefits are:

- Atomic batches across cores via `store.createAtom()` and `atom.flush()`. Important for Autobase, which often needs to update multiple cores (writer cores + view cores + system core) in lockstep.
- RocksDB's compression, snapshots, and recovery story.
- Fewer file handles per core (the v10 model opened many files; the v11 model multiplexes through one RocksDB instance).

The migration is automatic if you point Hypercore 11 at a v10-style storage directory.

Either way, the application-visible surface is the same: `corestore.get({ name })` returns a Hypercore and you don't directly think about files. But the operational shape (backup, sync, container volume) is meaningfully different between v10 and v11.

## Relationship to CRDTs

The Hypercore stack and the CRDT literature describe overlapping problems with non-overlapping vocabulary. Forced into one comparison:

| Property | Autobase | Automerge | Yjs | Iroh-docs |
|---|---|---|---|---|
| Convergence model | Linearize causal DAG, deterministic apply | State-based CRDT | RGA-tree-based CRDT | Signed range-of-events with author-keyed entries |
| Mutation primitive | Append signed block + causal refs | CRDT op | CRDT op | Append signed entry |
| Conflict resolution | Linearization order + apply semantics | CRDT merge function | Per-type merge | Last-writer-wins by author + timestamp |
| Causal model | Explicit DAG references | Vector clock | State vector | Author-keyed sequence numbers |
| Identity per writer | One Hypercore | None inherent — per-actor IDs | Per-client ID | One author key |
| Network primitive | Hypercore replication | Out of scope (any) | Out of scope (any) | Iroh sync protocol |
| Determinism contract | Developer-enforced (apply must be pure) | Built-in (CRDT algebra) | Built-in (CRDT algebra) | Built-in (LWW resolution) |

Autobase is closer to **deterministic state machine replication over a partial-order log** than to **CRDTs in the academic sense**. It gets eventual consistency the same way a Raft or Paxos log does — by all replicas agreeing on a sequence and applying it identically — but with the ordering produced by a linearizer over a causal DAG instead of a leader. The "CRDT-ness" is in the partial-order-merge structure, not in commutative state-merge functions.

For Myrhiza this matters because the determinism contract is the same on both sides: the apply function has to be pure. Autobase is the closer kin in shape, even though Automerge/Yjs are the ones with the marketing.

## Time and ordering

Within a single Hypercore: blocks are totally ordered by their position in the log. Block N causally precedes block N+1. The author's signature commits to length, so reordering within a core is impossible without forking.

Across Hypercores in an Autobase: ordering is the **causal DAG** induced by each writer's references to other writers' heads. The linearizer takes this DAG and produces a total order. There are no vector clocks, no hybrid logical clocks, no timestamps in the ordering primitive — just direct block-index references between cores.

This has consequences:

- **No real time.** Nothing in the data model knows or cares what wall-clock time it is. If your app needs "ordered by time" you have to put a timestamp in the block payload and accept that it's writer-supplied (and adversarial-writer-controllable). The linearizer ignores it.
- **"Concurrent" has a precise meaning.** Two blocks are concurrent iff neither's writer had seen the other when writing. The linearizer breaks ties deterministically (by writer key or by some stable tiebreaker — exact algorithm in the autobase source).
- **Reordering is bounded by `signedLength`.** Once a quorum of indexers signs a linearization prefix, that prefix is frozen and reordering can never reach into it. Indexer ack frequency tunes how recent the frozen-prefix is. Autobase's `ackInterval` defaults to 1 second; the autoacker appends null nodes that just reference current heads, so the signed length advances even when nobody's writing real content.

This is the right shape for a P2P event log. It's strictly more general than vector clocks (any vector-clock scheme can be modeled as DAG references) and it doesn't require synchronized clocks.

## Performance characteristics

Rough operating numbers, useful for spec authors making "is this primitive feasible for our use case?" calls.

**Hypercore append.** O(log N) Merkle tree update + one signature + one storage write. In the v11 RocksDB-backed storage, append latency is dominated by the RocksDB write; in v10's flat-file storage it's dominated by the four-file fsync. Either way, well under a millisecond on local SSD for small blocks. Throughput in the thousands of appends/sec on a modern laptop.

**Hypercore get (sequential).** O(1) — block bytes are stored by index. The bitfield says "do I have this?", the tree gives the verification path, the data file/RocksDB column gives the bytes.

**Hypercore get (random, sparse).** O(log N) — if you don't have the block, you request it from peers along with a Merkle proof of size O(log N). Network-bound.

**Hyperbee get.** O(log N) tree walk. Sparse-friendly: only the blocks on the walk path get fetched.

**Hyperbee put.** O(log N) tree walk + 1 append per node touched (B-tree update appends a few new nodes, not just one). For large bees, expect 3-5 appends per `put`.

**Hyperdrive put.** Blob append (chunked, one append per chunk on the content core) + Hyperbee put on the metadata core (3-5 appends). For a 1MB file with ~64KB chunks: ~16 content appends + ~5 metadata appends = ~21 storage writes. The atom primitive in v11 batches these.

**Autobase apply.** O(K) per linearizer step where K is the number of newly-linearized nodes. Reordering does an O(R) revert of R already-applied nodes followed by re-apply. For a healthy autobase with regular indexer acks, R is small (bounded by the gap between current head and signed length). For a partitioned autobase that reunites after a long split, R can be large.

**Replication bandwidth.** Diff-not-snapshot, scales with appends since the peer was last seen, plus the Merkle proofs (logarithmic overhead). For a long-lived log this is the property that matters: the steady-state cost is proportional to *new writes*, not total log size. Sparse readers pay only for what they actually fetch. For a million-block log where you read 100 blocks, you pay for ~100 blocks plus ~100 × log₂(1M) ≈ 2000 tree-node hashes for verification.

These are the right numbers for thinking about "is this fast enough for X". Chat (Keet): yes, easily. Real-time collaborative cursor positions across 100 peers: stretching. High-frequency game state: no, the model is event-grained, not tick-grained.

## Implications for Myrhiza

The shape that transfers:

- **Event-as-only-mutation** is what Myrhiza already wants. State-apply takes an event and produces new state; there is no in-place mutation. Hypercore validates that this shape scales to real apps (Keet runs on it; Pear runtime self-hosts updates on it).
- **Single-writer-per-log + multi-writer-via-merge** maps onto Myrhiza's `(peer, instance)` identity. Each instance writes its own log; cross-instance ordering is a separate problem (Autobase-shaped) at the application layer.
- **Determinism-as-developer-contract** is exactly Myrhiza's state-apply requirement. Hypercore has 5+ years of experience with developers getting this wrong, and the failure mode (silent cross-peer divergence) is well-documented in the Holepunch issue trackers. Forecasting: Myrhiza will hit the same class of bugs, and our WASM CM sandbox is the most plausible mitigation.
- **Sparse + verifiable replication + signed checkpoints** is the right shape for newcomer-onboarding and for resource-constrained peers. The newcomer doesn't replay history; they fetch the signed prefix + the proof and start from there.

The shape that does not transfer cleanly:

- **JS-only data layer.** Apps in Myrhiza are WASM components. A Hypercore-shaped log that lived inside the kernel and was exposed to apps via host imports (capability-mediated) could work — but at that point Myrhiza is reimplementing the protocol, not adopting it. The implementation cost is the implementation cost; the design cost is the smaller part.
- **`apply` as a JS callback.** In Myrhiza, `state-apply` is a WASM component. The contract becomes structural (the component's imports do or don't include nondeterministic capabilities) rather than promissory (a paragraph in a README). This is genuinely better, but it means the Autobase API doesn't translate one-to-one.
- **The "no time" property has consequences.** Hypercore apps that need "ordered by user-perceived time" hack around it (timestamps in payloads, indexer-driven sequence numbers). Myrhiza apps will hit the same problem and we should design our event-log abstraction with this in mind — explicit "kernel-supplied monotone tick" capability, distinct from wall clock, exposed to state-apply only in narrow ways that don't break determinism.
- **Hypercore's storage rewrite-mid-stream is a warning.** v10 and v11 have different on-disk layouts. Holepunch could afford the migration because they own the whole stack and the userbase. Myrhiza should pick a storage abstraction that survives a rewrite, or accept that a future migration is in our roadmap and design events-on-disk to be the migration interchange format.

## Cross-references

- Companion files in this folder: [`hypercore-stack.md`](./hypercore-stack.md), [`pear-runtime.md`](./pear-runtime.md), [`bare-runtime.md`](./bare-runtime.md), [`hyperswarm.md`](./hyperswarm.md), [`keet-and-apps.md`](./keet-and-apps.md), [`governance.md`](./governance.md), [`history.md`](./history.md), [`commercial.md`](./commercial.md), [`comparisons.md`](./comparisons.md), [`critiques.md`](./critiques.md), [`open-problems.md`](./open-problems.md), [`lessons.md`](./lessons.md).
- Adjacent prior art on the determinism / event-replay axis: [Agoric's transcript-driven replay](../agoric-endo/persistence.md) (vat snapshots + transcript replay, same shape one level down at the VM); [Holochain's source-chain](../holochain/) (per-agent append-only log + DHT-replicated entries, same single-writer-per-log shape with different network model).
- Adjacent prior art on the multi-writer-merge axis: [Iroh's iroh-docs](../iroh/) (signed entries with LWW, Rust impl, no Autobase-style linearization but a production multi-writer doc API).
