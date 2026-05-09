**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — comparisons against neighboring P2P / overlay systems

# Comparisons

Where Iroh sits relative to the systems a Myrhiza spec author might consider as alternatives or complements. For each comparison: what Iroh actually does, what the neighbor actually does, where they overlap, where they don't, and which is the right tool for which job.

## vs libp2p

The most important comparison. Iroh is **not** a libp2p fork — `beetle` (the original 2022 iroh) used `rust-libp2p` to interop with kubo, but the post-2023 iroh deliberately **left libp2p behind**. The team's own framing ([Comparing iroh & libp2p](https://www.iroh.computer/blog/comparing-iroh-and-libp2p), b5, Jan 5, 2024) is verbatim:

> *"Libp2p is built to keep its reliance on central points of failure at an absolute minimum, which comes at the cost of effectiveness. Iroh is built to maximize effectiveness, which comes at the cost of a little centralization."*

What that translates to mechanically:

| Axis | libp2p | Iroh |
|---|---|---|
| Transport set | Many — TCP, QUIC, WebSocket, WebTransport, WebRTC, … pluggable | One — QUIC over UDP (since 0.97, n0's `noq`) with relay-over-HTTPS fallback |
| Stream multiplexing | yamux / mplex over the chosen transport | QUIC-native (one stack, no second mux) |
| Protocol negotiation | `multistream-select` (per-stream, length-prefixed strings) | **ALPN** (TLS-native, one byte per protocol) |
| Endpoint shape | Multiple swarms, transport-per-protocol | **Single `Endpoint`** owns all transports; per-connection `Connection` |
| NAT traversal success | Reportedly capped ~70% in field tests | Higher — uses Tailscale-pioneered DERP-style relay fallback |
| Relay model | Circuit-relay-v2 (peer-operated, decentralized) | n0-operated default relays + private/self-hosted; e2ee through relay |
| Identity | PeerID = hash of public key (multi-curve) | NodeID = raw ed25519 public key (32 bytes) |
| DHT | Kademlia (native) | None (use Mainline DHT externally if needed) |
| Pubsub | gossipsub (native) | Spinout: `iroh-gossip` (separate crate) |
| Content addressing | bitswap (native) | Spinout: `iroh-blobs` (BLAKE3-verified streaming) |
| Browser story | Mature (js-libp2p, WebTransport, WebRTC) | In progress — alpha browser support since 0.32 (Feb 2025) |
| Wire spec | Multiple specs at [github.com/libp2p/specs](https://github.com/libp2p/specs) | Not yet published as a separate document (committed for 1.0) |

**The single most important architectural difference.** libp2p assumes you may run many transports at once and must negotiate them per-connection. Iroh assumes one transport (QUIC) is enough if you make it really good and route around its failures (relay fallback, multipath, noq). The libp2p design optimizes for *transport pluralism*; the iroh design optimizes for *operational simplicity* and *connection success rate*. For a Myrhiza app developer: an iroh-shaped API is *one `Endpoint`*; a libp2p-shaped API is *one `Swarm` per transport set you care about*.

**ALPN vs multistream-select.** ALPN is in the TLS handshake — protocol negotiation happens during the connection establishment, encrypted, no extra round-trips. Multistream-select happens after the secure channel is up, with per-stream string negotiation. ALPN is faster, simpler, and standard; the cost is you can't do "negotiate over plaintext, then encrypt." For QUIC (always-encrypted) this cost is zero.

**Where libp2p is genuinely ahead.** Native DHT, native pubsub, mature browser story, broader transport set, multi-org maintainer pool. If Myrhiza needs a Kademlia DHT for content discovery, libp2p has it on-the-shelf and iroh does not.

**The team's honest summary** ([HN comment](https://news.ycombinator.com/item?id=44383072), b_fiive): *"less configuration. more reliable. less pure p2p (iroh uses relays)"*.

## vs Hypercore / Holepunch / Pears

Different shape. Hypercore (now stewarded by Holepunch / "Pears") is an **append-log abstraction** with sparse replication and Hyperswarm for peer discovery. Where they overlap:

| Axis | Hypercore / Pears | Iroh |
|---|---|---|
| Primary primitive | Signed append-only log | Direct connection between NodeIDs |
| Discovery | Hyperswarm (DHT over Mainline, BEP-44) | DNS (`pkarr` / Mainline-DHT integration) + relay |
| Hole punching | UTP + Hyperswarm DHT | QUIC + DERP-style relay fallback (Tailscale-derived) |
| Content addressing | Per-block (Merkle tree per log) | Per-blob (BLAKE3 hash, via `iroh-blobs`) |
| Mutability | Native (the log itself) | Via `iroh-docs` (eventually-consistent k/v) |
| Identity | Log keypair (one per log) | NodeID (one per node) — ed25519 public key |
| Language | Node.js / TypeScript | Rust (with FFI bindings) |
| Production | Beaker → Pears desktop app, Keet messenger | Delta Chat, Spacedrive, multiple shipping apps |

**Where they overlap:** both are NAT-punching P2P stacks with cryptographic identifiers. Both have a story for content-addressed data (Hypercore at the block level via Merkle trees; iroh-blobs at the object level via BLAKE3). Both ship in production apps.

**Where they don't:** Hypercore *is* an append-log first; iroh is a *transport* first with content-addressing as a separate crate. Hypercore's discovery is DHT-only; iroh has a DNS-based path that bypasses DHT entirely. Hypercore is a single Node.js / JS ecosystem; iroh is Rust with FFI bindings (Node.js, Python in progress, Swift/Kotlin via UniFFI).

**Lesson for Myrhiza:** if Myrhiza wants append-log replication as a primitive (Holochain-source-chain shape), Hypercore is closer to a drop-in. If Myrhiza wants the transport-substrate-plus-pluggable-protocols shape, iroh is the cleaner separation. They are not direct competitors; iroh-docs + iroh-blobs is roughly *the protocol layer above iroh that targets Hypercore's shape*.

## vs Tailscale

Superficial similarity (both NAT-punch peers behind firewalls, both operate relay servers as a fallback) but very different goals.

| Axis | Tailscale | Iroh |
|---|---|---|
| Goal | Corporate VPN — give a user's many devices private IPv4/IPv6 addresses on a shared overlay | P2P app substrate — give an app a way to dial a peer by NodeID |
| Identity | Tied to user account at tailscale.com / coordination server | Cryptographic NodeID (ed25519 public key) |
| Coordination plane | Centralized (`tailscale.com`) — runs auth, key exchange, ACLs | None mandatory — DNS / pkarr is optional, relays are interoperable |
| Fallback relay | DERP (their own protocol, run by Tailscale) | DERP-derived (their own protocol, run by n0 by default) |
| Transport | WireGuard | QUIC (since 0.97, `noq`) |
| Permission model | Tailnet ACLs, MagicDNS | Per-app — apps build their own; iroh is below this layer |
| License | Mostly BSD-3 (some closed coordination server) | Apache-2.0 / MIT |
| Operational cost-bearer | Tailscale Inc. (paid plans for users) | n0 (free public relays for dev, paid for production) |

**Heritage overlap.** Iroh's NAT-traversal design is explicitly inspired by Tailscale's — the [DERP-style relay](https://tailscale.com/blog/how-tailscale-works) was the model. The b5 framing ([Comparing iroh & libp2p](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)): *"Iroh leverages concepts pioneered by Tailscale, leading to a higher success rate in NAT traversal and offering a clear relay fallback mechanism."*

**Where they fundamentally diverge.** Tailscale is a VPN for users; iroh is a transport for apps. Tailscale assumes a coordination server is fine because the user has a Tailscale account; iroh assumes coordination should be optional and pluggable. A Myrhiza spec author looking at Tailscale would conclude: the right primitives, wrong abstraction layer.

## vs Magic Wormhole / Croc

Both are one-shot file-transfer tools. They share NAT-punch heritage (relay-as-fallback). Iroh has a sibling project, [`sendme`](https://github.com/n0-computer/sendme), which fills the same niche on top of iroh.

| Axis | Magic Wormhole | Croc | Iroh `sendme` |
|---|---|---|---|
| Use case | "Type this code, get the file" | Same | Same |
| Identity / pairing | Short human-readable code (PAKE) | Short human-readable code (PAKE) | Long ticket (NodeID + auth token) |
| Transport | TCP + relay (Mailbox server) | TCP + relay (`croc.schollz.com`) | QUIC + DERP relay |
| Codebase | Python (reference), several ports | Go | Rust |
| Production scale | Decade-old, niche | Active, niche | Active, growing |

**Difference of kind, not degree.** Magic Wormhole and Croc are *applications* with a UX optimized for human-to-human one-shot file transfer (PAKE-derived short codes are the killer feature). Iroh's `sendme` borrows the UX shape but uses long machine-readable tickets, not short human-spoken codes. **PAKE-style human-readable code derivation is something iroh does not natively offer** as of 2026 — it would have to be built on top.

**Lesson:** if Myrhiza wants the *capability-handoff* UX of Magic Wormhole (short typeable code → secure introduction), iroh is the right transport but not the right paired API. Building a PAKE-on-iroh primitive is a real Myrhiza-layer todo.

## vs Veilid

A more recent, similar-vintage P2P substrate from the Cult of the Dead Cow alumni. Both target "general P2P transport" rather than a specific application.

| Axis | Veilid | Iroh |
|---|---|---|
| Transport | Custom (UDP, TCP, WebSocket, WebRTC) | QUIC (over UDP) + relay-over-HTTPS |
| Routing | Onion-style multi-hop privacy routing | Direct + relay (single hop) |
| Anonymity | Strong by design | Not by default (relay can see NodeID-to-NodeID metadata) |
| Default relay | Decentralized by design | n0-operated public relays |
| Production | Less visible | Multiple shipping apps |
| Funding | Cult of the Dead Cow / nonprofit | Number 0 (VC + founder) |
| License | MPL-2.0 | Apache-2.0 / MIT |
| Maturity | 0.x | 1.0-rc.0 (May 7, 2026) |

**Where Veilid wins:** anonymity-by-default. Iroh's e2ee relay protects payload but the relay sees NodeID-pair metadata; Veilid hides this with onion routing. For privacy-first apps Veilid is the better starting point.

**Where Iroh wins:** maturity, ecosystem, throughput. Veilid is closer to a Tor-shaped substrate; iroh is closer to a fast-data substrate. They could compose: Veilid as a custom transport plugged into iroh's `Endpoint` ([the iroh-tor blog](https://www.iroh.computer/blog/tor-custom-transport) shows this pattern for Tor).

## Summary table

| System | Primitive | Identity | Default relay | Production | Best fit |
|---|---|---|---|---|---|
| **Iroh** | QUIC connection by NodeID | ed25519 pubkey | n0-operated (4 servers) | 1.0-rc.0; multiple apps | App substrate; load-bearing for Myrhiza |
| libp2p | Multi-transport peer connection | PeerID (multi-curve) | Circuit-relay-v2 (peer-run) | IPFS, Filecoin, Polkadot | When you need DHT/pubsub on-the-shelf |
| Hypercore / Pears | Append-only log | Log keypair | Hyperswarm DHT | Keet, Pears desktop | When the log IS the abstraction |
| Tailscale | WireGuard mesh by user | Tailnet identity | DERP (Tailscale-run) | Production VPN | Corporate VPN, not app substrate |
| Magic Wormhole / Croc | One-shot file transfer | PAKE short code | Mailbox / `croc.schollz` | Niche utility | Human-to-human file send |
| Veilid | Onion-routed transport | Veilid pubkey | Distributed | Less visible | Privacy-first apps |

## Sources

- [Comparing iroh & libp2p (Jan 5, 2024)](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
- [iroh discussion #1277 — relation to rust-libp2p](https://github.com/n0-computer/iroh/discussions/1277)
- [libp2p / rust-libp2p discussion #5247 — comparison](https://github.com/libp2p/rust-libp2p/discussions/5247)
- [Iroh 0.97.0 — Custom Transports & noq](https://www.iroh.computer/blog/iroh-0-97-0-custom-transports-and-noq)
- [Iroh 0.96.0 — The QUIC Multipaths to 1.0](https://www.iroh.computer/blog/iroh-0-96-0-the-quic-multipaths-to-1-0)
- [Iroh — Tor custom transport](https://www.iroh.computer/blog/tor-custom-transport)
- [Iroh — Dial by NodeID, no address required](https://www.iroh.computer/blog/iroh-dns)
- [Tailscale — How Tailscale Works](https://tailscale.com/blog/how-tailscale-works)
- [GitHub — n0-computer/sendme](https://github.com/n0-computer/sendme)
- [Hypercore protocol](https://hypercore-protocol.org/)
- [Pears (Holepunch)](https://docs.pears.com/)
- [Magic Wormhole](https://magic-wormhole.readthedocs.io/)
- [Croc (schollz)](https://github.com/schollz/croc)
- [Veilid](https://veilid.com/)
- [HN 44383072 — b_fiive on iroh vs libp2p](https://news.ycombinator.com/item?id=44383072)
- [Medium — P2P Networking: WebRTC vs libp2p vs Iroh (Ark Builders)](https://medium.com/@ark-builders/the-deceptive-complexity-of-p2p-connections-and-the-solution-we-found-d2b5cbeddbaf)
