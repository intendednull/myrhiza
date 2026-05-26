**Date:** 2026-05-22
**Status:** active
**Subject:** AT Protocol federation architecture — the PDS / Relay / AppView trio, repos, MST, and CAR

# Architecture

AT Protocol partitions the network into three roles. Every user belongs to exactly one PDS at a time; every consuming app talks to an AppView; Relays sit in between and fan out the firehose. The trio is the architectural primitive: changing how data flows through it is changing the protocol.

```
+----------------+         +---------+         +--------------+
|  Personal Data |  push   |  Relay  |  push   |   AppView    |
|  Server (PDS)  | ----->  |  (a.k.a |  ----> |  (e.g.        |
|  hosts repos   |         |  former |         |  bsky.app)   |
|  signs commits |         |  "BGS") |         |  serves feeds|
+----------------+         +---------+         +--------------+
        ^                                              |
        | XRPC writes (login, post, like)              | XRPC reads
        | XRPC reads (own data)                        |
        +------------------ Client (app) <-------------+
```

Each role is small enough to describe in one paragraph; the interesting design tension is **who can run what**, which is covered in [federation.md](federation.md).

## Personal Data Server (PDS)

The PDS hosts the user's repository and signs commits on their behalf. From atproto's glossary:

> *"A server that hosts a user. It may assign handles and DIDs, and syncs repos with Relays."*

What lives on the PDS:

- The user's **repository** (collection of typed records — posts, likes, follows, etc.) in a Merkle Search Tree (see §Repository below).
- The user's **`#atproto` signing key**, held in the PDS's keystore.
- The user's **blob storage** (images, videos, anything not inline JSON).
- The user's **PLC operations** (if `did:plc`) — proposed but signed by the rotation key, not the PDS.

What the PDS does:

- **Authoritative writes.** All record additions, edits, and deletions go through the PDS, which signs the commit.
- **Outbound sync to Relay.** The PDS pushes new commits to Relays via the `com.atproto.sync.subscribeRepos` firehose (sync v1.1 as of May 2025).
- **Read serving for the user's own client.** Clients query the user's PDS for their own data via XRPC.
- **Account management.** Handle assignment, password recovery, app passwords, OAuth.

