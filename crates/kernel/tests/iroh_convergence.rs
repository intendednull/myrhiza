//! Cross-peer convergence over real `IrohNetwork` — closes the in-process
//! portion of [reports/2026-05-21-mvp-gap-analysis.md] item 19.
//!
//! Mirrors `crates/kernel/tests/convergence.rs` but routes through a
//! real iroh-gossip swarm in-process. See spec §3.3.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeSet;
use std::time::Duration;

use bincode::Options;
use myrhiza_kernel::identity::AuthorKeypair;
use myrhiza_test_utils::iroh_harness::IrohHarness;
use myrhiza_types::{GenesisV1, canonical_bincode};

mod helpers;

/// Covers: mvp.md §15.1 #2
///
/// Closes the in-process iroh portion of E2E-1 design §3.3 row 1.
/// Mirrors `convergence.rs::single_originator_single_receiver_converges`
/// but routes events through real `IrohNetwork` (loopback UDP, iroh-gossip
/// Plumtree forwarding) rather than `MemNetwork`'s in-memory bus.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_single_originator_single_receiver_converges() {
    let mut harness = IrohHarness::new([0x11; 32]);
    let cfg = helpers::fast_cfg(helpers::FAST_GOSSIP_TICK);

    let peer_a = harness
        .spawn_peer(
            1,
            Some(1),
            helpers::counter_handle(),
            cfg.clone(),
            vec![],
            vec![],
        )
        .await;
    let peer_a_pk = harness.peer_pubkey(0);
    let mut peer_b = harness
        .spawn_peer(
            2,
            None,
            helpers::counter_handle(),
            cfg,
            vec![peer_a_pk],
            vec![],
        )
        .await;

    // Allow the iroh-gossip swarm a moment to form before peer A starts
    // publishing. Without this, Plumtree may drop the first event because
    // B's join is still in flight (see iroh_gossip.rs:133 for the
    // empirical 200ms convention).
    tokio::time::sleep(Duration::from_millis(300)).await;

    let kp_a = AuthorKeypair::deterministic(1);
    let initial = 0_i64.to_be_bytes().to_vec();
    let genesis_payload = GenesisV1 {
        seed: harness.seed,
        founder_pubkey: kp_a.author,
        app_payload: initial,
    };
    let genesis_bytes = canonical_bincode()
        .serialize(&genesis_payload)
        .expect("encode genesis payload");
    peer_a
        .author(genesis_bytes, BTreeSet::new())
        .await
        .expect("genesis");

    for delta in [1_i64, 2, -1] {
        peer_a
            .author(delta.to_be_bytes().to_vec(), BTreeSet::new())
            .await
            .expect("increment");
    }

    let expected_state = 2_i64.to_be_bytes().to_vec();
    assert!(
        peer_b
            .await_digest(expected_state.clone(), Duration::from_secs(10))
            .await,
        "peer B must converge to state {expected_state:?} over real iroh"
    );
}

