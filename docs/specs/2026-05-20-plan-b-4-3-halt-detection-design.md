**Date:** 2026-05-20
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-4.3 — Transport-error/halt detection (split out from larger B-4.3 scope)

# Plan B-4.3 design — Halt-on-persistent-transport-error

## 1. Goal

Pay off one of the two B-4.1 deferrals that block honest deployment of `IrohNetwork` against real peers: **halt detection on persistent transport errors**.

The B-4.1 implementation of `IrohSubscription::recv` maps `iroh_gossip` mid-stream `ApiError`s to `SubError::Lagged(0)` (per `crates/network/src/iroh_transport.rs:206-226`). The runtime then treats this as a backfill nudge — pushes `PeerWarning::BroadcastLagged` and calls `publish_heads_summary`. If the underlying iroh-gossip actor has DIED (e.g. its endpoint shut down, its dispatcher task panicked), the next `recv` returns the same `ApiError` again, the runtime publishes again, infinite loop. The B-4.1 spec §6 documents this:

> **`ApiError` mid-stream**: mapped to `SubError::Lagged(0)`. The underlying gossip task may have died; B-4.1 will spin if the next `recv()` keeps yielding `ApiError`. Mitigation deferred to B-4.3 (proper halt detection).

B-4.3 fixes the spinning bug. Concrete shape:

1. **New `SubError::TransportError(String)` variant** distinct from `SubError::Lagged(u64)` and `SubError::DecodeFailed { peer }`. `IrohSubscription::recv` now surfaces iroh-gossip mid-stream `ApiError`s as `TransportError`, NOT `Lagged`. The semantic separation matters: `Lagged` is recoverable (publish HeadsSummary, peers resync via backfill); `TransportError` may be fatal (actor dead, endpoint shut down).
2. **Consecutive-error counter** on `Runtime`. Each `TransportError` increments a counter; each successful `recv` resets it. When the counter reaches a configurable threshold (default 5), the runtime signals halt via the existing `halt_watch_tx: watch::Sender<Option<String>>` (per `runtime.rs:379-381`) and exits the recv loop cleanly. The threshold is `RuntimeCfg::transport_error_halt_threshold: usize`.
3. **`MemBus::inject_transport_error()` test affordance** — sibling to the existing `MemBus::inject_lag()` affordance. Sets a per-subscription `force_transport_error: Arc<AtomicBool>` flag that the next `MemSubscription::recv` consumes once, returning `Err(SubError::TransportError("injected by MemBus::inject_transport_error".into()))`. Gated on `feature = "test-helpers"` (same gate as `inject_lag`).
4. **No changes to `IrohNetwork::publish` or the gossip wire format.** This slice is receive-side only.

**Out of scope (deferred to B-4.4 or later)**:

- **HeadsRequest direct-streams** (point-to-point via new ALPN + Router protocol-handler dispatch). Substantial standalone work; B-4.4.
- **Real cross-process tests.** B-4.4 or B-5.
- **Lag-event mapping test** (deferred from B-4.1 §4). Iroh-gossip 0.99.0 doesn't expose `JoinOptions::subscription_capacity` at its public API — forcing broadcast-channel overrun in `IrohSubscription::recv` requires either upstream patching or wrapping the underlying `tokio::sync::broadcast::Receiver` manually. Documented as a known gap; not blocking.
- **Backfilling `PeerWarning::SignatureInvalid` into `process_drift_message`** (drift handler keeps its silent-drop). Pre-existing behavior, no B-4.3 change.
- **Halt-on-decode-error.** `SubError::DecodeFailed` does NOT count toward the consecutive-error threshold — wire-decode failures from one bad peer should not halt the entire runtime, per B-4.1's routing-distinction rationale.

## 2. Scope decisions (locked during brainstorming + B-4.1/4.2 runtime survey, 2026-05-20)

