//! Kernel-tier acceptance tests for B-13 — kernel-mediated authoring
//! over `MemNetwork` (fast, no iroh).
//!
//! Proves the `propose → pre-check → sign → broadcast` path end-to-end at
//! the in-process tier: a `Runtime` started with a real
//! `counter-state-propose` handle turns an app-internal **intent** into a
//! kernel-signed, pre-checked, broadcast event via
//! `RuntimeHandle::propose_and_author`. The private key never reaches
//! propose — the kernel signs on its behalf (spec §2 / §6) — and a buggy
//! or malicious propose cannot get an invalid event applied because
//! `author`'s pre-check (state-apply dry-run) still gates (spec §4.4).
//!
//! Coverage (one `#[tokio::test]` per spec §7 row):
//!
//! | # | Test name | What it verifies |
//! |---|-----------|------------------|
//! | 1 | `intent_drives_propose_then_authors_event` | intent → propose → author → applied → state-changed → broadcast observed on the topic. |
//! | 2 | `propose_rejected_intent_surfaces_error` | propose-declined intent → `ProposeRejected`, no event, no broadcast. |
//! | 3 | `propose_output_failing_precheck_is_rejected` | propose output that state-apply rejects → `PreCheckRejected`, nothing committed/broadcast. |
//! | 4 | `propose_and_author_without_propose_component_errs` | runtime started with no propose component → `NoProposeComponent`. |
//! | 5 | `read_only_runtime_propose_and_author_errs` | runtime with no author key → `ReadOnly` (short-circuit before propose). |
//!
//! Per B-13 spec §7 / plan T4. The real-iroh analogue lives in
//! `iroh_propose_author.rs` (spec §7, `network-iroh` feature).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeSet;
use std::time::Duration;

use bincode::Options;
use myrhiza_kernel::identity::{AuthorKeypair, PeerKeypair};
use myrhiza_kernel::runtime::{Runtime, RuntimeError, RuntimeHandle};
use myrhiza_kernel::state_apply::StateApplyHandle;
use myrhiza_kernel::state_propose::StateProposeHandle;
use myrhiza_network::{GossipMessage, MemNetwork, Network, Subscription};
use myrhiza_test_utils::InProcessHarness;
use myrhiza_types::{Event, GenesisV1, canonical_bincode};

mod helpers;

// ---------------------------------------------------------------------
// Intent + payload helpers (counter v1 vocabulary)
// ---------------------------------------------------------------------
//
// The counter-state-propose contract (examples/counter/src/propose.rs):
//   intent[0]    = 0x00            // Increment discriminator
//   intent[1..9] = i64 BE delta
// Propose validates the intent and emits the 8-byte BE delta as the
// event payload — the same shape counter-state-apply consumes as a
// non-genesis increment. delta == 0, a wrong discriminator, or a
// too-short intent are rejected (ProposeError::Rejected).

/// Build a well-formed counter increment intent: `[0x00] + i64_be(delta)`.
fn increment_intent(delta: i64) -> Vec<u8> {
    let mut v = vec![0x00];
    v.extend_from_slice(&delta.to_be_bytes());
    v
}

