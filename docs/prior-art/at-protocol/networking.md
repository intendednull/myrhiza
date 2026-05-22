**Date:** 2026-05-22
**Status:** active
**Subject:** AT Protocol networking — XRPC, the firehose, repo sync v1.1, hosting requirements

# Networking

AT Protocol's wire side is conventional: HTTPS for RPC, WebSocket for the firehose, JSON or CBOR on the wire depending on endpoint. The interesting parts are the **firehose** (long-lived high-throughput event stream) and **repo sync** (how a Relay catches up a PDS it just started watching).

## XRPC — the RPC layer

XRPC is atproto's RPC-over-HTTP layer. It's roughly *"JSON-RPC with strong typing via Lexicon and conventions for read vs write vs subscribe."* See [lexicon.md](lexicon.md) for the schema side.

The pattern:

- **Method** = NSID like `com.atproto.repo.createRecord`.
- **Transport** = HTTPS (GET for query, POST for procedure) or WebSocket (for subscription).
- **Auth** = Bearer token (session JWT) or service-account JWT (for service-to-service).
- **Body** = JSON for HTTP, CBOR for some streaming endpoints.

URL shape:

```
GET  https://pds.example.com/xrpc/app.bsky.feed.getTimeline?limit=50
POST https://pds.example.com/xrpc/com.atproto.repo.createRecord (JSON body)
WSS  wss://relay.example.com/xrpc/com.atproto.sync.subscribeRepos?cursor=42
```

Convention: hostname is the operator's choice; path is `/xrpc/<NSID>`; method is determined by the Lexicon's primary type.

**Service proxying**: a PDS or AppView can advertise a `service` entry in the user's DID document, and clients can route XRPC calls via the user's PDS to other services. This is how clients talk to an AppView "through" their PDS for authenticated requests.

## The firehose — `com.atproto.sync.subscribeRepos`

The firehose is the load-bearing streaming primitive. Every state-changing event in the network flows through one of these:

- A user posts → PDS signs commit → PDS pushes to firehose → Relay merges into its own firehose → AppView reads.

The endpoint:

```
WSS wss://relay.example.com/xrpc/com.atproto.sync.subscribeRepos?cursor=<seq>
```

Each event is a CBOR frame containing:

- `seq` — monotonically increasing sequence number (consumer's cursor)
- `repo` — the user's DID
- `commit` — CID of the new commit
- `prev` — CID of the prior commit
- `blocks` — a CAR-format bundle of the new blocks (commit + MST diff + new records)
- `ops` — list of operations (`create`, `update`, `delete` with NSID + record key)
- `time` — server-side timestamp (informational, not load-bearing)

The consumer (Relay subscribing to a PDS, or AppView subscribing to a Relay) replays events in `seq` order and reconstructs the materialized view it cares about.

### Sync v1.1 (May 2025)

The current firehose protocol is **Sync v1.1**, rolled out May 2025 alongside new Relay deployments at `relay1.us-west.bsky.network` and `relay1.us-east.bsky.network`. The full cutover to v1.1 on the main `bsky.network` domain happened **January 27, 2026** (per the "Upcoming Relay Transition" blog post). Consumers were warned to expect WebSocket reconnections and "modest event duplication" during the transition.

What v1.1 changed (vs v1.0):

- **Cursor stability across Relay restarts.** v1.0 cursors were tied to a specific Relay instance's internal sequence; restarts caused cursor invalidation. v1.1 uses a more durable sequencing model.
- **Account events** — separate stream of account-status events (active, takendown, suspended, deactivated) that previously rode inside the repo firehose.
- **Cleaner backfill semantics** — consumers requesting older events get a deterministic answer or a "data is gone" indicator, rather than silent empty results.

Bluesky has been deliberate about firehose churn: this is the second wire-protocol revision since launch, and the v1 → v1.1 transition was telegraphed months in advance with a January 2026 cutover date. Production-grade discipline.

## Repository sync

Two patterns matter:

### Incremental (steady-state)

Consumer is caught up; PDS streams new commits as they happen via the firehose. Each event carries the new blocks via the inline CAR-formatted `blocks` field. Consumer applies blocks, advances cursor.

### Backfill (cold-start or recovery)

Consumer needs to catch up from cursor 0 (or from a stale cursor older than the Relay's retention window).

Two endpoints:

- **`com.atproto.sync.getRepo`** — returns the full repo as a CAR file. Used when the consumer wants a fresh snapshot.
- **`com.atproto.sync.getBlocks`** — returns a specific set of CIDs as a CAR file. Used for selective sync.

Because the MST is content-addressed, backfill is naturally diff-able: consumer fetches the current root CID, compares to its known root, fetches only the differing subtrees. The MST's deterministic structure makes this clean.

## Account migration over the wire

Migration is mostly choreography rather than new protocol:

1. **Identity preparation** — user submits a PLC operation to update the DID document's `service` entry to point to the new PDS.
2. **CAR export** — old PDS produces a CAR of the user's repo via `com.atproto.sync.getRepo`.
3. **Import on new PDS** — new PDS validates the CAR (signatures, MST integrity), commits its blocks to storage.
4. **Re-sign** — once the new PDS is the authoritative writer (per the updated DID document), all future commits are signed by the new PDS's instance of the `#atproto` signing key.
5. **Blob transfer** — separate endpoint for non-record binary data.

The new PDS validates the *old* commits by walking the chain back; it doesn't re-sign them. The migration model assumes the user's signing key transitions with them (which is why the rotation key is the gating credential for migration).

## Hosting requirements

This is where the federation honesty lives:

| Role | Minimum hardware | Storage trajectory | Operator cost |
|---|---|---|---|
| **PDS (1-20 users)** | 1 vCPU, 1 GB RAM, 20 GB SSD | ~1 GB/user/year | $5-10/mo VPS |
| **PDS (~1k users)** | 2 vCPU, 4 GB RAM, ~1 TB SSD | linear with users | $50-200/mo |
| **Relay (full firehose)** | Multi-core, 32-64 GB RAM, **multi-TB SSD** | grows with global activity | hundreds-to-thousands/mo |
| **AppView (bsky.app-scale)** | Full datacenter footprint | full materialized indices | enterprise |

PDS hosting is **genuinely accessible**: a person with a domain and a small VPS can host themselves and friends for the cost of coffee. Bluesky publishes a self-host install script for Ubuntu 24.04 that gets you running in under an hour.

Relay hosting is **prohibitively resource-heavy for individuals**. The firehose carries every commit from every PDS on the network — terabytes of storage, sustained gigabit-class throughput at scale. As of early 2026 there are a small number of independent Relay operators (the "Northsky" Relay is one publicly known third-party operator), but the tier remains structurally centralized. This is the load-bearing constraint that keeps atproto's federation story closer to ActivityPub-with-tiers than to a true P2P network.

See [federation.md](federation.md) for the honest accounting.

## Why no QUIC, no libp2p, no DHT?

A natural question for a Myrhiza reader: atproto could have used QUIC or libp2p or a DHT for its transport. It uses HTTPS+WebSocket. Why?

The published rationale is operational: HTTPS+WebSocket is the substrate every CDN, every load balancer, every browser, every mobile network already supports. Bluesky's design priority was **getting deployed quickly with off-the-shelf infrastructure**, not designing a novel transport. The cost is the federation centralization story — HTTPS-rooted services *can* be self-hosted but practically require operations skill, and the Relay tier's bandwidth requirements push toward big operators.

For Myrhiza, the relevant lesson is **the inverse**: P2P transports (iroh, libp2p) make it *harder* to bootstrap with off-the-shelf infrastructure but *easier* to maintain peer-symmetric topology. AT Protocol picked the deployment-friendly side of that trade-off and got Bluesky's federation centralization story as the structural consequence. Myrhiza is picking the other side and inheriting the bootstrap-complexity consequence.

## Sources

- XRPC spec: <https://atproto.com/specs/xrpc>
- Firehose / subscribeRepos spec: <https://atproto.com/specs/sync>
- Lexicon `com.atproto.sync`: <https://github.com/bluesky-social/atproto/tree/main/lexicons/com/atproto/sync>
- Sync v1.1 blog: <https://docs.bsky.app/blog/relay-sync-v1.1>
- Upcoming Relay Transition (2026-01): docs.bsky.app blog January 2026
- PDS self-host requirements: <https://github.com/bluesky-social/pds>
- Northsky third-party Relay: community-operated, see <https://github.com/blacksky-algorithms> for one known example
