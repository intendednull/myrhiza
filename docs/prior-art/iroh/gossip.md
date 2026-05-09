**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — `iroh-gossip` topic-based pub/sub via epidemic broadcast trees

# iroh-gossip

Topic-based pub/sub that disseminates messages across a swarm of peers interested in the same 32-byte topic ID. The [README](https://github.com/n0-computer/iroh-gossip/blob/main/README.md) describes it succinctly as *"based on epidemic broadcast trees to disseminate messages among a swarm of peers interested in a topic."*

## Versions

[`n0-computer/iroh-gossip`](https://github.com/n0-computer/iroh-gossip); not archived; tracks the iroh release cadence closely. Recent line: v0.95 (2025-11-06) → v0.96 (2026-01-29) → v0.97 (2026-03-16) → v0.98 (2026-04-20) → **v0.99.0 (2026-05-08)**. The v0.99 release notes are a single line: *"[breaking] Update iroh and noq to 1.0-rc.0"* ([release notes](https://github.com/n0-computer/iroh-gossip/releases/tag/v0.99.0)). The crate is in steady-state maintenance mode against a moving iroh API rather than under active redesign.

## Algorithm

Two stacked papers, both from Leitão, Pereira, and Rodrigues at Lisbon:

- [**HyParView**](https://asc.di.fct.unl.pt/~jleitao/pdf/dsn07-leitao.pdf) (Leitão et al., 2007) — partial-view membership protocol. Each peer maintains an *active view* (small, ~5 peers, with active TCP/QUIC connections) and a *passive view* (larger, ~30 peers, addresses only). When an active peer fails, it's promoted from the passive view. Provides probabilistic connectivity guarantees under heavy churn.
- [**Plumtree**](https://asc.di.fct.unl.pt/~jleitao/pdf/srds07-leitao.pdf) (Leitão et al., 2007) — epidemic broadcast tree built on top of a HyParView-style overlay. Eager-push along a spanning tree (one copy per peer pair, low overhead) plus lazy-push of message IDs to non-tree neighbors as a repair mechanism. If a tree edge fails, IHAVE messages let downstream peers pull the missing payload from a redundant path.

iroh-gossip's `proto` module implements this as a pure state machine (no I/O); the `net` module wires it onto iroh's QUIC connections via the `iroh_gossip::ALPN`. The split lets the same protocol be tested deterministically and run on different transports.

## Topics and membership

A **topic** is a 32-byte identifier (`TopicId`). Topics are not registered, declared, or coordinated — any peer can subscribe to any topic, and the swarm forms by transitive introduction. To join, you need at least one bootstrap peer who is already in the topic; from there HyParView fans out membership.

Joining is async and explicit ([`Gossip::subscribe(topic_id, bootstrap_peers)`](https://github.com/n0-computer/iroh-gossip/blob/main/README.md#getting-started)). The returned receiver emits a `joined()` event once you've established at least one peer connection in the topic. Leaves are similarly explicit, and clean shutdown signals departure to the swarm so peers don't keep dialing a corpse.

There is no built-in topic discovery. Topic IDs are application-defined; the conventional pattern is `BLAKE3("my-app:room-name")` or similar. Two peers who don't know the same topic ID cannot find each other through gossip.

## Broadcast semantics

**Best-effort, eventually-delivered.** No total order, no causal order, no exactly-once. Plumtree gives high probability of delivery even under churn, but not guaranteed. Duplicate suppression is per-message-ID (each message carries a hash); duplicates are dropped at the receiver. Messages have no built-in TTL; they propagate until every active peer has seen the ID once, then the lazy-push IHAVE protocol stops referencing them.

Per the proto docs ([docs.iroh.computer/connecting/gossip](https://docs.iroh.computer/connecting/gossip)): *"Gossip spreads information redundantly across overlapping paths, which improves resilience but increases traffic"* and *"peers may join, leave, change addresses, or drop messages at any time."* The system is sized for *"a few thousand peers per topic"* — not millions.

There is no built-in spam/abuse control. Anyone in the topic can broadcast anything to everyone else, and message authenticity is the application's job. iroh-gossip is the transport; sender authentication, rate limiting, and content filtering live above it.

## Use cases in the iroh ecosystem

- **iroh-docs** uses gossip per-NamespaceId for live entry notifications, so newly-written entries propagate in seconds rather than at the next pull-sync interval.
- **iroh-willow** uses gossip on a per-namespace basis for the same reason — fingerprint-mismatch hints flow over gossip, then RBSR sync runs over a direct QUIC stream.
- Number 0's [Sneedlock](https://www.iroh.computer/sneedlock), [Dumbpipe](https://github.com/n0-computer/dumbpipe), and the iroh-mainline DHT crawler all use gossip for application-level coordination signals.

The pattern: gossip is for *small messages with weak delivery guarantees*. Anything that needs reliable delivery, ordering, or large payloads goes through direct iroh streams or iroh-blobs.

## Implications for Myrhiza

Gossip is a defensible primitive for Myrhiza's "live" layer — push notifications, presence, transient signals between peers that already share an app context. The HyParView+Plumtree combination is well-studied and has the cleanest churn-tolerance story of the gossip family.

It is **not** a state-replication primitive on its own. The lessons:

- **Don't fold gossip messages into deterministic state.** Plumtree gives no ordering guarantee; a state-apply component that consumed gossip directly would diverge across peers. Gossip is a hint mechanism — "something interesting happened" — that triggers a deterministic pull (RBSR sync, blob fetch) which actually updates state.
- **Topic IDs are a flat namespace with no auth.** If two apps pick the same topic, their messages collide. Use BLAKE3-of-something-domain-prefixed for topic IDs, and treat the topic ID as untrusted in the receiver — verify message contents against your own authority model before acting.
- **Sized for thousands, not millions.** A Myrhiza app at internet scale needs a sharding/routing story above gossip. Holochain's [networking history](../holochain/networking.md) is the cautionary tale: don't build the easy "every peer hears everything" thing and call sharding a future-work item.

## Sources

- [iroh-gossip repository](https://github.com/n0-computer/iroh-gossip)
- [iroh-gossip README on main](https://github.com/n0-computer/iroh-gossip/blob/main/README.md)
- [iroh-gossip v0.99.0 release notes](https://github.com/n0-computer/iroh-gossip/releases/tag/v0.99.0)
- [iroh-gossip Cargo.toml @ v0.99.0](https://github.com/n0-computer/iroh-gossip/blob/v0.99.0/Cargo.toml)
- [HyParView paper (Leitão et al., 2007)](https://asc.di.fct.unl.pt/~jleitao/pdf/dsn07-leitao.pdf)
- [Plumtree paper (Leitão et al., 2007)](https://asc.di.fct.unl.pt/~jleitao/pdf/srds07-leitao.pdf)
- [iroh-gossip proto docs](https://docs.iroh.computer/connecting/gossip)
- [iroh-blobs — sibling doc](./blobs.md)
- [iroh-docs — sibling doc](./docs.md)
- [iroh-willow — sibling doc](./willow.md)
- [Holochain networking — sibling prior-art doc](../holochain/networking.md)
