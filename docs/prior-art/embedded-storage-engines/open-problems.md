**Date:** 2026-05-29
**Status:** active
**Subject:** What no embedded engine solves — the gaps Myrhiza must own above the storage layer

# Open problems

What an embedded storage engine structurally does *not* give Myrhiza. These are
the kernel's problems no matter which engine is picked — useful so a future spec
doesn't expect the engine to solve them.

## 1. The engine stores bytes; convergence is not its job

None of these engines knows what Myrhiza state *means*. They give durable,
crash-consistent, transactional byte storage. They do **not** give:

- deterministic state materialization (that is `state-apply`, by design — see
  [`determinism.md`](../../specs/2026-05-09-myrhiza-master-design/determinism.md));
- cross-peer convergence (that is the Merkle DAG + `state-apply`, not the store);
- conflict resolution (cr-sqlite bolts CRDTs *into* SQLite — the
  rejected-runner-up paradigm; Myrhiza keeps convergence in `state-apply`, see
  [sqlite.md](sqlite.md) and [`crdts/`](../crdts/)).

So the engine choice is *below* the interesting layer. It must not introduce
non-determinism that leaks upward — which it won't, because storage is a host
concern the deterministic profiles never touch directly.

## 2. B+tree-vs-LSM mismatch: one workload or two stores?

Myrhiza has two storage shapes (see [comparison.md](comparison.md)):

- **Append-mostly event DAG** (persisted by the B-9 storage layer) —
  write-heavy, sequential, rarely mutated → favors LSM (fjall/RocksDB).
- **Materialized state + per-peer `host.kv` + snapshot cache** — read-heavy,
  point/range lookups, frequently overwritten → favors B+tree (redb/LMDB).

No single engine is optimal for both. Open question for the B-9 spec: one engine
serving both (accept the suboptimal half), or two engines (e.g. fjall for the
DAG, redb for kv/snapshots — at the cost of two formats to keep stable, two
crash-consistency stories, larger binary)? The two-engine path multiplies the
format-stability risk, the axis Myrhiza weights most. Default lean: one engine,
measured on the real workload, until a bench proves the split worth its cost.

## 3. The kernel's *own* on-disk schema can break even if the engine's doesn't

A frozen engine format (even SQLite's) does not freeze *how Myrhiza lays out its
data inside it* — key encodings, value framing, index layout for events/heads/
snapshots/kv. That second format is Myrhiza's to version and migrate
([`schema-evolution/`](../schema-evolution/)). The engine solves byte
durability; it does not solve "the kernel changed how it keys events." See
[format-stability.md](format-stability.md) §Implications point 3.

## 4. Encryption at rest is not provided (uniformly)

These engines store plaintext bytes by default. Myrhiza's `host.seal`/`host.open`
operate on payloads *before* storage, so sealed content is stored as ciphertext
— but keys, indexes, the DAG structure, and `host.kv` plaintext are not
engine-encrypted. If at-rest encryption of the whole store is a requirement
(stolen-device threat model), it is a kernel responsibility (encrypt before put,
or an encrypting page layer), not an engine feature to expect.

## 5. Size/quota/GC against live roots is the kernel's problem

The engines reclaim space they internally free (LSM compaction; LMDB/redb
free-lists), but they do not know which *Myrhiza* data is still live. Pruning the
DAG, evicting old snapshots, GC-ing `host.kv`, and log truncation against live
heads are kernel policy — and overlap with the
[`content-addressed-blockstore`](../../reports/2026-05-29-prior-art-gap-analysis.md)
gap (mark-and-sweep against live roots). The engine is the substrate; retention
is above it (`convergence.md §4.2`, `risks.md §17`).

## 6. WASM / browser-native storage is a separate target

The browser-native path (jco, `wasm32`) has no POSIX filesystem; redb/fjall/sled
assume file I/O. Persisting in the browser means OPFS / IndexedDB, not a
file-backed engine. A pure-Rust engine *compiles* to `wasm32`, but it needs a
storage backend that exists in the browser — an open integration question, not
something any of these engines solves out of the box. See
[`jco/`](../jco/) and the spec's
[`browser-native.md`](../../specs/2026-05-09-myrhiza-master-design/browser-native.md).

## 7. Single-maintainer / maturity risk for the pure-Rust candidates

redb and fjall are each effectively one-maintainer projects (verified: redb
`cberner`, fjall `marvin-j97` dominate commits). redb is still finding
corruption-class bugs in 2026 (the 4.1.0 "AI-agent-discovered fixes"). This is
not disqualifying — they are widely used and actively fixed — but it is a real
bus-factor and maturity gap versus SQLite/RocksDB/LMDB's institutional backing
and decades. The mitigation is the same as the format mitigation: pin, own the
migration, keep an exit path (the data is a B-tree/LSM of bytes; a documented
escape hatch to another engine is feasible if a project stalls — as sled did).

## Sources

- https://raw.githubusercontent.com/cberner/redb/master/CHANGELOG.md
- https://raw.githubusercontent.com/spacejam/sled/main/README.md
- https://github.com/vlcn-io/cr-sqlite
- https://crates.io/api/v1/crates/redb
- https://crates.io/api/v1/crates/fjall
