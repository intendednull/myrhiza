**Date:** 2026-05-29
**Status:** active
**Subject:** git — loose objects, packfiles, reachability GC, reflog grace, cruft packs

# git as a content-addressed blockstore

Git is the canonical content-addressed object store: every object (blob, tree,
commit, tag) is named by the hash of its zlib-deflated contents and stored under
`.git/objects/`. Its GC story is the reference design for "mark-and-sweep from
refs with a grace window," and it is the system most readers already have a
mental model of, so it anchors the rest of this folder.

## Loose objects vs packfiles

Git stores objects in two physical forms:

- **Loose objects** — one zlib-compressed file per object, keyed by its SHA-1
  (now also SHA-256) hash, sharded as `.git/objects/ab/cdef…`. Simple,
  append-only, but "inefficient for large repositories since nearly identical
  objects are stored separately"
  ([Git internals — packfiles](https://git-scm.com/book/en/v2/Git-Internals-Packfiles)).
- **Packfiles** — a single `.pack` binary holding many objects, with a sibling
  `.idx` index giving byte offsets so any object can be seeked directly. Packing
  is where git's space savings come from.

When packing, git "looks for files that are named and sized similarly, and
stores just the deltas from one version of the file to the next." Git stores the
*newest* version intact and older versions as deltas against it (you most often
want fast access to the latest). The docs' worked example: a 22,054-byte blob,
then a near-identical edit stored as a **9-byte delta** against it. This is the
key contrast with chunk-based dedup ([restic.md](restic.md),
[casync.md](casync.md)): git's dedup unit is the whole object plus a delta
chain, not content-defined sub-object chunks. See
[compaction-and-repack.md](compaction-and-repack.md).

## When git packs: `git gc`

Git packs in three situations: too many loose objects accumulate, you run
`git gc` by hand, or you push to a remote. `git gc` is the repack-plus-prune
entry point — "Git will occasionally repack your database automatically, always
trying to save more space."

## GC = mark-and-sweep from refs

Git's collector is **reachability-based mark-and-sweep**, not refcounting. The
live roots are everything reachable from:

- branch/tag refs and `HEAD`,
- the **reflog** (per-ref history of where it pointed), and
- the index / other internal roots.

Anything not reachable from a root is *unreachable* and a GC candidate. There is
no per-object reference count anywhere in git. See
[gc-strategies.md](gc-strategies.md) for why this matters.

## The grace windows (this is the load-bearing part for Myrhiza)

Git does **not** delete unreachable objects immediately. Multiple overlapping
grace windows protect recently-detached and in-flight objects. Verified default
config values (from [git-gc(1)](https://git-scm.com/docs/git-gc)):

| Config | Default | What it protects |
|---|---|---|
| `gc.reflogExpire` | **90 days** | reachable reflog entries |
| `gc.reflogExpireUnreachable` | **30 days** | reflog entries no longer reachable from the ref tip |
| `gc.pruneExpire` | **2 weeks** (`git gc` runs `prune --expire 2.weeks.ago`) | loose unreachable objects |
| `gc.worktreePruneExpire` | **3 months** | stale worktree administrative files |

`"now"` expires everything immediately; `"never"` disables expiry. The reflog is
itself a retention mechanism: a commit you `git reset --hard` away from is
unreachable from any ref but still recoverable for 30 days because the reflog
points at it. The 2-week prune delay on loose objects exists specifically so a
concurrent `git` process that just *created* an object (but hasn't yet pointed a
ref at it) does not have that object swept out from under it. This is git's
answer to the concurrent-GC-vs-write race that boxo solves with an explicit lock
([concurrency-and-locking.md](concurrency-and-locking.md)) — git uses **object
mtime + a generous grace window** instead.

## Cruft packs (Git 2.37+/2.38, the modern unreachable-object story)

Storing unreachable objects loose (so each keeps an individual mtime for the
grace check) caused a real production failure mode: a repo with many unreachable
objects inside their grace window produces a **loose-object explosion** that "can
lead to inode starvation and degrade the performance of the whole system"
([cruft-packs docs](https://git-scm.com/docs/cruft-packs)).

The fix is the **cruft pack**: a single pack holding all unreachable objects,
with their per-object mtimes stored in a parallel `.mtimes` file (a 4-byte
unsigned int per object, located via binary search on the pack `.idx`).
`git repack --cruft` does an all-into-one repack where the main pack is
everything reachable and the cruft pack is everything else. This keeps the grace
window semantics (per-object mtime) without the inode cost of millions of loose
files. GitHub's engineering writeup ("Scaling Git's garbage collection")
documents this as the change that made GC viable for very large repos.

**Myrhiza relevance:** the cruft-pack episode is a concrete instance of "the
naive grace-window implementation (one loose file per dead object, keyed by
mtime) doesn't scale, so you batch the dead set into one container with
side-channel mtimes." A Myrhiza `FsStore` that keeps a grace window on truncated
log events will face the same scaling cliff if it keeps one file per dead event.
See [lessons.md](lessons.md).

## Sources

- [Git internals — packfiles](https://git-scm.com/book/en/v2/Git-Internals-Packfiles)
- [git-gc(1) documentation](https://git-scm.com/docs/git-gc)
- [Git cruft-packs documentation](https://git-scm.com/docs/cruft-packs)
- [git-repack(1) documentation](https://git-scm.com/docs/git-repack)
- [Scaling Git's garbage collection — GitHub Blog](https://github.blog/engineering/architecture-optimization/scaling-gits-garbage-collection/)
