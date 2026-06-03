**Date:** 2026-05-29
**Status:** active
**Subject:** RocksDB — reference LSM engine; the production benchmark fjall is measured against

# RocksDB (reference)

An embeddable, persistent C++ key-value store from Meta (Facebook). The
production-hardened LSM engine — the thing fjall's docs explicitly compare
themselves to ("matches RocksDB's default durability"), and the engine sled's
own README routes space-conscious users toward. Reference, not a Rust-native
candidate.

## History + license (verified)

- Forked from **Google's LevelDB at Facebook in April 2012** (by Dhruba
  Borthakur), then extended by the Facebook/Meta Database Engineering Team for
  server workloads (point lookups, range scans, high write throughput). (LevelDB
  itself was released by Google in 2011 — a common source of date confusion.)
  Powers MyRocks (MySQL storage engine for Facebook's social graph) and countless
  streaming/state-store systems (Kafka Streams, Flink, etc.).
- **License: Apache-2.0 OR GPLv2.** Originally BSD-3-clause, then BSD+Patents;
  **relicensed to Apache-2.0 (dual with GPLv2) in July 2017** after the ASF
  ruled the Facebook BSD+Patents clause incompatible with Apache projects (the
  same kerfuffle that hit React). For Myrhiza, dual Apache/GPL is acceptable;
  the BSD+Patents era is history.

## Architecture

LSM-tree (same family as [fjall](fjall.md)):

- Writes → in-memory **memtable** + a **WAL** for durability.
- Memtable flushes to immutable **SSTables** (Sorted String Tables).
- Background **compaction** merges SSTables. RocksDB exposes the compaction
  strategy as a tuning knob: **Leveled** (Tiered+Leveled in code), **Universal**
  (tiered), and **FIFO**. Each picks a different point in the
  write/read/space-amplification triangle.
- **Column families** — each is its own LSM-tree (own memtable, compaction,
  layout) but they **share one WAL** so write batches across column families are
  atomic. (fjall's keyspaces are the same idea.)

The cost of RocksDB's power is **configuration surface**: dozens of tunables
(block cache, bloom filters, compaction style, write buffer sizes) that must be
matched to the workload. This is a real downside for an embed-and-forget kernel
store.

## Durability + crash consistency

WAL-based: on crash, replay the WAL to recover committed-but-not-yet-flushed
writes. The default `WriteOptions` does *not* fsync the WAL on every write
(writes reach OS buffers, not disk) — the same "consistent but tail-not-durable
by default" posture as fjall and SQLite `synchronous=NORMAL`. See
[crash-consistency.md](crash-consistency.md).

## On-disk format stability

RocksDB's SST format **evolves**, but the library is engineered to read older
format versions (format-version compatibility is a first-class concern for an
engine that must roll out across huge fleets without downtime). The contract is
"the current library reads old data," not "the format is frozen." That is more
generous than sled and roughly comparable to redb's "stable with upgrade path,"
but the format is a moving target managed by the library rather than a frozen
spec like SQLite. See [format-stability.md](format-stability.md).

## Implications for Myrhiza

- **The realistic LSM benchmark.** If Myrhiza considers an LSM engine for the
  append-heavy DAG, RocksDB is the throughput/space-amplification reference to
  bench fjall against — but adopting RocksDB itself means a C++ dependency
  (`librocksdb-sys`, a `cc`/system-lib build), which cuts hard against Myrhiza's
  pure-Rust embeddability axis ([comparison.md](comparison.md)).
- **Configuration burden is a liability, not a feature** for a kernel that wants
  one sane durable setting. fjall's smaller knob surface is arguably a better
  fit even though RocksDB is more proven.
- The **column-family / shared-WAL atomicity model** is the design to borrow
  conceptually for per-topic partitioning with cross-topic atomic event batches
  ([lessons.md](lessons.md), "borrow"). fjall already mirrors it.

## Sources

- https://github.com/facebook/rocksdb/wiki/RocksDB-Overview
- https://github.com/facebook/rocksdb/wiki/Compaction
- https://en.wikipedia.org/wiki/RocksDB
- https://news.ycombinator.com/item?id=14779509
- https://www.vldb.org/pvldb/vol13/p3217-matsunobu.pdf
- https://www.cidrdb.org/cidr2017/papers/p82-dong-cidr17.pdf
