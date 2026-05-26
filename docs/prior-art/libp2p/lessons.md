**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — lessons for Myrhiza (validates / avoid / borrow)

# Lessons for Myrhiza

This is the consult-this-when-designing file. The other libp2p prior-art files are evidence; this file is decisions.

libp2p is **not a load-bearing dependency** for Myrhiza — we picked iroh. The lens here is therefore different from the iroh file's: "what does libp2p's existence + experience tell us about our own design, even though we don't link the libp2p crates?" The answer is: a lot, especially in the algorithm + protocol-design space. Gossipsub, Kademlia, and Noise are the most-load-bearing algorithm references in P2P; understanding them shapes how we design above iroh.

Each section is paired with the iroh file's equivalent — for many lessons, iroh and libp2p say the same thing from different starting points; for some, they disagree and we have to pick.

## Validates

Myrhiza design choices that libp2p's experience supports.

- **Pubkey-as-transport-identity (Ed25519, no DID layer).** libp2p's PeerId is `multihash(public_key)`, same shape as iroh's EndpointId, same shape as Myrhiza's PeerPubkey. Universal across modern P2P stacks. See [`identity.md`](identity.md). Mirrors [`../iroh/lessons.md`](../iroh/lessons.md) §Validates row 1.
- **QUIC-first transport.** libp2p's spec explicitly recommends QUIC (*"RECOMMENDED that libp2p implementations offer QUIC as one of their transports"*); both iroh and Myrhiza commit harder to QUIC-only. The 3-RTT cost of TCP-Noise-yamux is the cautionary tale. See [`transports.md`](transports.md), [`architecture.md`](architecture.md). Mirrors [`../iroh/lessons.md`](../iroh/lessons.md) §Validates row 2.
- **Consensus is impossible at the transport layer.** libp2p doesn't claim consensus either; its primitives (gossipsub, Kademlia) are eventually-consistent best-effort. The transport carries bytes; convergence is the application's problem. Mirrors iroh's [Consensus is Impossible](https://www.iroh.computer/blog/consensus-is-impossible) framing.
- **Mesh-based pub/sub is the right shape for "many subscribers, bounded fan-out."** Gossipsub's mesh model (`D≈6` per topic) is well-tuned and at-scale validated by Ethereum. Iroh-gossip's Plumtree is a sibling design in the same family. Myrhiza's adoption of epidemic gossip via iroh-gossip is supported by gossipsub's success. See [`gossipsub.md`](gossipsub.md).
- **Peer scoring is the load-bearing primitive for adversarial pub/sub.** Gossipsub v1.1's score function (P1-P7) is the canonical reference for "how to defend pub/sub against Sybil + eclipse + cover attacks." Any Myrhiza app at scale will need a similar layer. See [`gossipsub.md`](gossipsub.md) §"Peer scoring."
- **Kademlia (XOR distance over `sha256(key)`, k=20, α=10) is the right DHT shape.** Iroh's pkarr-on-Mainline-DHT is a Kademlia-flavored substrate. If Myrhiza ever needs its own DHT, the libp2p Kademlia parameter set is well-tuned. See [`discovery.md`](discovery.md).
- **Noise XX is the right libp2p / Myrhiza handshake at the TLS / Noise layer.** `Noise_XX_25519_ChaChaPoly_SHA256` — mutual auth, no preknown keys, the canonical pattern for P2P. See [`identity.md`](identity.md). iroh's QUIC-TLS uses the same Ed25519-key-signs-cert pattern; libp2p's libp2p-TLS spec is the more elaborate variant.
- **Multiaddr is a good peer-share format.** Self-describing, extensible. iroh's EndpointTicket is a constrained subset. Myrhiza's external-share format should follow this shape if we ever need to extend beyond iroh's ticket. See [`architecture.md`](architecture.md).
- **Cross-implementation interop testing (libp2p/test-plans) is the right discipline if Myrhiza ever becomes multi-impl.** Continuous CI across implementations is the only realistic way to maintain interop. Single-impl projects can be sloppier with specs; multi-impl can't. See [`implementations.md`](implementations.md).
- **Identify (post-handshake protocol exchange) is a useful primitive even if Myrhiza doesn't ship libp2p's version verbatim.** A "what protocols do you support, what addresses can I reach you on, what agent are you?" exchange is good hygiene. Mirrors patterns in Myrhiza's capability handshake. See [`architecture.md`](architecture.md) §"Identify."