| Decision | Chosen | Runner-up | Why |
|---|---|---|---|
| **Distinct `SubError::TransportError` variant** | New variant; `IrohSubscription::recv` returns this for iroh-gossip mid-stream `ApiError`s; `MemSubscription::recv` returns it when `force_transport_error` is set. Carries a `String` description (e.g. `"iroh-gossip api error: {orig}"`). | (a) Re-use `SubError::Lagged` and gate on consecutive count; (b) Generic `SubError::Fatal(String)` distinguishing recoverable from fatal at the variant level | The Lagged path's "publish HeadsSummary + push BroadcastLagged warning" is structurally wrong for transport errors — publishing more HeadsSummaries doesn't help a dead actor. Re-using Lagged forces every consumer to know about a hidden count-based meaning shift. (a) rejected. (b) is half-right (the semantic IS "fatal vs recoverable") but conflates structural error variants with policy decisions. The runtime's policy is "halt after N consecutive TransportErrors"; that's a runtime concern, not a SubError concern. Variant-level split (`TransportError` distinct from `Lagged`/`DecodeFailed`) leaves variant semantics clean and runtime policy explicit. |
| **Loose-vs-strict consecutive counter** | **Strict consecutive** — counter resets on `Ok(Some(_))` (an actual decoded message arrived). `Ok(None)` exits the recv loop before counter matters (clean stream close). `Err(Lagged)` and `Err(DecodeFailed)` do NOT reset the counter, but they ALSO don't increment it. Only `Err(TransportError)` increments. | (a) Reset on any non-TransportError outcome (treat Lagged/DecodeFailed as "evidence the transport is alive"); (b) Time-windowed counter (decay) | A peer flooding `DecodeFailed` and intermittent `TransportError` shouldn't reset to zero (option (a)) — the underlying transport may genuinely be flaky. Option (b) (time-windowed) adds runtime state without observable benefit at this scope. Strict consecutive is the cleanest "definitely the transport, not the publisher": only `TransportError` is transport-attributable, so only `TransportError` counts. |
| **Threshold default** | `RuntimeCfg::transport_error_halt_threshold: usize = 5`. Configurable per `RuntimeCfg`; tests can use a tight value (e.g. 2) for fast failure. | (a) Hardcoded 5; (b) 3 default | Five gives a small safety margin against transient flakes without long-spinning. Configurable so tests can validate the halt path at threshold=2 without waiting for 5 errors. 3 is too tight for production (one transient hiccup => halt); 10 is too loose (10 seconds × N retries before halt). |
| **Halt signal mechanism** | Reuse the existing `halt_watch_tx: watch::Sender<Option<String>>` at `runtime.rs:381`. On threshold, set `Some(format!("transport halted: {N} consecutive errors"))` and `return Ok(())` from the recv loop (exits the task cleanly). | (a) New halt-signal channel specifically for transport; (b) Panic from the task and let tokio propagate | The existing `halt_watch_tx` is exactly the right shape — `Option<String>` already carries a reason; runtime task already exits cleanly on `return Ok(())`; `RuntimeHandle::halt_watch: watch::Receiver<Option<String>>` already exposed at `runtime.rs:291`. Reusing it preserves the convention. Panic-based halt loses the reason string and may corrupt other tasks. |
| **`MemBus::inject_transport_error` shape** | Sibling to `MemBus::inject_lag`. Per-subscription `force_transport_error: Arc<AtomicBool>` flag in `TopicState::force_transport_error_flags: Vec<Weak<AtomicBool>>`. `MemSubscription::recv` checks the flag first (before checking `force_lag`); if set, swaps to `false` and returns `Err(SubError::TransportError("injected by MemBus::inject_transport_error".into()))`. | (a) Use the existing `inject_lag` and rely on counter behavior; (b) New `MemSubscription` type variant | (a) doesn't actually test TransportError — Lagged is a different error variant. The inject-test-affordance pattern is well-established (see `crates/network/src/memory.rs:91-127` for `inject_lag`); mirroring it for transport errors is one structurally-isomorphic addition. (b) adds a new test type for one method; unnecessary. |
| **`SubError::TransportError` propagation through `NetworkErased`** | Falls out automatically — `NetworkErased<N>::recv` delegates to `inner.recv()` which returns `SubError`. New variant transparently propagates. | Custom mapping | No mapping needed; `SubError` is the unified error type. |
| **Per-iteration check vs per-recv check** | Counter check happens AFTER each `recv` returns. Specifically: after `Err(SubError::TransportError(_))` handler increments the counter; if counter >= threshold, set halt signal and return. After `Ok(Some(_))`, reset counter to 0. `Ok(None)` exits via `return Ok(())` before the counter is touched (clean stream close, no halt). After `Err(SubError::Lagged(_))` or `Err(SubError::DecodeFailed { .. })`, leave counter unchanged. | Hold the counter elsewhere (e.g. `RuntimeHandle`) | Counter is per-runtime-task state; lives in `Runtime` struct. Tested via observable halt signal, not direct counter access. |
| **Lagged vs TransportError on real iroh-gossip** | Iroh-gossip's `Event::Lagged` (the underlying `tokio::sync::broadcast::RecvError::Lagged`-equivalent) stays mapped to `SubError::Lagged(0)` — semantically correct, recoverable via backfill. Iroh-gossip's mid-stream `ApiError` (the `Some(Err(_api_err))` arm of `IrohSubscription::recv`) becomes `SubError::TransportError(format!("iroh-gossip api error: {orig}"))`. | Conflate both into TransportError | The two are semantically distinct: `Event::Lagged` means "broadcast channel overrun, missed N messages" (recoverable); `ApiError` means "the gossip actor reported an error" (may be terminal). Preserving the distinction at the recv layer keeps the runtime's policy clean. |
| **Halt does not abort in-flight publish** | The halt signal is set and the recv loop exits; any in-flight `author` reply or `publish_heads_summary` completes naturally. `Runtime::start` returns `Ok(())` cleanly. | Force-abort in-flight | Forcing abort risks dropping responses to in-flight requests; clean shutdown is more predictable. Halt is observed by `RuntimeHandle::halt_watch.changed().await`. |
| **No re-bind / auto-recovery** | After halt, the runtime task is gone. The embedder must construct a new `Runtime` if recovery is desired. | Auto-restart loop | Auto-recovery hides the transport problem from the embedder. Halt + explicit re-construction matches the existing `RuntimeError::Network` -> halt pattern at `runtime.rs:475-478`. |
| **`SubError::TransportError` carries `String` (not structured fields)** | Description string only. The original `ApiError` is `iroh_gossip::ApiError` (`n0_error`-shaped, not `Clone`); converting to a String at the boundary loses structure but is simpler. | Wrap original error | The runtime treats TransportError as opaque (counts + halts); no consumer needs the structured fields. Match drift's `RuntimeError::Network(String)` pattern. |
| **Test runtime** | `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` for tests that drive `Runtime` with `MemNetwork` (the halt-test path). | Single-threaded | `Runtime::start` spawns a task; multi-threaded ensures the spawn-and-observe-halt-signal cycle isn't blocked by polling discipline. |

