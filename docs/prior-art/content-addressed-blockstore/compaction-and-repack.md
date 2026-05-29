**Date:** 2026-05-29
**Status:** active
**Subject:** Repack and compaction — reclaiming dead bytes from partially-live immutable containers

# Compaction and repack

Packing many small objects into a larger immutable container (a packfile, a pack
file, a HashSeq) buys you dedup, fewer inodes, and delta compression — but
creates a follow-on problem: **once a container is partly dead, you can't free
the dead bytes without rewriting the whole container.** This is the
write-amplification cost of packing, and how each system manages it is a theme
worth lifting whole.

## Why packing forces repacking

An immutable container holds N objects. Delete K of them (they became
unreachable) and the bytes are still on disk inside the container; the only ways
to reclaim them are (a) rewrite the container without the dead objects, or (b)
leave them and accept the bloat. Neither is free:

- Rewriting (repack) costs reading + re-writing the *live* remainder — pure
  write amplification proportional to the surviving data, not the freed data.
- Leaving them wastes space and, over time, fragments the store into mostly-dead
  containers.

So every packed store needs a **repack policy**: a threshold of dead-fraction
above which a container is worth rewriting.

## How each system repacks

| System | Container | Repack trigger | Knob |
|---|---|---|---|
| git | packfile | `git gc` / auto when too many loose objects or packs | `gc.auto`, `repack` heuristics; cruft pack for unreachable |
| restic | pack file | `prune` reclassifies partial packs | **`--max-unused`** (default **5%**), `--max-repack-size` |
| IPFS/boxo | block (no packing) | n/a — blocks are individually deletable | — |
| casync | chunk (no packing) | n/a — chunks individually deletable | — |
| iroh-blobs | blob/HashSeq | store-v2 rewrite manages this | (in flux, 0.90+) |

A sharp split emerges: **block/chunk stores (boxo, casync) sidestep repack
entirely** because their unit of storage *is* their unit of deletion — a dead
block is just an `unlink`. **Packed stores (git, restic) get better dedup and
fewer files but inherit the repack tax.** This is a direct tradeoff, not a free
lunch.

## restic's repack policy in detail

restic classifies every pack during `prune` as fully-used / fully-unused /
partially-used, and the **`--max-unused` (default 5%)** parameter is exactly the
dead-fraction tolerance: restic will leave up to 5% of the repository as dead
bytes rather than repack, repacking only when the dead fraction would exceed
that. `--max-repack-size` caps the bytes rewritten per run so a single prune
doesn't trigger a massive re-upload. And the doc is honest about the cost:
repacking "must download the file from the repository storage and re-upload the
needed data" — **bandwidth-intensive for remote storage**. See
[restic.md](restic.md).

## git's cruft pack is the dead-object-batching variant

git's repack story has a twist the others lack: unreachable-but-in-grace-window
objects. Rather than rewrite the main pack on every gc, `git repack --cruft`
sweeps all unreachable objects into a *separate* cruft pack with side-channel
mtimes ([git.md](git.md)), so the live pack stays clean and the dead set is
batched into one container that ages out wholesale. This is a useful pattern:
**segregate the dead-but-not-yet-collectable set into its own container** instead
of leaving it interleaved with live data.

## Myrhiza relevance

Two design forks for a Myrhiza `FsStore`:

1. **Pack or don't pack.** If the store packs events into larger immutable
   blobs (for fewer inodes / better compression of a per-author chain), it
   inherits the repack tax and needs a `--max-unused`-style threshold + a
   bandwidth cap on repack. If it stores each event/blob individually (boxo
   style), deletion is a cheap `unlink` and there's no compaction problem — at
   the cost of more inodes and worse compression. The cruft-pack
   *loose-object-explosion* episode ([git.md](git.md)) is the warning against the
   naive "one file per object" extreme at scale.
2. **Repack must respect the same GC lock.** Repacking *moves live bytes* (new
   container, delete old) — to a concurrent reader/serve that's the same hazard
   as deletion. Whatever lock/grace mechanism guards GC
   ([concurrency-and-locking.md](concurrency-and-locking.md)) must also guard
   repack, and restic's "remove from index before deleting the pack" ordering
   invariant is the template: update the pointer to the new container before
   unlinking the old.

Log truncation
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
~§200) is repack's analog at the event-log layer — reclaiming the dead tail below
a snapshot anchor — and faces the identical "rewrite the live remainder, don't
race a serve" constraints. See [lessons.md](lessons.md).

## Sources

- [git-repack(1) documentation](https://git-scm.com/docs/git-repack)
- [Git cruft-packs documentation](https://git-scm.com/docs/cruft-packs)
- [restic forget/prune documentation](https://restic.readthedocs.io/en/stable/060_forget.html)
- [restic design document](https://github.com/restic/restic/blob/master/doc/design.rst)
