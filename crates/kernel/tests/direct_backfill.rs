//! B-4.5 acceptance tests for the kernel-side direct-stream backfill.
//!
//! Per docs/specs/2026-05-21-plan-b-4-5-kernel-runtime-integration-design.md §4.1.
//!
//! All tests that exercise cross-task communication use
//! `multi_thread, worker_threads = 2` so the runtime task, drainer
//! task, and handler task can run concurrently.
//!
//! ## Test 4 — removed (B-4.7)
//!
//! `direct_backfill_legacy_gossip_routed_request_still_serviced` was
//! removed by B-4.7 when the gossip-routed `HeadsRequest` surface
//! (`GossipMessage::HeadsRequest` + `handle_heads_request`) was retired.
//! The surface it tested no longer exists in the kernel.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeSet;
use std::time::Duration;

use bincode::Options;
use myrhiza_kernel::identity::{AuthorKeypair, PeerKeypair};
use myrhiza_kernel::runtime::{AuthorCommand, PeerWarning, Runtime, RuntimeCfg};
use myrhiza_network::{GossipMessage, MemBus, MemNetwork, Network};
use myrhiza_types::{
    AuthorHead, AuthorPubkey, BundleHash, DirectHeadsRequest, EventHash, EventRequest, GenesisV1,
    HeadsSummary, HeadsSummarySignedPayload, Topic, canonical_bincode,
};
use tokio::time::timeout;

mod helpers;

// ---- shared constants --------------------------------------------------------

const APP_BUNDLE: BundleHash = BundleHash::from_bytes([0xAB; 32]);
const TOPIC_SEED: [u8; 32] = [0xCD; 32];

fn topic() -> Topic {
    Topic::derive(&APP_BUNDLE, &TOPIC_SEED, "main")
}

fn fast_cfg() -> RuntimeCfg {
    RuntimeCfg {
        drift_interval: 1,
        drift_min_interval: Duration::from_secs(0),
        drift_daily_cap: u32::MAX,
        heads_summary_tick: Duration::from_millis(50),
        ..RuntimeCfg::default()
    }
}

// ---- helpers -----------------------------------------------------------------

/// Build a correctly-signed `HeadsSummary` from `kp`, advertising a
/// single `AuthorHead` at `seq`. The head hash is `EventHash::ZERO`
/// (placeholder — the receiving runtime's DAG lookup is what matters).
///
/// Used by `direct_backfill_target_peer_unreachable_logs_warning` to
/// forge a message that passes `verify_heads_summary` so
/// `handle_heads_summary` actually runs and tries the direct-stream
/// backfill.
fn build_signed_heads_summary(kp: &PeerKeypair, t: Topic, seq: u64) -> HeadsSummary {
    let author = AuthorPubkey::from_bytes([0x42; 32]);
    let authors = vec![AuthorHead {
        author,
        seq,
        hash: EventHash::ZERO,
    }];
    let payload = HeadsSummarySignedPayload {
        authors: authors.clone(),
        kernel_fuel_table_version: 1,
        topic: t,
    };
    let bytes = canonical_bincode().serialize(&payload).expect("encode");
    let signature = kp.sign(&bytes);
    HeadsSummary {
        authors,
        kernel_fuel_table_version: 1,
        signed_by_peer: kp.public,
        signature,
    }
}