## 3. Code surface

### 3.0 `SubError::TransportError` variant — `crates/network/src/lib.rs`

In the existing `enum SubError` (currently `Lagged` + `DecodeFailed`):

```rust
#[derive(Debug, Error)]
pub enum SubError {
    // ... existing Lagged, DecodeFailed unchanged ...

    /// The transport layer (e.g. iroh-gossip's actor) reported an
    /// error mid-stream. Semantically distinct from
    /// [`SubError::Lagged`] (broadcast-channel overrun, recoverable
    /// via backfill) and [`SubError::DecodeFailed`] (wire-byte parse
    /// failure on a single message). May indicate the underlying
    /// transport has DIED; the runtime accumulates these and halts
    /// after `RuntimeCfg::transport_error_halt_threshold` consecutive
    /// occurrences. Per B-4.3 spec §3.0.
    #[error("transport error: {0}")]
    TransportError(String),
}
```

Update `crates/network/src/subscription.rs` rustdoc on `Subscription::recv` to list the new variant:

```rust
/// ... existing doc ...
///
/// - `Err(SubError::TransportError(reason))` — the underlying
///   transport reported an error mid-stream. The runtime counts
///   consecutive occurrences and halts after a configurable
///   threshold; see [`SubError::TransportError`] for the full
///   rationale.
///
/// # Errors
/// `Lagged`, `DecodeFailed`, and `TransportError` are all non-fatal
/// at the trait surface. Policy decisions (backfill vs discard vs
/// halt) live in the consumer (`Runtime`). Per B-4.3 spec §3.0.
```

### 3.1 `IrohSubscription::recv` mapping — `crates/network/src/iroh_transport.rs`

Update the existing `recv` body. Currently (per `iroh_transport.rs:206-226`):

```rust
Some(Err(_api_err)) => {
    // Stream-level error from iroh-gossip mid-flight.
    // ... maps to SubError::Lagged(0)
    return Err(SubError::Lagged(0));
}
```

Replace the `Some(Err(_api_err))` arm with:

```rust
Some(Err(api_err)) => {
    // Stream-level error from iroh-gossip mid-flight. Surface as
    // TransportError (distinct from Lagged); the runtime counts
    // consecutive TransportErrors and halts after a configurable
    // threshold. Per B-4.3 spec §3.0.
    return Err(SubError::TransportError(format!(
        "iroh-gossip api error: {api_err}"
    )));
}
```

Document at the call site that `Event::Lagged` (the iroh-gossip API event variant) STILL maps to `SubError::Lagged(0)` — these are two distinct underlying conditions.

