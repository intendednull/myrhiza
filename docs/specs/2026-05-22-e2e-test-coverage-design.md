**Date:** 2026-05-22
**Status:** draft
**Parent:** [implementation.md §20 item 19](2026-05-09-myrhiza-master-design/implementation.md), [mvp.md §15.1](2026-05-09-myrhiza-master-design/mvp.md)
**Subject:** Plan E2E-1 — close the in-process e2e coverage gap (real-iroh kernel convergence + coexistence + binary-shellout CLI)

# Plan E2E-1 — Close the in-process e2e coverage gap

## 1. Goal

Today the v1 acceptance bar is mechanically met on real WASM bytes through the real backend through the real kernel, but the network underneath is `MemNetwork` (an in-memory broadcast bus) for every convergence and coexistence test, and the CLI is exercised by calling the `myrhiza_cli::run` library function rather than the actual binary. [reports/2026-05-21-mvp-gap-analysis.md item 19](../reports/2026-05-21-mvp-gap-analysis.md) marks the E2E test suite as ❌ with the rationale "B-4.4 acceptance tests use in-process two-`IrohNetwork`-peers (sufficient for protocol shape but not a true E2E)."

Plan E2E-1 closes the load-bearing portion of that gap:

1. Wires real `IrohNetwork` (already feature-gated, already validated at the network-protocol layer in `crates/network/tests/iroh_gossip.rs`) through real `Runtime` through real WASM state-apply, mirroring the existing `MemNetwork`-backed convergence and coexistence tests.
2. Adds binary-shellout tests for `myrhiza-cli` that exercise the clap parser, `main()`, stdio handling, and `--author-seed` derivation path — the surface library tests cannot reach.

Cross-OS-process iroh tests (two binaries handshaking over loopback UDP) and state-tier per-app tests are deferred to a follow-up spec (E2E-2). See §6.

## 2. Design choices (summary)

**Choice A — Add a kernel-tuned `IrohPeerStack` to `myrhiza-test-utils`; do NOT touch the existing network-level test files.** Two `spawn_iroh_peer` helpers already exist in the workspace, with different signatures:

- [`crates/network/tests/iroh_gossip.rs:50`](../../crates/network/tests/iroh_gossip.rs) — `spawn_iroh_peer(lookup: &MemoryLookup)`; registers `iroh_gossip::ALPN` only.
- [`crates/network/tests/direct_streams_iroh.rs:66`](../../crates/network/tests/direct_streams_iroh.rs) — `spawn_iroh_peer(lookup: &MemoryLookup, register_heads_alpn: bool)`; conditionally registers `HEADS_REQUEST_ALPN` for direct-stream backfill.

Kernel convergence tests need the second shape: the `iroh_late_joiner_backfills_via_heads_summary` test depends on the Runtime issuing `request_heads` (B-4.4 / B-4.5), which only works if both peers register the heads-request ALPN. The new `crates/test-utils/src/iroh_harness.rs` therefore exposes a `spawn_iroh_peer` taking `register_heads_alpn: bool`, behaviorally a copy of the `direct_streams_iroh.rs` variant.

We accept the ~30 LOC of duplication between the network-tests files and the new test-utils helper, rather than introducing a `network → test-utils` dev-dep cycle. `test-utils` already depends on `network` ([`crates/test-utils/Cargo.toml:18`](../../crates/test-utils/Cargo.toml)); having `network/tests/*.rs` import from `test-utils` would form a dev-dep cycle (legal in Cargo, but a maintenance trap — every reader has to chase one direction or the other and discover the cycle). Duplication is the smaller cost.

Runner-up: form the dev-dep cycle to share one helper. Rejected — saves ~30 LOC at the cost of inverting one of the workspace's cleanest dep-direction invariants. Future readers tracing the network crate's test surface would have to chase the cycle back through test-utils to understand the helper's lifecycle.

**Choice B — Add `IrohHarness` as a parallel fixture to `InProcessHarness`, not as a replacement.** The two harnesses have the same shape but different lifecycle constraints. `InProcessHarness` is sync to construct (just a `MemBus`); `IrohHarness` is async because each peer's endpoint binds a UDP socket. Tests choose by topology. `InProcessHarness` stays the default for the determinism-property tests (where the bus is the simpler model); `IrohHarness` is what closes the network-realism gap.

