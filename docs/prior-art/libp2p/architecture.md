**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — architecture (Host / Swarm / NetworkBehaviour / multistream-select / protocol upgrade)

# Architecture

libp2p is a modular composition of orthogonal concerns: **transports** (carry bytes), **security** (encrypt + authenticate), **muxing** (one connection → N streams), **protocols** (define interactions over a stream), and **discovery** (find peers). The composition is glued by the *protocol upgrade* pattern and *multistream-select* negotiation. This file documents that composition; algorithm-specific sub-protocols (gossipsub, Kademlia) live in their own files.

The most important conceptual fact: **libp2p is not a runtime; it is a stack of negotiable layers**. Each implementation (rust / go / js / nim) exposes the layers slightly differently (Swarm in Rust, Host in Go, Libp2pNode in JS), but the wire-level shape is shared.

## Layered composition

```
┌────────────────────────────────────────────────────────────────┐
│  Application protocols                                         │
│  (gossipsub, kad-dht, identify, ping, autonat, dcutr, ...)    │
├────────────────────────────────────────────────────────────────┤
│  Stream muxer                                                  │
│  (yamux, mplex (deprecated), QUIC-native streams)             │
├────────────────────────────────────────────────────────────────┤
│  Security upgrade                                              │
│  (noise / tls / plaintext; QUIC has TLS 1.3 built in)         │
├────────────────────────────────────────────────────────────────┤
│  multistream-select (negotiates which protocol at each layer)  │
├────────────────────────────────────────────────────────────────┤
│  Base transport                                                │
│  (tcp / quic-v1 / websocket / webrtc / webtransport / uds)    │
└────────────────────────────────────────────────────────────────┘
```

When a peer dials `/ip4/1.2.3.4/udp/4001/quic-v1/p2p/12D3KooW...`:

1. **Transport** layer opens a QUIC connection (which subsumes security + muxing — QUIC has TLS 1.3 and multiple streams natively).
2. **Identity check** during TLS: the cert is self-signed by the peer's ed25519 / RSA / Secp256k1 / ECDSA key. The dialer verifies the cert's signing key matches the PeerId in the multiaddr.
3. **multistream-select** is *not* used for security/muxing here (QUIC handles both), but *is* used for per-stream protocol selection — every new stream begins with multistream-select.
4. **Per-stream protocol negotiation:** the dialer sends `/multistream/1.0.0\n/ipfs/ping/1.0.0\n`, the listener replies with the protocol name it supports (or `na`), then the actual protocol runs.

For TCP (`/ip4/1.2.3.4/tcp/4001`) the layering is more explicit: TCP carries multistream-select, which negotiates Noise XX (or TLS 1.3), which then carries multistream-select again to negotiate yamux, which gives N streams, each starting with multistream-select to negotiate the application protocol. **Three nested multistream-select handshakes** to open the first application stream over TCP.

## Multistream-select

multistream-select is libp2p's "what protocol shall we speak?" negotiation. It's a simple line-based protocol on a stream:

```
< /multistream/1.0.0
> /multistream/1.0.0
< /noise
> /noise          # accept, switch to noise
... noise handshake bytes ...
```

If the listener doesn't support the proposed protocol, it replies `na` (not available) and the dialer retries with the next candidate. Latency cost: 1 RTT per layer per stream (the v2 spec attempted to optimise this but didn't ship). This is one of the major footguns: opening a fresh stream on a TCP-Noise-yamux stack costs roughly **3 sequential RTTs** for protocol negotiation alone, before any application traffic. QUIC reduces this dramatically by handling security + muxing in the QUIC handshake itself; on QUIC, a new stream costs ~0 RTT for negotiation (multistream-select still runs, but in-stream, not blocking connection setup).

**multistream-select v2** has been proposed for years; it would optimistically send the first protocol's data alongside the negotiation request, eliminating the 1-RTT cost. The spec has stayed at [draft](https://github.com/libp2p/specs/blob/master/connections/inlined-multistream-select.md)-status without shipping; the production answer is "use QUIC."

## Host / Swarm / NetworkBehaviour (rust-libp2p)

