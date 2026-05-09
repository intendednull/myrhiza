# Testing & dev infrastructure

Holochain ships four overlapping test harnesses and one continuous performance observatory. The redundancy is historical — each was built when the previous one couldn't reach a particular regime — and all four are still maintained.

## The CLI: `hc`

The [`hc` CLI](https://blog.holochain.org/hc-cli-test-run-and-package-your-happ/) is the developer entry point. The relevant subcommands:

| Command | What it does |
|---|---|
| `hc dna pack` / `hc app pack` / `hc web-app pack` | Bundle compiled WASM + manifest into `.dna`, `.happ`, `.webhapp` artifacts |
| `hc sandbox create` / `generate` | Build a conductor config (optionally with hApps installed) in a temp dir; path persisted in a `.hc` file in CWD |
| `hc sandbox run` | Boot the configured conductors with admin/app websocket ports (passed via `-r 0,0,0` for auto-select) |
| `hc sandbox call` | Drive admin APIs from the shell |
| `hc sandbox list` / `clean` | Manage active sandboxes |
| `hc launch` | Live-reloading dev runner (UI + conductor together) |
| `hc run-local-services` (legacy / pre-0.5) | Bootstrap+signal locally; superseded by `kitsune2-bootstrap-srv` |

There is **no `hc test`**. Testing is delegated to Tryorama or Sweettest below; `hc` only handles packaging and sandbox lifecycle ([hc_sandbox README](https://github.com/holochain/holochain/blob/develop/crates/hc_sandbox/README.md), [holochain_cli_sandbox docs](https://docs.rs/holochain_cli_sandbox/latest/holochain_cli_sandbox/)).

## Tryorama — JS scenario harness

[Tryorama](https://github.com/holochain/tryorama) is the official TypeScript/JS multi-conductor harness, distributed as `@holochain/tryorama`. The model is: a **Scenario** owns a set of **Players**, each Player is a `(conductor, agent, installed hApp)` triple. Tryorama spawns conductors as child processes on the local machine, wires up admin/app websockets, and exposes the high-level Holochain client API. It does not own time — tests poll for consistency using helper functions like `dhtSync(players)`, which busy-waits on `get_agent_activity` matching across all players. Time control is by polling, not by deterministic stepping.

Tryorama overrides several Holochain defaults that are inappropriate for tests: `initiateJitterMs`, `roundTimeoutMs`, `transportTimeoutS` are tightened so tests don't sit through 5-minute gossip back-offs. The package is **no longer actively maintained for 0.7+** by the core team; a community fork at `holochain-open-dev/tryorama` continues development.

### TryCP — remote conductor orchestration

[Tryorama Control Protocol](https://blog.holochain.org/introducing-tryorama-control-protocol-trycp/) is an open WebSocket protocol that lets a Tryorama client drive conductor processes on **remote machines**. Each remote runs a `trycp_server`; the Tryorama runner spawns/kills/configures conductors via RPC. This powers tests over real internet links instead of localhost.

## Sweettest — Rust scenario harness

[`holochain::sweettest`](https://docs.rs/holochain/latest/holochain/sweettest/) is the in-tree Rust harness, used by the Holochain team itself for both unit and integration tests. The primitives are `SweetConductor`, `SweetConductorBatch` (multi-conductor cluster in one process), `SweetAgents`, `SweetCell`, and `SweetConductorConfig`. Because it is in-process, sweettest can directly inspect conductor internals (validation queues, source chains, op caches) — Tryorama can't, since it talks only to the websocket API. `SweetConductorConfig` exposes presets for offline / low-connectivity / no-network scenarios used to test local-only and partition-recovery code paths. Community helper crate [`sweettest-utils`](https://github.com/ddd-mtl/sweettest-utils) has common consistency-await helpers. Sweettest is the recommended path when you need test access to internal conductor state; Tryorama is the recommended path for app developers who only see the public API.

## Wind Tunnel — continuous performance observatory

[Wind Tunnel](https://github.com/holochain/wind-tunnel) is the production-readiness performance harness, [matured in 2025](https://blog.holochain.org/wind-tunnel-seeing-inside-the-storm/) into a continuous observatory. Each scenario is a Rust binary linking the `holochain_wind_tunnel_runner` library; **23 scenarios** cover app installation, zome calls, DHT sync, validation receipts, remote signals, countersigning. Workflow:

1. GitHub Actions builds scenarios on every merge to `main`.
2. The "Run performance tests on Nomad cluster" workflow deploys to a [HashiCorp Nomad cluster](https://nomad-server-01.holochain.org:4646/ui) of geographically distributed machines (with `canonical-scaled` variants spinning up Threefold nodes for larger runs).
3. **Telegraf** collects host OS metrics (CPU, memory, disk, network); the conductor pushes its own metrics via the `HOLOCHAIN_INFLUXIVE_FILE` env var; the runner emits scenario-level metrics (call latency, DHT sync lag, throughput).
4. All metrics land in [InfluxDB at `ifdb.holochain.org`](https://ifdb.holochain.org/); per-run summaries published to [the GitHub Pages dashboard](https://holochain.github.io/wind-tunnel/).

Reference numbers from the [Substack writeup](https://happeningscommunity.substack.com/p/wind-tunnel-testing-holochain-at): zome call ~4 ms, DHT sync 27–60 s depending on cluster size, remote signal round-trip 33 ms, write-and-read 54/sec, two-party countersigning 2.4 s, app install <100 ms. Side-by-side version comparison is on the roadmap but not yet shipped — version selection is per-run, not visualized as A/B.

## Logging and tracing

Holochain is instrumented with [`tokio-tracing`](https://github.com/tokio-rs/tracing). Two filter envs:

- `RUST_LOG` — controls conductor and Rust-side logs (standard `tracing-subscriber` syntax).
- `WASM_LOG` — same syntax, but applied inside the wasmer host to filter `debug!`/`info!` calls inside zomes.

Span-based tracing makes async causality readable across the gossip/validation pipeline. No first-party OpenTelemetry exporter; teams who want one wire their own subscriber.

## Common dev failure modes

- **Compile times.** A clean Holochain build is multi-tens-of-minutes; dev loops rely on incremental builds and on `nix`-cached toolchains (Holonix). Compile latency is the most-cited friction in community channels.
- **Tryorama startup races.** Historical bug: the Tryorama client tried to connect before Holochain fully booted, producing flaky tests. Fixed by parsing the conductor's stdout startup string before opening the websocket.
- **Gossip-timing flakes.** Tests that wait for DHT consistency hit gossip back-off windows; the fix is the `initiateJitterMs`/`roundTimeoutMs` overrides Tryorama applies by default, plus `await dhtSync(players)` polling helpers. The harnesses do **not** offer logical-clock control — there is no "advance time by N seconds" primitive — so tests that depend on schedulers (e.g. validation deadlines) must wall-clock-wait.
- **Network forks across versions.** Because kitsune2 was wire-incompatible with kitsune1, every project re-deployed at 0.5; mismatched dev/prod conductor versions silently fail to find each other rather than erroring. The recommended discipline is pinning conductor versions per network.
- **No deterministic scheduler.** Both Tryorama and Sweettest run real `tokio` runtimes with real wall clocks. Time-based test determinism is consequently brittle and a known gap relative to systems like Madsim or Loom.

## Implications for Myrhiza

- **Pick one harness, not four.** The redundancy here is technical debt — Tryorama (JS), Sweettest (Rust), TryCP (remote), Wind Tunnel (perf) is a maintenance tax. A unified harness with explicit "in-process" / "multi-process" / "multi-host" modes is cheaper.
- **Build deterministic time control from day 0.** Holochain's lack of a virtual scheduler is the root cause of most flake. Adopt `tokio::time::pause` or a Madsim-style simulator early so consistency tests are reproducible.
- **Continuous perf-on-main from week 1.** Wind Tunnel landed years after kitsune was already shipped; performance regressions in kitsune1 lived for months because nothing watched main. A 5-scenario perf gate on every merge catches regressions before they entrench.
- **Sandbox CLI is the right shape.** `hc sandbox` is genuinely good ergonomics; the `(create | generate | run | call | clean)` verbs map onto P2P dev work cleanly. Worth borrowing the verb shape.

## Sources

- [hc CLI: Test, Run, and Package Your hApp](https://blog.holochain.org/hc-cli-test-run-and-package-your-happ/)
- [hc_sandbox README](https://github.com/holochain/holochain/blob/develop/crates/hc_sandbox/README.md)
- [holochain_cli_sandbox docs](https://docs.rs/holochain_cli_sandbox/latest/holochain_cli_sandbox/)
- [Tryorama repository](https://github.com/holochain/tryorama)
- [Introducing Tryorama Control Protocol (TryCP)](https://blog.holochain.org/introducing-tryorama-control-protocol-trycp/)
- [holochain::sweettest module docs](https://docs.rs/holochain/latest/holochain/sweettest/)
- [sweettest-utils helper crate](https://github.com/ddd-mtl/sweettest-utils)
- [Wind Tunnel repository](https://github.com/holochain/wind-tunnel)
- [Wind Tunnel: Seeing Inside the Storm](https://blog.holochain.org/wind-tunnel-seeing-inside-the-storm/)
- [Dev Pulse 152: Wind Tunnel Updates](https://blog.holochain.org/dev-pulse-152-wind-tunnel-updates-holo-edge-node-container/)
- [Wind Tunnel: Testing Holochain at Scale (Substack)](https://happeningscommunity.substack.com/p/wind-tunnel-testing-holochain-at)
- [tokio-tracing](https://github.com/tokio-rs/tracing)
