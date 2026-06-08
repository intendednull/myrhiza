**Date:** 2026-06-08
**Status:** active
**Subject:** Decentralized topic→peers substrates — Mainline DHT / BEP44, pkarr, libp2p Kademlia providers, libp2p rendezvous, Hyperswarm DHT

# DHT rendezvous & provider records

The decentralized answer to **"given a topic key, find peers on it"**. All of these turn a key into a set of peers/records without a central server. They do *not* solve "how do I learn the key exists" — that is [in-band/centralized](./in-band-and-centralized.md).

## Mainline DHT + BEP44 (the substrate everyone reuses)

Mainline is BitTorrent's Kademlia DHT — ~10M+ nodes and ~15 years live (both approximate, order-of-magnitude), no bootstrap of a new network needed. BEP44 ("Storing arbitrary data in the DHT") adds two record types:

- **Immutable items** — stored under `SHA-1(value)`; self-verifying, no signature.
- **Mutable items** — stored under `SHA-1(ed25519_pubkey [+ salt])`, authenticated by a 64-byte ed25519 signature over `seq` concatenated with the value. Updates require a monotonically increasing `seq`; storing nodes MUST NOT downgrade to a lower `seq`. Optional `cas` (compare-and-swap) guards races (error 301 on mismatch).

Hard limits (verified, BEP44):
- Value `v`: storing nodes MAY reject if bencoded form > **1000 bytes**. "Not safe to assume storing more than 1000 bytes will succeed."
- Salt: MUST NOT exceed **64 bytes**. Salt lets one keypair publish many unrelated items.
- Expiry: items MAY expire in **2 hours**; SHOULD be re-announced **once an hour** to stay alive. Republish skippable if the 8 closest nodes already hold the data.

The 1000-byte cap is the load-bearing constraint: **BEP44 is a discovery/pointer layer, not storage.** You publish "here is where/who," not the data itself.

## pkarr — pubkey → DNS records over Mainline

[pkarr](https://github.com/pubky/pkarr) (Public-Key Addressable Resource Records; maintained under the `pubky`/Pubky org) turns an ed25519 pubkey into a self-sovereign TLD: you publish *signed DNS resource records* as a BEP44 mutable item keyed by your pubkey (z-base32 encoded as the name, e.g. `o4dksfbqk85...`). HTTP relays republish for browsers (no UDP DHT in-browser). Inherits BEP44's **1000-byte** ceiling — pkarr's own docs: "PKARR is for discovery, not storage." This is iroh's discovery backbone (see [iroh-discovery.md](./iroh-discovery.md)).

Note the shape: pkarr resolves **pubkey → records (addresses)**, not **topic → peers**. It is a key→value pointer indexed by *identity*, not by *topic interest*.

## libp2p Kademlia provider records

The provider-record pattern is the canonical "topic→peers" DHT primitive:

- A provider calls **`ADD_PROVIDER`** for key K (K = a **multihash** of content, not the full CID — lets multiple CID encodings of the same bytes share a rendezvous point).
- A seeker calls **`GET_PROVIDERS(K)`**, iteratively querying the *k* closest nodes to K until it collects provider PeerIDs.
- DHT servers accept provider records only from the source peer (verified by the connection's crypto handshake) — you can't announce others.
- Liveness (IPFS values): republish interval **22 hours**, expiration **48 hours**. Stale addresses dropped after the routing-table refresh interval (~30 min in kubo), after which only PeerIDs are returned.

The key insight for Myrhiza: a content hash *is* a usable rendezvous key. "Find peers on topic T" = "find providers of T." No human name required.

## libp2p rendezvous protocol

A lighter, *named-point* alternative to DHT lookups. Any node can be a rendezvous point; peers **`REGISTER`** a signed peer record under a namespace, others **`DISCOVER`** that namespace.

- A peer may only register *itself* (signed peer record).
- Default registration TTL **2 hours**, max **72 hours** (`E_INVALID_TTL` above the point's bound).
- Recommended caps: ≤**1000 registrations/peer**, ≤**1000 peers/namespace** per response, namespace ≤**255 chars**. `DISCOVER` returns a cookie for pagination.

Trade-off vs DHT: faster real-time discovery and explicit namespaces, but reintroduces a semi-centralized point and the spec admits spam mitigation is "TBD." Namespaces are human strings — *the opposite of content-addressed*.

## Hyperswarm DHT (hyperdht)

Holepunch's Kademlia DHT, the closest structural twin to Myrhiza's model: a **topic is a 32-byte key (a hash of something)**.

- `node.announce(topic, keyPair, ...)` — advertise that you serve `topic`.
- `node.lookup(topic)` — stream of peers announcing on `topic`.
- Also offers `mutablePut/Get` (ed25519-signed, seq-versioned) and `immutablePut/Get` (hash-keyed) — same BEP44-style split.
- Includes UDP holepunching so announced peers are actually dialable behind NATs.

Hyperswarm proves the exact primitive Myrhiza needs: **32-byte content hash → peer set**, fully decentralized, no human name in the loop. (See sibling [`pears`](../pears/) for the Holepunch stack context.)

## What carries over

The provider-record / `announce`-`lookup` shape (Kademlia, Hyperswarm) and the BEP44 signed-mutable-pointer shape (pkarr) are the two reusable decentralized primitives. Both accept an opaque hash as the key — they never need the topic to be human-readable. Both are *pointer layers*: small, signed, expiring, republished. Neither tells you a topic *exists*; you must already hold the key.

## Sources

- https://www.bittorrent.org/beps/bep_0044.html
- https://github.com/pubky/pkarr
- https://pubky.github.io/pkarr/
- https://github.com/libp2p/specs/blob/master/kad-dht/README.md
- https://github.com/libp2p/specs/blob/master/rendezvous/README.md
- https://github.com/holepunchto/hyperdht
- https://www.npmjs.com/package/@hyperswarm/dht