rust-libp2p's central abstraction is **`Swarm<NetworkBehaviour>`**. A `NetworkBehaviour` defines the protocol(s) the swarm runs (e.g. gossipsub, kad-dht, identify, ping). The Swarm:

- Owns the **transport stack** (e.g. `TcpTransport.upgrade(Noise).upgrade(Yamux)` composed with `QuicTransport`).
- Owns the **peer keypair** (Ed25519 default).
- Maintains the **connection set** (incoming + outgoing).
- Drives the protocols by polling the `NetworkBehaviour`'s `poll()` method, which returns `SwarmEvent`s and dial/listen/notify instructions.

Custom protocols implement `NetworkBehaviour` (in practice via the `derive(NetworkBehaviour)` macro, which composes multiple sub-behaviours into a unified one). The standard idiom is:

```rust
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    kad: kad::Behaviour<MemoryStore>,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
}
```

The macro generates an enum `MyBehaviourEvent` that fans out each sub-behaviour's events. Application code matches on the event variant and dispatches.

## Host (go-libp2p)

go-libp2p's central abstraction is **`Host`**, which exposes:

- `Network()` — the swarm-level connection manager.
- `Mux()` — protocol multiplexer (register handlers per protocol id).
- `Peerstore()` — known peer addresses + metadata.
- `ID()` / `Peerstore().PubKey(id)` — local identity.
- `EventBus()` — pub/sub for local lifecycle events.
- `ConnManager()` — connection pruning policy.

A Go protocol implementation registers via `host.SetStreamHandler(protocolID, handler)`. Compared to rust-libp2p's typed `NetworkBehaviour` composition, go-libp2p is more imperative — protocols are registered by string id and routed by the multiplexer.

## Libp2pNode (js-libp2p)

js-libp2p exposes a `createLibp2p({...config})` factory returning a `Libp2pNode`. Configuration is by composition:

```typescript
const node = await createLibp2p({
  addresses: { listen: ['/ip4/0.0.0.0/tcp/0', '/ip4/0.0.0.0/udp/0/quic-v1'] },
  transports: [tcp(), quic(), webSockets()],
  streamMuxers: [yamux()],
  connectionEncrypters: [noise()],
  services: {
    pubsub: gossipsub(),
    dht: kadDHT(),
    identify: identify(),
  },
})
```

Service objects are dependency-injected at construction time. js-libp2p 3.x (current `latest` 3.3.1) uses TypeScript with `interface-*` patterns — each transport / muxer / encrypter implements a published interface, and the node composes them.

## Connection lifecycle

The connection lifecycle is broadly the same across implementations:

