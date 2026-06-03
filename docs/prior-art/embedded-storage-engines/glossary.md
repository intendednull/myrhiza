**Date:** 2026-05-29
**Status:** active
**Subject:** Glossary — embedded-storage-engine terms used in this folder

# Glossary

System-specific terms used across this folder. For Myrhiza terms (state-apply,
DAG, host.kv, profiles) see the master spec.

- **ACID** — Atomicity, Consistency, Isolation, Durability: the transaction
  guarantees. All six engines here claim full ACID (with durability-default
  caveats, see [crash-consistency.md](crash-consistency.md)).

- **B-tree / B+tree** — a balanced, in-place-updatable tree keeping keys sorted
  for log-time point and range lookups. **Read-optimized.** redb, LMDB, SQLite.
  (B+tree = values only in leaves, leaves linked for range scans.)

- **Bw-tree** — a lock-free B-tree variant (Microsoft Research) using a
  page-indirection table so readers/writers proceed without locks. sled's
  lineage.

- **Column family** (RocksDB) / **keyspace** (fjall) — an independently
  configured/compacted LSM-tree within one database, sharing the WAL so
  cross-family/keyspace write batches are atomic.

- **Compaction** — the LSM background process merging SSTables to bound read
  amplification and reclaim space from overwritten/deleted keys. Non-
  deterministic in timing.

- **Copy-on-write (CoW) / shadow paging** — never overwrite a live page; write
  new pages and atomically flip a root pointer at commit. Gives durability
  **without a WAL**; the on-disk structure is always valid. redb, LMDB.

- **fsync** — the syscall forcing OS buffers to physical disk. The difference
  between "consistent" and "power-loss-durable." Several engines skip per-write
  fsync by default.

- **god byte** (redb) — a single bitfield byte in the 512-byte super-header
  controlling the whole DB via three flags: which of two commit slots is primary
  (`primary_bit`), whether recovery is required (`recovery_required`), and
  whether the primary slot was written with 2-phase commit and is provably valid
  (`two_phase_commit`).

- **LSM-tree (log-structured merge-tree)** — write data to an in-memory memtable
  + WAL, flush to immutable SSTables, merge via compaction. **Write-optimized**,
  at the cost of read/space amplification. fjall, RocksDB.

- **memtable** — the in-memory sorted buffer of recent LSM writes, flushed to an
  SSTable when full.

- **MVCC (multi-version concurrency control)** — readers see a consistent
  snapshot while a writer proceeds, by keeping multiple versions of data.
  Enables single-writer-many-reader without read locks.

- **on-disk format stability** — whether/how the byte layout of stored data
  changes across engine versions. The axis Myrhiza weights most: a break forks
  every peer's local state. See [format-stability.md](format-stability.md).

- **rollback journal** (SQLite default) — pre-image journaling: copy original
  pages to a journal before overwriting, so a crash can roll back. Contrast WAL
  (redo) and shadow paging (no log).

- **savepoint** (redb) — a named in-database checkpoint to roll back to.

- **single-writer-many-reader (1W/NR)** — one write transaction at a time,
  unlimited concurrent reads. redb, LMDB, SQLite-WAL, fjall (non-txn). Matches
  Myrhiza's kernel-is-sole-writer model.

- **SSTable (Sorted String Table)** — an immutable, sorted on-disk file of
  key-value pairs; the LSM on-disk unit.

- **torn write** — a page only partially persisted across a crash (e.g. half a
  4 KB page). CoW avoids referencing it; WAL detects it by checksum.

- **two-phase commit (2PC)** — here, redb's local commit protocol: write the
  inactive slot, fsync, then flip the god byte — not the distributed-transaction
  2PC.

- **WAL (write-ahead log) / journal** — append the intended change to a
  sequential log and fsync it before touching the main structure; replay on
  recovery. fjall (journal), RocksDB, SQLite-WAL.

- **WiscKey / key-value separation** — store large values in a separate log,
  keeping keys + pointers in the tree to cut write amplification. fjall's
  `value-log`.

- **write / read / space amplification** — how many times data is rewritten
  during compaction (WAF), how many disk reads per lookup (RAF), and how much
  redundant/overwritten data is stored (SAF). The LSM tuning triangle.

## Sources

- https://github.com/cberner/redb/blob/master/docs/design.md
- https://github.com/facebook/rocksdb/wiki/RocksDB-Overview
- http://www.lmdb.tech/doc/
- https://tool.oschina.net/uploads/apidocs/sqlite/wal.html
- https://raw.githubusercontent.com/fjall-rs/fjall/main/README.md
