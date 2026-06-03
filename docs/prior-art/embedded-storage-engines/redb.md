**Date:** 2026-05-29
**Status:** active
**Subject:** redb — pure-Rust copy-on-write B+tree embedded key-value store

# redb

A pure-Rust embedded key-value store, "simple, portable, high-performance,
ACID." Single-file, self-described as "loosely inspired by lmdb." Maintained
primarily by Christopher Berner (`cberner`). Repo: `github.com/cberner/redb`.

**Verified facts (2026-05-29):** latest crates.io version **4.1.0**, published
**2026-04-19**. License **MIT OR Apache-2.0** (both `LICENSE-MIT` and
`LICENSE-APACHE` present in-repo; Cargo.toml `license = "MIT OR Apache-2.0"`).
~5.56M all-time downloads, 281 reverse dependencies, ~4.5K GitHub stars, last
commit 2026-05-23 — actively maintained. The crate `created_at` of 2018 is the
name-registration date; the project's substantive history starts ~2021 and 1.0
shipped 2023-06-16.

## Data structure

A collection of **copy-on-write B+trees**. Each table is its own B+tree; a
top-level "table tree" maps table names to root pointers. Copy-on-write means a
write never mutates a live page in place — it writes new pages and atomically
flips a pointer at commit. This is the LMDB shadow-paging idea (see
[lmdb.md](lmdb.md)) re-implemented in safe Rust.

Consequence: **read-optimized.** Point reads and range scans are direct tree
descents with no compaction overhead. Write amplification comes from copying
the path-to-root on each commit, not from background compaction. This is the
opposite tradeoff from an LSM engine like [fjall](fjall.md) or
[rocksdb](rocksdb.md).

## Concurrency

"MVCC support for concurrent readers & writer, without blocking." The model is
**single writer, many readers** — one `WriteTransaction` at a time, unlimited
concurrent `ReadTransaction`s, each seeing a consistent snapshot. Readers never
block the writer and the writer never blocks readers, because copy-on-write
keeps old pages alive for in-flight readers. The API is a zero-copy,
thread-safe, `BTreeMap`-shaped surface. `ReadOnlyDatabase` (added in 3.0)
allows multi-*process* read access.

## ACID + crash consistency

Fully ACID, "crash-safe by default." No WAL: durability comes from
copy-on-write plus a two-phase commit governed by a single "god byte" in the
512-byte super-header. Details in [crash-consistency.md](crash-consistency.md).
Supports **savepoints and rollbacks** (named in-database checkpoints).

## On-disk format stability — the load-bearing fact

redb's README states: *"The file format is stable, and a reasonable effort will
be made to provide an upgrade path if there are any future changes to it."*

That commitment is real but it is **not "frozen-forever" like SQLite.** redb
has broken its on-disk format twice across majors, each with a migration path:

- **2.0.0** (a new format optimizing `len()` to constant time): *"not backwards
  compatible with 1.x."*
- **3.0.0** (2025-08-09): *"Removes support for file format v2 … Use
  `Database::upgrade()`, in redb 2.6, to migrate to the v3 file format."* The
  v3 format dropped the minimum DB size from ~2.5 MiB to ~50 KiB. Note the
  **staged migration**: you migrate v2→v3 *while on 2.6*, then upgrade the
  library to 3.x — you cannot jump straight from an old 2.x to 4.x against an
  un-migrated file.
- **4.0.0** (2026-04-02) was *not* a format break — it fixed a critical
  data-loss bug in `AccessGuardMut` accessors outliving their tables, and
  removed the `Legacy` type.

So redb's pre-1.0-style format churn ended at 1.0 (2023), but the format is on
its **third revision (v3, since 3.0.0)** with explicit, library-supported
migration. For Myrhiza this is the central caveat: see
[format-stability.md](format-stability.md) and [lessons.md](lessons.md).

## Notable 4.1.0 detail

The 4.1.0 changelog says the release *"contains a large number of bug fixes
discovered by AI coding agents"* — fixes to `restore_savepoint()` and table
operations that could cause corruption, plus ~15% read / ~1.5x write perf. An
honest reading: a single-maintainer engine is still finding
corruption-class bugs in 2026, and is leaning on AI fuzzing to find them. Not
disqualifying, but a maturity signal to weigh against SQLite/LMDB's decades.

## Implications for Myrhiza

- **Already in the dependency tree.** `iroh-blobs` depends on `redb = "4.1.0"`
  for its `fs-store` file-backed blob store (verified in iroh-blobs Cargo.toml).
  Picking redb for the kernel means one fewer storage engine in the binary and
  alignment with the transport dep. See [`iroh/`](../iroh/).
- **Pure Rust, zero C deps** — satisfies the embeddability axis cleanly
  (no `cc`, no system library, cross-compiles trivially, one `Cargo.toml` line).
- The read-optimized B+tree suits a **read-heavy materialized-state store**
  (the `host.kv` per-peer store, snapshot cache) better than a write-heavy
  append log. For the [persistent event DAG](../../specs/2026-05-09-myrhiza-master-design/architecture.md)
  (append-mostly), the write-amplification of copying path-to-root on every
  event commit is worth measuring against an LSM engine.
- The format-break history is the thing to design around: pin the redb major,
  treat any redb-major bump as a kernel-format migration event, and write the
  migration into the kernel rather than trusting `Database::upgrade()` to be
  invisible.

## Sources

- https://crates.io/api/v1/crates/redb
- https://raw.githubusercontent.com/cberner/redb/master/README.md
- https://raw.githubusercontent.com/cberner/redb/master/CHANGELOG.md
- https://github.com/cberner/redb/blob/master/docs/design.md
- https://www.redb.org/post/2023/06/16/1-0-stable-release/
- https://raw.githubusercontent.com/n0-computer/iroh-blobs/main/Cargo.toml
