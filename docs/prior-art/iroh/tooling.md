**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — tooling, CLIs, support libraries

The big surprise here, coming from a Holochain perspective: **iroh has no `iroh` CLI**. The library itself is the product. Tooling lives outside the main workspace as separate crates / repos, each one a small focused user-facing app built on top of the iroh `Endpoint`. The shape is more "small reusable Unix utilities" than "vertically integrated dev platform."

## There is no `iroh` CLI

The `iroh` workspace ([n0-computer/iroh](https://github.com/n0-computer/iroh)) ships five crates: `iroh`, `iroh-base`, `iroh-relay`, `iroh-dns`, and `iroh-dns-server`. None of them produce a binary called `iroh`. The reference URL `github.com/n0-computer/iroh/tree/main/iroh-cli` returns 404 — there used to be an `iroh-cli` (pre-0.30, when the project still tried to be a complete IPFS-style stack), but it was deleted in the 0.30 → 0.90 reorg when the team retrenched to "iroh is just the connection primitive." See [`./architecture.md`](./architecture.md) for that history.

What you get instead is a set of independent CLI tools, each pinned to a specific iroh version, each `cargo install`-able:

| Tool | Repo | Latest (May 2026) | Purpose |
|---|---|---|---|
| [`sendme`](https://crates.io/crates/sendme) | [n0-computer/sendme](https://github.com/n0-computer/sendme) | `0.34.0` | Send a file or directory between two machines via a one-shot ticket; uses `iroh-blobs` for BLAKE3 verified streaming + resume. The flagship demo. |
| [`dumbpipe`](https://crates.io/crates/dumbpipe) | [n0-computer/dumbpipe](https://github.com/n0-computer/dumbpipe) | `0.37.0` | `netcat` over iroh — pipes stdio between two endpoints. `brew install dumbpipe` works. |
| [`iroh-doctor`](https://crates.io/crates/iroh-doctor) | [n0-computer/iroh-doctor](https://github.com/n0-computer/iroh-doctor) | `0.99.1` | Network-diagnosis tool: probe relays, run NAT-class test, measure RTT, run a self-connectivity check. The closest thing to an "iroh CLI." |
| [`iroh-relay`](https://crates.io/crates/iroh-relay) | [main repo, `iroh-relay/`](https://github.com/n0-computer/iroh/tree/main/iroh-relay) | `1.0.0-rc.0` | The relay server itself, ships as a binary when built with `--features=server`. |
| [`callme`](https://github.com/n0-computer/callme) | n0-computer/callme | active May 2026 | P2P audio calls — example app, not a daily-driver tool. |

`cargo install` is the canonical install path for all of these. There are no signed Homebrew taps, no `.deb`/`.rpm`, no Windows installers from n0 (homebrew-core does carry `dumbpipe`, community-maintained). GitHub release artifacts publish for `iroh-relay` and `iroh-dns-server` on every `iroh` release — see the [v1.0.0-rc.0 release page](https://github.com/n0-computer/iroh/releases/tag/v1.0.0-rc.0) for the platform matrix (linux x64/arm64, macOS x64/arm64, windows x64).

## `iroh-relay` — the relay server

The relay is a public WebSocket-over-HTTPS server that brokers traffic when hole-punching fails. n0 runs the production fleet at `*.relay.iroh.network`. The full server is a Rust binary built from [`iroh/iroh-relay`](https://github.com/n0-computer/iroh/tree/main/iroh-relay) with `cargo build --release --features=server --bin iroh-relay`. Configuration is a TOML file with fields for HTTP/HTTPS ports, TLS cert paths (manual or Let's Encrypt via [`tokio-rustls-acme`](https://github.com/n0-computer/tokio-rustls-acme)), QUIC address-discovery (QAD) port, optional metrics endpoint, and access-control hooks. The [`iroh-relay/README.md`](https://github.com/n0-computer/iroh/blob/main/iroh-relay/README.md) walks the full self-host flow including a `--dev` mode for local HTTP-only testing and a self-signed-cert path for QAD.

There is also a **library mode**: enabling the `test-utils` feature on the `iroh` crate gives you `iroh::test_utils::run_relay_server()`, which spawns an in-process relay with a self-signed cert. Used widely in iroh's own integration tests; useful for app integration tests that want a hermetic network. See [`./testing.md`](./testing.md).

A Dockerfile lives in the [`docker/`](https://github.com/n0-computer/iroh/tree/main/docker) directory of the main repo. Self-hosting is genuinely supported, not aspirational.

## Test & bench tools

There is **no `iroh-test` crate** and no `iroh-bench` published crate. What exists:

- **[`iroh/bench/`](https://github.com/n0-computer/iroh/tree/main/iroh/bench)** — an unpublished workspace member (`publish = false` in its `Cargo.toml`) producing a `clap`-based benchmark binary. Used in CI; not a user-facing tool.
- **[`patchbay`](https://github.com/n0-computer/patchbay)** (`0.5.2` on crates.io, May 2026) — Linux network-namespace simulator. The de-facto integration-test substrate (see [`./testing.md`](./testing.md)).
- **[`chuck`](https://github.com/n0-computer/chuck)** — "A place to chuck in various integration type tests and benchmarks." Out-of-tree integration tests + netsim configs. Last touched February 2026.
- **`continuous-perf` / `iroh-perf`** — the public continuous benchmarking dashboard at [perf.iroh.computer](https://perf.iroh.computer/) (linked from the `iroh` README), driven by the [`iroh/bench`](https://github.com/n0-computer/iroh/tree/main/iroh/bench) binary.

No `madsim`-style deterministic simulator. No `loom` test suite as of 1.0-rc.0 — there's an `iroh_loom` cfg gate in `Cargo.toml` but no loom tests have shipped yet ([source](https://github.com/n0-computer/iroh/blob/main/Cargo.toml)).

## n0-spec — does not exist

There is **no public `n0-spec` repository** as of May 2026. Searching `github.com/n0-computer` returns no match; the URL [`github.com/n0-computer/n0-spec`](https://github.com/n0-computer/n0-spec) is 404. Protocols are documented as Rust doc comments and ad-hoc design docs in the iroh repo (e.g. [`TRANSPORTS.md`](https://github.com/n0-computer/iroh/blob/main/TRANSPORTS.md)) and in blog posts at [iroh.computer/blog](https://www.iroh.computer/blog). For the relay protocol specifically, the wire format is defined in [`iroh-relay/src/protos/relay.rs`](https://github.com/n0-computer/iroh/blob/main/iroh-relay/src/protos/relay.rs) — code-as-spec. There are no formal RFCs, no version tags on protocols, no test vectors hosted independent of the implementation.

This is a deliberate choice consistent with the project's stage. It is also a real gap if you need to write a second implementation. **Iroh is currently a single-implementation protocol.**

## Adjacent Number 0 crates worth knowing

The org page is large — [over 100 repos](https://github.com/n0-computer) — and most of it is exploratory or app-specific. The currently-load-bearing utility crates (all on crates.io, all updated April–May 2026):

| Crate | Purpose |
|---|---|
| [`noq`](https://crates.io/crates/noq) `1.0.0-rc.0` | n0's QUIC implementation, forked from Quinn. Iroh's transport. See [`./architecture.md`](./architecture.md). |
| [`n0-future`](https://crates.io/crates/n0-future) `0.3.2` | Re-exports of futures/streams/timers picked by n0. Runtime-agnostic, WASM-compatible. Stable but pre-1.0. |
| [`n0-error`](https://crates.io/crates/n0-error) `1.0.0-rc.0` | Error library with call-site location tracking. The whole stack uses it. |
| [`n0-snafu`](https://github.com/n0-computer/n0-snafu) `0.3.x` | Thin wrapper around `snafu` for n0's conventions. |
| [`n0-watcher`](https://github.com/n0-computer/n0-watcher) `0.3.x` | Async value-change observer; used for dynamic relay/endpoint state. |
| [`irpc`](https://crates.io/crates/irpc) | Streaming RPC over QUIC. Used by `iroh-blobs`, `iroh-docs`. |
| [`bao-tree`](https://crates.io/crates/bao-tree) | BLAKE3 verified-streaming primitives. Underlies `iroh-blobs`. |
| [`iroh-metrics`](https://crates.io/crates/iroh-metrics) `1.0.0-rc.0` | Prometheus-compatible metrics. |
| [`net-tools`](https://github.com/n0-computer/net-tools) | Cross-platform networking utilities (interface enumeration, route lookup, etc.). |
| [`patchbay`](https://crates.io/crates/patchbay) `0.5.2` | Linux netns testbed (see above). |
| [`n0-mainline`](https://github.com/n0-computer/n0-mainline) | BitTorrent Mainline DHT implementation, used by the experimental DHT-discovery transport. |

Note the absence of standalone `pkarr` and `mainline` from the n0 org — they consume the upstream [`pkarr` crate](https://crates.io/crates/pkarr) (from [pubky.org](https://pubky.org)) for public-key-addressable records, and have their own [`n0-mainline`](https://github.com/n0-computer/n0-mainline) fork rather than depending on a third-party Mainline crate. Pkarr is **upstream-as-dependency**, mainline is **forked-and-owned**.

## Implications for Myrhiza

For a Myrhiza spec author thinking about tooling and operator surfaces:

- **The "no `iroh` CLI" pattern is suggestive, not prescriptive.** Iroh treats the CLI as an app on top of the runtime rather than part of it (`sendme`, `dumbpipe`, `iroh-doctor` ship separately). For Myrhiza, this argues that the kernel binary is one thing and the operator/dev CLIs are another — but it's a question for the Myrhiza ops spec, not a settled answer.
- **Diagnostic tooling carries its own product weight.** `iroh-doctor` is the tool every iroh user reaches for when "it doesn't work." Whether Myrhiza ships a similar NAT-probe + relay-reachability + connectivity-test surface, and how soon, is a real ops decision; debugging-without-one is meaningfully worse, but the priority depends on Myrhiza's deployment model.
- **`n0-future`, `n0-error`, `irpc` move with iroh.** They are transitive dependencies the Myrhiza Cargo graph will inherit. Pin them in lockstep; don't mirror or fork without a specific reason.
- **`iroh-relay` is genuinely operable** (TLS, ACME, dev mode, library mode, Dockerfile). Whatever public infrastructure Myrhiza commits to operating will need similar operator polish; iroh's relay binary is reasonable prior art for "what a self-hostable Rust relay looks like."
- **The absence of `n0-spec` is a planning problem.** Iroh has no formal protocol-spec repo; the relay wire format lives in [`iroh-relay/src/protos/relay.rs`](https://github.com/n0-computer/iroh/blob/main/iroh-relay/src/protos/relay.rs) and the rest is "whatever the Rust source does." A Myrhiza spec author committing against an iroh subsystem must author Myrhiza's own wire spec for the boundary at which Myrhiza touches that subsystem; track the relay-protocol source-of-truth file the same way one would track an RFC.
- **No deterministic-simulator culture upstream.** Iroh's tests run real `tokio` + real wall clocks + real networks; there is no `madsim` / loom-style determinism gate. Myrhiza must build its own determinism infrastructure above iroh — see [`./testing.md`](./testing.md).

## Sources

- [iroh repository](https://github.com/n0-computer/iroh)
- [iroh README](https://github.com/n0-computer/iroh/blob/main/README.md)
- [iroh v1.0.0-rc.0 release](https://github.com/n0-computer/iroh/releases/tag/v1.0.0-rc.0)
- [iroh-relay README](https://github.com/n0-computer/iroh/blob/main/iroh-relay/README.md)
- [iroh-relay on crates.io](https://crates.io/crates/iroh-relay)
- [TRANSPORTS.md](https://github.com/n0-computer/iroh/blob/main/TRANSPORTS.md)
- [sendme](https://github.com/n0-computer/sendme), [dumbpipe](https://github.com/n0-computer/dumbpipe), [iroh-doctor](https://github.com/n0-computer/iroh-doctor)
- [patchbay](https://github.com/n0-computer/patchbay), [chuck](https://github.com/n0-computer/chuck)
- [n0-future](https://crates.io/crates/n0-future), [n0-error](https://crates.io/crates/n0-error), [irpc](https://crates.io/crates/irpc)
- [n0-mainline](https://github.com/n0-computer/n0-mainline)
- [iroh.computer/blog](https://www.iroh.computer/blog)
- [n0-computer org page](https://github.com/n0-computer)
