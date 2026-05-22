**Date:** 2026-05-22
**Status:** active
**Subject:** WebTransport — HTTP/3 / QUIC-based browser transport. **Complementary to WebRTC, not a replacement.**

# WebTransport

WebTransport is the W3C browser API for sending data over HTTP/3 (QUIC). It is **not a WebRTC replacement** — a common confusion that this folder must clear up explicitly.

| Property | WebRTC | WebTransport |
|---|---|---|
| Topology | **Peer-to-peer** (any-to-any) | **Client-to-server only** |
| Underlying transport | DTLS-SCTP over UDP | HTTP/3 over QUIC over UDP |
| NAT traversal | ICE + STUN + TURN | None (server has public IP) |
| Encryption | DTLS 1.2/1.3 | TLS 1.3 (via QUIC) |
| Stream model | SCTP streams (multiple per data channel) | QUIC streams (native to transport) |
| Datagrams | not really (unreliable channel approximates) | yes (first-class unreliable datagrams) |
| Setup latency | 300–600ms (SDP + ICE + DTLS + SCTP) | 0–1 RTT (QUIC handshake) |
| Browser support | universal (95.94%) | uneven (~80%; Safari shipped 26.4) |
| Cert model | self-signed + fingerprint | CA-signed *or* certhash-pinned for HTTP/3 |
| Signalling required | yes (out-of-band) | no (URL is the address) |

**WebTransport is QUIC-with-a-browser-API for client-to-server.** WebRTC is the peer-to-peer stack. They share UDP and TLS-1.3-ish cryptography; everything else is different.

## When to use WebTransport (Myrhiza-relevant cases)

1. **Browser ↔ Myrhiza-kernel-on-public-server.** If the Myrhiza topology includes browsers connecting to a Myrhiza node that has a public IP (a relay, a bootstrap server, an aggregator), WebTransport is dramatically simpler than WebRTC. No signalling, no ICE, lower setup latency.
2. **Pubsub fanout.** A Myrhiza node could expose a WebTransport endpoint that streams event-log updates to subscribed browsers. Modern QUIC streams handle this cleanly.
3. **Server-side gateway.** Some Myrhiza apps may need a "browse mode" — read-only access from a browser to a Myrhiza node, no peer-to-peer. WebTransport is the right transport.

## When WebTransport doesn't work

1. **Browser ↔ browser.** Cannot. WebTransport is client-only in the browser; the browser is never a WebTransport *server*. For peer-to-peer between browsers, you need WebRTC.
2. **Behind aggressive firewalls that block UDP.** QUIC needs UDP; WebTransport inherits that. WebSocket-over-TLS (wss/443) still works where WebTransport doesn't.
3. **In Safari before 26.4.** Safari shipped WebTransport in v26.4 (very recent as of 2026-05). iOS Safari support is even newer. Apps targeting older Safari users must use WebRTC or WebSocket.

## Browser support state

