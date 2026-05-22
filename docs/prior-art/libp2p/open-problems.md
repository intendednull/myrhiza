**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — open problems and structural limitations

# Open problems

What libp2p **structurally does not solve**. Each entry is a real problem the stack doesn't address, with the closest workaround documented. These aren't bugs — they're scoping decisions that leave the problem to the application layer or to other tools.

For Myrhiza, each open problem is either: (a) a problem we *also* don't solve (because we picked a similarly-scoped transport), (b) a problem we must compensate for, or (c) a problem libp2p doesn't solve but iroh does (or vice versa).

## Sybil resistance: none, by design

Anyone can spin up arbitrarily many PeerIds at zero cost. libp2p does not provide:

- Proof-of-personhood.
- Resource-cost-of-identity (no PoW, no stake, no payment).
- Identity rate limiting.
- A global Sybil floor.

The closest libp2p has is **gossipsub's peer scoring** (see [`gossipsub.md`](gossipsub.md)) — local-knowledge per-peer scores with IP-colocation penalty (P6). This is *local* Sybil mitigation (you can't dominate one peer's mesh) but not *global* (you can spam the network with N identities at 0.01 cost each).

**Mitigation patterns in production:**

- **Ethereum:** stake-weighted Sybil mitigation at the protocol-validator layer; the libp2p layer doesn't try.
- **Filecoin:** miner-collateral Sybil mitigation similarly above libp2p.
- **IPFS:** explicitly Sybil-tolerant — public DHT accepts all peers, accelerated client trusts no one.

**For Myrhiza:** same as iroh, no global Sybil floor inherited. Per-app membership proofs (capability-token-gated, social-graph-attested, fee-paying) live at the layer above the transport. See [`../iroh/lessons.md`](../iroh/lessons.md) §"Sybil resistance is none, by design."

## Identity portability: none

PeerId = `multihash(public_key)`. Lose the keypair, lose the identity. libp2p does not provide:

- Key rotation (a new key = a new PeerId).
- Multi-device identity (each device has its own PeerId).
- Recovery from key loss (no FROST, no threshold signatures, no social recovery).
- Sub-keys / delegated authority that can be revoked.

Same as iroh's "NodeID = identity is a category error" critique ([`../iroh/critiques.md`](../iroh/critiques.md) §"Identity is just a public key — portability gap").

**Mitigation patterns:**

- Application-layer identity systems (Status, Nostr, ENS) layer over libp2p and provide their own portable identity.
- Some libp2p users (Ethereum stakers) accept "validator key = single-device identity" with operational discipline (cold storage of mnemonic, hot wallet rotation, etc.).

**For Myrhiza:** the same architectural split as iroh — PeerPubkey (transport credential, per-device) vs PrincipalID (application identity, portable, multi-device, recoverable). PrincipalID lives at a Myrhiza-defined layer above the transport.

## Discoverability ("how do strangers meet?")

libp2p has *address resolution* (DHT lookup by PeerId, identify, signed peer records) but not *peer finding* in the social sense — "I don't know any PeerIds; how do I find one?"

The libp2p answer:

- **Bootstrap peers** — every implementation ships a hardcoded list (e.g. `bootstrap.libp2p.io`, the IPFS bootstrap nodes). You start with these.
- **DHT walking** — once connected to a bootstrap, walk the DHT to find more peers.
- **mDNS** — find local-network peers.
- **Rendezvous** — register at a known rendezvous server with an interest tag; others querying that tag find you.

But none of these solve the cold-start "I want to find peers who share my interest in topic X, and I don't know anyone who's in that topic." That's an out-of-band problem (QR code, share link, app store, web search).

**For Myrhiza:** same as iroh, deferred to the application or to a Myrhiza-designed discovery primitive. The bootstrap-peer pattern works but is operationally identical to iroh's "the four n0-operated relays" — someone is operating those bootstrap nodes.

## Browser-native pure-P2P: partial

libp2p has the **best browser-native P2P story in the ecosystem** — WebRTC + WebTransport with certhash. But it's still not perfect:

- WebRTC requires signaling. The signaling channel can be a libp2p stream (no dedicated server) but only if both peers are already connected to a common third peer.
- WebTransport requires a server (browser-to-server, not browser-to-browser).
- Both transports' browser support varies (Safari is laggy).
- WebRTC's underlying ICE/STUN/TURN infrastructure is operationally heavy.

The unsolved problem: **two browsers, no prior connection, no server, find each other and connect**. This is genuinely hard; libp2p doesn't solve it. The current best answer: WebRTC + a signaling server (which is centralization-equivalent to iroh's relay).

**For Myrhiza:** we inherit iroh's "relay-only browser story" — *worse* than libp2p's WebRTC story for true browser-to-browser. If Myrhiza ever needs in-browser peers (a Myrhiza app running entirely in a browser tab, not as a UI client of a native kernel), libp2p's WebRTC is the closest thing to a solution.

## Plumtree-without-scoring (Myrhiza-specific)

This is **the most Myrhiza-relevant open problem** in this folder.

Iroh-gossip uses HyParView + Plumtree (Leitão et al., 2007), not gossipsub. Plumtree assumes the overlay is benign-with-churn; it has **no built-in peer scoring**. At internet scale with adversarial peers, this is a known gap. The gossipsub paper's whole point is that Plumtree-without-scoring is exploitable; gossipsub adds scoring as the load-bearing defence.

**Mitigation patterns Myrhiza may need to build:**

