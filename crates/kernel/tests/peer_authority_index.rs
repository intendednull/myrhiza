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
        helpers::fast_cfg(helpers::FASTER_GOSSIP_TICK),
        vec![],
        vec![],
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
        helpers::fast_cfg(helpers::FASTER_GOSSIP_TICK),
        vec![],
        vec![],
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
    // Correct app_payload for EventBuilder::genesis is the raw counter seed
    // (0_i64 bytes) — NOT genesis_payload(...), which would double-wrap the
    // GenesisV1 struct that EventBuilder::genesis already builds internally.
    let e1 = builder.genesis(TOPIC_SEED, 0_i64.to_be_bytes().to_vec());
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
#[allow(
    clippy::too_many_lines,
    reason = "linear scenario; splitting helpers would obscure the MRU ordering assertion"
)]
async fn index_move_to_front_on_repeated_observation() {
    let bus = MemBus::new(256);
    let t = topic();

    // The author whose chain the index will track.
    let author_kp = AuthorKeypair::deterministic(200);
    let watched_author = author_kp.author;

    // Build events 1 and 2 so we can pre-seed B's DAG.
    let builder = EventBuilder::new(&author_kp);
    // Correct app_payload for EventBuilder::genesis is the raw counter seed
    // (0_i64 bytes) — NOT genesis_payload(...), which would double-wrap the
    // GenesisV1 struct that EventBuilder::genesis already builds internally.
    let e1 = builder.genesis(TOPIC_SEED, 0_i64.to_be_bytes().to_vec());
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
        vec![],
        vec![],
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
        vec![],
        vec![],
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
    // Correct app_payload for EventBuilder::genesis is the raw counter seed
    // (0_i64 bytes) — NOT genesis_payload(...), which would double-wrap the
    // GenesisV1 struct that EventBuilder::genesis already builds internally.
    let e1 = builder.genesis(TOPIC_SEED, 0_i64.to_be_bytes().to_vec());
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

/// Covers: convergence.md §4.2 — end-to-end convergence via direct-stream with a populated index (B-4.6 §4.1 test 4).
///
/// **What this test exercises**: full two-peer convergence via real
/// runtimes. A authors a chain; A's periodic HeadsSummary tick
/// populates B's peer-authority index AND triggers B's
/// `handle_heads_summary` backfill (the B-4.5 direct-stream path).
/// B converges to A's digest.
///
/// **What this test does NOT exercise**: the request_author_chain_gap
/// → direct-stream linkage specifically. In this scenario, B receives
/// A's events in-order via `handle_heads_summary`'s own backfill (the
/// B-4.5 path, already tested in `direct_backfill.rs::
/// direct_backfill_two_peer_convergence_over_mem`); `handle_event` never
/// returns Pending or InvalidChain because events arrive in order. The
/// request_author_chain_gap path is the subject of Test 6, which
/// genuinely drives `DagError::InvalidChain` via forged out-of-order
/// event injection.
///
/// **Pending-arm path note**: `request_missing_for` only routes to
/// `request_author_chain_gap` when `event.seq > known_head_seq + 1`
/// (the same-author gap branch). For pure cross-author Pending
/// (deps reference unknown hashes from a different author),
/// `request_missing_for` publishes a HeadsSummary instead (a
/// soft nudge), NOT a HeadsRequest. So the "Pending → direct-stream"
/// shape is structurally indistinguishable from the
/// "InvalidChain → direct-stream" shape that Test 6 exercises.
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
        helpers::fast_cfg(helpers::FASTER_GOSSIP_TICK),
        vec![],
        vec![],
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
        helpers::fast_cfg(helpers::FASTER_GOSSIP_TICK),
        vec![],
        vec![],
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
// Test 5: Pending event with unknown author publishes HeadsSummary soft-nudge
// ===========================================================================

/// Covers: convergence.md §4.2 — Pending/InvalidChain soft-nudge when index is empty (B-4.7 §3.1).
///
/// B's peer-authority index is empty (no HeadsSummary from any peer has
/// been received). An event with a same-author chain gap is injected.
/// `request_author_chain_gap` finds no peer in the index and calls
/// `publish_heads_summary()` — the same soft-nudge primitive used by the
/// cross-author Pending recovery path in `request_missing_for`.
/// A tap subscription captures the HeadsSummary soft-nudge.
/// The gossip-routed HeadsRequest path was retired in B-4.7.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_event_with_unknown_author_publishes_heads_summary_nudge() {
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
        vec![],
        vec![],
    )
    .await
    .expect("runtime_b start");

    // Give B's initial startup HeadsSummary a moment to flush so the tap
    // (opened below) doesn't capture it and confuse the assertion.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Tap — subscribes to the topic to capture B's soft-nudge emissions.
    let kp_tap = PeerKeypair::deterministic(502);
    let net_tap = MemNetwork::new(bus.clone(), kp_tap.public);
    let mut tap = net_tap.subscribe(t, vec![]).await.expect("tap subscribe");

    // Author key for the injected event — B has NEVER seen a HeadsSummary
    // from this author's peer, so B's index is empty for this author.
    let author_kp = AuthorKeypair::deterministic(503);
    let builder = EventBuilder::new(&author_kp);
    // Correct app_payload for EventBuilder::genesis is the raw counter seed
    // (0_i64 bytes) — NOT genesis_payload(...), which would double-wrap the
    // GenesisV1 struct that EventBuilder::genesis already builds internally.
    let e1 = builder.genesis(TOPIC_SEED, 0_i64.to_be_bytes().to_vec());
    let e2 = builder.next(&e1, BTreeSet::new(), 1_i64.to_be_bytes().to_vec());
    let e3 = builder.next(&e2, BTreeSet::new(), 2_i64.to_be_bytes().to_vec());

    // Inject event 3 (gap: B never saw seq=1 or seq=2). B's DAG returns
    // InvalidChain{expected_seq=1, got_seq=3}. Since the index is empty for
    // this author, B publishes a HeadsSummary soft-nudge (B-4.7 §3.1).
    let net_pub = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0xA5; 32]));
    net_pub
        .publish(t, GossipMessage::Event(e3))
        .await
        .expect("inject e3");

    // Drain the tap for up to 500ms looking for the HeadsSummary soft-nudge.
    let deadline = std::time::Instant::now() + Duration::from_millis(500);
    let mut saw_heads_summary_nudge = false;
    while std::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let r = timeout(remaining.min(Duration::from_millis(50)), tap.recv()).await;
        match r {
            Ok(Ok(Some(GossipMessage::HeadsSummary(_)))) => {
                // B published a HeadsSummary soft-nudge (empty index recovery,
                // B-4.7 §3.1 — replaces the retired gossip-routed HeadsRequest).
                saw_heads_summary_nudge = true;
                break;
            }
            Ok(Ok(None) | Err(_)) => break,
            _ => {}
        }
    }

    assert!(
        saw_heads_summary_nudge,
        "B must emit a HeadsSummary soft-nudge when the peer-authority index is \
         empty and an unknown author's chain gap is detected (B-4.7 §3.1)"
    );
}

