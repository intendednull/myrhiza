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
use myrhiza_kernel::pending::PendingCfg;
use myrhiza_kernel::runtime::RuntimeCfg;
use myrhiza_test_utils::iroh_harness::IrohHarness;
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

/// Covers: mvp.md §15.1 #2
///
/// Closes the in-process iroh portion of E2E-1 design §3.3 row 1.
/// Mirrors `convergence.rs::single_originator_single_receiver_converges`
/// but routes events through real `IrohNetwork` (loopback UDP, iroh-gossip
/// Plumtree forwarding) rather than `MemNetwork`'s in-memory bus.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_single_originator_single_receiver_converges() {
    let mut harness = IrohHarness::new([0x11; 32]);
    let cfg = fast_cfg();

    let peer_a = harness
        .spawn_peer(1, Some(1), helpers::counter_handle(), cfg.clone(), vec![])
        .await;
    let peer_a_pk = harness.peer_pubkey(0);
    let mut peer_b = harness
        .spawn_peer(2, None, helpers::counter_handle(), cfg, vec![peer_a_pk])
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
