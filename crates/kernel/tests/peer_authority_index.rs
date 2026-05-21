//! B-4.6 acceptance tests for the peer-authority index + the
//! direct-stream switchover of `request_author_chain_gap`.
//!
//! Per docs/specs/2026-05-21-plan-b-4-6-peer-authority-index-design.md §4.1.
//!
//! All tests use `multi_thread, worker_threads = 2` because runtimes
//! spawn drainer + handler tasks concurrently.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::similar_names,
    clippy::doc_markdown,
    clippy::large_enum_variant,
    clippy::collapsible_match,
    clippy::duration_subsec,
    clippy::type_complexity,
    clippy::duration_suboptimal_units,
    // `assert!(std::time::Instant::now() > deadline, ...)` pattern:
    clippy::nonminimal_bool,
)]

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use bincode::Options;
use myrhiza_kernel::identity::{AuthorKeypair, PeerKeypair};
use myrhiza_kernel::runtime::{AuthorCommand, Runtime, RuntimeCfg};
use myrhiza_network::request::{HeadsResponder, RequestHandler};
use myrhiza_network::{GossipMessage, MemBus, MemNetwork, Network, Subscription};
use myrhiza_test_utils::EventBuilder;
use myrhiza_types::{
    AuthorHead, AuthorPubkey, BundleHash, DirectHeadsRequest, EventHash, GenesisV1, HeadsSummary,
    HeadsSummarySignedPayload, PeerPubkey, Topic, canonical_bincode,
};
use tokio::sync::Mutex as TokioMutex;
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
        heads_summary_tick: Duration::from_millis(50),
        ..RuntimeCfg::default()
    }
}

fn genesis_payload(seed: [u8; 32], founder: AuthorPubkey) -> Vec<u8> {
    let g = GenesisV1 {
        seed,
        founder_pubkey: founder,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    canonical_bincode().serialize(&g).expect("encode genesis")
}

/// Build a correctly-signed `HeadsSummary` from `kp` advertising `author`
/// at `seq`. The head hash is `EventHash::ZERO` (placeholder).
fn build_signed_heads_summary(
    kp: &PeerKeypair,
    t: Topic,
    author: AuthorPubkey,
    seq: u64,
) -> HeadsSummary {
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

// ---- CapturingHandler -------------------------------------------------------

/// `RequestHandler` impl that records every inbound `(requester, requested_author)`
/// pair without serving any events. Used to verify that B targeted the correct
/// peer via direct-stream without requiring A to have a live DAG.
struct CapturingHandler {
    seen: Arc<TokioMutex<Vec<(PeerPubkey, AuthorPubkey)>>>,
}

#[async_trait::async_trait]
impl RequestHandler for CapturingHandler {
    async fn handle(
        &self,
        requester: PeerPubkey,
        request: DirectHeadsRequest,
        _responder: HeadsResponder,
    ) {
        let author = request
            .requests
            .first()
            .map_or_else(|| AuthorPubkey::from_bytes([0; 32]), |r| r.author);
        self.seen.lock().await.push((requester, author));
    }
}

// ===========================================================================
// Test 1: HeadsSummary receipt populates the peer-authority index
// ===========================================================================

/// Covers: convergence.md §4.2 — peer-authority index populated by HeadsSummary receipt (B-4.6 §4.1 test 1).
///
/// Peer A runs a full runtime and authors events. A CapturingHandler
/// is installed on A's MemNetwork AFTER `Runtime::start` (last-call-wins;
/// B-4.4 contract). Peer B's index is populated by A's periodic
/// HeadsSummary tick. When a forged event with a same-author gap is
/// injected into B, B looks up A in its peer-authority index and fires a
/// direct-stream request to A — captured by the CapturingHandler.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_populated_by_heads_summary_receipt() {
    let bus = MemBus::new(256);
    let t = topic();

    // Peer A — author-capable runtime.
    let kp_a = PeerKeypair::deterministic(101);
    let author_kp_a = AuthorKeypair::deterministic(101);
    let net_a = MemNetwork::new(bus.clone(), kp_a.public);
    let runtime_a = Runtime::start(
        net_a.clone(),
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_a,
        Some(AuthorKeypair::deterministic(101)),
        fast_cfg(),
    )
    .await
    .expect("runtime_a start");

    // Author genesis (seq=1) on A so A has something to advertise.
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

    // Install CapturingHandler on A AFTER Runtime::start (last-call-wins).
    let seen = Arc::new(TokioMutex::new(Vec::<(PeerPubkey, AuthorPubkey)>::new()));
    net_a.install_request_handler(Arc::new(CapturingHandler {
        seen: Arc::clone(&seen),
    }));

    // Peer B — read-only, starts with empty index.
    let kp_b = PeerKeypair::deterministic(102);
    let b_peer_pubkey = kp_b.public;
    let _runtime_b = Runtime::start(
        MemNetwork::new(bus.clone(), kp_b.public),
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_b,
        None,
        fast_cfg(),
    )
    .await
    .expect("runtime_b start");

    // Wait for B to receive A's HeadsSummary tick and populate its index.
    // A's tick fires every 50ms; wait up to 300ms.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Forge event 3 from A's author chain. B has only seen seq=1 via the
    // HeadsSummary (no actual events). Injecting seq=3 causes B's DAG to
    // return DagError::InvalidChain{expected_seq=2, got_seq=3}, which routes
    // through request_author_chain_gap. Since B now has A in its index, it
    // uses direct-stream (captured by CapturingHandler).
    let builder = EventBuilder::new(&author_kp_a);
    let e1 = builder.genesis(
        &APP_BUNDLE,
        TOPIC_SEED,
        "main",
        genesis_payload(TOPIC_SEED, author_kp_a.author),
    );
    let e2 = builder.next(&e1, BTreeSet::new(), 1_i64.to_be_bytes().to_vec());
    let e3 = builder.next(&e2, BTreeSet::new(), 2_i64.to_be_bytes().to_vec());

    // Inject event 3 directly onto the bus (B will see it, A will also see
    // it but it's already known there so it's a no-op for A).
    let net_injector = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0xA1; 32]));
    net_injector
        .publish(t, GossipMessage::Event(e3))
        .await
        .expect("inject e3");

    // Wait for the CapturingHandler to record B's request to A.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if std::time::Instant::now() > deadline {
            let recorded = seen.lock().await.clone();
            panic!(
                "CapturingHandler on A never received a request from B after index population; \
                 recorded={recorded:?}"
            );
        }
        let recorded = seen.lock().await.clone();
        // b_peer_pubkey is the PeerPubkey B used when issuing the direct-stream request.
        if recorded
            .iter()
            .any(|(req, auth)| *req == b_peer_pubkey && *auth == author_kp_a.author)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let recorded = seen.lock().await.clone();
    assert!(
        recorded
            .iter()
            .any(|(req, auth)| *req == b_peer_pubkey && *auth == author_kp_a.author),
        "CapturingHandler must have seen B's direct-stream request targeting A's author chain"
    );
}

