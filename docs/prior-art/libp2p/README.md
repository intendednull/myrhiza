**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — Protocol Labs' modular P2P networking stack (Rust / Go / JS / Nim / C++ / Python, gossipsub, Kademlia, Noise, QUIC, WebRTC)

# libp2p

libp2p is a modular networking stack for peer-to-peer applications, started in 2015 inside IPFS and split out as an independent project in 2016. It is **a** standard (not "the" standard) for P2P — the most-deployed alternative to iroh, Tor, Hyperswarm, BitTorrent, and direct UDP/TCP. Its largest deployment is the **Ethereum consensus layer** (Prysm, Lighthouse, Teku, Nimbus, Lodestar — all five major beacon-chain clients use libp2p), followed by **Filecoin** (the libp2p stack's original Protocol-Labs flagship at scale), **IPFS** (`kubo` is the canonical go-libp2p user), and a long tail of research-grade and production-grade users including Polkadot/Substrate (Parity wrote rust-libp2p), Lotus, Status (chat + Waku), Codex, and Nimbus.

Unlike iroh — which picked the transport layer and stopped — libp2p ships **DHT + pubsub + RPC + multiple transports + identity + multiplexing + service discovery** in one umbrella. That breadth is the system's central strength (it composes the entire P2P substrate) and the central critique levelled at it (it ships "one of everything" with the configuration burden that implies). This corpus is for Myrhiza spec authors who need to understand what libp2p does, what it costs, and where its design decisions cut differently from iroh's.

For Myrhiza, libp2p is **not a load-bearing dependency** — we chose iroh. But its **algorithms are load-bearing.** Myrhiza adopts epidemic-gossip pub/sub via `iroh-gossip` (which uses HyParView + Plumtree, not gossipsub itself), and the gossipsub paper + spec are the most-studied, most-attacked production gossip protocol in the P2P literature. Kademlia is the canonical DHT design; Noise is the canonical handshake. The Myrhiza spec author needs a curated reading of all three even though we don't link `libp2p-*` crates.

## Key facts

| Fact | Value |
|---|---|
| Origin | Spun out of [`go-ipfs`](https://github.com/ipfs/kubo) in 2015–2016; the [`libp2p/libp2p`](https://github.com/libp2p/libp2p) umbrella repo was created 2016-06-18 |
| Founder | [Juan Benet](https://github.com/jbenet) (Protocol Labs founder; same as IPFS/Filecoin/multiformats) |
| Stewards | Protocol Labs (primary); contributions from Parity (rust-libp2p origin), Status / Vac (nim-libp2p), Soramitsu (cpp-libp2p), ChainSafe (gossipsub TS port + jvm-libp2p), Eiger (WebTransport-websys) |
| Specs repo | [`libp2p/specs`](https://github.com/libp2p/specs) (1.8k stars, lifecycle stages 1A → 3A; spec status: still incomplete — README header says "currently incomplete, working to address this") |
| Implementations | **5 production:** go-libp2p, rust-libp2p, js-libp2p, nim-libp2p, jvm-libp2p (Kotlin). **3 experimental / partial:** py-libp2p ("v1.0 Coming Soon"), cpp-libp2p (Soramitsu), C/Swift stubs. |
| License (per repo, verified) | go-libp2p: **MIT**; rust-libp2p: **MIT** (every crate); js-libp2p: **Apache-2.0 OR MIT** (dual); nim-libp2p: **MIT**; cpp-libp2p: **Apache-2.0**; jvm-libp2p: **Apache-2.0**; py-libp2p: **MIT/Apache-2.0** |
| Current versions (verified 2026-05-22) | **go-libp2p 0.48.0** (2026-03-17); **rust-libp2p 0.56.0** on crates.io (2025-06-27), master HEAD at 0.57.0 unreleased; **js-libp2p 3.3.1** (npm `latest`); **nim-libp2p 1.15.3**; **jvm-libp2p** see [`implementations.md`](implementations.md) (Maven version published per-release, not pinned in this folder) |
| gossipsub spec | v1.0 (2020-03), v1.1 with peer scoring (r8 2021-12-14), v1.2 with `IDONTWANT` (Working Draft, 2023-07). **v1.1 is the production version everywhere.** Spec lifecycle: v1.0 = 3A Recommendation; v1.1 = 2A Candidate Recommendation; v1.2 = 1A Working Draft. |
| gossipsub paper | Vyzovitis, Napora, McCormick, Dias, Psaras. "GossipSub: Attack-Resilient Message Propagation in the Filecoin and ETH2.0 Networks." arXiv:2007.02754, 2020-07-06. |
| Kademlia | XOR distance over `sha256(key)`. `k=20`, `α=10` (concurrency). Provider records: 22h republish / 48h expire (IPFS). Spec: `kad-dht` r2, 2022-12-09. Recommended reading: Maymounkov & Mazières 2002. |
| Noise | `Noise_XX_25519_ChaChaPoly_SHA256` is the libp2p default + only-supported pattern. Spec: `noise` r5, 2022-12-07. Authentication: distinct Noise key + identity key, identity signs Noise static. |
| QUIC | RFC 9000 via `/quic-v1` multiaddr; legacy draft-29 via `/quic` being phased out. Spec: `quic` r1, 2022-12-30. ALPN: `libp2p`. TLS-based peer auth. |
| WebRTC | **Three flavors** — `webrtc` (browser ↔ browser direct, full WebRTC), `webrtc-direct` (browser ↔ server, no trusted-CA cert needed), `webrtc-star` (legacy, STUN/TURN-mediated). Spec: `webrtc` r1, 2023-04-12 (Candidate Recommendation). |
| Identity | `PeerId` = multihash of the public key (RSA / Ed25519 / Secp256k1 / ECDSA). Default Ed25519 in all modern implementations. No DID layer. |
| Multiaddr | Self-describing address format: `/ip4/1.2.3.4/udp/4001/quic-v1/p2p/12D3KooW...`. Standardized in [`multiformats/multiaddr`](https://github.com/multiformats/multiaddr). |

## Contents

Each file is independently skimmable. Cross-links land at the relevant sub-section.

**Algorithms (the load-bearing algorithmic content)**
- [**Gossipsub**](gossipsub.md) — mesh-based pub/sub, peer scoring, IDONTWANT (v1.2), Plumtree comparison; **the most-load-bearing file for Myrhiza** because iroh-gossip's Plumtree algorithm sits in the same design space.
- [**Discovery**](discovery.md) — Kademlia DHT (k=20, α=10, XOR-on-sha256), mDNS, rendezvous, AutoNAT, relay (Circuit Relay v2), DCUtR hole-punching.
- [**Transports**](transports.md) — TCP / QUIC / WebSocket / WebRTC / WebTransport; multistream-select; security upgrades (Noise / TLS); muxers (yamux / mplex).

**Architecture**
- [**Architecture**](architecture.md) — Host / Swarm / NetworkBehaviour; the protocol-upgrade pattern; transport composition; multistream-select.
- [**Identity & crypto**](identity.md) — PeerId, multihash, key types, Noise XX, libp2p-TLS.

**Implementations + ecosystem**
- [**Implementations**](implementations.md) — go-libp2p / rust-libp2p / js-libp2p / nim-libp2p / cpp-libp2p / jvm-libp2p / py-libp2p side-by-side (feature parity, license, license drift, license-of-record).
- [**Apps + production users**](apps.md) — Ethereum consensus layer (Prysm/Lighthouse/Teku/Nimbus/Lodestar), Filecoin/Lotus, IPFS/kubo, Status, Polkadot/Substrate, Waku, Drand.
- [**Governance**](governance.md) — Protocol Labs stewardship, the 2024 PL restructuring, working-group model, spec lifecycle stages.
- [**History**](history.md) — IPFS → libp2p split (2015–16), the Parity rust-libp2p era, the ETH2 + Filecoin drive, the iroh fork-out (2023), the 2024 maintenance reset.

**Project lens**
- [**Critiques**](critiques.md) — third-party critiques (iroh's "Why not libp2p?", the configurability complaint, the hole-punching 70% claim, the security advisory record).
- [**Open problems**](open-problems.md) — what libp2p structurally doesn't solve (browser-native pure-P2P, Sybil, identity portability, single canonical spec).
- [**Lessons for Myrhiza**](lessons.md) — **the consult-this-when-designing decision file.** validates / avoid / borrow.

## How to use

- Designing Myrhiza's pub/sub layer → read [`gossipsub.md`](gossipsub.md) first, then [`../iroh/gossip.md`](../iroh/gossip.md) to compare Plumtree vs gossipsub.
- Designing peer discovery → read [`discovery.md`](discovery.md) for Kademlia's full design, then `lessons.md` for what we adopt vs defer.
- Choosing a transport stack → read [`transports.md`](transports.md). Myrhiza inherits iroh's QUIC-first stance; libp2p's TCP/WebSocket/WebRTC story is useful for contrast.
- Comparing libp2p to iroh → read [`critiques.md`](critiques.md) and [`../iroh/critiques.md`](../iroh/critiques.md) §"Why not libp2p?" — they form a pair.
- Auditing the dependency landscape if Myrhiza ever revisits the iroh-vs-libp2p choice → start at [`lessons.md`](lessons.md) and walk up.

**Framing disclosure.** This corpus is written from an iroh-as-primary-transport, gossipsub-as-algorithm-reference stance. The "Implications for Myrhiza" sub-sections frame libp2p's choices through that lens — specifically: where libp2p's algorithms (gossipsub, Kademlia, Noise) are worth studying even though we don't link the libp2p Rust crates. Future readers auditing whether the iroh-over-libp2p choice itself was correct should weigh this corpus accordingly: it is a learn-from-libp2p-into-Myrhiza artifact, not a neutral catalog or a libp2p advocacy document. The iroh team's framing (in [`../iroh/critiques.md`](../iroh/critiques.md)) is reflected here where their critique is fair and surfaced separately where it is marketing.

## Sources

- [libp2p homepage](https://libp2p.io/)
- [libp2p docs](https://docs.libp2p.io/)
- [libp2p/specs repository](https://github.com/libp2p/specs)
- [libp2p/libp2p umbrella repository](https://github.com/libp2p/libp2p)
- [GossipSub paper (arXiv:2007.02754)](https://arxiv.org/abs/2007.02754)
- [Kademlia paper (Maymounkov & Mazières 2002)](https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf)
- [iroh — Comparing iroh & libp2p (Jan 2024)](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
- [Myrhiza prior-art: iroh — critiques §"Why not libp2p?"](../iroh/critiques.md#why-not-libp2p--pure-p2p-pushback)
- [Myrhiza spec: Plan B-4.1 — Iroh-gossip subscribe + publish](../../specs/2026-05-20-plan-b-4-1-iroh-gossip-design.md)
