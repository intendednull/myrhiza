**Date:** 2026-05-29
**Status:** active
**Subject:** GC strategies — refcount vs reachability (mark-and-sweep) across the five systems

# GC strategies: refcount vs mark-and-sweep

The central design axis of a content-addressed store is **how it decides a block
is dead**. There are two families, and a striking finding of this survey is that
*every one of the production content-addressed stores here uses mark-and-sweep
reachability, and none uses pure reference counting* — despite refcounting being
the "obvious" online answer.

## The two families

**Reference counting.** Keep a per-object counter of how many references point at
it; free at zero. Cheap and incremental (no full-store walk), and gives prompt
reclamation. But: it cannot reclaim **reference cycles**; the counter is a piece
of mutable derived state that can desync from reality under crashes or concurrent
writers (a miscounted refcount frees live data or leaks dead data); and it
requires every reference mutation to be transactionally coupled to a counter
update.

**Mark-and-sweep reachability.** Define a set of **roots**, walk the graph,
**mark** everything reachable, then **sweep** (delete) everything unmarked. No
per-object counter to desync; handles cycles trivially; the store's truth is just
"the bytes + the roots," and reachability is *re-derived* each run. Cost: a full
walk over the root set's transitive closure, and a scan of the whole store to
find the unmarked.

## How each system lands

| System | Roots | Algorithm | Refcount anywhere? |
|---|---|---|---|
| git | refs + reflog + index | mark-and-sweep, with grace windows | no |
| restic | snapshots | mark-and-sweep (`prune` walks trees) | no |
| IPFS/boxo | pins + MFS root | mark-and-sweep (`repo gc` marked-set) | no |
| casync | `.caidx`/`.caibx` indexes | external sweep (no online GC) | no |
| iroh-blobs | tags | mark-and-sweep (sweep untagged) | no |

The convergent choice is not an accident. In a **content-addressed** store the
same bytes can be reached by many independent roots (that's the whole point of
dedup), so a refcount would have to be incremented/decremented on every
root-graph mutation across the whole system — and content-addressed graphs are
exactly where accidental sharing and (in mutable-DAG systems) cycles arise.
Reachability sidesteps both: roots are explicit and few; everything else is
derived.

## The cost mark-and-sweep pays, and how each system pays it

Mark-and-sweep's weakness is that the sweep is a *global* operation — it must
look at every object to decide what's unmarked, and it must not race a concurrent
writer. The systems differ in how they pay that:

- **git** pays with **grace windows** (mtime + `prune.expire 2.weeks`,
  `reflogExpireUnreachable 30 days`) — never delete something *recently*
  unreachable, on the theory a concurrent process may still be attaching it. See
  [git.md](git.md).
- **restic** pays with a **global exclusive lock** during `prune`, plus the
  `forget`/`prune` split so the expensive sweep is rare and scheduled. See
  [restic.md](restic.md).
- **boxo** pays with the **`GCLocker` RWMutex** — pins take a shared read lock,
  GC takes the exclusive write lock, `GCRequested` provides fairness. The most
  precise of the three. See [ipfs-boxo.md](ipfs-boxo.md).
- **casync** doesn't pay online at all — the sweep is offline/external, which is
  only safe because its stores aren't concurrently mutated. See
  [casync.md](casync.md).
- **iroh-blobs** sweeps untagged blobs; the tag layer alone does **not**
  guarantee serialization against in-flight transfers (a known sharp edge). See
  [iroh-blobs-disk.md](iroh-blobs-disk.md).

The grace window vs. lock distinction is itself a theme:
[concurrency-and-locking.md](concurrency-and-locking.md).

## Myrhiza relevance

Myrhiza's event log is a **per-author Merkle DAG** with explicit
heads/roots — which is to say the reachability model already fits natively.
Log truncation past a snapshot
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
~§200) is a mark-and-sweep: the snapshot anchor + current heads are the roots,
events not reachable below the truncation point are sweepable. The survey says:
**do not introduce per-event refcounts.** Keep roots explicit (heads, snapshot
anchors, retention tags) and re-derive reachability. This also aligns with the
determinism posture — reachability is a pure function of (roots, DAG); a refcount
is mutable derived state that could desync across peers. See [lessons.md](lessons.md).

## Sources

- [Git cruft-packs documentation](https://git-scm.com/docs/cruft-packs)
- [restic forget/prune documentation](https://restic.readthedocs.io/en/stable/060_forget.html)
- [Guide to IPFS garbage collection — LogRocket](https://blog.logrocket.com/guide-ipfs-garbage-collection/)
- [ipfs/boxo blockstore source](https://github.com/ipfs/boxo/blob/main/blockstore/blockstore.go)
