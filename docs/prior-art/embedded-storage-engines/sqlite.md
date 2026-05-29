**Date:** 2026-05-29
**Status:** active
**Subject:** SQLite — the reference for format stability; B-tree, rollback-journal/WAL, public domain

# SQLite (reference)

Not a Rust-native candidate (it is C, embedded via the `rusqlite`/`libsqlite3-sys`
bindings or the pure-Rust reimplementation efforts), but the **gold standard
reference** on the one axis Myrhiza weights most: on-disk format stability. The
most-deployed database in the world; public domain.

## The format-stability benchmark

SQLite is the bar every other engine here is measured against:

> All releases of SQLite version 3 can read and write database files created by
> the very first SQLite 3 release (version 3.0.0) going back to **2004-06-18**.
> This is "backwards compatibility". The developers promise to maintain backwards
> compatibility of the database file format for all future releases of SQLite 3.

That is **20+ years of a frozen on-disk format.** Forward compatibility is
*not* guaranteed (an old SQLite may not read a file using a newer feature), but
backward compatibility is absolute. This is precisely the property a
pick-once-commit-hard P2P kernel store wants: a file written today is readable
by every future kernel version, so an engine bump never forks a peer's local
state. No pure-Rust engine here matches this. See
[format-stability.md](format-stability.md).

SQLite also documents "SQLite As An Application File Format" — the explicit
position that a SQLite DB is a reasonable *document/container* format, not just
a query engine. Relevant if Myrhiza ever wants its on-disk store to be
inspectable with off-the-shelf tooling.

## Structure + concurrency

- **B-tree** per table/index, in a single file (plus journal/WAL sidecars).
- **Single writer, many readers** — and in WAL mode, "readers do not block
  writers and a writer does not block readers." Default is still the rollback
  journal; WAL has been available since 3.7.0 (2010).

## Durability (the nuanced part — a teaching case for Myrhiza)

SQLite's durability is governed by `journal_mode` × `PRAGMA synchronous`, and
the defaults are *not* fully durable. The crucial distinction
(see [crash-consistency.md](crash-consistency.md)):

- **`synchronous=FULL`** ensures the database is never *corrupted* by a
  crash/power loss.
- **`synchronous=NORMAL`** (the common WAL setting) can lose the *last*
  committed transaction on power loss — the DB stays consistent, but the tail
  is not durable.
- **`synchronous=EXTRA`** adds a directory fsync for last-transaction durability
  in DELETE journal mode.

"FULL ensures that the database isn't corrupted, but NOT that the last
transaction is durable." Third-party writeups — e.g. agwa's *"SQLite's
Durability Settings are a Mess"* and avi.im's *"SQLite commits are not durable
under default settings"* — document how easy it is to assume durability you
don't have. Myrhiza's Persister must set these knobs explicitly, not inherit
defaults.

## Context: litestream / cr-sqlite

Two SQLite-ecosystem projects clarify the local-first design space (neither is a
Myrhiza dependency; they are reference points for what SQLite alone does *not*
do):

- **litestream** (Ben Johnson; Apache-2.0) — streaming, continuous replication of a
  SQLite file to S3/another file for disaster recovery. Single-node DR, *not*
  multi-writer. Shows that SQLite-alone has no built-in replication.
- **cr-sqlite / Vlcn** (`vlcn-io/cr-sqlite`; MIT) — a loadable extension adding
  CRDTs + multi-writer convergence to SQLite via metadata tables and triggers,
  so peers with a shared schema merge and converge (last-write-wins, counters,
  fractional-index CRDTs). Latest release **v0.16.3, 2024-01-17** — note the
  release cadence has been quiet since (no banner, but ~28 months / ~2.4 years
  without a release as of 2026-05-29; weigh maintenance accordingly). cr-sqlite
  is the *application-layer
  convergence* problem Myrhiza solves differently — via deterministic
  `state-apply` over a Merkle DAG, not via CRDT triggers in the storage engine.
  See [`crdts/`](../crdts/) and the spec's
  [`convergence.md`](../../specs/2026-05-09-myrhiza-master-design/convergence.md).

## Implications for Myrhiza

- **Use SQLite as the format-stability yardstick**, not (likely) as the kernel
  engine — pulling in C bindings cuts against the pure-Rust embeddability axis,
  and Myrhiza doesn't need SQL. But its 20-year frozen format is the standard a
  redb/fjall pick should be honestly measured against ([lessons.md](lessons.md)).
- **The durability-knob lesson transfers directly:** every engine here has a
  "consistent but last-write-not-durable" default. Myrhiza's Persister must
  choose the durable setting deliberately and document the convergence-tail
  consequence of the cheaper one.
- cr-sqlite is the **rejected runner-up paradigm**: convergence-in-the-storage-
  engine. Myrhiza keeps convergence in `state-apply` and treats storage as dumb
  durable bytes — name this when citing.

## Sources

- https://www.sqlite.org/onefile.html (the backward-compatibility promise quoted above)
- https://www.sqlite.org/formatchng.html
- https://sqlite.org/fileformat.html
- https://sqlite.org/appfileformat.html
- https://tool.oschina.net/uploads/apidocs/sqlite/wal.html
- https://www.agwa.name/blog/post/sqlite_durability
- https://avi.im/blag/2025/sqlite-fsync/
- https://litestream.io/
- https://github.com/vlcn-io/cr-sqlite