Runner-up: parameterize one harness over `Network`. Rejected — `MemNetwork::new(bus, peer_pubkey)` and `IrohNetwork::new(endpoint, gossip)` take fundamentally different constructor arguments. A generic harness would need a `NetFactory` trait that both peer types implement; the abstraction earns nothing in tests and obscures the per-test wiring that is exactly the thing under test. Two siblings is clearer than one generic.

**Choice C — Hand-roll subprocess wiring via `std::process::Command` + `env!("CARGO_BIN_EXE_myrhiza-cli")`; do NOT add `assert_cmd`.** Three binary-shellout tests do not justify the transitive dependency footprint of `assert_cmd`. The crate pulls in `bstr`, `predicates`, `predicates-core`, `predicates-tree`, `wait-timeout`, `difflib`, and a transitive `regex-automata` — none currently in the workspace's `Cargo.lock`. The ergonomic win is real but small (~10 lines saved per test), and the workspace has zero existing precedent for `assert_cmd`.

Hand-rolled `Command::new(env!("CARGO_BIN_EXE_myrhiza-cli"))` produces a `Child` we can `stdin().write_all(...)` to and `wait_with_output()` against; assertions on `status.code()`, `stdout`, `stderr` are stdlib. The repeated setup is ~12 lines per test, factored into one helper in the test module.

Runner-up: add `assert_cmd` for ergonomics. Rejected — for three tests, transitive-dep cost > ergonomic win. If the binary-test count grows past ~6, reconsider.

**Choice D — `IrohHarness::spawn_peer` accepts the same shape of inputs as `InProcessHarness::spawn_peer`, plus a bootstrap-peer hint when constructing the second peer.** Mirroring the existing harness API keeps the diff between MemNetwork-backed and IrohNetwork-backed tests minimal: peer A subscribes with empty bootstrap, peer B subscribes with peer A's pubkey as bootstrap. The iroh-gossip swarm-formation latency (~200ms for two peers per `crates/network/tests/iroh_gossip.rs:133`) is absorbed by `PeerHandle::await_digest`'s existing 5-second timeout — no new timing knobs.

**Choice E — Iroh-backed tests are feature-gated `network-iroh` and live in dedicated test files.** Per the existing convention (`crates/kernel/tests/attribution.rs` already has `#[cfg(feature = "network-iroh")]` test 11-12), new iroh-backed convergence and coexistence tests go in `crates/kernel/tests/iroh_convergence.rs` and `crates/kernel/tests/iroh_coexistence.rs`. The `MemNetwork`-backed sibling files remain unchanged. The `Justfile`'s `test-iroh` recipe gets extended from `cargo test -p myrhiza-network --features network-iroh --tests` to also include the kernel iroh-feature tests.

Runner-up: mingle iroh tests into the existing convergence.rs / coexistence.rs files behind `#[cfg(feature = "network-iroh")]`. Rejected — the existing files are already long (1017 + 465 lines) and have well-established test patterns; commingling would force every reader of those files to mentally feature-toggle. Separate files keep blast radius small.

**Choice F — Determinism implications: iroh-backed tests are NOT strict determinism tests; they are network-realism integration tests.** `state-apply` and the topological-sort path remain deterministic functions; the harness around them is what differs. iroh-gossip's swarm formation, NAT traversal stubs, and Plumtree forwarding introduce timing nondeterminism (when does the receiver see a message? after one hop or two? in this run order or that?), but the *eventual convergence* property the tests assert is unchanged. Use `await_digest` with bounded timeouts as the convergence oracle; never assert on message-arrival ordering or per-step intermediate state.

This matches the [CLAUDE.md](../../CLAUDE.md) rule "Determinism is a load-bearing property" applied at the right layer: state-apply is deterministic; integration tests about *the system* are realism tests, not determinism tests.

**Choice G — Cross-OS-process iroh tests are explicitly deferred.** The current gap analysis ranks item 19 as ❌ in part because no test spawns two OS processes that converge over real iroh. Closing this requires either (i) a test-only driver binary that takes scripted commands or (ii) extending `myrhiza-cli` with a "network mode." Both are larger than the in-process gap closure and have their own design questions (subprocess discovery, port allocation, log capture). E2E-1 explicitly does NOT include this work; spec E2E-2 (TBD) will. See §6.

