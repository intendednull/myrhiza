**Date:** 2026-06-08
**Status:** active
**Subject:** iroh's discovery providers and the iroh-gossip bootstrap-peers gap — what iroh actually resolves vs what Myrhiza needs

# iroh discovery & the gossip bootstrap gap

Myrhiza's transport is iroh-gossip. This file pins down exactly what iroh's discovery layer does and — critically — what it does **not** do, because that gap is the central design constraint for `host.subscribe`. (See sibling [`iroh`](../iroh/) for the broader stack.)

## The crucial distinction: discovery resolves NodeID→address, NOT topic→peers

iroh "discovery" answers one question: **given a NodeID (an ed25519/elliptic-curve public key), where on the network is it?** It returns a home relay URL and/or direct-dial addresses. It is an *identity→location* map. It says nothing about which nodes are interested in a topic.

> **Terminology note:** recent iroh renames `NodeId`→`EndpointId` (and `NodeAddr`→`EndpointAddr`) — see [iroh#3301](https://github.com/n0-computer/iroh/issues/3301). Current `iroh-gossip` docs show `subscribe(topic_id, bootstrap_peers: Vec<EndpointId>)`. The concept (an identity pubkey) is unchanged; this corpus uses "NodeID" throughout, which maps 1:1 onto `EndpointId` in current releases.

iroh ships four discovery providers (verified, iroh docs):

| Provider | Default | Mechanism | Centralization |
|---|---|---|---|
| **DNS discovery** | enabled | custom DNS server (n0 runs `dns.iroh.link` / `iroh-dns-server`) | central (n0) |
| **Pkarr** | enabled (via DNS) | signed pkarr packets over HTTP relays → Mainline DHT | decentralized (DHT) |
| **Local / mDNS** | disabled | mDNS-like LAN discovery | LAN-local |
| **DHT (Mainline)** | disabled | pkarr packets directly on BitTorrent Mainline DHT | decentralized |

A pkarr packet here contains `{NodeID, home_relay_url}` — i.e. it republishes *node location*, keyed by node identity. Note: iroh docs state publishing pkarr packets *directly* onto Mainline (vs via HTTP relay) "is not yet supported in iroh natively" via the default path; the DHT provider exists but is off by default.

## The gap: `iroh-gossip subscribe` needs bootstrap peers you must supply

A gossip swarm is centered on a **`TopicId` — a 32-byte identifier**, "usually some random 32-bytes." To join:

```rust
gossip.subscribe(topic_id, bootstrap_peers).await?
```

`bootstrap_peers` is a list of **NodeIDs already in the topic's overlay**, and it is the **caller's responsibility** — the official example literally carries a `bootstrap_peers()` placeholder commented *"insert your bootstrap peers here, or get them from your environment."*

**iroh-gossip itself does not resolve a topic to peers.** Once you reach *any* member, HyParView+Plumtree take over (peer sampling — see [in-band-and-centralized.md](./in-band-and-centralized.md)). But the *first* member is a chicken-and-egg the gossip layer punts on. Discovery (above) can turn a known bootstrap *NodeID* into an address, but it cannot tell you *which* NodeIDs are on topic T. That mapping does not exist in stock iroh.

So Myrhiza has a precise hole to fill: **`TopicId (BLAKE3 hash) → set of bootstrap NodeIDs`**, which discovery then resolves to addresses.

## How others fill exactly this hole over Mainline

`distributed-topic-tracker` (rustonbsd, Sept 2025) is a community crate built to auto-bootstrap iroh-gossip topics with zero hardcoded peers, via Mainline. Its mechanism (verified from its PROTOCOL.md) is instructive:

- **Time-rotated deterministic signing key**: `keypair_seed = SHA512(topic_hash ++ unix_minute)[..32]`, where `unix_minute = floor(unixtime/60)`. Every peer that knows `topic_hash` independently derives the *same* BEP44 mutable keypair for the current minute.
- **Deterministic salt**: `salt = SHA512("salt" ++ topic_hash ++ unix_minute)[..32]`.
- Peers `get_mutable(signing_pubkey, salt)` to read the current peer list, and publish themselves the same way. The rotating key spreads load and limits replay to a one-minute window.
- **Optional access control via a *separate* secret**: an encryption keypair derived from a shared `initial_secret` encrypts the per-record one-time keys, so peers *without the secret* can find the DHT address but cannot decrypt the active peer list. Rate-limited (`max_bootstrap_records`) against flooding.

This is the canonical pattern to study: the **publicly-derivable rotating-key** half gives open discovery from a content hash alone; the **secret-derived encryption** half adds capability-like gating on top. Both halves are relevant to Myrhiza (see [`lessons.md`](./lessons.md)).

## Browser constraint

Myrhiza must run under jco transpile in browsers. In-browser, there is no UDP socket → no direct Mainline DHT participation. Both pkarr and `distributed-topic-tracker`-style schemes therefore depend on **HTTP relays** to read/write DHT records from the browser. Any Myrhiza topic→peers resolver must have a relay-mediated path, not only a native UDP path. (See sibling [`jco`](../jco/).)

## Sources

- https://docs.iroh.computer/concepts/discovery
- https://www.iroh.computer/blog/iroh-dns
- https://www.iroh.computer/blog/iroh-global-node-discovery
- https://github.com/n0-computer/iroh-gossip
- https://docs.rs/iroh-pkarr-node-discovery
- https://github.com/rustonbsd/distributed-topic-tracker
- https://rustonbsd.github.io/2025/09/03/distributed-topic-tracker.html
