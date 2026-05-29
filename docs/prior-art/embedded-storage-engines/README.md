**Date:** 2026-05-29
**Status:** active
**Subject:** Embedded storage engines — redb / fjall / sled + SQLite / RocksDB / LMDB, compared for a pick-once kernel-storage decision

# Embedded storage engines

A comparative survey of the embedded storage engines a Myrhiza kernel could
sit on. The kernel owns storage (`CLAUDE.md`: "the kernel owns storage"); no
spec yet picks an engine. This folder is the reference to consult *before* that
pick lands, because it is a **pick-once-commit-hard** decision: a stored
on-disk format is a forward commitment to every peer's local state, and a
format break forks that state.

Two groups:

- **Primary (Rust-embeddable, candidate for direct use):** [redb](redb.md)
  (copy-on-write B+tree), [fjall](fjall.md) (LSM-tree), [sled](sled.md)
  (Bw-tree-ish, stalled).
- **References (mature engines we learn from, mostly C/C++):**
  [SQLite](sqlite.md), [RocksDB](rocksdb.md), [LMDB](lmdb.md). SQLite carries
  the [litestream / cr-sqlite](sqlite.md#context-litestream--cr-sqlite)
  context.

The decision axes are in [comparison.md](comparison.md); the recommendation
matrix is in [lessons.md](lessons.md).

## Key facts

| Engine | Lang | Structure | Concurrency | License | Latest (verified 2026-05-29) | Pulse |
|---|---|---|---|---|---|---|
| redb | pure Rust | copy-on-write B+tree (LMDB-inspired) | MVCC, 1 writer / N readers | MIT OR Apache-2.0 | 4.1.0 (2026-04-19) | active (commit 2026-05-23) |
| fjall | pure Rust | LSM-tree (`lsm-tree` + `value-log`) | MVCC snapshots, 1 writer / N readers, opt. txns | MIT OR Apache-2.0 | 3.1.4 (2026-04-14) | active (commit 2026-05-27) |
| sled | pure Rust | lock-free log + Bw-tree-ish | lock-free, multi-writer | MIT OR Apache-2.0 | **1.0.0-alpha.124** (2024-10-11); last stable **0.34.7** (2021-09-12) | rewrite in progress, no shipped release in ~19 mo |
| SQLite | C | B-tree, rollback-journal or WAL | 1 writer / N readers (WAL) | public domain | (reference) | extremely active, format frozen since 2004 |
| RocksDB | C++ | LSM-tree (LevelDB fork) | 1 writer / N readers, column families | Apache-2.0 OR GPLv2 | (reference) | active (Meta) |
| LMDB | C | mmap dual-B+tree, shadow paging | 1 writer / N readers, MVCC | OpenLDAP Public License | (reference) | mature/stable, low churn |

Format-stability ranking (heaviest Myrhiza weight): **SQLite** (frozen 20+ yr) >
**LMDB** (stable, rare bumps) > **redb** (stable *commitment*, but broke v1→v2
and v2→v3 with migration paths) > **fjall** ("major bump + migration path" on
break) > **RocksDB** (format evolves, library handles it) > **sled** (no stable
format; pre-1.0).

## Reading order

1. [README.md](README.md) — this file.
2. [comparison.md](comparison.md) — the decision axes side by side.
3. [redb.md](redb.md) / [fjall.md](fjall.md) / [sled.md](sled.md) — the three
   Rust candidates in depth.
4. [crash-consistency.md](crash-consistency.md) — WAL vs shadow-paging vs
   god-byte; torn-write and tail-corruption recovery.
5. [format-stability.md](format-stability.md) — the load-bearing axis: who
   commits to what, who has broken it.
6. [sqlite.md](sqlite.md) / [rocksdb.md](rocksdb.md) / [lmdb.md](lmdb.md) — the
   reference engines.
7. [open-problems.md](open-problems.md) — what no engine solves for Myrhiza.
8. [lessons.md](lessons.md) — **the decision file: pick-once recommendation
   matrix.**
9. [glossary.md](glossary.md).

## How to use

Consult before any spec on the **B-9 storage layer** (the gap-analysis build
slice "Storage layer for runtime restart"), the **persistent event DAG** it
durably backs, the **`maintenance.md §12.2` Persister module**, or the
currently-unbacked **`host.kv` per-peer store** (`architecture.md §3.5`).
[lessons.md](lessons.md) keys a recommendation to each of those four surfaces.

## Framing

**Framing disclosure.** These docs are written from Myrhiza's *current design
stance*, not as a neutral catalog of embedded storage engines. That stance is:
a **capability-mediated** kernel (apps reach the host only through declared
imports; storage is a host concern, never directly touchable by app code); a
**P2P-only** runtime with no central server (so a corrupted/lost write is a
cross-peer convergence problem, not just local data loss); apps as **WASM
components on the Component Model + Wasmtime** (and jco in the browser), which
forces the **pure-Rust, cross-compiles-to-`wasm32`** embeddability weighting;
and **event-log-replay `state-apply`** over a per-author Merkle DAG (so on-disk
format stability is weighted above raw throughput — a format break forks every
peer's replayed state). The "Implications for Myrhiza" notes in each file read
the engine through that lens; an engine excellent for a server database may score
poorly here for reasons that would not apply elsewhere.

**Soft-pedal warning.** Storage is a *load-bearing target* for Myrhiza — one of
these engines will likely be adopted — and a corpus written to support a coming
adoption has a built-in incentive to under-state the problems Myrhiza would
*inherit* from its eventual pick (single-maintainer bus factor, the format-break
history, the durability-default trap, young-format risk). Those are surfaced
deliberately in [open-problems.md](open-problems.md) and the "avoid" section of
[lessons.md](lessons.md); read them as the counterweight. **No engine is
committed yet** — the trade matrix in [comparison.md](comparison.md) and
[lessons.md](lessons.md) is presented as a starting argument to audit, not a
settled pick, precisely because of that incentive.

## Glossary stub

Full glossary in [glossary.md](glossary.md). Quick hits: **LSM-tree**
(log-structured merge-tree, write-optimized, append+compact); **B+tree**
(read-optimized in-place tree); **copy-on-write / shadow paging** (never
overwrite a live page; durability without a WAL); **MVCC** (multi-version
concurrency control); **WAL** (write-ahead log); **write/read/space
amplification**; **torn write** (a page partially written across a crash);
**god byte** (redb's single durability-controlling byte).

## Related prior-art

- [`iroh/`](../iroh/) — the transport dep; **iroh-blobs uses redb 4.1.0** for
  its file-backed blob store (`fs-store`). See [redb.md](redb.md).
- [`agoric-endo/persistence.md`](../agoric-endo/persistence.md) — the
  orthogonal-persistence story; explicitly tells the reader to "verify the ACID
  story… pick once, commit hard" and never does. This folder is that
  verification.
- [`schema-evolution/`](../schema-evolution/) — the *application-state* format
  story (Cambria, lenses). This folder is the *engine* format story underneath.
- [`crdts/`](../crdts/) — what materializes *into* `host.kv` / the DAG.

## Sources

- https://crates.io/api/v1/crates/redb
- https://crates.io/api/v1/crates/fjall
- https://crates.io/api/v1/crates/sled
- https://github.com/cberner/redb
- https://github.com/fjall-rs/fjall
- https://github.com/spacejam/sled
- https://raw.githubusercontent.com/n0-computer/iroh-blobs/main/Cargo.toml