### 3.2 `RuntimeCfg::transport_error_halt_threshold` — `crates/kernel/src/runtime.rs`

Find `RuntimeCfg` (grep for `pub struct RuntimeCfg`). Add a new field:

```rust
pub struct RuntimeCfg {
    // ... existing fields unchanged ...

    /// Number of consecutive `SubError::TransportError` returns from
    /// `Subscription::recv` after which the runtime halts. Default
    /// 5. Set tighter (e.g. 2) in tests to validate the halt path.
    /// Per B-4.3 spec §3.2.
    pub transport_error_halt_threshold: usize,
}
```

Update the `impl Default for RuntimeCfg` (or equivalent default constructor — grep for `RuntimeCfg::default` or `Default for RuntimeCfg`):

```rust
impl Default for RuntimeCfg {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            transport_error_halt_threshold: 5,
        }
    }
}
```

### 3.3 Runtime consecutive-counter + halt logic — `crates/kernel/src/runtime.rs`

Add a new field to `Runtime`:

```rust
struct Runtime {
    // ... existing fields ...

    /// Count of consecutive `SubError::TransportError` returns from
    /// the active subscription. Resets to 0 on any successful recv
    /// (`Ok(Some(_))` or `Ok(None)`). When this exceeds
    /// `cfg.transport_error_halt_threshold`, the runtime signals
    /// halt and exits the task. Per B-4.3 spec §3.3.
    consecutive_transport_errors: usize,
}
```

Initialize in `Runtime::start` (find via grep):

```rust
let mut runtime = Runtime {
    // ... existing initializers ...
    consecutive_transport_errors: 0,
};
```

Update the recv-loop match arm. Currently `runtime.rs:521-548`:

```rust
recv_result = sub.recv() => match recv_result {
    Ok(Some(m)) => { let _ = self.handle_message(m).await; }
    Ok(None) => return Ok(()),
    Err(SubError::Lagged(n)) => { /* ... existing Lagged handler ... */ }
    Err(SubError::DecodeFailed { peer }) => { /* ... existing handler ... */ }
},
```

Replace with:

```rust
recv_result = sub.recv() => match recv_result {
    Ok(Some(m)) => {
        self.consecutive_transport_errors = 0;
        let _ = self.handle_message(m).await;
    }
    Ok(None) => return Ok(()),
    Err(SubError::Lagged(n)) => {
        // existing Lagged handler — unchanged in behavior;
        // counter NOT reset (Lagged is not evidence the transport is alive).
        #[allow(clippy::expect_used)]
        self.peer_warnings
            .lock()
            .expect("peer_warnings mutex poisoned")
            .push(PeerWarning::BroadcastLagged { dropped: n });
        self.publish_heads_summary().await?;
    }
    Err(SubError::DecodeFailed { peer }) => {
        // existing DecodeFailed handler — unchanged in behavior;
        // counter NOT reset.
        #[allow(clippy::expect_used)]
        self.peer_warnings
            .lock()
            .expect("peer_warnings mutex poisoned")
            .push(PeerWarning::DecodeFailed { peer });
    }
    Err(SubError::TransportError(reason)) => {
        self.consecutive_transport_errors += 1;
        if self.consecutive_transport_errors >= self.cfg.transport_error_halt_threshold {
            // Halt: signal via halt_watch_tx and exit the loop.
            // The embedder observes via RuntimeHandle::halt_watch.
            // Per B-4.3 spec §3.3.
            let halt_msg = format!(
                "transport halted: {} consecutive errors (latest: {})",
                self.consecutive_transport_errors, reason
            );
            let _ = self.halt_watch_tx.send(Some(halt_msg));
            return Ok(());
        }
        // Below threshold: log + continue. No backfill (unlike Lagged).
        // No peer attribution (unlike DecodeFailed — transport errors
        // aren't attributable to a single peer).
        #[allow(clippy::expect_used)]
        self.peer_warnings
            .lock()
            .expect("peer_warnings mutex poisoned")
            .push(PeerWarning::TransportError {
                reason,
                consecutive: self.consecutive_transport_errors,
            });
    }
},
```

Add the new `PeerWarning::TransportError` variant alongside the existing variants:

```rust
pub enum PeerWarning {
    // ... existing BroadcastLagged, DecodeFailed, SignatureInvalid ...

    /// `Subscription::recv` returned a `SubError::TransportError`.
    /// Carries the description string from the transport error AND
    /// the current consecutive-count (so logs/observability can see
    /// the runtime's proximity to halt). The runtime halts when
    /// `consecutive` exceeds `cfg.transport_error_halt_threshold`.
    /// Per B-4.3 spec §3.0.
    TransportError {
        /// Description string from the underlying error.
        reason: String,
        /// Consecutive-error count at the time this warning was
        /// pushed (1, 2, 3, ... up to `transport_error_halt_threshold`).
        consecutive: usize,
    },
}
```

### 3.4 `MemBus::inject_transport_error` — `crates/network/src/memory.rs`

Sibling to existing `MemBus::inject_lag` (per `memory.rs:91-127`). Need to extend `TopicState` with a parallel `force_transport_error_flags: Vec<Weak<AtomicBool>>` field; `MemSubscription` needs a parallel `force_transport_error: Arc<AtomicBool>` field; `MemSubscription::recv` checks transport-error flag BEFORE lag flag.

**`TopicState` (modify `memory.rs:21-27`):**

```rust
struct TopicState {
    sender: tokio::sync::broadcast::Sender<GossipMessage>,
    force_lag_flags: Vec<Weak<AtomicBool>>,
    /// Mirrors `force_lag_flags`. Set via [`MemBus::inject_transport_error`].
    /// One entry per live subscription on this topic. Per B-4.3 spec §3.4.
    force_transport_error_flags: Vec<Weak<AtomicBool>>,
}
```

**`MemBus::sender_for` (modify `memory.rs:73-86`):** the existing `or_insert_with` literal also initializes `TopicState` and MUST be updated:

```rust
fn sender_for(&self, topic: Topic) -> tokio::sync::broadcast::Sender<GossipMessage> {
    let mut topics = self.topics.lock().unwrap_or_else(/* ... */);
    topics
        .entry(topic)
        .or_insert_with(|| TopicState {
            sender: tokio::sync::broadcast::channel(self.capacity_per_topic).0,
            force_lag_flags: Vec::new(),
            force_transport_error_flags: Vec::new(),  // NEW
        })
        .sender
        .clone()
}
```

**`MemSubscription` (modify `crates/network/src/subscription.rs` — locate via grep):**

```rust
pub struct MemSubscription {
    pub(crate) rx: tokio::sync::broadcast::Receiver<GossipMessage>,
    pub(crate) force_lag: Arc<AtomicBool>,
    /// Per-subscription one-shot transport-error flag. Set via
    /// [`MemBus::inject_transport_error`]. Per B-4.3 spec §3.4.
    ///
    /// **Visibility**: `pub(crate)` so `MemBus::make_subscription`
    /// in `memory.rs` can initialize it directly (same pattern as
    /// the existing `rx` + `force_lag` fields).
    pub(crate) force_transport_error: Arc<AtomicBool>,
}
```

**`MemBus::make_subscription` (modify `memory.rs:54-71`):**

```rust
fn make_subscription(self: &Arc<Self>, topic: Topic) -> MemSubscription {
    let mut topics = self.topics.lock().unwrap_or_else(/* ... */);
    let state = topics.entry(topic).or_insert_with(|| TopicState {
        sender: tokio::sync::broadcast::channel(self.capacity_per_topic).0,
        force_lag_flags: Vec::new(),
        force_transport_error_flags: Vec::new(),
    });
    let lag_flag = Arc::new(AtomicBool::new(false));
    let transport_flag = Arc::new(AtomicBool::new(false));
    state.force_lag_flags.retain(|w| w.strong_count() > 0);
    state.force_transport_error_flags.retain(|w| w.strong_count() > 0);
    state.force_lag_flags.push(Arc::downgrade(&lag_flag));
    state.force_transport_error_flags.push(Arc::downgrade(&transport_flag));
    MemSubscription {
        rx: state.sender.subscribe(),
        force_lag: lag_flag,
        force_transport_error: transport_flag,
    }
}
```

**`MemBus::inject_transport_error` (new method, mirror `inject_lag` at `memory.rs:111-127`):**