Hardware footprint (per [`bluesky-social/pds`](https://github.com/bluesky-social/pds)): **1 vCPU, 1 GB RAM, 20 GB SSD for 1-20 users** on Ubuntu 24.04. Self-hosting is genuinely accessible at this scale. Bluesky's own PDS shards each host much more aggressively.

The PDS is the **least centralization-prone tier** — running one for yourself and 19 friends is a weekend project. As of 2026 there are hundreds of independent PDSes in operation, though they collectively host ~1% of users.

## Relay

The Relay aggregates repos from many PDSes and produces a unified firehose of commit events. From the glossary:

> *"An aggregator of data repos from across the Atmosphere. It syncs repos from PDSes and produces change events that AppViews use to fetch user data."*

Until November 2023 this was called the **Big Graph Service (BGS)**. The rename announcement ([`atproto/discussions/1847`](https://github.com/bluesky-social/atproto/discussions/1847), 2023-11-10) called BGS "a placeholder name" that "creates extra confusion and friction." The Go implementation still lives at `bluesky-social/indigo/cmd/bigsky` for legacy reasons.

What the Relay does:

- **Subscribes to many PDSes via `com.atproto.sync.subscribeRepos`** — the firehose, a long-lived WebSocket emitting CBOR-encoded commit events.
- **Validates commits** — checks signature, MST integrity, that the commit chain is well-formed.
- **Reproduces the firehose downstream** — AppViews and any other consumer subscribe to the Relay's own firehose, which is a merged stream of all PDSes it watches.
- **Stores backfill data** — a Relay holds enough recent commits to let new subscribers catch up; older data may be pruned or kept depending on operator policy.

Hardware footprint: **terabytes of storage**, sustained high write throughput. As of early 2026, Bluesky operates the primary public Relays at `relay1.us-west.bsky.network` and `relay1.us-east.bsky.network` (transitioned January 2026 from the older `bsky.network` endpoint).

**This is the centralized tier.** Only a handful of independent operators run a Relay because the resource cost is prohibitive for individuals and small organizations. See [federation.md](federation.md) for the honest numbers.

## AppView

The AppView assembles user-facing experiences from the firehose. From the glossary:

> *"An application in the Atmosphere that aggregates data from across the network. It communicates with PDSes to publish information and functions similarly to search engines on the web."*

The dominant AppView is `bsky.app` — Bluesky's own Twitter-shaped feed product. It is *also* operated by Bluesky PBC. Other AppViews exist but serve narrower audiences:

- **Statusphere** — status/availability indicators
- **Whitewind** — long-form blog posts
- **Smoke Signal** — events and RSVPs
- **Bluesky-specific AppViews** for video, search, suggestions

What the AppView does:

- **Subscribes to a Relay's firehose** (or directly to many PDSes — uncommon).
- **Filters and indexes commits** relevant to its app (`bsky.app` only cares about `app.bsky.*` records; Whitewind only cares about `com.whtwnd.blog.entry`).
- **Builds materialized views** — feeds, follower counts, search indices, etc.
- **Serves clients via XRPC** for reads.
- **Forwards writes to the user's PDS** — the AppView never writes; it instructs the client to issue the write to the user's home PDS.

AppViews are stateless with respect to identity — they don't custody keys, they don't sign anything, they consume the firehose and emit derived data. The AppView tier is **the most pluggable**: any developer can build one. The barrier is product-market fit, not protocol.

## Repository (data repo)

Each user has one repository. From the glossary:

> *"The public dataset which represents a user, comprised of collections of JSON records and unstructured blobs, identified by a single permanent DID."*

Structure:

- **Path = `<collection>/<record-key>`** where `<collection>` is an NSID (e.g. `app.bsky.feed.post`) and `<record-key>` is a TID (timestamp-id, base32-encoded clock).
- **Values are CBOR-encoded records** validated against their Lexicon schema (see [lexicon.md](lexicon.md)).
- **Storage backend: Merkle Search Tree** (see §MST below).
- **Mutations recorded via signed commits** — each commit is a CBOR object with `did`, `data` (MST root CID), `rev` (logical clock), and `sig` (ECDSA signature by the `#atproto` signing key).

The repository is **append-mostly**: deletes are real (records removed from the MST), but the commit chain references prior roots by CID so historical state is recoverable via Relay-stored backfill.

## Merkle Search Tree (MST)

The MST is atproto's content-addressed storage primitive. Properties:

- **Deterministic from content.** *"The overall structure and shape of the MST is deterministic based on the current key/value content."* This is critical: two PDSes given the same set of records will produce byte-identical MSTs.
- **Depth derived from key.** Each key is hashed with SHA-256 and the leading-binary-zeros count (divided by 2) becomes the node's depth in the tree. This avoids the rebalancing problems of B-trees while giving probabilistic balance.
- **Key prefix compression** within each node — common prefixes are elided.
- **Stable structure under inserts.** Adding a record at depth N only touches nodes at depth N and above; lower-depth subtrees stay byte-identical.
- **CIDs everywhere.** Each MST node is identified by the CID (multihash + multicodec) of its CBOR serialization. The root CID is the canonical content hash of the whole repository.

The MST is **the single most Myrhiza-relevant data-structure choice** atproto made. It gives:

- **Determinism** that survives independent reconstruction (a Relay rebuilding a repo from individual commits produces byte-identical MST nodes to the PDS).
- **Diff-friendly sync** — two parties can compare root CIDs, then walk the tree exchanging just the changed subtrees.
- **Verifiable proofs** — a Relay can serve a Merkle proof that record `R` is or isn't in the repo without trusting the AppView.

If Myrhiza's snapshot-portability story (open problem in `prior-art/willow/open-problems.md`) wants a content-addressed canonical form, MST is a strong candidate. See [lessons.md](lessons.md) §"Borrow: MST".

## CAR (Content Addressable aRchive) files

A repository is exported as a **CAR v1 file** — IPFS's standard format for a self-contained DAG of CBOR blocks. The CAR contains:

- The current signed commit (root block).
- All MST nodes reachable from the commit.
- All record blocks reachable from the MST leaves.

CAR files are how account migration works: the new PDS imports the old PDS's CAR and (after the user signs a PLC update) takes over hosting. See [identity.md](identity.md) §"Account migration".

## Signed commits

Each commit object contains:

| Field | Purpose |
|---|---|
| `did` | The user's DID — bound to the commit |
| `version` | Commit format version (`3` currently) |
| `data` | CID of the MST root |
| `rev` | Revision (logical clock, monotonic TID) |
| `prev` | CID of the prior commit (null for the genesis commit) |
| `sig` | ECDSA signature over the CBOR-serialized unsigned commit |

The `sig` is generated by the PDS using the user's `#atproto` signing key. Relays validate the signature on receipt; AppViews trust the Relay's validation.

**Why the commit chain matters**: it's the audit log for the repository. A reader who has the genesis commit and can resolve all commits to the head has cryptographic proof that the PDS (or anyone with the signing key) authored every record. Combined with the PLC operation log (which authorizes which key can sign), this gives end-to-end identity-to-content traceability.

## XRPC

The wire protocol is **XRPC** — atproto's RPC-over-HTTP layer. It's roughly JSON-RPC with conventions baked in for query vs procedure vs subscription, Lexicon-validated request/response types, and Bearer-token auth.

XRPC supports three patterns:

- **Query** — idempotent read; GET request.
- **Procedure** — mutating call; POST request.
- **Subscription** — long-lived WebSocket; used by the firehose.

Every endpoint is identified by an NSID (`com.atproto.repo.createRecord`, `app.bsky.feed.getTimeline`) and has a Lexicon schema defining the request and response shapes. Clients are usually code-generated from the Lexicon.

## Implications for Myrhiza

What atproto's architecture gets right (worth borrowing):

- **MST as deterministic content-addressed storage.** Almost exactly what Myrhiza's state-apply wants for snapshot canonicalization.
- **Lexicon schemas + NSID namespacing** for typed records on a shared storage substrate (see [lexicon.md](lexicon.md)).
- **Signed commit chains** as the audit log primitive — each commit references prior by CID and is signed.
- **CAR files for repository portability** — content-addressable export means "import" is just "unpack and verify."

What it gets wrong for Myrhiza's purposes:

- **The PDS is a single point of write authority.** A Myrhiza user can't write to their own repo from two devices simultaneously without coordination; the PDS serializes writes. In a peer-symmetric setting this becomes a multi-writer CRDT problem the PDS architecture sidesteps by making one server the authority.
- **The Relay tier is the centralization bottleneck.** Even if Myrhiza wanted a "fan out the firehose" service, doing it without a single big operator requires either DHT-like content addressing or replicated gossip — neither of which atproto attempts.
- **AppViews are read-only consumers.** Atproto cleanly separates "who can write to your repo" (only your PDS) from "who shows you content" (any AppView). Myrhiza's peer-symmetric model collapses these — every peer is both writer and reader of its own data. That's a strength (no AppView lock-in) and a constraint (no specialization).

## Sources

- AT Protocol overview: <https://atproto.com/guides/overview>
- AT Protocol glossary: <https://atproto.com/guides/glossary>
- Federation architecture: <https://docs.bsky.app/docs/advanced-guides/federation-architecture>
- Repository spec: <https://atproto.com/specs/repository>
- BGS-to-Relay rename: <https://github.com/bluesky-social/atproto/discussions/1847>
- Indigo Go relay implementation: <https://github.com/bluesky-social/indigo>
- PDS self-host repo: <https://github.com/bluesky-social/pds>
- Bluesky PDS hosting on community wiki: <https://atproto.wiki/en/wiki/reference/core-architecture/pds>
- Relay Sync v1.1 blog (2025-05): <https://docs.bsky.app/blog/relay-sync-v1.1>
- Upcoming Relay Transition (2026-01): docs.bsky.app blog announcement of January 27, 2026 cutover
