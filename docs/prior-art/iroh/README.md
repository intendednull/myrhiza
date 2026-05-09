**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — Rust P2P networking stack from Number 0 (load-bearing dependency)

# Iroh

Iroh is a Rust P2P stack from [Number 0](https://www.iroh.computer/) (n0) whose central primitive is "dial a peer by its 32-byte Ed25519 public key, get an authenticated bidirectional QUIC connection." Every other module — content-addressed blobs, eventually-consistent docs, gossip overlays, custom transports — is built around that primitive. As of 2026-05-08, iroh just shipped `1.0.0-rc.0` (2026-05-07) after a long pre-1.0 churn that included the `NodeId → EndpointId` rename in 0.94, the multipath-QUIC landing in 0.96, and the Quinn-fork-graduates-to-`noq` move in 0.97.

Unlike Holochain or Spritely (peer systems we *learn from*), Iroh is a **load-bearing dependency** Myrhiza will likely commit to as its P2P transport substrate. The lens of this corpus is therefore "what does a Myrhiza spec author need to know to commit hard against this library and write specs against its surface?" — not "what could we steal." Treat the open-problems and critiques files as a watch list for places Myrhiza's own specs will need to compensate.

## Key facts

| Fact | Value |
|---|---|
| Origin | Forked from [beetle](https://github.com/n0-computer/beetle) (an IPFS-rewrite, archived in late 2023 — last push 2023-11-22); pivoted Feb 17 2023 with [A new direction for iroh](https://www.iroh.computer/blog/a-new-direction-for-iroh) |
| Founder | Brendan O'Brien ([b5](https://github.com/b5)), ex-Protocol Labs / IPFS |
| Steward | [Number 0](https://number0.computer/) (private company); funding "partly venture capital and partly founder-backed" — no publicly disclosed rounds |
| License | Apache-2.0 / MIT dual, no CLA/DCO, inbound=outbound |
| Primary language | Rust (single canonical implementation; no published wire spec yet) |
| Current versions (as of 2026-05-08) | `iroh 1.0.0-rc.0` (2026-05-07), `iroh-blobs 0.101.0`, `iroh-docs 0.99.0`, `iroh-gossip 0.99.0`, `iroh-willow 0.0.1` (stalled on iroh 0.34) |
| Last stable | `iroh 0.98.2` (2026-04-28) |
| Workspace crates | `iroh`, `iroh-base`, `iroh-dns`, `iroh-dns-server`, `iroh-relay` (the legacy `iroh-net` was folded into `iroh` in **0.29** — Dec 2024 — and the workspace was further consolidated in the 0.90 "Canary Series" reorg, Jun 2025) |
| Identity | NodeID = 32-byte Ed25519 public key (renamed `EndpointId` in 0.94). No DID layer. Application-managed key custody. |
| Relays | 4 n0-operated default relays. DERP-derived protocol over HTTPS/WebSocket. Self-hostable via `iroh-relay`. |
| FFI / mobile | `iroh-ffi` README self-declares "reference example only" since Feb 2025 (GitHub `archived` flag is *not* set; last push 2026-02-07 — unmaintained for production). Production paths are paid (`iroh-c-ffi`, active 2026; `iroh-js` dormant since Dec 2023) or third-party. |
| Browser / WASM | Relay-only over WebSocket; no WebTransport-backed transport in production. |
| Verified production users | [Delta Chat](https://delta.chat/) 1.48+ (hundreds of thousands of devices), [Spacedrive](https://www.spacedrive.com/), [Holochain](https://www.holochain.org/) 0.6.1-rc (default transport), [Paycode](https://paycode.com/), [Dumbpipe](https://dumbpipe.dev/) (n0 demo). |

## Contents

Each file is independent and skimmable standalone.

**Networking core**
- [**Architecture**](architecture.md) — `Endpoint`, crate split, ALPN-based protocol multiplexing, `Router`, connection lifecycle, `EndpointAddr` / tickets.
- [**Transports**](transports.md) — Quinn → `noq` graduation (0.97), multipath QUIC (0.96), custom-transports API, browser viability.
- [**NAT traversal**](nat-traversal.md) — DERP-derived relay protocol, n0 default relay fleet, QAD (replaces STUN), QUIC NAT-traversal extension status, hole-punching reliability.

**Data plane**
- [**Blobs**](blobs.md) — `iroh-blobs`, BLAKE3 + Bao verified streaming, HashSeq collections, no-built-in-discovery, tag-based GC.
- [**Docs**](docs.md) — `iroh-docs`, multi-author KV with author-signed writes, range-based set reconciliation, last-writer-wins.
- [**Gossip**](gossip.md) — `iroh-gossip`, HyParView + Plumtree topic-based pub/sub.
- [**Willow**](willow.md) — `iroh-willow` Willow-protocol implementation; stalled on iroh 0.34 since March 2025.

**Identity, client, ecosystem**
- [**Identity**](identity.md) — Ed25519 keys, `NodeId` → `EndpointId` rename, encoding contexts, no DID layer, no rotation, FROST research.
- [**Mobile and WASM**](mobile-and-wasm.md) — iroh-ffi archival, paid mobile bindings, browser relay-only, no idle/wake mode.
- [**Apps**](apps.md) — Delta Chat, Spacedrive, Holochain, Paycode, Dumbpipe; one Dumbpipe end-to-end walkthrough.
- [**Ecosystem**](ecosystem.md) — Number 0 governance posture, adjacent crates (`n0-future`, `noq`), conferences, community size.

**Project lens**
- [**History**](history.md) — beetle (2022) → pivot (Feb 2023) → module spinout (0.28, Nov 2024) → 1.0-rc (May 2026).
- [**Governance**](governance.md) — Number 0 the company, undisclosed funding, single-implementation-protocol risk, stewardship if n0 fails.
- [**Comparisons**](comparisons.md) — vs libp2p (most important), Hypercore / Pears, Tailscale, Magic Wormhole / Croc.
- [**Critiques**](critiques.md) — HN/Reddit verbatim quotes — relay metadata leak, API churn, Windows Defender FP, Quinn fork debt, "complex enough I gave up."
- [**Open problems**](open-problems.md) — discovery, identity portability, Sybil, relay economics, censorship, durability, consensus, perf benchmarks, wire spec.

**Tooling, distribution, testing**
- [**Tooling**](tooling.md) — no `iroh` CLI exists; tooling is `sendme`, `dumbpipe`, `iroh-doctor`. No `n0-spec` repo.
- [**Distribution**](distribution.md) — monthly minor cadence, breaking-change policy pre-1.0, mobile artifact distribution.
- [**Testing**](testing.md) — `patchbay` netns simulator, `chuck` perf-on-main, proptest in relay codec, no determinism guarantees.

**Reference**
- [**Lessons for Myrhiza**](lessons.md) — validates / avoid / borrow — **the consult-this-when-designing file.**
- [**Glossary**](glossary.md) — Endpoint, EndpointId, ALPN-routing, EndpointAddr, ticket, RelayUrl, DERP, QAD, pkarr, Bao, HashSeq, etc.

## Recommended reading order

For a Myrhiza spec author committing to iroh as transport: start with [**lessons.md**](lessons.md) for the action-oriented summary. Then read [**architecture.md**](architecture.md) (the `Endpoint` is the only API surface the kernel directly couples to) and [**open-problems.md**](open-problems.md) (everything Myrhiza will need to solve at the layer above). Then dip into the data-plane files for whatever module the spec touches.

For a code reviewer evaluating an iroh-related PR: read [**transports.md**](transports.md) and [**distribution.md**](distribution.md) for current version expectations, then the relevant subsystem file.

## How to use this prior-art doc

This corpus exists to spare future Myrhiza spec authors from re-running the research every time iroh ships a breaking minor. The pinned version numbers and dates are accurate as of the **Date:** in this README; bump the date when meaningful churn happens upstream.

**Framing disclosure.** These docs are written from a Component-Model-as-foundation, P2P-only, capability-mediated-host-imports stance — most "Implications for Myrhiza" sub-sections frame iroh's choices through that lens. Iroh itself takes no position on Component Model or capabilities; we are reading a transport library through a runtime-design lens.

This corpus also carries a second framing bias the Holochain/Spritely folders don't: iroh is a **load-bearing dependency** Myrhiza is committing to, not a peer system we learn from at arm's length. The corpus has structural incentive to soft-pedal problems Myrhiza will inherit and to read iroh's strengths as confirmation of decisions Myrhiza has already made. Future readers should treat the "Validates" entries in [`lessons.md`](lessons.md) with appropriate skepticism: those are claims about *us* dressed as observations about iroh. The "Avoid" and "Open problems" sections are the load-bearing critical content; weight them accordingly.

It's a learn-from-iroh-into-Myrhiza artifact, not a neutral catalog. The Holochain and Spritely prior-art folders carry the framing-disclosure pattern for the same reason.
