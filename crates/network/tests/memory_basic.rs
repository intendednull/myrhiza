//! In-process `MemNetwork`: subscribe, publish, recv across two handles.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use myrhiza_network::{GossipMessage, MemBus, MemNetwork, Network, Subscription};
use myrhiza_types::{AuthorHead, AuthorPubkey, EventHash, HeadsSummary, Topic};

fn topic(seed: u8) -> Topic {
    Topic::from_bytes([seed; 32])
}

fn sample_heads_summary() -> HeadsSummary {
    HeadsSummary {
        authors: vec![AuthorHead {
            author: AuthorPubkey::from_bytes([1; 32]),
            seq: 1,
            hash: EventHash::ZERO,
        }],
        kernel_fuel_table_version: 1,
    }
}

#[tokio::test]
async fn mem_network_delivers_to_subscriber() {
    let bus = MemBus::new(16);
    let net_a = MemNetwork::new(bus.clone());
    let net_b = MemNetwork::new(bus);

    let t = topic(1);
    let mut sub_b = net_b.subscribe(t).await.expect("subscribe");

    let msg = GossipMessage::HeadsSummary(sample_heads_summary());
    net_a.publish(t, msg.clone()).await.expect("publish");

    let received = sub_b.recv().await.expect("recv").expect("non-empty");
    match received {
        GossipMessage::HeadsSummary(h) => assert_eq!(h.kernel_fuel_table_version, 1),
        _ => panic!("wrong variant"),
    }
}

#[tokio::test]
async fn mem_network_topic_isolation() {
    let bus = MemBus::new(16);
    let net = MemNetwork::new(bus);

    let t1 = topic(1);
    let t2 = topic(2);
    let mut sub2 = net.subscribe(t2).await.expect("subscribe t2");

    // Publish on t1; sub2 (on t2) should NOT receive.
    net.publish(t1, GossipMessage::HeadsSummary(sample_heads_summary()))
        .await
        .expect("publish");

    // Use timeout: sub2 should never fire.
    let r = tokio::time::timeout(std::time::Duration::from_millis(50), sub2.recv()).await;
    assert!(r.is_err(), "sub2 must not receive cross-topic events");
}

#[tokio::test]
async fn mem_network_lag_surfaces_as_sub_error() {
    use myrhiza_network::SubError;
    let bus = MemBus::new(2); // capacity 2 — tiny on purpose
    let net = MemNetwork::new(bus);
    let t = topic(3);
    let mut sub = net.subscribe(t).await.expect("subscribe");

    // Flood 10 messages without recv'ing — broadcast channel will lag.
    for _ in 0..10 {
        net.publish(t, GossipMessage::HeadsSummary(sample_heads_summary()))
            .await
            .expect("publish");
    }

    // First recv after lag returns Err(Lagged).
    let r = sub.recv().await;
    assert!(
        matches!(r, Err(SubError::Lagged(_))),
        "first recv after overflow must be Lagged, got {r:?}"
    );
}

/// Covers: spec §6.3 — `MemBus::inject_lag(topic)` test affordance for
/// deterministic lag-recovery testing (review-finding M-3).
///
/// Without this affordance, the existing
/// `lagged_broadcast_recovers_via_heads_summary` convergence test
/// relies on natural capacity-overflow timing, which is intrinsically
/// consumer-speed-dependent. With `inject_lag` the next `recv` on any
/// subscriber of the named topic deterministically returns
/// `Err(SubError::Lagged(1))` exactly once.
///
/// Gated on `feature = "test-helpers"` because `MemBus::inject_lag`
/// itself carries the same gate (it is intentionally absent from the
/// non-test public surface). Workspace test runs enable the feature
/// transitively via the kernel/test-utils dev-dep chain.
#[cfg(feature = "test-helpers")]
#[tokio::test]
async fn inject_lag_forces_next_recv_to_return_lagged() {
    use myrhiza_network::SubError;
    let bus = MemBus::new(8);
    let net = MemNetwork::new(bus.clone());
    let t = topic(4);
    let mut sub = net.subscribe(t).await.expect("subscribe");

    bus.inject_lag(t);

    let r = sub.recv().await;
    assert!(
        matches!(r, Err(SubError::Lagged(_))),
        "expected Lagged after inject_lag, got {r:?}"
    );

    // After the forced lag fires once, the flag clears and normal
    // delivery resumes. Publish a real message and confirm it arrives.
    net.publish(t, GossipMessage::HeadsSummary(sample_heads_summary()))
        .await
        .expect("publish");
    let r2 = tokio::time::timeout(std::time::Duration::from_millis(200), sub.recv()).await;
    let delivered = r2.expect("recv did not time out").expect("recv ok");
    assert!(
        delivered.is_some(),
        "post-lag publish must deliver normally"
    );
}
