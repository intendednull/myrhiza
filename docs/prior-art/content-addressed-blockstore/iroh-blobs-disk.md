**Date:** 2026-05-29
**Status:** active
**Subject:** iroh-blobs on-disk store — wire-vs-disk distinction, outboard format, tag GC, the store v2 rewrite

# iroh-blobs on disk

This file covers iroh-blobs from the **persistence** angle, complementing the
wire-side coverage in [`prior-art/iroh/blobs.md`](../iroh/blobs.md). Read that
file first for the BLAKE3 / Bao / verified-streaming / HashSeq model; this one is
strictly about what the bytes look like on the disk a Myrhiza `FsStore` would
own. iroh-blobs is a **hard dependency** (B-10 shipped its fetch path on
`MemStore`; `FsStore` is the deferred production wiring) — so unlike the other
four systems here, its disk model is something Myrhiza commits to, not merely
learns from.

> Versions and the "not yet production quality" caveat are documented canonically
> in [`prior-art/iroh/blobs.md`](../iroh/blobs.md) §Versions and are **not
> repeated here** to keep one source of truth. Short version: the `0.90`–`0.101`
> "store v2" rewrite line is where the on-disk format lives and is mid-flux; the
> README on `main` still tells production users to use `0.35`. The B-10 spec pins
> `iroh-blobs = "=0.101.0"`, which targets `iroh = "=1.0.0-rc.0"` (verified in
> [`b-10` spec](../../specs/2026-05-26-b-10-bundle-distribution-design.md) §4.7).

## The wire-vs-disk distinction (the reason this file exists)

BLAKE3 is a tree hash: the input is split into 1024-byte chunks hashed pairwise
into a binary Merkle tree whose root is the blob's 32-byte hash. **Bao** is the
encoding that packages a blob plus the tree nodes needed to verify any byte
range. iroh-blobs uses this two ways
([`prior-art/iroh/blobs.md`](../iroh/blobs.md) §Bao):

- **Inline (wire form)** — data chunks and their inner hash-tree nodes are
  *interleaved* in one byte stream the sender pushes over QUIC. This is the
  format optimized for transfer: the receiver verifies each chunk against the
  root as bytes arrive and tears down the connection on the first mismatch.
- **Outboard (disk form)** — the content is stored **unmodified** on disk, and a
  separate *outboard* file holds just the inner hash-tree nodes. This lets the
  store serve verified streams of files it already had without re-encoding them.

This is the load-bearing distinction for Myrhiza: **the bytes on disk are not the
bytes on the wire.** On disk a blob is (data file, outboard file); on the wire it
is one interleaved Bao stream. A `FsStore` is therefore not "save the wire bytes"
— it is "store the content verbatim + an outboard sidecar, re-derive the wire
stream on demand." This decoupling is what makes range requests and resumable
partial fetches cheap, and it is the same idea as restic's separate index files
and git's separate `.idx` — verification metadata lives beside, not inside, the
content.

## Incomplete / partial blobs

Because fetches are range-addressable and resumable, the on-disk store must track
*which ranges of a blob it actually has*. A partially-downloaded blob is a real
on-disk state, not an error: re-asking for the missing tail costs only the
missing range plus its verification path
([`prior-art/iroh/blobs.md`](../iroh/blobs.md) §Wire protocol). Any Myrhiza
`FsStore` inherits this — it must distinguish "complete verified blob" from
"partial blob with a known set of present ranges," and GC must treat a partial
blob that a download is actively filling as live
([concurrency-and-locking.md](concurrency-and-locking.md)).

## Tags are the only retention primitive

iroh-blobs keeps blobs alive by **tags**. Importing data returns a tag; GC
sweeps any blob (or HashSeq-rooted subtree) not referenced by a live tag. There
is **no refcount, no LRU, no quota** at the iroh-blobs layer
([`prior-art/iroh/blobs.md`](../iroh/blobs.md) §Tagging-and-GC). A HashSeq tag is
a recursive root: tagging the seq keeps all member blobs (the same
recursive-pin semantics as [ipfs-boxo.md](ipfs-boxo.md)). See
[retention-and-roots.md](retention-and-roots.md).

The hazard the iroh folder already flags: "drop [a tag] too eagerly and a
concurrent transfer can race against deletion" — i.e. iroh-blobs at the tag layer
does **not** give you boxo's `GCLocker` guarantee for free; the 0.90+ rewrite is
partly motivated by tightening exactly these semantics. A Myrhiza `FsStore`
cannot assume the underlying store serializes GC against in-flight serves; it
must own that discipline. See [lessons.md](lessons.md).

## Myrhiza relevance

- B-10 already maps app bundles onto HashSeqs with a per-bundle retention tag
  (`bundle/<manifest_hash>`), kept alive while installed, dropped on uninstall
  ([b-10 spec](../../specs/2026-05-26-b-10-bundle-distribution-design.md) §4.3).
  That is the tag-as-root model in production. LRU / quota / refcount discipline
  is explicitly deferred to the B-9 storage layer.
- The `FsStore` decision is the on-disk counterpart of that already-shipped wire
  path: same content addressing, same Bao verification, but now with the
  data+outboard disk layout and a GC that must be concurrency-safe against the
  sync/serve paths.

## Sources

- [`prior-art/iroh/blobs.md`](../iroh/blobs.md) — canonical iroh-blobs wire-side doc (versions, Bao, tags)
- [iroh-blobs repository](https://github.com/n0-computer/iroh-blobs)
- [Bao verified streaming](https://github.com/oconnor663/bao)
- [B-10 bundle distribution design](../../specs/2026-05-26-b-10-bundle-distribution-design.md)
