**Date:** 2026-05-22
**Status:** active
**Subject:** The decision file — what WebRTC prior art validates, what to avoid, what to borrow when designing Myrhiza's browser-peer transport story.

# Lessons for Myrhiza — WebRTC for browser P2P

Synthesis across [`stack.md`](stack.md), [`browser-stack.md`](browser-stack.md), [`implementations.md`](implementations.md), [`signalling.md`](signalling.md), [`libp2p-webrtc.md`](libp2p-webrtc.md), [`webtransport.md`](webtransport.md). Format: validates / avoid / borrow.

## Validates

1. **Browser direct-P2P is technically achievable but requires WebRTC.** The browser has no UDP/QUIC socket primitives, no listening TCP, no raw socket access. RTCPeerConnection + RTCDataChannel is the only path to direct peer connections. Any Myrhiza browser-peer plan that wants to avoid a relay must use WebRTC. *Source: [`browser-stack.md`](browser-stack.md), [`stack.md`](stack.md).*

2. **Browser support is mature.** RTCPeerConnection ships in Chrome 23+, Firefox 22+, Safari 11+, Edge 79+; 95.94% global support per caniuse. The browser-side is stable; the trade-offs are elsewhere (signalling, TURN, ICE failure rate). *Source: [`browser-stack.md`](browser-stack.md).*

3. **Multiple Rust implementations exist for native peers.** `webrtc-rs` (Tokio-coupled, 5k★, v0.17.1), `str0m` (Sans-IO, 552★, v0.19.0), `libdatachannel` C++ (MPL-2.0, 2.6k★) with Rust bindings — three viable Rust choices for the native side. Myrhiza isn't forced into a single library. *Source: [`implementations.md`](implementations.md).*

4. **Sans-IO design is the right architectural pattern for a kernel-mediated runtime.** `str0m`'s pure-state-machine approach (no embedded IO, no embedded runtime) is the canonical Sans-IO Rust example. For Myrhiza — where the kernel owns IO and components are pure-by-construction — Sans-IO is the natural integration point. *Source: [`implementations.md`](implementations.md).*

5. **libp2p-webrtc shows the WIT/component path is feasible.** The libp2p-webrtc spec (Candidate Recommendation 2023-04-12) defines a browser-to-server and browser-to-browser WebRTC transport that's been integrated into rust-libp2p. Browser-to-server is stable; browser-to-browser is newer but shipping. This validates "WebRTC as one transport among many in a P2P stack." *Source: [`libp2p-webrtc.md`](libp2p-webrtc.md).*

## Avoid

| Pitfall | Source | Mitigation |
|---|---|---|
| **Claiming WebRTC is "P2P" without nuance.** WebRTC requires out-of-band signalling (which is not P2P) and frequently a TURN relay (also not P2P). Marketing it as a pure P2P transport misleads spec authors and users. | [`signalling.md`](signalling.md), [`stack.md`](stack.md) | Myrhiza specs must distinguish: (a) signalling = how peers find each other (out-of-band); (b) ICE = how peers attempt direct connection (P2P when it works); (c) TURN = relay fallback (not P2P). State which layer the spec is talking about. |
| **Ignoring TURN. ICE failure rate in production is non-trivial.** Symmetric NATs, restrictive firewalls, mobile carrier NATs all push ICE failure to 10–30% in real-world deployments. Without a TURN fallback, those users can't connect at all. | [`signalling.md`](signalling.md), [`stack.md`](stack.md) | If Myrhiza supports WebRTC for browser peers, plan for TURN as a deployed component. Either operate TURN (operator cost) or use a TURN-relay-as-service. Document the trust model — TURN sees encrypted traffic, not plaintext, but volumes are visible. |
| **`simple-peer` as a current dependency.** v9.11.1 published 2022-02-17 — ~4 years stale. 7.8k★ but 97 open issues. Feross moved on; maintainer attention low. | [`implementations.md`](implementations.md) | Use `trystero` (v0.24.0, 2026-04-27, actively maintained) or the browser-native `RTCPeerConnection` API directly. |
| **WebTransport as a WebRTC replacement.** WebTransport is QUIC-over-H3, client-to-server, not P2P. It's complementary for browser-to-server use cases (and has matured: Chrome 97+, Firefox 114+, Safari 26.4). It does not replace WebRTC for peer-to-peer. | [`webtransport.md`](webtransport.md) | If Myrhiza wants browser-to-server traffic (e.g., a coordinating relay), WebTransport is the modern choice. For browser-to-browser, WebRTC remains the only path. |
| **mDNS-for-ICE-candidates as stable spec.** The mDNS ICE-candidate draft (`draft-ietf-mmusic-mdns-ice-candidates-03`) expired 2021-12 but ships in browsers anyway. Real but politically awkward. | [`browser-stack.md`](browser-stack.md) | Document that local-network peer discovery via mDNS is real browser behavior. Don't cite the IETF draft as a stable spec — note the expired status. |
| **DTLS-over-SCTP wire format as the long-term plan.** WebRTC's data-channel wire format (DTLS over UDP, with SCTP framing inside) is a complex legacy. WebRTC-NV proposals (`RTCRawDataChannel`, raw socket access in browsers) would replace it but are years out. | [`stack.md`](stack.md) | Accept the wire format as-is for v1 browser support. Track WebRTC-NV but don't gate v1 on it. |
| **Tying Myrhiza signalling to a specific server protocol.** WebRTC signalling has no standard — every WebRTC app reinvents it. Picking a server protocol (Matrix, custom HTTPS, signalling-over-iroh-gossip) couples Myrhiza to that protocol's stewardship. | [`signalling.md`](signalling.md) | Treat signalling as the transport-pluggability boundary. Define the *information* exchanged (SDP offer/answer, ICE candidates) and let operators choose the *channel*. |
| **`webrtc-rs` Tokio coupling in a non-Tokio kernel.** `webrtc-rs` requires a Tokio runtime. If Myrhiza's kernel uses a different async runtime (or wants Sans-IO), `webrtc-rs` is the wrong choice. | [`implementations.md`](implementations.md) | If Myrhiza is Tokio-based, `webrtc-rs` is fine. Otherwise prefer `str0m` (Sans-IO) or `libdatachannel`. |