```rust
/// Arm every live subscription on `topic` so that its next `recv`
/// returns `Err(SubError::TransportError("injected by MemBus::inject_transport_error"))`
/// exactly once, then resumes normal delivery.
///
/// Gate-paired with [`MemBus::inject_lag`]: both are deterministic
/// test affordances that bypass the natural-overflow / natural-actor-
/// death paths which are timing-dependent and hard to assert against.
///
/// Per B-4.3 spec §3.4.
///
/// # Panics
/// Does not panic. Mutex poisoning is recovered transparently.
#[cfg(any(test, feature = "test-helpers"))]
pub fn inject_transport_error(&self, topic: Topic) {
    let mut topics = self.topics.lock().unwrap_or_else(/* ... */);
    if let Some(state) = topics.get_mut(&topic) {
        state.force_transport_error_flags.retain(|w| w.strong_count() > 0);
        for weak in &state.force_transport_error_flags {
            if let Some(flag) = weak.upgrade() {
                flag.store(true, Ordering::SeqCst);
            }
        }
    }
}
```

**`MemSubscription::recv` (modify the existing recv body — locate via grep for `impl Subscription for MemSubscription`):**

```rust
async fn recv(&mut self) -> Result<Option<GossipMessage>, SubError> {
    // Check transport-error flag FIRST (before lag flag) — both flags
    // can be set independently, but transport-error has priority since
    // it represents a more severe failure mode.
    if self.force_transport_error.swap(false, Ordering::SeqCst) {
        return Err(SubError::TransportError(
            "injected by MemBus::inject_transport_error".to_string()
        ));
    }
    if self.force_lag.swap(false, Ordering::SeqCst) {
        return Err(SubError::Lagged(1));
    }
    // existing recv body (broadcast::Receiver::recv path) unchanged.
    // ...
}
```

### 3.5 No `IrohNetwork` Cargo / feature changes

This slice is variant-addition + runtime-policy. No new workspace dependencies. No iroh API changes.

## 4. Acceptance tests

| # | Test name | Flavor | Pattern |
|---|---|---|---|
| 1 | `transport_error_variant_decodes_and_displays` | default | Pure: construct `SubError::TransportError("foo")`, format via `{}`, assert it contains `"foo"`. Confirms the `#[error]` derive works. |
| 2 | `mem_bus_inject_transport_error_surfaces_in_recv` | default | Two `MemNetwork` handles + shared `MemBus`. `bus.inject_transport_error(topic)` after subscribe; next `recv` returns `Err(SubError::TransportError("injected by MemBus::inject_transport_error"))`. Resumes normal delivery on the call after. |
| 3 | `runtime_increments_consecutive_counter_on_transport_error` | `multi_thread, worker_threads = 2` | Single peer over `MemNetwork` with `cfg.transport_error_halt_threshold = 3`. Call `bus.inject_transport_error(topic)` twice (poll between to consume the first); peer_warnings accumulates 2 `PeerWarning::TransportError { consecutive: 1 }` + `{ consecutive: 2 }`. Runtime is still alive (no halt). |
| 4 | `runtime_halts_at_threshold_transport_errors` | `multi_thread, worker_threads = 2` | Same setup as #3, `transport_error_halt_threshold = 2`. Inject twice. Assert `halt_watch` resolves to `Some(reason)` where reason contains `"transport halted"` and `"consecutive"`. Verify runtime task exited (`Shutdown` reply on author channel returns Err). |
| 5 | `successful_recv_resets_consecutive_counter` | `multi_thread, worker_threads = 2` | `transport_error_halt_threshold = 3`. Inject transport error; pump a recv. Inject again; pump. Publish a real message (peer B as a 3rd MemNetwork handle); peer A's runtime receives it — counter resets to 0. Inject 3 more transport errors; assert halt at the 3rd (because counter restarted). Confirms the reset semantic. |
| 6 | `lagged_does_not_increment_transport_error_counter` | `multi_thread, worker_threads = 2` | `transport_error_halt_threshold = 2`. Inject lag 5 times (interleave with polls); assert no halt. Then inject 2 transport errors; assert halt. Confirms Lagged is structurally distinct from TransportError per spec §2 "Loose-vs-strict consecutive counter" row. |
~~| 7 | `decode_failed_does_not_increment_transport_error_counter` | ... |~~

**Test 7 dropped** — `MemSubscription::recv` never produces `SubError::DecodeFailed` (the broadcast receiver yields typed `GossipMessage`, no bincode decode step). Testing the DecodeFailed-doesn't-increment branch would require either an `inject_decode_failed` affordance on `MemBus` (scope expansion) or a mock `Subscription` impl. The runtime code path for `DecodeFailed` is structurally symmetric with the `Lagged` path (both push a warning, neither touches `consecutive_transport_errors`); test 6 covers the load-bearing "non-incrementing" invariant. The DecodeFailed-specific test adds little signal for substantial harness cost.