## 3. Architecture

### 3.1 Coverage matrix — what each test layer proves

| Layer | Test file | Real WASM? | Network realism? | Cross-process? | Determinism oracle | Currently in tree? |
|---|---|---|---|---|---|---|
| Backend instantiation | `wasmtime-backend/tests/*.rs` | ✅ | n/a | n/a | direct-call assertion | ✅ |
| Kernel + state-apply | `kernel/tests/acceptance.rs` | ✅ | n/a | n/a | direct-call assertion | ✅ |
| Kernel + MemNetwork convergence | `kernel/tests/convergence.rs` | ✅ | 🟡 in-memory bus | ❌ | `await_digest` (timeout-bounded) | ✅ |
| Kernel + MemNetwork coexistence | `kernel/tests/coexistence.rs` | ✅ | 🟡 in-memory bus | ❌ | `await_digest` | ✅ |
| **Kernel + IrohNetwork convergence** | **`kernel/tests/iroh_convergence.rs`** | **✅** | **✅ real iroh QUIC** | **❌ in-proc** | **`await_digest`** | **❌ → E2E-1** |
| **Kernel + IrohNetwork coexistence** | **`kernel/tests/iroh_coexistence.rs`** | **✅** | **✅ real iroh QUIC** | **❌ in-proc** | **`await_digest`** | **❌ → E2E-1** |
| CLI library (lib::run) | `myrhiza-cli/tests/e2e.rs` | ✅ | n/a | 🟡 single-proc | direct-call return | ✅ |
| **CLI binary (subprocess)** | **`myrhiza-cli/tests/cli_binary.rs`** | **✅** | **n/a** | **✅ binary subproc** | **exit code + captured stdout/stderr** | **❌ → E2E-1** |
| CLI cross-process over iroh | (driver binary) | ✅ | ✅ | ✅ | TBD | ❌ → E2E-2 |

The "Determinism oracle" column makes Choice F explicit: state-apply is deterministic by spec, but at the integration layer the test asserts on an *eventually-converged* digest (timeout-bounded `await_digest`), not on per-step intermediate values or message arrival order. This matches the column's reading across all rows.

Three new test files, one new harness module, three test-utils additions (feature, helper, re-export). No production code changes.

### 3.2 Test-utils additions

```rust
// crates/test-utils/src/iroh_harness.rs (NEW — feature-gated `network-iroh`)

#![cfg(feature = "network-iroh")]

use std::sync::Arc;

use iroh::address_lookup::MemoryLookup;
use myrhiza_kernel::identity::{AuthorKeypair, PeerKeypair};
use myrhiza_kernel::runtime::{Runtime, RuntimeCfg};
use myrhiza_kernel::state_apply::StateApplyHandle;
use myrhiza_network::{IrohNetwork, RequestHandler, HEADS_REQUEST_ALPN};
use myrhiza_types::{BundleHash, PeerPubkey, Topic};

/// One iroh peer's underlying stack. Holds the endpoint + gossip + router
/// alongside the IrohNetwork handle the Runtime uses. Ownership lives on
/// the harness so endpoints are not dropped mid-test (a drop tears down
/// the UDP socket and silently breaks every running peer).
pub struct IrohPeerStack {
    pub endpoint: iroh::Endpoint,
    pub gossip: iroh_gossip::Gossip,
    pub router: iroh::protocol::Router,
    pub network: IrohNetwork,
}

/// Spawn one iroh peer with the iroh-gossip ALPN registered, and (if
/// `register_heads_alpn`) the heads-request direct-stream ALPN too. The
/// heads-request handler is the `IrohNetwork::protocol_handler()`, which
/// shares state via Arc with the `IrohNetwork` we return — so a later
/// `network.install_request_handler(...)` call wires the actual responder.
///
/// Mirrors `crates/network/tests/direct_streams_iroh.rs::spawn_iroh_peer`
/// (~30 LOC duplicated by Choice A above; the alternative was a dev-dep cycle).
pub async fn spawn_iroh_peer(
    lookup: &MemoryLookup,
    register_heads_alpn: bool,
) -> IrohPeerStack { /* ... */ }

/// Multi-peer fixture for iroh-backed convergence + coexistence tests.
///
/// Owns the shared `MemoryLookup` registry so each spawned peer's address
/// is discoverable by every other peer. Peers are owned by the harness;
/// dropping the harness tears them all down together (avoids the
/// "endpoint died mid-test" hazard from manual lifecycle management).
///
/// **Constructor difference from `InProcessHarness`:** there is no
/// `bus_capacity` arg — iroh has no bus. Otherwise the field set
/// (`app_bundle_hash`, `topic_name`, `seed`, `topic`) is identical so
/// test bodies remain near-identical between MemNetwork and IrohNetwork
/// variants.
pub struct IrohHarness {
    pub lookup: MemoryLookup,
    pub app_bundle_hash: BundleHash,
    pub topic_name: String,
    pub seed: [u8; 32],
    pub topic: Topic,
    peers: Vec<IrohPeerStack>,
}

impl IrohHarness {
    pub fn new(seed: [u8; 32]) -> Self { /* ... */ }

    /// Spawn a peer with the given identity seeds. `bootstrap` is the
    /// pubkey of an already-spawned peer this one should dial; pass empty
    /// for the first peer (it waits for inbound joins). The harness
    /// always registers the heads-request ALPN on every peer — kernel
    /// tests are the only consumer, and they all rely on direct-stream
    /// backfill availability (see Choice A).
    ///
    /// After construction, the Runtime's `install_request_handler` has
    /// already run inside `Runtime::start` — peers are ready to serve
    /// backfill requests without further setup.
    pub async fn spawn_peer(
        &mut self,
        peer_seed: u64,
        author_seed: Option<u64>,
        handle: StateApplyHandle,
        cfg: RuntimeCfg,
        bootstrap: Vec<PeerPubkey>,
    ) -> PeerHandle { /* ... */ }
}
```