/// Encode the founder's CreatePoll-style genesis payload for the counter:
/// a `GenesisV1` whose `app_payload` seeds the running counter to zero.
/// `Runtime::author` wraps this into the outer `Event` envelope.
fn counter_genesis_payload(seed: [u8; 32], founder: &AuthorKeypair) -> Vec<u8> {
    let payload = GenesisV1 {
        seed,
        founder_pubkey: founder.author,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    canonical_bincode()
        .serialize(&payload)
        .expect("encode GenesisV1")
}

// ---------------------------------------------------------------------
// Direct Runtime::start (propose-bearing) — the harness `spawn_peer`
// passes `None` for the propose handle, so B-13's author-side runtime is
// constructed directly here with `Some(propose)`. Shares the harness bus
// + topic + seed so a bare subscriber (or a second peer) can observe the
// broadcast.
// ---------------------------------------------------------------------

/// Spawn a propose-bearing runtime on the harness bus/topic.
///
/// `author_seed = None` makes the peer read-only (no author key);
/// `propose = None` omits the propose component. Returns the
/// `RuntimeHandle` (which exposes `propose_and_author`, `digest_watch`,
/// and `author_tx`) directly — B-13's API surface under test.
async fn start_runtime(
    harness: &InProcessHarness,
    peer_seed: u64,
    author_seed: Option<u64>,
    handle: StateApplyHandle,
    propose: Option<StateProposeHandle>,
) -> RuntimeHandle {
    let peer_key = PeerKeypair::deterministic(peer_seed);
    let net = MemNetwork::new(harness.bus.clone(), peer_key.public);
    let author_key = author_seed.map(AuthorKeypair::deterministic);
    Runtime::start(
        net,
        harness.topic,
        harness.app_bundle_hash,
        harness.topic_name.clone(),
        handle,
        peer_key,
        author_key,
        propose,
        // BACKGROUND_QUIET_TICK keeps periodic HeadsSummary timers from
        // racing the bare-subscriber assertion in test 1: the only
        // message published on the topic is the authored event.
        helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
        vec![],
        vec![],
    )
    .await
    .expect("Runtime::start")
}

/// Subscribe a bare reader on the harness bus/topic to observe the
/// runtime's broadcast (the [`MemNetwork`] sent-log of spec §7 test 1).
/// Subscribing through a distinct peer key models a separate gossip
/// participant.
async fn topic_reader(harness: &InProcessHarness) -> impl Subscription {
    let net = MemNetwork::new(
        harness.bus.clone(),
        myrhiza_types::PeerPubkey::from_bytes([0xEE; 32]),
    );
    net.subscribe(harness.topic, vec![])
        .await
        .expect("subscribe bare reader")
}

/// Send `AuthorCommand::Shutdown` to a `RuntimeHandle` so the spawned
/// task exits cleanly without leaking.
async fn shutdown(rt: &RuntimeHandle) {
    let _ = rt
        .author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Shutdown)
        .await;
}

/// Poll `cond` until it holds or `timeout` elapses (condition-based
/// waiting, not a fixed sleep). Returns whether the condition was met.
async fn poll_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if cond() {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Receive the next `GossipMessage::Event` off `sub`, skipping any
/// non-event traffic (`HeadsSummary` etc.), bounded by `timeout`. Returns
/// `None` on timeout or stream close.
async fn recv_next_event(sub: &mut impl Subscription, timeout: Duration) -> Option<Event> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return None;
        }
        match tokio::time::timeout(remaining, sub.recv()).await {
            Ok(Ok(Some(GossipMessage::Event(ev)))) => return Some(ev),
            // Non-event message — keep draining within the deadline.
            Ok(Ok(Some(_))) => {}
            // Stream closed (`None`), recoverable sub error (`Err`), or the
            // recv timed out (outer `Err`) — nothing more to observe.
            Ok(Ok(None) | Err(_)) | Err(_) => return None,
        }
    }
}

// ---------------------------------------------------------------------
// Test 1: intent → propose → author → applied → state-changed → broadcast
// ---------------------------------------------------------------------