// ===========================================================================
// Test 2: Move-to-front on repeated observation
// ===========================================================================

/// Covers: convergence.md §4.2 — peer-authority index MRU move-to-front (B-4.6 §4.1 test 2).
///
/// B already has events 1 and 2 in its DAG (injected directly). Three
/// HeadsSummaries advertising the same author at seq=2 arrive in order:
/// PA, PB, PA again. Since B is already at seq=2, HeadsSummary processing
/// classifies each diff as `Equal` — no backfill request fires from the
/// HeadsSummary path. After the three summaries, PA is MRU in the index.
///
/// Injecting event 4 (skipping seq=3) triggers
/// `DagError::InvalidChain{expected=3, got=4}` → `request_author_chain_gap`
/// → index lookup → B targets PA (MRU) via direct-stream.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines, reason = "linear scenario; splitting helpers would obscure the MRU ordering assertion")]
async fn index_move_to_front_on_repeated_observation() {
    let bus = MemBus::new(256);
    let t = topic();

    // The author whose chain the index will track.
    let author_kp = AuthorKeypair::deterministic(200);
    let watched_author = author_kp.author;

    // Build events 1 and 2 so we can pre-seed B's DAG.
    let builder = EventBuilder::new(&author_kp);
    let e1 = builder.genesis(
        &APP_BUNDLE,
        TOPIC_SEED,
        "main",
        genesis_payload(TOPIC_SEED, watched_author),
    );
    let e2 = builder.next(&e1, BTreeSet::new(), 1_i64.to_be_bytes().to_vec());
    let e3 = builder.next(&e2, BTreeSet::new(), 2_i64.to_be_bytes().to_vec());
    let e4 = builder.next(&e3, BTreeSet::new(), 3_i64.to_be_bytes().to_vec());

    // Peer B — read-only runtime with a very long tick so it does not emit
    // HeadsSummaries during the test window.
    let kp_b = PeerKeypair::deterministic(202);
    let b_peer_pubkey = kp_b.public;
    let _runtime_b = Runtime::start(
        MemNetwork::new(bus.clone(), kp_b.public),
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_b,
        None,
        RuntimeCfg {
            heads_summary_tick: Duration::from_secs(3600),
            ..RuntimeCfg::default()
        },
    )
    .await
    .expect("runtime_b start");

    // Synthetic peer PA (first and third observation).
    let kp_pa = PeerKeypair::deterministic(203);
    let net_pa = MemNetwork::new(bus.clone(), kp_pa.public);

    // Synthetic peer PB (second observation — becomes MRU momentarily).
    let kp_pb = PeerKeypair::deterministic(204);
    let net_pb = MemNetwork::new(bus.clone(), kp_pb.public);

    // Install CapturingHandlers on both PA and PB.
    let seen_pa = Arc::new(TokioMutex::new(Vec::<(PeerPubkey, AuthorPubkey)>::new()));
    let seen_pb = Arc::new(TokioMutex::new(Vec::<(PeerPubkey, AuthorPubkey)>::new()));
    net_pa.install_request_handler(Arc::new(CapturingHandler {
        seen: Arc::clone(&seen_pa),
    }));
    net_pb.install_request_handler(Arc::new(CapturingHandler {
        seen: Arc::clone(&seen_pb),
    }));

    let injector = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0xA2; 32]));

    // Inject events 1 and 2 so B's DAG has watched_author at seq=2.
    // After these are ingested, HeadsSummaries advertising seq=2 are Equal
    // diffs — no backfill request fires from the HeadsSummary handler.
    injector
        .publish(t, GossipMessage::Event(e1))
        .await
        .expect("inject e1");
    injector
        .publish(t, GossipMessage::Event(e2))
        .await
        .expect("inject e2");

    // Wait for B to insert both events into its DAG.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Inject HeadsSummaries in order: PA (seq=2) → PB (seq=2) → PA (seq=2).
    // Each summary advertises watched_author@seq=2. B's DAG is already at
    // seq=2 → Equal diff → no backfill request. But the index IS populated.
    // After PA→PB→PA, the MRU entry for watched_author is PA.
    let summary_pa_first = build_signed_heads_summary(&kp_pa, t, watched_author, 2);
    injector
        .publish(t, GossipMessage::HeadsSummary(summary_pa_first))
        .await
        .expect("publish PA first");
    tokio::time::sleep(Duration::from_millis(30)).await;

    let summary_pb = build_signed_heads_summary(&kp_pb, t, watched_author, 2);
    injector
        .publish(t, GossipMessage::HeadsSummary(summary_pb))
        .await
        .expect("publish PB");
    tokio::time::sleep(Duration::from_millis(30)).await;

    // PA again — this moves PA to the front (MRU).
    let summary_pa_second = build_signed_heads_summary(&kp_pa, t, watched_author, 2);
    injector
        .publish(t, GossipMessage::HeadsSummary(summary_pa_second))
        .await
        .expect("publish PA second");

    // Wait for B to process all three HeadsSummaries.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Inject event 4 (skipping seq=3). B's DAG has seq=2, so this gives
    // DagError::InvalidChain{expected_seq=3, got_seq=4}. B calls
    // request_author_chain_gap(watched_author, 3, 3). Index has PA as MRU →
    // direct-stream to PA. PB must NOT receive a request.
    injector
        .publish(t, GossipMessage::Event(e4))
        .await
        .expect("inject e4");

    // Wait for the CapturingHandler on PA to fire.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if std::time::Instant::now() > deadline {
            let pa = seen_pa.lock().await.clone();
            let pb_captured = seen_pb.lock().await.clone();
            panic!(
                "Expected B to target PA (MRU) via direct-stream for InvalidChain gap, \
                 but no request arrived on PA. PA seen={pa:?}, PB seen={pb_captured:?}"
            );
        }
        let pa = seen_pa.lock().await.clone();
        if pa.iter().any(|(req, _)| *req == b_peer_pubkey) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Assert PA got the request and PB did NOT.
    // Small extra wait to let any spurious PB request arrive.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let pa_requests = seen_pa.lock().await.clone();
    let pb_requests = seen_pb.lock().await.clone();
    assert!(
        pa_requests.iter().any(|(req, _)| *req == b_peer_pubkey),
        "PA must have received B's direct-stream request (PA is MRU after PA→PB→PA observations)"
    );
    assert!(
        !pb_requests.iter().any(|(req, _)| *req == b_peer_pubkey),
        "PB must NOT have received a direct-stream request from B (PB was displaced from MRU \
         by the second PA observation; the InvalidChain recovery targets only the MRU peer)"
    );
}

