**Date:** 2026-05-22
**Status:** active
**Subject:** WebRTC native implementations — libdatachannel, str0m, webrtc-rs, pion, libwebrtc

# Implementations

Four real choices for "server-side or native-app WebRTC" outside the browser:

| Impl | Language | Architecture | Latest | License | Stars |
|---|---|---|---|---|---|
| **libdatachannel** | C++ (with C bindings) | Threaded, IO-included | 0.24.3 (2026-05-09) | **MPL-2.0** (since v0.18) | 2.6k |
| **str0m** | Rust | **Sans-IO** pure state machine | 0.19.0 (2026-05-04) | MIT OR Apache-2.0 | 552 |
| **webrtc-rs** | Rust | Tokio-coupled (v0.17.x); sans-IO core (v0.20.x master) | 0.17.1 stable; 0.20.0-alpha.1 master | MIT OR Apache-2.0 | 5.0k |
| **pion/webrtc** | Go | IO-included, idiomatic Go | v4.2.13 (2026-05-22) | MIT | 16.5k |
| **libwebrtc** (Chromium) | C++ | Massive, embedded in browsers | rolling | BSD-3-Clause | (Chromium fork; not separately starred) |

Plus the elephant in the room: **`libwebrtc`** itself, Google's C++ WebRTC stack inside Chromium. Mentioned for completeness; not a realistic Myrhiza embedding target (huge codebase, tightly coupled to Chromium build system, designed to be the implementation *inside* a browser, not embedded by external apps).

## libdatachannel