/// Spec §7 test 1.
///
/// A propose-bearing runtime authors its genesis (counter = 0), then
/// `propose_and_author(increment_intent(5))` drives the real
/// counter-state-propose component: it validates the intent, emits the
/// 8-byte BE delta, and the kernel signs + pre-checks + applies +
/// broadcasts it. Assertions:
///   - the call returns an `EventHash`,
///   - the runtime's `digest_watch` reflects the increment (counter = 5),
///   - a bare subscriber on the topic observes the authored event whose
///     payload is the 8-byte BE delta (the broadcast / "sent-log").
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn intent_drives_propose_then_authors_event() {
    let harness = InProcessHarness::new(256, [0x51; 32]);
    // Subscribe the observer BEFORE authoring so no broadcast is missed.
    let mut reader = topic_reader(&harness).await;

    let rt = start_runtime(
        &harness,
        1,
        Some(1),
        helpers::counter_handle(),
        Some(helpers::counter_propose_handle()),
    )
    .await;
    let kp = AuthorKeypair::deterministic(1);

    // Author genesis (counter = 0) so propose has an initialized state to
    // increment. The genesis is itself a broadcast event; we drain it
    // from the reader below so the increment is unambiguous.
    let genesis = counter_genesis_payload(harness.seed, &kp);
    let (gtx, grx) = tokio::sync::oneshot::channel();
    rt.author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Author {
            payload: genesis,
            deps: BTreeSet::new(),
            reply: gtx,
        })
        .await
        .expect("send genesis author");
    grx.await.expect("genesis reply").expect("genesis authored");

    // Drain the genesis broadcast so the next event we read is the
    // increment (its payload is the GenesisV1 envelope, not an 8-byte
    // delta — distinguishable, but draining keeps the assertion crisp).
    let genesis_ev = recv_next_event(&mut reader, Duration::from_secs(5))
        .await
        .expect("genesis event broadcast");
    assert_eq!(
        genesis_ev.seq, 1,
        "first broadcast on the topic is the founder's genesis (seq=1)"
    );

    // Wait for the genesis state to settle (counter = 0) before issuing
    // the intent, so the increment is computed from the post-genesis
    // state rather than racing the apply.
    assert!(
        poll_until(Duration::from_secs(5), || {
            *rt.digest_watch.borrow() == 0_i64.to_be_bytes().to_vec()
        })
        .await,
        "runtime must settle on post-genesis counter = 0; saw {:?}",
        rt.digest_watch.borrow()
    );

    // The B-13 surface under test: intent → propose → author.
    let hash = rt
        .propose_and_author(increment_intent(5))
        .await
        .expect("propose_and_author(increment 5)");

    // State changed: counter is now 5 (8-byte BE i64).
    assert!(
        poll_until(Duration::from_secs(5), || {
            *rt.digest_watch.borrow() == 5_i64.to_be_bytes().to_vec()
        })
        .await,
        "runtime digest must reflect the proposed increment (counter = 5); saw {:?}",
        rt.digest_watch.borrow()
    );

    // Broadcast observed: the next event on the topic is the proposed
    // increment, carrying the 8-byte BE delta payload, and its wire hash
    // matches the returned EventHash.
    let inc_ev = recv_next_event(&mut reader, Duration::from_secs(5))
        .await
        .expect("increment event broadcast");
    assert_eq!(
        inc_ev.payload,
        5_i64.to_be_bytes().to_vec(),
        "broadcast event payload must be the propose-produced 8-byte BE delta"
    );
    assert_eq!(
        inc_ev.wire_hash(),
        hash,
        "broadcast event wire_hash must equal the hash returned by propose_and_author"
    );

    shutdown(&rt).await;
}

// ---------------------------------------------------------------------
// Test 2: propose declines the intent → ProposeRejected, no event/broadcast
// ---------------------------------------------------------------------

/// Spec §7 test 2.
///
/// A zero-delta intent makes counter-state-propose return
/// `Err("zero-delta intent rejected")`, surfaced as
/// `RuntimeError::ProposeRejected`. No event is authored: the runtime's
/// digest stays at the post-genesis state and no second event is
/// broadcast on the topic.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn propose_rejected_intent_surfaces_error() {
    let harness = InProcessHarness::new(256, [0x52; 32]);
    let mut reader = topic_reader(&harness).await;

    let rt = start_runtime(
        &harness,
        1,
        Some(1),
        helpers::counter_handle(),
        Some(helpers::counter_propose_handle()),
    )
    .await;
    let kp = AuthorKeypair::deterministic(1);

    let genesis = counter_genesis_payload(harness.seed, &kp);
    let (gtx, grx) = tokio::sync::oneshot::channel();
    rt.author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Author {
            payload: genesis,
            deps: BTreeSet::new(),
            reply: gtx,
        })
        .await
        .expect("send genesis author");
    grx.await.expect("genesis reply").expect("genesis authored");

    // Drain the genesis broadcast and settle on counter = 0.
    let _ = recv_next_event(&mut reader, Duration::from_secs(5))
        .await
        .expect("genesis event broadcast");
    assert!(
        poll_until(Duration::from_secs(5), || {
            *rt.digest_watch.borrow() == 0_i64.to_be_bytes().to_vec()
        })
        .await,
        "runtime must settle on post-genesis counter = 0"
    );

    // Zero-delta intent → propose returns Err → ProposeRejected, with the
    // component's verbatim message (not double-prefixed).
    let err = rt
        .propose_and_author(increment_intent(0))
        .await
        .expect_err("zero-delta intent must be rejected by propose");
    assert!(
        matches!(&err, RuntimeError::ProposeRejected(msg) if msg == "zero-delta intent rejected"),
        "expected ProposeRejected(\"zero-delta intent rejected\"), got {err:?}"
    );

    // No event authored: digest unchanged, and no second event is
    // broadcast within a settle window (only the genesis was).
    assert_eq!(
        *rt.digest_watch.borrow(),
        0_i64.to_be_bytes().to_vec(),
        "rejected propose must not change runtime state"
    );
    assert!(
        recv_next_event(&mut reader, Duration::from_millis(300))
            .await
            .is_none(),
        "rejected propose must not broadcast any further event"
    );

    shutdown(&rt).await;
}