**Spec-coverage annotations**:
- Tests 1, 2 → `convergence.md §4.2` (sync protocol — lag-recovery is part of it; TransportError is the parallel for non-recoverable transport failure)
- Tests 3-7 → `convergence.md §4.4` (pre-check unification — parallel to halt-on-pre-check-failure)

If those anchors don't exist as named, adjust to closest existing heading per the implementer's `grep -n "^### 4\|^## 4" docs/specs/2026-05-09-myrhiza-master-design/convergence.md` survey.

## 5. Justfile changes

None. Existing `just ci` recipe covers the new tests automatically.

## 6. Edge cases

- **Counter at threshold-1, then a successful recv**: counter resets to 0. The "5 errors in a row" semantic is strict consecutive. Tested by test 5.
- **`Ok(None)` (stream closed) before threshold**: runtime returns `Ok(())` immediately (existing behavior). No halt signal triggered — the "stream closed" path is the clean shutdown path; the halt signal is for "the transport failed catastrophically".
- **`TransportError` arrives while the runtime is already publishing a HeadsSummary**: the publish completes (`tokio::select!` is biased to author commands, so an in-flight publish via the author path is unaffected; the recv arm fires after publish resolves). Halt happens on the next recv iteration if the threshold is exceeded.
- **`MemBus::inject_transport_error` called when no subscriptions exist**: no-op. Matches existing `inject_lag` behavior.
- **Multiple subscriptions on same topic, inject_transport_error fires once per call**: each subscription has its own `force_transport_error` flag; all are armed. Each consumes once.
- **Iroh-gossip's `ApiError` may have non-displayable variants**: the `format!("{api_err}")` call uses iroh-gossip's `Display` impl, which is `n0_error`-shaped. The string is opaque; no consumer parses it.
- **`PeerWarning::TransportError` is per-call (per-error-event), not aggregated**: each error increments the warning list. The `consecutive` field lets observability see the trajectory toward halt.
- **Threat model — DoS-to-halt via crafted ApiError traffic**: a sufficiently noisy peer that drives the local `IrohSubscription` to return `ApiError` consistently (e.g. by exploiting iroh-gossip protocol-layer bugs to trigger errors on receive) could increment the counter toward halt at the default threshold of 5. This IS a viable DoS vector against the runtime in adversarial conditions. The counter is intentionally non-attributable (unlike `DecodeFailed` which carries `peer: Option<PeerPubkey>`) — transport errors aren't reliably attributable to a single peer. Mitigations deferred (B-4.4+ scope): per-peer error attribution if iroh-gossip surfaces it, rate-limited error counting, or threshold auto-tuning based on observed traffic patterns. Documenting the gap honestly per CLAUDE.md "surface tradeoffs explicit".

## 7. Surface change summary

**New variants**:
- `SubError::TransportError(String)` — in `myrhiza_network::lib`.
- `PeerWarning::TransportError { reason: String, consecutive: usize }` — in `myrhiza_kernel::runtime`.

**New `Runtime` field**: `consecutive_transport_errors: usize`.

**New `RuntimeCfg` field**: `transport_error_halt_threshold: usize` (default 5).

**New `MemBus` test affordance**: `inject_transport_error(topic)` — gated on `feature = "test-helpers"`.

**Modified existing files**:
- `crates/network/src/lib.rs` — `SubError::TransportError` variant.
- `crates/network/src/subscription.rs` — `Subscription::recv` rustdoc + `MemSubscription` field.
- `crates/network/src/memory.rs` — `MemBus` per-subscription flag plumbing + `inject_transport_error` method.
- `crates/network/src/iroh_transport.rs` — `IrohSubscription::recv` ApiError mapping (Lagged → TransportError).
- `crates/kernel/src/runtime.rs` — `RuntimeCfg` field + `Runtime` counter + recv-loop handler + `PeerWarning::TransportError`.

**New files**:
- `crates/kernel/tests/halt_detection.rs` — 7 acceptance tests.

## 8. Non-goals (explicit)

- **No HeadsRequest direct-streams.** Deferred to B-4.4.
- **No real cross-process tests.** Deferred to B-4.4 or later.
- **No iroh-gossip `Event::Lagged` mapping changes.** Stays mapped to `SubError::Lagged(0)` per B-4.1.
- **No halt-on-decode-error.** `DecodeFailed` counter is intentionally separate (zero, never incremented).
- **No auto-recovery / re-bind.** Embedder responsibility.
- **No structured `TransportError` carrying the original `ApiError`.** String-only; the runtime treats TransportError as opaque.
- **No backfill on TransportError.** Distinct from Lagged.