Per [caniuse webtransport](https://caniuse.com/webtransport), as of 2026-05:

| Browser | Version | Status |
|---|---|---|
| Chrome | 97+ (2022-01) | shipped |
| Edge | 98+ (2022-02) | shipped |
| Firefox | 114+ (2023-06) | shipped |
| Safari | **26.4+ (2026-Q2)** | shipped, **very recent** |
| iOS Safari | 26.4-26.5 | shipped, **very recent** |
| Samsung Internet | 18.0+ | shipped |

**Global coverage ~80%**. The Safari lag is the historical story; it has finally been resolved, but app support that assumes Safari WebTransport is brand-new.

## Architecture

WebTransport is HTTP/3 with extensions:

- **HTTP/3 over QUIC** is the underlying protocol. RFC 9114 + RFC 9000.
- **Extended CONNECT method** (RFC 9220) lets the browser issue a `CONNECT-LIKE` HTTP/3 request to upgrade the connection to WebTransport.
- After upgrade, the browser exposes:
  - **Bidirectional streams** (`createBidirectionalStream()`) — like WebSocket but multiplexed and per-stream-isolated.
  - **Unidirectional streams** (`createUnidirectionalStream()`) — send-only, low-overhead.
  - **Datagrams** (`writable`/`readable` on the `datagrams` property) — unreliable, unordered.

The API is *much* cleaner than RTCDataChannel:

```js
const transport = new WebTransport('https://example.com:4433/');
await transport.ready;

// Bidirectional stream
const stream = await transport.createBidirectionalStream();
const writer = stream.writable.getWriter();
writer.write(new Uint8Array([1, 2, 3]));

// Datagrams (unreliable)
const dgramWriter = transport.datagrams.writable.getWriter();
dgramWriter.write(new Uint8Array([4, 5, 6]));
```

No SDP, no ICE, no DTLS fingerprint juggling. Just a URL.

## `serverCertificateHashes` — the certhash trick

Vanilla HTTPS requires a CA-signed certificate. For Myrhiza-shape deployments (self-hosted nodes that don't have CA certs), this would be the same blocker as `wss://`.

WebTransport solves this with **`serverCertificateHashes`**, a `WebTransportOptions` property:

```js
const transport = new WebTransport('https://my-self-signed-server.example:4433/', {
  serverCertificateHashes: [{
    algorithm: 'sha-256',
    value: new Uint8Array([0xab, 0xcd, /*...*/])
  }]
});
```

The browser:
1. Connects to the URL.
2. Receives the server's TLS cert (which may be self-signed).
3. Computes SHA-256 of the cert's `SubjectPublicKeyInfo`.
4. Verifies it matches one of the pinned hashes.
5. Accepts the connection if so.

The trade-off: pinned certs must have **a maximum validity of 14 days** (per the WebTransport spec, to limit the risk of stolen-key reuse). The server must rotate.

This is exactly the libp2p-webrtc-direct pattern (cf. [`libp2p-webrtc.md`](libp2p-webrtc.md) and [`prior-art/libp2p/transports.md`](../libp2p/transports.md)). Both have invented "pin a hash of the cert, no CA needed" because the CA model doesn't work for P2P.

### Limitations of `serverCertificateHashes`

- **Only `localhost` and IPv4/IPv6 literals + 14-day-cert hostnames** (Chrome impl detail). You cannot use it with a publicly-CA-resolved hostname; in that case, just use a normal CA cert.
- **Cert rotation every 14 days.** Servers must implement automatic rotation. clients must be re-told the new hash out-of-band.
- **Only one cert at a time pinned per `WebTransport`.** Re-pin on rotation.

For Myrhiza: certhash + 14-day rotation is a known pattern; the bookkeeping is manageable but real.

## Comparison: WebRTC-Direct vs WebTransport-with-certhash

Both solve the "browser to non-CA-trusted server" problem. They are *the same idea* implemented in two different transports.

| | WebRTC-Direct | WebTransport-with-certhash |
|---|---|---|
| Spec home | libp2p (no IETF) | W3C / IETF |
| Browser support | ~95% (RTCPeerConnection) | ~80% (still rolling out in Safari) |
| Underlying protocol | DTLS-SCTP over UDP | QUIC |
| Setup latency | medium (SDP-less but ICE-light) | low (0–1 RTT) |
| Cert validity cap | longer (no hard cap) | 14 days |
| Server-side stack | libp2p-rust / libp2p-go SFU-shape | HTTP/3 server with WebTransport extension |
| Browser-to-browser? | No (server-only) | No (server-only) |

For Myrhiza, **WebTransport-with-certhash is the cleaner long-term answer for browser-to-Myrhiza-node connections**. Once Safari adoption settles (within a year or two), it should dominate. WebRTC remains necessary only for the *browser-to-browser* case.

## The split-stack browser-peer architecture

The takeaway for the Myrhiza spec:

```
                      Browser peer
                     ┌──────────────┐
                     │              │
                     │  app code    │
                     │              │
                     ├──────────────┤
                     │  Myrhiza     │
                     │  browser     │
                     │  kernel      │
                     ├─┬──────────┬─┤
                     │ │ WebRTC   │ │   ←── browser ↔ browser P2P
                     │ │ DataChan │ │
                     │ ├──────────┤ │
                     │ │ WebTrans │ │   ←── browser → Myrhiza node (signalling + bulk transfer)
                     │ ├──────────┤ │
                     │ │ WebSocket│ │   ←── fallback when WebTransport not available
                     │ └──────────┘ │
                     └──────────────┘
```

Three transports, each for its own purpose:

1. **WebRTC** — browser-to-browser peer connections. Mandatory for true P2P.
2. **WebTransport** — browser-to-Myrhiza-node. Carries signalling for WebRTC + bulk data when the node is the destination. Should be preferred over WebSocket where supported.
3. **WebSocket** — fallback for old Safari, for blocked-UDP networks. Same wire format as WebTransport for forward-compat.

The "browser-peer profile" spec should accommodate all three; **the client picks**, the spec defines the negotiation. This is more complex than "WebRTC everywhere" but it is honest about browser variability.

## Production users of WebTransport

- **Google Stadia** (deprecated as a product, but the streaming protocol used WebTransport). Notable for surfacing latency limits.
- **Cloud Gaming services** generally — Xbox Cloud, GeForce Now have explored WebTransport.
- **Twitch low-latency streaming** experiments.
- **WebRTC-Direct alternatives** in some libp2p deployments.

Honest scale: WebTransport is **early-adopter territory**. The biggest deployments are gaming and streaming, not generic P2P apps. It is *available* in browsers but not *common* in production apps yet.

## Implications for Myrhiza

1. **WebTransport solves the easier problem (browser-to-server) better than WebRTC.** When the Myrhiza node is a server, prefer WebTransport.
2. **WebRTC remains necessary for browser-to-browser.** No browser-to-browser WebTransport exists; the spec has no path to add it.
3. **certhash + 14-day rotation is the right pattern for Myrhiza's self-hosted nodes serving browsers.** Implement once, reuse for any browser-to-self-hosted connection.
4. **Safari support is fresh.** Apps targeting current Safari users need testing. Apps targeting 1-year-old Safari users need WebSocket fallback.
5. **WebTransport's API ergonomics are dramatically better than WebRTC's.** If the Myrhiza spec can offload work to WebTransport instead of WebRTC, the spec is simpler.

## Sources

- W3C WebTransport spec: <https://www.w3.org/TR/webtransport/>
- IETF WebTransport over HTTP/3: <https://datatracker.ietf.org/doc/draft-ietf-webtrans-http3/>
- RFC 9220 — Bootstrapping WebSockets with HTTP/3: <https://datatracker.ietf.org/doc/rfc9220/>
- RFC 9000 — QUIC v1: <https://datatracker.ietf.org/doc/rfc9000/>
- caniuse webtransport: <https://caniuse.com/webtransport>
- Chrome WebTransport API explainer: <https://developer.chrome.com/docs/web-platform/webtransport>
- W3C WebTransport API on MDN: <https://developer.mozilla.org/en-US/docs/Web/API/WebTransport>
- libp2p WebTransport spec: <https://github.com/libp2p/specs/blob/master/webtransport/README.md>
- Cross-refs: [`stack.md`](stack.md), [`libp2p-webrtc.md`](libp2p-webrtc.md), [`prior-art/libp2p/transports.md`](../libp2p/transports.md), [`prior-art/iroh/transports.md`](../iroh/transports.md)
