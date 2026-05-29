**Date:** 2026-05-29
**Status:** active
**Subject:** fjall — pure-Rust LSM-tree embeddable key-value storage engine

# fjall

"A log-structured, embeddable key-value storage engine written in Rust." LSM
(log-structured merge) family — the write-optimized counterpart to redb's
read-optimized B+tree. Maintained under the `fjall-rs` org by Marvin Knewtson
(`marvin-j97`). Repo: `github.com/fjall-rs/fjall`.

**Verified facts (2026-05-29):** latest crates.io version **3.1.4**, published
**2026-04-14**. License **MIT OR Apache-2.0** (both license files in-repo;
Cargo.toml `license = "MIT OR Apache-2.0"`). ~1.0M all-time downloads, 57
reverse dependencies, ~2K GitHub stars, last commit 2026-05-27 — actively and
rapidly developed (frequent point releases through 2026). First published
2023-12-21; younger than redb.

## Architecture — a layered family

fjall is the top layer of a three-crate stack, all under `fjall-rs`:

- **`lsm-tree`** (max 3.1.4) — the core single LSM-tree: memtable, SSTables,
  compaction, MVCC snapshots, block cache.
- **`value-log`** (max 1.9.0) — optional key-value separation (WiscKey-style):
  large values stored in a separate log, keys + pointers in the tree. For
  "large blob use cases."
- **`fjall`** — the database/transaction layer: keyspaces, journal, persist
  modes, optional transactions.

"Each keyspace is its own physical LSM-tree, and thus isolated from other
keyspaces." (Keyspaces are roughly RocksDB column families;
see [rocksdb.md](rocksdb.md).) fjall supports **cross-keyspace atomic
semantics** — a write batch can span keyspaces atomically.

## LSM mechanics (the tradeoff to understand)

Writes go to an in-memory memtable + a journal (the WAL). When the memtable
fills it is flushed to an immutable on-disk SSTable. Background **compaction**
merges SSTables to bound read amplification and reclaim space from
overwritten/deleted keys. This makes writes cheap and sequential (good for an
append-heavy event log / DAG) at the cost of:

- **Write amplification** — data is rewritten during compaction.
- **Read amplification** — a read may touch several SSTable levels.
- **Space amplification** — overwritten data lingers until compaction.
- **Compaction is a background, non-deterministic process** — it runs on its
  own schedule and consumes I/O. (Relevant to Myrhiza only as host-side
  behavior; it never touches `state-apply` determinism — see
  [open-problems.md](open-problems.md).)

## Concurrency

"Internally synchronized for multi-threaded access" — clone the `Database` /
`Keyspace` handles freely, no external locking. The backing store is an MVCC
key-value store with "repeatable snapshot reads." Transactions come in two
flavors: `SingleWriterTxDatabase` (one writer) and `OptimisticTxDatabase`
(multi-writer with optimistic concurrency control + conflict detection at
commit). The plain non-transactional `Keyspace` is single-writer-many-reader
like the others here.

## Durability / crash consistency

fjall uses a **journal** (WAL). On a clean drop it tries to persist the journal
synchronously. Durability is tunable via `Database::persist(PersistMode)`:

> By default, any operation will flush to OS buffers, but **not** to disk. This
> matches RocksDB's default durability.

So the default is *crash-consistent but not power-loss-durable for the last
writes* — exactly the SQLite `synchronous=NORMAL` shape (see
[crash-consistency.md](crash-consistency.md)). The maintainer's guidance on
transient I/O errors: "let the application crash and restart" rather than try to
continue. Recovery on open replays the journal.

## On-disk format stability

README commitment: *"Future breaking changes will result in a major version
bump and a migration path."* Same posture as redb, but fjall is younger (3.x,
first published end-2023) and its format is correspondingly less battle-tested.
No frozen-format guarantee. See [format-stability.md](format-stability.md).

## Embeddability

"100% safe & stable Rust." The only non-Rust-flavored dep is `lz4_flex` (LZ4 in
Rust) for compression, pulled in by the `lz4` feature which is **enabled by
default** (so LZ4 compression is on out of the box), plus `bytes`/`byteview` for
its `Slice` type — all Rust. No C toolchain. Cross-compiles like any pure-Rust
crate.

## Implications for Myrhiza

- **Write-optimized fit for the append-mostly DAG.** The [persistent event
  DAG](../../specs/2026-05-09-myrhiza-master-design/architecture.md) is
  append-dominant; LSM sequential-write economics suit it better than redb's
  copy-the-path-to-root B+tree. Worth a head-to-head bench on the actual event
  workload before deciding.
- **Keyspaces map cleanly onto Myrhiza's multi-topic / per-app partitioning**
  — one keyspace per topic or per app, isolated, with cross-keyspace atomic
  batches for events that touch multiple topics.
- **Younger + single-maintainer.** Fewer reverse deps (57) and a shorter track
  record than redb (281) or SQLite (decades). The format-stability axis — which
  Myrhiza weights heavily — favors the more-proven engine here.
- The "OS buffers, not disk" default durability must be made explicit in the
  kernel's Persister config; a P2P node that loses its last N events on power
  loss is a convergence-tail problem, not just a local one.

## Sources

- https://crates.io/api/v1/crates/fjall
- https://crates.io/api/v1/crates/lsm-tree
- https://crates.io/api/v1/crates/value-log
- https://raw.githubusercontent.com/fjall-rs/fjall/main/README.md
- https://github.com/fjall-rs/fjall