/// Covers: convergence.md §4.1, mvp.md §15.1 #2
///
/// Closes the in-process iroh portion of E2E-1 design §3.3 row 2.
/// Mirrors `convergence.rs::concurrent_multi_author_converges`. The
/// 500ms pre-publish settle (vs 300ms in single-originator) matches
/// the three-peer settle in `iroh_gossip.rs:172` — concurrent authoring
/// during gossip warm-up is the most flake-prone path.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_concurrent_multi_author_converges() {
    let mut harness = IrohHarness::new([0x22; 32]);
    let cfg = helpers::fast_cfg(helpers::FAST_GOSSIP_TICK);

    let mut peer_a = harness
        .spawn_peer(
            1,
            Some(1),
            helpers::counter_handle(),
            cfg.clone(),
            vec![],
            vec![],
        )
        .await;
    let peer_a_pk = harness.peer_pubkey(0);
    let mut peer_b = harness
        .spawn_peer(
            2,
            Some(2),
            helpers::counter_handle(),
            cfg,
            vec![peer_a_pk],
            vec![],
        )
        .await;

    // Give the swarm time to settle before any author event. Three-peer
    // case uses 500ms (`iroh_gossip.rs:172`); this two-peer concurrent
    // case is comparably timing-sensitive because both peers publish
    // during the warm-up window.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Peer A authors genesis (founder = A).
    let kp_a = AuthorKeypair::deterministic(1);
    let genesis = GenesisV1 {
        seed: harness.seed,
        founder_pubkey: kp_a.author,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    let g_bytes = canonical_bincode().serialize(&genesis).expect("encode");
    peer_a
        .author(g_bytes, BTreeSet::new())
        .await
        .expect("genesis");

    // Wait up to 10s for B to ingest genesis before B authors.
    let initial_state = 0_i64.to_be_bytes().to_vec();
    assert!(
        peer_b
            .await_digest(initial_state, Duration::from_secs(10))
            .await,
        "peer B must ingest genesis before concurrent authoring begins"
    );

    // Concurrent authoring: A authors +1 and +2; B authors +10 and +20.
    // Canonical topo-sort yields 0 + 1 + 2 + 10 + 20 = 33 on both peers.
    for delta in [1_i64, 2] {
        peer_a
            .author(delta.to_be_bytes().to_vec(), BTreeSet::new())
            .await
            .expect("a inc");
    }
    for delta in [10_i64, 20] {
        peer_b
            .author(delta.to_be_bytes().to_vec(), BTreeSet::new())
            .await
            .expect("b inc");
    }

    let expected_state = 33_i64.to_be_bytes().to_vec();
    assert!(
        peer_a
            .await_digest(expected_state.clone(), Duration::from_secs(10))
            .await,
        "peer A must converge to state {expected_state:?} over real iroh"
    );
    assert!(
        peer_b
            .await_digest(expected_state.clone(), Duration::from_secs(10))
            .await,
        "peer B must converge to state {expected_state:?} over real iroh"
    );
}

/// Covers: mvp.md §15.1 #2, convergence.md §4.2
///
/// Closes the in-process iroh portion of E2E-1 design §3.3 row 3.
/// Mirrors `convergence.rs::late_joiner_backfills_via_heads_summary`.
/// Validates the Runtime-issued backfill path end-to-end over real
/// iroh: late-joining B observes a `HeadsSummary` from A's
/// `heads_summary_tick`, issues `request_heads` over real iroh, and
/// catches up via direct-stream backfill (B-4.4/4.5).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_late_joiner_backfills_via_heads_summary() {
    let mut harness = IrohHarness::new([0x33; 32]);
    let cfg = helpers::fast_cfg(helpers::FAST_GOSSIP_TICK);
    let peer_a = harness
        .spawn_peer(
            1,
            Some(1),
            helpers::counter_handle(),
            cfg.clone(),
            vec![],
            vec![],
        )
        .await;

    // A authors genesis + 5 increments BEFORE B joins.
    let kp_a = AuthorKeypair::deterministic(1);
    let genesis = GenesisV1 {
        seed: harness.seed,
        founder_pubkey: kp_a.author,
        app_payload: 0_i64.to_be_bytes().to_vec(),
    };
    let g_bytes = canonical_bincode().serialize(&genesis).expect("encode");
    peer_a
        .author(g_bytes, BTreeSet::new())
        .await
        .expect("genesis");
    for delta in [1_i64, 1, 1, 1, 1] {
        peer_a
            .author(delta.to_be_bytes().to_vec(), BTreeSet::new())
            .await
            .expect("inc");
    }

    // Now B joins. Its bootstrap is A's pubkey so it dials A
    // immediately and joins A's iroh-gossip swarm.
    let peer_a_pk = harness.peer_pubkey(0);
    let mut peer_b = harness
        .spawn_peer(
            2,
            None,
            helpers::counter_handle(),
            cfg,
            vec![peer_a_pk],
            vec![],
        )
        .await;

    // Expected: 0 + 5*1 = 5. The path is: A's
    // `heads_summary_tick` fires → `HeadsSummary` published → B sees gap
    // (its DAG has nothing for A) → B issues `request_heads` over real
    // iroh direct-stream → A's installed `KernelRequestHandler`
    // responds with all 6 events → B applies them.
    let expected_state = 5_i64.to_be_bytes().to_vec();
    assert!(
        peer_b
            .await_digest(expected_state, Duration::from_secs(15))
            .await,
        "late-joiner B must converge via HeadsSummary backfill over real iroh"
    );
}
