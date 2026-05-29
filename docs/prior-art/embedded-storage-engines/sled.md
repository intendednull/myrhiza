**Date:** 2026-05-29
**Status:** active
**Subject:** sled — pure-Rust lock-free embedded DB; long-running 1.0 rewrite, no stable release since 2021

# sled

"The champagne of beta embedded databases" (the repo's own tagline). A pure-Rust
embedded key-value store by Tyler Neely (`spacejam`). Architecturally
ambitious — a lock-free, log-structured design influenced by the Bw-tree (a
lock-free B-tree variant from Microsoft Research) and LLAMA. Repo:
`github.com/spacejam/sled`.

**The headline: maintenance status.** sled is *not abandoned* but it is *not
shippable for a pick-once kernel-storage decision.* The honest, verified picture
(2026-05-29):

- **Last published stable: `0.34.7`, 2021-09-12** — ~4.7 years ago.
- **Newest published anything: `1.0.0-alpha.124`, 2024-10-11** — ~19 months
  ago. The 1.0 line has never left alpha.
- GitHub `pushed_at` is recent (2026-04-04) — there *is* ongoing work on `main`
  — but it has not produced a release in over a year and a half.
- 171 open issues, ~9K stars (popularity is legacy: 497 reverse deps, mostly
  pinned to old `0.34.x`). License **MIT OR Apache-2.0**. Effectively a
  single-maintainer project (`spacejam`, 3532 commits; next contributor 72).

## What the maintainer says (verbatim, from the repo README)

> This README is out of sync with the main branch which contains a large
> in-progress rewrite

> if reliability is your primary constraint, **use SQLite. sled is beta.**

> if storage price performance is your primary constraint, **use RocksDB. sled
> uses too much space sometimes.**

> quite young, should be considered unstable for the time being.

> the on-disk format is going to change in ways that require manual migrations
> before the `1.0.0` release!

The maintainer is steering reliability-first and space-first users to SQLite and
RocksDB respectively. For a project that weights format stability heavily, that
guidance is dispositive.

## The rewrite

The 1.0 effort is a full storage-subsystem rewrite on a modular basis under the
**komora project** (`github.com/komora-io`), in particular the **marble**
storage engine. Stated goals: dramatically lower space amplification and GC /
write-amplification overhead, plus a tree-node memory-layout rewrite to cut
fragmentation and serialization cost. It is real engineering, but it has been
in flight for years without a stable landing.

## Architecture (for the record)

- **Lock-free, log-structured.** Writes append to a log; a page-table indirection
  layer (Bw-tree style) lets readers and writers proceed without locks.
- **Multi-writer / lock-free concurrency** — distinct from the
  single-writer-many-reader model of redb / LMDB / SQLite. (More writer
  concurrency in theory; the cost has historically been space amplification and
  GC complexity.)
- ACID transactions are **optimistic** — the README warns: do not perform IO or
  touch external state inside a transaction closure, because the closure may be
  retried.
- `flush` / `flush_async` for explicit durability points.

## On-disk format stability

**None, by the project's own admission.** The format will change with required
manual migrations before 1.0, and 1.0 has not arrived. This is the
worst-possible position on the axis Myrhiza weights most — see
[format-stability.md](format-stability.md).

## Implications for Myrhiza

- **Do not pick sled for the kernel.** Not because it is bad engineering — the
  Bw-tree / komora-marble direction is interesting — but because a
  pick-once-commit-hard kernel store needs a *stable on-disk format and a
  shipped release*, and sled has neither. Its own maintainer routes
  reliability-first users to SQLite.
- **Worth studying, not adopting** ([lessons.md](lessons.md), "borrow"): the
  lock-free Bw-tree page-indirection idea, and the komora/marble GC-vs-space
  redesign, are reference material if Myrhiza ever needs a multi-writer local
  store. They are not a v1 dependency.
- Treat sled as the cautionary data point on **single-maintainer pre-1.0
  ambition**: a clever design that never reached a stable format is a forked-
  state risk every Myrhiza peer would inherit.

## Sources

- https://crates.io/api/v1/crates/sled
- https://github.com/spacejam/sled
- https://raw.githubusercontent.com/spacejam/sled/main/README.md
- https://github.com/komora-io