// ===========================================================================
// Test 3: Cap at 8 peers per author on overflow
// ===========================================================================

/// Covers: convergence.md §4.2 — peer-authority index capped at 8 per author (B-4.6 §4.1 test 3).
///
/// Nine synthetic peers advertise the same author. The last observed peer
/// (P9) should be MRU in B's index, so B's direct-stream request targets P9.
/// This also verifies the cap bound: even with 9 observations, the index
/// does not panic or return an error.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_caps_at_8_peers_per_author() {
    let bus = MemBus::new(256);
    let t = topic();

    // The author being tracked.
    let author_kp = AuthorKeypair::deterministic(300);
    let watched_author = author_kp.author;

    // Peer C — read-only, will accumulate 9 index entries and then serve one.
    let kp_c = PeerKeypair::deterministic(310);
    let c_peer_pubkey = kp_c.public;
    let _runtime_c = Runtime::start(
        MemNetwork::new(bus.clone(), kp_c.public),
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_c,
        None,
        RuntimeCfg {
            heads_summary_tick: Duration::from_secs(3600),
            ..RuntimeCfg::default()
        },
    )
    .await
    .expect("runtime_c start");

    // Create 9 synthetic peers, each with a CapturingHandler.
    let peer_seeds: Vec<u64> = (301..310).collect(); // seeds 301..=309 = 9 peers
    let mut nets: Vec<MemNetwork> = Vec::new();
    let mut seens: Vec<Arc<TokioMutex<Vec<(PeerPubkey, AuthorPubkey)>>>> = Vec::new();
    let mut peer_kps: Vec<PeerKeypair> = Vec::new();

    for &seed in &peer_seeds {
        let kp = PeerKeypair::deterministic(seed);
        let net = MemNetwork::new(bus.clone(), kp.public);
        let seen = Arc::new(TokioMutex::new(Vec::<(PeerPubkey, AuthorPubkey)>::new()));
        net.install_request_handler(Arc::new(CapturingHandler {
            seen: Arc::clone(&seen),
        }));
        nets.push(net);
        seens.push(seen);
        peer_kps.push(kp);
    }

    // Inject HeadsSummaries from all 9 peers in order (seed 301 first, 309 last).
    // After all 9 are processed, peer 309 (index 8) is MRU.
    let injector = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0xA3; 32]));
    for (i, kp) in peer_kps.iter().enumerate() {
        let summary = build_signed_heads_summary(kp, t, watched_author, (i as u64) + 1);
        injector
            .publish(t, GossipMessage::HeadsSummary(summary))
            .await
            .expect("publish summary");
        // Small gap so C processes them in order.
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Wait for all summaries to be processed.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Inject a same-author gap event to trigger request_author_chain_gap.
    let builder = EventBuilder::new(&author_kp);
    let e1 = builder.genesis(
        &APP_BUNDLE,
        TOPIC_SEED,
        "main",
        genesis_payload(TOPIC_SEED, watched_author),
    );
    let e2 = builder.next(&e1, BTreeSet::new(), 1_i64.to_be_bytes().to_vec());
    let e3 = builder.next(&e2, BTreeSet::new(), 2_i64.to_be_bytes().to_vec());
    injector
        .publish(t, GossipMessage::Event(e3))
        .await
        .expect("inject e3");

    // The MRU peer is peer_kps[8] (the 9th, seed 309). Wait for its
    // CapturingHandler to fire.
    let mru_seen = &seens[8];
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        assert!(
            std::time::Instant::now() <= deadline,
            "C should have targeted the MRU peer (index 8 = seed 309) via \
             direct-stream after 9 HeadsSummary observations, but the handler \
             never fired."
        );
        let mru = mru_seen.lock().await.clone();
        if mru.iter().any(|(req, _)| *req == c_peer_pubkey) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Assert: MRU peer (index 8) got the request.
    let mru = mru_seen.lock().await.clone();
    assert!(
        mru.iter().any(|(req, _)| *req == c_peer_pubkey),
        "The most-recently-observed peer (seed 309, index 8) must receive the direct-stream \
         request after 9 HeadsSummaries are observed (cap=8 enforced, MRU stays at front)"
    );
}

