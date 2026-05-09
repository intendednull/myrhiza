**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — test infrastructure, determinism (or lack of it), interop

## Three layers of tests

Iroh's tests fall into three categories, each running in different CI workflows:

1. **In-process unit + integration tests** — `cargo test` / `cargo nextest`, run by [`.github/workflows/ci.yml`](https://github.com/n0-computer/iroh/blob/main/.github/workflows/ci.yml) and [`tests.yaml`](https://github.com/n0-computer/iroh/blob/main/.github/workflows/tests.yaml). Standard Rust, real `tokio` runtime.
2. **Network-namespace tests** — built on [`patchbay`](https://github.com/n0-computer/patchbay), run by [`patchbay.yml`](https://github.com/n0-computer/iroh/blob/main/.github/workflows/patchbay.yml) on self-hosted Linux runners.
3. **Netsim** — out-of-tree perf/regression scenarios in [`n0-computer/chuck`](https://github.com/n0-computer/chuck), driven by [`netsim.yml`](https://github.com/n0-computer/iroh/blob/main/.github/workflows/netsim.yml) on every push to `main`.

Plus a separate `flaky.yaml` workflow that re-runs known-flaky tests on a schedule, and a `wine.yaml` for Windows-via-Wine smoke tests. The CI footprint is large for a pre-1.0 library and is a real strength.

## Patchbay — Linux netns network simulation

[`patchbay`](https://github.com/n0-computer/patchbay) is the heart of iroh's hole-punching test infrastructure. It builds realistic network topologies out of Linux network namespaces, with `veth` pairs, `nftables` rules for NAT policies, and `tc qdisc` for link conditioning (latency, loss, bandwidth). Each node gets its own private network stack, runs unprivileged via user namespaces, and tears down when the `Lab` is dropped.

A `Lab` builder API lets a test express:

```rust
let dc = lab.add_router("dc").preset(RouterPreset::Public).build().await?;
let home = lab.add_router("home").preset(RouterPreset::Home).build().await?;
let dev = lab.add_device("laptop").iface("eth0", home.id()).build().await?;
dev.iface("eth0")?.set_condition(LinkCondition::Wifi, LinkDirection::Both).await?;
```

…and then run real iroh `Endpoint`s inside each device namespace. This is qualitatively different from a mock-the-network pattern — it exercises the real socket code, real QUIC stack, real hole-punching path against real NAT.

Iroh's patchbay-driven tests live in [`iroh/tests/patchbay/`](https://github.com/n0-computer/iroh/tree/main/iroh/tests/patchbay):

- `nat.rs` — NAT-traversal scenarios across cone / symmetric / hairpinning topologies.
- `degrade.rs` — connection behavior under packet loss and bandwidth caps.
- `switch-uplink.rs` — endpoint mobility / network-change handling.

Patchbay-on-CI runs on self-hosted Linux runners (`runs-on: [self-hosted, linux, X64]`) because user-namespace setup needs `kernel.apparmor_restrict_unprivileged_userns=0` and unprivileged-userns-create — most cloud CI doesn't allow it. This is the operational tax of running real network stacks in CI.

## Netsim / chuck — perf + regression scenarios

The [`chuck`](https://github.com/n0-computer/chuck) repo ("a place to chuck in various integration type tests and benchmarks") owns longer-running scenarios. The [`netsim.yml`](https://github.com/n0-computer/iroh/blob/main/.github/workflows/netsim.yml) workflow runs on every push to `main`, generates report tables, and (for release branches) pushes results to a public dashboard. Output ends up at [perf.iroh.computer](https://perf.iroh.computer/) (the continuous-perf URL referenced in the iroh README) — performance regressions on `main` are caught before they merge into a release.

This is **continuous performance observation on every push**, which is unusual for a P2P stack and a real differentiator. Compare Holochain's Wind Tunnel, which only matured in 2025 long after kitsune was already shipped.

## Property tests — present, narrow

`proptest` is in the dependency tree of `iroh-base` and `iroh-relay` ([`iroh-base/Cargo.toml`](https://github.com/n0-computer/iroh/blob/main/iroh-base/Cargo.toml), [`iroh-relay/Cargo.toml`](https://github.com/n0-computer/iroh/blob/main/iroh-relay/Cargo.toml)). The actual proptest blocks are scoped to the relay wire-protocol codec — see [`iroh-relay/src/protos/relay.rs`](https://github.com/n0-computer/iroh/blob/main/iroh-relay/src/protos/relay.rs) and the regression seeds in [`iroh-relay/proptest-regressions/protos/`](https://github.com/n0-computer/iroh/tree/main/iroh-relay/proptest-regressions/protos). That covers `relay.txt` and `send_recv.txt` — the round-trip codec for the relay's frame format.

What property tests **don't** cover: the endpoint state machine, the discovery flow, the path-selection logic, the QUIC transport (in `noq`). Those are tested with example-based tests, in-process integration tests, and patchbay scenarios — no fuzzed state-space exploration.

## Fuzz tests — absent

There is **no `fuzz/` directory** in the iroh repo and no `cargo-fuzz` targets registered. Given the QUIC transport now lives in `noq` and noq is a fork of Quinn, some inherited fuzz coverage exists upstream — but no n0-authored fuzzers ship with iroh as of 1.0-rc.0. For a wire-protocol library at this maturity, this is a real gap. (Worth tracking: noq may grow fuzz targets independently in [n0-computer/noq](https://github.com/n0-computer/noq).)

## Loom tests — staged, not landed

The workspace `Cargo.toml` declares an `iroh_loom` cfg gate ([`Cargo.toml`](https://github.com/n0-computer/iroh/blob/main/Cargo.toml#L42)) and excludes it from `unexpected_cfgs` warnings. **No `iroh_loom` test modules ship in 1.0-rc.0.** The infrastructure is there; the tests are not. This means concurrency invariants (e.g. the connection-establishment race window, the path-selection synchronization) are not model-checked.

## Determinism — none

Iroh makes **no determinism claims**. Every test runs against a real `tokio` runtime with real wall clocks and real socket I/O. There is no virtual scheduler, no `madsim` integration, no `tokio::time::pause` discipline in the test suite. The integration test at [`iroh/tests/integration.rs`](https://github.com/n0-computer/iroh/blob/main/iroh/tests/integration.rs) candidly notes:

> "At the moment, these tests unfortunately interact with deployed services, specifically the 'real' DNS server infrastructure and 'real' relays."

That is, the integration suite reaches out to the n0-staging relay fleet and to n0's pkarr DNS servers. Tests are not hermetic. The CI workflow exports `IROH_FORCE_STAGING_RELAYS=1` so the real production fleet isn't loaded down.

This is appropriate for a network library — non-determinism is intrinsic to the problem domain — but it's **fundamentally incompatible with what Myrhiza needs from `state-apply`**. Myrhiza's state-apply components must be deterministic functions of `(prior state, event)`, and iroh's testing tells us nothing about whether iroh-mediated event delivery preserves the inputs to that function. Iroh guarantees "you'll get the bytes through eventually"; it does not guarantee "every peer sees the same byte stream in the same order." Ordering and convergence are Myrhiza's job, not iroh's.

## Conformance / interop tests — none

There is no test-vector fixture directory, no cross-implementation test suite, no n0-spec conformance harness (because n0-spec doesn't exist — see [`./tooling.md`](./tooling.md)). The relay protocol is tested by sending bytes from the iroh client to the iroh server, using the same Rust crate on both ends. No JS or Go reference implementation exists to test against. **Iroh is currently a single-implementation protocol**, and its test suite reflects that.

The `archived` tag on [`n0-computer/test-plans`](https://github.com/n0-computer/test-plans) (last update January 2025) is the historical artifact of an attempt to participate in libp2p-style cross-implementation interop tests. That effort has been abandoned.

## CI infrastructure — well-engineered

From [`.github/workflows/`](https://github.com/n0-computer/iroh/tree/main/.github/workflows):

- `ci.yml` — main pipeline: clippy/fmt/test on Linux/macOS/Windows, MSRV check, doc check, `cargo deny`.
- `tests.yaml` — extended test matrix, separate from `ci.yml` so failures don't block PR merge on flaky-but-cheap tests.
- `patchbay.yml` — netns tests on self-hosted Linux runners (described above).
- `netsim.yml` / `netsim_runner.yaml` — perf scenarios from `chuck`, runs on `push: main` and reports results.
- `flaky.yaml` — re-runs known-flaky tests on schedule.
- `beta.yaml` — runs against Rust beta to catch upcoming compiler changes.
- `wine.yaml` — Windows-via-Wine smoke tests on Linux.
- `release.yml` — builds `iroh-relay` + `iroh-dns-server` binaries for the [release-page](https://github.com/n0-computer/iroh/releases) artifact set.
- `cleanup.yaml`, `commit.yml`, `docs.yaml`, `docker.yaml`, `pick-runner.yml`, `project_sync.yaml` — ancillary.
- `sccache-probe`, `sccache-action` — build cache to keep compile times manageable.

`sccache` is universal across workflows. MSRV is currently `1.91`, locked in `iroh-relay/Cargo.toml`. `RUSTFLAGS=-Dwarnings` is set everywhere — zero-warning discipline across the matrix.

## Implications for Myrhiza

- **Iroh's tests don't exercise our determinism invariants.** State-apply purity, event-order convergence, and cross-peer state hashing are *not* tested by anything iroh ships. Myrhiza must build its own test harness — at minimum a multi-node simulator with deterministic time and message-delivery control — and iroh test infrastructure stops at the connection layer.
- **Adopt `patchbay` for the connection-layer tests we still need.** When Myrhiza needs to verify "this protocol still works under a symmetric NAT," `patchbay` gives us that without reinventing it. Its API is small and its license (Apache-2.0/MIT) is compatible.
- **Build a deterministic-time test substrate for the kernel.** Use `tokio::time::pause` discipline + a virtual-clock test harness. The target Holochain failed to hit; iroh hasn't even attempted it. If Myrhiza wants reproducible state-apply tests across releases, the time substrate is non-negotiable from week 1.
- **Plan for fuzz coverage of our own wire formats.** Iroh's relay protocol is proptested; iroh's QUIC (now noq) inherits fuzz coverage from upstream Quinn but has no dedicated fuzz suite. Anything Myrhiza defines as a wire format — capability tickets, event envelopes, manifest formats — needs `cargo-fuzz` targets from day one.
- **Author conformance vectors early.** If Myrhiza ever hopes for a second implementation, freeze test vectors before the first release. Iroh deferred this and is now a single-implementation protocol; that's a structural commitment we should not silently inherit.
- **Continuous-perf-on-main is the right shape.** Iroh's [perf.iroh.computer](https://perf.iroh.computer/) running on every merge is good prior art. A 5-scenario perf gate on every Myrhiza PR catches regressions before they entrench. Holochain shipped this years late; iroh shipped it early; we should ship it from day one.

## Sources

- [iroh CI workflows](https://github.com/n0-computer/iroh/tree/main/.github/workflows)
- [`iroh/tests/integration.rs`](https://github.com/n0-computer/iroh/blob/main/iroh/tests/integration.rs)
- [`iroh/tests/patchbay/`](https://github.com/n0-computer/iroh/tree/main/iroh/tests/patchbay)
- [patchbay repo](https://github.com/n0-computer/patchbay)
- [chuck repo](https://github.com/n0-computer/chuck)
- [iroh-relay proptest source](https://github.com/n0-computer/iroh/blob/main/iroh-relay/src/protos/relay.rs)
- [iroh-relay proptest-regressions](https://github.com/n0-computer/iroh/tree/main/iroh-relay/proptest-regressions)
- [`Cargo.toml` workspace lints + cfg gates](https://github.com/n0-computer/iroh/blob/main/Cargo.toml)
- [perf.iroh.computer](https://perf.iroh.computer/)
- [test-plans (archived)](https://github.com/n0-computer/test-plans)
- [noq (n0's QUIC)](https://github.com/n0-computer/noq)
