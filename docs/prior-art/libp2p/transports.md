**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — transports (TCP / QUIC / WebSocket / WebRTC / WebTransport)

# Transports

libp2p separates *base transport* (carries bytes) from *security upgrade* (encrypts + authenticates) and *muxer* (one connection → N streams). Some transports (QUIC, WebRTC) bundle all three; others (TCP, WebSocket) are bare and require explicit security + muxer upgrades on top.

The transport surface is one of libp2p's most visible differentiators against iroh: where iroh ships **one transport (QUIC) + relay** as the production answer and only opened a pluggable `Transport` trait in 0.97, libp2p has shipped **six distinct production transports** for years. The breadth is a strength (browser viability is built in) and a cost (interop is harder, configuration is harder, attack surface is larger).

## Transport matrix

| Transport | Multiaddr | Security | Muxer | Production status | Where supported (impl) |
|---|---|---|---|---|---|
| **TCP** | `/ip4|6/.../tcp/<port>` | Noise XX / TLS 1.3 (upgrade) | yamux (upgrade) | Active, universal | go, rust, js, nim, jvm, cpp |
| **QUIC v1 (RFC 9000)** | `/ip4|6/.../udp/<port>/quic-v1` | TLS 1.3 (built-in) | QUIC streams (built-in) | Active, **recommended default** | go, rust, js, nim, jvm |
| **QUIC draft-29** | `/ip4|6/.../udp/<port>/quic` | TLS 1.3 | QUIC streams | **Being phased out** | go (legacy), rust (legacy) |
| **WebSocket** | `/.../tcp/<port>/ws` or `/wss` | Noise / TLS (upgrade) | yamux (upgrade) | Active | go, rust, js, nim |
| **WebTransport** | `/.../udp/<port>/quic-v1/webtransport/certhash/...` | TLS 1.3 (HTTP/3-derived) | WebTransport streams | Active, browser-focused | go (server), rust (client `webtransport-websys`), js |
| **WebRTC (browser↔browser)** | `/webrtc/p2p/<id>` (signaled) | DTLS 1.2 | WebRTC data channels | Active, 2A spec | js, rust (`webrtc-websys`) |
| **WebRTC-Direct (browser↔server)** | `/.../udp/<port>/webrtc-direct/certhash/<hash>` | DTLS 1.2, hash-pinned | WebRTC data channels | Active draft | go, rust (`webrtc`), js |
| **WebRTC-Star** (STUN/TURN signaling) | `/dns4/.../wss/p2p-webrtc-star/...` | DTLS 1.2 | WebRTC data channels | **Deprecated 2023, archived 2024** | js (legacy only) |
| **TLS 1.3** (as security layer) | n/a — used inside TCP/WebSocket | — | — | Active | go, rust, js, nim |
| **Noise** (as security layer) | n/a — used inside TCP/WebSocket | — | — | **Recommended default** | go, rust, js, nim, jvm |
| **mplex** (muxer) | n/a | — | — | **Deprecated 2024**, removed from defaults | (legacy only) |
| **yamux** (muxer) | n/a | — | — | Active default | go, rust, js, nim, jvm |

## TCP

TCP is the universal floor. Every libp2p implementation supports it. Listen on a TCP port, dial via `/ip4/X/tcp/Y`, run Noise/TLS handshake over the socket, then run yamux for streaming.

The cost: **TCP head-of-line blocking**. A single dropped packet stalls every stream on the connection. For pub/sub workloads (gossipsub) this is bad — one slow IHAVE blocks all subsequent payload deliveries. QUIC's per-stream loss handling avoids this entirely. The libp2p docs explicitly recommend QUIC over TCP whenever UDP is available.

## QUIC (RFC 9000)