// ---------------------------------------------------------------------
// Test 3: propose output fails pre-check → PreCheckRejected, nothing committed
// ---------------------------------------------------------------------

/// Spec §7 test 3 — the load-bearing safety property.
///
/// The runtime pairs the real counter-state-propose (which happily emits
/// an 8-byte delta for a well-formed intent) with the
/// `pre-check-rejector` state-apply, whose `pre_check` returns
/// `Reject("not allowed")` for every event. So propose SUCCEEDS, but the
/// kernel's pre-check (state-apply dry-run, run inside `author`) rejects
/// the produced payload → `RuntimeError::PreCheckRejected`. Nothing is
/// committed (digest stays empty) and nothing is broadcast — proving a
/// buggy/malicious propose still cannot get an invalid event applied.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn propose_output_failing_precheck_is_rejected() {
    let harness = InProcessHarness::new(256, [0x53; 32]);
    let mut reader = topic_reader(&harness).await;

    // pre-check-rejector as the state-apply side: every pre_check Rejects.
    let rt = start_runtime(
        &harness,
        1,
        Some(1),
        helpers::pre_check_rejector_handle(),
        Some(helpers::counter_propose_handle()),
    )
    .await;

    // No genesis needed: counter-state-propose does not inspect prior
    // state, so it emits a valid 8-byte delta even from the empty initial
    // state. `author`'s pre-check then rejects it.
    let err = rt
        .propose_and_author(increment_intent(5))
        .await
        .expect_err("pre-check-rejector must reject the proposed payload");
    assert!(
        matches!(&err, RuntimeError::PreCheckRejected(reason) if reason == "not allowed"),
        "expected PreCheckRejected(\"not allowed\"), got {err:?}"
    );

    // Nothing committed: the digest never advanced past the empty
    // construction default, and no event was broadcast on the topic.
    assert!(
        rt.digest_watch.borrow().is_empty(),
        "pre-check-rejected propose must not commit any state; saw {:?}",
        rt.digest_watch.borrow()
    );
    assert!(
        recv_next_event(&mut reader, Duration::from_millis(300))
            .await
            .is_none(),
        "pre-check-rejected propose must not broadcast any event"
    );

    shutdown(&rt).await;
}

// ---------------------------------------------------------------------
// Test 4: no propose component installed → NoProposeComponent
// ---------------------------------------------------------------------

/// Spec §7 test 4.
///
/// A runtime started with `propose = None` (the existing default for all
/// non-B-13 runtimes) returns `RuntimeError::NoProposeComponent` from
/// `propose_and_author` — reached only after the read-only short-circuit,
/// so the runtime here has an author key.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn propose_and_author_without_propose_component_errs() {
    let harness = InProcessHarness::new(256, [0x54; 32]);
    let rt = start_runtime(&harness, 1, Some(1), helpers::counter_handle(), None).await;

    let err = rt
        .propose_and_author(increment_intent(5))
        .await
        .expect_err("runtime with no propose component must error");
    assert!(
        matches!(err, RuntimeError::NoProposeComponent),
        "expected NoProposeComponent, got {err:?}"
    );

    shutdown(&rt).await;
}

// ---------------------------------------------------------------------
// Test 5: read-only runtime → ReadOnly (short-circuit before propose)
// ---------------------------------------------------------------------

/// Spec §7 test 5.
///
/// A runtime with no author key (`author_seed = None`) returns
/// `RuntimeError::ReadOnly` from `propose_and_author`. Per spec §4.4 the
/// check short-circuits BEFORE running propose — so even though this
/// runtime DOES hold a propose component, it never runs (don't spend a
/// WASM call we cannot act on).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_only_runtime_propose_and_author_errs() {
    let harness = InProcessHarness::new(256, [0x55; 32]);
    let rt = start_runtime(
        &harness,
        1,
        None, // read-only: no author key
        helpers::counter_handle(),
        Some(helpers::counter_propose_handle()),
    )
    .await;

    let err = rt
        .propose_and_author(increment_intent(5))
        .await
        .expect_err("read-only runtime must error");
    assert!(
        matches!(err, RuntimeError::ReadOnly),
        "expected ReadOnly, got {err:?}"
    );

    shutdown(&rt).await;
}
