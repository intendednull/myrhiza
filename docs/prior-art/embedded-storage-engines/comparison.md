**Date:** 2026-05-29
**Status:** active
**Subject:** Side-by-side comparison of the six engines on the kernel-storage decision axes

# Comparison matrix

The axes a pick-once-commit-hard kernel-storage decision needs, side by side.
Weighted for Myrhiza: **on-disk format stability** is the heaviest axis (a
format break forks every peer's local state), then **embeddability** (pure-Rust,
zero external deps), then **crash-consistency correctness**, then concurrency
fit, then raw performance. Per-engine detail in [redb.md](redb.md),
[fjall.md](fjall.md), [sled.md](sled.md), [sqlite.md](sqlite.md),
[rocksdb.md](rocksdb.md), [lmdb.md](lmdb.md).

## At a glance

| Axis | redb | fjall | sled | SQLite | RocksDB | LMDB |
|---|---|---|---|---|---|---|
| Language | pure Rust | pure Rust | pure Rust | C | C++ | C |
| Data structure | CoW B+tree | LSM-tree | lock-free log / Bw-tree | B-tree | LSM-tree | mmap CoW B+tree |
| Read/write bias | read-opt | write-opt | mixed | read-opt | write-opt | read-opt |
| Concurrency | 1W / NR, MVCC | 1W / NR (+opt txns) | lock-free multi-W | 1W / NR (WAL) | 1W / NR | 1W / NR, MVCC |
| Durability mech. | CoW + god-byte 2PC, no WAL | journal (WAL) | log + flush | rollback-journal / WAL | WAL | shadow paging, no WAL |
| Default durability | crash-safe | OS buffers (not disk) | flush-controlled | NORMAL caveats | OS buffers (not disk) | crash-safe (sync meta) |
| ACID | full | full (txn modes) | optimistic txns | full | full | full, serializable |
| **Format stability** | **stable + upgrade path; broke v1→v2, v2→v3** | **major-bump + migration; young** | **none (pre-1.0, will change)** | **frozen since 2004** | **library reads old; format evolves** | **very stable, rare bumps** |
| Zero external dep | yes | yes (lz4_flex pure-Rust) | yes | C lib / bindings | C++ lib / bindings | C lib + mmap |
| Maintenance pulse | active, 1 maintainer | active, 1 maintainer | rewrite stalled, no release 19mo | extremely active | active (Meta) | mature, low churn |
| In Myrhiza dep tree? | **yes (via iroh-blobs)** | no | no (legacy transitive) | no | no | no |
| License | MIT/Apache-2.0 | MIT/Apache-2.0 | MIT/Apache-2.0 | public domain | Apache-2.0/GPLv2 | OpenLDAP |

(1W/NR = single writer, many readers.)

## How the axes interact

- **B+tree vs LSM is the core read/write tradeoff.** B+trees (redb, LMDB,
  SQLite) give cheap reads and in-place-ish updates with copy-the-path write
  cost. LSM (fjall, RocksDB) gives cheap sequential writes with background
  compaction and read/space amplification. Myrhiza has *both* a read-heavy
  workload (materialized state in `host.kv`, snapshot serving) and a write-heavy
  one (the append-mostly event DAG). One engine must serve both, or the kernel
  uses two — see [open-problems.md](open-problems.md) and
  [lessons.md](lessons.md).

- **Pure-Rust collapses several axes at once.** redb / fjall / sled need no C
  toolchain, cross-compile trivially (incl. to `wasm32` for the browser-native
  path), and have no system-library surface. The C/C++ references (SQLite,
  RocksDB, LMDB) each drag in a build dependency and FFI boundary. This axis
  alone removes the three references from serious *adoption* contention for a
  pure-Rust kernel; they remain reference points.

- **Format stability dominates the long-run cost.** A raw-throughput win is
  recoverable (buy faster disks, optimize later). A format break is not — if the
  kernel's engine changes its on-disk format and a peer can't read its own old
  state, that peer has forked. Detail in [format-stability.md](format-stability.md).

- **"Default durability" is a trap shared by the LSM engines and SQLite.** fjall,
  RocksDB, and SQLite-`synchronous=NORMAL` all default to "consistent but the
  last writes may be lost on power loss." For a P2P node this is a
  convergence-tail problem, not just local data loss. redb and LMDB are
  crash-safe by default. Detail in [crash-consistency.md](crash-consistency.md).

## The realistic shortlist for Myrhiza

Eliminating on the heavy axes: the three C/C++ references fail **pure-Rust
embeddability**; **sled** fails **format stability + shipped release**. That
leaves **redb** and **fjall** as the real candidates, with SQLite/LMDB as the
format-stability yardsticks and RocksDB as the LSM perf yardstick. The
redb-vs-fjall call is in [lessons.md](lessons.md).

## Sources

- https://crates.io/api/v1/crates/redb
- https://crates.io/api/v1/crates/fjall
- https://crates.io/api/v1/crates/sled
- https://raw.githubusercontent.com/cberner/redb/master/README.md
- https://raw.githubusercontent.com/fjall-rs/fjall/main/README.md
- https://www.sqlite.org/formatchng.html
- https://github.com/facebook/rocksdb/wiki/RocksDB-Overview
- http://www.lmdb.tech/doc/
