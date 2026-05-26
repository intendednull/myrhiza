**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — implementations comparison (go / rust / js / nim / cpp / jvm / py)

# Implementations

libp2p ships in seven languages with varying maturity, feature parity, and stewardship. The five production-grade implementations are **go-libp2p**, **rust-libp2p**, **js-libp2p**, **nim-libp2p**, **jvm-libp2p**. Two are partial/experimental: **cpp-libp2p**, **py-libp2p**.

**Critical anti-pattern (per skill Lesson #5/#7):** do not assume single license across all impl repos. Verified per-repo:

| Implementation | License | Stewards | Verified via |
|---|---|---|---|
| **go-libp2p** | MIT | Protocol Labs + IPFS team | [`libp2p/go-libp2p`](https://github.com/libp2p/go-libp2p) repo license field |
| **rust-libp2p** | **MIT** (every crate, verified per-Cargo.toml) | rust-libp2p team (Elena Frank `@elenaf9`, João Oliveira `@jxs`); historical author Parity Technologies | crates.io + repo license fields |
| **js-libp2p** | **Apache-2.0 OR MIT** (dual) | Protocol Labs + ChainSafe (for gossipsub) | npm registry `license` field |
| **nim-libp2p** | **Apache-2.0 OR MIT** (dual; `LICENSE-APACHEv2` + `LICENSE-MIT` in repo root) | Status Research & Development GmbH (Vac team) | repo LICENSE files |
| **jvm-libp2p** | **MIT + Apache-2.0** (Permissive License Stack — dual-licensed per repo NOTICE) | ConsenSys (Teku Eth2 client) + ChainSafe | [`libp2p/jvm-libp2p`](https://github.com/libp2p/jvm-libp2p) |
| **cpp-libp2p** | **Apache-2.0 OR MIT** (dual; `LICENSE-APACHE` + `LICENSE-MIT` in repo root) | Soramitsu (KAGOME Polkadot client) | [`libp2p/cpp-libp2p`](https://github.com/libp2p/cpp-libp2p) |
| **py-libp2p** | MIT/Apache-2.0 | Ethereum Foundation (Trinity client era), now in slow recovery | [`libp2p/py-libp2p`](https://github.com/libp2p/py-libp2p) |

The dual Apache-2.0 OR MIT on js-libp2p is common in the JavaScript-around-IPFS world; `@chainsafe/libp2p-gossipsub` (the gossipsub TS port) is **Apache-2.0 only**, not dual — a single-license drift inside the otherwise-dual js ecosystem. Verified at npm registry directly.

## go-libp2p (the reference implementation)

[`github.com/libp2p/go-libp2p`](https://github.com/libp2p/go-libp2p) is the canonical libp2p. Verified facts (2026-05-22):

- **Version:** `v0.48.0` (released 2026-03-17). Per `version.json` on `master`.
- **Go version requirement:** Go 1.25.7 (per `go.mod`).
- **License:** MIT.
- **Stars:** ~6,800 (most-starred libp2p impl).
- **Default branch:** `master`.

Used by:

- **kubo / go-ipfs** — the reference IPFS implementation. Every public IPFS node runs go-libp2p.
- **Lotus (Filecoin)** — Protocol Labs' Filecoin reference client.
- **Prysm (Ethereum)** — one of the five Eth2 consensus clients.
- **Drand** — distributed randomness beacon used by Filecoin.
- **Boxo** (formerly go-libp2p-pubsub-tracer + assorted IPFS components).

go-libp2p is the implementation **all other implementations interop against**. When a spec change is contested, go-libp2p's behavior is the de facto answer. The retracted versions in `go.mod` are `v0.26.1` and `v0.36.0` — both pulled due to release-workflow issues, not protocol bugs.

## rust-libp2p

[`github.com/libp2p/rust-libp2p`](https://github.com/libp2p/rust-libp2p). Verified facts:

- **Version on crates.io:** `libp2p 0.56.0` (2025-06-27).
- **Version on `master` (unreleased):** `0.57.0`.
- **License:** MIT (every crate in the workspace verified).
- **Stars:** ~5,500.
- **Rust version requirement:** **`rust-version = "1.88.0"`** in workspace.package; edition 2024.
- **Workspace structure:** 60+ member crates organized into `core/`, `swarm/`, `identity/`, `protocols/`, `transports/`, `muxers/`, `misc/`, `examples/`, `interop-tests/`.
- **Maintainers:** Elena Frank (`@elenaf9`), João Oliveira (`@jxs`). Historical author "Parity Technologies <admin@parity.io>" appears in most crate Cargo.toml `authors` fields — the original rust-libp2p was written at Parity for Substrate/Polkadot, then handed to Protocol Labs ~2019.
- **MSRV policy:** The workspace pins one Rust version; the practical MSRV moves as the workspace bumps. Stated explicitly in workspace.toml.

Used by:

- **Substrate / Polkadot** — the original use case, predating the IPFS handoff.
- **Subspace / Autonomys** — recent network using rust-libp2p heavily.
- **Storm** (Holepunch's IPFS-alternative experiment).
- **Iroh** (historically — iroh forked off the libp2p approach in 2023, but its early codebase had rust-libp2p in the dependency graph).
- **Lighthouse (Ethereum)** — written in Rust, uses a combination of rust-libp2p and custom code.

Workspace crates relevant to Myrhiza-style comparisons (versions as of crates.io 2026-05-22; master versions in parentheses if different):

| Crate | crates.io | Master | License |
|---|---|---|---|
| `libp2p` | 0.56.0 | 0.57.0 | MIT |
| `libp2p-core` | — (in 0.56) | 0.44.0 | MIT |
| `libp2p-swarm` | 0.47.1 (2026-01-21) | 0.48.0 | MIT |
| `libp2p-identity` | 0.2.13 | (workspace-relative) | MIT |
| `libp2p-gossipsub` | 0.49.4 (2026-03-26) | 0.50.0 | MIT |
| `libp2p-kad` | 0.48.0 (2025-06-27) | 0.49.0 | MIT |
| `libp2p-noise` | 0.46.1 (2025-06-27) | 0.47.0 | MIT |
| `libp2p-quic` | 0.13.0 (2025-06-27) | 0.14.0 | MIT |
| `libp2p-tcp` | — | 0.45.0 | MIT |
| `libp2p-websocket` | — | 0.46.0 | MIT |
| `libp2p-webrtc` | 0.9.0-alpha.1 (2025-06-27, **alpha for years**) | 0.10.0-alpha | MIT |
| `libp2p-webrtc-websys` | — | 0.5.0 | MIT |
| `libp2p-webtransport-websys` | — | 0.6.0 | MIT |

**Notable:** `libp2p` umbrella crate has not had a stable release since 2025-06-27 (`0.56.0`). Master is on `0.57.0` and has been for ~11 months. This is unusually quiet — earlier release cadence was every 2–3 months. The discussion forum + GitHub Discussions surfaces ongoing API discussion but no published reason for the slow release cadence. Watch this.

## js-libp2p

[`github.com/libp2p/js-libp2p`](https://github.com/libp2p/js-libp2p). Verified facts:

- **Monorepo structure:** `packages/*` workspace pattern, with the `libp2p` package as the main entry.
- **`libp2p` npm version:** `3.3.1` (npm `latest` dist-tag). Major version 3.x; the JS ecosystem went through a TypeScript-first rewrite ~2023 that bumped to 1.0 + further breakers since.
- **`libp2p` npm license:** `Apache-2.0 OR MIT` (dual).
- **Stars:** ~2,500.
- **Default branch:** `main`.
- **Production users:** **Helia** (the modern JS IPFS implementation, successor to `js-ipfs`); various Decentraland tooling; ChainSafe browser clients.

Notable adjacent npm packages (all `@libp2p/*` or `@chainsafe/*`):

| Package | npm `latest` | License | Steward |
|---|---|---|---|
| `libp2p` | 3.3.1 | Apache-2.0 OR MIT | Protocol Labs |
| `@libp2p/kad-dht` | 16.3.0 | Apache-2.0 OR MIT | Protocol Labs |
| `@chainsafe/libp2p-gossipsub` | 14.1.2 | **Apache-2.0** (single-license — drift from js-libp2p's dual!) | ChainSafe |
| `@chainsafe/libp2p-noise` | (~16.x) | Apache-2.0 | ChainSafe |
| `@chainsafe/libp2p-yamux` | (~7.x) | Apache-2.0 | ChainSafe |

The pattern: **ChainSafe maintains many of the "extension" libp2p packages in JS** under their own scope. This is partly historical (ChainSafe started forking gossipsub from go-libp2p-pubsub when js-libp2p's pubsub lagged) and partly a stewardship distribution. License drift (Apache-2.0-only vs the broader dual-licensed js-libp2p ecosystem) is a real but minor friction — Apache-2.0 is the more-conservative choice for the gossipsub-as-attack-target use case.

## nim-libp2p

[`github.com/vacp2p/nim-libp2p`](https://github.com/vacp2p/nim-libp2p). Verified facts:

- **Org:** `vacp2p` (Status Research & Development's "Vac" sub-org), **not** `status-im` or `libp2p` directly.
- **Version:** `1.15.3` per `libp2p.nimble` on master.
- **License:** MIT.
- **Author field:** "Status Research & Development GmbH" (the legal entity behind Status).

Used by:

- **Nimbus (Ethereum)** — one of the five Eth2 consensus clients.
- **Waku** — Status' privacy-preserving messaging stack.
- **Codex** — Status' incentivized data-availability network.
- Other Status / Vac research projects (Status Network, Logos, etc.).

The Vac team is heavily involved in gossipsub work — the **v1.2 IDONTWANT spec** is co-authored by `@Menduist` (Nimbus / Status), and Status has contributed several gossipsub optimizations upstream. nim-libp2p is small (vs go / rust by stargazers) but quietly load-bearing for a non-trivial slice of Ethereum's beacon network.

## jvm-libp2p

[`github.com/libp2p/jvm-libp2p`](https://github.com/libp2p/jvm-libp2p). Written in **Kotlin**, runs on the JVM.

- **License:** Apache-2.0.
- **Default branch:** `develop` (unusual — most libp2p impls use `master` or `main`).
- **Stewards:** ConsenSys (Teku Eth2 client), with ChainSafe contributions.
- **Version:** published per-release to Maven Central as `io.libp2p:jvm-libp2p:<n>`; this folder does not pin a specific version because `develop` is the active branch (versioned via Gradle at release time).

Production use: **Teku** (the Java/Kotlin Eth2 consensus client). One of the five major Eth2 clients.

## cpp-libp2p (Soramitsu)

[`github.com/libp2p/cpp-libp2p`](https://github.com/libp2p/cpp-libp2p). C++17 implementation.

- **License:** Apache-2.0.
- **Steward:** Soramitsu (Japanese blockchain consultancy).
- **Production use:** **KAGOME** — Soramitsu's Polkadot/Substrate parachain runtime in C++.

Smaller community than the top-five; feature parity is decent for the protocols KAGOME needs but lags behind go/rust/js on newer specs (WebTransport, IDONTWANT).

## py-libp2p

[`github.com/libp2p/py-libp2p`](https://github.com/libp2p/py-libp2p). Python implementation.

- **License:** MIT.
- **Status:** README says "v1.0 Coming Soon"; libp2p.io site has it labeled the same.
- **History:** Originally driven by Ethereum Foundation's **Trinity** client effort (Ethereum 1.0 in Python). Trinity was discontinued in 2021, leaving py-libp2p without a primary downstream. The repo entered a maintenance lull.
- **Recovery:** ~2024–25 the Ethereum Foundation funded resumed development. Recent commits are coming through but the "v1.0" milestone is years overdue.

Production use: limited. Some research projects and Python-side education materials use py-libp2p; no shipping at-scale apps depend on it.

## Cross-implementation feature parity

A simplified matrix. ✅ = production; 🟡 = partial / experimental; ❌ = missing.

| Feature | go | rust | js | nim | jvm | cpp | py |
|---|---|---|---|---|---|---|---|
| TCP transport | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| QUIC (RFC 9000) | ✅ | ✅ | ✅ | ✅ | 🟡 | 🟡 | ❌ |
| WebSocket | ✅ | ✅ | ✅ | ✅ | ✅ | 🟡 | ✅ |
| WebTransport | ✅ | 🟡 (client only) | ✅ | ❌ | ❌ | ❌ | ❌ |
| WebRTC | ❌ | 🟡 (alpha) | ✅ | ❌ | ❌ | ❌ | ❌ |
| WebRTC-Direct | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Noise XX | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| libp2p-TLS | ✅ | ✅ | ✅ | ✅ | ✅ | 🟡 | 🟡 |
| yamux | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Kademlia DHT | ✅ | ✅ | ✅ | 🟡 | 🟡 | ✅ | 🟡 |
| Gossipsub v1.1 | ✅ | ✅ | ✅ | ✅ | ✅ | 🟡 | 🟡 |
| Gossipsub v1.2 IDONTWANT | ✅ | ✅ | ✅ | ✅ | 🟡 | ❌ | ❌ |
| Circuit Relay v2 | ✅ | ✅ | ✅ | ✅ | ✅ | 🟡 | 🟡 |
| DCUtR hole punching | ✅ | ✅ | ✅ | ✅ | 🟡 | ❌ | ❌ |
| AutoNAT | ✅ | ✅ | ✅ | ✅ | 🟡 | ❌ | ❌ |
| mDNS | ✅ | ✅ | ✅ | ✅ | 🟡 | ❌ | ❌ |
| Identify / identify-push | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | 🟡 |
| Rendezvous | ✅ | ✅ | 🟡 | ❌ | ❌ | ❌ | ❌ |

The takeaway: **go-libp2p has every production feature; rust-libp2p is close behind**; **js-libp2p has WebRTC** (uniquely); **nim has Eth2-relevant subset**; **jvm has Eth2-Teku-required subset**; cpp/py are partial.

## Interop testing

[`libp2p/test-plans`](https://github.com/libp2p/test-plans) runs a continuous cross-implementation test matrix. The pattern:

- Each tested protocol has a "test plan" — a YAML describing a test scenario.
- The runner spins up Docker containers of different implementation versions and runs the scenario.
- Results published as a dashboard at <https://github.com/libp2p/test-plans>.

This is **rare and valuable** for a multi-implementation P2P stack. Most other multi-impl protocols (MLS, Matrix, ActivityPub) rely on bilateral hand-rolled testing. libp2p's investment in a continuous interop CI is a notable engineering choice — and a model worth borrowing.

The [universal-connectivity](https://github.com/libp2p/universal-connectivity) demo app is the canonical "every implementation talking to every other implementation" showcase: Go + Rust + JS all chatting via gossipsub over QUIC + WebRTC + WebTransport.

## Implications for Myrhiza

- **Implementation diversity is the load-bearing reason libp2p has spec rigor.** When seven implementations have to interop, the spec has to be precise. Myrhiza is single-impl-by-default (Rust kernel, jco-compiled browser kernel) — we don't get this discipline for free. Myrhiza's spec discipline must be self-imposed; we should not assume "code is the spec" works for cross-peer interop.
- **The "ChainSafe maintains the JS gossipsub" pattern is informative.** When a sub-protocol grows complex enough to warrant a dedicated team, it makes sense to fork ownership. Myrhiza's per-app behavior components could follow the same pattern — the core kernel ships the lattice, but specialized behaviors (a CRDT engine, an MLS layer, a Tor netlayer) can be team-owned outside the kernel.
- **`test-plans` is the right shape for any future Myrhiza cross-impl story.** If Myrhiza ever ships a non-Rust kernel (compiled C, Swift, Kotlin), a `test-plans`-style continuous-interop CI is the only realistic way to keep them honest. Bake the cost in from day one if it ever becomes a real possibility.
- **Watch the rust-libp2p release cadence.** The 11-month gap between 0.56 (2025-06-27) and master 0.57.0 is unusual. If Myrhiza ever revisits the iroh-vs-libp2p choice, the rust-libp2p maintenance velocity is a real signal — the project is healthy on commits-per-month but quiet on releases.
- **License-of-record varies.** Don't assume "libp2p is MIT" — the JS ecosystem is dual Apache-2.0/MIT, the Java/cpp implementations are Apache-2.0 only, and `@chainsafe/libp2p-gossipsub` is single-license Apache-2.0. If Myrhiza ever vendors any libp2p crate (even just for reference), check the license per-crate.

## Sources

- [libp2p/go-libp2p](https://github.com/libp2p/go-libp2p) — verified version v0.48.0 (2026-03-17), MIT, Go 1.25.7
- [libp2p/rust-libp2p](https://github.com/libp2p/rust-libp2p) — verified workspace at 0.57.0 master / 0.56.0 crates.io, MIT, rust 1.88
- [libp2p/js-libp2p](https://github.com/libp2p/js-libp2p) — verified npm `libp2p@3.3.1`, dual Apache-2.0/MIT
- [vacp2p/nim-libp2p](https://github.com/vacp2p/nim-libp2p) — verified version 1.15.3, **Apache-2.0 OR MIT (dual)**
- [libp2p/jvm-libp2p](https://github.com/libp2p/jvm-libp2p) — **MIT + Apache-2.0** (Permissive License Stack)
- [libp2p/cpp-libp2p](https://github.com/libp2p/cpp-libp2p) — **Apache-2.0 OR MIT (dual)**
- [libp2p/py-libp2p](https://github.com/libp2p/py-libp2p) — MIT
- [libp2p/test-plans](https://github.com/libp2p/test-plans) — cross-impl interop CI
- [libp2p/universal-connectivity](https://github.com/libp2p/universal-connectivity) — multi-impl demo
- [`libp2p` crate on crates.io](https://crates.io/crates/libp2p)
- [`libp2p` package on npm](https://www.npmjs.com/package/libp2p)
- [`@chainsafe/libp2p-gossipsub` on npm](https://www.npmjs.com/package/@chainsafe/libp2p-gossipsub)
- [libp2p production-ready implementations](https://libp2p.io/) — homepage