1. **Dial** — the application calls `swarm.dial(addr_or_peer_id)`. If `peer_id`, the peerstore is queried for known addresses; otherwise the multiaddr is used directly.
2. **Transport pick** — the swarm filters available transports by which can speak the multiaddr (TCP listens for `/ip4.../tcp/...`, QUIC for `/ip4.../udp/.../quic-v1`, etc.).
3. **Connection establishment** — TCP three-way handshake or QUIC handshake. If TCP, multistream-select picks the security protocol next.
4. **Security handshake** — Noise XX (default; mutual auth) or TLS 1.3. Both establish a session key + verify the remote PeerId.
5. **Muxer negotiation** — multistream-select picks yamux. (QUIC's native streams replace this layer entirely.)
6. **Identify** — once the connection is up, both sides spontaneously open an identify stream (`/ipfs/id/1.0.0`) and exchange peer info: protocol versions, supported protocols, listen addresses, agent string ("rust-libp2p/0.56.0"). This is *not* part of the handshake — connections are usable before identify completes.
7. **Application streams** — open a stream, run multistream-select to pick the protocol id, then carry application traffic.

## Identify

The identify protocol (`/ipfs/id/1.0.0`) is foundational. Every connection produces an identify exchange in both directions. Payload:

- `agent_version` — string, e.g. `"go-libp2p/0.48.0"`.
- `protocol_version` — usually `"ipfs/0.1.0"` (historical name from the IPFS-origin era).
- `public_key` — the peer's public key (verifies against PeerId).
- `listen_addrs` — the peer's listen multiaddrs.
- `observed_addr` — the multiaddr at which we observed the remote (used by **AutoNAT** to infer NAT status).
- `protocols` — list of supported protocol ids ("/ipfs/ping/1.0.0", "/meshsub/1.1.0", ...).
- `signed_peer_record` — signed bundle of identity + addrs (added later; enables relay/PX use cases).

Identify is the load-bearing primitive for ambient peer-state — it's how you discover that the peer you just connected to also speaks gossipsub, or that its listen address differs from the one you dialed.

## Protocol upgrade pattern

The *protocol upgrade* pattern is libp2p's name for the layered negotiation. Each layer can be upgraded independently:

- TCP base → Noise upgrade → Yamux upgrade → gossipsub stream.
- TCP base → TLS upgrade → Yamux upgrade → kad-dht stream.
- WebSocket base → Noise upgrade → Yamux upgrade → identify stream.
- QUIC base (which is already TLS+muxed) → gossipsub stream directly.

The upgrade chain is configured per-transport at swarm-build time. Different transports can have different upgrade chains; the swarm picks the right chain based on the multiaddr being dialed.

## Multiaddr

The multiaddr is libp2p's self-describing address format, specified at [multiformats/multiaddr](https://github.com/multiformats/multiaddr). Examples:

- `/ip4/1.2.3.4/tcp/4001` — bare TCP, no peer identity.
- `/ip4/1.2.3.4/tcp/4001/p2p/12D3KooWA...` — TCP with required PeerId.
- `/ip4/1.2.3.4/udp/4001/quic-v1/p2p/12D3KooWA...` — QUIC v1 with PeerId.
- `/dnsaddr/bootstrap.libp2p.io/p2p/QmNn...` — DNS-resolved bootstrap.
- `/p2p/12D3KooWB.../p2p-circuit/p2p/12D3KooWA...` — A is reachable via B as a circuit relay.
- `/dns4/server.example.com/tcp/443/wss/p2p/12D3KooW...` — WebSocket-Secure.
- `/ip4/1.2.3.4/udp/4001/webrtc-direct/certhash/uEi.../p2p/12D3KooW...` — WebRTC-Direct with cert pinning.

The multiaddr format is **standardised separately from libp2p** (multiformats project) and used by iroh, Filecoin, and various non-libp2p tools. The Myrhiza-relevant lesson: multiaddrs are a perfectly good external-share format; iroh's `EndpointTicket` (base32-encoded `EndpointId + Vec<TransportAddr>`) is a libp2p-style multiaddr bundle in different framing.

## Transports

| Transport | Spec status | Where supported |
|---|---|---|
| **TCP** | 3A Recommendation | Every impl (core) |
| **QUIC (RFC 9000)** | 3A Recommendation, r1 2022-12-30 | go, rust, js, nim, jvm. **Recommended default** per spec. |
| **WebSocket / WSS** | Active | go, rust, js, nim |
| **WebTransport** | Implementation status varies; spec at 2A | go, js (server-side); rust via `webtransport-websys` (client-side, WASM); not in nim |
| **WebRTC** (browser-to-browser) | 2A Candidate Recommendation, r1 2023-04-12 | js (browser), rust (`webrtc-websys`) |
| **WebRTC-Direct** (browser-to-server, no trusted CA) | Active draft | go, rust, js. Standalone certhash via multiaddr. |
| **WebRTC-Star** (STUN+TURN-mediated) | Deprecated 2023; archived 2024 | js (legacy) |
| **mplex** | Deprecated 2024 | Removed from defaults in all impls |
| **yamux** | Active | Universal default |

The **QUIC-vs-TCP** stance is "use QUIC unless UDP is blocked." Per the spec: *"Due to its inherently faster handshake latency (a single network-roundtrip), and generally better performance characteristics, it is RECOMMENDED that libp2p implementations offer QUIC as one of their transports. However, UDP is blocked in a small fraction of networks, therefore it is RECOMMENDED that libp2p nodes offer a TCP-based connection option as a fallback."*

## NAT traversal pipeline

NAT traversal is a layered pipeline of progressively-fancier techniques. The libp2p stack composes them:

1. **AutoNAT** — peer asks others to dial it back to determine if it's reachable. Discovered NAT status (`Public`, `Private`, `Unknown`) drives subsequent choices.
2. **AutoRelay** — if private, the peer registers with relay nodes (Circuit Relay v2) so others can reach it via a reserved circuit address (`/p2p/<RELAY>/p2p-circuit/p2p/<SELF>`).
3. **DCUtR (Direct Connection Upgrade through Relay)** — once two peers are connected via relay, they coordinate simultaneous hole punching to upgrade to a direct QUIC or TCP connection. Success rate, per libp2p's own published [hole-punching test data](https://blog.libp2p.io/2022-01-20-libp2p-hole-punching/), caps at ~70% in field conditions. Iroh's January 2024 comparison blog cited this number as the reason iroh switched to a relay-with-direct-upgrade approach over QAD instead of DCUtR-style coordination.
4. **Circuit Relay v2** — TURN-shaped fallback. The relay forwards encrypted bytes between peers. The relay sees connection metadata (which PeerId is talking to which, when, how much) but not content. The protocol is on libp2p QUIC streams.
5. **UPnP / NAT-PMP / PCP** — try to open the firewall directly via router protocols. Modest success rate on consumer routers; usually a complement, not a replacement.

Compare iroh's pipeline (`../iroh/nat-traversal.md`): relay-first by default; QAD (QUIC Address Discovery, replaces STUN) to learn observed address; race-to-direct as connections upgrade. The libp2p pipeline is older, more modular, and uses more named sub-protocols; iroh's is younger, less composed, and faster on the happy path.

## Implications for Myrhiza

- **The protocol-upgrade composition is over-engineered for Myrhiza's needs.** We pick one transport (iroh's QUIC) and one security model (Noise via QUIC's TLS 1.3). Negotiating between TCP-Noise-Yamux vs TCP-TLS-Mplex vs QUIC is unnecessary complexity for a closed-stack runtime that ships its own transport.
- **multistream-select is a real latency cost.** Three nested negotiations on TCP-Noise-yamux is a recurrent libp2p complaint. Myrhiza inherits iroh's ALPN-based protocol-multiplexing on QUIC's single handshake — see [`../iroh/architecture.md`](../iroh/architecture.md). That is the correct shape.
- **Multiaddr is worth borrowing as a peer-share format.** Self-describing, extensible, human-readable. iroh's EndpointTicket is a constrained subset; if Myrhiza ever wants to express "this peer at this address via this transport," multiaddr is the right pattern.
- **identify is a useful primitive even outside libp2p.** A bidirectional "what protocols do you support, what addresses can I dial you on, what agent are you?" exchange on every new connection is good hygiene. Myrhiza's app-level capability handshake could mirror identify's shape: cheap, async, post-connection.
- **NAT traversal as a pipeline of named sub-protocols (AutoNAT + AutoRelay + DCUtR) is more legible than iroh's tighter integration**, but at the cost of more moving parts and lower success rates in practice. Myrhiza picks iroh's choice but should know how libp2p decomposes the problem; we may steal AutoNAT-style observed-address reporting if our discovery layer needs it.

## Sources

- [libp2p specs README](https://github.com/libp2p/specs)
- [libp2p connections specs](https://github.com/libp2p/specs/tree/master/connections)
- [multistream-select spec](https://github.com/libp2p/specs/tree/master/connections)
- [multiformats/multiaddr](https://github.com/multiformats/multiaddr)
- [identify spec](https://github.com/libp2p/specs/blob/master/identify/README.md)
- [rust-libp2p workspace Cargo.toml](https://github.com/libp2p/rust-libp2p/blob/master/Cargo.toml)
- [go-libp2p Host doc](https://pkg.go.dev/github.com/libp2p/go-libp2p/core/host)
- [js-libp2p createLibp2p docs](https://github.com/libp2p/js-libp2p/blob/main/packages/libp2p/README.md)
- [libp2p hole-punching blog (Jan 2022)](https://blog.libp2p.io/2022-01-20-libp2p-hole-punching/)
- [iroh — architecture (sibling doc)](../iroh/architecture.md)
- [iroh — NAT traversal (sibling doc)](../iroh/nat-traversal.md)
