**Date:** 2026-05-29
**Status:** active
**Subject:** Content-addressed blockstores — how git / restic / IPFS-boxo / casync / iroh-blobs persist a content-addressed object graph on disk

# Content-addressed blockstores

This folder is about the *on-disk* side of content-addressed storage: how a
local store packs objects, deduplicates them, decides which bytes are still
live, reclaims the rest, and does so without corrupting an in-flight read. It is
the persistence counterpart to the wire-side blob transfer documented in
[`prior-art/iroh/blobs.md`](../iroh/blobs.md) — the wire moves verified bytes
between peers; this folder is about what happens to those bytes once they land
on a disk the kernel owns.

It pairs with `embedded-storage-engines` (redb / fjall / sled) as the second
half of the corpus's **Storage & persistence** section. The split is by layer:
embedded-storage-engines is about the *KV/transaction substrate* (ACID, WAL,
crash recovery); this folder is about the *object-graph retention model* that
sits on top of whatever KV layer the kernel picks. A Myrhiza `FsStore` is both —
a blob graph (this folder) materialized over a storage engine (that folder).

These five systems are studied as **reference designs**, not dependencies. The
one exception is iroh-blobs, which Myrhiza *does* depend on (B-10 shipped its
fetch path); its on-disk store is documented here so the kernel's `FsStore`
decision is grounded in the same survey as the others. See
[lessons.md](lessons.md) for how each system maps onto Myrhiza's decision
surface.

## How to use / framing disclosure

These docs are written from Myrhiza's **current design stance** —
capability-mediated host access, P2P-only (no central server), Component-Model
WASM on Wasmtime, and `state-apply` materialized by replaying a per-author
Merkle event log. The survey selects, weights, and reads each system *through
that lens*: which GC model fits a deterministic event-log replay, which lock
shape fits a kernel that serves blobs to live peers, which dedup model fits
per-author/per-event attribution. It is **not a neutral catalog** of storage
engines; a reader evaluating these tools for a different architecture (a central
server, a mutable filesystem, a non-deterministic store) should re-derive the
weightings. The evidence files (git / restic / ipfs-boxo / casync /
iroh-blobs-disk and the theme files) aim to state verifiable facts plainly; the
Myrhiza-facing judgments live in the "Myrhiza relevance" sections and in
[lessons.md](lessons.md).

One caveat specific to the load-bearing case: **iroh-blobs is a hard Myrhiza
dependency**, so this corpus has a standing incentive to soft-pedal the problems
Myrhiza would inherit through it (an in-flux on-disk format, a tag layer that
does not by itself serialize GC against in-flight serves, and the absence of
quota/LRU/availability guarantees). Those are stated bluntly in
[iroh-blobs-disk.md](iroh-blobs-disk.md) and [open-problems.md](open-problems.md)
precisely to resist that pull — treat any place this folder sounds reassuring
about iroh-blobs's disk story as a prompt to re-verify against the upstream repo,
not as settled fact.

## Key facts

| System | Object model | Dedup unit | GC model | Concurrent-GC safety | License |
|---|---|---|---|---|---|
| **git** | loose zlib objects → packfiles | whole object (delta-compressed in packs) | mark-and-sweep from refs; reflog grace + 2-week prune delay; cruft packs hold unreachable | mtime grace window, single-process lock | GPL-2.0 |
| **restic** | content-defined chunks → pack files | variable chunk (avg 1 MiB, CDC) | mark-and-sweep from snapshot roots (`prune`) | exclusive repo lock during prune | BSD-2-Clause |
| **IPFS / boxo** | blocks (CID) | whole block | mark-and-sweep from pins + MFS root | **`GCLocker`**: RWMutex, pins take read lock, GC takes write lock | Apache-2.0 / MIT |
| **casync** | content-defined chunks → `.castr` chunk store | variable chunk (min 16K / avg 64K / max 256K, buzhash) | none built in — chunk store is append-only; external sweep | n/a (no online GC) | LGPL-2.1 |
| **iroh-blobs** | BLAKE3 blob + outboard | whole blob (range-addressable via Bao tree) | tag-based reachability sweep | tags + 0.90 "store v2" rewrite | Apache-2.0 / MIT |

Verified values and their sources are in the per-system files. The license, GC
model, and chunk-size numbers above are each individually verified — see
[git.md](git.md), [restic.md](restic.md), [ipfs-boxo.md](ipfs-boxo.md),
[casync.md](casync.md), and [iroh-blobs-disk.md](iroh-blobs-disk.md).

## Canonical reading order

1. **[git.md](git.md)** — the reference everyone already knows; establishes
   loose-vs-packed, reachability GC, and the grace-window idea.
2. **[restic.md](restic.md)** — adds content-defined chunking, pack files, and
   snapshots-as-GC-roots; the "backup repo" shape.
3. **[ipfs-boxo.md](ipfs-boxo.md)** — the pin model and the `GCLocker`
   concurrent-GC-vs-serve lock. **This is the load-bearing one for Myrhiza**
   (cite `boxo/blockstore`, not the archived `go-ipfs-blockstore`).
4. **[casync.md](casync.md)** — comparator; CDC without an online GC story.
5. **[iroh-blobs-disk.md](iroh-blobs-disk.md)** — the dependency; wire-vs-disk
   distinction and tag GC.

Then the four cross-cutting theme files:

6. **[gc-strategies.md](gc-strategies.md)** — refcount vs mark-and-sweep, the
   central axis.
7. **[retention-and-roots.md](retention-and-roots.md)** — pin / tag / ref /
   snapshot as the same primitive under different names.
8. **[concurrency-and-locking.md](concurrency-and-locking.md)** — "don't collect
   a block a concurrent operation is serving."
9. **[compaction-and-repack.md](compaction-and-repack.md)** — repacking
   partially-live containers; the write-amplification cost.

Finally [open-problems.md](open-problems.md) (what these systems do *not* solve)
and [lessons.md](lessons.md) (the Myrhiza decision file).

## Glossary stub

Full definitions in [glossary.md](glossary.md).

- **Content addressing** — an object's name is the hash of its bytes; identical
  bytes are stored once.
- **Mark-and-sweep** — GC that walks live roots, marks everything reachable,
  then deletes the unmarked.
- **Refcount GC** — GC that tracks a per-object reference count and frees at
  zero; cheaper online but vulnerable to cycles and miscount.
- **Pin / tag / ref / root** — the system-specific name for "a thing GC must not
  collect."
- **Grace window** — a delay before an unreachable object is actually deleted,
  to avoid racing a concurrent writer or losing recently-detached history.
- **Repack / compaction** — rewriting a storage container to drop the dead bytes
  it still holds.
- **CDC** — content-defined chunking; cut boundaries at content-derived offsets
  so edits don't shift every downstream chunk.

## Sources

- [ipfs/boxo blockstore](https://github.com/ipfs/boxo/tree/main/blockstore)
- [restic design document](https://github.com/restic/restic/blob/master/doc/design.rst)
- [Git internals — packfiles](https://git-scm.com/book/en/v2/Git-Internals-Packfiles)
- [casync (systemd)](https://github.com/systemd/casync)
- [iroh-blobs](https://github.com/n0-computer/iroh-blobs)
- [`prior-art/iroh/blobs.md`](../iroh/blobs.md)
