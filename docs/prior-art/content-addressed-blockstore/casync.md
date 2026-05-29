**Date:** 2026-05-29
**Status:** active
**Subject:** casync — content-addressable chunk store (comparator); CDC without an online GC

# casync as a comparator

[casync](https://github.com/systemd/casync) ("Content-Addressable Data
Synchronizer") is Lennart Poettering's tool for distributing and updating large
filesystem images and directory trees. It is included here as a **comparator,
not a paradigm to adopt**: it shows the chunk-store idea taken to its extreme
(file boundaries erased before chunking) and, by contrast, shows what a
content-addressed store looks like with *no online garbage collector at all*.

- **Author / origin:** Lennart Poettering; first announced 2017-06-20
  ([0pointer.net blog](https://0pointer.net/blog/casync-a-tool-for-distributing-file-system-images.html)).
- **License:** LGPL-2.1.
- **Status:** hosted under the systemd org but independent of systemd; the
  best-maintained reimplementation is the Go [`desync`](https://github.com/folbricht/desync).

## The chunk store and file formats

casync serializes a directory tree into a linear `.catar` stream (like a tar),
then chunks that stream and stores the chunks in a **`.castr`** chunk store — a
flat directory of files each named by the SHA-256 of its (xz-compressed)
contents. An **index file** (`.caidx` for filesystem trees, `.caibx` for blob
images) lists the ordered chunk hashes that reconstruct the original. So:

- **chunk** = content-addressed, content-defined-sized unit; the dedup atom.
- **`.castr`** = the content-addressed store (the "blockstore").
- **`.caidx` / `.caibx`** = the manifest of chunk hashes (the "root").

## Content-defined chunking with buzhash

casync cuts chunk boundaries with the **buzhash** rolling hash (not restic's
Rabin fingerprint — same CDC idea, different hash). A boundary is declared when
the rolling hash matches a *discriminator* derived from the target average size.
Verified default chunk-size triple (from `src/cachunker.h` and the announcement):
`CA_CHUNK_SIZE_AVG_DEFAULT` is `64*1024`, and the `CA_CHUNKER_INIT` macro derives
min = avg/4 and max = avg*4:

| Parameter | Default |
|---|---|
| min chunk size | **16 KiB** (avg/4) |
| average chunk size | **64 KiB** |
| max chunk size | **256 KiB** (avg*4) |

(restic's averages are far larger — ~1 MiB — because restic backs up arbitrary
file content, while casync targets OS-image delta distribution where small
chunks expose more cross-version overlap. See [restic.md](restic.md).)

## The distinguishing trick: erase file boundaries first

casync chunks the *concatenated* `.catar` stream, so "small files are lumped
together with their siblings and large files are chopped into pieces," letting it
"recognize similarities in files and directories beyond file boundaries." This
maximizes dedup across two OS images that share most files but differ in a few.
The cost: a chunk is no longer "a file" — it is an arbitrary window of the
serialized tree, so chunk-level reasoning about *what* lives in a chunk is lost
(restic 0.18.0 leans into the same opacity as a privacy feature; see
[restic.md](restic.md)).

## No online GC — the deliberate omission

casync's chunk store is effectively **append-only and immutable**. There is no
`prune`, no pin lock, no reflog. Reclaiming space means: figure out which chunks
no index still references and delete the rest — an *external*, offline sweep the
operator runs, not a primitive the tool brokers. This is fine for casync's
deployment model (a server hosts the chunk store for image distribution; old
images are pruned by retiring their `.caidx` and sweeping) but it means casync
offers **no concurrent-GC-vs-serve story** — see
[concurrency-and-locking.md](concurrency-and-locking.md). That absence is exactly
why it is a comparator and not a template: a Myrhiza `FsStore` that serves blobs
to live peers *needs* the online, concurrency-safe GC that boxo provides
([ipfs-boxo.md](ipfs-boxo.md)) and casync declines.

## Myrhiza relevance

Two takeaways, both via [lessons.md](lessons.md):

1. **Borrow:** if the kernel ever stores *large assets* (not just small events),
   CDC-with-boundary-erasure is the right dedup model — but it trades away the
   ability to map a chunk back to an app-level object, which conflicts with
   per-event/per-author attribution. Likely the wrong fit for the event log,
   plausibly right for an app's bulk asset blobs.
2. **Avoid:** casync's "no online GC, sweep externally" model is a trap for a
   live P2P store. It works only because casync's stores are operator-managed and
   read-mostly. Myrhiza's store is read-write-and-serve concurrently, so an
   offline sweep is not an option.

## Sources

- [casync — A tool for distributing file system images (0pointer.net)](https://0pointer.net/blog/casync-a-tool-for-distributing-file-system-images.html)
- [systemd/casync repository](https://github.com/systemd/casync)
- [casync cachunker.h (chunk-size defaults)](https://github.com/systemd/casync/blob/main/src/cachunker.h)
- [folbricht/desync (Go reimplementation)](https://github.com/folbricht/desync)