The `PeerHandle` type returned is the **existing** `myrhiza_test_utils::harness::PeerHandle` — same author / await_digest / drift_log surface. The only difference between an `InProcessHarness`-spawned peer and an `IrohHarness`-spawned peer is what `Runtime::start` is given for its `Network` parameter.

**Why the harness owns the heads-request ALPN registration:** `Runtime::start` calls `network.install_request_handler(...)` internally (at [`crates/kernel/src/runtime.rs:603`](../../crates/kernel/src/runtime.rs)), and `IrohNetwork::install_request_handler` updates the shared state behind the `protocol_handler()` returned at peer construction time. The Router must already be accepting `HEADS_REQUEST_ALPN` for that handler to receive any inbound streams, which is why the harness registers the ALPN at `spawn_iroh_peer` time, **before** calling `Runtime::start`. This is the load-bearing detail that makes `iroh_late_joiner_backfills_via_heads_summary` work end-to-end.

### 3.3 Iroh convergence tests

`crates/kernel/tests/iroh_convergence.rs` mirrors three of the existing `convergence.rs` tests, gated on `network-iroh`:

| Test | Mirrors | What it proves |
|---|---|---|
| `iroh_single_originator_single_receiver_converges` | `convergence.rs::single_originator_single_receiver_converges` | Real iroh-gossip swarm forms, A's genesis + 3 increments reach B via Plumtree forwarding, B's state-apply replays to the expected digest. Asserts only on B's final converged digest via `await_digest` (5s timeout). |
| `iroh_concurrent_multi_author_converges` | `convergence.rs::concurrent_multi_author_converges` | Two peers each author concurrently after both ingesting genesis; canonical topo-sort yields the same converged state on **both** peers (mirrors the existing test's two `await_digest` calls — peer_a and peer_b must each reach `33_i64.to_be_bytes()`). This is the iroh-backed test most sensitive to gossip-warm-up timing; see §8 on the 500ms pre-publish settling step. |
| `iroh_late_joiner_backfills_via_heads_summary` | `convergence.rs::late_joiner_backfills_via_heads_summary` | The Runtime-issued backfill path (HeadsSummary observed → Runtime issues `request_heads` over real iroh → late-joiner replays) is exercised end-to-end. The direct-stream substrate has been tested in [`direct_streams_iroh.rs`](../../crates/network/tests/direct_streams_iroh.rs) since B-4.4, but the kernel-driven flow that triggers `request_heads` from observing a `HeadsSummary` gap has only been tested via `MemNetwork`. |

We do NOT mirror every convergence test. The other `convergence.rs` tests (drift, equivocation, peer-warning routing) are validated at the protocol layer by [`crates/network/tests/iroh_gossip.rs`](../../crates/network/tests/iroh_gossip.rs) and at the kernel-logic layer by `convergence.rs` (MemNetwork); the path between them is the gap, which one good test from each category covers.

### 3.4 Iroh coexistence tests

`crates/kernel/tests/iroh_coexistence.rs` mirrors [`coexistence.rs::two_apps_coexist_no_event_crossing`](../../crates/kernel/tests/coexistence.rs) verbatim on identity binding — **distinct author keypairs per app** (counter: `AuthorKeypair::deterministic(501)`, echo: `AuthorKeypair::deterministic(502)`), distinct bundle hashes, distinct derived topics. The only structural change from the MemNetwork variant is the network layer:

- MemNetwork variant uses one shared `MemBus` for both topics; the bus routes by topic.
- iroh variant uses **one** `IrohHarness` (and therefore one `MemoryLookup`) — both runtimes' iroh peers live on the same in-process node and join two different iroh-gossip swarms. This mirrors how a real deployment would have one peer participating in two iroh swarms; address-discovery scope is naturally per-process, not per-topic.

The assertions match the MemNetwork variant: counter's state must be `5_i64.to_be_bytes()` after genesis + Increment(+5), echo's state must be `b"hello"` (its genesis app_payload), the two digests must differ, neither runtime's `dropped_at_apply` may contain entries, and neither's `peer_warnings` may surface `SignatureInvalid`.

### 3.5 CLI binary tests

`crates/myrhiza-cli/tests/cli_binary.rs` runs `myrhiza-cli` as an actual subprocess via `std::process::Command::new(env!("CARGO_BIN_EXE_myrhiza-cli"))` (no `assert_cmd`; see Choice C). Each test pipes scripted input to the child's stdin and captures stdout/stderr/exit-code via `Child::wait_with_output()`. Three tests:

| Test | Contract under test |
|---|---|
| `cli_binary_increment_loop_yields_final_state_via_stdout_views` | Binary entrypoint (`main.rs`) wires `--bundle` + `--author-seed` + stdin + stdout correctly. After scripted input `inc 5\ninc 3\nquit\n`, asserts: (a) exit code 0, (b) stdout contains the progressive views `counter: 0\n`, `counter: 5\n`, `counter: 8\n` (matching the existing `e2e.rs::counter_stdout_shows_progressive_views`), (c) stderr contains `final state: [0, 0, 0, 0, 0, 0, 0, 8]` (the `eprintln!` at [`main.rs:38`](../../crates/myrhiza-cli/src/main.rs) — note that "final state" goes to **stderr**, not stdout). Validates clap parsing and that `io::stdout().lock()` does not buffer-swallow the final view. |
| `cli_binary_missing_bundle_exits_nonzero_with_diagnostic` | A `--bundle` path that does not exist must produce a non-zero exit code and a diagnostic on stderr — not a panic, not a hang. The library function returns `Err(_)`; the binary's `?` operator propagates it into `Box<dyn Error>`, which the runtime prints to stderr before exiting non-zero. |
| `cli_binary_dispatch_rejection_does_not_abort_loop` | Mirrors `e2e.rs::counter_dispatch_rejection_does_not_abort_loop` through the binary — a bogus action on stdin must surface `dispatch rejected:` on stdout and the next `inc 1` must still apply (final stderr line shows state `[0,0,0,0,0,0,0,1]`). Validates that the binary's stdin loop survives parse errors. |

No new production dependency. `tempfile` is already in `crates/test-utils/Cargo.toml` — if a test needs a per-process temp bundle path it goes through the existing `myrhiza_test_utils::bundle` builder.

### 3.6 CI + Justfile

Update `Justfile`'s `test-iroh` recipe to include the new kernel + test-utils targets (added lines only — the existing line is unchanged):

```
test-iroh:
    cargo test -p myrhiza-network --features network-iroh --tests
    cargo test -p myrhiza-kernel --features network-iroh --tests
    cargo test -p myrhiza-test-utils --features network-iroh --tests
```

CI's `ci` task already calls `test-iroh`, so the new tests run on every PR. The existing `test` task (which uses no features) skips iroh-feature tests via `#[cfg(feature = "network-iroh")]`, so it does not double-run them.

Binary tests in `myrhiza-cli/tests/cli_binary.rs` run under the default `cargo test`, no feature gate. They depend on the `myrhiza-cli` binary being built; Cargo builds bin targets automatically before running an integration test in the same package, and exposes the resolved path via `env!("CARGO_BIN_EXE_myrhiza-cli")` at test-compile time.

## 4. Slice sequence

E2E-1 lands as ONE PR — the changes are tightly coupled (you cannot meaningfully test the IrohHarness without an iroh-backed test that uses it). Internal task ordering inside the plan document for incremental commits:

1. **T1** — Add `network-iroh` feature to `myrhiza-test-utils` Cargo.toml (gated re-export of `iroh`, `iroh-gossip`, `IrohNetwork`); add empty `iroh_harness` module behind the feature gate; verify `cargo check -p myrhiza-test-utils --features network-iroh` passes.
2. **T2** — Add `spawn_iroh_peer` + `IrohPeerStack` to `crates/test-utils/src/iroh_harness.rs`, modeled on [`crates/network/tests/direct_streams_iroh.rs:66`](../../crates/network/tests/direct_streams_iroh.rs) (which already handles the `register_heads_alpn` branch). **Do not modify** the network-tests files — duplication is accepted per Choice A. Verify `cargo test -p myrhiza-test-utils --features network-iroh --tests` builds.
3. **T3** — Add `IrohHarness::new` + `IrohHarness::spawn_peer`. Smoke test: spawn two peers, both subscribe via Runtime::start, publish a heads_summary from one's `network.publish(...)`, assert the other peer's harness-wrapped receiver decodes it. (Validates the harness in isolation before any kernel-integration test depends on it.)
4. **T4** — Add `crates/kernel/tests/iroh_convergence.rs` with `iroh_single_originator_single_receiver_converges`. Verify pass under `cargo test -p myrhiza-kernel --features network-iroh --test iroh_convergence`.
5. **T5** — Add `iroh_concurrent_multi_author_converges`. Use a 500ms pre-publish settle (matches the three-peer settle in [`iroh_gossip.rs:172`](../../crates/network/tests/iroh_gossip.rs)) since concurrent authoring during gossip warm-up is the most flake-sensitive path.
6. **T6** — Add `iroh_late_joiner_backfills_via_heads_summary`. This is the test that validates the load-bearing detail named at the end of §3.2 (heads-ALPN registered at Router build time → install_request_handler updates shared state → late-joiner's `request_heads` succeeds).
7. **T7** — Add `crates/kernel/tests/iroh_coexistence.rs` with the mirrored two-app coexistence test. Distinct author keypairs per §3.4.
8. **T8** — Add `crates/myrhiza-cli/tests/cli_binary.rs` with `cli_binary_increment_loop_yields_final_state_via_stdout_views`. Use `std::process::Command` (no `assert_cmd` per Choice C); factor the child-spawn boilerplate into one module-local helper.
9. **T9** — Add `cli_binary_missing_bundle_exits_nonzero_with_diagnostic` and `cli_binary_dispatch_rejection_does_not_abort_loop`.
10. **T10** — Update `Justfile`'s `test-iroh` recipe per §3.6. Re-run all of T1-T9 under the updated recipe to confirm no regression.
11. **T11** — *Docs only — must wait until T1–T10 are green in local `just ci`.* Update [reports/2026-05-21-mvp-gap-analysis.md](../reports/2026-05-21-mvp-gap-analysis.md) item 19 status from ❌ to 🟡 (cross-process gap remains, in-process closed), with citation to this spec.

Each task = one commit. The plan document (`docs/plans/2026-05-22-e2e-test-coverage.md`) elaborates each into checkbox sub-steps with exact code.

## 5. Test plan / acceptance

This entire spec IS the test plan — the deliverable is tests. Acceptance is mechanical:

- `just ci` returns exit 0 with zero warnings on the feature branch (this gate includes `fmt-check`, `lint`, `test`, `test-iroh`, and `spec-coverage-check`).
- The three new iroh-feature test files compile under `cargo test --features network-iroh` and each test passes within its own outer `await_digest` timeout.
- `crates/myrhiza-cli/tests/cli_binary.rs` passes under default `cargo test` (no feature gate).
- `tests/spec-coverage.md` regenerates cleanly (existing `spec-coverage-check` gate).
- The gap analysis doc is updated to reflect new state (T11).

## 6. Out of scope (deferred)

Explicitly deferred from E2E-1 — these belong in subsequent specs:

| Item | Where it lands | Why deferred |
|---|---|---|
| **Cross-OS-process iroh convergence** | Spec E2E-2 (TBD; ~1 week after E2E-1 lands) | Requires a test-driver binary or `myrhiza-cli` network-mode flag, plus cross-process address discovery via tempfile / port allocation / log scraping. Independent concern from in-process realism; benefits from a clean slate spec to design subprocess discovery deliberately. |
| **State-tier per-app tests** | Spec E2E-3 (TBD) | Today's tests use the in-Rust `counter_handle()` helper; per-app state-apply unit tests for the counter / echo fixtures would catch state-apply bugs at app level rather than runtime level. Adjacent concern; not what this spec is closing. |
| **Browser-tier tests (jco)** | Blocked on implementation.md item 21 | Cannot proceed until the jco backend exists. |
| **Property-based convergence tests** | Spec E2E-4 (TBD) | The existing fixed-script convergence tests are sufficient for acceptance; property-based generation of event orderings (proptest over shuffle permutations) is a follow-up rigor improvement, not a v1 gap. |

After E2E-1 + E2E-2, item 19's status flips from ❌ to ✅ in the gap analysis.

## 7. Prior art / references

- [`crates/network/tests/iroh_gossip.rs`](../../crates/network/tests/iroh_gossip.rs) — proves the `IrohNetwork` substrate at the protocol layer. The `spawn_iroh_peer` helper this spec promotes was authored and validated in B-4.1; we reuse, not re-design.
- [`crates/test-utils/src/harness.rs`](../../crates/test-utils/src/harness.rs) — `InProcessHarness` + `PeerHandle` is the shape `IrohHarness` mirrors. Same author / await_digest / drift_log surface; only the `Network` parameter to `Runtime::start` differs.
- [`crates/kernel/tests/convergence.rs`](../../crates/kernel/tests/convergence.rs) — the existing MemNetwork-backed convergence tests are the spec for what the iroh-backed tests must assert.
- [`crates/kernel/tests/attribution.rs`](../../crates/kernel/tests/attribution.rs) tests 11–12 — existing precedent for `#[cfg(feature = "network-iroh")]` kernel tests; the kernel crate's Cargo.toml already has the feature pass-through wired.
- [`reports/2026-05-21-mvp-gap-analysis.md`](../reports/2026-05-21-mvp-gap-analysis.md) — item 19's current ❌ status and stated rationale; this spec's success is when that line can flip to 🟡.

External prior-art consulted: [`prior-art/iroh/`](../prior-art/iroh/) for the `MemoryLookup` design — production iroh-net uses the DHT + relay for address discovery; `MemoryLookup` is a test-only shortcut that bypasses both. The kernel-tier iroh tests therefore validate the iroh substrate's protocol shape but not its production discovery path (acknowledged in §8). Test patterns otherwise mirror existing in-tree conventions, and the iroh-net API surface is already understood from the B-4 sequence.

## 8. Honest gaps in this spec

- **In-process iroh does not catch all of what cross-process catches.** Two `IrohNetwork` instances in the same OS process share an executor and a libc; some classes of bug (file-descriptor leaks under heavy load, signal-handling regressions, log-buffer flushing on abnormal exit) only surface across a true process boundary. E2E-1 narrows the gap but does not close it.
- **Iroh's `MemoryLookup` is a test-only address discovery shortcut.** Production peers would use the iroh DHT + relay. Tests do not exercise those paths. (This is a known limitation, not introduced by this spec.)
- **iroh-gossip swarm formation timing is best-effort.** The 200ms sleep convention from `iroh_gossip.rs` is empirical, not principled. Under CI load it may flake; the mitigation is `await_digest` with a 5-second outer timeout. If flakes appear, the fix is to lengthen the timeout, not the sleep — the timeout is the right knob because it bounds total convergence latency, which is what tests actually care about.
