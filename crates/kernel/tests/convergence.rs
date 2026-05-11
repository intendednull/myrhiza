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