## Avoid

Pitfalls libp2p's experience reveals — and how Myrhiza's iroh-inherited stack avoids them.

| Pitfall | Source | Myrhiza mitigation |
|---|---|---|
| **multistream-select latency on multi-layer stacks.** TCP-Noise-yamux opens a stream in ~3 RTTs of nested negotiation. v2 has been "almost shipped" for years. | [`architecture.md`](architecture.md), [`critiques.md`](critiques.md) | We inherit iroh's QUIC-only stack with ALPN-based protocol multiplexing. Single negotiation, single RTT. **Don't ever introduce a multi-layer-negotiation stack** for backwards compat. |
| **Configuration burden.** libp2p has dozens of config knobs per peer. Defaults are sensible but tuning for production takes weeks. | [`critiques.md`](critiques.md) | Myrhiza's iroh-derived API surface is narrow — kernel-mediated capability boundary means app developers don't even see most of the transport config. Keep it narrow. Resist any urge to "expose tunables" past the kernel boundary. |
| **DHT performance cliff at scale.** Cold provider lookups take 10–60s on the IPFS public DHT. Kademlia's asymptotics + IPFS's churn = real cost. | [`discovery.md`](discovery.md), [`open-problems.md`](open-problems.md) | Don't run a public DHT. Keep discovery scoped per-app (gossipsub-style topic membership) or via curated indexer-shape relay nodes. If Myrhiza ever needs content-addressed peer discovery, study the [accelerated DHT client](https://blog.ipfs.io/2023-09-13-accelerated-dht-client/) pattern first. |
| **Hole-punching ~70% ceiling.** DCUtR + AutoNAT + Circuit Relay + UPnP all combined: ~70% direct-connection success in field conditions. Remaining 30% always needs relay. | [`discovery.md`](discovery.md), [`critiques.md`](critiques.md), [`open-problems.md`](open-problems.md) | Don't promise pure-P2P-no-relay. Iroh's "relay-first, race direct, never expose failure to user" stance is the right framing. Document relay infrastructure as a first-class operational concern, not a fallback. |
| **Plumtree-without-scoring is exploitable at scale.** Iroh-gossip's HyParView+Plumtree is gossipsub's sibling but lacks the v1.1 score function — gossipsub-v1.1 exists *because* Plumtree-without-scoring was attackable in Filecoin's mainnet. | [`gossipsub.md`](gossipsub.md), [`open-problems.md`](open-problems.md) | Document explicitly that **iroh-gossip is not adversary-resistant out of the box**. Either build app-level peer scoring above it (validity rates, IP colocation detection, signed reputation), or constrain Myrhiza apps to "a few thousand peers per topic" scope where benign-overlay assumptions hold. |
| **WebRTC alpha in rust-libp2p.** `libp2p-webrtc` has been at `0.9.0-alpha.1` for ~year. Production WebRTC is js-only. | [`transports.md`](transports.md), [`implementations.md`](implementations.md) | We inherit iroh's "relay-only browser story" — *worse* than libp2p's WebRTC for browser-to-browser, but acceptable for the "kernel-mediated app component" use case Myrhiza targets. If browser-native peer-to-peer becomes a Myrhiza requirement, the rust ecosystem is genuinely thin and we'd be writing the WebRTC stack ourselves. |
| **Spec incompleteness is admitted.** libp2p/specs README: "specifications are currently incomplete." Some core protocols are implementation-defined. | [`critiques.md`](critiques.md), [`governance.md`](governance.md) | Myrhiza must spec everything that crosses a peer boundary. Don't fall into the "spec lags implementation" pattern — even with a single canonical kernel, the spec is the determinism contract. |
| **PeerId = transport identity, not application identity.** Lose the keypair, lose the PeerId. Same problem iroh has. libp2p does not solve key rotation, multi-device, or recovery. | [`identity.md`](identity.md), [`open-problems.md`](open-problems.md) | Separate Myrhiza's PrincipalID (application identity, recoverable, multi-device, rotatable) from PeerPubkey (transport credential, per-device). PrincipalID lives in a Myrhiza-defined layer above iroh. Identical lesson to [`../iroh/lessons.md`](../iroh/lessons.md) §Avoid row 6. |
| **mplex deprecated 2024.** Vulnerable to memory-exhaustion attacks because no flow control. Removed from libp2p defaults. | [`transports.md`](transports.md) | If Myrhiza ever needs an explicit muxer (it shouldn't — QUIC streams suffice), it must have window-based backpressure. The yamux design is the model; mplex is the anti-pattern. |
| **Multi-stakeholder governance is slower than single-vendor governance.** libp2p evolves slowly across 6 implementations. Iroh ships breaking changes monthly. | [`governance.md`](governance.md), [`history.md`](history.md) | Myrhiza is single-stewarded today (Rust kernel, jco browser kernel). That gives us iroh's velocity. If Myrhiza ever multi-implements, we inherit libp2p's coordination cost — that's a real tradeoff to plan around if it ever becomes relevant. |
| **Decoupled gossipsub-spec maintainer (vyzo / ChainSafe / Vac) creates implementation drift.** Different impls ship v1.2 IDONTWANT at different rates. | [`implementations.md`](implementations.md) | Myrhiza is single-impl per profile (one canonical kernel for state-apply); spec-vs-implementation drift is internal, not cross-impl. Mostly OK; the discipline still applies — write specs, don't infer them from code. |
| **Browser-native peer-to-peer needs WebRTC + signaling server in practice.** Even libp2p's best browser story still needs a signaling channel — usually a libp2p stream over a relay. | [`transports.md`](transports.md), [`open-problems.md`](open-problems.md) | Myrhiza's browser kernel inherits iroh's "relay-only" — *worse* on this axis. Document the gap explicitly: if Myrhiza ever needs true browser-to-browser without any server (including without a Myrhiza relay), neither stack solves it. |

## Borrow

Concrete primitives + patterns worth studying or replicating in Myrhiza's design.

1. **Gossipsub v1.1 peer-scoring shape (P1-P7).** The seven score parameters are the canonical reference for "how to score peers in a pub/sub mesh." If Myrhiza ever builds a peer-scoring layer above iroh-gossip's Plumtree, the parameter set is the starting point. Especially: P6 IP colocation (defend against single-IP Sybil), P5 application score (let the app inject signed reputation), P3 mesh-delivery-rate (catch lazy peers). See [`gossipsub.md`](gossipsub.md) §"Peer scoring."
2. **Gossipsub v1.2 IDONTWANT mechanism.** Tell mesh peers "I already have message X" to reduce duplicate-payload amplification. Iroh-gossip's Plumtree already has a related mechanism (lazy IHAVE) but if Myrhiza's app ever pushes large payloads through gossip, IDONTWANT-shape duplicate suppression is the right tool. See [`gossipsub.md`](gossipsub.md) §"Versions."
3. **PeerId encoding pattern.** `multihash(public_key)` with `identity` multihash for short keys (≤42 bytes) and `sha2-256` for longer keys. The "identity multihash" optimisation means most PeerIds are the public key with a length prefix. Cleaner than iroh's bare 32-byte EndpointId in one sense (extensible to other key types) but heavier (multihash framing). Myrhiza's PeerPubkey is fine as-is, but the multihash framing is worth considering if Myrhiza ever needs to support non-Ed25519 keys.
4. **multiaddr (self-describing address format).** `/ip4/.../udp/.../quic-v1/p2p/12D3KooW...` — composable, extensible, parses cleanly. Iroh's EndpointTicket is a base32-encoded subset. Myrhiza's external-share format should adopt multiaddr-like structure if we ever extend beyond iroh's ticket.
5. **AutoNAT-style reachability probing.** "Ask other peers to dial me back" — 5 messages, returns Public/Private/Unknown. The kernel could expose this to apps to drive UI ("you're behind a NAT, here's what that means"). See [`discovery.md`](discovery.md) §"AutoNAT."
6. **identify (post-connection protocol exchange).** Cheap, async, bidirectional. After Myrhiza peers connect, both sides exchange "what versions of what protocols do you speak?" The capability handshake at the Myrhiza-app layer could follow this shape — but at the app capability layer, not at the transport layer.
7. **Spec lifecycle stages (1A/2A/3A).** Working Draft → Candidate Recommendation → Recommendation. Myrhiza's `docs/specs/` could adopt a similar label set — explicit about which specs are sketches vs final. Cheap to add; clarifies status for readers. See [`governance.md`](governance.md) §"Spec lifecycle stages."
8. **libp2p/test-plans cross-impl interop CI.** Continuous Docker-based "every impl talks to every other impl" testing. If Myrhiza ever ships a non-Rust kernel (Swift mobile, Kotlin Android, jco WASM), a test-plans-style CI is the discipline that keeps them honest. See [`implementations.md`](implementations.md) §"Interop testing."
9. **Signed peer records.** Bundle of `(peer_id, addresses, seq, timestamp)` signed by the peer's identity key. Forward-able without trust — peer A can give peer B's record to peer C without C trusting A. Same as iroh's pkarr-on-Mainline-DHT signed records, but with explicit composition into PX (peer exchange) flows. See [`discovery.md`](discovery.md) §"Identify-observed address" and [`gossipsub.md`](gossipsub.md) §"Peer exchange."
10. **Worked-example documentation pattern.** The walkthrough in [`apps.md`](apps.md) (kubo IPFS publishing + fetching a CID end-to-end) is the right shape for a Myrhiza spec author to understand what each layer actually does. Every Myrhiza spec touching multi-peer interaction should have a concrete "here's how an app actually does X" walkthrough.

## How to use this file

When designing a Myrhiza feature that touches pub/sub, peer discovery, or peer identity:

1. **Find the row in Avoid** that names a pitfall close to your design. Read the linked subsystem file for full evidence.
2. **Find the entry in Borrow** that names a primitive close to what you're designing. Cross-reference with [`../iroh/lessons.md`](../iroh/lessons.md) — libp2p and iroh often have the same primitive in different framings, and Myrhiza inherits iroh's by default.
3. **Promote any decision into a Myrhiza spec** under `docs/specs/`. This file captures what we learn from prior art; the spec is where Myrhiza decisions live.

When libp2p ships a major change (gossipsub v2, multistream-select v2, a substantive WebRTC update), update the affected subsystem files + this file's date. Especially watch:

- gossipsub v2 (if it ever arrives) — the next-generation peer-scoring + amplification-reduction design.
- multistream-select v2 — the latency fix that's been pending for years.
- rust-libp2p `libp2p` 0.57+ release — the long-pending stable that's been on master for 11 months.

## Sources

- [`gossipsub.md`](gossipsub.md) — the canonical reference for peer-scoring + mesh-based pub/sub.
- [`discovery.md`](discovery.md) — Kademlia DHT + AutoNAT + DCUtR + Circuit Relay.
- [`transports.md`](transports.md) — QUIC / WebRTC / WebTransport.
- [`identity.md`](identity.md) — PeerId + Noise XX + libp2p-TLS.
- [`architecture.md`](architecture.md) — Host/Swarm/multistream-select.
- [`critiques.md`](critiques.md) — third-party honest assessments.
- [`open-problems.md`](open-problems.md) — what libp2p structurally doesn't solve.
- [`../iroh/lessons.md`](../iroh/lessons.md) — the paired iroh decisions file.
- [GossipSub paper (arXiv:2007.02754, Vyzovitis et al. 2020-07-06)](https://arxiv.org/abs/2007.02754)
- [Kademlia paper (Maymounkov & Mazières 2002)](https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf)
- [Plumtree paper (Leitão et al. 2007)](https://asc.di.fct.unl.pt/~jleitao/pdf/srds07-leitao.pdf)
- [Myrhiza Plan B-4.1 — Iroh-gossip subscribe + publish](../../specs/2026-05-20-plan-b-4-1-iroh-gossip-design.md)
