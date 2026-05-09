**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — mobile and browser viability

The honest summary up front: native mobile via iroh works *if* you write your own thin FFI; the official `iroh-ffi` repo is unmaintained for production and the README explicitly calls it "reference example only." Browser support is **relay-only** — it works, end-to-end encryption holds, but every byte goes through a relay because browser sandboxes can't open UDP sockets.

## iroh-ffi — unmaintained for production

The [iroh-ffi repo](https://github.com/n0-computer/iroh-ffi) carries UniFFI bindings for Swift / Kotlin / Python plus a NAPI binding for JavaScript. n0 paused active updates in February 2025 ([FFI update post](https://www.iroh.computer/blog/ffi-updates)) and the README opens with *"This repository is archived and provided as a reference example only."* The GitHub `archived` flag is *not* set as of 2026-05-08 — the last push was 2026-02-07 — but treat the repo as dormant for any production purpose. Reasons given:

- "We don't think our FFI story is good enough right now." Generated bindings exposed too much Rust-shaped API surface to feel native.
- Ecosystem fragmentation: a Python protocol implementation built on FFI only talks to other Python-on-FFI peers, not to the broader iroh universe.

The post-1.0 plan considers three options: higher-level bridges (Flutter Rust Bridge, React Native UniFFI), composable FFI generated per-protocol, or native re-implementations against a 1.0 spec. None of these has shipped as of May 2026; the v1.0.0-rc.0 milestone closed May 7 2026 ([1.0 milestone](https://github.com/n0-computer/iroh/milestone/34)) without a successor FFI announcement.

What *does* exist:

- **iroh-c-ffi** ([repo](https://github.com/n0-computer/iroh-c-ffi)) — a separate, narrower C-ABI binding. Less abandoned than iroh-ffi but still small-surface.
- **App-author-maintained wrappers.** Delta Chat and Spacedrive both maintain their own thin Rust→native shims. n0 explicitly recommends this path ("write your own small wrapper around iroh that covers just what you need") in the FFI update post.

This is the "library, not platform" trade-off in plain view. If you need iOS/Android, you write Rust, you use `cargo-ndk` / `cargo-lipo`, you generate UniFFI bindings yourself for the surface you actually call. That works — Delta Chat 1.48 ships this on hundreds of thousands of devices ([Delta Chat realtime announcement](https://delta.chat/en/2024-11-20-webxdc-realtime)) — but it's not a turnkey path.

## Browser / wasm32-unknown-unknown

iroh has compiled to WebAssembly since **0.32 (alpha)** and **0.33 (beta)** ([0.32 release post](https://www.iroh.computer/blog/iroh-0-32-0-browser-alpha-qad-and-n0-future)). The official documentation lives at [docs.iroh.computer/deployment/wasm-browser-support](https://docs.iroh.computer/deployment/wasm-browser-support). What works:

- Compilation to wasm32 via `wasm-bindgen`. `iroh = { version = "X", default-features = false }` is the canonical incantation; metrics, test-utils, and DHT discovery features won't link.
- Connections to *any* native iroh peer, **end-to-end encrypted**, but **always via a relay**. The browser opens a WebSocket to the relay; QUIC frames tunnel inside. Phase 0 of the [Iroh & the Web](https://www.iroh.computer/blog/iroh-and-the-web) roadmap.
- Examples that compile and run today: a browser echo server and a chat-room UI ([iroh-examples/browser-echo](https://github.com/n0-computer/iroh-examples/tree/main/browser-echo)).

What does **not** work:

- **No direct connections in browser.** Browser sandboxes cannot send UDP to arbitrary IPs. Hole-punching is impossible. Phase 3 of the roadmap explores WebRTC data channels for browser-to-browser direct paths; not shipped.
- **No WebTransport-backed transport in production.** [`web-transport-iroh`](https://github.com/n0-computer/web-transport-iroh) exists as an experiment expressing WebTransport semantics over iroh connections, but the *reverse* — using browser-native WebTransport as iroh's transport — is not available. WebTransport would require DNS names + valid TLS certs on every peer, which defeats pubkey-as-identity for arbitrary devices. The 0.32 post explicitly notes this trade-off.
- **No NPM bundle.** You write a Rust wrapper crate with wasm-bindgen, ship the resulting `.wasm` + JS shim yourself. Common Wasm/browser troubleshooting lives in [discussion #3200](https://github.com/n0-computer/iroh/discussions/3200).

Net effect for a browser-hosted Myrhiza app: **all peer traffic transits a relay.** End-to-end encryption holds, n0's relays are public infrastructure, and self-hosting a relay is straightforward — but the "directly between users with no third party" property does not survive contact with browser tabs.

## Battery, wake, idle — no special story

Iroh has **no built-in idle / wake-aware mode.** It is a userland Rust library; it does not know about Doze, App Standby, iOS background-app refresh, or Apple's Push Notification Service. The connection state machine runs while the host process runs.

This is the same shape as libp2p, Tailscale's native libraries, etc. — and it means a "real" mobile app on iroh has to:

- Tear down endpoints when the app suspends; reconnect on resume.
- Use *the platform's* push channel (APNs / FCM) as a wakeup-from-cold doorbell. Iroh cannot wake your app.
- Accept that a "P2P chat" will not deliver a message to a backgrounded iOS app the way a server-pushed notification will.

Delta Chat's integration is informative here ([Delta Chat realtime post](https://delta.chat/en/2024-11-20-webxdc-realtime)): the *realtime channel* (peer gossiping inside an open webxdc app) uses iroh; the *base messaging layer* is still email-over-SMTP, which delivers to the platform's mail backend even when the app is asleep. Iroh handles the "users are both interactively in the same room" path; the platform handles the "deliver while app is dead" path. They explicitly designed it as a hybrid for this reason.

## Code size

n0 has not published a canonical "iroh-on-mobile is N MB" number. Two data points:

- **ESP32 (extreme low-end).** [iroh on an ESP32](https://www.iroh.computer/blog/iroh-on-esp32) — runs in 4 MiB flash + 4 MiB SPIRAM, with link-time optimization eating "88.53%" of the available binary space and a single-threaded tokio runtime. The example uses *patched* iroh and patched dependencies; it is a proof of concept, not a supported configuration. Useful as a "what is the smallest thing that works" data point.
- **Wasm bundle.** No published number from n0. Default-features-off iroh + a thin wasm-bindgen wrapper produces a multi-megabyte `.wasm` (anecdotally; you measure your own). Tree-shaking is feature-flag-driven; you pay for what's enabled.

Realistic take: an iroh-using mobile app is in the same neighborhood as a Tailscale-using mobile app or a libp2p-using mobile app. Tens of MB of native code, not orders of magnitude more or less.

## Apps shipping iroh on mobile today

Verified production deployments as of May 2026:

- **Delta Chat (1.48+).** Android + iOS + desktop. Realtime channels in webxdc apps use iroh. n0 calls out "hundreds of thousands of devices" on the [Delta Chat solutions page](https://www.iroh.computer/solutions/delta-chat); this is consistent with Delta Chat's own user numbers and is the largest verified mobile deployment.
- **Spacedrive.** v2 alpha shipped in 2025; v3 the local-first data engine described mid-2026. P2P sync uses iroh on macOS, Windows, Linux, with Android and iOS clients in development per the [Spacedrive repo](https://github.com/spacedriveapp/spacedrive). Mobile shipping status is "in active development, not yet GA on app stores" — verify before claiming.
- **Holochain 0.7+.** Holochain switched its default network transport to iroh in 0.6.1 ([Holochain on X](https://x.com/Holochain/status/2014017238815158760)), replacing the WebRTC-based tx5. Volla Quintus phones running Holochain hApps therefore ship iroh transitively. See [`../holochain/`](../holochain/) for the full Holochain story.

I did not find evidence of an iroh-on-iOS app store presence with millions of installs. Delta Chat is the largest credibly verified mobile deployment; everyone else is smaller, in development, or self-distributed.

## Implications for Myrhiza

A Myrhiza spec author committing to iroh as the transport will inherit the constraints above wholesale. The implications worth surfacing — each a question for a Myrhiza-layer spec, not a settled answer:

- **Browser-hosted Myrhiza apps are viable but relay-bound.** If a Myrhiza node is "tab open in Chrome," all its traffic goes through a relay. Whether Myrhiza ships its own relay fleet, depends on n0's, or makes apps choose is an ops-spec decision; the relay infrastructure is a kernel concern, not an afterthought.
- **Mobile distribution is unsolved upstream.** Plan to maintain a Myrhiza-specific FFI layer atop `iroh-c-ffi`, narrowed to capabilities the kernel exposes — not the whole iroh API. Don't bet on iroh-ffi being rebooted.
- **The kernel owns background lifecycle.** Apps cannot assume "always connected." When endpoints are torn down, when they wake from external push, how state-apply components observe (or don't observe) connection state — these are Myrhiza-layer specs, and they cross over into the determinism story (state-apply called after a long sleep + reconnect must produce the same output as on a healthy connection).
- **Two-tier delivery is the realistic shape for messaging.** Delta Chat's pattern — iroh for the live realtime channel, email-over-SMTP for the wake-from-cold delivery — is a stronger product model than "iroh for everything." A Myrhiza messaging app will need a similar slower-but-more-reliable tier; whether that's email, a CRDT-on-relay store, or a Myrhiza-native mailbox is a per-app decision.
- **WebTransport direct paths are not available.** As of May 2026 the path from a browser tab to a native peer is `WebSocket → relay → QUIC → native`. The architecture documents and roadmaps Myrhiza writes should not assume otherwise.

## Sources

- [iroh-ffi repo (archived)](https://github.com/n0-computer/iroh-ffi)
- [iroh-c-ffi repo](https://github.com/n0-computer/iroh-c-ffi)
- [FFI update (Feb 2025)](https://www.iroh.computer/blog/ffi-updates)
- [Iroh & the Web blog post](https://www.iroh.computer/blog/iroh-and-the-web)
- [iroh 0.32.0 release post](https://www.iroh.computer/blog/iroh-0-32-0-browser-alpha-qad-and-n0-future)
- [WASM/browser support docs](https://docs.iroh.computer/deployment/wasm-browser-support)
- [iroh-examples/browser-echo](https://github.com/n0-computer/iroh-examples/tree/main/browser-echo)
- [web-transport-iroh experiment](https://github.com/n0-computer/web-transport-iroh)
- [Common Wasm/browser troubleshooting](https://github.com/n0-computer/iroh/discussions/3200)
- [Mobile libraries roadmap discussion #517](https://github.com/n0-computer/iroh/discussions/517)
- [iroh on an ESP32 blog post](https://www.iroh.computer/blog/iroh-on-esp32)
- [Delta Chat realtime / iroh announcement](https://delta.chat/en/2024-11-20-webxdc-realtime)
- [Delta Chat solutions page](https://www.iroh.computer/solutions/delta-chat)
- [Spacedrive repo](https://github.com/spacedriveapp/spacedrive)
- [Holochain 0.6.1 → iroh transport (n0 post on X)](https://x.com/Holochain/status/2014017238815158760)
- [iroh v1.0.0-rc.0 milestone](https://github.com/n0-computer/iroh/milestone/34)
