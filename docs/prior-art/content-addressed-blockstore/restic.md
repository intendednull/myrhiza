**Date:** 2026-05-29
**Status:** active
**Subject:** restic — content-defined chunking, pack files, snapshots-as-roots, forget vs prune, repack

# restic as a content-addressed blockstore

restic is a deduplicating backup tool whose repository is a content-addressed
blob store with an explicit mark-and-sweep GC (`prune`). It is a production
example of "content-defined chunking + pack files + reachability GC from snapshot
roots," and a close analog to a store that wants to dedup *across*
events/snapshots rather than store each whole.

- **License:** BSD-2-Clause (verified — `LICENSE` opens "BSD 2-Clause License";
  authored by Alexander Neumann, 2014).
- **Current release:** 0.18.1 (2025-09-21); 0.18.0 (2025-03-27) was the prior
  feature release (verified via the GitHub releases page).

## Content-defined chunking (CDC)

restic splits file data into **variable-length chunks** at boundaries chosen by
the content itself, using a **Rabin fingerprint** rolling hash over a 64-byte
sliding window. A new chunk boundary is declared when the low bits of the
fingerprint are zero; the [chunker
source](https://github.com/restic/chunker/blob/master/chunker.go) pins this at
**20 bits** by default (`splitmask: (1 << 20) - 1, // aim to create chunks of 20
bits or about 1MiB on average`). (restic's original 2015 CDC blog post said "21
bits"; the shipped default is 20 — cite the source, not the blog.) Verified
bounds from the
[design doc](https://github.com/restic/restic/blob/master/doc/design.rst):

> "Files smaller than 512 KiB are not split, Blobs are of 512 KiB to 8 MiB in
> size. The implementation aims for 1 MiB Blob size on average."

The chunker's polynomial is **randomly chosen per-repository at init** and stored
in the config, to frustrate watermark/chunk-fingerprinting attacks. restic 0.18.0
added a further mitigation: chunks are randomly assigned to pack files so an
observer can't infer which file a chunk came from.

Why CDC matters: editing the middle of a large file re-chunks only the affected
region; the surrounding chunks keep their hashes and dedup against the prior
backup. A whole-object store (git, iroh-blobs) cannot dedup a 1-byte edit to a
1 GB blob — see [gc-strategies.md](gc-strategies.md) and [casync.md](casync.md),
which uses the same CDC idea with a different rolling hash (buzhash).

## Pack files

Chunks (called **blobs** in restic) are bundled into **pack files**. The pack
layout puts the header at the *end*:
`EncryptedBlob1 || … || EncryptedBlobN || EncryptedHeader || Header_Length`.
This lets restic stream blobs into a pack as they are produced during backup,
without rewriting once the pack closes. Each blob is independently encrypted
(AES-256-CTR + Poly1305-AES MAC) and the blob type (data / tree, plus zstd-
compressed variants in repo format v2) is a one-byte tag. An **index** maps each
blob hash → `(pack, offset, length)`.

This is the same "many small objects packed into a larger immutable container"
pattern as git packfiles ([git.md](git.md)) and iroh-blobs's store, and it
forces the same repack problem: deleting one dead blob means rewriting the whole
pack ([compaction-and-repack.md](compaction-and-repack.md)).

## Snapshots are the GC roots

A **snapshot** is a JSON document referencing a root **tree** blob by hash, plus
timestamp/paths/metadata. Trees reference sub-trees and data blobs by hash. So
the reachable set is: walk every snapshot → its root tree → all descendant
trees and data blobs. Snapshots are the *only* GC roots — nothing else holds a
blob alive. This is exactly the pin/root model of [ipfs-boxo.md](ipfs-boxo.md)
and the tag model of [iroh-blobs-disk.md](iroh-blobs-disk.md); see
[retention-and-roots.md](retention-and-roots.md).

## `forget` vs `prune` — the two-phase delete

This split decouples root-removal from byte-reclamation:

- **`forget`** removes *snapshot* files (the roots). It is cheap and reversible-
  ish until prune runs. After `forget`, the blobs those snapshots referenced may
  now be unreachable — but they are still on disk.
- **`prune`** does the actual mark-and-sweep: "All snapshots and directories
  within snapshots are scanned to determine which data is still in use," then
  every pack is classified as fully-used (keep), completely-unused (delete), or
  partially-used (keep or repack). This is **explicit reachability analysis, not
  reference counting** — snapshots are the only roots.

Decoupling root-removal from space-reclamation means the expensive sweep runs on
its own schedule, and a too-eager `forget` can be caught before `prune` makes it
permanent. Myrhiza's "truncate the log past a snapshot" operation
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§"snapshot-as-bootstrap with log-pruning", ~line 200) is the same shape:
removing the root (old log tail) is a separate decision from reclaiming the
bytes.

## Repacking partially-used packs + key knobs

Verified `prune` parameters ([forget/prune
docs](https://restic.readthedocs.io/en/stable/060_forget.html)):

- **`--max-unused`** (default **5%**) — how much dead data restic tolerates
  leaving in place rather than repacking. This is a deliberate
  write-amplification ceiling.
- **`--max-repack-size`** — cap on bytes eligible for repack in one run.
- **`--repack-cacheable-only`** — only repack metadata.

Repacking a partial pack means downloading it, extracting the live blobs, and
re-uploading them in a new pack — "bandwidth-intensive, particularly for remote
storage." A client removing data must hold an **exclusive repository lock**, and
a pack must be removed from the referencing index *before* the pack itself is
deleted (write-ordering invariant; see [concurrency-and-locking.md](concurrency-and-locking.md)).

## Sources

- [restic design document](https://github.com/restic/restic/blob/master/doc/design.rst)
- [restic chunker source (`chunker.go`, splitmask = 20 bits)](https://github.com/restic/chunker/blob/master/chunker.go)
- [restic forget/prune documentation](https://restic.readthedocs.io/en/stable/060_forget.html)
- [restic Introducing CDC blog post (2015; says "21 bits", superseded by 20-bit default)](https://restic.net/blog/2015-09-12/restic-foundation1-cdc/)
- [restic releases](https://github.com/restic/restic/releases)
- [restic LICENSE](https://github.com/restic/restic/blob/master/LICENSE)
