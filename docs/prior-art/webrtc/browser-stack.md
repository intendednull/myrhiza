**Date:** 2026-05-22
**Status:** active
**Subject:** WebRTC inside the browser — RTCPeerConnection, mDNS-for-ICE privacy, vendor quirks

# Browser stack

What the browser actually does when JavaScript calls `new RTCPeerConnection()`. Browser internals are documented inconsistently; this file consolidates the load-bearing facts a Myrhiza spec author needs to know.

## All four major browsers ship WebRTC

Per [caniuse RTCPeerConnection](https://caniuse.com/rtcpeerconnection), as of May 2026:

| Browser | First shipped | Engine | Notes |
|---|---|---|---|
| **Chrome / Chromium / Edge** | Chrome 23 (2012-11) | libwebrtc (Chromium's C++) | The reference implementation. Everything else interops with this. |
| **Firefox** | Firefox 22 (2013-06) | mozilla-central's own C++ WebRTC fork | Originally a separate Mozilla implementation; over time has converged with libwebrtc but is still a separate codebase. |
| **Safari** | Safari 11 (2017-09) | WebKit's port of libwebrtc | Apple was the last major browser to ship WebRTC. iOS Safari support landed simultaneously. |
| **Edge (legacy, pre-Chromium)** | partial only | Microsoft's ORTC | Edge 79+ (2020) is Chromium-based; legacy Edge had only "ObjectRTC", a precursor that did not interop with WebRTC. Effectively obsolete. |

Coverage as of 2026-05: **95.94% global**. WebRTC is a baseline web platform feature.

The "all browsers" reality is more nuanced once you look at behavior:

- **libwebrtc-based browsers (Chrome, Edge, Safari, Opera, Brave)** — share a codebase. Bugs propagate; features land within a few months of each other. About 88% of users.
- **Firefox** — independent implementation. Some behavior differs at the edges. ICE candidate ordering, mDNS hostname format, SDP munging quirks. Test on Firefox separately.
- **Mobile Safari** — same WebRTC.framework as desktop Safari, but with App Store WebView restrictions. WebRTC in an in-app browser (e.g. WKWebView) historically had gaps; Safari 14.5+ exposes a fuller surface.

## What RTCPeerConnection actually does

Inside the browser, instantiating an `RTCPeerConnection`:

1. Initializes the ICE agent (no candidate gathering yet — that starts on `addTransceiver`, `addTrack`, or `createDataChannel`).
2. Initializes a DTLS context (generates a fresh keypair if no cert is supplied via `certificates` config — this happens lazily).
3. Sets up the SDP state machine.

Then, as the application calls `createOffer()` or attaches tracks/channels, the browser:

4. Triggers ICE candidate gathering across all network interfaces.
5. Asks the OS for STUN servers (from `iceServers` config) and TURN servers (same).
6. Begins emitting `icecandidate` events as candidates are discovered.
7. Builds the SDP that includes the local DTLS fingerprint + initial candidates.

On `setRemoteDescription`, parses peer's SDP, verifies the structure, starts connectivity checks against received candidates.

On `selected candidate pair`, begins the DTLS handshake.

On DTLS established, begins SCTP association.

On SCTP up, the `RTCDataChannel` open event fires (if any).

The whole flow is *opaque* — the browser does it. JS gets events at major transitions but cannot inspect internal state in detail (no "tell me which candidate pair you're using" API, except via `getStats()` which dumps a large structured blob).

## mDNS for ICE candidates — the privacy-improvement that matters

The single most operationally important browser behavior change in the last decade for WebRTC is **mDNS-obfuscated host candidates**.

### The problem

Pre-2019, when a browser gathered ICE candidates, it included **every local IP address on every network interface** in the SDP. Including `10.0.0.42`, `192.168.1.7`, etc. This SDP was sent to the peer (and any signalling server in between) and from there often persisted in logs.

The consequence: **any WebRTC-using website could learn the user's private LAN IP**, even without the user accepting a media stream. By embedding a hidden `<video>` with a WebRTC peer-connection that never actually negotiated, sites could log local IPs. This became a fingerprinting + tracking vector.

### The fix

[`draft-ietf-mmusic-mdns-ice-candidates`](https://datatracker.ietf.org/doc/draft-ietf-mmusic-mdns-ice-candidates/) (Internet-Draft, **expired 2021-12-06**, never reached RFC status, but shipped in browsers anyway). The fix:

- Browsers replace local IPs in ICE candidates with **mDNS `.local` hostnames** like `abcd1234-5678-9abc-def0-1234567890ab.local`.
- The hostname-to-IP mapping is published only via local-link mDNS multicast (RFC 6762), reachable only on the local network.
- Peers on the same LAN can resolve the hostname (via local mDNS) and connect directly.
- Peers off the LAN see only the hostname (useless to them) and either fail or fall back to STUN-reflexive / TURN-relay.

### Status by browser

- **Chrome 76+ (2019-07):** mDNS-obfuscated host candidates by default, even without user-granted media permission.
- **Firefox 70+ (2019-10):** same.
- **Safari 12.1+ (2019-03):** same.
- **Edge (Chromium)** inherits Chromium's behavior.

The draft expired without becoming an RFC, but **it shipped in every major browser anyway** — the privacy benefit was important enough that the IETF-process delay didn't matter. This is an unusual case of "vendor consensus implements a draft, formal standardization stalls, nobody cares."

### Implications

- **Local-network peer-to-peer WebRTC depends on mDNS multicast** working. Corporate networks that block mDNS (some do, for security) break local WebRTC. The fallback is STUN-reflexive, which doesn't help if both peers are behind the same NAT.
- **Browsers that have user-granted media permission expose the underlying IP.** Chrome's behavior: once the user accepts a `getUserMedia()` request, host candidates revert to real IPs. This is a UX-driven privacy/functionality tradeoff.
- **Spec authors who study the SDP can't compare local IPs across browsers.** Different browsers generate different mDNS hostnames for the same underlying IP. Identity-correlation via SDP is harder than it once was.

For Myrhiza: this is a *feature*, not a problem. If Myrhiza's signalling channel relays SDP, the mDNS hostnames mean the signalling-channel operator sees less metadata than they would have pre-2019. Browser-vendor privacy improvements ratcheting forward.

## DataChannel-specific behavior

`RTCDataChannel` is the data-only path; this is what Myrhiza cares about. Browser-specific gotchas:

- **Maximum message size.** The spec defines `maxMessageSize` per peer (negotiated via SDP). In practice:
  - Chrome: 256 KB.
  - Firefox: 1 GB (effectively unbounded, but the underlying SCTP fragmenting limits practical use).
  - Safari: 64 KB (historically lower).
  - **Cross-browser: assume 64 KB max per message.** Larger payloads must be chunked at the application layer.
- **Backpressure.** `bufferedAmount` reports queued bytes. `bufferedamountlow` event fires when below threshold. Standard, but easy to misuse — sending a large file without checking `bufferedAmount` blows out memory.
- **Ordered vs unordered.** Configurable per channel. Once set, can't change.
- **Reliable vs unreliable.** `maxRetransmits: 0` or `maxPacketLifeTime: 0` gives UDP-like delivery. Useful for telemetry / multiplayer; rarely useful for Myrhiza event-log shape.
- **Stream IDs.** Each `RTCDataChannel` gets an SCTP stream ID; the count is bounded by SCTP (65,535 streams per association). For Myrhiza, one association will have a small number of channels — not a constraint.

## `getStats()` — observability

The only structured introspection JS has into a WebRTC connection is `peerConnection.getStats()`. It returns a structured map of metrics:

- per-candidate-pair latency, packet loss
- per-channel byte counts, dropped messages
- ICE state, DTLS state, current selected pair
- jitter buffer stats (for media)

This is the data Myrhiza's browser-peer kernel would surface to apps for observability. It's *verbose* and the field names change between browsers (libwebrtc evolves), but it exists.

## TLS / DTLS cert lifetime

Browsers generate a fresh DTLS cert + keypair per `RTCPeerConnection` *by default*. The application can opt to specify a long-lived cert via `RTCPeerConnection({ certificates: [cert] })`, where `cert = await RTCPeerConnection.generateCertificate(...)`.

Why this matters:

- **Default behavior:** every peer connection has a fresh fingerprint. Identity is *not* tied to the WebRTC cert; the application's identity layer (Myrhiza's identity) is separate.
- **Long-lived certs:** apps can pin a cert per session/user, surfacing identity in the cert fingerprint. This is *unusual* but legal.

For Myrhiza, the **default fresh-cert behavior is right**. Myrhiza identity is its own layer (Ed25519 keys signing events), and the DTLS cert is just per-session encryption keys. Don't try to overload them.

## SDP munging

A historical anti-pattern: applications would `setLocalDescription(sdp)` with an SDP they'd hand-edited (substituting codec preferences, modifying ICE candidates, etc). This worked because the browser was tolerant of any SDP it could parse.

Modern WebRTC has moved away from this:

- `RTCRtpTransceiver` / `RTCRtpSender.setParameters()` exposes most modification surfaces via real APIs.
- Browsers increasingly *reject* SDP they didn't generate. Chrome warns; Safari sometimes errors.
- "SDP munging" is officially deprecated; the W3C considers it out of spec.

For Myrhiza: don't munge SDP. Use the APIs.

## What's still vendor-specific

After 10+ years of WebRTC, you'd expect cross-browser parity. The reality:

- **mDNS hostname format** — same general scheme, different UUID formatting per browser.
- **Default codec preferences** — different orderings; only matters for media, not data channels.
- **SCTP message size limits** — already noted; cross-browser-safe is 64 KB.
- **`getStats()` field names** — drift between Chromium and Firefox; libraries like `webrtc-stats-aggregator` paper over it.
- **`onnegotiationneeded` event timing** — Chrome and Firefox differ in when they fire it; race conditions are common.
- **Trickle ICE behavior** — Firefox sometimes batches candidates differently than Chromium.
- **Renegotiation flow** — different browsers fire ICE restart events at slightly different times.

These are the long tail of "WebRTC interop bugs" — every team that ships WebRTC at scale has a list. The list never gets shorter.

For Myrhiza: this means **the browser-peer kernel needs a small but real cross-browser test matrix**. "It works in Chrome" is not enough. If Safari support is in scope (likely yes — it's >15% of mobile traffic), Safari testing is mandatory.

## Implications for Myrhiza

1. **The browser owns the WebRTC stack.** The Myrhiza browser-peer kernel is a *consumer* of the browser's WebRTC, not an implementer. This is good — we don't have to port libwebrtc to WASM — but it means our spec is bound by what the browser exposes.
2. **mDNS-for-ICE means LAN WebRTC depends on mDNS routing.** Local-network Myrhiza peer-to-peer needs LANs to allow mDNS multicast. Most consumer LANs do; some enterprise LANs don't. Note as a known limitation.
3. **64 KB per-message cap.** The Myrhiza event-log message format must fit in 64 KB per RTCDataChannel `send()`, or be chunked. Probably we want chunking anyway for streaming behavior.
4. **`getStats()` is the observability surface.** The Myrhiza browser-peer kernel should expose a normalized projection of `getStats()` to apps via host imports (read-only). Useful for "this peer is slow" diagnostics.
5. **Cross-browser test matrix is non-negotiable.** Whatever the Myrhiza browser-peer spec ends up being, it must be tested on at least Chromium + Firefox + Safari; mobile Safari especially.

## Sources

- caniuse RTCPeerConnection: <https://caniuse.com/rtcpeerconnection>
- mDNS-for-ICE draft (expired 2021-12-06): <https://datatracker.ietf.org/doc/draft-ietf-mmusic-mdns-ice-candidates/>
- RFC 6762 — Multicast DNS: <https://datatracker.ietf.org/doc/rfc6762/>
- W3C WebRTC Recommendation (2025-03-13): <https://www.w3.org/TR/webrtc/>
- Chrome 76 mDNS hosts announcement: <https://groups.google.com/g/discuss-webrtc/c/6stQXi72BEU>
- MDN RTCDataChannel: <https://developer.mozilla.org/en-US/docs/Web/API/RTCDataChannel>
- Cross-refs: [`stack.md`](stack.md), [`signalling.md`](signalling.md), [`open-problems.md`](open-problems.md)