[`paullouisageneau/libdatachannel`](https://github.com/paullouisageneau/libdatachannel) — the lightweight C++ WebRTC implementation maintained by Paul-Louis Ageneau.

> *"libdatachannel is a standalone implementation of WebRTC Data Channels, WebRTC Media Transport, and WebSockets in C++ with C bindings for multiple platforms..."* — README

**Architecture:**
- C++17, with explicit C bindings for FFI.
- Threaded internally; does its own IO via OpenSSL/MbedTLS/GnuTLS + libjuice/libnice for ICE.
- Dependencies: a TLS library, `usrsctp` (vendored), `plog` (vendored), `libjuice` or `libnice` for ICE. Optional `libsrtp` for media.

**Notable properties:**
- **Smallest mainstream WebRTC.** No Chromium dependency. Build artifacts are MB-scale, not GB.
- **MPL-2.0 since v0.18** (was LGPL prior). The MPL-2.0 license is file-level copyleft: derivative files must be open, but linking from a proprietary codebase is OK. This is more permissive than LGPL and broadly compatible with commercial use.
- **WebSocket and ICE built-in.** Not just WebRTC — the library also provides a usable WebSocket client/server and integrates ICE candidate gathering.
- **WebAssembly target.** The README mentions: *"may be compiled as is to WebAssembly for browsers."* This is interesting but not heavily used — in practice you'd just use the browser's native WebRTC API.

**Bindings & wrappers:**
- Node.js wrapper: [`murat-dogan/node-datachannel`](https://github.com/murat-dogan/node-datachannel) — Node-API bindings; actively maintained.
- Rust wrapper: [`lerouxrgd/datachannel-rs`](https://github.com/lerouxrgd/datachannel-rs) — Rust FFI; less active.
- Python wrapper: [`murat-dogan/python-datachannel`](https://github.com/murat-dogan/python-datachannel).

**Ecosystem reality:**
- Used by [Moonlight](https://moonlight-stream.org/) for low-latency game streaming.
- Used by some self-hosted WebRTC stacks (SFUs, embedded media servers).
- Honest scale: this is a *niche* library compared to libwebrtc/pion in raw deployment numbers, but a *substantial* one in C++ projects that need WebRTC without Chromium.

**Myrhiza-relevance:** If Myrhiza ever needs a native-host WebRTC implementation (e.g. for a desktop kernel that wants to be a WebRTC peer to mobile browsers), libdatachannel is the smallest sensible choice. The MPL-2.0 license is workable. The C bindings make it embeddable from Rust via `bindgen`.

## str0m

[`algesten/str0m`](https://github.com/algesten/str0m) — pure state-machine WebRTC in Rust. **The architectural odd one out** in this list, and the most interesting for Myrhiza's spec authors.

> *"A Sans I/O implementation meaning the `Rtc` instance itself is not doing any network talking. Furthermore it has no internal threads or async tasks. All operations are happening from the calls of the public API."* — README

**Architecture:**
- **Sans-IO design.** The `Rtc` struct is a pure state machine; you feed it `Input::Receive(...)` (incoming UDP datagrams + a timestamp) and `Input::Timeout(...)` (the current time), and it produces `Output::Transmit(...)` (datagrams to send) or `Output::Timeout(...)` (when to call it next). There are no threads, no async tasks, no internal sockets.
- The developer provides the UDP socket, the time source, and the event loop. The library is just the protocol.
- Crate: `str0m 0.19.0`, published 2026-05-04 by Martin Algesten; current rust-edition 2024, MSRV 1.85.0.

**Why sans-IO matters:**
- **Trivially testable.** Feed packet sequences, assert on outputs. No mocking sockets.
- **Embeddable in any runtime.** Works inside tokio, inside smol, inside an Embassy embedded async runtime, inside a single-threaded WASM context.
- **Deterministic.** Same inputs → same outputs. Useful for protocol fuzzing, useful for reproducing bugs from production traces.

**Costs:**
- **The developer integrates IO.** This is real work — you have to write a UDP socket loop, a timer loop, and feed the state machine. The crate ships [examples](https://github.com/algesten/str0m/tree/main/examples) for tokio/std-net, but they are templates, not turn-key code.
- **Smaller ecosystem.** 552 stars (vs pion's 16.5k); fewer "import and go" tutorials.
- **No media-side niceties.** str0m supports media (SRTP), but the integration is bare — you handle RTP packetization, jitter buffer, codec choice. Compared to pion's batteries-included A/V tooling, str0m is the protocol kernel.

**Production use:**
- Notable users include the [whip-whep-rs](https://github.com/whip-whep-rs) project (WHIP/WHEP servers for live media).
- Used inside `libp2p-webrtc` (the rust-libp2p server-side WebRTC) for its protocol layer.
- Honest scale: not "production scale" in the sense pion/libdatachannel are; the codebase is younger and the deployment surface is smaller.

**Architecture note for Myrhiza:** Sans-IO is the same architectural pattern Myrhiza's deterministic `state-apply` components use. **str0m is the closest design template in the WebRTC ecosystem to how Myrhiza thinks about state machines** (deterministic, pure-function-of-inputs, IO at the edge). Reading str0m's code as a study of sans-IO Rust at scale is valuable independent of whether Myrhiza ever uses it directly.

## webrtc-rs

[`webrtc-rs/webrtc`](https://github.com/webrtc-rs/webrtc) — pure-Rust WebRTC, originally a port of pion. The largest Rust WebRTC project by stars (5.0k).

**Two-track development:**
- **v0.17.x** — the Tokio-coupled stable line. Last release 0.17.1 (2026-02-06). In "feature freeze" for bug fixes only.
- **v0.20.0-alpha.1** — published 2026-03-01; reworking around a sans-IO `rtc` crate (similar architectural shift to str0m). The README explicitly states *"v0.17.x is the final feature release of the Tokio-coupled async WebRTC implementation"* — they're moving toward runtime-agnostic.

**Architecture (v0.17.x):**
- Tokio-async throughout. Every Peer/DataChannel method is `async fn`.
- Splits into ~15 sub-crates: `webrtc-ice`, `webrtc-dtls`, `webrtc-sctp`, `webrtc-srtp`, `webrtc-rtp`, etc. The "WebRTC" top-level crate is mostly glue.
- The split was inherited from pion's package layout (pion has the same split in Go).

**Maintainership:**
- Created by Rain Liu (`rainliu`). Active commits as of 2026-05.
- Sponsors: Recall.ai (Gold), Stream Chat / ChannelTalk (Silver), AdrianEddy (Bronze). Real funding.
- Governance is "BDFL by rainliu" — not a foundation, not multi-organization.

**Critique:**
- **The Tokio coupling is the biggest historical criticism.** Embedding in non-Tokio runtimes was awkward; the v0.20 rewrite addresses this directly.
- **Documentation is uneven.** Sub-crates have rustdoc, but the high-level "how do I write a WebRTC client" doc lags. The examples directory is the primary tutorial.

**Production use:**
- [Recall.ai](https://recall.ai) — meeting-bot infrastructure. Confirmed by Gold sponsor status.
- Used in some Tauri-based desktop apps that need native WebRTC.
- Crates.io 4.2M total downloads, 866k recent (90-day). Real usage.

**Myrhiza-relevance:** If Myrhiza wants Rust WebRTC and accepts Tokio, webrtc-rs is the path of least resistance (more mature than str0m, larger ecosystem). If Myrhiza wants sans-IO Rust WebRTC, str0m today is more polished, but webrtc-rs 0.20+ will compete on that axis.

## pion/webrtc

[`pion/webrtc`](https://github.com/pion/webrtc) — the Go canonical WebRTC implementation. The biggest non-libwebrtc WebRTC project (16.5k stars), and the production workhorse for Go-based real-time media.

**Architecture:**
- Pure Go (no Cgo). Important for cross-compilation and embeddability.
- Mirrors the libwebrtc public API surface closely — `PeerConnection`, `DataChannel`, `Track`, etc.
- Splits into sub-modules: `pion/ice`, `pion/dtls`, `pion/sctp`, `pion/srtp`. Same split webrtc-rs inherited.

**Maintainership:**
- Sean DuBois (`Sean-Der`) is the most visible maintainer; the project has dozens of regular contributors.
- Funded by **NLnet** through the User-Operated Internet fund (per the README: *"Work on Pion's congestion control and bandwidth estimation was funded through the User-Operated Internet fund"*).
- Community Discord, commercial support via `team@pion.ly`.
- One of the better-governed open-source media projects — wide contributor base, no single corporate sponsor controlling direction.

**Production use:**
- [Twitch's mobile broadcasting](https://github.com/twitchtv/twirp) (uses pion components).
- [LiveKit](https://livekit.io) — open-source WebRTC SFU; pion is the protocol substrate.
- [Janus-gateway](https://janus.conf.meetecho.com) alternatives; many smaller SFUs.
- [Mainflux/IoT projects](https://github.com/mainflux/mainflux).
- WHIP/WHEP server reference implementations.

**Critique:**
- **Go is the limit.** Embedding pion in non-Go projects requires FFI or RPC, both of which give up the language's advantages.
- **Verbose API.** Mirrors libwebrtc's verbose JavaScript-style callback patterns. Idiomatic Go errs toward terseness; pion is on the verbose end.

**Myrhiza-relevance:** Not directly relevant if Myrhiza stays in Rust/WASM. But pion is the **most production-deployed non-browser WebRTC outside libwebrtc** — the design choices it makes (which RFCs to implement strictly, which to deviate from) are the practical reference for "what does real production WebRTC look like." Reading pion's issue tracker is the best way to see what *actually goes wrong* in production WebRTC.

## libwebrtc (Chromium)

The Google reference implementation. C++. Hundreds of thousands of LOC. Embedded in every Chromium-based browser and in mobile WebRTC SDKs (the iOS / Android `WebRTC.framework`).

**Why it's mentioned:**
- It's the *de facto* spec — when an RFC and libwebrtc disagree, libwebrtc usually wins (browsers ship libwebrtc).
- Any non-libwebrtc implementation has to interop with it. Every bug in libwebrtc is everyone else's bug.

**Why it's not a Myrhiza option:**
- Build system is bound to Chromium's `gn` + `ninja`. Standalone builds exist but are operationally painful.
- The "embeddable" path is the iOS/Android SDKs, which are themselves giant frameworks.
- For server-side or non-Chromium-browser use, every team picks one of the four lighter-weight stacks above.

## Side-by-side comparison

| Axis | libdatachannel | str0m | webrtc-rs (0.17) | webrtc-rs (0.20+) | pion |
|---|---|---|---|---|---|
| Language | C++ | Rust | Rust | Rust | Go |
| IO model | threaded, internal | **sans-IO** | tokio-async | sans-IO + async wrapper | goroutine-based |
| Production scale | medium | small | medium | (early) | large |
| Stars | 2.6k | 552 | 5.0k | (same repo) | 16.5k |
| License | MPL-2.0 | MIT OR Apache-2.0 | MIT OR Apache-2.0 | MIT OR Apache-2.0 | MIT |
| Lines of code (approx) | ~30k C++ | ~25k Rust | ~50k Rust | ~50k Rust | ~80k Go |
| Embeddable in Myrhiza? | yes via FFI | yes (most natural) | yes if Tokio is OK | yes (target architecture) | no (Go RPC only) |
| Sans-IO == Myrhiza-shape? | no | **yes** | no | yes | no |

## Choosing for Myrhiza

If a native Myrhiza host kernel ever needs to be a WebRTC peer (e.g. desktop kernel ↔ browser-peer):

1. **First choice: str0m.** Sans-IO matches Myrhiza's state-machine architecture. Embeddable in any runtime. License-compatible. Smaller surface to audit. The trade-off — developer integrates IO — is the *right* trade-off for a runtime that already owns IO at the kernel layer.
2. **Second choice: webrtc-rs 0.20+ once stable.** Larger ecosystem, more production miles. Wait for 0.20 to leave alpha.
3. **Third choice: libdatachannel via Rust FFI.** If for some reason the Rust options don't work; expect more friction.
4. **Avoid: pion** (Go, doesn't fit), **libwebrtc** (build system, scale).

If Myrhiza is browser-only (no native peer), none of the above matter — the browser ships libwebrtc, and the JS surface is whatever the W3C spec exposes. The libraries listed here are for the *other end* of the connection (server, native peer, gateway).

## Sources

- libdatachannel: <https://github.com/paullouisageneau/libdatachannel>
- str0m repo: <https://github.com/algesten/str0m>
- str0m crates.io (v0.19.0, 2026-05-04, 1.2M downloads, MIT OR Apache-2.0): <https://crates.io/crates/str0m>
- webrtc-rs repo: <https://github.com/webrtc-rs/webrtc>
- webrtc-rs crates.io (v0.17.1 stable, v0.20.0-alpha.1, 4.2M downloads): <https://crates.io/crates/webrtc>
- pion/webrtc: <https://github.com/pion/webrtc>
- libwebrtc: <https://webrtc.googlesource.com/src/>
- NLnet funding for pion: <https://nlnet.nl/project/Pion/>
- Cross-refs: [`stack.md`](stack.md), [`lessons.md`](lessons.md)
