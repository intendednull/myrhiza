**Date:** 2026-05-21
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-5 — Coexistence acceptance test with a second WASM state-apply

# Plan B-5 design — Two-app coexistence acceptance test

## 1. Goal + scope correction

The post-B-4 gap analysis (`docs/reports/2026-05-21-mvp-gap-analysis.md`) listed mvp.md §15.1 criterion 2 (multi-peer convergence on same WASM bytes) as 🟡 partial. **This was wrong**: `helpers::counter_handle()` (used by every B-1 + B-4 convergence test) loads the real `counter-state-apply.wasm` fixture via `WasmtimeBackend::instantiate_state_apply` — it is NOT an in-Rust native helper. Every existing convergence test already proves criterion 2 against real WASM bytes. Criterion 2 is ✅ shipped.

The actual next-priority gap is criterion 4: **"A second app instance (different state component, different topic) coexists on the same peer; events do not cross."** The existing `coexistence_two_topics_no_event_crossing` test (`crates/kernel/tests/convergence.rs:206`) uses the SAME counter app on both topics — it proves topic isolation but not the "different state component" half of the criterion.

B-5 closes criterion 4 by:

1. Building a second WASM state-apply fixture (`tests/fixtures/echo-state-apply/`) with semantics distinct from counter.
2. Adding the `just build-fixtures` recipe for it.
3. Adding a `test-utils` helper to install + instantiate it as a `StateApplyHandle`.
4. Writing a coexistence acceptance test that spawns TWO runtimes on the SAME peer with DIFFERENT bundles on DIFFERENT topics; asserts events authored on one runtime don't appear on the other.
5. Correcting the gap analysis to reflect criterion 2's ✅ status.

**Out of scope (still deferred to later slices)**:

- Counter `propose` + `interaction` components (criterion 3 — B-7).
- Poll app (B-6).
- `examples/` workspace member layout (B-8).
- SDK ergonomics (B-8).
- iroh-blobs distribution (B-10).

## 2. Scope decisions (locked during brainstorming + counter-fixture survey, 2026-05-21)

