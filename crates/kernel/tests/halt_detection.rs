//! B-4.3 acceptance tests for halt-on-persistent-transport-error.
//!
//! Per docs/specs/2026-05-20-plan-b-4-3-halt-detection-design.md §4.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use myrhiza_kernel::identity::PeerKeypair;
use myrhiza_kernel::runtime::{AuthorCommand, PeerWarning, Runtime, RuntimeCfg};
use myrhiza_network::{GossipMessage, MemBus, MemNetwork, Network, SubError, Subscription};
use myrhiza_types::{AuthorHead, BundleHash, HeadsSummary, Topic};

mod helpers;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_topic(seed: u8) -> Topic {
    Topic::from_bytes([seed; 32])
}

fn test_bundle_hash(seed: u8) -> BundleHash {
    BundleHash::from_bytes([seed; 32])
}

/// Build a `RuntimeCfg` with overrides suitable for halt-detection tests.
/// `heads_summary_tick` is deliberately long (1 hour) so the periodic
/// `HeadsSummary` ticker doesn't interfere with transport-error injection.
fn halt_cfg(threshold: usize) -> RuntimeCfg {
    RuntimeCfg {
        drift_interval: 1,
        drift_min_interval: Duration::from_secs(0),
        drift_daily_cap: u32::MAX,
        heads_summary_tick: Duration::from_hours(1),
        distribution_sync_tick: Duration::from_hours(1),
        pending_cfg: myrhiza_kernel::pending::PendingCfg::default(),
        broadcast_capacity: 256,
        kernel_fuel_table_version: 1,
        drift_stash_cap: 256,
        transport_error_halt_threshold: threshold,
    }
}

/// Spin up a single read-only peer runtime over a shared `MemBus`.
/// Returns the `RuntimeHandle`. Peer is read-only (no author key).
async fn spawn_peer_runtime(
    bus: Arc<MemBus>,
    topic: Topic,
    threshold: usize,
    peer_seed: u8,
) -> myrhiza_kernel::runtime::RuntimeHandle {
    // Construction shape mirrors `pending_event_triggers_heads_request_not_heads_summary`
    // in crates/kernel/tests/convergence.rs — the simplest single-peer form.
    let peer_key = PeerKeypair::deterministic(u64::from(peer_seed));
    let net = MemNetwork::new(bus, peer_key.public);
    let bundle_hash = test_bundle_hash(peer_seed);
    Runtime::start(
        net,
        topic,
        bundle_hash,
        "main".into(),
        helpers::counter_handle(),
        peer_key,
        None, // read-only — no author key
        halt_cfg(threshold),
        vec![],
        vec![],
    )
    .await
    .expect("Runtime::start")
}

/// Force the runtime's `sub.recv()` future to restart from the top by
/// sending a dummy `AuthorCommand::Author` on the author channel.
///
/// ## Why this works
///
/// The runtime's recv loop is `loop { tokio::select! { biased; cmd =
/// author_rx.recv() => ...; recv_result = sub.recv() => ...; _ = ticker.tick()
/// => ... } }`. The `sub.recv()` future is RECREATED on every loop iteration.
/// When the author arm fires, the recv future (currently blocking on
/// `broadcast::Receiver::recv().await`) is CANCELLED (dropped). On the next
/// iteration a FRESH `sub.recv()` is created, which checks the
/// `force_transport_error` flag at the top of its body BEFORE blocking on the
/// broadcast channel.
///
/// A read-only runtime has no author key; `author()` returns
/// `RuntimeError::ReadOnly` immediately. We discard the reply.
async fn kick_recv(handle: &myrhiza_kernel::runtime::RuntimeHandle) {
    let (tx, _rx) = tokio::sync::oneshot::channel();
    let _ = handle
        .author_tx
        .send(AuthorCommand::Author {
            payload: vec![],
            deps: BTreeSet::new(),
            reply: tx,
        })
        .await;
    // Give the runtime task a moment to process the author command and restart
    // the recv future so the next sub.recv() can check the flag.
    tokio::time::sleep(Duration::from_millis(50)).await;
}

