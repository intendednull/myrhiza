**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — `iroh-blobs` content-addressed blob transfer

# iroh-blobs

A QUIC-streamed, BLAKE3-verified, range-addressable blob transfer protocol. The "data plane" of an iroh deployment that wants to ship bytes between peers without trusting either end of the pipe.

## Versions

The repo lives at [`n0-computer/iroh-blobs`](https://github.com/n0-computer/iroh-blobs); not archived; actively developed. Two co-existing release lines as of 2026-05-08:

- **`0.35` (maintenance branch)** — last "production-quality" line. The README on `main` opens with: *"this version of iroh-blobs is not yet considered production quality. For now, if you need production quality, use iroh-blobs 0.35"* ([README, main](https://github.com/n0-computer/iroh-blobs/blob/main/README.md)).
- **`0.90`–`0.101` (rewrite line)** — the "store v2" rewrite. v0.90 published 2025-06-27, v0.100 published 2026-04-20, v0.101 published 2026-05-08 (dates from the [crates.io API](https://crates.io/crates/iroh-blobs)). The 0.100 line targets `iroh = "0.98"` ([Cargo.toml @ v0.100.0](https://github.com/n0-computer/iroh-blobs/blob/v0.100.0/Cargo.toml)).

If you pin against current iroh, you are pinning against a not-yet-production-quality library by the maintainers' own admission. Plan for that.

## Core model

Everything is content-addressed by a 32-byte BLAKE3 hash. The README's vocabulary is sparse and load-bearing:

- **Blob** — a sequence of bytes, no metadata.
- **Link** — the BLAKE3 hash of a blob. 32 bytes.
- **HashSeq** — a blob whose contents are a sequence of links. Length is always a multiple of 32. This is how iroh-blobs models "collections": a HashSeq is a blob whose payload is the hashes of other blobs.
- **Provider / Requester** — symmetric protocol roles; a node is usually both.

There are no manifests, no MIME types, no filenames at the blob layer. A "collection" is a `HashSeq` plus some application-defined convention; iroh-blobs ships one such convention (the historical `Collection` type) but does not require it.

## Bao and BLAKE3 verified streaming

BLAKE3 is a tree hash: the input is split into 1024-byte chunks, chunks are hashed pairwise into a binary Merkle tree, and the root is the file's hash ([BLAKE3 spec](https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf)). This tree structure is what makes "verified streaming" cheap: any contiguous byte range can be verified against the root with `O(log n)` sibling hashes alongside the data.

[**Bao**](https://github.com/oconnor663/bao) is the encoding format that packages this. iroh-blobs uses [`bao-tree`](https://crates.io/crates/bao-tree) (`v0.16` in iroh-blobs 0.100) to implement two modes:

- **Inline** — chunks and their inner hash-tree nodes are interleaved in a single byte stream the sender pushes over QUIC.
- **Outboard** — the content is stored unmodified on disk; an *outboard* file holds just the inner hash-tree nodes. Lets you serve verified streams of files you already had, without re-encoding.

The receiver verifies each chunk against the root hash *as bytes arrive*. If a chunk fails verification, the connection is torn down before any unverified bytes leak to application code. Sender mis-cooperation is detected, not tolerated.

The chunk granularity is also the granularity at which **range requests** work. A request describes data as `(blob hash, byte range set)`, the provider replies with the bytes plus enough hash-tree nodes to verify them. From the proto docs: *"verified streaming also facilitates range requests: fetching a verifiable contiguous subsequence of a blob by streaming only the portions of the BLAKE3 binary tree required to verify the designated subsequence"* ([docs.iroh.computer/protocols/blobs](https://docs.iroh.computer/protocols/blobs)). This generalizes to HashSeqs: you can request "blob #4 through #7 of this HashSeq, byte range `[1MB, 2MB)` of each."

## Wire protocol

QUIC, framed by `iroh::protocol::Router` ALPN dispatch (`iroh_blobs::ALPN`). Request shape is roughly: requester opens a stream, writes a request describing one or more `(hash, range_set)` tuples, provider streams Bao-encoded responses back on the same stream. Resumable because requests are by hash + range — re-asking for the missing tail of a partially-received blob just costs the missing range plus its verification path.

## Discovery: there isn't one

**iroh-blobs has no DHT, no gossip, no built-in announce.** The README says it plainly: *"Connection establishment is left up to the user or higher level APIs."* The canonical sharing primitive is a [`BlobTicket`](https://docs.rs/iroh-blobs/latest/iroh_blobs/ticket/struct.BlobTicket.html) — `(NodeAddr, Hash, BlobFormat)` — encoded as a string the human pastes somewhere. Out-of-band, by design.

If you want "find me a peer with hash `H`," you build it on top — typically with [`iroh-gossip`](./gossip.md) for discovery and iroh-blobs for the transfer. iroh-docs combines both into a higher-level abstraction; raw iroh-blobs does not.

## Tagging and GC

Blobs in the local store are kept alive by **tags** ([store.tag API](https://docs.rs/iroh-blobs/latest/iroh_blobs/store/index.html)). Importing data returns a tag; the GC sweeps any blob (or HashSeq-rooted subtree) not referenced by a live tag. Tags are the only retention primitive; there is no refcount, no LRU, no quota system at the iroh-blobs layer.

Implication: every component that wants persistent data must own its tags and clean them up. Forget to drop a tag and the GC won't reclaim the bytes; drop one too eagerly and a concurrent transfer can race against deletion. The 0.90+ rewrite line is partly motivated by getting these semantics tighter — `lazy-list-tags`, `experiment-api`, `get-error-refactor` branches in the repo all touch this surface.

## What's actually shipping right now

The 0.35 line is what production iroh deployments (Holochain 0.6+, Sneedlock, Dumbpipe, Number 0's own products) link against today. The 0.90+ line is where active development is, but the explicit "not production quality" warning on `main` means the API surface and on-disk format are still in flux. iroh-docs 0.99.0 (released today, 2026-05-08) pins `iroh-blobs = "0.101"` ([iroh-docs Cargo.toml @ v0.99.0](https://github.com/n0-computer/iroh-docs/blob/v0.99.0/Cargo.toml)), which means the iroh ecosystem itself is mid-migration.

## Implications for Myrhiza

App-bundle distribution maps cleanly onto iroh-blobs. A bundle is a `HashSeq` of `(wasm component, manifest, asset)` blobs; the BLAKE3 root *is* the bundle ID. Verified streaming gives you tamper-evident downloads from untrusted peers without a separate signature pass. Range requests give you "fetch only the components I don't already have" essentially for free.

The discovery gap is real, though. Myrhiza needs an answer to "given a bundle hash, find a peer with the bytes" — either layer iroh-gossip on top, build a DHT, or embed announce-on-publish into whatever Myrhiza uses for app distribution metadata. Holochain's lesson on [networking](../holochain/networking.md) applies: don't ship a "temporary" centralized fallback that becomes culturally permanent.

The 0.35-vs-rewrite split forces a pin decision at integration time. Default position: track the rewrite line so we move with the ecosystem, accept the API churn, but design Myrhiza's blob-distribution capability surface to be format-version-aware — bundles signed with their iroh-blobs major version so a future on-disk format break doesn't fork the network. The compounding risk is iroh's general absence of a published wire-format spec ([open-problems.md §9](./open-problems.md)): without a frozen spec to pin against, even a patch-level change in iroh-blobs's encoding can break inter-peer compatibility silently.

The tag-based GC is fine for local cache management but it is *not* a content-availability guarantee. If Myrhiza wants "this bundle stays available," that's a higher-layer pinning service, not a property of iroh-blobs.

## Sources

- [iroh-blobs repository](https://github.com/n0-computer/iroh-blobs)
- [iroh-blobs README on main](https://github.com/n0-computer/iroh-blobs/blob/main/README.md)
- [iroh-blobs releases](https://github.com/n0-computer/iroh-blobs/releases)
- [iroh-blobs Cargo.toml @ v0.100.0](https://github.com/n0-computer/iroh-blobs/blob/v0.100.0/Cargo.toml)
- [iroh-blobs protocol docs (docs.iroh.computer)](https://docs.iroh.computer/protocols/blobs)
- [iroh-blobs API docs (docs.rs)](https://docs.rs/iroh-blobs/latest/iroh_blobs/)
- [BLAKE3 specification](https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf)
- [Bao verified streaming](https://github.com/oconnor663/bao)
- [bao-tree crate](https://crates.io/crates/bao-tree)
- [Holochain networking — sibling prior-art doc](../holochain/networking.md)
- [iroh-docs Cargo.toml @ v0.99.0](https://github.com/n0-computer/iroh-docs/blob/v0.99.0/Cargo.toml)
- [iroh-gossip — sibling doc](./gossip.md)
