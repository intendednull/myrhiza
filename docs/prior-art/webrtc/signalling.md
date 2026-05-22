**Date:** 2026-05-22
**Status:** active
**Subject:** WebRTC signalling — how peers find each other before the WebRTC connection exists. **Load-bearing for Myrhiza.**

# Signalling

Signalling is **the unsolved-by-WebRTC problem**. The W3C spec is explicit that signalling is out of scope:

> *"WebRTC does not specify any particular signalling protocol... [the application] must take care of communicating ICE candidates and session descriptions between peers."* — W3C WebRTC Recommendation, §1

This is a feature, not a bug: WebRTC standardizes the data plane (RTCPeerConnection, ICE, DTLS, SCTP) and leaves the control plane (how do two peers exchange SDP + ICE candidates?) to the application. **That choice is the most consequential design decision a browser-P2P system makes.** Every WebRTC product has answered this differently, and the answer leaks into the product's privacy model, scaling cost, censorship resistance, and onboarding flow.

For Myrhiza's browser-peer profile, this is the single most important file in the corpus. If you ship WebRTC, you ship a signalling channel — and every signalling channel is itself a system with operators, costs, metadata leaks, and failure modes.

## The shape of the problem

Two browsers, neither of which can `listen()`, both behind NATs, need to exchange:

1. **SDP offer** (from caller) — describes local capabilities + DTLS fingerprint + initial ICE candidates.
2. **SDP answer** (from callee) — same shape, in response.
3. **ICE candidates** (both directions, ongoing as they're discovered — "trickle ICE").

These must arrive at each other within ICE's timeout window (~30s) and ideally fast (~100ms each direction for low setup latency). The signalling channel does not need to be high-bandwidth (a few KB total per session) but does need to be:

- **Reachable from inside both browsers.** No raw sockets. HTTP, WebSocket, or anything reachable through `fetch()` / `WebSocket` / `EventSource`.
- **Authenticated**, ideally end-to-end. SDP fingerprint pinning is only as good as the channel that delivered it.
- **Available** at the moment both peers want to connect. Some signalling channels (BitTorrent trackers) involve a rendezvous step.
- **Bidirectional**, eventually. Trickle ICE keeps streaming candidates after the initial offer/answer.

## Catalogue of signalling patterns

### 1. Dedicated WebSocket signalling server (canonical)

The textbook pattern. Both browsers connect via WebSocket to a server you operate; the server relays SDP + ICE messages by room/session ID.

**Examples:**
- [easyrtc](https://github.com/open-easyrtc/open-easyrtc), [SimpleWebRTC](https://github.com/SimpleWebRTC/SimpleWebRTC) (archived), every Twilio Video / LiveKit / Daily product.
- Production: Discord's voice signalling, Zoom's WebRTC fallback, Google Meet (until they pivoted to QUIC-only for some paths).

**Properties:**
- ✅ **Reliable.** WebSocket is well-understood; servers are easy to scale (it's a tiny message-passing service).
- ✅ **Low-latency.** A signalling round-trip is one server hop (sub-100ms typical).
- ✅ **Authenticated.** You authenticate users to the WebSocket; the WebSocket trusts you.
- ❌ **Operationally costly.** Someone runs and pays for the server. Server is a single point of failure.
- ❌ **Privacy.** Server sees who is dialing whom. Same metadata leak as iroh's relays. Encrypted-content does not hide the social graph.
- ❌ **Censorship surface.** The WebSocket endpoint can be blocked at the network level.

**Verdict for Myrhiza:** This is what Myrhiza would ship by default if it follows the iroh-relay pattern. It works, it scales, it has all the same trust properties as the relay. The fact that we'd be running our own signalling server is the right framing for the spec.

### 2. HTTP long-polling / Server-Sent Events

A degenerate WebSocket: server-sent-events for server-to-client, plain POST for client-to-server. Slower (latency = polling interval) but works behind proxies that block WebSocket.

**Examples:**
- Older WebRTC apps; Slack Calls historically used a hybrid HTTP-SSE + WebSocket for resilience.

**Verdict:** Mostly historical. Modern WebSocket support is universal enough that you don't need this fallback unless you're targeting hostile corporate networks. Worth keeping in the back pocket; not worth specifying as a Myrhiza default.

### 3. libp2p stream (over libp2p-relay or any libp2p transport)

The libp2p approach: signalling is *just another stream* on an existing libp2p connection. If the two peers already have a libp2p connection (typically through a libp2p relay node), they can negotiate WebRTC by opening a `/webrtc-signalling/1.0.0` stream and exchanging SDP over it.

**Examples:**
- [libp2p-webrtc spec (browser-to-browser)](https://github.com/libp2p/specs/blob/master/webrtc/webrtc.md) — the most relevant external spec for what Myrhiza-over-WebRTC would look like.
- js-libp2p production deployments (IPFS, Filecoin clients, some game-multiplayer projects).

**Properties:**
- ✅ **Elegant layering.** Signalling is *not* a new system; it's a stream on the existing transport graph.
- ✅ **Encrypted, authenticated.** Inherits libp2p's Noise/TLS handshake.
- ✅ **No new signalling-server class** — relay nodes (which exist anyway) are the rendezvous.
- ❌ **Requires existing libp2p connectivity.** If you don't have a libp2p relay path between A and B, you can't signal. So the relay isn't optional, it's mandatory for the first session.
- ❌ **Relay sees who-talks-to-whom.** Same metadata leak; this is libp2p's universal "relay knows the social graph" problem.

**Verdict for Myrhiza:** If Myrhiza adopts iroh-as-substrate, the iroh-relay can serve the same function — an iroh stream is the signalling channel, the relay is the rendezvous, the same metadata leak applies. This is the **single closest pre-existing design pattern** to what Myrhiza would build. The libp2p spec is worth reading line-by-line; see [`libp2p-webrtc.md`](libp2p-webrtc.md).

### 4. Manual copy-paste (the "out-of-band" pattern)

Caller creates offer, JSON-encodes it, sends to callee via email/Slack/QR-code. Callee creates answer, sends back the same way. Trickle ICE is folded into the initial blob (no streaming).

**Examples:**
- WebTorrent's manual-trade fallback. <https://instant.io/> for browser-only file transfer.
- The "send a magnet link, it works" UX.

**Properties:**
- ✅ **No server.** Genuinely zero infrastructure.
- ✅ **No metadata leak.** Whatever channel you use is your problem.
- ❌ **Terrible UX.** Long blobs of base64-encoded JSON.
- ❌ **Static.** No trickle ICE without resending; if the initial candidates fail, the whole session fails.
- ❌ **Only one-shot.** Reconnects require a fresh exchange.

**Verdict:** Useful as a fallback or for one-off "share file with this stranger" UX. Not a primary signalling channel for Myrhiza.

### 5. DHT signalling (trystero / WebTorrent / BitTorrent trackers)

The most interesting "decentralized" approach. Peers exchange SDP via a *third-party network they don't own*.

[**trystero**](https://github.com/dmotz/trystero) implements this with seven distinct backends:

| Backend | Mechanism | What's the third-party? |
|---|---|---|
| `bittorrent` | BitTorrent trackers — peers announce on a topic-derived infohash, see each other in the swarm | Public BitTorrent tracker fleet (no central authority) |
| `nostr` | Send SDP as Nostr events to a topic | Nostr relay fleet (federated) |
| `mqtt` | Public MQTT broker as message bus | Public MQTT broker (e.g. test.mosquitto.org) |
| `supabase` | Supabase realtime (PostgreSQL listen/notify under the hood) | Supabase as backend |
| `firebase` | Firebase realtime database | Google |
| `ipfs` | IPFS pubsub on a topic | js-ipfs / libp2p pubsub |
| `websocket` | Self-hosted WebSocket relay (back to pattern 1) | Whoever runs the relay |

The genius of trystero: **the application code is identical across all seven backends.** Same JS API; swap the import.

**Examples:**
- Tone.land, Cardgame.io, many small multiplayer games. Trystero has ~2.6k stars but the apps using it tend to be small.
- WebTorrent (similar pattern, BitTorrent-tracker-specific).

**Properties:**
- ✅ **No first-party signalling server.** You inherit a third-party network's properties.
- ✅ **Censorship-resistant** (depends on backend; BitTorrent + Nostr are quite resistant).
- ✅ **Free** (at small scale; large-scale use of public infrastructure is anti-social).
- ❌ **Metadata visible to the third-party.** A public MQTT broker sees your topic-name + payload-size + timing. A BitTorrent tracker sees who announces a topic.
- ❌ **Variable reliability.** Public trackers go down, MQTT brokers rate-limit, Nostr relays disconnect. You depend on someone else's uptime.
- ❌ **Hash-based topic discovery.** You and the peer must derive the same topic hash. Trystero hashes a string + secret; if anyone else hashes the same string, they're in your room. Use real secrets.

**Verdict for Myrhiza:** Trystero is the **single most interesting prior art for "P2P apps without our own signalling server."** If Myrhiza had a no-Myrhiza-infra mode for ad-hoc apps, the trystero pattern is the template. But: the metadata leak is real, and the reliability of any specific third-party backend is not guaranteed. Reading the trystero source as a study of "what would a backend-pluggable Myrhiza signalling layer look like" is high-value work. See [`signalling.md`](signalling.md) §"JS-ecosystem patterns".

### 6. PeerJS-style "broker" servers (cloud signalling)

A specific incarnation of pattern 1: PeerJS ships its own signalling broker protocol (`PeerServer`), with a public free deployment at `0.peerjs.com` and self-hostable.

**Verdict:** Functionally equivalent to pattern 1; useful as a low-friction-onboarding choice when you don't want to write your own signalling. Worth knowing the project exists; not architecturally distinctive.

### 7. STUN-as-signalling (theoretical, not used in practice)

Could STUN itself carry the SDP? Technically the STUN protocol has extension attributes; in principle a STUN-with-extensions could carry SDP blobs. Nobody does this — STUN servers don't keep state across requests, so the "relay an SDP to peer B" semantics don't fit.

## The metadata-leak hierarchy

Different signalling patterns leak different things:

| Pattern | Server sees who | Server sees when | Server sees content | Censorship surface |
|---|---|---|---|---|
| Dedicated WebSocket | yes (account) | yes | depends on E2E encryption | central |
| libp2p / iroh stream | relay sees NodeIDs | yes | encrypted | per-relay |
| Manual copy-paste | nobody | nobody | nobody | none |
| BitTorrent tracker | infohash + IP | yes | encrypted (payload is in extension) | per-tracker |
| Nostr relay | npub + tags | yes | encrypted | per-relay |
| Public MQTT | topic + IP | yes | depends on encryption | per-broker |
| Firebase/Supabase | account + project | yes | depends on encryption | central |
| PeerJS broker | peer ID + IP | yes | content (PeerJS is plaintext over wss) | central |

For Myrhiza, the question is which metadata leak is acceptable. Iroh's relays already see who-talks-to-whom (cf. [`prior-art/iroh/critiques.md`](../iroh/critiques.md)); adding WebRTC signalling on top of iroh's relays does not increase the leak. Adding a WebRTC signalling layer on top of *trystero* would change the leak shape to "third-party network sees topic hashes."

## Authentication

WebRTC's DTLS-fingerprint trust model assumes the signalling channel delivers the SDP correctly. If the signalling channel is MITM'd, WebRTC is MITM'd: an attacker rewrites the fingerprint in the SDP, intercepts traffic, and re-encrypts to each side. There is no "Certificate Transparency" for WebRTC fingerprints.

The practical implication: **WebRTC inherits the authentication of its signalling channel.**

- WebSocket signalling over `wss://` with authenticated users → MITM only by the signalling-server operator (or someone who compromises it).
- libp2p stream over Noise → MITM requires breaking Noise, which means breaking libp2p's transport.
- Manual copy-paste over Signal → MITM requires breaking Signal.
- BitTorrent tracker → SDPs are *not* end-to-end authenticated by the tracker. Anyone in the swarm sees the SDP. trystero papers over this by hashing in a shared secret, but trystero's threat model is "discover peers who know the same secret," not "authenticate peers with cryptographic identity."

For Myrhiza, the implication is clear: **the Myrhiza identity must sign the SDP fingerprint**, and the signalling channel becomes a transport-not-trust layer. Even if Myrhiza uses public trackers for rendezvous, the SDP fingerprint must be verifiable against the peer's Myrhiza identity (event-log keys, Ed25519). This is *additional work* WebRTC does not do for you.

## What the libp2p-webrtc spec gets right

The libp2p-webrtc browser-to-browser spec ([`webrtc/webrtc.md`](https://github.com/libp2p/specs/blob/master/webrtc/webrtc.md), CR 2023-04-12) makes one specific design choice that Myrhiza should copy: **the SDP is generated deterministically from libp2p identities** at both ends. Both peers know each other's Ed25519 PeerIds; the SDP fingerprint is bound to the peer's libp2p identity via the existing libp2p TLS spec applied to WebRTC.

In practice: the signalling channel only carries the *non-key-derived* parts of the SDP (ICE candidates, codec preferences). The fingerprint is implicitly authenticated by the peer's libp2p identity. A signalling MITM cannot rewrite the fingerprint without also breaking libp2p's identity layer.

This is the right pattern for Myrhiza. The Myrhiza identity becomes the trust anchor; the signalling channel is just transport.

## Implications for Myrhiza

1. **Pick the signalling pattern before specifying the browser-peer profile.** It is the load-bearing decision; everything else follows. The default candidates:
   - **iroh-stream-as-signalling** (mirror of libp2p-webrtc but over iroh's transport graph). Requires iroh kernel to broker. Inherits iroh's relay metadata-leak profile.
   - **dedicated WebSocket service operated by Myrhiza** (parallel to iroh's relays). Simpler operationally; adds a new system.
   - **trystero-style pluggable backend.** Maximum flexibility, third-party metadata leaks, optional for ad-hoc apps.

2. **Authenticate the SDP fingerprint with the Myrhiza identity.** Don't trust signalling-channel-as-identity. Sign the SDP with the Myrhiza Ed25519 keypair; verify on receipt.

3. **Use trickle ICE.** Don't ship the offer/answer with all candidates pre-gathered; the latency penalty is real. The signalling channel must be a stream, not a one-shot.

4. **Plan for the signalling channel to be down sometimes.** Browser-peer reconnect logic needs a fallback. The "user reopens the app, peer is offline" path has to work. (Trystero's multi-backend pattern is the cleanest answer here: try one, fall back to another.)

5. **The signalling channel is your social-graph metadata.** Whoever operates it (Myrhiza, iroh, BitTorrent trackers, Nostr relays) knows who talks to whom. Accept that, or design a signalling layer that uses mix-network or onion-routing primitives (a real research problem, no off-the-shelf answer).

## Sources

- W3C WebRTC Recommendation §1 (signalling out of scope): <https://www.w3.org/TR/webrtc/>
- libp2p WebRTC browser-to-browser spec (r1, 2023-04-12): <https://github.com/libp2p/specs/blob/master/webrtc/webrtc.md>
- libp2p WebRTC-Direct spec (browser-to-server, 2023-04-12): <https://github.com/libp2p/specs/blob/master/webrtc/webrtc-direct.md>
- trystero (serverless WebRTC, 7 backends): <https://github.com/dmotz/trystero>
- trystero npm package (v0.24.0, 2026-04-27): <https://registry.npmjs.org/trystero>
- PeerJS (broker-based signalling): <https://github.com/peers/peerjs-server>
- RFC 8838 — Trickle ICE: <https://datatracker.ietf.org/doc/rfc8838/>
- RFC 8866 — SDP: <https://datatracker.ietf.org/doc/rfc8866/>
- Cross-refs: [`stack.md`](stack.md), [`libp2p-webrtc.md`](libp2p-webrtc.md), [`signalling.md`](signalling.md) §"JS-ecosystem patterns", [`open-problems.md`](open-problems.md), [`prior-art/iroh/critiques.md`](../iroh/critiques.md)