## 9. Prior-art consultation

Consulted via the `using-prior-art` skill, 2026-05-20:

- **`prior-art/iroh/architecture.md` §"Connection errors"** (lines 84-90) — distinguishes `ConnectError` (TLS/QUIC handshake), `ConnectionError` (mid-stream protocol failure), `WriteError` / `ReadError` (stream-level). Iroh-gossip's `ApiError` is `n0_error`-shaped and wraps these primitives. The shape "transport-layer errors are categorically distinct from application-layer errors" is exactly what B-4.3's `SubError::TransportError` variant captures.
- **`prior-art/iroh/lessons.md` §Avoid row 1** (API churn) — `format!("iroh-gossip api error: {api_err}")` opaquely captures the error without coupling Myrhiza's wire format to iroh's `ApiError` shape. Survives the next rename.
- **`prior-art/iroh/lessons.md` §Borrow row 1** (kernel-owned `Endpoint`) — the kernel embedder constructs `Router` + `Gossip` once at boot; if iroh-gossip's actor dies, that's a kernel-wide event. Halt-at-runtime-level is appropriate; per-message recovery isn't.

**Runner-up paradigms rejected:**

- **Halt at the network layer** (in `IrohSubscription::recv`): putting halt policy inside the transport would couple every consumer to the same threshold. Runtime-side counter respects the trait's "just surface errors; consumer decides policy" discipline.
- **Auto-reconnect on TransportError**: would hide the failure from the embedder. Halt-and-report matches the existing `RuntimeError::Network -> halt` pattern at `runtime.rs:475-478`.

**Remaining gaps in the prior-art corpus**:

- **iroh-gossip actor-death observable signaling.** None of the consulted prior-art covers what iroh-gossip reports when its internal task dies (vs. transient API errors). B-4.3 treats both as undifferentiated `ApiError`; if iroh-gossip ever surfaces an actor-dead signal, the runtime can distinguish "transient retry" from "permanent halt" — that's a future refinement.
- **Threshold tuning for production traffic.** The default 5 is a guess; real traffic patterns may show transient ApiError bursts that legitimately exceed 5 (e.g. relay flap, NAT rebinding). Worth empirical tuning post-real-deployment.

## 10. Future work — explicit deferrals

- **B-4.4** — HeadsRequest direct-streams (new ALPN + Router protocol-handler) + real cross-process tests.
- **Backfilling `PeerWarning::SignatureInvalid` into `process_drift_message`** (drift handler asymmetry, per B-4.2 §10).
- **Lagged-event mapping test on real iroh-gossip** (B-4.1 §4 deferred). Requires upstream patching to expose `JoinOptions::subscription_capacity` or wrap the underlying broadcast channel.
- **Halt threshold auto-tuning** based on observed traffic patterns.
- **Distinguishing transient iroh-gossip errors from actor-death** if/when iroh-gossip exposes the distinction.

## 11. Sources

- `crates/network/src/iroh_transport.rs:206-226` — current `IrohSubscription::recv` ApiError → Lagged(0) mapping (replaced in §3.1).
- `crates/network/src/memory.rs:21-127` — `MemBus`, `TopicState`, `inject_lag` (template for §3.4).
- `crates/network/src/subscription.rs` — `Subscription` trait + `MemSubscription` (modified in §3.0 + §3.4).
- `crates/kernel/src/runtime.rs:155-237` — `PeerWarning` enum (extended in §3.3).
- `crates/kernel/src/runtime.rs:259-323` — `RuntimeHandle` + `halt_watch` exposure (existing surface; reused).
- `crates/kernel/src/runtime.rs:379-381` — `halt_watch_tx` field (reused in §3.3).
- `crates/kernel/src/runtime.rs:436-489` — `Runtime::start` initialization (modified in §3.3).
- `crates/kernel/src/runtime.rs:505-555` — recv-loop match (modified in §3.3).
- [`prior-art/iroh/architecture.md`](../prior-art/iroh/architecture.md) §"Connection errors".
- [`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md) §Avoid row 1 + §Borrow row 1.
- [`docs/specs/2026-05-20-plan-b-4-1-iroh-gossip-design.md`](2026-05-20-plan-b-4-1-iroh-gossip-design.md) §6 "ApiError mid-stream" — deferred to B-4.3.
- [`docs/specs/2026-05-20-plan-b-4-2-attribution-design.md`](2026-05-20-plan-b-4-2-attribution-design.md) §10 — direct-streams + cross-process tests deferred to B-4.3/4.4.