| Decision | Chosen | Runner-up | Why |
|---|---|---|---|
| **Echo over Poll as the second app** | New fixture `echo-state-apply` with semantics: state = the most recently applied event payload (genesis sets it; each subsequent event overwrites). | (a) Build the full Poll app (item 16 from implementation.md §20); (b) Trivial constant-state app (always returns `[]`). | Poll requires multi-author vote-tally logic and is much larger than just "second state component." (b) doesn't exercise the state-transition path. Echo is minimal but real: state actually CHANGES on each event, so coexistence asserts both "B's topic events don't reach A's runtime" AND "A's events that DO reach A produce the expected state." Builds in <50 LoC of state-apply. Poll lands as a separate slice (B-6 or B-7). |
| **Echo fixture as a sibling Cargo project**, not a workspace member | Same pattern as `counter-state-apply-fixture`: separate Cargo.toml under `tests/fixtures/echo-state-apply/`, excluded from the workspace via the root `Cargo.toml`'s `workspace.exclude` list. | Add to workspace. | The counter fixture is intentionally NOT a workspace member because it builds for `wasm32-unknown-unknown` and needs different lint defaults (no-std + manual bump allocator + float-Display avoidance). The new echo fixture inherits the same constraints; matching the existing pattern is the obviously-correct call. |
| **Reuse the counter fixture's WIT verbatim** | `tests/fixtures/echo-state-apply/wit/world.wit` is identical to the counter fixture's WIT. Re-declares `myrhiza:kernel@1.0.0` package with the `state-apply` world. | Share via a relative include. | wit-bindgen 0.30 requires the WIT to live inside the fixture crate. Duplication is the existing pattern; not worth restructuring for this slice. |
| **Echo state-apply ABI: state = payload bytes** | On `apply(prior, event)`: decode `Event` from canonical bincode; for genesis (seq=1), state = `GenesisV1::app_payload`; for non-genesis, state = `event.payload`. `state-digest`: SHA-256 over state bytes (matches counter's pattern). | Constant state, append-only log, etc. | "State is the most recent event's payload" is the simplest non-trivial state transition. The state-digest changes with each event, so convergence is observable. Crucially, the GENESIS application matches counter's pattern (extracts `GenesisV1.app_payload`), so the fixture exercises the same Genesis decode path the counter does. |
| **`test-utils::bundle::build_signed_echo_bundle()` mirrors `build_signed_counter_bundle`** | New helper in `crates/test-utils/src/bundle.rs` that builds a signed bundle from the echo fixture (analogous to the existing counter helper). Uses the same manifest shape (helpers-only state-apply manifest). | Pass paths to a generic builder. | The counter pattern is well-established (5 fixtures, all with their own helpers). One more helper is mechanical; introducing a generic builder is unnecessary refactoring for this slice. |
| **`helpers::echo_handle()` mirrors `counter_handle()`** | New helper in `crates/kernel/tests/helpers/mod.rs` returning a `StateApplyHandle` backed by a fresh wasmtime instance of the echo fixture. | Pass instances as test arguments. | Existing tests use module-scoped helpers; matches the pattern. |
| **Coexistence test = same peer, two runtimes** | New test in `crates/kernel/tests/coexistence.rs` (or extend `convergence.rs`). Construct one `PeerKeypair`; spawn two `Runtime` instances with different state-apply handles (counter + echo) on different topics. Author an event on each; assert the OTHER runtime's digest does NOT change and the OTHER runtime's dropped_at_apply / peer_warnings counts stay clean. | Same test file as `coexistence_two_topics_no_event_crossing`. | A new file under `coexistence.rs` makes the intent obvious and gives the criterion-4 test its own home. The existing test stays as the "different peers, different topics, same app" check — both are valuable. |
| **No new `examples/` directory** | Build a fixture, not an example. Examples land in B-8 with the SDK + dependency-direction CI check. | Build `examples/echo/` now. | Premature; examples have additional constraints (dep direction, manifest signing flow, doc surface) we don't need for this slice. The fixture path is well-trodden. |
| **Gap analysis correction is a single-paragraph edit in the existing report** | Amend `docs/reports/2026-05-21-mvp-gap-analysis.md` inline to note criterion 2's actual ✅ status; remove the now-stale "B-5: counter app full + multi-peer convergence" section; update the recommendation to point to B-6 (poll app) as the next-after-B-5 slice. | Write a separate "correction" report. | Single source of truth for the roadmap. Inline correction with date-tagged note is sufficient. |

## 3. Code surface

### 3.1 New fixture: `tests/fixtures/echo-state-apply/`

Files to create:

- `tests/fixtures/echo-state-apply/Cargo.toml` — mirrors counter's, only `name` differs.
- `tests/fixtures/echo-state-apply/Cargo.lock` — `cargo build` will generate.
- `tests/fixtures/echo-state-apply/wit/world.wit` — verbatim copy of counter's WIT.
- `tests/fixtures/echo-state-apply/src/lib.rs` — no-std state-apply with semantics:
  ```text
  apply(prior, event_bytes):
      decode Event from event_bytes
      if event.seq == 1:
          decode GenesisV1 from event.payload
          return (Accept, GenesisV1.app_payload)
      else:
          return (Accept, event.payload)
  state-digest(state):
      return blake3(state) [32 bytes]
  ```

The lib.rs structure copies the counter's bump allocator + wit-bindgen scaffolding, replaces only the `apply` body.

### 3.2 Justfile recipe — `Justfile`

Add an entry for echo to the existing `build-fixtures` recipe. Identical shape to the counter entry (`cargo build` + `wasm-tools component embed` + `wasm-tools component new`).

Output: `tests/fixtures/built/echo-state-apply.wasm`.

### 3.3 test-utils bundle helper — `crates/test-utils/src/bundle.rs`

```rust
/// Build + sign + write a bundle containing the echo state-apply
/// fixture. Returns the bundle blob + `BundleAddress`. Mirrors
/// `build_signed_counter_bundle`. Used by the coexistence acceptance
/// test in B-5.
pub fn build_signed_echo_bundle() -> (Bundle, BundleAddress) {
    // ... mechanical mirror of build_signed_counter_bundle ...
}
```

The implementation reuses `helpers_only_state_apply_manifest()` (existing) — echo declares the same capability surface as counter (just `host.hash` + `host.log` deterministic imports).

### 3.4 Kernel test helper — `crates/kernel/tests/helpers/mod.rs`

```rust
/// Install + instantiate the echo-state-apply fixture and return a
/// fresh `StateApplyHandle`. Each call returns an independent wasmtime
/// instance with its own Store. Per B-5 spec §3.4.
#[must_use]
pub fn echo_handle() -> StateApplyHandle {
    let (_bundle, addr) = build_signed_echo_bundle();
    let flow = InstallFlow::new();
    let loaded = flow.load(&addr).expect("InstallFlow::load");
    let backend = WasmtimeBackend::new().expect("WasmtimeBackend::new");
    let instance = backend
        .instantiate_state_apply(&loaded.component_bytes, &loaded.manifest)
        .expect("instantiate_state_apply");
    StateApplyHandle::new(instance)
}
```

### 3.5 Acceptance test — `crates/kernel/tests/coexistence.rs` (new file)

```rust
//! B-5 acceptance: same peer, two runtimes, different bundles, different topics.
//! Closes mvp.md §15.1 criterion 4.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

// ... imports ...

mod helpers;

/// Covers: mvp.md §15.1 #4, convergence.md §4.6 — two-app coexistence on the same peer (B-5).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_apps_coexist_no_event_crossing() {
    let bus = MemBus::new(256);

    // Same physical peer keypair; two separate Runtime instances.
    let peer_kp_counter = PeerKeypair::deterministic(501);
    let peer_kp_echo = PeerKeypair::deterministic(501); // SAME pubkey

    let app_bundle_counter = BundleHash::from_bytes([0xC1; 32]);
    let app_bundle_echo    = BundleHash::from_bytes([0xE1; 32]);

    let topic_counter = Topic::derive(&app_bundle_counter, &[0x11; 32], "main");
    let topic_echo    = Topic::derive(&app_bundle_echo,    &[0x22; 32], "main");
    assert_ne!(topic_counter, topic_echo);

    // Both MemNetworks share the same bus + peer_pubkey (since they are
    // the same peer); the bus's request-handler registry collides on
    // peer_pubkey, but the LAST install wins per B-4.4 contract — only
    // the echo runtime's handler is reachable post-startup. That is
    // acceptable for this test because we never issue request_heads
    // against either runtime; the assertion is on no-event-crossing
    // via the gossip path.
    let net_counter = MemNetwork::new(bus.clone(), peer_kp_counter.public);
    let net_echo    = MemNetwork::new(bus.clone(), peer_kp_echo.public);

    let runtime_counter = Runtime::start(
        net_counter,
        topic_counter,
        app_bundle_counter,
        "main".into(),
        helpers::counter_handle(),
        peer_kp_counter,
        Some(AuthorKeypair::deterministic(501)),
        fast_cfg(),
    ).await.expect("counter runtime");

    let runtime_echo = Runtime::start(
        net_echo,
        topic_echo,
        app_bundle_echo,
        "main".into(),
        helpers::echo_handle(),
        peer_kp_echo,
        Some(AuthorKeypair::deterministic(502)),
        fast_cfg(),
    ).await.expect("echo runtime");

    // Author genesis + an event on each runtime.
    author_genesis_then_event(&runtime_counter, /* counter increment */).await;
    author_genesis_then_event(&runtime_echo, /* echo payload */).await;

    // Wait for both runtimes to settle their own state.
    let counter_digest_eventual = /* expected counter state digest */;
    let echo_digest_eventual    = /* expected echo state digest */;
    runtime_counter.digest_watch.changed_to(&counter_digest_eventual).await;
    runtime_echo.digest_watch.changed_to(&echo_digest_eventual).await;

    // ASSERTION: the runtimes' digests are DIFFERENT and STABLE. Crucially,
    // if events were crossing, the counter runtime would try to apply an
    // echo-shaped event and either drop_at_apply or surface a peer warning.
    // Both `dropped_at_apply` should be empty and `peer_warnings` should
    // have no SignatureInvalid / DecodeFailed entries for the OTHER topic.
    assert!(runtime_counter.dropped_at_apply.lock().unwrap().is_empty());
    assert!(runtime_echo.dropped_at_apply.lock().unwrap().is_empty());

    // The two digests must NOT be equal (they're entirely independent
    // state spaces).
    assert_ne!(
        runtime_counter.digest_watch.borrow().as_slice(),
        runtime_echo.digest_watch.borrow().as_slice(),
        "two-app coexistence: each runtime must have its own digest",
    );
}
```

The exact assertion shape needs adapting to the actual `RuntimeHandle` field names — the spec is the design, not the literal code; see Task plan for the concrete walkthrough.

### 3.6 Gap analysis correction — `docs/reports/2026-05-21-mvp-gap-analysis.md`

Inline edit:

1. In the §"v1 acceptance criteria status" table, change criterion 2 from 🟡 to ✅ with the corrected justification: "Existing convergence tests use `helpers::counter_handle()`, which loads the real `counter-state-apply.wasm` fixture via `WasmtimeBackend::instantiate_state_apply` — every B-1 + B-4 convergence test already runs on real WASM bytes."
2. In the §"Proposed slice sequence" section, replace the B-5 entry with the new B-5 = "two-app coexistence test." Update the recommendation paragraph to point to B-5 (coexistence) as next, and re-anchor the rest of the sequence (B-6 = poll app, B-7 = interaction harness, etc.) accordingly.
3. Append a dated correction note at the end of the doc explaining the original error + when it was caught.

## 4. Acceptance tests

The new `crates/kernel/tests/coexistence.rs::two_apps_coexist_no_event_crossing` IS the acceptance test (§3.5).

Plus a smoke test for the echo fixture itself in `crates/kernel/tests/acceptance.rs` (or as a new test in `coexistence.rs`):

```rust
/// Smoke test: echo-state-apply.wasm loads, instantiates, applies a
/// genesis + one increment, produces a state digest. Mirrors the
/// existing kernel_instantiates_and_applies_increment test but for
/// the echo fixture.
#[test]
fn kernel_instantiates_and_applies_echo() { /* ... */ }
```

## 5. Justfile changes

Add `echo-state-apply` to the `build-fixtures` recipe alongside the existing fixtures.

## 6. Edge cases

| Scenario | Behavior |
|---|---|
| Echo's `apply` returns a state of arbitrary length | Acceptable; the state-digest is over the bytes regardless. |
| Both runtimes try to publish on the bus simultaneously | The bus broadcasts to topic-subscribers only. Each runtime's subscription is per-topic; no cross-pollination is possible at the bus layer. |
| MemBus's request-handler registry collision on same peer_pubkey | Last install wins per B-4.4 contract. The test doesn't issue `request_heads` against either runtime, so this is benign. Document inline. |
| Echo's genesis: `GenesisV1::app_payload` empty | State after genesis = empty bytes. Subsequent events overwrite. |
| Subsequent events with payload = previous state | State doesn't change; digest doesn't change. Acceptable. |

## 7. Surface change summary

**New crate-public surface**:

- `myrhiza_test_utils::bundle::build_signed_echo_bundle()` — new helper.

**New test surface**:

- `crates/kernel/tests/helpers/mod.rs::echo_handle()` — new test helper.
- `crates/kernel/tests/coexistence.rs` — new test file.

**Unchanged**:

- `Network`, `Runtime`, kernel, runtime, all production surfaces.
- `WasmtimeBackend`, `InstallFlow`, `StateApplyHandle` — unchanged; echo uses the existing APIs.

## 8. Non-goals (explicit)

- Counter `propose` / `interaction` components (criterion 3) — B-7.
- Full Poll app — B-6.
- `examples/` workspace member — B-8.
- SDK ergonomics — B-8.
- Iroh-blobs distribution path — B-10.
- v1.1 behavior profile — post-v1.
- Cross-process tests — post-v1.

## 9. Prior-art consultation

- `tests/fixtures/counter-state-apply/` — the canonical fixture template; echo mirrors its structure.
- `crates/kernel/tests/convergence.rs::coexistence_two_topics_no_event_crossing` (line 206) — the existing test that proves same-app two-topic isolation. B-5's test proves the harder property: two-app same-peer isolation.
- `mvp.md §15.1` — defines criterion 4 precisely as "different state component, different topic."
- `mvp.md §15.4` workspace shape — confirms `tests/fixtures/` is the right home for fixtures vs `examples/`.

## 10. Future work — explicit deferrals

- **B-6** — Poll app (item 16 from implementation.md §20).
- **B-7** — Counter `propose` + `interaction` components + native CLI harness (criterion 3).
- **B-8** — SDK ergonomics + `examples/` layout + dep-direction CI.
- **B-9 (optional)** — Storage layer.
- **B-10 (optional)** — Iroh-blobs distribution.

## 11. Sources

- `tests/fixtures/counter-state-apply/src/lib.rs` — template for echo's lib.rs.
- `tests/fixtures/counter-state-apply/Cargo.toml` — template for echo's Cargo.toml.
- `tests/fixtures/counter-state-apply/wit/world.wit` — verbatim WIT.
- `crates/test-utils/src/bundle.rs::build_signed_counter_bundle` — template for `build_signed_echo_bundle`.
- `crates/kernel/tests/helpers/mod.rs::counter_handle` (line 33) — template for `echo_handle`.
- `crates/kernel/tests/convergence.rs::coexistence_two_topics_no_event_crossing` (line 206) — sibling test for two-topic isolation.
- `crates/kernel/tests/acceptance.rs::kernel_instantiates_and_applies_increment` (line 92) — template for echo smoke test.
- `Justfile` — `build-fixtures` recipe.
- `docs/reports/2026-05-21-mvp-gap-analysis.md` — gap analysis being corrected.
