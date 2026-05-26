**Date:** 2026-05-22
**Status:** active
**Subject:** What WebRTC + the surrounding browser-P2P stack does not solve for a kernel-mediated P2P runtime.

# Open problems — WebRTC for browser P2P

Each entry: problem + why it matters for Myrhiza + canonical sources.

## 1. Standardized signalling is not solved and won't be

WebRTC has explicitly *no* standard signalling protocol. RFC 8825 says the signalling channel is an application choice. Twenty years in, every WebRTC app reinvents it. There is no standards-track path to changing this — the working assumption is that signalling stays application-defined forever.

**What's needed:** Myrhiza picks a signalling channel. Options: signal-over-iroh-gossip (uses Myrhiza's existing transport for SDP exchange), HTTPS-based (out-of-band server), QR-code-once (manual user-mediated, like Bluetooth pairing), or signal-over-out-of-band-cap-token-URL.

**Canonical sources:** [`signalling.md`](signalling.md), [`prior-art/iroh/`](../iroh/) (existing transport), [`prior-art/at-protocol/`](../at-protocol/) (HTTP-based signalling pattern).

## 2. TURN operator economics

WebRTC works when ICE works. When it fails (10-30% of real-world connections), TURN relay is required. TURN is bandwidth-expensive; commercial TURN-as-a-service has per-GB pricing. Myrhiza needs a story for who pays for TURN bandwidth in a P2P network.

**What's needed:** decide between operator-funded (Myrhiza-org runs TURN), user-funded (users pay for connectivity), peer-pool TURN (any peer with bandwidth volunteers), or no TURN (10-30% of connections fail). All have spec implications.

**Canonical sources:** [`signalling.md`](signalling.md), [`stack.md`](stack.md), [`prior-art/iroh/`](../iroh/) (DERP relay economics).

## 3. Browser certificate management

DTLS-over-UDP requires certificates on both sides. Browsers generate ephemeral self-signed certificates per session. Native peers must do the same — but if the native peer's certificate changes each session, it's not stable identity. The relationship between WebRTC's DTLS cert and Myrhiza's peer keypair is unclear.

**What's needed:** decide whether Myrhiza's peer identity (Ed25519 PeerKeypair) is bound to or independent of the WebRTC DTLS cert. Likely independent — the DTLS cert is per-session; peer identity is per-peer.

**Canonical sources:** [`stack.md`](stack.md), [`prior-art/signal/`](../signal/) (production-grade certificate-vs-identity separation).

## 4. WebRTC connection state-machine in a Wasm component

Myrhiza components are Wasm; the WebRTC connection state-machine lives kernel-side. The interface (component requests connection / receives data / handles disconnect) needs spec definition. The complexity is high — connection states include `new`, `connecting`, `connected`, `disconnected`, `failed`, `closed`; per-DataChannel states too.

**What's needed:** WIT-typed WebRTC capability surface. Smaller subset of full WebRTC API; only what Myrhiza apps actually need.

**Canonical sources:** [`stack.md`](stack.md), [`prior-art/jco/browser-viability.md`](../jco/browser-viability.md), [`prior-art/wasm-component-model/`](../wasm-component-model/).

## 5. Browser-to-browser libp2p-webrtc is not yet mature

libp2p-webrtc has browser-to-server stable; browser-to-browser is newer (post-2024). For Myrhiza to have browser-peer-to-browser-peer working out of the box via libp2p, this needs further maturation. Native libp2p-webrtc browser-to-browser is years behind native-to-native.

**What's needed:** track libp2p-webrtc browser-to-browser maturity. Either wait for it, build it, or use libdatachannel directly (skipping libp2p).

**Canonical sources:** [`libp2p-webrtc.md`](libp2p-webrtc.md), [`prior-art/libp2p/transports.md`](../libp2p/transports.md).

## 6. Mobile / iOS WebRTC quirks

iOS Safari ships WebRTC but with restrictions: Safari-only (no third-party browser engines until 2024 EU DMA), background-mode limitations, energy-budget throttling. Android Chrome is closer to desktop. PWA support for WebRTC differs.

**What's needed:** document mobile WebRTC capabilities + restrictions per platform. Myrhiza browser-peer profile on iOS may need different signalling timeouts, connection-keepalive intervals, etc.

**Canonical sources:** [`browser-stack.md`](browser-stack.md), [`prior-art/pears/`](../pears/) (mobile-P2P UX patterns).

## 7. WebTransport vs WebRTC choice for client-server

WebTransport is the modern replacement for many WebSocket use cases (lower latency, multiplexed streams over QUIC). For Myrhiza relay-to-browser traffic, WebTransport is simpler than WebRTC. But WebTransport is browser-to-server only, not P2P.

**What's needed:** Myrhiza spec deciding: browser-to-Myrhiza-server traffic uses WebTransport (modern); browser-to-browser traffic uses WebRTC (the only path). State the rationale.

**Canonical sources:** [`webtransport.md`](webtransport.md).

## 8. Connection establishment latency

WebRTC connection setup takes hundreds of milliseconds to seconds (ICE candidate gathering, STUN binding, DTLS handshake, DataChannel open). Cold-start for a user opening a Myrhiza app is felt. Iroh's QUIC-with-pubkey shaves this for native peers but not for browser peers.

**What's needed:** pre-warmed connections (idle DataChannels open before user action), connection pooling, or connection-resume after page reload. All have spec implications.

**Canonical sources:** [`stack.md`](stack.md), [`prior-art/iroh/`](../iroh/) (QUIC connection-resume).

## 9. NAT traversal failure modes are opaque

When ICE fails, "fails" can mean many things: no relay reachable, TURN auth rejected, asymmetric NAT, mDNS blocked, browser denied permissions. Debugging is hard; the browser surfaces minimal information.

**What's needed:** Myrhiza-side telemetry on connection failures (which step failed, what error code, time-to-failure). Without this, operators can't diagnose user reports.

**Canonical sources:** [`signalling.md`](signalling.md), [`stack.md`](stack.md).

## 10. WebRTC-NV / raw sockets / future browser P2P

WebRTC-NV proposals would expose lower-level APIs (`RTCRawDataChannel`, possible raw UDP sockets) to browsers. ChromeOS shipped raw socket access for installed PWAs (limited). The trajectory is "the browser will get better P2P primitives eventually." Years out.

**What's needed:** don't gate Myrhiza v1 on WebRTC-NV. Track the proposal. Design Myrhiza's browser-peer transport pluggability so that when raw sockets ship, switching to them is a transport-layer change, not an app-layer change.

**Canonical sources:** [`browser-stack.md`](browser-stack.md), WebRTC-NV W3C drafts.

## Cross-references

- [`README.md`](README.md), [`lessons.md`](lessons.md)
- Per-system evidence files
- [`prior-art/libp2p/`](../libp2p/), [`prior-art/iroh/`](../iroh/), [`prior-art/jco/`](../jco/), [`prior-art/sframe/`](../sframe/), [`prior-art/pears/`](../pears/), [`prior-art/at-protocol/`](../at-protocol/), [`prior-art/wasm-component-model/`](../wasm-component-model/)

## Sources

All sources in evidence files.
