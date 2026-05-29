**Date:** 2026-05-29
**Status:** active
**Subject:** Glossary — system-specific terms used across the content-addressed-blockstore folder

# Glossary

System-specific and cross-cutting terms. General terms (content addressing,
mark-and-sweep, refcount, grace window, repack, CDC) are in the
[README stub](README.md#glossary-stub); this file covers the per-system
vocabulary.

## Cross-cutting

- **Blob / block / chunk / object** — the unit of content-addressed storage. git
  calls it an *object*; IPFS a *block*; restic and casync a *chunk* (restic also
  says *blob*); iroh-blobs a *blob*. All mean "bytes named by their hash."
- **Root** — a named pointer GC must not collect from. See *ref*, *snapshot*,
  *pin*, *tag*, *index file* below.
- **Reachable / unreachable** — an object is reachable if a path from some root
  reaches it; mark-and-sweep deletes the unreachable.
- **Outboard** — verification (hash-tree) metadata stored *beside* the content
  rather than interleaved with it. iroh-blobs's term; restic's *index* and git's
  *.idx* play the analogous role.

## git

- **Loose object** — one zlib-compressed object file under `.git/objects/`,
  keyed by hash.
- **Packfile** — a `.pack` holding many objects (delta-compressed), with a
  sibling `.idx` offset index.
- **ref** — a branch/tag/HEAD pointer; a GC root.
- **reflog** — per-ref history of prior values; retains recently-detached
  objects for a grace window (`gc.reflogExpireUnreachable`, default 30 days).
- **cruft pack** — a pack holding unreachable objects with per-object mtimes in a
  parallel `.mtimes` file, written by `git repack --cruft`; avoids the
  loose-object explosion.
- **`git gc`** — the repack-plus-prune entry point.

## restic

- **CDC** — content-defined chunking; restic uses a **Rabin fingerprint** rolling
  hash over a 64-byte window, boundary when the low 21 bits are zero. Chunks
  512 KiB–8 MiB, ~1 MiB average; files <512 KiB unsplit.
- **pack file** — bundle of encrypted blobs with header at the end.
- **snapshot** — JSON doc referencing a root tree by hash; the *only* GC root.
- **`forget`** — removes snapshot files (roots).
- **`prune`** — mark-and-sweep from snapshots; reclaims and repacks.
- **`--max-unused`** — dead-fraction tolerance before repacking (default 5%).

## IPFS / boxo

- **CID** — content identifier (multihash + codec + version) naming a block.
- **blockstore** — the block-graph layer over a generic datastore.
- **pin** — retention root. *Recursive* (block + descendants), *direct* (one
  block), *indirect* (kept because an ancestor is recursively pinned).
- **MFS** — Mutable File System; its root is protected from GC ("implicit
  pinning").
- **GCBlockstore** — `Blockstore` + `GCLocker`.
- **GCLocker** — the lock interface: `PinLock()` (read lock; concurrent
  put→pin), `GCLock()` (write lock; exclusive collection), `GCRequested()`
  (fairness: GC is queued and waiting).
- **Unlocker** — the handle returned by `PinLock`/`GCLock`; release to drop the
  lock.

## casync

- **`.catar`** — linear serialization of a directory tree (tar-like).
- **`.castr`** — the content-addressed chunk store (a directory of
  hash-named, xz-compressed chunks).
- **`.caidx` / `.caibx`** — chunk index files (filesystem-tree / blob-image),
  listing the ordered chunk hashes that reconstruct the original; the retention
  root.
- **buzhash** — casync's rolling hash for CDC (defaults: min 16K / avg 64K /
  max 256K).

## iroh-blobs

- **BLAKE3** — the tree hash; root is the blob's 32-byte content hash.
- **Bao** — encoding packaging a blob + the hash-tree nodes for range
  verification. *Inline* (interleaved, wire form) vs *outboard* (sidecar, disk
  form).
- **HashSeq** — a blob whose contents are a sequence of 32-byte hashes; how
  iroh-blobs models a collection. A tag on a HashSeq is a recursive root.
- **tag** — the only retention primitive (no refcount/LRU/quota).
- **FsStore / MemStore** — the on-disk vs in-memory `Store` implementations;
  B-10 shipped on `MemStore`, `FsStore` is the deferred production wiring.

## Sources

- [Git internals — packfiles](https://git-scm.com/book/en/v2/Git-Internals-Packfiles)
- [restic design document](https://github.com/restic/restic/blob/master/doc/design.rst)
- [ipfs/boxo blockstore source](https://github.com/ipfs/boxo/blob/main/blockstore/blockstore.go)
- [casync (0pointer.net)](https://0pointer.net/blog/casync-a-tool-for-distributing-file-system-images.html)
- [`prior-art/iroh/blobs.md`](../iroh/blobs.md)
