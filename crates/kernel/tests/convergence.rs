//! Cross-peer convergence acceptance tests for plan B-1.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::time::Duration;

use bincode::Options;
use myrhiza_kernel::pending::PendingCfg;
use myrhiza_kernel::runtime::RuntimeCfg;
use myrhiza_test_utils::InProcessHarness;
use myrhiza_types::{GenesisV1, canonical_bincode};

mod helpers;

fn fast_cfg() -> RuntimeCfg {
    RuntimeCfg {
        drift_interval: 1,
        drift_min_interval: Duration::from_secs(0),
        drift_daily_cap: u32::MAX,
        heads_summary_tick: Duration::from_millis(100),
        pending_cfg: PendingCfg::default(),
        broadcast_capacity: 256,
        kernel_fuel_table_version: 1,
        drift_stash_cap: 256,
        transport_error_halt_threshold: 5,
    }
}

/// Covers: mvp.md §15.1 #1+#2, convergence.md §4
#[tokio::test]
async fn single_originator_single_receiver_converges() {
    let harness = InProcessHarness::new(256, [0x11; 32]);

    let cfg = fast_cfg();
    let peer_a = harness
        .spawn_peer(1, Some(1), helpers::counter_handle(), cfg.clone())
        .await;
    let mut peer_b = harness
        .spawn_peer(2, None, helpers::counter_handle(), cfg)
        .await;

    // A authors genesis. `Runtime::author` wraps `payload` in the full
    // Event envelope with `seq = 1` / `prev = ZERO` for the founder's
    // first event; the fixture's `apply` decodes that payload as a
    // `GenesisV1` when `seq == 1`. So the payload must be the canonical
    // bincode of `GenesisV1` (not raw initial state bytes).
    let initial = 0_i64.to_be_bytes().to_vec();
    let kp_a = myrhiza_kernel::identity::AuthorKeypair::deterministic(1);
    let genesis_payload = GenesisV1 {
        seed: harness.seed,
        founder_pubkey: kp_a.author,
        app_payload: initial.clone(),
    };
    let genesis_payload_bytes = canonical_bincode()
        .serialize(&genesis_payload)
        .expect("encode genesis payload");
    peer_a
        .author(genesis_payload_bytes, std::collections::BTreeSet::new())
        .await
        .expect("genesis");

    // Then 3 increments: +1, +2, -1 → final state = 0 + 1 + 2 - 1 = 2.
    for delta in [1_i64, 2, -1] {
        peer_a
            .author(
                delta.to_be_bytes().to_vec(),
                std::collections::BTreeSet::new(),
            )
            .await
            .expect("increment");
    }

    let expected_state = 2_i64.to_be_bytes().to_vec();
    assert!(
        peer_b
            .await_digest(expected_state.clone(), Duration::from_secs(5))
            .await,
        "peer B must converge to state {expected_state:?}"
    );
}