QUIC is the recommended transport for non-browser libp2p nodes. Per the [QUIC spec](https://github.com/libp2p/specs/blob/master/quic/README.md): *"It is RECOMMENDED that libp2p implementations offer QUIC as one of their transports."*

Key facts:

- **RFC 9000 only** (`/quic-v1` multiaddr component). The legacy draft-29 (`/quic` component) is being phased out.
- **TLS 1.3 baked in.** The PeerId is verified during the TLS handshake using libp2p's TLS spec (self-signed cert with the peer's ed25519 key signing the cert's signing key).
- **ALPN: `libp2p`.** Every libp2p QUIC connection negotiates the `libp2p` ALPN id. This is how a libp2p QUIC peer distinguishes itself from a generic QUIC service on the same UDP port.
- **Per-stream loss handling.** A stream's packet loss doesn't stall other streams.
- **0-RTT and 1-RTT handshakes.** New connection = 1 RTT (server cert verified). Resumed connection = 0 RTT. Versus TCP-Noise-yamux which is 3+ RTTs.
- **UDP socket sharing.** A single UDP port can host both raw QUIC and WebTransport (which is HTTP/3-over-QUIC), saving NAT mappings.

The Rust implementation is [`libp2p-quic 0.13.0`](https://crates.io/crates/libp2p-quic) (master at 0.14.0), built on [`quinn`](https://github.com/quinn-rs/quinn) — the same crate iroh originally used before forking to `noq`. Go's implementation is [`quic-go`](https://github.com/quic-go/quic-go), maintained by Marten Seemann (also the QUIC spec author for libp2p — `@marten-seemann`). The cross-implementation interop is excellent precisely because both crates are written by people who write QUIC specs.

## WebSocket / WebSocket-Secure

`/ws` or `/wss`. Useful for browsers (real WebSocket support in every modern browser) and for traversing port-restrictive firewalls (WebSocket on 443 looks like HTTPS to corporate filters).

Cost: still TCP underneath (with HOL blocking), plus WebSocket framing overhead, plus TLS handshake (`wss`) on top of TCP. For browser-to-server use cases this is fine; for server-to-server, prefer QUIC.

**WebSocket-Secure (`wss`)** requires a real TLS certificate signed by a CA the browser trusts. This is the **browser-to-server connectivity bottleneck**: you cannot dial an arbitrary peer's WSS endpoint from a browser unless that peer has a public-CA-signed cert. This is why WebRTC-Direct exists.

## WebTransport

WebTransport is HTTP/3-over-QUIC plus a JS API. The browser side speaks WebTransport; the server side speaks QUIC with a WebTransport upgrade.

Key feature for libp2p: **`certhash` multiaddr pinning** lets a browser dial a server with a self-signed TLS cert by pinning the cert's hash in the multiaddr (`/certhash/uEi...`). This dodges the public-CA requirement that blocks `wss` for arbitrary peers.

Trade-offs:

- **Browser support is recent and uneven.** Chrome shipped WebTransport in mid-2022; Firefox in 2024; Safari is still partial as of 2026-05. js-libp2p degrades to WebRTC for Safari users.
- **Server stack is non-trivial.** WebTransport-over-QUIC requires a custom HTTP/3 stack that supports WebTransport extensions. go-libp2p and js-libp2p have it; rust-libp2p has only `webtransport-websys` (client-side WASM, no native server).
- **WebTransport-websys (rust)** at version `0.6.0` (master) — authored by Yiannis Marangos (Eiger) and oblique. Browser-only target.

The Myrhiza-relevant fact: **WebTransport is the cleanest path from browser to a non-CA-trusted server**, and `certhash` is the right pattern for cert-pinned P2P dialing in a browser. If Myrhiza ever needs a true in-browser peer (jco-hosted app component dialing a Myrhiza node), WebTransport with `certhash` is the spec to study.

## WebRTC (three flavors)

WebRTC is the only libp2p transport that supports **true browser-to-browser** connectivity. The spec ([webrtc r1, 2023-04-12](https://github.com/libp2p/specs/blob/master/webrtc/README.md), Candidate Recommendation) defines three distinct profiles:

### 1. WebRTC (browser ↔ browser)

The standard browser WebRTC stack. Two browsers exchange SDP offer/answer via a signaling channel (any libp2p stream — typically over a relay), perform ICE candidate gathering with STUN, and establish a DTLS-encrypted SCTP data channel.

- **Signaling:** libp2p streams. The signaling channel runs over an existing libp2p connection (relay-mediated). This avoids the libp2p-webrtc-star pattern of dedicated signaling servers.
- **NAT traversal:** ICE + STUN (Google's `stun.l.google.com:19302` by default) + optional TURN.
- **Latency cost:** signaling is multi-RTT; once established, data channels are low-latency UDP.

### 2. WebRTC-Direct (browser ↔ server)

The breakthrough: a browser can dial a public server without that server providing a CA-trusted TLS cert. The multiaddr includes a `certhash` (multihash of the server's self-signed cert), the browser pins to it during the DTLS handshake.

- **No signaling server.** The browser dials a known multiaddr directly.
- **No STUN/TURN.** The server is publicly reachable; no NAT-traversal coordination needed.
- **Cert rotation:** the server can rotate its cert; clients must learn the new certhash out-of-band (e.g. via identify on an existing connection).
- **Why this matters:** the spec is explicit — *"libp2p transport protocol without the need for trusted TLS certificates. Enable browsers to connect to public server nodes without those server nodes providing a TLS certificate within the browser's trustchain. This specification notes that this capability cannot be achieved with WebSocket transport, since browsers require remote endpoints to have trusted certificates."*

### 3. WebRTC-Star (deprecated, archived 2024)

The legacy approach: dedicated signaling servers brokered browser-to-browser connections. The server saw connection metadata (which peer is dialing which), which was a centralization concern. js-libp2p deprecated WebRTC-Star in 2023 and archived the repo (`libp2p/js-libp2p-webrtc-star`) in 2024.

**Status note:** `libp2p/js-libp2p-webrtc-star` was officially archived 2024-09 (per the GitHub `archived` flag). New deployments use plain WebRTC (libp2p-stream signaling) or WebRTC-Direct.

### Rust implementations

| Crate | Version | Target | License |
|---|---|---|---|
| [`libp2p-webrtc`](https://crates.io/crates/libp2p-webrtc) | `0.9.0-alpha.1` on crates.io; master at `0.10.0-alpha` | Server (Tokio + str0m + webrtc-rs) | MIT |
| `libp2p-webrtc-websys` (in workspace) | `0.5.0` master | Browser (WASM, native browser WebRTC API) | MIT |

`libp2p-webrtc` has been **stuck in alpha for years** (the crates.io version was last published 2025-06-27 still labeled `0.9.0-alpha.1`). The native Rust WebRTC stack is genuinely hard to ship — str0m + webrtc-rs are heavy dependencies — and the rust-libp2p team has openly flagged this as a maintenance burden. Production use in Rust is rare; production use in JS (browser) is the path most teams take.

## What iroh has that libp2p doesn't (and vice versa)

| Feature | iroh | libp2p |
|---|---|---|
| QUIC native | Yes (originally Quinn, forked to `noq` in 0.97) | Yes (Quinn / quic-go) |
| TCP fallback | No | Yes |
| WebSocket | No | Yes |
| WebTransport | No (planned but not shipped per `../iroh/transports.md`) | Yes |
| WebRTC | No | Yes (3 flavors) |
| WebRTC browser-to-browser direct | No | Yes |
| Browser native peer | Relay-only (WebSocket to relay) | Full WebRTC, WebTransport with certhash |
| Pluggable transport API | Yes (0.97+, used by Tor / Veilid) | Yes (always — `Transport` trait predates iroh's existence) |
| Multipath QUIC | Yes (0.96) | No |
| QAD (QUIC Address Discovery, replaces STUN) | Yes (0.32+) | No (uses STUN or AutoNAT) |

The structural difference: **iroh has one transport done well + a recent extension point**; **libp2p has six transports done seperately + a long-established extension point**. iroh's approach optimises for the happy path (NAT-traversed QUIC); libp2p's approach optimises for breadth (every conceivable network condition).

## Yamux + (deprecated) mplex

For TCP and WebSocket, libp2p needs an external muxer to get multiple streams on one connection. Two options:

- **yamux** — HashiCorp's stream multiplexer ([rust-yamux](https://github.com/libp2p/rust-yamux)). Window-based flow control, sliding-window backpressure. Production default. Maintained by libp2p.
- **mplex** — libp2p's own minimal muxer. Per the [mplex spec](https://github.com/libp2p/specs/blob/master/mplex/README.md), no flow control, no backpressure — which makes it vulnerable to memory exhaustion attacks. **Deprecated 2024 and removed from defaults in all implementations.** Listed for historical reference; new code should not enable mplex.

QUIC and WebRTC have native multiplexing (QUIC streams, WebRTC data channels) and don't use an external muxer.

## Security upgrades: Noise vs TLS 1.3

For TCP + WebSocket transports, libp2p offers two security upgrade paths:

- **Noise** (XX pattern, see [`identity.md`](identity.md)) — Noise Protocol Framework with 25519 + ChaChaPoly + SHA256. Spec: `/noise` r5, 2022-12-07. **The libp2p-default**. Smaller code, well-understood, no certificate machinery.
- **TLS 1.3** — Standard TLS with self-signed certs containing the peer's libp2p key as an extension. Spec: [`/tls`](https://github.com/libp2p/specs/tree/master/tls). Used by QUIC (which is TLS-1.3-based by definition) and as an optional upgrade for TCP/WebSocket.

Choice criteria: Noise is leaner and the default; TLS is the QUIC requirement and is sometimes preferred when interop with non-libp2p TLS systems matters. Both are considered Recommendation-grade.

## Implications for Myrhiza

- **Myrhiza inherits iroh's QUIC-first stance.** That is the right call — QUIC's per-stream loss handling, low-latency handshake, and UDP-port-sharing match Myrhiza's "kernel owns one transport per host" architecture.
- **Browser-native peer support is libp2p's most distinctive capability.** WebTransport + WebRTC are the two paths from a browser to a peer. Myrhiza's browser-kernel (jco) inherits iroh's relay-only browser story, which is genuinely worse than libp2p's WebRTC story for true browser-to-browser apps. If Myrhiza ever needs in-browser-peer use cases (a Myrhiza app component running in a browser as a full peer, not just as a UI client of a native kernel), WebRTC-via-libp2p is the comparison study.
- **multistream-select on TCP-Noise-yamux is a latency tax** Myrhiza doesn't pay because we picked QUIC. Good. Don't let backwards-compat pressure ever introduce a multi-layer-negotiation stack.
- **The deprecated WebRTC-Star pattern is a useful negative example.** Centralized signaling servers see who-talks-to-whom metadata even when traffic is encrypted — same critique iroh faces with relays. Myrhiza's app-layer pub/sub on top of iroh-gossip should not introduce a new "rendezvous server" pattern that recreates this metadata leak.
- **The yamux design (window-based backpressure) is the right shape for any future muxer Myrhiza might author** if it ever moves below QUIC. Mplex's no-backpressure design is the cautionary tale.

## Sources

- [libp2p QUIC spec (r1, 2022-12-30)](https://github.com/libp2p/specs/blob/master/quic/README.md)
- [libp2p TLS spec](https://github.com/libp2p/specs/tree/master/tls)
- [libp2p Noise spec (r5, 2022-12-07)](https://github.com/libp2p/specs/blob/master/noise/README.md)
- [libp2p WebRTC specs (r1, 2023-04-12)](https://github.com/libp2p/specs/tree/master/webrtc)
- [libp2p WebTransport spec](https://github.com/libp2p/specs/tree/master/webtransport)
- [libp2p WebSocket spec](https://github.com/libp2p/specs/tree/master/websockets)
- [yamux spec](https://github.com/libp2p/specs/tree/master/yamux)
- [mplex spec (deprecated)](https://github.com/libp2p/specs/blob/master/mplex/README.md)
- [libp2p-quic crate](https://crates.io/crates/libp2p-quic)
- [libp2p-webrtc crate](https://crates.io/crates/libp2p-webrtc)
- [quic-go](https://github.com/quic-go/quic-go)
- [rust-yamux](https://github.com/libp2p/rust-yamux)
- [iroh — transports (sibling doc)](../iroh/transports.md)
- [iroh — NAT traversal (sibling doc)](../iroh/nat-traversal.md)
