**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — peer discovery (Kademlia DHT, mDNS, rendezvous, AutoNAT, Circuit Relay, DCUtR)

# Discovery

Discovery in libp2p means two distinct problems:

1. **Address resolution** — I know a PeerId, find its addresses. Solved by the Kademlia DHT, identify-on-connect, AutoNAT-discovered addrs, and signed peer records.
2. **Peer finding** — I don't know which PeerIds exist that match my interest. Solved by mDNS (local), rendezvous (out-of-band registry), gossipsub topic membership, and the DHT's provider records.

This is the same distinction iroh draws between "discovery" and "discoverability" ([`../iroh/critiques.md`](../iroh/critiques.md) §"Discovery — how do strangers meet?"). libp2p has invested heavily in *both* layers; iroh has invested only in the first. The result: libp2p has real solutions to "find a peer who serves content X" (DHT provider records) and "find peers who care about topic T" (gossipsub mesh membership), where iroh punts both to the application.

This file is the deep dive on **how libp2p's discovery layer actually works**. The Myrhiza-relevant fact is that **we inherit iroh's narrower scope** — pkarr-on-Mainline-DHT for `PeerId → addresses` — and any "discoverability" primitive becomes Myrhiza's problem to design.

## Kademlia DHT (the canonical sub-protocol)

The Kademlia DHT is libp2p's primary discovery substrate. The spec ([kad-dht r2, 2022-12-09](https://github.com/libp2p/specs/blob/master/kad-dht/README.md), 3A Recommendation) implements a Kademlia-flavored distributed hash table with libp2p-specific extensions.

### Algorithm baseline

Kademlia [Maymounkov & Mazières, 2002](https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf) is the canonical structured P2P overlay. Each peer has a node ID; each piece of data has a key. The distance between two IDs is `XOR(id1, id2)`. The peer "closest" to a key (smallest XOR) stores or serves that key.

libp2p's variant:

- **Node ID:** `sha256(PeerId)` — the PeerId is hashed before use in the XOR distance metric. **Important verification:** the spec says *"In all cases, the distance between two keys is `XOR(sha256(key1), sha256(key2))`"* — both node IDs and content keys are SHA-256'd before XOR comparison. This is libp2p-specific; the original Kademlia paper used the raw node ID.
- **Replication factor `k = 20`** — each peer maintains a routing table of up to 20 peers per bucket. The spec says *"The recommended value for `k` is 20."* The 20 closest peers to a key are responsible for storing it.
- **Concurrency `α = 10`** — when doing a lookup, query α peers in parallel at each step. *"The concurrency of node and value lookups are limited by parameter `α`, with a default value of 10."* Original paper had α = 3; libp2p picked 10 for faster lookups at the cost of more bandwidth.
- **Bucket structure:** 256 buckets (one per bit of the SHA-256 output); each holds up to k peers.

### Operations

- **`FIND_NODE(peer_id)`** — locate the k peers closest to a target. Answer: routing-table entries.
- **`FIND_VALUE(key)`** — locate the value at a key. May return `value` (terminal) or `closer peers` (continue iterating).
- **`PUT_VALUE(key, value, signature)`** — store a signed value at the k closest peers. IPNS records are stored this way.
- **`ADD_PROVIDER(key)`** — register *yourself* as serving content for key. Stored at the k closest peers. Used by IPFS for content discovery.
- **`GET_PROVIDERS(key)`** — find peers serving content for key. The IPFS lookup primitive.

### Provider records (the IPFS use case)

The most-stressed operation in production is `ADD_PROVIDER` / `GET_PROVIDERS`. IPFS uses this to answer "which peers have this CID?" Per the spec:

- **Provider record TTL:** 48 hours (records expire 48h after the most recent provide).
- **Republish interval:** 22 hours (providers re-add their records every 22h, before the 48h TTL expires).
- **Replication factor:** k = 20 (the 20 closest peers store the provider record).

