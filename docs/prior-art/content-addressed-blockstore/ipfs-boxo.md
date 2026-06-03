**Date:** 2026-05-29
**Status:** active
**Subject:** IPFS boxo blockstore — blocks/CIDs, pin model, mark-and-sweep, and the GCLocker concurrent-GC-vs-serve lock

# IPFS boxo blockstore

[`boxo`](https://github.com/ipfs/boxo) is the maintained set of Go reference
libraries for IPFS (it powers Kubo, the dominant implementation). Its
[`blockstore`](https://github.com/ipfs/boxo/tree/main/blockstore) package is the
**load-bearing reference for this folder** because it is the one that states the
concurrent-GC-vs-serve safety problem as an explicit, named lock interface
rather than leaving it implicit in a grace window (git) or a global repo lock
(restic).

> **Cite `boxo/blockstore`, not the archived `go-ipfs-blockstore`.** The old
> `github.com/ipfs/go-ipfs-blockstore` package's own docs now say "switch to the
> maintained version at github.com/ipfs/boxo/tree/main/blockstore." The
> interfaces below are the same in both; boxo is the live path.

- **License:** Apache-2.0 / MIT dual (verified — boxo repo declares dual
  license).

## The block model

A **block** is a chunk of bytes addressed by its **CID** (content identifier:
multihash + codec + version). The blockstore is "a thin wrapper over a datastore,
giving a clean interface for Getting and Putting block objects" — i.e. the
object-graph layer sits over a generic KV datastore, exactly the layering
Myrhiza would have between a `FsStore` and an embedded engine
(see `embedded-storage-engines`). Blocks form a Merkle DAG; a file is a DAG of
blocks under a root CID.

## Pins are the retention roots

IPFS GC is mark-and-sweep from **pins** plus the MFS root. Three pin types
([IPFS pinning docs](https://docs.ipfs.tech/how-to/pin-files/)):

- **Recursive pin** — the CID and *all descendants* are retained. `ipfs add`
  pins recursively by default.
- **Direct pin** — only that one block, not its children.
- **Indirect pin** — a block retained solely because an ancestor is recursively
  pinned (i.e. derived, not stored as a root).

**MFS (Mutable File System)** entries are not pins but are also protected — "a
mechanism for implicit pinning." So the root set is: recursively-pinned blocks +
their descendants, directly-pinned blocks, the MFS root + its descendants, plus
the pinner's own internal blocks. See [retention-and-roots.md](retention-and-roots.md).

## `ipfs repo gc` = mark-and-sweep

The collector "creates a 'marked' set" containing all of the above roots and
their recursive descendants, "then iterates over every block in the blockstore
and deletes any block that is not found in the marked set." A pinned object
cannot be collected; everything unpinned and not in MFS is fair game. This is
**reachability, not refcounting** — there is no per-block reference count; pins
are explicit roots and GC re-derives reachability each run. Contrast
[gc-strategies.md](gc-strategies.md).

## GCLocker — the "don't collect a block a concurrent op is serving" lock

This is the piece Myrhiza should study most closely. The
[`GCBlockstore`](https://github.com/ipfs/boxo/blob/main/blockstore/blockstore.go)
interface = `Blockstore` + `GCLocker`. `GCLocker` is implemented over a
`sync.RWMutex` plus an `atomic` request counter, with three methods (doc-comment
text verified against the source):

- **`PinLock()` → `Unlocker`** — "locks the blockstore for sequences of puts
  expected to finish with a pin (before GC). Multiple put→pin sequences can write
  through at the same time, but no GC should happen simultaneously." Implemented
  as a **read lock**: any number of concurrent pinning writers, all blocking GC.
- **`GCLock()` → `Unlocker`** — "locks the blockstore for garbage collection. No
  operations that expect to finish with a pin should occur simultaneously."
  Implemented as the **write lock**: exclusive, excludes all pinners.
- **`GCRequested()` → `bool`** — true once `GCLock` has been called and is
  *waiting* to acquire the lock. Lets long-running writers voluntarily yield so a
  pending GC isn't starved.

The hazard this prevents: a peer is mid-way through `put(block) → pin(root)`. If
GC runs *between* the put and the pin, it sees a block with no live root pointing
at it and deletes it — corrupting the operation. The read/write lock makes
"add-then-pin" atomic *with respect to GC* without serializing the adds against
each other. `GCRequested` adds cooperative fairness so writers don't indefinitely
block a queued collector.

boxo states this safety property as an explicit named lock. git approximates it
with a grace window ([git.md](git.md)); restic approximates it with a global
exclusive lock during prune ([restic.md](restic.md)); boxo names it directly. See
[concurrency-and-locking.md](concurrency-and-locking.md).

## Myrhiza relevance

The `GCLocker` shape maps directly onto the kernel's
`myrhiza-state-snapshot-cache` retention model
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.2; [risks.md](../../specs/2026-05-09-myrhiza-master-design/risks.md) §19
"Snapshot lifecycle"): a sync provider serving an old log range to a peer behind
on heads is exactly a "concurrent operation serving a block," and log truncation
against live roots ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
~§200) is exactly the GC that must not run while that serve is in flight. The
`PinLock`/`GCLock` RWMutex + `GCRequested` triple is a ready-made template. See
[lessons.md](lessons.md).

## Sources

- [ipfs/boxo blockstore source](https://github.com/ipfs/boxo/blob/main/blockstore/blockstore.go)
- [ipfs/boxo repository](https://github.com/ipfs/boxo)
- [go-ipfs-blockstore (archived; redirects to boxo)](https://pkg.go.dev/github.com/ipfs/go-ipfs-blockstore)
- [IPFS pinning docs](https://docs.ipfs.tech/how-to/pin-files/)
- [Guide to IPFS garbage collection — LogRocket](https://blog.logrocket.com/guide-ipfs-garbage-collection/)