/// Construct a minimal valid `HeadsSummary` signed by the given peer keypair.
/// Used in test 5 to publish a real message that the runtime accepts as
/// `Ok(Some(m))` and resets the consecutive-transport-error counter.
fn signed_heads_summary(kp: &myrhiza_kernel::identity::PeerKeypair, topic: Topic) -> HeadsSummary {
    use bincode::Options;
    use myrhiza_types::{HeadsSummarySignedPayload, canonical_bincode};

    let authors: Vec<AuthorHead> = vec![];
    let kernel_fuel_table_version = 1u32;

    let payload = HeadsSummarySignedPayload {
        authors: authors.clone(),
        kernel_fuel_table_version,
        topic,
    };
    let sign_bytes = canonical_bincode()
        .serialize(&payload)
        .expect("encode HeadsSummarySignedPayload");
    let signature = kp.sign(&sign_bytes);

    HeadsSummary {
        authors,
        kernel_fuel_table_version,
        signed_by_peer: kp.public,
        signature,
    }
}

// ---------------------------------------------------------------------------
// Test 1: pure display/decode
// ---------------------------------------------------------------------------

/// Covers: convergence.md §4.2
///
/// Pure unit test: construct `SubError::TransportError("foo")`, format via
/// `Display`, assert the string contains "foo" and "transport error".
/// Confirms the `#[error("transport error: {0}")]` derive produces the right
/// output.
#[test]
fn transport_error_variant_decodes_and_displays() {
    let e = SubError::TransportError("foo".to_string());
    let s = format!("{e}");
    assert!(
        s.contains("foo"),
        "Display must include the reason string; got: {s:?}"
    );
    assert!(
        s.contains("transport error"),
        "Display must contain 'transport error'; got: {s:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: MemBus inject affordance — single-subscription surface
// ---------------------------------------------------------------------------

/// Covers: convergence.md §4.2
///
/// Subscribe one `MemNetwork` handle to a topic. Call
/// `bus.inject_transport_error(topic)`; verify the next `recv()` returns
/// `Err(SubError::TransportError(reason))` where `reason` contains
/// "injected by `MemBus`". Then publish a real `HeadsSummary` onto the bus
/// and verify normal delivery resumes — the one-shot flag was consumed.
#[tokio::test]
async fn mem_bus_inject_transport_error_surfaces_in_recv() {
    let bus = MemBus::new(64);
    let net_a = MemNetwork::new(
        bus.clone(),
        myrhiza_types::PeerPubkey::from_bytes([0xA1; 32]),
    );
    let topic = test_topic(2);

    let mut sub = net_a
        .subscribe(topic, vec![])
        .await
        .expect("subscribe net_a");

    // Arm the transport-error flag.
    bus.inject_transport_error(topic);

    // Next recv must yield TransportError (no need for a wake-up message
    // because the flag is checked at the TOP of recv() before the broadcast
    // recv blocks — the current future is a fresh call that hasn't entered
    // the blocking broadcast recv yet).
    let r = sub.recv().await;
    match r {
        Err(SubError::TransportError(reason)) => {
            assert!(
                reason.contains("injected by MemBus"),
                "reason must mention inject site; got: {reason:?}"
            );
        }
        other => panic!("expected TransportError, got {other:?}"),
    }

    // Publish a real HeadsSummary; verify normal delivery resumes on the
    // next recv after the one-shot flag was consumed.
    let kp = PeerKeypair::deterministic(2);
    let summary = signed_heads_summary(&kp, topic);
    net_a
        .publish(topic, GossipMessage::HeadsSummary(summary))
        .await
        .expect("publish HeadsSummary");

    let r2 = tokio::time::timeout(Duration::from_millis(500), sub.recv())
        .await
        .expect("recv did not time out after transport-error injection")
        .expect("recv Ok");
    assert!(
        r2.is_some(),
        "post-transport-error publish must deliver a real message"
    );
}

// ---------------------------------------------------------------------------
// Test 3: runtime accumulates sub-threshold warnings
// ---------------------------------------------------------------------------

/// Covers: convergence.md §4.4
///
/// Single read-only peer with `transport_error_halt_threshold = 3`. Inject
/// transport errors twice, using `kick_recv` between injections to force the
/// runtime's `sub.recv()` future to restart so it observes the flag. Assert:
/// (a) `peer_warnings` accumulates 2 `PeerWarning::TransportError` entries
///     with `consecutive = 1` and `consecutive = 2` respectively.
/// (b) The runtime is still alive (`halt_watch` stays `None`).
///
/// ## Polling discipline
///
/// `MemSubscription::recv` checks the `force_transport_error` flag at the
/// TOP of each call, before blocking on the broadcast channel. The runtime's
/// recv future is created fresh each loop iteration inside `tokio::select!`.
/// When the `author_rx` arm fires (via `kick_recv`), the recv future is
/// cancelled and a NEW one starts — which sees the flag immediately, without
/// needing a broadcast message to unblock it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_increments_consecutive_counter_on_transport_error() {
    let bus = MemBus::new(64);
    let topic = test_topic(3);

    let handle = spawn_peer_runtime(bus.clone(), topic, 3, 3).await;

    // Let the runtime settle its startup HeadsSummary publish.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Inject error #1; force the recv future to restart via kick_recv so it
    // sees the flag immediately.
    bus.inject_transport_error(topic);
    kick_recv(&handle).await;

    // Give the runtime task time to process the TransportError and push the
    // warning before we inspect peer_warnings.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Inject error #2; same kick pattern.
    bus.inject_transport_error(topic);
    kick_recv(&handle).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // (a) Two warnings must have been pushed.
    let warnings = handle
        .peer_warnings
        .lock()
        .expect("peer_warnings mutex")
        .clone();
    let transport_warnings: Vec<_> = warnings
        .iter()
        .filter(|w| matches!(w, PeerWarning::TransportError { .. }))
        .collect();
    assert_eq!(
        transport_warnings.len(),
        2,
        "must have 2 TransportError warnings after 2 sub-threshold injections; \
         saw warnings={warnings:?}"
    );
    // First warning: consecutive = 1.
    assert!(
        matches!(
            transport_warnings[0],
            PeerWarning::TransportError { consecutive: 1, .. }
        ),
        "first warning must have consecutive=1; got {:?}",
        transport_warnings[0]
    );
    // Second warning: consecutive = 2.
    assert!(
        matches!(
            transport_warnings[1],
            PeerWarning::TransportError { consecutive: 2, .. }
        ),
        "second warning must have consecutive=2; got {:?}",
        transport_warnings[1]
    );

    // (b) Runtime still alive — halt_watch is None.
    let halt_val = handle.halt_watch.borrow().clone();
    assert!(
        halt_val.is_none(),
        "runtime must NOT have halted at sub-threshold (2 < 3); got halt={halt_val:?}"
    );

    // Cleanup.
    let _ = handle.author_tx.send(AuthorCommand::Shutdown).await;
}

// ---------------------------------------------------------------------------
// Test 4: runtime halts at threshold
// ---------------------------------------------------------------------------

/// Covers: convergence.md §4.4
///
/// Single read-only peer with `transport_error_halt_threshold = 2`. Inject
/// two transport errors (threshold reached on the second). Assert:
/// (a) `halt_watch` resolves to `Some(reason)` within 5 seconds.
/// (b) `reason` contains both "transport halted" and "consecutive".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_halts_at_threshold_transport_errors() {
    let bus = MemBus::new(64);
    let topic = test_topic(4);

    let handle = spawn_peer_runtime(bus.clone(), topic, 2, 4).await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Inject error #1 (consecutive = 1, below threshold of 2).
    bus.inject_transport_error(topic);
    kick_recv(&handle).await;
    // Wait for the warning to be pushed (counter = 1, not yet halted).
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Inject error #2 (consecutive = 2 = threshold — runtime must halt).
    bus.inject_transport_error(topic);
    kick_recv(&handle).await;

    // Wait for the halt signal.
    let mut halt_rx = handle.halt_watch.clone();
    tokio::time::timeout(Duration::from_secs(5), async {
        // changed() fires when the watch transitions from None to Some(_).
        halt_rx.changed().await.expect("halt_watch channel closed");
    })
    .await
    .expect("halt_watch must signal within 5 seconds after reaching threshold");

    let halt_val = halt_rx.borrow().clone();
    let reason = halt_val.expect("halt_watch must be Some after halt");
    assert!(
        reason.contains("transport halted"),
        "halt reason must contain 'transport halted'; got: {reason:?}"
    );
    assert!(
        reason.contains("consecutive"),
        "halt reason must contain 'consecutive'; got: {reason:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 5: successful recv resets the consecutive counter
// ---------------------------------------------------------------------------

/// Covers: convergence.md §4.4
///
/// Threshold = 3. Phase 1: inject 2 transport errors via `kick_recv` (counter
/// reaches 2, runtime still alive). Phase 2: peer B publishes a
/// properly-signed `HeadsSummary`; the runtime receives it as `Ok(Some(_))`
/// → counter resets to 0. Phase 3: inject 3 more transport errors via
/// `kick_recv`; the runtime halts at the 3rd (because the counter restarted
/// from 0). This proves the reset semantic: the first 2 errors are
/// "forgotten" by the successful recv, and the 3-error threshold must be
/// met again from zero.
///
/// ## Why this proves the reset
///
/// Without the reset, the runtime would have consecutive=2 going into phase
/// 3. The first new error would push consecutive to 3 (≥ threshold) and halt
/// immediately. With the reset, the first new error is consecutive=1, the
/// second is consecutive=2, and the third is consecutive=3 (= threshold) —
/// the halt only fires on the THIRD new error. The test asserts no halt
/// after the 1st and 2nd new errors, then asserts halt after the 3rd.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn successful_recv_resets_consecutive_counter() {
    let bus = MemBus::new(64);
    let topic = test_topic(5);

    // Peer A's runtime (threshold = 3).
    let handle_a = spawn_peer_runtime(bus.clone(), topic, 3, 5).await;

    // Peer B's keypair for signing a real HeadsSummary.
    let kp_b = PeerKeypair::deterministic(55);
    let net_b = MemNetwork::new(bus.clone(), kp_b.public);

    tokio::time::sleep(Duration::from_millis(50)).await;

    // --- Phase 1: inject 2 transport errors; counter reaches 2 (< 3). ---
    bus.inject_transport_error(topic);
    kick_recv(&handle_a).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    bus.inject_transport_error(topic);
    kick_recv(&handle_a).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Assert no halt yet.
    assert!(
        handle_a.halt_watch.borrow().is_none(),
        "runtime must NOT have halted at counter=2 with threshold=3"
    );

    // --- Phase 2: peer B publishes a valid HeadsSummary — counter resets. ---
    // The runtime's subscription will receive this as Ok(Some(_)) and reset
    // consecutive_transport_errors to 0.
    //
    // After kick_recv, the recv future is fresh and blocking on the broadcast
    // channel. Publishing a message unblocks it — no kick needed here.
    let summary = signed_heads_summary(&kp_b, topic);
    net_b
        .publish(topic, GossipMessage::HeadsSummary(summary))
        .await
        .expect("peer B publish HeadsSummary");

    // Give the runtime's recv loop time to process the real message and
    // reset the counter.
    tokio::time::sleep(Duration::from_millis(150)).await;

    // --- Phase 3: inject 3 more transport errors — halt only at the 3rd. ---
    // After the reset, counter = 0. Each new transport error increments from
    // there. The runtime must NOT halt after 1 or 2 new errors (threshold=3).

    bus.inject_transport_error(topic);
    kick_recv(&handle_a).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    // Counter = 1 (1st after reset).
    assert!(
        handle_a.halt_watch.borrow().is_none(),
        "runtime must NOT halt at counter=1 after reset (threshold=3)"
    );

    bus.inject_transport_error(topic);
    kick_recv(&handle_a).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    // Counter = 2 (2nd after reset).
    assert!(
        handle_a.halt_watch.borrow().is_none(),
        "runtime must NOT halt at counter=2 after reset (threshold=3)"
    );

    // 3rd error after reset — counter = 3 = threshold → halt.
    bus.inject_transport_error(topic);
    kick_recv(&handle_a).await;

    let mut halt_rx = handle_a.halt_watch.clone();
    tokio::time::timeout(Duration::from_secs(5), async {
        halt_rx.changed().await.expect("halt_watch channel closed");
    })
    .await
    .expect(
        "halt_watch must signal within 5 s at the 3rd injection after reset \
         — proves the counter was actually reset to 0 by the successful recv",
    );

    let halt_val = halt_rx.borrow().clone();
    assert!(
        halt_val.is_some(),
        "halt_watch must be Some after 3 transport errors post-reset"
    );
}

// ---------------------------------------------------------------------------
// Test 6: Lagged does NOT increment the transport-error counter
// ---------------------------------------------------------------------------

/// Covers: convergence.md §4.4
///
/// Per spec §2 "Loose-vs-strict consecutive counter": `Err(SubError::Lagged)`
/// is neutral — it neither increments nor resets `consecutive_transport_errors`.
///
/// Threshold = 2. Inject lag 5 times (each via `kick_recv` to force the
/// runtime to observe the flag); assert no halt. Then inject 2 transport
/// errors via `kick_recv`; assert halt. If Lagged were incorrectly counted
/// toward the threshold, the runtime would halt at the 2nd lag injection —
/// but it must survive all 5 lags and only halt after 2 consecutive
/// transport errors.
///
/// `inject_lag` uses the same one-shot flag mechanism as
/// `inject_transport_error`; `kick_recv` forces a fresh `sub.recv()` call
/// that observes each lag flag in turn.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lagged_does_not_increment_transport_error_counter() {
    let bus = MemBus::new(64);
    let topic = test_topic(6);

    let handle = spawn_peer_runtime(bus.clone(), topic, 2, 6).await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Inject lag 5 times. Each kick_recv forces a fresh sub.recv() which
    // observes the Lagged flag. The counter is not incremented.
    for _ in 0..5 {
        bus.inject_lag(topic);
        kick_recv(&handle).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // No halt yet — 5 Lagged errors with threshold=2 must not cause a halt.
    assert!(
        handle.halt_watch.borrow().is_none(),
        "5 Lagged errors must NOT cause a halt when transport_error_halt_threshold = 2; \
         Lagged must be neutral per spec §2"
    );

    // Now inject 2 transport errors — these should push consecutive to 2 = threshold.
    bus.inject_transport_error(topic);
    kick_recv(&handle).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Assert no halt after 1 transport error (threshold = 2).
    assert!(
        handle.halt_watch.borrow().is_none(),
        "runtime must NOT halt after 1 transport error (threshold=2)"
    );

    bus.inject_transport_error(topic);
    kick_recv(&handle).await;

    // Runtime must halt.
    let mut halt_rx = handle.halt_watch.clone();
    tokio::time::timeout(Duration::from_secs(5), async {
        halt_rx.changed().await.expect("halt_watch channel closed");
    })
    .await
    .expect(
        "halt_watch must signal within 5 s at the 2nd transport error after 5 lagged — \
         proves Lagged was structurally distinct (did not accumulate toward threshold)",
    );

    let halt_val = halt_rx.borrow().clone();
    assert!(
        halt_val.is_some(),
        "halt_watch must be Some after 2 consecutive transport errors"
    );
}