Real-world cost: IPFS's content discovery is **slow and expensive at scale**. A provider lookup can take 10–60 seconds in production, with substantial bandwidth amplification (you contact ~30+ peers to walk the DHT). Protocol Labs has invested heavily in [Hydra Booster](https://github.com/libp2p/hydra-booster) (now archived) and [accelerated-DHT-client](https://blog.ipfs.io/2023-09-13-accelerated-dht-client/) optimisations. Per the iroh team's [pivot post](https://www.iroh.computer/blog/a-new-direction-for-iroh): *"IPFS is that the performance is… not great… to put it politely, so it is not really a useful primitive."* The performance critique is largely about DHT provider lookup, not about Kademlia-as-routing.

### Public DHT vs private DHT

libp2p supports both:

- **Public DHT** — the global IPFS DHT (~25k–50k Kubo peers as of 2024–25, per IPFS metrics). Joining is automatic by default in Kubo.
- **Private DHT** — your app spawns its own DHT with no shared bootstrap. Use case: Filecoin's miner network, Storm's coordination network, etc.

The choice is a config flag (`Mode::Server` vs `Mode::Client` and a custom set of bootstrap peers). Many production apps run private DHTs to avoid the global DHT's churn and noise.

## mDNS (LAN discovery)

For local-area peers, libp2p uses mDNS — multicast DNS announces on `224.0.0.251:5353`. A peer broadcasts its PeerId + multiaddrs on the local subnet; other peers listen and add them to their peerstore.

- **Service name:** `_p2p._udp.local`.
- **Latency:** ~1 second to detect a new local peer.
- **Caveats:** mDNS doesn't cross subnets / VLANs. Some corporate networks block it. Many residential routers have flaky mDNS forwarding.

The use case: zero-config peer discovery in apps that should "just work" on the same LAN. Useful for collaborative editing, local file sharing, mesh-IoT. Almost never load-bearing in production deployments.

## Rendezvous

A registry pattern: peers register at a known rendezvous server with "I am interested in namespace N," and others can query "who's interested in namespace N?" Spec: [rendezvous](https://github.com/libp2p/specs/blob/master/rendezvous/README.md).

- The rendezvous server is just a libp2p peer that speaks the `/libp2p/rendezvous/1.0.0` protocol.
- Registrations have a TTL.
- Multiple rendezvous servers can be queried in parallel; redundancy is application-level.

Use case: app bootstrap. "Find peers in my app's namespace" without putting an interest topic on the global DHT.

## AutoNAT

AutoNAT is the **"am I reachable from the outside internet?"** primitive. Spec: [autonat](https://github.com/libp2p/specs/tree/master/autonat).

The protocol:

1. Peer A wants to know its reachability.
2. A connects to peer B (any peer running AutoNAT service).
3. A sends `Dial` request with A's listen multiaddrs.
4. B attempts to dial A on those addresses (from B's perspective — from the outside).
5. B replies with `Ok` if the dial succeeded, `Err` if not.
6. A repeats with several Bs to get statistical confidence.

Outcome: A learns whether it is `Public`, `Private` (behind NAT), or `Unknown`. This drives downstream choices: a private peer enables AutoRelay; a public peer doesn't.

**AutoNAT v2** is in development with privacy and DoS improvements (the autonatv2 example in rust-libp2p workspace). Not yet shipped as Recommendation.

## Circuit Relay v2 + AutoRelay

When a peer is behind NAT and not reachable directly, it can register with relay peers. Spec: [circuit-v2](https://github.com/libp2p/specs/blob/master/relay/circuit-v2.md).

- **Reservation:** the unreachable peer (A) sends `RESERVE` to a relay (R), which replies with a `voucher`. The voucher includes a TTL.
- **Circuit dial:** to dial A, a peer (B) dials R first, then sends `CONNECT { peer: A }`. R proxies the bytes between A's existing connection and B.
- **Circuit multiaddr:** A advertises `/p2p/<R>/p2p-circuit/p2p/<A>`. B uses this to dial A through R.
- **Limits:** the v2 spec sets per-circuit traffic limits (default 128 KiB) and time limits (default 2 min). Exceeded circuits are torn down. *Reserved circuits* (with vouchers) get higher limits.

**AutoRelay** is the protocol that auto-picks relays. A NAT-private peer discovers available relays (via DHT or bootstrap config) and registers reservations with them. Each peer maintains 1–3 relay reservations for redundancy.

The Circuit Relay v2 protocol is the libp2p analog of iroh's DERP relay protocol ([`../iroh/nat-traversal.md`](../iroh/nat-traversal.md)). The same metadata-leakage critique applies — the relay sees which PeerIds talk to each other.

## DCUtR (Direct Connection Upgrade through Relay)

Once two peers are connected via relay, they can attempt to upgrade to a direct connection via simultaneous hole punching. Spec: [DCUtR](https://github.com/libp2p/specs/blob/master/relay/DCUtR.md).

The protocol:

1. A and B are connected via R.
2. A sends `Connect` to B over the circuit.
3. Both A and B include their observed addresses (learned from AutoNAT / identify-observed).
4. After exchanging addresses, A and B simultaneously dial each other on those addresses.
5. NAT firewall hole punching: the outbound packets create translation entries on each side's NAT, so the inbound packets find their way back.
6. If successful, both sides have a direct connection; the circuit is closed.

**Hole punching success rate is the critical real-world question.** The libp2p team has published [data from their hole-punching tests](https://blog.libp2p.io/2022-01-20-libp2p-hole-punching/) showing ~70% success in field conditions. The iroh team explicitly calls out this number in [their comparison post](https://www.iroh.computer/blog/comparing-iroh-and-libp2p): *"with libp2p, your ability to connect to a specific given peer is much more dependent on the network conditions between you and that peer."*

The 70% is honest data — most real-world NAT configurations are punch-able; the failure cases are:

- **Symmetric NATs** (random outbound port assignment) — fundamentally unpunchable.
- **Carrier-grade NATs** (multiple users behind one IP) — partial; depends on the CGN's session table.
- **Firewall + IDS** — some enterprise firewalls drop unexpected inbound UDP regardless of state.

In those 30% of cases, the connection stays on the relay. Per-app cost: the relay traffic continues, with its bandwidth/CPU cost and metadata leak.

Iroh's design choice is to skip DCUtR-style coordination and use **QAD (QUIC Address Discovery)** to learn observed addresses directly during a QUIC handshake, with **continuous direct-path racing** as a background task. The libp2p approach is more legible (a named protocol for hole punching with explicit failure modes); the iroh approach is more aggressive (always probe for direct path).

## Identify-observed address

Mentioned in [`architecture.md`](architecture.md): every libp2p connection produces an identify exchange that includes `observed_addr` — the multiaddr at which the peer observes you. Aggregating observed_addr across connections is the lightweight "what's my public IP?" answer, complementary to AutoNAT.

## Provider records vs gossipsub mesh membership

The two main "find peers who care about X" primitives:

- **DHT provider records** — durable, queryable, expensive to look up. Use when "X" is a content hash that occasionally needs lookup.
- **Gossipsub mesh membership** — ephemeral, no lookup (peers are continuously connected through the mesh), cheap to maintain. Use when "X" is an interest topic with ongoing traffic.

Mixing them is common: a CID is gossiped on a topic for fast notify, and the DHT serves as the durable fallback. iroh-gossip + iroh-blobs sketch the same pattern at smaller scale.

## Implications for Myrhiza

- **`PeerId → addresses` resolution is solved.** Kademlia's libp2p variant is mature, with k=20 / α=10 as well-tuned defaults. Iroh's pkarr-on-Mainline-DHT is the same shape — a DHT lookup of a signed record by public key. Myrhiza inherits this for free.
- **"Find peers interested in topic T" is half-solved.** Gossipsub's mesh membership *is* the answer if you're already in the mesh; getting into the mesh requires bootstrap peers, which is an outer-layer discovery problem. Myrhiza's iroh-gossip dependency inherits the same bootstrap requirement (see [`../iroh/gossip.md`](../iroh/gossip.md) — "topics are not registered… you need at least one bootstrap peer who is already in the topic").
- **"Find peers serving content C" is solved by DHT provider records.** Myrhiza chose not to inherit this (iroh-blobs has no built-in discovery — apps share blob hashes out-of-band). If Myrhiza ever needs content-addressed discovery (a use case: "find any peer who has this app bundle"), the libp2p provider-records pattern is the reference design. Cost: a fresh DHT with bootstrap peers, lookup latency in the 10–60s range at IPFS scale.
- **AutoNAT-style reachability probing is worth borrowing.** Myrhiza's runtime should know whether a peer is publicly reachable for the same reasons libp2p needs to: it drives relay-vs-direct choices, NAT classification, and observability. The protocol is simple (5 messages).
- **The DHT performance cliff at scale is a real concern.** If Myrhiza ever runs a DHT itself (rather than piggybacking on Mainline), we will inherit IPFS's "provider lookups take 30 seconds" problem. Pre-emptively study the [accelerated-DHT-client](https://blog.ipfs.io/2023-09-13-accelerated-dht-client/) optimisations and bake them in from day one.
- **mDNS is cheap to ship.** A 100-line implementation gives Myrhiza apps zero-config LAN discovery. Worth adding when Myrhiza has its first multi-user-same-LAN use case.

## Sources

- [libp2p Kademlia DHT spec (r2, 2022-12-09)](https://github.com/libp2p/specs/blob/master/kad-dht/README.md)
- [Kademlia paper (Maymounkov & Mazières, 2002)](https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf)
- [libp2p AutoNAT spec](https://github.com/libp2p/specs/tree/master/autonat)
- [libp2p Circuit Relay v2 spec](https://github.com/libp2p/specs/blob/master/relay/circuit-v2.md)
- [libp2p DCUtR spec](https://github.com/libp2p/specs/blob/master/relay/DCUtR.md)
- [libp2p rendezvous spec](https://github.com/libp2p/specs/blob/master/rendezvous/README.md)
- [libp2p mDNS spec](https://github.com/libp2p/specs/tree/master/discovery/mdns.md)
- [libp2p hole-punching results blog (Jan 2022)](https://blog.libp2p.io/2022-01-20-libp2p-hole-punching/)
- [Accelerated DHT client blog (Sep 2023)](https://blog.ipfs.io/2023-09-13-accelerated-dht-client/)
- [libp2p-kad rust crate](https://crates.io/crates/libp2p-kad)
- [go-libp2p-kad-dht](https://github.com/libp2p/go-libp2p-kad-dht)
- [iroh — comparing iroh & libp2p (Jan 2024)](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
- [iroh — NAT traversal (sibling doc)](../iroh/nat-traversal.md)
