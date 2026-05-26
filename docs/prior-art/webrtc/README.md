**Date:** 2026-05-22
**Status:** active
**Subject:** WebRTC — browser P2P transport (stack + implementations + signalling + WebTransport)

# WebRTC

WebRTC = the standardized browser P2P stack (RTCPeerConnection / RTCDataChannel + ICE + DTLS + SCTP-over-DTLS-over-UDP) plus a constellation of native implementations (libdatachannel, str0m, webrtc-rs, pion) plus a non-trivial set of "how do peers find each other" patterns (signalling). This folder treats it as one ecosystem because **for Myrhiza's browser-peer profile, WebRTC is the only path to true browser-to-browser P2P that doesn't traverse a server hop**. Iroh has no WebRTC; libp2p does but treats it as one of six transports; Myrhiza, via jco, has to either pick WebRTC or accept a relay-only browser story.

**WebTransport** is included here as a complementary HTTP/3 / QUIC-based browser transport — *not* a replacement, contrary to common confusion. It solves browser-to-server, not browser-to-browser. We document it alongside because spec authors comparing browser-peer options will encounter both.

## Key facts

| Fact | Value | Source |
|---|---|---|
| W3C Recommendation | `webrtc` published 2025-03-13 (latest version of REC) | <https://www.w3.org/TR/webrtc/> |
| IETF overview RFC | RFC 8825, January 2021 (informational, "applicability statement") | <https://www.rfc-editor.org/rfc/rfc8825.html> |
| ICE | RFC 8445 (July 2018), Proposed Standard, obsoletes RFC 5245 | <https://datatracker.ietf.org/doc/rfc8445/> |
| STUN | **RFC 8489 (February 2020)**, obsoletes RFC 5389 | <https://datatracker.ietf.org/doc/rfc8489/> |
| TURN | **RFC 8656 (February 2020)**, obsoletes RFC 5766 + 6156 | <https://datatracker.ietf.org/doc/rfc8656/> |
| mDNS for ICE candidates | `draft-ietf-mmusic-mdns-ice-candidates-03`, **expired 2021-12** (never an RFC; shipped in browsers anyway) | <https://datatracker.ietf.org/doc/draft-ietf-mmusic-mdns-ice-candidates/> |
| Browser support (RTCPeerConnection) | Chrome 23+, Firefox 22+, Safari 11+, Edge 79+; **95.94% global** | <https://caniuse.com/rtcpeerconnection> |
| Browser support (WebTransport) | Chrome 97+, Edge 98+, Firefox 114+, **Safari 26.4 (very recent)**; ~80% global | <https://caniuse.com/webtransport> |
| `libdatachannel` | v0.24.3 (2026-05-09), C++ with C bindings, **MPL-2.0** (since v0.18), 2.6k★ | <https://github.com/paullouisageneau/libdatachannel> |
| `str0m` | v0.19.0 (crates.io 2026-05-04), Rust, MIT OR Apache-2.0, "Sans-IO" pure state machine, 552★ | <https://crates.io/crates/str0m> |
| `webrtc-rs` | v0.17.1 stable (2026-02-06), v0.20.0-alpha.1 (2026-03-01), MIT OR Apache-2.0, Tokio-coupled, 5k★ | <https://crates.io/crates/webrtc> |
| `pion/webrtc` | v4.2.13 (2026-05-22), Go, MIT, 16.5k★, the Go canonical | <https://github.com/pion/webrtc> |
| `simple-peer` (npm) | v9.11.1 published **2022-02-17** (~4 years stale), MIT, 7.8k★, 97 open issues | <https://registry.npmjs.org/simple-peer> |
| `trystero` (npm) | v0.24.0 (2026-04-27), TypeScript, MIT, 2.6k★, "Serverless peer-to-peer for the web" | <https://registry.npmjs.org/trystero> |
| `libp2p-webrtc` spec | r1 / Candidate Recommendation, 2023-04-12; browser-to-server stable, browser-to-browser newer | <https://github.com/libp2p/specs/tree/master/webrtc> |

## What WebRTC actually is

A misconception worth correcting up front: **WebRTC is not pure P2P**.

1. **Signalling is out-of-band.** Two browsers cannot start a WebRTC session without first exchanging SDP offer/answer + ICE candidates through *some* channel that is itself not WebRTC — typically a WebSocket to a signalling server, or a DHT, or a BitTorrent tracker (trystero), or a libp2p stream over a relay.
2. **NAT traversal often fails.** ICE successfully establishes a direct connection somewhere in the 70–90% range depending on network mix; the remaining 10–30% require **TURN relay**, which is not P2P (the relay sees all traffic, just encrypted).
3. **DTLS encrypts but signalling sees metadata.** The signalling channel observes who is dialing whom and when. End-to-end content encryption between peers does not hide that.

These three facts are load-bearing for the Myrhiza spec. See [`lessons.md`](lessons.md) for the design implications.

## Folder layout

