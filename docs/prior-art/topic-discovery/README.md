**Date:** 2026-06-08
**Status:** active
**Subject:** Topic/stream discovery in P2P systems — rendezvous, peer discovery, and content-addressed (non-human) topic IDs, for `host.subscribe`

# Topic Discovery (prior art)

How decentralized systems answer two questions Myrhiza's `host.subscribe` must answer:

1. **Topic-ID discovery** — how does a peer *learn which topic to subscribe to* when topic IDs are opaque content hashes, not human names?
2. **Topic→peers resolution** — given a topic ID (a 32-byte key), how does a peer *find other peers already on that topic* to bootstrap a gossip overlay?

These are distinct problems with distinct solutions. Most systems conflate them; the corpus separates them deliberately because Myrhiza's answers differ.

## Key facts

| System | Topic-ID discovery | Topic→peers resolution | Topic-ID shape | Decentralized? |
|---|---|---|---|---|
| **iroh-gossip** | out of band (caller's problem) | **out of band** — `subscribe(topic, bootstrap_peers)` requires caller-supplied NodeIDs | 32-byte `TopicId` | gossip yes; bootstrap no |
| **iroh discovery** | n/a (resolves NodeID→addr, not topic→peers) | resolves NodeID→address only | NodeID = ed25519 pubkey | pkarr/DHT yes; n0-DNS no |
| **pkarr / Mainline + BEP44** | n/a | mutable record keyed by ed25519 pubkey; ≤1000-byte value | SHA-1(pubkey[+salt]) | yes |
| **libp2p Kademlia providers** | n/a | `GET_PROVIDERS(multihash)` → provider PeerIDs | multihash of content | yes |
| **libp2p rendezvous** | namespace string (human) | `DISCOVER(ns)` → peers; via rendezvous point | namespace ≤255 chars | semi (named points) |
| **Hyperswarm DHT** | out of band | `announce(topic)`/`lookup(topic)` → peers | 32-byte key (hash) | yes |
| **HyParView/Plumtree** | n/a | in-overlay peer sampling *after* you're in | n/a | yes (post-join) |
| **distributed-topic-tracker** | n/a | time-rotated BEP44 key derived from `topic_hash` | 32-byte topic hash | yes |
| **Nostr NIP-65** | follow graph (social) | `kind:10002` relay list → relays | npub / event id | no (relays) |
| **AT Proto relay/Jetstream** | n/a (firehose is global) | one big relay aggregates all PDS streams | DID / collection | no (relay) |
| **Matrix spaces** | **in-state**: `m.space.child` events list child rooms | `via` server hints in child event | room id | federated |

## Table of contents

- [`lessons.md`](./lessons.md) — **the decision file.** Validates / Avoid / Borrow, tied to `host.subscribe`. Read this if you read nothing else.
- [`dht-rendezvous.md`](./dht-rendezvous.md) — Mainline DHT + BEP44, pkarr, libp2p Kademlia provider records, libp2p rendezvous protocol, Hyperswarm DHT. The decentralized topic→peers substrate.
- [`iroh-discovery.md`](./iroh-discovery.md) — iroh's discovery providers (pkarr, n0-DNS, mDNS/local, DHT), what they resolve, and the bootstrap-peers gap on `iroh-gossip subscribe`.
- [`in-band-and-centralized.md`](./in-band-and-centralized.md) — in-state enumeration (Matrix `m.space.child`, parent-lists-child), gossip peer-sampling (HyParView/Plumtree), and the centralized contrast (Nostr NIP-65 relays, AT Proto relay/firehose/Jetstream).
- [`open-problems.md`](./open-problems.md) — what none of these structurally solve → Myrhiza's risk list.

## Canonical reading order

1. `README.md` (this file) — the two-problem framing.
2. `lessons.md` — what to take, avoid, borrow.
3. `iroh-discovery.md` — Myrhiza's actual transport (iroh-gossip); the bootstrap gap is the central constraint.
4. `dht-rendezvous.md` — the decentralized fill for that gap.
5. `in-band-and-centralized.md` — the in-state pattern Myrhiza likely leans on, plus contrasts.
6. `open-problems.md` — residual risk.

## Glossary stub

- **Topic ID** — opaque content-addressed identifier for a gossip swarm. In Myrhiza, a bundle-derived BLAKE3 hash; in iroh-gossip/Hyperswarm, a 32-byte key.
- **Bootstrap peers** — NodeIDs already in a topic's overlay, needed to *enter* it. Distinct from discovery. (Recent iroh renames `NodeId`→`EndpointId`; this corpus's "NodeID" maps 1:1 onto it — see [iroh-discovery.md](./iroh-discovery.md).)
- **Discovery (iroh sense)** — resolving a NodeID (pubkey) to dialable network addresses. *Not* topic→peers.
- **Rendezvous point** — a peer/record where parties on a topic register so others can find them.
- **Provider record** — a DHT entry "PeerID X provides content/key K"; the Kademlia rendezvous primitive.
- **In-state enumeration** — learning child topic IDs from events inside a parent topic you already follow (e.g. a server topic lists its channel topics).
- **BEP44** — BitTorrent extension for storing mutable/immutable items in the Mainline DHT.
- **Outbox model** — Nostr pattern: discover *where* a user publishes from their advertised relay list, not by querying everyone.
- **Zooko's Triangle** — you can't have a namespace that is simultaneously decentralized, secure, and human-readable. Content-addressed IDs pick decentralized+secure, sacrificing human-readable.

## Sources

- https://github.com/n0-computer/iroh-gossip
- https://docs.iroh.computer/concepts/discovery
- https://github.com/pubky/pkarr
- https://www.bittorrent.org/beps/bep_0044.html
- https://github.com/libp2p/specs/blob/master/kad-dht/README.md
- https://github.com/libp2p/specs/blob/master/rendezvous/README.md
- https://github.com/holepunchto/hyperdht
- https://nips.nostr.com/65
