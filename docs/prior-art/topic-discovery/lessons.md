**Date:** 2026-06-08
**Status:** active
**Subject:** Decisions for `host.subscribe` topic discovery — what to validate, avoid, and borrow from P2P discovery prior art

# Lessons for `host.subscribe`

The decision file. Every bullet is targeted at Myrhiza's `host.subscribe`: a kernel-mediated capability letting a sandboxed *interaction* component subscribe to multiple content-addressed gossip topics and aggregate their per-topic state feeds in-sandbox. Topic IDs are **bundle-derived BLAKE3 hashes** (non-human). Transport is **iroh-gossip** over Mainline-style discovery, native (Wasmtime) and browser (jco).

## The two questions this corpus answers for Myrhiza

**Q1 — How does a sandboxed app learn a foreign topic ID to subscribe to?**
Three patterns exist; rank them:

1. **In-state enumeration (primary).** Learn child topic IDs from events inside a parent topic you already follow — the Matrix `m.space.child` shape ([in-band-and-centralized.md](./in-band-and-centralized.md)). A "server" topic's convergent state enumerates its "channel" topic IDs; the interaction component reads them and asks the kernel to subscribe. This keeps enumeration *inside* convergent state (deterministic, verifiable) while the subscription act stays peer-local and non-deterministic. **This is the recommended default.**
2. **Out-of-band (root entry only).** Someone pastes a bundle hash / invite ticket. This covers the *first* topic; everything below it is in-state. Matrix bundles a `via` bootstrap hint with the child id — Myrhiza should bundle a bootstrap pointer with an out-of-band topic ref too.
3. **DHT/global directory (rejected for enumeration).** No system here enumerates *which topics exist* from a DHT — DHTs answer "find peers for a key you hold," not "list keys." Content-addressed IDs are unguessable by design; a global topic index would be both infeasible and a privacy leak.

**Q2 — How does a peer find OTHER peers on a topic to bootstrap an iroh-gossip subscription (topic-key → peers)?**
`iroh-gossip subscribe(topic, bootstrap_peers)` *requires caller-supplied bootstrap NodeIDs* and resolves none itself ([iroh-discovery.md](./iroh-discovery.md)). Fill the hole with a **topic-key → bootstrap-NodeIDs** resolver, then let iroh discovery resolve NodeID→address and HyParView/Plumtree expand the overlay. Proven mechanisms: Hyperswarm `announce`/`lookup`, Kademlia provider records, or a rotating-key BEP44 scheme (`distributed-topic-tracker`).

## Validates

- **A content hash is a sufficient rendezvous key.** Hyperswarm (32-byte topic = hash), Kademlia provider records (multihash key), and BEP44 (hash-derived target) all key discovery on opaque hashes with no human name. Myrhiza's BLAKE3 topic IDs are a first-class DHT/announce key as-is — no naming layer needed for topic→peers.
- **Separating discovery from membership is correct.** iroh's clean split — discovery resolves *NodeID→address*; gossip handles *topic membership* via peer sampling — validates keeping topic→peers resolution as a distinct kernel concern, not something baked into the transport or exposed to the sandbox.
- **In-state enumeration is a real, deployed pattern.** Matrix `m.space.child` proves "parent topic lists child topic IDs (+ bootstrap hints)" works at scale and keeps opaque IDs out of the human's hands. Validates Myrhiza's "server topic lists channel topics" design.
- **Pointer records, not data records.** BEP44/pkarr's hard **1000-byte** ceiling validates treating any DHT publication as a *small signed pointer* ("who/where for topic T"), with actual state flowing over gossip. Discovery layer ≠ storage layer.
- **Post-bootstrap self-healing is free.** HyParView shuffle + passive-view promotion means once the kernel finds *one* peer, churn resilience is handled below `host.subscribe`. The capability only needs to solve cold-start.

## Avoid

