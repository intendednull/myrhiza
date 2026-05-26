**Date:** 2026-05-22
**Status:** active
**Subject:** WebRTC — the protocol stack (RTC API → SCTP → DTLS → ICE → STUN/TURN → UDP)

# Stack

WebRTC is **not one protocol** — it is a layer cake of independently specified protocols glued together by the W3C WebRTC API. Spec authors who treat it as "the WebRTC protocol" miss that every layer has its own failure modes, its own RFC, and its own ecosystem of implementations.

The cake, top to bottom:

```
┌─────────────────────────────────────────────────┐
│ Application JS                                  │
├─────────────────────────────────────────────────┤
│ W3C WebRTC API                                  │  ← RTCPeerConnection, RTCDataChannel, MediaStream
│   - RTCPeerConnection                            │
│   - RTCDataChannel                               │
├─────────────────────────────────────────────────┤
│ SCTP (for data channels)        │ SRTP (media)  │  ← RFC 4960 + RFC 8831 (SCTP/DTLS), RFC 3711 (SRTP)
├─────────────────────────────────────────────────┤
│ DTLS 1.2 (or 1.3 since recently)                │  ← RFC 6347 + RFC 9147; mandatory; provides cert exchange
├─────────────────────────────────────────────────┤
│ ICE (RFC 8445)                                  │  ← candidate gathering, connectivity checks
│   ├ STUN (RFC 8489)                              │  ← reflexive address discovery
│   └ TURN (RFC 8656)                              │  ← relay fallback when direct fails
├─────────────────────────────────────────────────┤
│ UDP (or TCP fallback for ICE-TCP)               │  ← RFC 768 (UDP), RFC 6544 (ICE-TCP)
└─────────────────────────────────────────────────┘
```

## Layer 1: the W3C WebRTC API

The browser-facing surface. Three top-level objects:

- **`RTCPeerConnection`** — the connection lifecycle owner. Holds the SDP state machine (offer/answer), the ICE agent (candidate gathering), the DTLS transport, and any number of associated SCTP/SRTP streams.
- **`RTCDataChannel`** — a bidirectional, ordered-or-unordered, reliable-or-unreliable bytestream over SCTP-over-DTLS. The data-only path; ignores SRTP entirely. This is the Myrhiza-relevant surface.
- **`MediaStream` / `RTCRtpSender` / `RTCRtpReceiver`** — the audio/video path. Uses SRTP, not SCTP. Not relevant for Myrhiza-data use cases, but worth knowing because the same `RTCPeerConnection` carries both, and SDP describes them as a single bundle.

