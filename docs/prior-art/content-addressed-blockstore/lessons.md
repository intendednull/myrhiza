**Date:** 2026-05-29
**Status:** active
**Subject:** Lessons — what this survey validates, what to avoid, what to borrow for Myrhiza's on-disk store

# Lessons for Myrhiza

The decision file. Each lesson ties to a named Myrhiza surface: the kernel-owned
local blob/event store (`FsStore`, deferred from B-10 which shipped iroh-blobs
fetch on `MemStore`), the snapshot-cache retention model
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.2; [risks.md](../../specs/2026-05-09-myrhiza-master-design/risks.md) §19), and
log-truncation / GC-against-live-roots
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
~§200). This is the on-disk counterpart to the wire-side
[`prior-art/iroh/blobs.md`](../iroh/blobs.md).

## Validates (Myrhiza choices this survey supports)

- **Mark-and-sweep reachability over refcounting.** All five systems use
  reachability from explicit roots; *none* uses pure refcounting
  ([gc-strategies.md](gc-strategies.md)). Myrhiza's per-author Merkle DAG with
  explicit heads is already a reachability graph — log truncation past a snapshot
  (`convergence.md` ~§200) is a mark-and-sweep with snapshot anchors + live heads
  as roots. **Do not add per-event refcounts.** Bonus: reachability is a pure
  function of (roots, DAG), so it stays inside the determinism posture; a
  refcount is mutable derived state that can desync across peers.

- **Tags/roots as the retention primitive (B-10's choice).** B-10's
  `bundle/<manifest_hash>` tag — kept alive while installed, dropped on uninstall
  ([b-10 spec](../../specs/2026-05-26-b-10-bundle-distribution-design.md) §4.3) —
  is the same recursive-root model as restic snapshots, IPFS recursive pins, and
  iroh HashSeq tags ([retention-and-roots.md](retention-and-roots.md)). The
  survey confirms this is the right shape; extend it (not replace it) for the
  snapshot cache and live heads.

- **Wire-vs-disk separation.** iroh-blobs already separates the interleaved Bao
  wire stream from the data+outboard disk layout
  ([iroh-blobs-disk.md](iroh-blobs-disk.md)). The survey validates keeping disk
  format a purely local concern: the content hash is the cross-peer invariant,
  the on-disk pack/outboard layout is not. Peers converge on hashes regardless of
  how they store them ([open-problems.md](open-problems.md) §7).

- **Two-phase delete (root-removal separate from byte-reclamation).** restic's
  `forget` (remove root) vs `prune` (reclaim bytes) split
  ([restic.md](restic.md)) matches Myrhiza's natural split between deciding to
  truncate (a convergence/policy decision) and actually reclaiming the bytes (a
  maintenance pass). Keep them separate so the expensive sweep is scheduled and a
  too-eager truncation decision is catchable before it's permanent.

## Avoid (specific pitfalls + Myrhiza mitigation)

- **Grace-window-*only* GC for a store that serves live peers.** git's mtime
  window ([git.md](git.md)) is probabilistic — it assumes a writer attaches its
  root within the window and can still lose a live-but-unrooted object under
  clock skew or a stalled process ([open-problems.md](open-problems.md) §4).
  *Mitigation:* for the kernel store, use boxo's **explicit `GCLocker`** as the
  primary guarantee and a grace window only as a backstop for the partial-blob-
  being-filled case ([concurrency-and-locking.md](concurrency-and-locking.md)).

- **casync's offline/external GC model.** No online collector is fine for
  read-mostly operator-managed image stores; it is a trap for a concurrently-
  served P2P node ([casync.md](casync.md)). *Mitigation:* Myrhiza's store must
  collect online, concurrency-safe — design the lock in from the start.

- **Loose-object explosion.** git's pre-cruft-pack failure: one file per dead
  object (each keyed by mtime for the grace check) caused inode starvation at
  scale ([git.md](git.md)). *Mitigation:* if the kernel keeps a grace window on
  truncated events, batch the dead set into one container with side-channel
  timestamps (git's cruft-pack pattern), don't keep one file per dead event.

- **Assuming a tag/pin guarantees availability.** A retention root keeps bytes on
  *this* disk only; it is not a network durability guarantee
  ([open-problems.md](open-problems.md) §1). *Mitigation:* durable availability
  is [maintenance.md](../../specs/2026-05-09-myrhiza-master-design/maintenance.md)
  §12's persister/participation layer, never a blockstore property — keep the two
  concerns separate so neither is mistaken for the other.

- **Letting the disk format leak into a cross-peer invariant.** Several systems'
  on-disk formats are in flux (iroh-blobs store v2 "not production quality";
  restic repo v2; git pack versions). *Mitigation:* pin nothing cross-peer to the
  pack layout; the content hash is the only cross-peer contract
  ([open-problems.md](open-problems.md) §7).

## Borrow (primitives worth studying directly)

- **boxo's `GCLocker` (the headline borrow).** `PinLock` (read lock — many
  concurrent serves/writes) vs `GCLock` (write lock — exclusive collection) plus
  `GCRequested` (fairness, so long serves yield to a queued GC). Doc-comment text
  verified against [boxo source](https://github.com/ipfs/boxo/blob/main/blockstore/blockstore.go).
  Maps 1:1 onto "a sync/snapshot provider serving an event range
  ([maintenance.md](../../specs/2026-05-09-myrhiza-master-design/maintenance.md)
  §12.2) must hold a read lock that excludes the truncating GC's write lock." Use
  this shape, enforced by the type system, over restic's coarser global lock.
  ([concurrency-and-locking.md](concurrency-and-locking.md))

- **restic's `--max-unused` dead-fraction tolerance + repack bandwidth cap.** If
  the kernel ever packs events/blobs into larger containers, copy restic's
  "tolerate up to N% dead bytes before repacking, and cap bytes rewritten per
  pass" policy ([compaction-and-repack.md](compaction-and-repack.md)) — and its
  "remove the index pointer before deleting the container" write-ordering
  invariant, which also applies to log truncation.

- **git's cruft-pack mtime sidecar.** The pattern of segregating dead-but-in-
  grace objects into one container with a parallel mtime file
  ([git.md](git.md)) — if Myrhiza wants a grace window without the loose-object
  inode cost.

- **IPFS recursive vs direct pins.** The recursive/direct distinction
  ([retention-and-roots.md](retention-and-roots.md)) is the right vocabulary for
  "keep this whole bundle/snapshot subtree" vs "keep exactly this one event/blob
  I'm actively serving" — the latter being a clean way to express an in-flight
  serve as a temporary root.

- **CDC (restic Rabin / casync buzhash) — but scoped.** Content-defined chunking
  is the right dedup model for *bulk app assets* if Myrhiza ever stores them, not
  for the event log (where per-event/per-author attribution must survive and CDC
  erases object boundaries — [casync.md](casync.md)). Borrow it deliberately and
  narrowly.

## Sources

- [ipfs/boxo blockstore source](https://github.com/ipfs/boxo/blob/main/blockstore/blockstore.go)
- [restic design document](https://github.com/restic/restic/blob/master/doc/design.rst)
- [restic forget/prune documentation](https://restic.readthedocs.io/en/stable/060_forget.html)
- [git-gc(1) documentation](https://git-scm.com/docs/git-gc)
- [Git cruft-packs documentation](https://git-scm.com/docs/cruft-packs)
- [`prior-art/iroh/blobs.md`](../iroh/blobs.md)
- [B-10 bundle distribution design](../../specs/2026-05-26-b-10-bundle-distribution-design.md)