- **[`README.md`](README.md)** — this file. Key facts + ToC + reading order.
- **[`stack.md`](stack.md)** — the protocol stack: RTCPeerConnection / DataChannel API → SCTP → DTLS → ICE → STUN/TURN → UDP. What each layer does and why.
- **[`signalling.md`](signalling.md)** — **the load-bearing-for-Myrhiza file.** How WebRTC peers find each other. Catalogues every signalling pattern (HTTPS server, WebSocket server, libp2p stream, libp2p-webrtc, DHT, BitTorrent tracker, Nostr, MQTT, IPFS pubsub, manual copy-paste).
- **[`implementations.md`](implementations.md)** — libdatachannel / str0m / webrtc-rs / pion side-by-side. Architecture (sans-IO vs Tokio-coupled), maturity, ecosystem fit.
- **[`browser-stack.md`](browser-stack.md)** — what `RTCPeerConnection` actually does inside the browser, mDNS-for-ICE privacy behavior, what's still vendor-specific.
- **[`webtransport.md`](webtransport.md)** — HTTP/3-based browser transport. **Complementary, not a WebRTC replacement.** Why it doesn't do browser-to-browser; when it's the right tool.
- **[`libp2p-webrtc.md`](libp2p-webrtc.md)** — the libp2p ecosystem's two WebRTC profiles (browser-to-server / WebRTC-Direct, browser-to-browser). Spec status, implementation maturity, certhash trick.
- **[`open-problems.md`](open-problems.md)** — what WebRTC structurally doesn't solve (and what is unlikely to be solved): signalling discovery, TURN economics, mobile background, metadata leaks, browser-quirk inventory.
- **[`lessons.md`](lessons.md)** — **the consult-this-when-designing decision file.** Validates / avoid / borrow for the Myrhiza browser-peer profile.

## Recommended reading order

1. **[`stack.md`](stack.md)** — get the protocol layering straight; without this nothing else makes sense.
2. **[`signalling.md`](signalling.md)** — the most important file for Myrhiza. The honest answer to "how does WebRTC find peers" is the design decision you cannot avoid. JS-ecosystem patterns (simple-peer, trystero, PeerJS) are covered inline here.
3. **[`browser-stack.md`](browser-stack.md)** — what the browser actually does when you call `new RTCPeerConnection()`. The privacy-via-mDNS behavior is a load-bearing-for-spec fact.
4. **[`implementations.md`](implementations.md)** — pick a library mental model: libdatachannel for embedded, str0m for sans-IO Rust, pion for Go, webrtc-rs for Tokio Rust.
5. **[`libp2p-webrtc.md`](libp2p-webrtc.md)** — the spec closest to what Myrhiza would build (browser-peer as a transport profile).
6. **[`webtransport.md`](webtransport.md)** — the complementary alternative when browser-to-server is acceptable.
7. **[`open-problems.md`](open-problems.md)** — context and unsolved problems (TURN economics + NAT-traversal accounting covered here).
8. **[`lessons.md`](lessons.md)** — synthesis for the Myrhiza spec.

## Cross-links to other corpus folders

- **[`prior-art/jco/browser-viability.md`](../jco/browser-viability.md)** — the runtime side of the browser-peer profile. The jco file says "browsers have no raw socket API; need WebRTC or relay." This folder is the WebRTC half.
- **[`prior-art/libp2p/transports.md`](../libp2p/transports.md)** — libp2p's transport matrix, which includes WebRTC. The libp2p-webrtc spec is the most directly relevant spec for "what would Myrhiza-over-WebRTC look like."
- **[`prior-art/iroh/transports.md`](../iroh/transports.md), [`prior-art/iroh/critiques.md`](../iroh/critiques.md)** — iroh's browser story is relay-only (WebTransport-backed alpha since 0.32). The lesson is: even the best QUIC-based P2P stack does *not* solve browser-to-browser.
- **[`prior-art/pears/transport-comparison.md`](../pears/transport-comparison.md)** — Hyperswarm's UDP holepunching as a non-browser comparator. "DHT discovery + holepunch" is what WebRTC reinvents from scratch every signalling channel.

## How to use

This folder is for: spec authors weighing browser-peer transport options for Myrhiza; reviewers reading those specs; and future agents who need to skim the WebRTC ecosystem without reading 30 specs and 8 README's.

**Framing disclosure.** These docs are written from a Myrhiza-needs-a-browser-peer-profile stance — most "Implications for Myrhiza" sub-sections frame WebRTC's choices through whether they survive contact with our jco-transpiled-WASM-component browser kernel. Future readers auditing whether browser-peer-via-WebRTC is itself the right primitive (vs accepting relay-only and shipping faster) should weigh the corpus accordingly: it's a learn-from-WebRTC-into-Myrhiza-browser-peer artifact, not a neutral catalog of "should we use WebRTC at all." We also have a load-bearing-dependency-style bias: if Myrhiza commits to WebRTC, the corpus has an incentive to soft-pedal its operational rough edges. We try to surface those (especially in [`stack.md`](stack.md) §"Operational reality" and [`open-problems.md`](open-problems.md)) — but be skeptical.

## Sources

- W3C WebRTC Recommendation (2025-03-13): <https://www.w3.org/TR/webrtc/>
- RFC 8825 — WebRTC overview: <https://www.rfc-editor.org/rfc/rfc8825.html>
- RFC 8445 — ICE: <https://datatracker.ietf.org/doc/rfc8445/>
- RFC 8489 — STUN: <https://datatracker.ietf.org/doc/rfc8489/>
- RFC 8656 — TURN: <https://datatracker.ietf.org/doc/rfc8656/>
- libdatachannel: <https://github.com/paullouisageneau/libdatachannel>
- str0m: <https://github.com/algesten/str0m>, <https://crates.io/crates/str0m>
- webrtc-rs: <https://github.com/webrtc-rs/webrtc>, <https://crates.io/crates/webrtc>
- pion/webrtc: <https://github.com/pion/webrtc>
- simple-peer: <https://github.com/feross/simple-peer>, <https://registry.npmjs.org/simple-peer>
- trystero: <https://github.com/dmotz/trystero>, <https://registry.npmjs.org/trystero>
- libp2p WebRTC specs: <https://github.com/libp2p/specs/tree/master/webrtc>
- caniuse RTCPeerConnection: <https://caniuse.com/rtcpeerconnection>
- caniuse WebTransport: <https://caniuse.com/webtransport>