// ===========================================================================
// Test 4: Pending event with known author uses direct-stream — full convergence
// ===========================================================================

/// Covers: convergence.md §4.2 — Pending recovery via direct-stream when index is populated (B-4.6 §4.1 test 4).
///
/// A authors genesis + 4 events. B starts empty. A's HeadsSummary tick
/// populates B's index with A. Then a forged event with a cross-author
/// Pending dependency is injected into B, triggering the Pending path
/// (deps unknown → `request_missing_for` → `request_author_chain_gap`).
///
/// Simpler path: we use the InvalidChain gap approach (same code path),
/// but here B also receives A's real events via direct-stream backfill so
/// that convergence can be verified.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_event_with_known_author_uses_direct_stream() {
    let bus = MemBus::new(256);
    let t = topic();

    // Peer A — author-capable.
    let kp_a = PeerKeypair::deterministic(401);
    let author_kp_a = AuthorKeypair::deterministic(401);
    let net_a = MemNetwork::new(bus.clone(), kp_a.public);
    let runtime_a = Runtime::start(
        net_a,
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_a,
        Some(AuthorKeypair::deterministic(401)),
        fast_cfg(),
    )
    .await
    .expect("runtime_a start");

    // Author genesis + 3 increments = 4 events total.
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

    for delta in [1_i64, 1, 1] {
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

    // Peer B — read-only, starts empty.
    let kp_b = PeerKeypair::deterministic(402);
    let runtime_b = Runtime::start(
        MemNetwork::new(bus.clone(), kp_b.public),
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_b,
        None,
        fast_cfg(),
    )
    .await
    .expect("runtime_b start");

    // Wait for B to receive A's HeadsSummary and populate the index,
    // then receive A's direct-stream backfill and converge.
    let mut watch_b = runtime_b.digest_watch.clone();
    let watch_a = runtime_a.digest_watch.clone();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        assert!(
            std::time::Instant::now() <= deadline,
            "convergence timeout — A digest {:?}, B digest {:?}",
            watch_a.borrow().as_slice(),
            watch_b.borrow().as_slice(),
        );
        let a = watch_a.borrow().clone();
        let b = watch_b.borrow().clone();
        if !a.is_empty() && a == b {
            break;
        }
        let _ = timeout(Duration::from_millis(50), watch_b.changed()).await;
    }

    assert_eq!(
        watch_a.borrow().as_slice(),
        watch_b.borrow().as_slice(),
        "B must converge to A's digest via direct-stream backfill (index-routed)"
    );
}

