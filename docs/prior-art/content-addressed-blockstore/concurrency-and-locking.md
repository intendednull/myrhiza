**Date:** 2026-05-29
**Status:** active
**Subject:** Concurrent-GC-vs-serve safety — "don't collect a block a concurrent operation is serving"

# Concurrency and locking

The defining correctness hazard of an *online* content-addressed store: GC runs
concurrently with reads, writes, and serves. The classic bug is the **add-then-
pin race** — a writer puts a block, and before it attaches a root, GC sees an
unrooted block and deletes it. The symmetric bug is the **serve-vs-collect
race** — GC deletes a block another peer is mid-stream of fetching. Every system
here addresses this; the spread of answers — grace window, global lock, RWMutex —
is the part of this folder most directly applicable to the Myrhiza store.

## The four answers, weakest to strongest guarantee

### 1. casync — no online GC (avoid this for a live store)

casync's chunk store is append-only and GC is offline/external. There is *no*
concurrency story because the store is assumed read-mostly and operator-managed.
Correct for image distribution; wrong for a P2P node that serves and mutates
concurrently. See [casync.md](casync.md).

### 2. git — mtime grace window

git never deletes a *recently* unreachable object. `git gc` prunes loose objects
only after `prune.expire` (default **2 weeks**) measured against the object's
**mtime**, and keeps unreachable reflog entries for **30 days**. The bet: any
concurrent process attaching a new object will have pointed a ref at it well
within the window. The cruft pack
([git.md](git.md)) preserves per-object mtimes in a `.mtimes` sidecar so this
window survives packing. Probabilistic, not a hard lock — but robust in practice
and lock-free for the common path.

### 3. restic — global exclusive lock + write-ordering invariant

`prune` (and any data-removing op) takes an **exclusive repository lock**, so no
backup runs during a sweep. Plus a hard ordering rule: **"a pack must be removed
from the referencing index before it is deleted."** Removing the index entry
first means a concurrent reader either sees the pack-with-index (and reads it) or
sees neither — never a dangling index pointing at deleted bytes. Simple and
correct, but the exclusive lock serializes writers against GC entirely. See
[restic.md](restic.md).

### 4. boxo — `GCLocker` RWMutex (the precise answer)

boxo names the property exactly and solves it with a reader-writer lock plus a
fairness counter (doc-comment text verified against
[boxo source](https://github.com/ipfs/boxo/blob/main/blockstore/blockstore.go)):

- **`PinLock()`** = a **read lock**: "Multiple put→pin sequences can write
  through at the same time, but no GC should happen simultaneously." Many
  concurrent add-then-pin writers, all of them blocking GC.
- **`GCLock()`** = the **write lock**: exclusive, "no operations that expect to
  finish with a pin should occur simultaneously."
- **`GCRequested()`** = fairness: a long-running writer can poll this and yield
  so a queued GC isn't starved indefinitely.

This is strictly more concurrent than restic's global lock (writers don't block
each other, only GC) while giving a *hard* guarantee git's grace window only
gives probabilistically. The cost is that every writer must remember to take
`PinLock` and every collector to take `GCLock` — discipline the type system
should enforce.

## The general pattern

Across all four working answers: **make "create + attach root" atomic with
respect to GC.** Whether by lock (boxo/restic) or by grace window (git), the
store must guarantee that an object in the gap between "written" and "rooted" is
not collected. A store that serves bytes to remote peers must additionally treat
**an in-flight serve as an implicit root** for its duration — boxo via a direct
pin or held `PinLock`, git via the mtime window.

## Myrhiza relevance

The kernel's situation is precisely boxo's: maintenance modules (sync provider,
snapshot provider, replay buffer per
[maintenance.md](../../specs/2026-05-09-myrhiza-master-design/maintenance.md)
§12.2) **serve events/snapshots concurrently** with the kernel truncating the log
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
~§200). A sync provider streaming an old event range to a peer behind on heads is
exactly "a concurrent operation serving a block"; truncation is the GC that must
not run mid-serve. The survey's recommendation: adopt **boxo's `GCLocker`
shape** — a `serve`/`pin` read lock against a `truncate`/`gc` write lock, with a
`GCRequested`-style yield for long serves — in preference to restic's coarse
global lock or git's grace-window-only approach. A grace window is a fine
*backstop* (catches the partial-blob-being-filled case
[iroh-blobs-disk.md](iroh-blobs-disk.md)) but should not be the *only* mechanism
for a store that serves to live peers. See [lessons.md](lessons.md).

## Sources

- [ipfs/boxo blockstore source](https://github.com/ipfs/boxo/blob/main/blockstore/blockstore.go)
- [restic design document](https://github.com/restic/restic/blob/master/doc/design.rst)
- [git-gc(1) documentation](https://git-scm.com/docs/git-gc)
- [Git cruft-packs documentation](https://git-scm.com/docs/cruft-packs)