## Borrow

1. **`str0m` Sans-IO pattern as the kernel-side WebRTC.** State-machine library with no IO; kernel drives it. Matches Myrhiza's kernel-owns-IO discipline. *See [`implementations.md`](implementations.md).*

2. **libp2p-webrtc spec as the transport-layer model.** libp2p has thought about how to fit WebRTC into a transport-pluggable stack. Their model (one of N transports, browser-side and native-side both supported, fallback to relay) is directly applicable. *See [`libp2p-webrtc.md`](libp2p-webrtc.md), cross-ref [`prior-art/libp2p/transports.md`](../libp2p/transports.md).*

3. **trystero for serverless WebRTC patterns.** Trystero shows that creative signalling channels (BitTorrent trackers, Nostr, Firebase, IPFS PubSub) can replace dedicated signalling servers for hobbyist-scale apps. The library is small enough that the patterns are readable. *See [`implementations.md`](implementations.md).*

4. **TURN-relay-as-capability.** Following the kernel-mediation pattern: an app that wants WebRTC gets a TURN-relay capability from the kernel, not direct access. The kernel can throttle, log, or revoke TURN usage. *See [`signalling.md`](signalling.md), cross-ref [`prior-art/iroh/`](../iroh/) (DERP relay parallel).*

5. **DataChannel-only WebRTC for Myrhiza v1.** Most P2P apps need data channels, not media channels. Restricting Myrhiza's WebRTC surface to RTCDataChannel (skipping RTP/RTCP/SRTP) keeps the implementation small. Media support is a v2+ topic ([`prior-art/sframe/`](../sframe/)). *See [`stack.md`](stack.md).*

6. **WebTransport as a Myrhiza-to-server fallback path.** When direct WebRTC fails and a relay is available, WebTransport (browser → server) is the modern way to reach a Myrhiza-operated relay from the browser. Lower complexity than DTLS-SCTP. *See [`webtransport.md`](webtransport.md).*

## The single most important lesson

**WebRTC is "the browser's only direct-peer transport" — not "browser P2P solved."** The path from "I have a WebRTC connection" to "I have a working P2P app" still requires (a) signalling out-of-band, (b) TURN fallback for ICE failures, (c) certificate/key management for DTLS, (d) connection-state management. Myrhiza picking WebRTC for browser peers commits to all four. The alternative — relay-only browser peers — is simpler operationally at the cost of "no direct peer connection from the browser." Pick deliberately.

## Cross-references

- [`README.md`](README.md), [`stack.md`](stack.md), [`browser-stack.md`](browser-stack.md), [`implementations.md`](implementations.md), [`signalling.md`](signalling.md), [`libp2p-webrtc.md`](libp2p-webrtc.md), [`webtransport.md`](webtransport.md)
- [`prior-art/libp2p/transports.md`](../libp2p/transports.md) (libp2p-webrtc parent context)
- [`prior-art/iroh/critiques.md`](../iroh/critiques.md) (iroh's browser story)
- [`prior-art/jco/browser-viability.md`](../jco/browser-viability.md) (runtime side of browser-peer)
- [`prior-art/sframe/`](../sframe/) (E2EE media if Myrhiza grows A/V)

## Sources

All sources in evidence files.