The W3C spec ([`webrtc` Recommendation, 2025-03-13](https://www.w3.org/TR/webrtc/)) is mature; the surface barely changes year to year. Most "WebRTC API" critiques are about ergonomics (verbose, callback-heavy) rather than capability gaps.

### The SDP offer/answer dance

`RTCPeerConnection` is fundamentally a state machine driven by **SDP** (Session Description Protocol, RFC 8866) blobs. Caller creates an `offer`, callee responds with an `answer`. Each blob describes local media/data tracks, ICE candidates, DTLS fingerprints, and codec preferences.

```
Caller                                       Callee
  │  pc.createOffer() → SDP_A                  │
  │  pc.setLocalDescription(SDP_A)             │
  │ ─── [signalling channel] ──→ SDP_A ─────→  │
  │                                            │  pc.setRemoteDescription(SDP_A)
  │                                            │  pc.createAnswer() → SDP_B
  │                                            │  pc.setLocalDescription(SDP_B)
  │ ←──── [signalling channel] ─── SDP_B ───── │
  │  pc.setRemoteDescription(SDP_B)            │
  │                                            │
  │ ─── ICE candidates (both directions) ──→   │  via signalling channel, trickle
  │ ◆─── DTLS handshake (direct or via TURN)─◆ │
  │ ◆─── SCTP association + RTCDataChannel ──◆ │
```

The signalling channel is **not part of WebRTC**. See [`signalling.md`](signalling.md) — every WebRTC deployment invents or imports its own.

### Trickle ICE

Originally, the SDP offer/answer carried all ICE candidates at once. Modern WebRTC supports **trickle ICE** (RFC 8838): the offer/answer can be sent before candidate gathering completes, with candidates streaming in as they're discovered. This shaves seconds off connection setup. Every modern browser does this; you have to opt out to get the old behavior.

## Layer 2a: SCTP (for data channels)

[**Stream Control Transmission Protocol**](https://datatracker.ietf.org/doc/rfc4960/) — RFC 4960, originally designed for telecom signalling on dedicated networks, repurposed for WebRTC. Carries the bytes for `RTCDataChannel`.

SCTP gives data channels:

- **Multiple streams** over one association (similar to QUIC's streams, predating QUIC by ~15 years). Each `RTCDataChannel` is one SCTP stream.
- **Configurable reliability**: per-channel `ordered` flag + `maxRetransmits` / `maxPacketLifeTime`. Unreliable-unordered (like UDP), reliable-ordered (like TCP), and intermediate modes are all on the same association.
- **Message-oriented** rather than byte-stream-oriented. Each `send()` call is one logical message (chunked if needed).
- **Built-in congestion control** (SCTP's own, similar shape to TCP's).

The "encapsulation" RFC is **RFC 8831** (SCTP over DTLS over UDP). RFC 8832 specifies the WebRTC-specific data channel protocol on top of SCTP. Both are 2021 Proposed Standards.

### Why SCTP is showing its age

- The SCTP spec predates QUIC and predates the "everything over UDP" consensus. Its handshake is a 4-way COOKIE-ECHO/COOKIE-ACK exchange that runs *after* the DTLS handshake. Total: ~3 RTTs (ICE + DTLS + SCTP) before the first byte of application data flows.
- SCTP's congestion control is a separate stack from any underlying QUIC/TCP control, making interactions on shared bottlenecks suboptimal.
- The most-used SCTP implementation (`usrsctp`, used by Chromium/libdatachannel/etc) has been **functionally unmaintained**: rare commits, no active release pipeline. This is a chronic critique within the WebRTC implementer community.

For browser-to-browser this is "live with it" (it's what RFC 8831 mandates). For new transport designs, QUIC-with-streams supersedes SCTP-over-DTLS cleanly — which is part of why WebTransport exists (see [`webtransport.md`](webtransport.md)).

## Layer 2b: SRTP (for media)

Mentioned for completeness. [RFC 3711](https://datatracker.ietf.org/doc/rfc3711/) (with DTLS-SRTP key derivation per [RFC 5763](https://datatracker.ietf.org/doc/rfc5763/)). Carries audio/video. Not used by `RTCDataChannel`. Skip for Myrhiza.

## Layer 3: DTLS

[**Datagram TLS**](https://datatracker.ietf.org/doc/rfc6347/) — RFC 6347 (DTLS 1.2) or RFC 9147 (DTLS 1.3). Provides:

- **Encryption** (AES-GCM, ChaCha20-Poly1305).
- **Authentication** of the peer's certificate fingerprint (which is exchanged via the SDP, signed by no one — the trust comes from the signalling channel knowing it sent the right SDP to the right peer).
- **Handshake transport** for SCTP/SRTP keying material.

DTLS over UDP is a *full* TLS handshake adapted to handle UDP packet loss/reorder. The handshake costs 1–2 RTTs depending on caching.

### "Certificate fingerprints" are not CA certs

The DTLS cert in a WebRTC connection is **self-signed**. The trust model is:

1. Browser-A generates a keypair and a self-signed cert.
2. Browser-A's SDP offer includes a SHA-256 fingerprint of that cert (`a=fingerprint:sha-256 AB:CD:...`).
3. Browser-B receives the SDP via the signalling channel.
4. During the DTLS handshake, Browser-B verifies that the cert presented matches the fingerprint in the SDP.

The security guarantee: **whoever sent the SDP is who you're encrypting with.** This is "trust the signalling channel" — the SDP itself is unauthenticated; if the signalling channel is MITM'd, WebRTC is MITM'd. (See [`signalling.md`](signalling.md) for the practical implication: signalling MUST be authenticated end-to-end if you care about MITM resistance.)

Spec-side, this is unusual: WebRTC declined the CA-PKI trust model that TLS uses, in favor of "the signalling channel is the trust anchor." For Myrhiza this is actually a *good* match — Myrhiza apps already have an identity model (event-log keys), and we control the signalling, so fingerprint-pinning fits cleanly.

## Layer 4: ICE

[**Interactive Connectivity Establishment**](https://datatracker.ietf.org/doc/rfc8445/) — RFC 8445 (July 2018, Proposed Standard, obsoletes the original RFC 5245 from 2010). The brains of NAT traversal.

ICE is a state machine that:

1. **Gathers candidates** (potential IP:port pairs) from multiple sources:
   - **Host candidate** — local IPs on each network interface. (Modern browsers replace public/private IPv4s with mDNS `.local` hostnames; see [`browser-stack.md`](browser-stack.md).)
   - **Server-reflexive (srflx)** candidate — your public IP as seen by a STUN server.
   - **Peer-reflexive (prflx)** candidate — your public IP as seen by the peer during connectivity checks.
   - **Relayed (relay)** candidate — an IP:port allocated on a TURN server that will forward traffic.
2. **Exchanges candidates** with the peer through the signalling channel (trickled).
3. **Performs connectivity checks** by sending STUN binding requests over each candidate pair, finding pairs that work in both directions.
4. **Nominates** a working pair as the connection's chosen path.

The "connectivity check" is itself a STUN exchange — STUN is reused as both the address-discovery protocol *and* the connectivity-probe protocol inside ICE. This makes STUN the workhorse of WebRTC.

### ICE-Lite

A minimal ICE mode for servers that have public IPs and don't need to do candidate gathering. The server doesn't ping the client; the client does all the work. Used by some SFU (Selective Forwarding Unit) media servers — not generally useful for Myrhiza's symmetric browser-to-browser case.

## Layer 5a: STUN

[**Session Traversal Utilities for NAT**](https://datatracker.ietf.org/doc/rfc8489/) — RFC 8489 (February 2020, **obsoletes RFC 5389**). Most online references still say "STUN = RFC 5389" — that's the 2008 version. The current normative reference is RFC 8489.

STUN is two things:

1. **A reflexive-address protocol.** Client sends a `Binding Request` to `stun.example.com:3478`; server replies with an `XOR-MAPPED-ADDRESS` containing the client's IP:port as seen from outside the NAT.
2. **A connectivity-check protocol** (used inside ICE). Same wire format, used to test if a candidate pair is reachable.

STUN works over UDP (port 3478 by default), TCP (port 3478), or TLS (port 5349, "STUN over TLS"). UDP is the common case.

### Free public STUN servers

`stun.l.google.com:19302` is the canonical public STUN endpoint (free, operated by Google with no formal SLA). Cloudflare runs `stun.cloudflare.com:3478`. STUN's resource cost is trivially low (the response is a single UDP packet), so a free public STUN is operationally fine — but spec authors should not assume "free STUN is free forever."

## Layer 5b: TURN

[**Traversal Using Relays around NAT**](https://datatracker.ietf.org/doc/rfc8656/) — RFC 8656 (February 2020, **obsoletes RFC 5766 + RFC 6156**). The "if all else fails, relay everything" fallback.

A TURN server *allocates* a public IP:port pair for a client, and forwards anything sent to that pair back to the client. The client tells the peer "send to this allocation address," and the peer's packets are bounced off the TURN server.

This works for *every* NAT topology (including symmetric NATs that defeat STUN), but it has costs:

- **Bandwidth.** Every packet traverses the TURN server. If your WebRTC session pushes 1 Mbit/s, the TURN server pays for 1 Mbit/s ingress + 1 Mbit/s egress per session, per direction.
- **Latency.** Adds round-trip(s) through the TURN's geographic location.
- **Metadata.** The TURN operator sees source IP, destination IP, packet timings, sizes. Same metadata leak as iroh relays (cf. [`prior-art/iroh/critiques.md`](../iroh/critiques.md)).
- **Operational obligation.** Someone runs and pays for the TURN. No free public TURNs of consequence; sketchy "free TURN" services exist but are unreliable. Production WebRTC deployments self-host coturn, or pay a provider (Twilio Network Traversal Service, Xirsys, Cloudflare Calls).

See [`open-problems.md`](open-problems.md) §2 (TURN economics) for the operational accounting.

## Layer 6: UDP (with ICE-TCP fallback)

WebRTC primarily uses UDP. **ICE-TCP** (RFC 6544) exists as a fallback when UDP is blocked by aggressive firewalls (some corporate networks, some mobile carriers). ICE-TCP wraps the entire DTLS/SCTP stack in a TCP connection, defeating one of the main advantages of WebRTC (per-message unreliability). It's a "last resort, but better than nothing" path.

Most browsers implement ICE-TCP, but ICE-TCP usage in production is uncommon — networks that block UDP usually also block the TCP ports a TURN-TCP server would listen on. In practice, the fallback chain is: direct UDP → STUN-reflexive UDP → TURN-UDP → TURN-TCP → TURN-TLS-443. The last is the "looks like HTTPS" fallback that works behind ~all firewalls but costs the most.

## Putting it together: a connection establishment trace

What actually happens, in time-order, when two browsers connect via WebRTC:

```
t=0     [JS]   A: new RTCPeerConnection({iceServers: [STUN, TURN]})
t=0     [JS]   B: new RTCPeerConnection({iceServers: [STUN, TURN]})
t=10ms  [JS]   A: createOffer() → SDP_A
t=15ms  [signal] A → signalling → B: SDP_A
t=20ms  [browser-A] ICE gathering: query STUN for srflx, allocate from TURN, run mDNS for host
t=40ms  [signal] A → signalling → B: ICE candidate 1 (host, .local)
t=80ms  [browser-B] ICE gathering: parallel
t=90ms  [signal] A → signalling → B: ICE candidate 2 (srflx, real public)
t=100ms [JS]   B: setRemoteDescription(SDP_A); createAnswer() → SDP_B
t=110ms [signal] B → signalling → A: SDP_B
t=120ms [signal] B → signalling → A: ICE candidates...
t=200ms [ICE]  A ↔ B: connectivity checks across all candidate pairs (STUN binding requests)
t=250ms [ICE]  selected pair nominated (e.g. srflx ↔ srflx if both have STUN-reachable NATs)
t=260ms [DTLS] DTLS handshake over selected pair (1.5 RTTs)
t=400ms [SCTP] SCTP association setup (1 RTT, 4-way handshake)
t=500ms [data] first RTCDataChannel.send() bytes leave the browser
```

In ideal conditions (both peers have predictable NATs, signalling is fast, STUN responds quickly) you get from `new RTCPeerConnection` to `send()` in 300–600ms. In adversarial conditions (symmetric NATs requiring TURN, slow signalling channel, mobile network) it can take 2–5 seconds. ICE failure ends the whole exchange after a timeout (default ~30s).

This is the timing budget every browser-peer profile inherits. QUIC for comparison is 1-RTT or 0-RTT (resumed). WebRTC's setup is structurally heavier because of the SDP exchange + multi-layer handshake. **Spec authors should not assume "WebRTC is fast"** — once connected, the data channel is low-latency UDP, but the initial connect is comparable to a TLS-TCP handshake plus a TURN allocation.

## What the stack does well

- **NAT traversal in *most* cases.** Browsers can talk across home routers, mobile carriers, corporate offices, without any user configuration.
- **Encryption is mandatory.** DTLS is not optional; there is no "plain WebRTC."
- **Per-stream reliability tuning.** Unreliable-unordered + reliable-ordered on the same connection, with different lossy/lossless tradeoffs per message stream.
- **Codec-aware** (for media). Not relevant for data channels, but the ecosystem has invested heavily in real-time A/V over the same stack.

## What the stack does poorly

- **Setup latency.** 300–600ms best case, multi-second worst case.
- **SCTP-over-DTLS-over-UDP is three handshakes in series.** Modern transports (QUIC) collapse this to one.
- **Signalling is your problem.** Every deployment reinvents it; see [`signalling.md`](signalling.md).
- **20–30% of connections need TURN.** And TURN is not free; see [`open-problems.md`](open-problems.md) §2 (TURN economics).
- **The implementation is enormous.** `libwebrtc` (Google's reference) is hundreds of thousands of LOC; even the lighter stacks ([`implementations.md`](implementations.md)) are still 50k+ LOC because the stack itself is huge.

## Sources

- W3C WebRTC Recommendation (2025-03-13): <https://www.w3.org/TR/webrtc/>
- RFC 8825 — Overview: Real-Time Protocols for Browser-Based Applications: <https://www.rfc-editor.org/rfc/rfc8825.html>
- RFC 8445 — ICE: <https://datatracker.ietf.org/doc/rfc8445/>
- RFC 8489 — STUN (obsoletes RFC 5389): <https://datatracker.ietf.org/doc/rfc8489/>
- RFC 8656 — TURN (obsoletes RFC 5766): <https://datatracker.ietf.org/doc/rfc8656/>
- RFC 4960 — SCTP: <https://datatracker.ietf.org/doc/rfc4960/>
- RFC 8831 — WebRTC Data Channels (SCTP over DTLS over UDP): <https://datatracker.ietf.org/doc/rfc8831/>
- RFC 8832 — WebRTC Data Channel Establishment Protocol: <https://datatracker.ietf.org/doc/rfc8832/>
- RFC 6347 — DTLS 1.2: <https://datatracker.ietf.org/doc/rfc6347/>
- RFC 9147 — DTLS 1.3: <https://datatracker.ietf.org/doc/rfc9147/>
- RFC 8838 — Trickle ICE: <https://datatracker.ietf.org/doc/rfc8838/>
- RFC 6544 — ICE-TCP: <https://datatracker.ietf.org/doc/rfc6544/>
- Sibling: [`signalling.md`](signalling.md), [`browser-stack.md`](browser-stack.md), [`open-problems.md`](open-problems.md) §2 (TURN economics), [`webtransport.md`](webtransport.md)