/// Construct the `GenesisV1` payload that the counter fixture expects for
/// seq=1. Each runtime must author genesis before subsequent events.
fn genesis_payload(seed: [u8; 32], founder: AuthorPubkey) -> Vec<u8> {
    let g = GenesisV1 {
        seed,
        founder_pubkey: founder,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    canonical_bincode().serialize(&g).expect("encode genesis")
}

// ===========================================================================
// Test 1: end-to-end two-peer convergence over direct-streams
// ===========================================================================

/// Covers: convergence.md §4.2 — direct-stream HeadsSummary-driven backfill (B-4.5 §4.1 test 1).
///
/// A authors a genesis event + 4 increments (5 events total). B starts
/// empty. A's periodic `HeadsSummary` tick fires (50ms), B receives it,
/// classifies the gap, and calls `issue_direct_backfill(A, …)`. A's
/// `KernelRequestHandler` receives the inbound request, forwards it to
/// A's runtime task, which runs `serve_direct_heads_request` and streams
/// all 5 events back. B's drainer forwards each event into
/// `internal_event_rx`; the select-loop arm calls `handle_event`; B
/// converges to A's digest.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn direct_backfill_two_peer_convergence_over_mem() {
    let bus = MemBus::new(256);
    let t = topic();

    // Peer A — author-capable.
    let kp_a = PeerKeypair::deterministic(101);
    let author_kp_a = AuthorKeypair::deterministic(101);
    let net_a = MemNetwork::new(bus.clone(), kp_a.public);
    let runtime_a = Runtime::start(
        net_a,
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_a,
        Some(AuthorKeypair::deterministic(101)),
        fast_cfg(),
        vec![],
    )
    .await
    .expect("runtime_a start");

    // Peer B — read-only, starts empty.
    let kp_b = PeerKeypair::deterministic(102);
    let net_b = MemNetwork::new(bus.clone(), kp_b.public);
    let runtime_b = Runtime::start(
        net_b,
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_b,
        None,
        fast_cfg(),
        vec![],
    )
    .await
    .expect("runtime_b start");

    // A authors genesis (seq=1) + 4 increments = 5 events.
    let (tx0, rx0) = tokio::sync::oneshot::channel();
    runtime_a
        .author_tx
        .send(AuthorCommand::Author {
            payload: genesis_payload(TOPIC_SEED, author_kp_a.author),
            deps: BTreeSet::new(),
            reply: tx0,
        })
        .await
        .expect("author genesis");
    rx0.await.expect("genesis reply").expect("genesis ok");

    for delta in [1_i64, 1, 1, 1] {
        let (tx, rx) = tokio::sync::oneshot::channel();
        runtime_a
            .author_tx
            .send(AuthorCommand::Author {
                payload: delta.to_be_bytes().to_vec(),
                deps: BTreeSet::new(),
                reply: tx,
            })
            .await
            .expect("author increment");
        rx.await.expect("increment reply").expect("increment ok");
    }

    // Wait for B to converge to A's digest.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut digest_b = runtime_b.digest_watch.clone();
    let digest_a = runtime_a.digest_watch.clone();
    loop {
        assert!(
            std::time::Instant::now() <= deadline,
            "convergence timeout — A digest {:?}, B digest {:?}",
            digest_a.borrow().as_slice(),
            digest_b.borrow().as_slice(),
        );
        let a = digest_a.borrow().clone();
        let b = digest_b.borrow().clone();
        if !a.is_empty() && a == b {
            break;
        }
        let _ = timeout(Duration::from_millis(50), digest_b.changed()).await;
    }

    assert_eq!(
        digest_a.borrow().as_slice(),
        digest_b.borrow().as_slice(),
        "B must converge to A's digest via direct-stream backfill"
    );
}

// ===========================================================================
// Test 2: unreachable target peer logs DirectRequestFailed
// ===========================================================================

/// Covers: convergence.md §4.2 — direct-stream backfill failure (B-4.5 §4.1 test 2).
///
/// Forge a correctly-signed `HeadsSummary` from peer A (a real
/// `PeerKeypair` so the signature verifies) and publish it onto the bus.
/// However, A's `MemNetwork` has no `KernelRequestHandler` registered
/// (no `Runtime::start` was called for A), so B's `request_heads(A, …)`
/// returns `NetError::RequestFailed`. B must surface
/// `PeerWarning::DirectRequestFailed { peer: A, … }`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn direct_backfill_target_peer_unreachable_logs_warning() {
    let bus = MemBus::new(256);
    let t = topic();

    // Peer B — read-only runtime.
    let kp_b = PeerKeypair::deterministic(202);
    let net_b = MemNetwork::new(bus.clone(), kp_b.public);
    let runtime_b = Runtime::start(
        net_b,
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_b,
        None,
        fast_cfg(),
        vec![],
    )
    .await
    .expect("runtime_b start");

    // Peer A — a MemNetwork with NO Runtime attached. No
    // install_request_handler is ever called, so the MemBus has no
    // handler entry for kp_a.public. B's request_heads will return
    // NetError::RequestFailed.
    let kp_a = PeerKeypair::deterministic(201);
    let net_a = MemNetwork::new(bus.clone(), kp_a.public);
    // Publish a valid HeadsSummary so B's verify_heads_summary passes
    // and handle_heads_summary calls issue_direct_backfill.
    let summary = build_signed_heads_summary(&kp_a, t, 1);
    net_a
        .publish(t, GossipMessage::HeadsSummary(summary))
        .await
        .expect("publish heads_summary");

    // Wait for B to record DirectRequestFailed { peer: kp_a.public }.
    let target_peer = kp_a.public;
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if std::time::Instant::now() > deadline {
            let warnings = runtime_b.peer_warnings.lock().unwrap().clone();
            panic!(
                "DirectRequestFailed warning for peer {target_peer:?} never appeared; \
                 warnings={warnings:?}"
            );
        }
        let saw = runtime_b.peer_warnings.lock().unwrap().iter().any(|w| {
            matches!(
                w,
                PeerWarning::DirectRequestFailed { peer, .. } if *peer == target_peer
            )
        });
        if saw {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Cleanup.
    let _ = runtime_b.author_tx.send(AuthorCommand::Shutdown).await;
}