/// Covers: convergence.md §4.1, mvp.md §15.1 #2
#[tokio::test]
async fn concurrent_multi_author_converges() {
    let harness = InProcessHarness::new(256, [0x22; 32]);
    let cfg = fast_cfg();
    let mut peer_a = harness
        .spawn_peer(1, Some(1), helpers::counter_handle(), cfg.clone())
        .await;
    let mut peer_b = harness
        .spawn_peer(2, Some(2), helpers::counter_handle(), cfg)
        .await;

    // Peer A authors genesis (founder = A).
    let kp_a = myrhiza_kernel::identity::AuthorKeypair::deterministic(1);
    let genesis = GenesisV1 {
        seed: harness.seed,
        founder_pubkey: kp_a.author,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    let g_bytes = canonical_bincode().serialize(&genesis).expect("encode");
    peer_a
        .author(g_bytes, std::collections::BTreeSet::new())
        .await
        .expect("genesis");

    // Wait up to 5s for B to ingest genesis before B authors.
    // (B must see A's chain head before it can sign seq=2 events against
    // its own author key with the right prev; this test exercises concurrent
    // authoring from a SHARED post-genesis state.) `await_digest` is a
    // timeout-bounded poll on the digest watch channel — under tokio's
    // paused clock it advances when no other task is ready, but the
    // recv side is still racing publish, hence the explicit deadline.
    let initial_state = 0_i64.to_be_bytes().to_vec();
    assert!(
        peer_b
            .await_digest(initial_state, Duration::from_secs(5))
            .await,
        "peer B must ingest genesis before concurrent authoring begins"
    );

    // Concurrent authoring: A authors +1 and +2; B authors +10 and +20.
    // Replay through canonical topo-sort yields 0 + 1 + 2 + 10 + 20 = 33.
    for delta in [1_i64, 2] {
        peer_a
            .author(
                delta.to_be_bytes().to_vec(),
                std::collections::BTreeSet::new(),
            )
            .await
            .expect("a inc");
    }
    for delta in [10_i64, 20] {
        peer_b
            .author(
                delta.to_be_bytes().to_vec(),
                std::collections::BTreeSet::new(),
            )
            .await
            .expect("b inc");
    }

    let expected_state = 33_i64.to_be_bytes().to_vec();
    assert!(
        peer_a
            .await_digest(expected_state.clone(), Duration::from_secs(5))
            .await,
        "peer A must converge to state {expected_state:?}"
    );
    assert!(
        peer_b
            .await_digest(expected_state.clone(), Duration::from_secs(5))
            .await,
        "peer B must converge to state {expected_state:?}"
    );
}

/// Covers: convergence.md §4.2
#[tokio::test]
async fn late_joiner_backfills_via_heads_summary() {
    let harness = InProcessHarness::new(256, [0x33; 32]);
    let cfg = fast_cfg();
    let peer_a = harness
        .spawn_peer(1, Some(1), helpers::counter_handle(), cfg.clone())
        .await;

    // A authors genesis + 5 increments BEFORE B joins.
    let kp_a = myrhiza_kernel::identity::AuthorKeypair::deterministic(1);
    let genesis = GenesisV1 {
        seed: harness.seed,
        founder_pubkey: kp_a.author,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    let g_bytes = canonical_bincode().serialize(&genesis).expect("encode");
    peer_a
        .author(g_bytes, std::collections::BTreeSet::new())
        .await
        .expect("genesis");
    for delta in [1_i64, 1, 1, 1, 1] {
        peer_a
            .author(
                delta.to_be_bytes().to_vec(),
                std::collections::BTreeSet::new(),
            )
            .await
            .expect("inc");
    }

    // Now B joins.
    let mut peer_b = harness
        .spawn_peer(2, None, helpers::counter_handle(), cfg)
        .await;

    // Expected: 0 + 5*1 = 5.
    let expected_state = 5_i64.to_be_bytes().to_vec();
    assert!(
        peer_b
            .await_digest(expected_state, Duration::from_secs(10))
            .await,
        "late-joiner B must converge via HeadsSummary backfill"
    );
}

/// Covers: mvp.md §15.1 #4, convergence.md §4.6
#[tokio::test]
async fn coexistence_two_topics_no_event_crossing() {
    use bincode::Options;
    use myrhiza_types::{GenesisV1, canonical_bincode};

    // Two harnesses on the SAME bus but different seeds → different topics.
    let bus = myrhiza_network::MemBus::new(256);
    let app_bundle_hash = myrhiza_types::BundleHash::from_bytes([0xAB; 32]);

    let seed_a = [0x11; 32];
    let seed_b = [0x22; 32];
    let topic_a = myrhiza_types::Topic::derive(&app_bundle_hash, &seed_a, "main");
    let topic_b = myrhiza_types::Topic::derive(&app_bundle_hash, &seed_b, "main");
    assert_ne!(topic_a, topic_b);

    let cfg = fast_cfg();

    // Peer1 spawns runtimes on BOTH topics.
    let peer_key_1 = myrhiza_kernel::identity::PeerKeypair::deterministic(1);
    let net = myrhiza_network::MemNetwork::new(bus.clone(), peer_key_1.public);
    let kp_a = myrhiza_kernel::identity::AuthorKeypair::deterministic(1);
    let runtime_a = myrhiza_kernel::runtime::Runtime::start(
        net.clone(),
        topic_a,
        app_bundle_hash,
        "main".into(),
        helpers::counter_handle(),
        peer_key_1,
        Some(myrhiza_kernel::identity::AuthorKeypair::deterministic(1)),
        cfg.clone(),
    )
    .await
    .expect("runtime_a");

    let peer_key_2 = myrhiza_kernel::identity::PeerKeypair::deterministic(2);
    let net2 = myrhiza_network::MemNetwork::new(bus.clone(), peer_key_2.public);
    let runtime_b = myrhiza_kernel::runtime::Runtime::start(
        net2,
        topic_b,
        app_bundle_hash,
        "main".into(),
        helpers::counter_handle(),
        peer_key_2,
        Some(myrhiza_kernel::identity::AuthorKeypair::deterministic(2)),
        cfg,
    )
    .await
    .expect("runtime_b");

    // Author distinct values on each topic; assert no cross-pollution.
    let genesis_a = GenesisV1 {
        seed: seed_a,
        founder_pubkey: kp_a.author,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    let _ = runtime_a
        .author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Author {
            payload: canonical_bincode().serialize(&genesis_a).expect("encode"),
            deps: std::collections::BTreeSet::new(),
            reply: {
                let (tx, _rx) = tokio::sync::oneshot::channel();
                tx
            },
        })
        .await;

    // Wait a beat. If cross-topic delivery happened, runtime_b.equivocation_log
    // or its digest_watch would react. Both should remain at empty state.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let b_digest = runtime_b.digest_watch.borrow().clone();
    assert!(
        b_digest.is_empty(),
        "runtime_b on topic_b must NOT receive runtime_a's topic_a events; saw digest {b_digest:?}"
    );
}

/// Covers: convergence.md §4.7
#[tokio::test]
async fn drift_detected_when_state_apply_corrupted() {
    let harness = InProcessHarness::new(256, [0x44; 32]);
    let cfg = fast_cfg();
    let peer_a = harness
        .spawn_peer(1, Some(1), helpers::counter_handle(), cfg.clone())
        .await;
    // Peer B uses a corrupting state-apply that flips one digest byte at apply #3.
    let peer_b = harness
        .spawn_peer(2, None, helpers::corrupting_counter_handle(3), cfg)
        .await;

    let kp_a = myrhiza_kernel::identity::AuthorKeypair::deterministic(1);
    let genesis = GenesisV1 {
        seed: harness.seed,
        founder_pubkey: kp_a.author,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    peer_a
        .author(
            canonical_bincode().serialize(&genesis).expect("encode"),
            std::collections::BTreeSet::new(),
        )
        .await
        .expect("genesis");
    // Author 4 increments — enough events that B applies past the corruption threshold.
    for delta in [1_i64, 1, 1, 1] {
        peer_a
            .author(
                delta.to_be_bytes().to_vec(),
                std::collections::BTreeSet::new(),
            )
            .await
            .expect("inc");
    }

    // Wait deterministically for drift gossip to settle: poll the drift_log
    // on both peers until at least one records a divergence, or the deadline
    // expires. This mirrors the `await_digest` pattern (M-5 style) and avoids
    // a brittle fixed-sleep that races with cross-peer drift-message flight.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let (drift_a, drift_b) = loop {
        let drift_a = peer_a.drift_log();
        let drift_b = peer_b.drift_log();
        if !drift_a.is_empty() || !drift_b.is_empty() {
            break (drift_a, drift_b);
        }
        if std::time::Instant::now() >= deadline {
            break (drift_a, drift_b);
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    };

    assert!(
        !drift_a.is_empty() || !drift_b.is_empty(),
        "at least one peer must detect drift within deadline: a={drift_a:?} b={drift_b:?}"
    );
}

/// Covers: convergence.md §4.4.1
#[tokio::test]
async fn equivocating_author_chain_first_seen_wins() {
    use std::collections::BTreeSet;

    use myrhiza_kernel::dag::{DagError, EventDag};
    use myrhiza_test_utils::EventBuilder;
    use myrhiza_types::Topic;

    let bundle_hash = myrhiza_types::BundleHash::from_bytes([0xAA; 32]);
    let seed = [0x55; 32];
    let topic = Topic::derive(&bundle_hash, &seed, "main");
    let mut dag = EventDag::new(topic, bundle_hash, "main".into());

    let kp = myrhiza_kernel::identity::AuthorKeypair::deterministic(1);
    let builder = EventBuilder::new(&kp);

    let g1 = builder.genesis(&bundle_hash, seed, "main", vec![0xAA]);
    let g2 = builder.genesis(&bundle_hash, seed, "main", vec![0xBB]);
    assert_ne!(g1.wire_hash(), g2.wire_hash());

    dag.insert(g1.clone()).expect("first genesis");
    let r = dag.insert(g2.clone()).expect_err("equivocation");
    match r {
        DagError::Equivocation {
            author,
            seq,
            local_hash,
            remote_hash,
        } => {
            assert_eq!(author, kp.author);
            assert_eq!(seq, 1);
            assert_eq!(local_hash, g1.wire_hash());
            assert_eq!(remote_hash, g2.wire_hash());
        }
        other => panic!("expected Equivocation, got {other:?}"),
    }

    // Non-genesis equivocation.
    let e2a = builder.next(&g1, BTreeSet::new(), 1_i64.to_be_bytes().to_vec());
    let e2b = builder.next(&g1, BTreeSet::new(), 2_i64.to_be_bytes().to_vec());
    assert_ne!(e2a.wire_hash(), e2b.wire_hash());
    dag.insert(e2a.clone()).expect("first non-genesis");
    let r = dag.insert(e2b).expect_err("equivocation");
    assert!(matches!(r, DagError::Equivocation { seq: 2, .. }));
}

/// Covers: convergence.md §4.2, convergence.md §4.8
#[test]
fn pending_buffer_evicts_oldest_under_capacity() {
    use myrhiza_kernel::pending::{PendingBuffer, PendingCfg};
    use myrhiza_types::{AuthorPubkey, Event, EventHash, Hlc};
    use std::collections::BTreeSet;

    let cfg = PendingCfg {
        max_total: 3,
        max_per_author: 10,
        ttl: Duration::from_hours(1),
    };
    let mut pb = PendingBuffer::new(cfg);
    for i in 0u8..5 {
        let e = Event {
            author: AuthorPubkey::from_bytes([i; 32]),
            seq: 1,
            prev: EventHash::ZERO,
            deps: BTreeSet::new(),
            hlc: Hlc {
                wall_ms: 0,
                logical: 0,
            },
            payload: vec![i],
            signature: [0; 64],
        };
        let mut missing = BTreeSet::new();
        missing.insert(EventHash::blake3(&[i]));
        pb.insert(e, missing);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(pb.len(), 3, "PendingBuffer must cap at max_total = 3");
}

/// Covers: convergence.md §4.2 (deterministic variant per review M-9).
///
/// Uses `MemBus::inject_lag` (spec §6.3 / M-3) to force a single
/// `SubError::Lagged` on B's subscription, replacing the prior
/// natural-capacity-overflow setup whose timing was
/// consumer-speed-dependent. With the deterministic affordance both
/// indicators must be observable: B records a `BroadcastLagged`
/// warning AND B converges to the full A-authored state via the
/// `HeadsSummary` recovery nudge.
#[tokio::test]
async fn lagged_broadcast_recovers_via_heads_summary() {
    // Bus capacity is comfortably above the event count so natural
    // overflow cannot accidentally fire; the only lag in the test is
    // the one injected below.
    let harness = InProcessHarness::new(64, [0x66; 32]);
    let cfg = fast_cfg();
    let peer_a = harness
        .spawn_peer(1, Some(1), helpers::counter_handle(), cfg.clone())
        .await;
    let mut peer_b = harness
        .spawn_peer(2, None, helpers::counter_handle(), cfg)
        .await;

    // Arm a single forced Lagged on B's topic BEFORE A publishes any
    // event. The flag is set on the shared MemBus; B's next `recv`
    // that starts after this call consumes the flag and surfaces
    // `SubError::Lagged(1)`, triggering the HeadsSummary recovery
    // path in handle_subscription_error.
    harness.bus.inject_lag(harness.topic);

    let kp_a = myrhiza_kernel::identity::AuthorKeypair::deterministic(1);
    let genesis = GenesisV1 {
        seed: harness.seed,
        founder_pubkey: kp_a.author,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    peer_a
        .author(
            canonical_bincode().serialize(&genesis).expect("encode"),
            std::collections::BTreeSet::new(),
        )
        .await
        .expect("genesis");

    // Author 10 increments. The injected lag causes B to miss some
    // initial deliveries (depending on the precise interleave) and
    // observe `Lagged` on a subsequent recv; the recovery HeadsSummary
    // B then publishes drives A to backfill the gap so B converges.
    for delta in [1_i64; 10] {
        peer_a
            .author(
                delta.to_be_bytes().to_vec(),
                std::collections::BTreeSet::new(),
            )
            .await
            .expect("inc");
    }

    // Both indicators must hold deterministically: the lag warning
    // proves the lag-detection path fired, and full convergence
    // proves the HeadsSummary recovery actually backfilled. The
    // earlier `converged || lagged_seen` relaxation was a workaround
    // for the non-deterministic natural-overflow setup — no longer
    // necessary now that inject_lag is available (review M-9).
    let expected = 10_i64.to_be_bytes().to_vec();
    let converged = peer_b
        .await_digest(expected.clone(), Duration::from_secs(5))
        .await;
    let warnings = peer_b.peer_warnings();
    let lagged_seen = warnings.iter().any(|w| {
        matches!(
            w,
            myrhiza_kernel::runtime::PeerWarning::BroadcastLagged { .. }
        )
    });
    assert!(
        lagged_seen,
        "B must record a BroadcastLagged warning after inject_lag; \
         saw warnings={warnings:?}",
    );
    assert!(
        converged,
        "B must converge to state==10 after lag-recovery HeadsSummary; \
         saw digest={:?} warnings={warnings:?}",
        peer_b.current_digest(),
    );
}

/// Covers: convergence.md §4.2 — when an inbound event reveals the receiver is
/// behind on the event's author chain AND the peer-authority index is empty
/// (no `HeadsSummary` has been received from any peer for that author), the
/// recovery action post-B-4.7 is a `HeadsSummary` soft-nudge, NOT a
/// gossip-routed `HeadsRequest` (which was retired in B-4.7).
///
/// Setup: peer B subscribes to a shared bus. Peer A is *not* spawned as
/// a runtime — instead, the test manually constructs A's signed events
/// (genesis seq=1, seq=2, seq=3) via `EventBuilder` and injects ONLY
/// event 3 directly onto the bus. B has never observed A before, so the
/// DAG's chain-integrity check rejects event 3 with
/// `DagError::InvalidChain { expected_seq: 1, got_seq: 3 }`.
///
/// Because no `HeadsSummary` from any peer was preloaded, B's
/// peer-authority index is empty. `request_author_chain_gap` finds no
/// target peer and falls through to `publish_heads_summary()` — the same
/// soft-nudge primitive used by the cross-author Pending recovery path in
/// `request_missing_for`. Per B-4.7 spec §3.1.
///
/// A third "tap" subscription on the bus captures B's outbound gossip
/// and we assert at least one captured message is `HeadsSummary`. This
/// proves the soft-nudge protocol shape rather than just eventual
/// convergence.
#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "linear scenario test; splitting into helpers would obscure the protocol-shape assertion this test makes"
)]
async fn pending_event_triggers_heads_summary_nudge_when_index_empty() {
    use myrhiza_kernel::identity::AuthorKeypair;
    use myrhiza_network::{GossipMessage, MemBus, MemNetwork, Network, Subscription};
    use myrhiza_test_utils::EventBuilder;
    use myrhiza_types::{BundleHash, Topic};

    // Build a bus + topic + bundle shared between B, the publisher, and the tap.
    let bus = MemBus::new(256);
    let app_bundle_hash = BundleHash::from_bytes([0xAB; 32]);
    let topic_name = "main".to_string();
    let seed = [0x77u8; 32];
    let topic = Topic::derive(&app_bundle_hash, &seed, &topic_name);

    // Use a long heads-summary tick so B's periodic ticker doesn't
    // emit a HeadsSummary during the test window; we want to observe
    // ONLY the missing-deps recovery emission.
    let cfg = RuntimeCfg {
        drift_interval: 1,
        drift_min_interval: Duration::from_secs(0),
        drift_daily_cap: u32::MAX,
        heads_summary_tick: Duration::from_hours(1),
        pending_cfg: PendingCfg::default(),
        broadcast_capacity: 256,
        kernel_fuel_table_version: 1,
        drift_stash_cap: 256,
        transport_error_halt_threshold: 5,
    };

    // Spawn peer B (read-only — it never authors).
    let peer_key_b = myrhiza_kernel::identity::PeerKeypair::deterministic(2);
    let net_b = MemNetwork::new(bus.clone(), peer_key_b.public);
    let runtime_b = myrhiza_kernel::runtime::Runtime::start(
        net_b,
        topic,
        app_bundle_hash,
        topic_name.clone(),
        helpers::counter_handle(),
        peer_key_b,
        None,
        cfg,
    )
    .await
    .expect("runtime_b");

    // Give B's startup HeadsSummary a chance to flush so the tap (started
    // below) doesn't capture it and confuse the assertion. 50ms is enough
    // for the spawned task's first `publish_heads_summary().await` to
    // run on the in-process bus.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Now open the tap — captures everything published from this point on.
    let net_tap = MemNetwork::new(
        bus.clone(),
        myrhiza_types::PeerPubkey::from_bytes([0xA3; 32]),
    );
    let mut tap = net_tap
        .subscribe(topic, vec![])
        .await
        .expect("tap subscribe");

    // Construct A's events manually (A is NOT a running runtime).
    let kp_a = AuthorKeypair::deterministic(1);
    let builder = EventBuilder::new(&kp_a);
    let g_payload = 0_i64.to_be_bytes().to_vec();
    let e1 = builder.genesis(&app_bundle_hash, seed, &topic_name, g_payload);
    let e2 = builder.next(
        &e1,
        std::collections::BTreeSet::new(),
        1_i64.to_be_bytes().to_vec(),
    );
    let e3 = builder.next(
        &e2,
        std::collections::BTreeSet::new(),
        2_i64.to_be_bytes().to_vec(),
    );

    // Inject ONLY event 3 onto the bus. B's DAG returns
    // InvalidChain { author: A, expected_seq: 1, got_seq: 3 }. Because
    // B's peer-authority index is empty (no HeadsSummary preloaded),
    // request_author_chain_gap publishes a HeadsSummary soft-nudge
    // (B-4.7 §3.1) instead of a gossip-routed HeadsRequest.
    let net_pub = MemNetwork::new(
        bus.clone(),
        myrhiza_types::PeerPubkey::from_bytes([0xA4; 32]),
    );
    net_pub
        .publish(topic, GossipMessage::Event(e3.clone()))
        .await
        .expect("publish e3");

    // Drain the tap for up to ~300ms looking for B's soft-nudge emission.
    // Filter out the Event(e3) we just published. Look for HeadsSummary.
    let deadline = std::time::Instant::now() + Duration::from_millis(300);
    let mut saw_heads_summary_nudge = false;
    let mut captured: Vec<GossipMessage> = Vec::new();
    while std::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let r = tokio::time::timeout(remaining.min(Duration::from_millis(50)), tap.recv()).await;
        match r {
            Ok(Ok(Some(msg))) => {
                if let GossipMessage::HeadsSummary(_) = &msg {
                    // B published a HeadsSummary soft-nudge in response to
                    // the empty-index recovery path (B-4.7 §3.1).
                    saw_heads_summary_nudge = true;
                }
                captured.push(msg);
            }
            // None = subscription closed, Err = lagged/closed — both end the drain.
            Ok(Ok(None) | Err(_)) => break,
            Err(_) => {
                // poll timeout — keep looping until deadline
            }
        }
    }

    assert!(
        saw_heads_summary_nudge,
        "B must publish a HeadsSummary soft-nudge when the peer-authority index is \
         empty on an InvalidChain recovery (B-4.7 §3.1). \
         Captured messages: {captured:#?}"
    );

    // Cleanup: shutdown B's runtime so the test exits cleanly.
    let _ = runtime_b
        .author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Shutdown)
        .await;
}

/// Covers: convergence.md §4.4.1 + review-finding M-8.
///
/// Runtime-level companion to `equivocating_author_chain_first_seen_wins`
/// (which exercises `EventDag::insert` directly). This test proves that
/// equivocation also surfaces end-to-end through the `Runtime` +
/// `MemBus` ingest path — i.e., that `handle_event` records a
/// `DagError::Equivocation` into `equivocation_log` rather than
/// swallowing it.
///
/// Setup: a single read-only runtime B subscribes to a shared bus. The
/// test acts as a hostile publisher and injects TWO genesis events
/// (seq=1) signed by the SAME author key K but carrying different
/// `app_payload` bytes → different `body_hash` → different `wire_hash`.
/// B accepts the first as genesis and flags the second as
/// `DagError::Equivocation`. The runtime's equivocation arm pushes one
/// `EquivocationFlag` onto the log.
///
/// (A second runtime for the author K is intentionally omitted — its
/// equivocation log would also fire, but B is the interesting "third
/// party" observer because it didn't author either event.)
#[tokio::test]
async fn equivocation_via_membus_surfaces_in_peer_warnings() {
    use myrhiza_kernel::identity::AuthorKeypair;
    use myrhiza_network::{GossipMessage, MemBus, MemNetwork, Network};
    use myrhiza_test_utils::EventBuilder;
    use myrhiza_types::{BundleHash, Topic};

    // Shared bus + topic + bundle.
    let bus = MemBus::new(256);
    let app_bundle_hash = BundleHash::from_bytes([0xCD; 32]);
    let topic_name = "main".to_string();
    let seed = [0x88u8; 32];
    let topic = Topic::derive(&app_bundle_hash, &seed, &topic_name);

    // Long heads-summary tick + permissive drift so neither the periodic
    // ticker nor the rate limiter perturbs the test surface.
    let cfg = RuntimeCfg {
        drift_interval: 1,
        drift_min_interval: Duration::from_secs(0),
        drift_daily_cap: u32::MAX,
        heads_summary_tick: Duration::from_hours(1),
        pending_cfg: PendingCfg::default(),
        broadcast_capacity: 256,
        kernel_fuel_table_version: 1,
        drift_stash_cap: 256,
        transport_error_halt_threshold: 5,
    };

    // Spawn read-only B. No author key — B never authors, only observes.
    let peer_key_b2 = myrhiza_kernel::identity::PeerKeypair::deterministic(2);
    let net_b = MemNetwork::new(bus.clone(), peer_key_b2.public);
    let runtime_b = myrhiza_kernel::runtime::Runtime::start(
        net_b,
        topic,
        app_bundle_hash,
        topic_name.clone(),
        helpers::counter_handle(),
        peer_key_b2,
        None,
        cfg,
    )
    .await
    .expect("runtime_b");

    // Give B's subscription + startup HeadsSummary a chance to settle so
    // the next publishes are actually delivered to B's recv loop.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Build two conflicting genesis events signed by the SAME author key
    // but with different `app_payload` bytes. Different payload →
    // different body_hash → different wire_hash, so each is a distinct
    // event from the DAG's perspective; same (author, seq=1) makes the
    // second one an equivocation against the first.
    let kp = AuthorKeypair::deterministic(1);
    let builder = EventBuilder::new(&kp);
    let g1 = builder.genesis(&app_bundle_hash, seed, &topic_name, vec![0xAA]);
    let g2 = builder.genesis(&app_bundle_hash, seed, &topic_name, vec![0xBB]);
    assert_ne!(
        g1.wire_hash(),
        g2.wire_hash(),
        "the two conflicting genesis events must have distinct wire hashes"
    );

    // Publish both directly onto the bus (hostile publisher path — NOT
    // going through a Runtime::author call, which would refuse to author
    // two seq=1 events).
    let net_pub = MemNetwork::new(
        bus.clone(),
        myrhiza_types::PeerPubkey::from_bytes([0xB3; 32]),
    );
    net_pub
        .publish(topic, GossipMessage::Event(g1.clone()))
        .await
        .expect("publish g1");
    net_pub
        .publish(topic, GossipMessage::Event(g2.clone()))
        .await
        .expect("publish g2");

    // Poll B's equivocation_log until it records the conflict, or the
    // deadline expires. This mirrors the `drift_detected_when_state_apply_corrupted`
    // poll pattern and avoids racing the runtime's recv loop with a
    // brittle fixed-sleep.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let log = loop {
        let log = runtime_b
            .equivocation_log
            .lock()
            .expect("equivocation_log mutex")
            .clone();
        if !log.is_empty() {
            break log;
        }
        if std::time::Instant::now() >= deadline {
            break log;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    };

    assert_eq!(
        log.len(),
        1,
        "B must record exactly one equivocation flag for the conflicting seq=1 \
         genesis from author K; saw log={log:?}"
    );
    let flag = &log[0];
    assert_eq!(flag.author, kp.author, "equivocation author must be K");
    assert_eq!(flag.seq, 1, "equivocation seq must be 1 (genesis)");
    // `local_hash` is whichever event B accepted first; `remote_hash` is
    // the conflicting event observed second. Since we published g1
    // before g2, local should be g1 and remote should be g2 — but bus
    // delivery preserves publish order on a single subscriber, so this
    // ordering is deterministic.
    assert_eq!(
        flag.local_hash,
        g1.wire_hash(),
        "local_hash must be g1 (accepted first)"
    );
    assert_eq!(
        flag.remote_hash,
        g2.wire_hash(),
        "remote_hash must be g2 (conflicting, observed second)"
    );

    // Cleanup: shutdown B's runtime so the test exits cleanly.
    let _ = runtime_b
        .author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Shutdown)
        .await;
}

