**Date:** 2026-06-08
**Status:** active
**Subject:** In-state topic enumeration (Matrix spaces), in-overlay peer sampling (HyParView/Plumtree), and centralized contrasts (Nostr NIP-65, AT Proto relay/Jetstream)

# In-band enumeration, peer sampling, and centralized contrasts

The decentralized DHT layer ([dht-rendezvous.md](./dht-rendezvous.md)) answers "find peers for a key you already hold." This file covers the *other* discovery question — **how do you learn the key exists at all** — and the in-overlay peer-finding that happens *after* bootstrap.

## In-state enumeration: learn child topics from a parent topic's events

The pattern most relevant to Myrhiza's content-addressed IDs. You don't discover topic IDs from a global directory — you **read them out of the state of a topic you're already in.** A parent topic's events enumerate its child topic IDs.

**Matrix spaces** are the canonical worked example. A space is a room whose state contains **`m.space.child`** state events; each one's `state_key` is the **id of a child room/space**, and its content carries a `via` key — "a list of candidate servers that can be used to join the room." A client walks the hierarchy by recursing: find a room of type `m.space`, read its `m.space.child` events, recurse into each child, repeat. `"suggested": true` hints eager display. (See sibling [`matrix-state-resolution`](../matrix-state-resolution/) for Matrix's broader model.)

Two things transfer directly:
1. **The child ID is opaque** (a room id, here a server-generated string; for Myrhiza, a BLAKE3 hash). It does not need to be human-readable because you never type it — you learn it from a parent you already trust/follow.
2. **The enumeration carries a bootstrap hint** (`via` servers). Matrix bundles *what to join* with *who to join through* in the same event. This is exactly the "topic ID + bootstrap pointer" pairing Myrhiza needs.

The "server topic lists its channel topics" design in Myrhiza's `host.subscribe` brief is structurally identical to `m.space.child`: the server's deterministic state enumerates channel topic IDs; an interaction component reads them and subscribes. This keeps topic *enumeration* inside convergent state, while keeping the *subscription act* non-deterministic and peer-local.

## In-overlay peer sampling: HyParView + Plumtree (after you're in)

iroh-gossip's membership is **HyParView** (peer sampling) + **Plumtree** (broadcast). Once you reach *one* member of a topic, these find you the rest — no further external discovery:

- **HyParView** keeps two partial views: a small **active view** (live connections / neighbors used for broadcast) and a larger **passive view** (a backup pool). On an active-peer failure, a passive peer is promoted. Periodic **shuffle** exchanges refresh the passive view, so the overlay self-heals under churn.
- **Plumtree** builds epidemic broadcast trees over those active links — eager-push along the tree, lazy-pull (IHAVE) to repair gaps. It handles *message passing*; HyParView handles *topology*.

Key boundary for Myrhiza: peer sampling solves "find *more* peers on a topic I'm *already* on." It does **not** solve cold-start ("find the *first* peer"). That is why iroh-gossip still demands external bootstrap peers (see [iroh-discovery.md](./iroh-discovery.md)). Peer sampling is in-sandbox-irrelevant: it lives in the kernel/transport, below `host.subscribe`.

## Centralized contrast 1: Nostr NIP-65 (outbox model)

Nostr's discovery is **social + relay-centric**, the antithesis of content-addressing. **NIP-65** (`kind:10002`, verified) is a replaceable event listing a user's relays as `r` tags: `["r","wss://example.com"]` with optional `"read"`/`"write"` marker (omitted = both). The **outbox model**: to read user U's posts, fetch U's `kind:10002`, then query U's *write* relays — not the whole network.

Lesson by contrast: Nostr's "topic" is effectively *a person*, discovered via the **follow graph** (you know whose feed you want), and resolution is *which relays they write to*. There is no content-addressed topic and no DHT — discovery rides entirely on social knowledge + named relay URLs. Myrhiza cannot borrow the mechanism (human npubs, URL relays) but should note the structural move: **publish your own "where to find me" pointer**, and let followers resolve it. That is the pkarr/`m.space.child` move in a centralized dress.

## Centralized contrast 2: AT Protocol relay / firehose / Jetstream

AT Proto inverts the problem entirely: **no per-topic discovery at all** — a **relay** crawls every PDS, aggregates all repo updates into one global **firehose**, and consumers filter client-side. **Jetstream** (Jaz, Sept 2024) is a lightweight JSON variant that filters by collection and shrinks bandwidth ~99% (≈850 MB/day to tail all of Bluesky). The main relay is `bsky.network`.

Trade-off (verified): per the official atproto Jetstream docs, Jetstream events "do not include cryptographic signatures or Merkle tree nodes, meaning the data is not self-authenticating" — you trust the relay. This is the firehose anti-pattern for Myrhiza: it scales discovery by centralizing *everything* into one trusted aggregator and dropping verifiability. Myrhiza's per-topic convergent state-apply is the deliberate opposite — discovery must stay per-topic and verifiable, never a trust-the-aggregator global stream. (See sibling [`at-protocol`](../at-protocol/).)

## Synthesis for Myrhiza

- **Topic-ID discovery** → favor **in-state enumeration** (Matrix `m.space.child` shape): learn child topic IDs from a parent topic's convergent state. Out-of-band sharing (a pasted ticket/bundle hash) covers the *root* entry.
- **Topic→peers** → DHT announce/lookup or rotating-key BEP44 (decentralized), with discovery resolving NodeID→addr. Peer sampling takes over post-bootstrap.
- Avoid the centralized shortcuts (single relay/firehose) — they trade away the per-topic verifiability Myrhiza's convergence depends on.

## Sources

- https://github.com/matrix-org/matrix-spec-proposals/blob/main/proposals/1772-groups-as-rooms.md
- https://deepwiki.com/matrix-org/matrix-spec-proposals/4.1-spaces-and-room-organization
- https://asc.di.fct.unl.pt/~jleitao/pdf/dsn07-leitao.pdf
- https://www.bartoszsypytkowski.com/hyparview/
- https://www.bartoszsypytkowski.com/plumtree/
- https://nips.nostr.com/65
- https://docs.bsky.app/blog/jetstream
- https://jazco.dev/2024/09/24/jetstream/
- https://docs.bsky.app/docs/advanced-guides/firehose