// ===========================================================================
// Test 3: handler topic-validation drops wrong-topic requests (clean EOF)
// ===========================================================================

/// Covers: convergence.md §4.6 — topic-binding on direct-stream requests (B-4.5 §4.1 test 3).
///
/// Peer A's `KernelRequestHandler` is bound to `topic_a`. A requester
/// sends a `DirectHeadsRequest` carrying `topic_b` (mismatched). The
/// handler's topic-validation branch drops the responder (clean EOF)
/// without forwarding to the runtime task. The requester receives
/// `stream.next() == None`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn direct_backfill_handler_topic_validation_drops_wrong_topic() {
    let bus = MemBus::new(256);
    let topic_a = topic();
    let topic_b = Topic::derive(&APP_BUNDLE, &[0xEF; 32], "main");
    assert_ne!(
        topic_a, topic_b,
        "topics must differ for this test to be meaningful"
    );

    // Peer A — runtime bound to topic_a.
    let kp_a = PeerKeypair::deterministic(301);
    let net_a = MemNetwork::new(bus.clone(), kp_a.public);
    let _runtime_a = Runtime::start(
        net_a.clone(),
        topic_a,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_a,
        None,
        fast_cfg(),
        vec![],
    )
    .await
    .expect("runtime_a start");

    // Requester — a plain MemNetwork (no runtime).
    let kp_req = PeerKeypair::deterministic(302);
    let net_req = MemNetwork::new(bus.clone(), kp_req.public);

    // Issue a direct-stream request with the WRONG topic.
    let wrong_topic_req = DirectHeadsRequest {
        topic: topic_b, // mismatched: A is bound to topic_a
        requests: vec![EventRequest {
            author: AuthorPubkey::from_bytes([0x11; 32]),
            from_seq: 1,
            to_seq: 3,
        }],
    };
    let mut stream = net_req
        .request_heads(PeerKeypair::deterministic(301).public, wrong_topic_req)
        .await
        .expect("request_heads should succeed at the network level");

    // Wrong-topic → handler drops responder → requester sees clean EOF.
    let first = timeout(Duration::from_secs(2), stream.next()).await;
    match first {
        Ok(None) => { /* expected: clean EOF */ }
        Ok(Some(Ok(_))) => panic!("expected clean EOF, got an event"),
        Ok(Some(Err(e))) => panic!("expected clean EOF, got stream error: {e:?}"),
        Err(e) => panic!("timeout waiting for EOF on wrong-topic request: {e}"),
    }
}

// ===========================================================================
// Test 6: multiple concurrent backfills converge (three peers)
// ===========================================================================

/// Author a genesis event + `increments` additional events on `runtime`.
/// Genesis uses `seed` and `founder`. Returns without blocking on state-apply
/// outcome for post-genesis events (B's chain uses a non-TOPIC_SEED genesis
/// which the counter fixture may reject, but the events are still authored
/// and placed in the DAG for backfill).
async fn author_chain(
    runtime: &myrhiza_kernel::runtime::RuntimeHandle,
    seed: [u8; 32],
    founder: myrhiza_types::AuthorPubkey,
    increments: &[i64],
    must_succeed: bool,
) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    runtime
        .author_tx
        .send(AuthorCommand::Author {
            payload: genesis_payload(seed, founder),
            deps: BTreeSet::new(),
            reply: tx,
        })
        .await
        .expect("genesis send");
    if must_succeed {
        rx.await.expect("genesis reply").expect("genesis ok");
    } else {
        let _ = rx.await;
    }
    for &delta in increments {
        let (tx, rx) = tokio::sync::oneshot::channel();
        runtime
            .author_tx
            .send(AuthorCommand::Author {
                payload: delta.to_be_bytes().to_vec(),
                deps: BTreeSet::new(),
                reply: tx,
            })
            .await
            .expect("increment send");
        let _ = rx.await; // ignore state-apply result for non-canonical chains
    }
}