/// Covers: verification.md §22.8 — `PeerHandle::await_digest` must NOT
/// return `true` solely because the digest already equals the expected
/// value at call time. The function should always wait for at least one
/// fresh `changed()` notification before performing the equality check;
/// otherwise tests can pass vacuously (no fresh signal, no actual
/// cross-peer delivery exercised).
///
/// Setup: spawn read-only peer B on an empty harness. Its
/// `digest_watch` is initialized to `Vec::<u8>::new()` and no events
/// ever arrive, so the watch never observes a `changed()` signal.
/// Call `await_digest(vec![], 100ms)`:
/// - Buggy code: returns `true` immediately because the pre-wait
///   equality check matches (`*borrow() == expected`).
/// - Fixed code: blocks on `changed()` until the 100ms timeout, then
///   returns `false`.
///
/// We assert `false`. If `true` is observed, the pre-wait race is
/// still present.
#[tokio::test]
async fn await_digest_does_not_return_on_stale_already_equal_state() {
    let harness = InProcessHarness::new(64, [0x99; 32]);
    let cfg = fast_cfg();
    let mut peer_b = harness
        .spawn_peer(2, None, helpers::counter_handle(), cfg)
        .await;

    // B has never received any event; its digest_watch holds the
    // initial empty Vec. The target matches the initial value, but
    // since no fresh `changed()` has arrived, `await_digest` must
    // block until the timeout (it must NOT return on the stale match).
    let target: Vec<u8> = Vec::new();
    let returned = peer_b
        .await_digest(target, Duration::from_millis(100))
        .await;
    assert!(
        !returned,
        "await_digest must NOT return true on the stale initial digest \
         — it must wait for a fresh changed() signal first (review Q-3)"
    );
}