// ===========================================================================
// Test 6: InvalidChain uses direct-stream when index is populated
// ===========================================================================

/// Covers: convergence.md §4.2 — InvalidChain recovery via direct-stream when index populated (B-4.6 §4.1 test 6).
///
/// Exercises the `DagError::InvalidChain` arm of `handle_event` directly:
/// `got_seq > expected_seq` triggers `request_author_chain_gap`, which uses
/// the peer-authority index (populated by an injected HeadsSummary from A) to
/// issue a direct-stream request to A. A CapturingHandler on A's MemNetwork
/// records the request, proving the B-4.6 direct-stream switchover fires for
/// the InvalidChain path — not just eventual convergence.
///
/// Structural differences from Test 4:
/// - B uses a 1-hour tick so no automatic HeadsSummary backfill races with
///   the CapturingHandler assertion.
/// - A's index entry is planted via `build_signed_heads_summary` rather than
///   the periodic tick, giving deterministic timing.
/// - Only event 3 (seq=3) is injected; B has no prior events for A's author
///   (expected_seq=1), so `InvalidChain{expected_seq=1, got_seq=3}` fires
///   and `request_author_chain_gap(A.author, 1, 2)` routes to direct-stream.
/// - Assertion is on CapturingHandler, not on digest convergence.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_chain_uses_direct_stream_when_index_populated() {
    let bus = MemBus::new(256);
    let t = topic();

    // Peer A — runs a full runtime so its MemNetwork can receive direct-stream
    // requests (CapturingHandler is installed after start, last-call-wins).
    // PeerKeypair doesn't impl Clone (ZeroizeOnDrop) so we construct twice
    // via `deterministic` (reproducible — same seed → same keypair); one is
    // moved into Runtime::start, the other is used below to sign the forged
    // HeadsSummary that pre-populates B's peer-authority index.
    let kp_a_for_runtime = PeerKeypair::deterministic(601);
    let kp_a = PeerKeypair::deterministic(601);
    let author_kp_a = AuthorKeypair::deterministic(601);
    let net_a = MemNetwork::new(bus.clone(), kp_a.public);
    let _runtime_a = Runtime::start(
        net_a.clone(),
        t,
        APP_BUNDLE,
        "main".into(),
        helpers::counter_handle(),
        kp_a_for_runtime,
        Some(AuthorKeypair::deterministic(601)),
        // Long tick — A should not interfere with B's index via its own
        // HeadsSummary during the test; B's index is seeded manually below.
        RuntimeCfg {
            heads_summary_tick: Duration::from_secs(3600),
            ..RuntimeCfg::default()
        },
        vec![],
        vec![],
    )
    .await
    .expect("runtime_a start");

    // Install CapturingHandler on A AFTER Runtime::start (last-call-wins).
    // B will issue direct-stream requests to kp_a.public, which this handler
    // intercepts. The handler does not emit any events back — only records.
    let seen = Arc::new(TokioMutex::new(Vec::<(PeerPubkey, AuthorPubkey)>::new()));
    net_a.install_request_handler(Arc::new(CapturingHandler {
        seen: Arc::clone(&seen),
    }));

    // Peer B — read-only, very long tick so no automatic HeadsSummary
    // fires during the test window. Index is seeded manually below.
    let kp_b = PeerKeypair::deterministic(602);
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
        vec![],
        vec![],
    )
    .await
    .expect("runtime_b start");

    // Seed B's peer-authority index: inject a HeadsSummary signed by kp_a
    // advertising author_kp_a.author at seq=3. B's index records
    // kp_a.public as the candidate peer for author_kp_a.author.
    // No actual events are sent, so B's DAG has expected_seq=1 for A's author.
    let injector = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0xA6; 32]));
    let summary = build_signed_heads_summary(&kp_a, t, author_kp_a.author, 3);
    injector
        .publish(t, GossipMessage::HeadsSummary(summary))
        .await
        .expect("inject HeadsSummary from A");

    // Give B time to process the HeadsSummary and populate its index.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Build a valid chain of 3 events signed by author_kp_a.
    // Only e3 (seq=3) will be injected. B has expected_seq=1 for A's author,
    // so inserting e3 returns DagError::InvalidChain{expected_seq=1, got_seq=3},
    // which triggers request_author_chain_gap(A.author, 1, 2).
    // The peer-authority index has kp_a.public for A's author → direct-stream
    // to A → CapturingHandler records the request from B.
    let builder = EventBuilder::new(&author_kp_a);
    let e1 = builder.genesis(TOPIC_SEED, 0_i64.to_be_bytes().to_vec());
    let e2 = builder.next(&e1, BTreeSet::new(), 1_i64.to_be_bytes().to_vec());
    let e3 = builder.next(&e2, BTreeSet::new(), 2_i64.to_be_bytes().to_vec());

    // Inject only e3 (seq=3). B has no prior events for A's author
    // (expected_seq=1), so InvalidChain fires and request_author_chain_gap
    // issues a direct-stream request to kp_a.public via the index.
    injector
        .publish(t, GossipMessage::Event(e3))
        .await
        .expect("inject e3");

    // Wait for the CapturingHandler on A to record B's direct-stream request.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if std::time::Instant::now() > deadline {
            let recorded = seen.lock().await.clone();
            panic!(
                "CapturingHandler on A never received a direct-stream request from B \
                 after InvalidChain gap injection; recorded={recorded:?}"
            );
        }
        let recorded = seen.lock().await.clone();
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
        "CapturingHandler must have seen B's direct-stream request to A targeting \
         A's author chain (InvalidChain{{expected_seq=1, got_seq=3}} → \
         request_author_chain_gap → peer-authority index lookup → direct-stream)"
    );
}
