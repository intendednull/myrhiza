**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — apps shipping in production

The headline number n0 has cited consistently in 2025–2026: "half a million unique nodes hitting the public network in a thirty day span," roughly doubling when private-relay-running partners are counted, with at least 40 projects building on iroh ([Iroh 1.0 roadmap](https://www.iroh.computer/blog/road-to-1-0)). Verifiable production deployments are concentrated in a smaller set; below is what I could confirm against the apps' own sources.

| App | What iroh does | Status May 2026 | Distinction |
|---|---|---|---|
| **Delta Chat** ([delta.chat](https://delta.chat/)) | P2P realtime channels for webxdc apps; multi-device setup transfer | 1.48+ shipping on Android, iOS, desktop. n0 cites "hundreds of thousands of devices" | Uses iroh — base messaging is still email-over-SMTP |
| **Spacedrive** ([spacedrive.com](https://www.spacedrive.com/)) | Direct device-to-device library sync, NAT traversal, mDNS local discovery, relay fallback | v2 alpha shipped 2025; v3 "local-first data engine" mid-2026 | Iroh-native — replaces a centralized cloud sync entirely |
| **Holochain** (≥ 0.6.1) | Default network transport, replacing tx5/WebRTC for hole-punching | Shipping in 0.6.1+ (released late 2025); 0.7 dev versions emphasize iroh | Uses iroh as transport — Holochain's own application/DHT layer sits on top |
| **Dumbpipe** ([dumbpipe.dev](https://www.dumbpipe.dev/)) | The whole product — encrypted netcat-by-pubkey | Shipping; CLI tool, ~200-line wrapper around the iroh Rust crate | Iroh-native — n0's own demo |
| **Paycode** | Connecting payment terminals to point-of-sale at highway toll booths "with no additional servers" | Shipping; cited in [iroh for payments post](https://www.iroh.computer/blog/iroh-for-payments) (Mar 2026) | Uses iroh — narrow industrial deployment |

I did **not** verify a project named "Quary" using iroh; the prompt suggested it but searches turned up nothing. n0's homepage as fetched May 2026 lists customers Spacedrive, Nous (Distributed AI), Shaga, Paycode, Rave (Video Streaming), Delta Chat, and Holochain. Of those, Nous, Shaga, and Rave have public-facing material thinner than the others — they're real but smaller, and I have not separately verified scale. The "40+ projects" figure is real but mostly small.

## Per-app detail

### Delta Chat — the largest verified deployment

After "nearly two years of collaboration with the Iroh team," Delta Chat 1.48 shipped iroh-backed P2P networking on November 20, 2024 ([realtime announcement](https://delta.chat/en/2024-11-20-webxdc-realtime)). Two integration points:

- **Realtime channels for webxdc apps.** When a user opens a webxdc app inside a chat that calls `joinRealtimeChannel()`, Delta Chat sends an end-to-end encrypted system message containing an iroh ticket. Other devices that open the same webxdc consume the ticket and establish a direct iroh connection. No DHT lookup, no broadcast — the existing chat is the bootstrap channel. This sidesteps the slowest part of typical P2P bootstrap.
- **Multi-device setup.** When a user adds a new device to their account, the keys + state transfer over iroh rather than over the legacy email-channel-with-out-of-band-codes path.

Privacy property worth noting: Delta Chat runs iroh relays on its own chatmail servers, mirroring email federation. Relays don't see WLAN-only addresses peers advertise to each other and don't store IPs while facilitating connections. This is the privacy posture iroh's relay design enables — every Myrhiza relay deployment should adopt similar logging discipline.

Delta Chat's own framing: "We regard iroh to be one of the most interesting efforts to arise out of the ashes of Web3" ([Delta Chat solutions page](https://www.iroh.computer/solutions/delta-chat)). No public post-mortem of regrets; the public material is uniformly positive. Floris Bruynooghe (n0) and the Delta Chat team have given joint conference talks ([deltachat-and-iroh slides](https://devork.be/talks/deltachat-and-iroh.pdf)) walking through the integration.

### Spacedrive — iroh-native sync

Spacedrive's [README](https://github.com/spacedriveapp/spacedrive/blob/main/README.md) lists "Iroh (QUIC, hole-punching, local discovery)" as a core component. Per the project's documentation: "Devices connect directly via Iroh/QUIC. No servers, no cloud, no single point of failure." The integration covers:

- Direct library sync between devices in the same Spacedrive "library."
- BIP39 mnemonic-based pairing ceremony to bootstrap mutual trust between devices.
- Local-network discovery via mDNS, with iroh's relay infrastructure as the fallback when devices are off-LAN.
- Spacebot (Spacedrive's AI agent) reachable via P2P from any paired device.

Spacedrive is the cleanest "iroh-native" example: there is no parallel server-based sync path. v2 (June 2025) was a ground-up rewrite addressing v1-alpha lessons; v3 ([launch post](https://spacedrive.com/blog/spacedrive-v3-launch)) repositions as a "local-first data engine." Mobile clients are in development per the repo; the desktop apps are the mature surface today.

### Holochain — iroh as transport substitute

Holochain switched its **default** network transport to iroh in release 0.6.1, replacing the previous `tx5` WebRTC-based stack ([Holochain on X](https://x.com/Holochain/status/2014017238815158760), [Holochain 0.6 upgrade](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)). The community framing: iroh is "the engine behind the 'It Just Works' feeling in the latest Holochain 0.7 dev versions." A `relay_url` is now a required conductor-config field.

This is "uses iroh" not "is iroh-native": Holochain still has its own DHT, agent identity, validation, and source chain semantics. Iroh is purely the bytes-between-peers layer. For Myrhiza this is the most directly relevant adoption pattern — a richer runtime using iroh as the floor and *not* exposing iroh's own protocols (blobs, gossip, docs) directly to apps. See [`../holochain/networking.md`](../holochain/networking.md) for the Holochain side.

### Paycode — narrow industrial deployment

The [iroh for payments](https://www.iroh.computer/blog/iroh-for-payments) post (March 2026) describes Paycode using iroh to connect payment terminals to point-of-sale systems at highway toll booths "with no additional servers." This is a real shipping deployment in a constrained industrial setting; the customer story is n0-published rather than independently verified, and details on scale are not public. Useful as an existence proof for "iroh works in low-touch infrastructure deployments" rather than a reference customer at consumer scale.

### Dumbpipe — n0's own demo

[Dumbpipe](https://www.dumbpipe.dev/) ([repo](https://github.com/n0-computer/dumbpipe)) is the simplest end-to-end iroh app and the cleanest pedagogical example. The whole tool is "approximately 200 lines around the iroh Rust crate." It works as a netcat that uses 32-byte EndpointIDs instead of `host:port`. Available via `cargo install dumbpipe` or `brew install dumbpipe`. Practical use cases the docs demonstrate: video streaming with FFmpeg, terminal sharing via tty-share, web-server forwarding, Unix-socket tunneling for tools like Zellij.

## Worked example: Dumbpipe end-to-end

Walk through the simplest possible iroh connection. Two machines, A (listener) and B (connector).

```bash
# Machine A — listen for an incoming pipe; forward stdin/stdout
$ dumbpipe listen
> ticket: dumbpipeAcrwhmusoqf362j3jpzrehzkw3bqamcp2mmbhn3fmag3mzzfjp4...
```

What happened on machine A:
1. Generated a fresh Ed25519 keypair (or loaded a persisted one).
2. Bound a QUIC endpoint to a random UDP port.
3. Connected to its closest iroh relay (default n0 infrastructure unless overridden) and registered for incoming connections.
4. Published a pkarr-signed DNS record to `dns.iroh.link` mapping its EndpointID to its relay URL + direct addresses.
5. Printed a ticket bundling EndpointID + relay URL + direct addresses + an `ALPN` for the dumbpipe protocol.

```bash
# Machine B — paste the ticket; pipe lines through
$ echo "hello from B" | dumbpipe connect dumbpipeAcrwhmusoqf362j...
```

What happened on machine B:
1. Decoded the ticket (base32-lowercase + postcard) into EndpointID + relay URL + direct addresses.
2. Initiated a QUIC connection to A. The connection attempt races: direct UDP to each candidate address simultaneously with a relayed path through the relay URL.
3. If hole-punching succeeds (the [9-out-of-10 number](https://www.iroh.computer/blog/comparing-iroh-and-libp2p) n0 cites), the relay path is dropped after handshake; bytes flow direct A↔B with QUIC's authenticated encryption rooted in A's Ed25519 key.
4. If hole-punching fails, the relayed path stays, end-to-end encryption still holds because the QUIC handshake bound to A's pubkey — relay sees ciphertext only.

The text "hello from B\n" arrives on A's stdout. Connection closes.

The whole thing is roughly: 32-byte pubkey + relay URL + direct addresses → encrypted bidirectional byte stream, in well under a second on a healthy network. **This is the primitive Myrhiza inherits.**

## Implications for Myrhiza

- **"Bytes between known peers" composes well; "find peers from nothing" is not iroh's job.** Every successful iroh app has a *bootstrap channel* outside iroh — Delta Chat uses email, Spacedrive uses BIP39 pairing ceremonies, Holochain uses its own DNA-membership semantics, Dumbpipe uses copy-paste. Plan Myrhiza's join/invite flow as a separate concern; don't expect iroh to do peer discovery from a cold start without a hint.
- **Tickets are an under-appreciated UX primitive.** A single base32 string that combines "who" + "where to find them" + "what app" is the right bootstrap shape for invite links, QR codes, deep links. Myrhiza should adopt the ticket pattern (with its own ALPN and app-data extension) rather than inventing a competing format.
- **Hybrid online/offline is the realistic shape.** Delta Chat's iroh-realtime + email-store-and-forward hybrid is a much stronger product than "iroh for everything." Apps that need delivery to backgrounded mobile clients need a slower-but-reliable second tier. Myrhiza should make this a first-class kernel pattern: state-apply components see events the same way regardless of which tier delivered them.
- **The "iroh-native" deployments are the smallest.** Spacedrive and Dumbpipe are iroh-native and are also the smallest by user count. Delta Chat and Holochain are the largest — and they treat iroh as a layer, not the whole stack. The lesson: a runtime that succeeds will look more like Delta Chat's pattern than Spacedrive's.
- **No app has a horror-story post-mortem.** I could not find an "iroh failed us, here's why we left" public writeup. This is a genuinely good signal for committing — but also a small-N signal, given the modest production fleet.

## Sources

- [iroh 1.0 roadmap (production-stat citations)](https://www.iroh.computer/blog/road-to-1-0)
- [iroh homepage customer list](https://www.iroh.computer/)
- [Delta Chat 1.48 P2P announcement](https://delta.chat/en/2024-11-20-webxdc-realtime)
- [Delta Chat & Iroh integration slides](https://devork.be/talks/deltachat-and-iroh.pdf)
- [Iroh solutions: Delta Chat](https://www.iroh.computer/solutions/delta-chat)
- [Spacedrive repo + README](https://github.com/spacedriveapp/spacedrive)
- [Spacedrive v3 launch post](https://spacedrive.com/blog/spacedrive-v3-launch)
- [Holochain 0.6 upgrade notes](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)
- [Holochain on X — iroh transport announcement](https://x.com/Holochain/status/2014017238815158760)
- [iroh for payments (Paycode)](https://www.iroh.computer/blog/iroh-for-payments)
- [Dumbpipe homepage](https://www.dumbpipe.dev/)
- [Dumbpipe repo](https://github.com/n0-computer/dumbpipe)
- [iroh tickets concept](https://docs.iroh.computer/concepts/tickets)
- [Comparing iroh & libp2p (9-of-10 success rate)](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