- Per-peer payload-validity tracking (analog of gossipsub's P4 invalid-messages score).
- IP-colocation detection (analog of P6).
- App-level signed-reputation (analog of P5).
- A mesh-rebalancing heartbeat that prunes low-score peers (gossipsub's primary defence).

None of this is in iroh-gossip today. Either Myrhiza builds it in a layer above iroh-gossip, or Myrhiza accepts the benign-overlay assumption. The latter is fine for small swarms (~thousands of peers per topic, per iroh-gossip's documented scope) but breaks at internet scale.

## Wire-spec single-implementation drift

Some libp2p protocols are de-facto specified by go-libp2p's behavior, with other implementations chasing. The libp2p/specs README acknowledges this; the lifecycle stage system tries to formalize it (3A Recommendation requires ≥2 interoperable implementations). But some sub-protocols have shipped at 2A Candidate Recommendation with only one implementation having production-quality support:

- WebRTC (rust-libp2p alpha; go has WebRTC-Direct but not full WebRTC).
- WebTransport (browser-side js; server-side go + experimental rust).

For Myrhiza: this is a comparable risk to iroh's "single-implementation with no published wire spec" but in a less severe form (libp2p has *a* spec, just with implementation-quality variance).

## DHT performance at scale

Documented in [`discovery.md`](discovery.md) and [`critiques.md`](critiques.md). Cold provider-record lookups in the IPFS DHT take 10–60 seconds. The libp2p team has invested in [accelerated DHT client](https://blog.ipfs.io/2023-09-13-accelerated-dht-client/) and other optimisations, but the fundamental cost — `O(log N · k · roundtrips)` for Kademlia walks — is not eliminable.

For Myrhiza: if we ever need content-addressed peer discovery at IPFS scale, we inherit this problem. The mitigation is to *not* run a public DHT — keep discovery scoped to per-app communities or to a curated indexer layer.

## multistream-select v2 has been "almost shipped" for years

The latency overhead of multistream-select on TCP-Noise-yamux (~3 RTTs to open the first stream) is a known cost with a known fix: optimistic protocol selection in v2. The spec has stayed at draft status without shipping. The pragmatic answer is "use QUIC where you can; pay the cost where you can't." For Myrhiza, QUIC-first inheritance from iroh avoids this entirely.

## NAT traversal success rate ceiling

DCUtR-style coordinated hole punching caps at ~70% in field conditions ([blog data, Jan 2022](https://blog.libp2p.io/2022-01-20-libp2p-hole-punching/)). The remaining 30% (symmetric NATs, CGNs, restrictive firewalls) **cannot be punched** with current techniques — they will always require relay fallback.

This isn't a libp2p deficiency; it's a structural fact of how the internet is NAT'd. iroh has the same problem and the same mitigation (relay fallback). The difference is iroh's defaults are tuned for "always relay first, race direct" (so the user never sees a connection failure, only a slower connection); libp2p's defaults are tuned for "direct first, relay on failure" (which exposes the 30% failure mode if relays aren't configured).

## Browser idle / wake / background

Same as iroh ([`../iroh/open-problems.md`](../iroh/open-problems.md)) — browsers and mobile OSes aggressively suspend background tabs and background-app processes. libp2p connections die under suspension; on wake, peers must reconnect. There's no "idle but maintainable" state.

**Mitigation patterns:**

- Server-side state durability (the peer disappears, the server remembers).
- Push notifications to wake the peer (mobile only).
- Quick-reconnect protocols (libp2p has some — identify-push, connection-manager-graceful-disconnect — but they're operational, not architectural fixes).

## Per-implementation feature drift

The implementations are not interchangeable. As documented in [`implementations.md`](implementations.md):

- WebRTC: production in js, alpha in rust, partial in go.
- IDONTWANT (gossipsub v1.2): present in 4 of 7 impls.
- AutoNAT v2: only in development.

For Myrhiza this means: even though libp2p has 7 implementations, you can't realistically pick "the one that runs natively in my stack" and assume feature parity with the rust + go canonical implementations.

## Implications for Myrhiza

- **Sybil + identity portability + discoverability are Myrhiza-layer problems**, same as in iroh. Document explicitly that neither transport solves them. Design Myrhiza's PrincipalID + capability-token + discovery layers without expecting the transport to help.
- **Plumtree-without-scoring is the Myrhiza-specific risk.** The gossipsub paper's existence is the evidence that this matters at scale. If Myrhiza ever ships at internet scale on iroh-gossip's current Plumtree, we will hit this problem. Plan for it: either embed app-level scoring above iroh-gossip, or accept the small-swarm constraint.
- **DHT performance + NAT 70%-ceiling are inherited problems** of any P2P stack. Don't expect to do better than libp2p / iroh on these.
- **multistream-select latency** is avoided by Myrhiza's QUIC-first inheritance from iroh. Good. Don't let backwards-compat ever introduce a multi-layer-negotiation stack.
- **The "browser-native pure-P2P is partial" gap** is real and Myrhiza inherits the worse version of it from iroh. If browser-as-peer becomes a Myrhiza use case, libp2p's WebRTC is the comparison study.

## Sources

- [libp2p spec lifecycle document](https://github.com/libp2p/specs/blob/master/00-framework-01-spec-lifecycle.md)
- [libp2p hole-punching blog](https://blog.libp2p.io/2022-01-20-libp2p-hole-punching/)
- [libp2p accelerated DHT client blog](https://blog.ipfs.io/2023-09-13-accelerated-dht-client/)
- [GossipSub paper (arXiv:2007.02754)](https://arxiv.org/abs/2007.02754) — the v1.1 peer-scoring section explicitly motivates "Plumtree at scale is exploitable"
- [iroh — open problems (sibling doc)](../iroh/open-problems.md)
- [iroh — critiques (sibling doc)](../iroh/critiques.md)