/// Covers: convergence.md §4.4 — `dropped_at_apply` (review-finding M-4).
///
/// Events rejected by `state-apply` during `replay_full` must be
/// recorded in `RuntimeHandle::dropped_at_apply`, not silently dropped.
/// The event remains in the DAG (so a future topo ordering may
/// re-accept it on a peer with different prior state) but never commits
/// to local `state`. Surfacing the drop makes the spec-mandated map
/// observable for diagnostics.
///
/// ## Test design (approach (c) per plan Task 13)
///
/// The pre-check-rejector fixture's wasm `apply` function returns
/// `Reject("not allowed")` unconditionally. Per spec §4.4, pre-check
/// and apply are the SAME wasm function called in dry-run vs canonical
/// modes — both Reject. That makes the originator path unusable for
/// this test: `Runtime::author` calls `pre_check` before signing, sees
/// `Rejected`, and returns `RuntimeError::PreCheckRejected` without
/// ever inserting the event into the DAG. The event never reaches
/// `replay_full`, so `dropped_at_apply` would stay empty.
///
/// The apply-time-only scope is reached by bypassing the originator's
/// pre-check entirely: hand-construct a signed Event via `EventBuilder`
/// and publish it onto the bus as a `GossipMessage::Event`. Peer B
/// (running the pre-check-rejector handle) receives the event via its
/// recv loop, inserts it into the DAG (DAG insertion does NOT consult
/// `state-apply`; only sig / chain / equivocation checks gate that),
/// then runs `replay_full`, which calls `apply` over the topo order
/// and lands the event in `dropped_at_apply` with the reject reason.
///
/// (Alternative approaches considered: (a) build a new wasm fixture
/// that accepts in pre-check and rejects in apply — heavy: needs a new
/// fixture, new build step, new manifest. (b) Stub `StateApplyHandle`
/// without crossing the wasm boundary — requires `StateApplyHandle` to
/// be `dyn`-friendly or a new test seam. (c) Bypass pre-check via
/// `MemBus` injection — lightest; mirrors the equivocation-via-membus
/// test pattern from Batch 5. Chose (c) per plan-task guidance.)
#[tokio::test]
async fn dropped_at_apply_records_rejected_events() {
    use myrhiza_kernel::identity::AuthorKeypair;
    use myrhiza_network::{GossipMessage, MemBus, MemNetwork, Network};
    use myrhiza_test_utils::EventBuilder;
    use myrhiza_types::{BundleHash, Topic};

    // Shared bus + topic + bundle. Peer B subscribes to the bus and
    // runs the pre-check-rejector handle so its `replay_full` rejects
    // every event.
    let bus = MemBus::new(256);
    let app_bundle_hash = BundleHash::from_bytes([0xDE; 32]);
    let topic_name = "main".to_string();
    let seed = [0x55u8; 32];
    let topic = Topic::derive(&app_bundle_hash, &seed, &topic_name);

    // Long heads-summary tick + permissive drift so neither perturbs
    // the test surface. Matches the equivocation-via-membus pattern.
    let cfg = RuntimeCfg {
        drift_interval: 1,
        drift_min_interval: Duration::from_secs(0),
        drift_daily_cap: u32::MAX,
        heads_summary_tick: Duration::from_hours(1),
        pending_cfg: PendingCfg::default(),
        broadcast_capacity: 256,
        kernel_fuel_table_version: 1,
        drift_stash_cap: 256,
        transport_error_halt_threshold: 5,
    };

    // Spawn read-only B with the rejector handle. No author key — B
    // never authors, only observes the event we inject.
    let peer_key_b3 = myrhiza_kernel::identity::PeerKeypair::deterministic(2);
    let net_b = MemNetwork::new(bus.clone(), peer_key_b3.public);
    let runtime_b = myrhiza_kernel::runtime::Runtime::start(
        net_b,
        topic,
        app_bundle_hash,
        topic_name.clone(),
        helpers::pre_check_rejector_handle(),
        peer_key_b3,
        None,
        cfg,
    )
    .await
    .expect("runtime_b");

    // Give B's subscription + startup HeadsSummary a chance to settle
    // so the injected event is actually delivered to B's recv loop.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Hand-construct a signed genesis event from a hostile-publisher
    // perspective (not going through `Runtime::author`, which would
    // be vetoed by pre-check). The pre-check-rejector fixture rejects
    // every event regardless of payload bytes, so the payload shape is
    // not load-bearing here — but using a well-formed `GenesisV1`
    // payload keeps the test surface honest (an event that passed the
    // DAG's structural checks would reach `replay_full` in real
    // deployment too).
    let kp_a = AuthorKeypair::deterministic(1);
    let builder = EventBuilder::new(&kp_a);
    let genesis = builder.genesis(
        &app_bundle_hash,
        seed,
        &topic_name,
        0_i64.to_be_bytes().to_vec(),
    );
    let genesis_hash = genesis.wire_hash();

    // Publish the event directly onto the bus. B's recv loop will pick
    // it up, the DAG accepts it (sig + chain valid; DAG does NOT run
    // state-apply), then `replay_full` rejects it and records the drop.
    let net_pub = MemNetwork::new(
        bus.clone(),
        myrhiza_types::PeerPubkey::from_bytes([0xC3; 32]),
    );
    net_pub
        .publish(topic, GossipMessage::Event(genesis.clone()))
        .await
        .expect("publish genesis");

    // Poll B's dropped_at_apply until it records the rejection, or the
    // deadline expires. This mirrors the equivocation-via-membus poll
    // pattern and avoids racing the recv loop with a fixed-sleep.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let map = loop {
        let snapshot = runtime_b
            .dropped_at_apply
            .lock()
            .expect("dropped_at_apply mutex")
            .clone();
        if !snapshot.is_empty() {
            break snapshot;
        }
        if std::time::Instant::now() >= deadline {
            break snapshot;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    };

    assert_eq!(
        map.len(),
        1,
        "B must record exactly one dropped_at_apply entry for the \
         injected event the pre-check-rejector handle refused to \
         apply; saw map={map:?}"
    );
    let reason = map
        .get(&genesis_hash)
        .expect("dropped_at_apply must be keyed by the event's wire_hash");
    assert_eq!(
        reason, "not allowed",
        "reject reason must match the fixture's hard-coded string"
    );

    // Cleanup: shutdown B's runtime so the test exits cleanly.
    let _ = runtime_b
        .author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Shutdown)
        .await;
}
