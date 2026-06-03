**Date:** 2026-05-29
**Status:** active
**Subject:** LMDB — reference mmap B+tree, shadow paging, single-writer; redb's intellectual ancestor

# LMDB (reference)

The Lightning Memory-Mapped Database. A C library by **Howard Chu (CTO, Symas
Corp; Chief Architect, OpenLDAP)**, originally built as the OpenLDAP backend.
The direct intellectual ancestor of [redb](redb.md) (redb's README: "loosely
inspired by lmdb"). Reference, not a Rust-native candidate (used via the
`heed`/`lmdb-rkv` bindings).

## License (verified)

**OpenLDAP Public License** — a permissive BSD-style license. LMDB ships inside
the OpenLDAP source tree (`libraries/liblmdb`). Permissive enough for any use;
not one of the dual-MIT/Apache Rust crates but unencumbered.

## Architecture — the design redb mirrors

- **Memory-mapped B+tree.** The DB file is `mmap`-ed; reads are pointer chases
  into the mapped region with zero copies and no separate buffer pool — LMDB
  delegates caching to the OS page cache.
- **Two B+trees:** one for user data, one **free-list** tree tracking pages
  freed by deletes/updates for reuse.
- **Copy-on-write / shadow paging.** A write never overwrites a live page; it
  writes new pages and atomically updates a single root pointer in a
  double-buffered meta page. *"Using copy-on-write semantics (with shadow
  paging) provides durability without any need for logging."* The on-disk
  structure is always valid — there is no WAL, no journal, no recovery replay.
  This is exactly redb's god-byte/two-commit-slot model (see
  [crash-consistency.md](crash-consistency.md)).

## Concurrency — single-writer, many-reader, serializable

- **One write transaction at a time** (a single writer lock). No write-write
  races by construction.
- **Unlimited concurrent readers**, each pinned to an MVCC snapshot. "Readers
  never block writers" — a read transaction using older pages can live
  indefinitely.
- Isolation is **serializable** (the strongest), trivially, because there is
  only ever one writer.

The cost of the single-writer model: write throughput is bounded by one writer.
For a P2P kernel where the writer is the single kernel process applying events
in order, this is *not a limitation* — it matches the access pattern. It is the
same model SQLite (WAL), redb, and fjall (non-txn) all use.

The famous footgun: a **long-lived read transaction pins old pages**, so the
free-list cannot reclaim them and the file grows. Reader hygiene (commit/abort
read txns promptly) is mandatory operational discipline.

## On-disk format stability

LMDB's format is **highly stable** — the engine is mature, low-churn, and
explicitly values on-disk stability (it has been the OpenLDAP backend for over a
decade). Format bumps are rare and documented. Second only to SQLite among the
engines here. See [format-stability.md](format-stability.md).

## Implications for Myrhiza

- **LMDB is the proof that the redb design works at production scale and over
  time.** If Myrhiza picks redb, LMDB's decade-plus track record is the evidence
  that copy-on-write-B+tree / single-writer-many-reader / no-WAL is a sound base
  for a durable store. redb is "LMDB's design in safe Rust" — the appeal is
  getting LMDB's properties without `mmap`-in-C's UB hazards.
- The **single-writer model is a feature for Myrhiza, not a constraint** — the
  kernel is the sole writer applying events sequentially; many readers
  (interaction/behavior components reading materialized state, sync providers
  reading the DAG) run concurrently against snapshots.
- **Reader-hygiene discipline transfers to redb** (which shares the
  pin-old-pages behavior): the kernel must not hold read snapshots open across
  long operations, or local storage bloats.
- LMDB itself means a C dependency + the `mmap` operational caveats (it does not
  play well with some network filesystems, and `mmap` makes some failure modes
  harder to reason about) — reasons to prefer the pure-Rust redb that mirrors
  its design.

## Sources

- http://www.lmdb.tech/doc/
- https://en.wikipedia.org/wiki/Lightning_Memory-Mapped_Database
- https://dbdb.io/db/lmdb
- https://www.symas.com/post/getting-down-and-dirty-with-lmdb
- https://github.com/LMDB/lmdb/blob/mdb.master/libraries/liblmdb/COPYRIGHT