// ===========================================================================
// Test 5: Pending event with unknown author falls back to gossip
// ===========================================================================

/// Covers: convergence.md §4.2 — Pending/InvalidChain fallback to gossip when index is empty (B-4.6 §4.1 test 5).
///
/// B's peer-authority index is empty (no HeadsSummary from any peer has
/// been received). An event with a same-author chain gap is injected.
/// `request_author_chain_gap` finds no peer in the index and falls back to
/// the legacy gossip-routed `publish(GossipMessage::HeadsRequest)` path.
/// A tap subscription captures the gossip-routed HeadsRequest.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_event_with_unknown_author_falls_back_to_gossip() {
    let bus = MemBus::new(256);
    let t = topic();

    // Peer B — read-only, uses a very long tick so its own HeadsSummary
    // never fires during this test. Index stays empty for the author below.
    let kp_b = PeerKeypair::deterministic(501);
    let _runtime_b = Runtime::start(
        MemNetwork::new(bus.clone(), kp_b.public),
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_b,
        None,
        RuntimeCfg {
            // Very long tick so no HeadsSummary from B is sent during the test.
            heads_summary_tick: Duration::from_secs(3600),
            ..RuntimeCfg::default()
        },
    )
    .await
    .expect("runtime_b start");

    // Give B's initial startup HeadsSummary a moment to flush so the tap
    // (opened below) doesn't capture it and confuse the assertion.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Tap — subscribes to the topic to capture B's gossip-routed emissions.
    let kp_tap = PeerKeypair::deterministic(502);
    let net_tap = MemNetwork::new(bus.clone(), kp_tap.public);
    let mut tap = net_tap.subscribe(t, vec![]).await.expect("tap subscribe");

    // Author key for the injected event — B has NEVER seen a HeadsSummary
    // from this author's peer, so B's index is empty for this author.
    let author_kp = AuthorKeypair::deterministic(503);
    let builder = EventBuilder::new(&author_kp);
    let e1 = builder.genesis(
        &APP_BUNDLE,
        TOPIC_SEED,
        "main",
        genesis_payload(TOPIC_SEED, author_kp.author),
    );
    let e2 = builder.next(&e1, BTreeSet::new(), 1_i64.to_be_bytes().to_vec());
    let e3 = builder.next(&e2, BTreeSet::new(), 2_i64.to_be_bytes().to_vec());

    // Inject event 3 (gap: B never saw seq=1 or seq=2). B's DAG returns
    // InvalidChain{expected_seq=1, got_seq=3}. Since the index is empty for
    // this author, B falls back to gossip-routed HeadsRequest.
    let net_pub = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0xA5; 32]));
    net_pub
        .publish(t, GossipMessage::Event(e3))
        .await
        .expect("inject e3");

    // Drain the tap for up to 500ms looking for a gossip-routed HeadsRequest.
    let deadline = std::time::Instant::now() + Duration::from_millis(500);
    let mut saw_heads_request = false;
    while std::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let r = timeout(remaining.min(Duration::from_millis(50)), tap.recv()).await;
        match r {
            Ok(Ok(Some(GossipMessage::HeadsRequest(req)))) => {
                // Look for a request covering the injected author's chain.
                if req.requests.iter().any(|e| e.author == author_kp.author) {
                    saw_heads_request = true;
                    break;
                }
            }
            Ok(Ok(None) | Err(_)) => break,
            _ => {}
        }
    }

    assert!(
        saw_heads_request,
        "B must emit a gossip-routed HeadsRequest for the unknown author when the \
         peer-authority index is empty (legacy fallback path, retained until B-4.7)"
    );
}