/// Covers: convergence.md §4.2 — concurrent direct-stream backfills (B-4.5 §4.1 test 6).
///
/// Three peers on a shared bus. A and B both author chains anchored
/// to the canonical `TOPIC_SEED` (so both chains' events are valid
/// for C's DAG topic). C starts empty and receives `HeadsSummary`
/// from A and B; both summaries trigger `issue_direct_backfill`
/// calls. Both drainer tasks feed into C's single `internal_event_rx`.
///
/// **What this test proves**: the two drainers do not collide when
/// pushing events into the same `internal_event_rx`. C reaches B's
/// digest (B's events are authored later and end up canonical, since
/// the counter `state-apply` reduces both chains' increments into a
/// single integer). The convergence of C to a non-zero digest that
/// equals the digest produced after applying BOTH chains' events
/// proves both backfill paths landed events.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn direct_backfill_multiple_concurrent_backfills_converge() {
    let bus = MemBus::new(256);
    let t = topic();

    // Peer A — author one of the two parallel chains (both share
    // TOPIC_SEED so DAG-genesis validation accepts both as topic
    // members).
    let kp_a = PeerKeypair::deterministic(601);
    let author_kp_a = AuthorKeypair::deterministic(601);
    let runtime_a = Runtime::start(
        MemNetwork::new(bus.clone(), kp_a.public),
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_a,
        Some(AuthorKeypair::deterministic(601)),
        fast_cfg(),
        vec![],
    )
    .await
    .expect("runtime_a start");
    author_chain(&runtime_a, TOPIC_SEED, author_kp_a.author, &[1, 1], true).await;

    // Peer B — different author on the SAME canonical topic seed.
    // Both chains are valid for C's DAG; both contribute to C's
    // final state under the counter state-apply.
    let kp_b = PeerKeypair::deterministic(602);
    let author_kp_b = AuthorKeypair::deterministic(602);
    let runtime_b = Runtime::start(
        MemNetwork::new(bus.clone(), kp_b.public),
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_b,
        Some(AuthorKeypair::deterministic(602)),
        fast_cfg(),
        vec![],
    )
    .await
    .expect("runtime_b start");
    author_chain(&runtime_b, TOPIC_SEED, author_kp_b.author, &[2, 2], true).await;

    // Peer C — empty, read-only. Receives HeadsSummary from A and B.
    let kp_c = PeerKeypair::deterministic(603);
    let runtime_c = Runtime::start(
        MemNetwork::new(bus.clone(), kp_c.public),
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_c,
        None,
        fast_cfg(),
        vec![],
    )
    .await
    .expect("runtime_c start");

    // Wait until both A's and B's digests are non-empty. They may
    // not converge to identical values — each peer's digest reflects
    // only events its own state-apply has observed — but each
    // individually proves its chain was authored.
    let mut watch_a = runtime_a.digest_watch.clone();
    let mut watch_b = runtime_b.digest_watch.clone();
    let deadline_pre = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        assert!(
            std::time::Instant::now() <= deadline_pre,
            "A or B digest never became non-empty"
        );
        if !watch_a.borrow().is_empty() && !watch_b.borrow().is_empty() {
            break;
        }
        let _ = timeout(Duration::from_millis(50), watch_a.changed()).await;
        let _ = timeout(Duration::from_millis(50), watch_b.changed()).await;
    }

    // C converges to a non-empty digest that equals both A's and B's
    // (all three peers have applied A's + B's chains via gossip
    // dissemination + direct-stream backfill). The equality assertion
    // is load-bearing: under the counter state-apply, the digest is a
    // function of every applied event, so C's digest matching A's
    // (= B's) requires that BOTH chains' events landed in C.
    let mut watch_c = runtime_c.digest_watch.clone();
    let deadline_c = std::time::Instant::now() + Duration::from_secs(6);
    loop {
        assert!(
            std::time::Instant::now() <= deadline_c,
            "C never converged: A={:?} B={:?} C={:?}",
            watch_a.borrow().as_slice(),
            watch_b.borrow().as_slice(),
            watch_c.borrow().as_slice(),
        );
        let a = watch_a.borrow().clone();
        let b = watch_b.borrow().clone();
        let c = watch_c.borrow().clone();
        if !c.is_empty() && c == a && c == b {
            break;
        }
        let _ = timeout(Duration::from_millis(50), watch_c.changed()).await;
    }

    let final_a = watch_a.borrow().clone();
    let final_b = watch_b.borrow().clone();
    let final_c = watch_c.borrow().clone();
    assert!(
        !final_c.is_empty(),
        "C's digest must be non-empty after concurrent direct-stream backfills"
    );
    assert_eq!(
        final_a, final_c,
        "C must converge to A's digest (A's chain landed via direct-stream backfill)"
    );
    assert_eq!(
        final_b, final_c,
        "C must converge to B's digest (B's chain landed via direct-stream backfill — proves the second drainer's events reached C, not just the first)"
    );
}
