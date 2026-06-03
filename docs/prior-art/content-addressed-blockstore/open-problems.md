**Date:** 2026-05-29
**Status:** active
**Subject:** Open problems — what content-addressed blockstores structurally do NOT solve

# Open problems

These are the things the five systems here do *not* solve, by design or by gap.
Naming them keeps a Myrhiza store author from assuming a property they don't get.

## 1. Content-availability is not a retention guarantee

A pin/tag/ref keeps a block alive *on the local disk*. It says nothing about
whether the block exists *anywhere on the network*. iroh-blobs is explicit:
"tag-based GC is fine for local cache management but it is *not* a content-
availability guarantee" ([`prior-art/iroh/blobs.md`](../iroh/blobs.md)). IPFS has
the same hole — pinning locally doesn't replicate; that's what pinning *services*
exist to sell. **None of these systems provides durable availability;** that is a
higher-layer replication/participation concern. For Myrhiza this is
[maintenance.md](../../specs/2026-05-09-myrhiza-master-design/maintenance.md) §12
(persister/archival modules + Sybil-resistant participation), not a blockstore
primitive.

## 2. No quota / LRU / cost-bounded eviction

Retention here is binary: rooted (kept) or unrooted (collectable). **None of
these systems caps total store size or evicts least-recently-used blobs under
pressure.** iroh-blobs has "no refcount, no LRU, no quota at the iroh-blobs
layer." git has no size ceiling. restic's repo grows until you `forget`. A store
that must live on a consumer device with bounded disk
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.5 storage ceiling) needs an eviction policy these systems don't supply —
explicitly deferred to B-9 in
[b-10 spec](../../specs/2026-05-26-b-10-bundle-distribution-design.md) §"Tag GC +
LRU / quota".

## 3. Refcount's cycle problem is dodged, not solved

Mark-and-sweep handles cycles, but at the cost of a global walk
([gc-strategies.md](gc-strategies.md)). No system here offers *incremental*
reclamation with cycle support — you either accept the full-walk cost
(reachability) or the cycle bug (refcount). For very large stores this is a real
limitation; generational/incremental GC of content-addressed graphs is not a
solved problem in any of these tools.

## 4. The grace window is a heuristic, not a proof

git's mtime grace window ([git.md](git.md)) is a *probabilistic* answer to the
add-then-pin race — it assumes a concurrent writer attaches its root within the
window. Under clock skew, a paused process, or a window set too short, it can
still delete a live-but-not-yet-rooted object. Only boxo's explicit lock
([concurrency-and-locking.md](concurrency-and-locking.md)) gives a *hard*
guarantee, and it pays with mandatory lock discipline. There is no free
lunch: hard guarantee ⇒ explicit locking; lock-free ⇒ probabilistic window.

## 5. Repack write-amplification has no escape inside packed stores

Once you pack ([compaction-and-repack.md](compaction-and-repack.md)), reclaiming
dead bytes costs rewriting live bytes. `--max-unused` (restic) just chooses *how
much* waste to tolerate; it doesn't eliminate the amplification. The only systems
without this cost are the ones that don't pack (boxo, casync) — and they pay in
inodes and lost compression instead.

## 6. Verification metadata is extra disk

The data+outboard split ([iroh-blobs-disk.md](iroh-blobs-disk.md)), restic's
index files, and git's `.idx` all store *verification/locator* metadata beside
the content. This is necessary for range-verified reads but is pure overhead the
naive "just store the bytes" model doesn't have. None of these systems makes it
free; they make it *worth it*.

## 7. Determinism across implementations is not guaranteed

A content hash is deterministic, but **the on-disk layout, pack boundaries, and
GC timing are not** — and several of these systems warn their formats are in
flux. iroh-blobs's store v2 is "not yet production quality"
([`prior-art/iroh/blobs.md`](../iroh/blobs.md)); git's pack format has versioned
across history; restic bumped to repo format v2 for compression. For Myrhiza,
where the *content* hash is the cross-peer convergence anchor, the lesson is that
the **disk format is a local concern and must not leak into any cross-peer
invariant** — peers may store the same logical blob with totally different pack
layouts and still converge on the same content hash. This is exactly the
wire-vs-disk separation ([iroh-blobs-disk.md](iroh-blobs-disk.md)).

## Sources

- [`prior-art/iroh/blobs.md`](../iroh/blobs.md)
- [restic design document](https://github.com/restic/restic/blob/master/doc/design.rst)
- [IPFS pinning docs](https://docs.ipfs.tech/how-to/pin-files/)
- [git-gc(1) documentation](https://git-scm.com/docs/git-gc)