- **Don't put discovery/subscription into canonical state.** Which topics a peer subscribed to, message order, and delivery are **non-deterministic and peer-local** — they must never enter a state-digest (would break cross-peer convergence). `state-apply` must reject `host.subscribe`. The *enumeration of child topic IDs* may live in state (it's derived deterministically); the *act of subscribing* may not. Keep this line bright.
- **Avoid the firehose / single-relay shortcut.** AT Proto's relay aggregates everything into one trusted, *non-self-authenticating* stream (Jetstream drops signatures). That is the opposite of per-topic verifiable convergence. Don't let "discovery is hard" tempt a global aggregator.
- **Avoid human-named rendezvous namespaces.** libp2p rendezvous and Nostr relays key on human strings (namespace ≤255 chars; relay URLs). Myrhiza's IDs are content hashes; bolting on human names reintroduces Zooko's-Triangle naming problems and a squatting/spam surface the hash model avoids.
- **Don't assume a UDP DHT path exists.** In-browser (jco) there is no UDP socket → no direct Mainline participation. Any topic→peers resolver must have an **HTTP-relay-mediated** path (as pkarr and browser pkarr clients do), or browser peers can't bootstrap. Design the resolver relay-first, native-fast-path-second.
- **Don't leak topic interest to the whole DHT in the clear.** Publishing "PeerID P is on topic T" under a publicly-derivable key (plain provider record / un-gated `announce`) lets any observer enumerate a topic's membership. For private topics this is a deanonymization vector (see [open-problems.md](./open-problems.md)).
- **Don't rely on caller-hardcoded bootstrap peers.** iroh-gossip's example punts to a hardcoded list; that's fine for demos, unacceptable for a capability that must work cold. The kernel must own a real resolver.

## Borrow

- **The `m.space.child` pairing: child-ID + bootstrap-hint in one in-state record.** When a parent topic enumerates a child topic ID, attach a bootstrap pointer (current rendezvous key or a few known NodeIDs) in the same event, à la Matrix's `via`. The sandbox reads `(child_topic_id, hint)`; the kernel uses the hint to bootstrap. One read solves both Q1 and Q2 for in-state children.
- **Rotating publicly-derivable BEP44 key for open topic→peers** (`distributed-topic-tracker`): `signing_seed = SHA512(topic_hash ++ unix_minute)[..32]`, salt `= SHA512("salt" ++ topic_hash ++ unix_minute)[..32]`; every peer derives the same minute-keyed mutable record from the topic hash alone, `get_mutable`/`put_mutable` to find/announce peers. Decentralized, no central server, works from the content hash with zero extra inputs. This is the strongest direct fit for the kernel's topic→bootstrap resolver.
- **Secret-gated encryption layered on the rotating key** for *private/capability-scoped* topics: derive a *second* key from a shared secret to encrypt the per-record peer list, so non-holders can locate the DHT slot but not read membership. Maps cleanly onto Myrhiza's attenuable, revocable capability model — the capability *is* the secret needed to resolve a private topic's peers.
- **Kademlia `ADD_PROVIDER`/`GET_PROVIDERS` semantics** as the mental model and fallback: provider records keyed by the topic hash, source-authenticated, with republish (~22h) / expiry (~48h). A well-understood, spec'd alternative to the rotating-key scheme if minute-rotation churn proves too chatty.
- **iroh's pkarr-over-DHT discovery, unchanged, for NodeID→address.** Once the resolver yields bootstrap NodeIDs, reuse iroh's existing discovery (pkarr/n0-DNS/mDNS) to dial them. Don't reinvent identity→location; only build topic→identity.

## Recommended shape (synthesis)

1. **Root topic in:** out-of-band ticket carrying `topic_id (BLAKE3) + bootstrap hint`.
2. **Child topics in:** in-state enumeration (`m.space.child`-style events) carrying `child_topic_id + hint`. Enumeration is deterministic state; the subscribe call is not.
3. **topic→peers:** kernel-owned resolver — rotating publicly-derivable BEP44 key from the topic hash for public topics; secret-gated variant (capability = secret) for private/scoped topics; relay-mediated for browser. Provider-record semantics as fallback.
4. **NodeID→address:** existing iroh discovery.
5. **overlay growth + churn:** HyParView/Plumtree, kernel-side, invisible to the sandbox.
6. **capability boundary:** `host.subscribe` handle is unforgeable, per-topic, attenuable, revocable; `state-apply` rejects it; nothing about subscription state crosses into any digest. **Discovery ends here** — at "the kernel holds bootstrap peers." The *streaming-handle ABI* (how the unforgeable per-topic handle delivers a per-message stream across the WASM boundary, and how single-use request-tokens give way to a stream shape) is out of this folder's scope; see siblings [`streaming-capabilities`](../streaming-capabilities/) and [`wasm-async-streaming`](../wasm-async-streaming/).

## Sources

- https://github.com/n0-computer/iroh-gossip
- https://docs.iroh.computer/concepts/discovery
- https://rustonbsd.github.io/2025/09/03/distributed-topic-tracker.html
- https://github.com/rustonbsd/distributed-topic-tracker
- https://github.com/holepunchto/hyperdht
- https://github.com/libp2p/specs/blob/master/kad-dht/README.md
- https://www.bittorrent.org/beps/bep_0044.html
- https://github.com/pubky/pkarr
- https://deepwiki.com/matrix-org/matrix-spec-proposals/4.1-spaces-and-room-organization
- https://nips.nostr.com/65
- https://jazco.dev/2024/09/24/jetstream/
