**Date:** 2026-05-29
**Status:** active
**Subject:** Crash-consistency models — WAL vs shadow-paging vs god-byte; torn writes, tail corruption, durability defaults

# Crash consistency

How each engine survives a crash or power loss mid-write, and what it does and
doesn't guarantee. This matters doubly for Myrhiza: a peer that corrupts or
silently loses tail events is not just a local-data-loss problem, it is a
**convergence problem** — its DAG head diverges from what it broadcast.

## Two families of durability mechanism

### 1. Write-ahead log (WAL / journal) — fjall, RocksDB, SQLite

Append the intended change to a sequential log and fsync *that* before touching
the main data structure. On crash, replay (redo) the committed log records the
main structure hadn't absorbed yet. Cheap sequential writes; recovery is a log
scan. The classic LSM/relational approach.

- **fjall:** a journal; replayed on open. Default `PersistMode` flushes to OS
  buffers, *not* disk.
- **RocksDB:** WAL shared across column families (so cross-CF batches are
  atomic); default `WriteOptions` doesn't fsync per write.
- **SQLite:** rollback journal (default) or WAL mode (since 3.7.0). WAL uses
  far fewer fsyncs and lets readers and writers run concurrently.

### 2. Copy-on-write / shadow paging — redb, LMDB

Never overwrite a live page. Write new pages, then atomically flip one pointer
to make them the new root. The on-disk structure is **always valid** — there is
no torn intermediate state to recover, hence **no WAL needed**. The tradeoff is
write amplification (copy the path to root each commit) instead of log replay.

- **LMDB:** double-buffered meta page; the root-pointer flip is the atomic
  commit point. *"Shadow paging provides durability without any need for
  logging."*
- **redb:** the same idea with an explicit, well-documented mechanism (below).

## redb's god-byte two-phase commit (worth understanding)

redb's super-header is **512 bytes**, holding immutable fields (page size,
region size) plus **two "commit slots"** (double-buffered transaction pointers).
A single **"god byte"** — a bitfield of **three flags** — controls the whole
database (per redb's `docs/design.md`):

- a **`primary_bit`** selecting which commit slot (0 or 1) holds the latest
  commit,
- a **`recovery_required`** flag indicating recovery must run on open (a full
  repair walking the B+tree, or a quick-repair loading allocator state from a
  table), and
- a **`two_phase_commit`** flag indicating the primary slot was written with
  2-phase commit and is therefore provably valid — when set, repair need not
  look at the secondary slot.

Commit writes the new tree + the inactive slot, fsyncs, then flips the
`primary_bit`. A crash mid-commit leaves the *old* slot intact, so on reopen the
DB returns to either the last full commit or the last non-durable commit —
always a consistent state. With **quick-repair** enabled, redb saves allocator
state per commit and runs full 2-phase commit (the `two_phase_commit` flag), so
the primary slot is provably valid without scanning checksums.

Honest caveat from redb's own design notes: even with 2-phase commit, an
attacker with enough control to crash the process at will can leave the god byte
pointing at an invalid slot — a threat-model corner, not a normal-operation
risk.

## Torn writes and tail corruption

A **torn write** = a page partially persisted across a crash (e.g. 2 KB of a 4
KB page made it to disk). The two families handle it differently:

- **CoW (redb/LMDB):** a torn *new* page is never referenced because the root
  pointer hadn't flipped yet — the old tree is intact. Robust by construction.
  The risk concentrates in the single super-header / meta-page write (mitigated
  by double-buffering + checksums).
- **WAL (fjall/RocksDB/SQLite):** a torn WAL *tail* record is detected by
  checksum and discarded on replay — you lose the torn (uncommitted) tail but
  keep everything before it. A torn write to the *main* data file is the danger
  the WAL exists to prevent (redo from the log). SQLite's `synchronous=FULL`
  exists to guarantee the main file is never corrupted this way.

## The durability-default trap (shared, and load-bearing for Myrhiza)

Several engines default to **"consistent but not power-loss-durable for the most
recent writes":**

- fjall — OS buffers, not disk, by default.
- RocksDB — WAL not fsynced per write by default.
- SQLite `synchronous=NORMAL` (WAL) — "might roll back the last transaction
  following a power loss." *"FULL ensures the database isn't corrupted, but NOT
  that the last transaction is durable."*

redb and LMDB are crash-*safe* by default (the structure survives), and redb's
non-durable-commit mode is opt-in.

**For Myrhiza:** the Persister (`maintenance.md §12.2`) must pick its durability
setting deliberately. A node that fsyncs only periodically is faster but can
broadcast an event, then lose it locally on power loss — a self-inflicted fork
of its own author chain (the [persistent event DAG](../../specs/2026-05-09-myrhiza-master-design/architecture.md)).
The safe default for the DAG/event store is **fsync-on-commit before
broadcast** (don't announce a head you can't prove you durably hold). The
per-peer `host.kv` store (local-only, no convergence stake) can use the cheaper
mode. See [lessons.md](lessons.md).

## Sources

- https://github.com/cberner/redb/blob/master/docs/design.md
- http://www.lmdb.tech/doc/
- https://tool.oschina.net/uploads/apidocs/sqlite/wal.html
- https://www.agwa.name/blog/post/sqlite_durability
- https://avi.im/blag/2025/sqlite-fsync/
- https://github.com/facebook/rocksdb/wiki/RocksDB-Overview
- https://raw.githubusercontent.com/fjall-rs/fjall/main/README.md
