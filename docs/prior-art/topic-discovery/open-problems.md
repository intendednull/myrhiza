**Date:** 2026-06-08
**Status:** active
**Subject:** What P2P topic-discovery systems structurally do NOT solve → Myrhiza's `host.subscribe` discovery risk list

# Open problems

Structural gaps the surveyed systems do not close. These become Myrhiza's risk list for `host.subscribe`. Each names the gap, who fails to solve it, and the consequence for Myrhiza.

## 1. Cold-start bootstrap is everyone's punted problem

`iroh-gossip subscribe` *requires* caller-supplied bootstrap NodeIDs and resolves none itself ([iroh-discovery.md](./iroh-discovery.md)). Peer sampling (HyParView/Plumtree) only expands an overlay you've *already entered*. So "find the very first peer on a topic" is solved by **nobody** in the stock stack — it is delegated to a DHT announce/lookup or a hardcoded list. Myrhiza must build and own this resolver; it cannot inherit it. Risk: if the resolver is unavailable (DHT unreachable, relays down), a topic is *unjoinable* even though its ID is known.

## 2. Membership enumeration leaks topic interest

Any decentralized topic→peers scheme that publishes "PeerID P announces on topic T" under a publicly-derivable key (plain Kademlia provider records, un-gated Hyperswarm `announce`, the public half of `distributed-topic-tracker`) lets a passive DHT observer **enumerate a topic's membership** and correlate peers across topics. None of these systems solve private-set discovery. For Myrhiza's capability-scoped/private topics this is a deanonymization vector. The secret-gated encryption layer hides the *peer list contents* but the *DHT slot is still probeable* (you learn someone is there, just not who). Mitigations (PSI, blinded rendezvous) are research-grade, not off-the-shelf. **(unverified that any production P2P system ships private-set topic discovery.)**

## 3. DHT records are tiny, expiring, and need constant republish

BEP44 caps values at **1000 bytes** and expires items in ~**2 hours** (republish hourly); Kademlia provider records expire ~**48h** (republish ~22h). This forces a "small signed pointer + active republish daemon" design and means **a topic with no currently-online, actively-republishing member becomes undiscoverable** — even if peers hold its ID. There is no durable, offline-surviving topic→peers record. Risk for Myrhiza: low-traffic / dormant topics silently fall off the discovery layer. The rotating-minute-key scheme makes this worse (the slot moves every 60s; a peer offline for a minute must re-derive and re-find).

## 4. No system enumerates *which topics exist*

DHTs answer "peers for a key you hold," never "list the keys." In-state enumeration (Matrix `m.space.child`) only reveals children *of topics you already follow*. So topic *existence* is fundamentally gated on already holding a reference (out-of-band, or via a parent). This is *correct* for content-addressed privacy, but it means **there is no recovery path if every reference to a topic is lost** — the topic is cryptographically unreachable, not merely hard to find. Myrhiza inherits this as a permanent property, not a bug: design UX around it (durable invite tickets, parent-topic anchoring) rather than expecting discoverability.

## 5. Mainline DHT is attackable (Sybil / eclipse / poisoning)

Provider/mutable-record lookups on an open DHT are subject to Sybil and eclipse attacks: an adversary controlling node IDs near a topic's key can withhold or poison the peer list, isolating or misdirecting joiners. The surveyed specs acknowledge but do not solve this (libp2p rendezvous spec: spam mitigation "TBD"; BEP44 authenticates the *record* but not the *routing*). Myrhiza's BLAKE3 topic IDs don't help here — the *routing to the key* is the weak point, not the record signature. Cross-reference Myrhiza's own [`sybil-resistance`](../sybil-resistance/) corpus; the discovery layer needs its own threat treatment.

## 6. Browser peers are second-class for DHT participation

No surveyed DHT runs natively in a browser (no UDP). pkarr/Hyperswarm browser support is **relay-mediated**, which reintroduces a trusted-ish HTTP relay into an otherwise decentralized path and a censorship/availability chokepoint. Myrhiza's jco target inherits this: browser peers depend on relays to bootstrap, so "fully decentralized discovery" is only true for native peers. Relay liveness becomes a discovery dependency for the entire browser cohort.

## 7. Rotating-key freshness vs. clock skew

The `distributed-topic-tracker` `unix_minute` derivation assumes peers share a clock to within the rotation window. Skew across the minute boundary means peers derive *different* keys and miss each other. The crate mitigates by checking adjacent windows, but **this is a verified design tension, not a solved problem** — coarser windows reduce skew sensitivity but enlarge the replay/observation window (item 2). Myrhiza must pick a window size that trades clock-skew tolerance against membership-leak exposure; there is no free setting.

## 8. Capability revocation vs. discovery caching

Myrhiza requires subscriptions to be **revocable**. But discovery artifacts (a peer that learned a topic's rotating key, a cached provider record, a gossiped bootstrap hint) **propagate and persist independently of the capability**. Revoking a capability stops the kernel from *delivering* to the sandbox, but it cannot un-publish a topic's bootstrap pointers already in the DHT or recall a topic ID a peer has cached. None of the surveyed systems model "discovery-time revocation." Consequence: revocation must be enforced at the *delivery/membership* boundary (kernel refuses to gossip to a revoked peer; topic re-keys), not at the *discovery* boundary — discovery is best treated as monotonic and un-revocable. This is a genuine design constraint to flag in the `host.subscribe` spec.

## Sources

- https://www.bittorrent.org/beps/bep_0044.html
- https://github.com/libp2p/specs/blob/master/kad-dht/README.md
- https://github.com/libp2p/specs/blob/master/rendezvous/README.md
- https://github.com/n0-computer/iroh-gossip
- https://rustonbsd.github.io/2025/09/03/distributed-topic-tracker.html
- https://jazco.dev/2024/09/24/jetstream/