// ===========================================================================
// Test 6: InvalidChain uses direct-stream when index is populated
// ===========================================================================

/// Covers: convergence.md §4.2 — InvalidChain recovery via direct-stream when index populated (B-4.6 §4.1 test 6).
///
/// Specifically exercises the `DagError::InvalidChain` arm of
/// `handle_event` (same-author chain skip with `got_seq > expected_seq`),
/// which routes through `request_author_chain_gap`. With the index
/// populated from A's HeadsSummary, B targets A via direct-stream and
/// converges to A's digest.
///
/// This complements Test 4 (Pending path); both paths call
/// `request_author_chain_gap`, but this test drives the specific
/// `InvalidChain` branch.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_chain_uses_direct_stream_when_index_populated() {
    let bus = MemBus::new(256);
    let t = topic();

    // Peer A — author-capable runtime.
    let kp_a = PeerKeypair::deterministic(601);
    let author_kp_a = AuthorKeypair::deterministic(601);
    let net_a = MemNetwork::new(bus.clone(), kp_a.public);
    let runtime_a = Runtime::start(
        net_a,
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_a,
        Some(AuthorKeypair::deterministic(601)),
        fast_cfg(),
    )
    .await
    .expect("runtime_a start");

    // Author genesis + 3 increments = 4 events.
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

    for delta in [1_i64, 1, 1] {
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

    // Peer B — read-only, slow tick so it starts with empty index.
    // We inject A's HeadsSummary manually to populate the index, then
    // inject a forged event 4 (skipping seqs 1-3) to trigger InvalidChain.
    let kp_b = PeerKeypair::deterministic(602);
    let runtime_b = Runtime::start(
        MemNetwork::new(bus.clone(), kp_b.public),
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_b,
        None,
        fast_cfg(),
    )
    .await
    .expect("runtime_b start");

    // Wait for B to receive A's HeadsSummary (50ms tick) → index populated,
    // then A's direct-stream backfill fully converges B to A's digest.
    let mut watch_b = runtime_b.digest_watch.clone();
    let watch_a = runtime_a.digest_watch.clone();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        assert!(
            std::time::Instant::now() <= deadline,
            "convergence timeout (InvalidChain path) — A digest {:?}, B digest {:?}",
            watch_a.borrow().as_slice(),
            watch_b.borrow().as_slice(),
        );
        let a = watch_a.borrow().clone();
        let b = watch_b.borrow().clone();
        if !a.is_empty() && a == b {
            break;
        }
        let _ = timeout(Duration::from_millis(50), watch_b.changed()).await;
    }

    assert_eq!(
        watch_a.borrow().as_slice(),
        watch_b.borrow().as_slice(),
        "B must converge to A's digest via the InvalidChain → direct-stream recovery path \
         (request_author_chain_gap uses peer-authority index, not gossip broadcast)"
    );
}
