**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p-webrtc — libp2p's two WebRTC transport profiles (closest external prior art for Myrhiza-over-WebRTC)

# libp2p-webrtc

The libp2p ecosystem has invested years in making WebRTC a first-class transport. Two distinct specs, both Candidate-Recommendation, dated 2023-04-12:

1. **WebRTC (browser ↔ browser)** — [`libp2p/specs/webrtc/webrtc.md`](https://github.com/libp2p/specs/blob/master/webrtc/webrtc.md). Browsers connect to other browsers, signalling rides on existing libp2p streams (typically through a libp2p relay).
2. **WebRTC-Direct (browser ↔ server)** — [`libp2p/specs/webrtc/webrtc-direct.md`](https://github.com/libp2p/specs/blob/master/webrtc/webrtc-direct.md). Browser connects to a public-IP server *without* requiring the server to have a CA-trusted TLS cert. The trust anchor is a `certhash` pinned in the multiaddr.

Plus the **archived** earlier attempt:

3. **WebRTC-Star (deprecated)** — [`libp2p/js-libp2p-webrtc-star`](https://github.com/libp2p/js-libp2p-webrtc-star), **archived 2024-09**. Centralized signalling-server pattern; superseded by signalling-over-libp2p-streams.

This is **the most directly relevant external prior art** for the Myrhiza browser-peer profile. Reading these specs end-to-end is high-value time for anyone designing Myrhiza's browser-peer.

## WebRTC (browser ↔ browser) — Candidate Recommendation, 2023-04-12

The motivation from the spec, verbatim:

> *"libp2p transport protocol enabling two private nodes (e.g. two browsers) to establish a direct connection. Browser A wants to connect to Browser node B with the help of server node R. Both A and B cannot listen for incoming connections due to running in a constrained environment (i.e. a browser) with its only transport capability being the W3C WebRTC `RTCPeerConnection` API and being behind a NAT and/or firewall."*

The design:

1. **Both browsers maintain a libp2p connection to a *relay* node R.** R has a public IP and can be reached over WebSocket-Secure, WebTransport, or WebRTC-Direct.
2. **Caller (A) opens a `/webrtc-signaling/0.0.1` stream to R**, targeted at B's PeerId.
3. R proxies the stream to B (assuming B is also connected to R).
4. **A and B exchange SDP offer/answer + ICE candidates** over this stream.
5. The browsers' `RTCPeerConnection` establishes a *direct* DTLS-SCTP-over-UDP connection (or via TURN if direct fails).
6. The libp2p connection is then "upgraded" — subsequent libp2p streams flow over the WebRTC data channel directly, not through R.

**Why this is elegant:**

- The signalling channel is *not a special new system*. It's a libp2p stream over the existing libp2p transport graph.
- Authentication of the SDP is *inherited from libp2p's identity layer*. Both peers already authenticated each other via Noise/TLS when they connected to R; SDP fingerprints can be tied to PeerIds.
- The relay R is "load-bearing for connection setup, then optional once the WebRTC is up." This matches iroh's relay model exactly.

**Authentication detail (the load-bearing trick):**

Per the libp2p-tls spec, every libp2p peer has a self-signed TLS cert containing its PeerId as an extension. For WebRTC, the libp2p-webrtc spec specifies that the DTLS cert's SHA-256 fingerprint is the peer's PeerId-derived identity. **The SDP fingerprint is implicitly authenticated by the peer's libp2p identity.** A signalling MITM can't rewrite the fingerprint without breaking libp2p's TLS layer.

This is the design pattern Myrhiza should copy directly. The Myrhiza Ed25519 identity becomes the trust anchor for WebRTC; the signalling channel is just a transport.

**Multiaddr format:**

```
/ip4/$IP/udp/$PORT/quic-v1/p2p/$RELAY_PEER_ID/p2p-circuit/webrtc/p2p/$TARGET_PEER_ID
```

The `/p2p-circuit/webrtc/p2p/...` suffix says "via this relay, establish a WebRTC connection to this peer." The browser dialer parses this, opens the signalling stream, and lets WebRTC do its thing.

**Implementation status:**

| Implementation | Crate / package | Status |
|---|---|---|
| js-libp2p | `@libp2p/webrtc` | Active, production-ready |
| rust-libp2p (browser side) | `libp2p-webrtc-websys` v0.5.0 | Active, master only |
| rust-libp2p (server side) | `libp2p-webrtc` v0.9.0-alpha.1 on crates.io (last 2025-06-27); 0.10.0-alpha on master | **Stuck in alpha for years**; production rare in Rust |
| go-libp2p | `p2p/transport/webrtc` | Active, production |

Per [`prior-art/libp2p/transports.md`](../libp2p/transports.md): *"`libp2p-webrtc` has been stuck in alpha for years (the crates.io version was last published 2025-06-27 still labeled `0.9.0-alpha.1`). The native Rust WebRTC stack is genuinely hard to ship — str0m + webrtc-rs are heavy dependencies — and the rust-libp2p team has openly flagged this as a maintenance burden."*

**Honest scale:** Real apps using libp2p-webrtc browser-to-browser exist (some Filecoin clients, some IPFS-adjacent things, some game multiplayer prototypes), but the production volume is **small compared to libp2p over TCP/QUIC**. The browser-to-browser path is real but niche.

## WebRTC-Direct (browser ↔ server) — Candidate Recommendation, 2023-04-12

Motivation, verbatim:

> *"No need for trusted TLS certificates. Enable browsers to connect to public server nodes without those server nodes providing a TLS certificate within the browser's trustchain. Note that we can not do this today with our Websocket transport as the browser requires the remote to have a trusted TLS certificate."*

The design:

1. **The server has a public IP and listens on UDP.**
2. **The server has a self-signed TLS cert.** Its SHA-256 fingerprint is the `certhash`.
3. **The multiaddr advertises the certhash:**
   ```
   /ip4/$IP/udp/$PORT/webrtc-direct/certhash/$HASH/p2p/$PEER_ID
   ```
4. **Browser dials this multiaddr.** Internally, it constructs an `RTCPeerConnection`, generates SDP with the server's certhash as expected fingerprint, and initiates ICE with a single candidate (the server's public address — no STUN needed).
5. DTLS handshake verifies the server's cert against the certhash.
6. Connection up; libp2p streams flow over the WebRTC data channel.

**Why this is interesting:**

- **No signalling server.** The multiaddr *is* the address. The browser knows where to dial and what cert to expect.
- **No STUN/TURN.** The server is public; no NAT traversal.
- **No persistent relay.** Once the multiaddr is shared, the browser can connect any time the server is up.
- **Cert rotation:** the server can rotate; clients must re-learn the certhash out-of-band (e.g. via `identify` on an existing connection, or by re-fetching a bootstrap multiaddr list).

**Comparison with WebTransport-with-certhash:**

These solve the same problem with different transports. See [`webtransport.md`](webtransport.md) for the side-by-side. Briefly:

- **WebRTC-Direct** uses DTLS over UDP (same as WebRTC); supported in older browsers; rust-libp2p has shipped this.
- **WebTransport-with-certhash** uses QUIC; better browser support trajectory; lower setup latency; W3C-standardized.

For new designs, **WebTransport-with-certhash is the cleaner choice**. WebRTC-Direct is "WebTransport before WebTransport was widely available."

**Multiaddr format detail (the load-bearing piece for Myrhiza):**

```
/ip4/198.51.100.42/udp/4001/webrtc-direct/certhash/uEiCsmkXQLfL_3Z4yI3o7VBQ5MzLZ/p2p/12D3KooW...
```

- `ip4 / udp / port` — the network address.
- `webrtc-direct` — protocol identifier.
- `certhash/uEi...` — multihash-base64-encoded SHA-256 of the cert. Pinned in the address itself.
- `p2p/12D3...` — the server's libp2p PeerId.

This is **one-shot dial**: the browser can dial the server without any prior setup, given the multiaddr. The multiaddr is shareable, copy-pasteable, stable over the cert's lifetime.

For Myrhiza, this format is worth studying: if Myrhiza nodes ever expose direct-dial multiaddrs, this is the pattern.

## What changed in browser implementations

The libp2p-webrtc browser-to-browser spec is a year older than the browser-to-browser implementations got stable. Browser implementations evolved:

- **2019-2021:** WebRTC-Star (centralized signalling) was the only option. js-libp2p shipped it. It was always considered a stopgap.
- **2022:** WebRTC-Direct spec stabilizes; browser-to-server starts working in production.
- **2023:** WebRTC browser-to-browser spec (with libp2p-stream-signalling) reaches Candidate Recommendation. js-libp2p ships.
- **2024:** WebRTC-Star formally deprecated; archived.

The progression matters because it shows the design trajectory: **centralized signalling → no signalling (when possible) → in-band signalling (when not)**. This is the same trajectory a Myrhiza-over-WebRTC design would follow.

## What the libp2p-webrtc design gets right (and Myrhiza should copy)

1. **Signalling channel = transport stream.** Don't invent a new signalling system; reuse the existing one. For Myrhiza, this means: signalling rides on iroh streams (or Myrhiza's own primary transport), not on a separate WebSocket service.

2. **SDP fingerprint authentication tied to peer identity.** The DTLS fingerprint *is* the peer's identity-derived value. A MITM can't rewrite without breaking the identity layer.

3. **Multiaddr format encodes everything needed to dial.** No "look up the server's address in a database" step. The multiaddr is the dial string.

4. **certhash pinning for self-hosted nodes.** The pattern shipped, has been hardened, has been deployed. Myrhiza can copy it.

5. **Relay as rendezvous, not relay as forwarder.** The libp2p relay serves only to set up the WebRTC connection; once WebRTC is up, traffic goes peer-to-peer. The relay sees less than a "true relay" would. (Though it still sees who dialed whom — same metadata leak.)

## What the libp2p-webrtc design gets wrong (and Myrhiza should avoid)

1. **The Rust server-side implementation has been stuck in alpha for years.** This is not the spec's fault but it is a real signal that *implementing* WebRTC server-side in Rust is hard. Myrhiza should plan for this: budget the engineering work realistically, or pick a different language (Go via pion) for the server-side WebRTC piece.

2. **WebRTC-Star existed as long as it did.** The centralized-signalling stopgap shipped for years before in-band signalling was workable. Myrhiza should not ship a stopgap that we plan to deprecate — pick the right pattern up front.

3. **multistream-select on the WebRTC datachannel.** libp2p protocols negotiate via multistream-select, which adds an extra RTT after the WebRTC connection is up. Myrhiza shouldn't replicate this — pick one wire format per channel and stick with it.

4. **No published per-connection benchmarks.** libp2p as a whole, and libp2p-webrtc specifically, doesn't publish "this is how much latency, this is how much bandwidth, this is how many concurrent sessions per server" data. Spec authors have to benchmark themselves. Same problem iroh has.

## Implications for Myrhiza

1. **The libp2p-webrtc browser-to-browser spec is the closest published external design to what Myrhiza-over-WebRTC would look like.** Read it line by line. Don't paraphrase.

2. **The certhash pattern is the right answer for Myrhiza nodes serving browsers.** Whether the underlying transport is WebRTC-Direct or WebTransport, certhash + 14-day rotation is the model.

3. **The Rust ecosystem's WebRTC story is weaker than the Go ecosystem's.** This is a load-bearing fact for Myrhiza. If we need server-side WebRTC and we want Rust, we likely want str0m + custom integration (cf. [`implementations.md`](implementations.md)) rather than waiting on libp2p-webrtc to leave alpha.

4. **The spec's "signalling rides on existing stream" pattern matches iroh's relay model.** Myrhiza-over-iroh inherits this directly; the signalling stream is just another iroh stream over the iroh relay.

5. **Per-connection metadata leaks at the relay are real.** libp2p doesn't solve this; Myrhiza won't either. Document the leak honestly, design around it where possible (e.g. private relays per app, ephemeral PeerIds), but don't claim it's solved.

## Sources

- libp2p WebRTC browser-to-browser spec (CR, 2023-04-12): <https://github.com/libp2p/specs/blob/master/webrtc/webrtc.md>
- libp2p WebRTC-Direct browser-to-server spec (CR, 2023-04-12): <https://github.com/libp2p/specs/blob/master/webrtc/webrtc-direct.md>
- libp2p WebRTC spec README: <https://github.com/libp2p/specs/tree/master/webrtc>
- libp2p-webrtc-star (archived 2024-09): <https://github.com/libp2p/js-libp2p-webrtc-star>
- libp2p TLS spec (identity-bound certs): <https://github.com/libp2p/specs/tree/master/tls>
- `libp2p-webrtc` crate (Rust, server, stuck in alpha): <https://crates.io/crates/libp2p-webrtc>
- `libp2p-webrtc-websys` (Rust, browser): <https://crates.io/crates/libp2p-webrtc-websys>
- js-libp2p `@libp2p/webrtc`: <https://github.com/libp2p/js-libp2p/tree/main/packages/transport-webrtc>
- Cross-refs: [`stack.md`](stack.md), [`signalling.md`](signalling.md), [`webtransport.md`](webtransport.md), [`implementations.md`](implementations.md), [`prior-art/libp2p/transports.md`](../libp2p/transports.md)
