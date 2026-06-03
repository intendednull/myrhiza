//! B-4.2 acceptance tests for sender attribution on `HeadsSummary`
//! plus drop-as-unsubscribe semantics. The companion `HeadsRequest`
//! attribution tests were retired in B-4.7 (§3.7) along with the
//! gossip-routed `HeadsRequest` surface.
//!
//! Per docs/specs/2026-05-20-plan-b-4-2-attribution-design.md §5.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::time::Duration;

use bincode::Options;
use myrhiza_kernel::identity::PeerKeypair;
use myrhiza_kernel::runtime::{PeerWarning, Runtime, RuntimeCfg};
use myrhiza_network::{GossipMessage, MemBus, MemNetwork, Network};
use myrhiza_types::{
    AuthorHead, AuthorPubkey, AuthorSeq, BundleHash, DriftAnchor, DriftMessage, EventHash,
    HeadsSummary, HeadsSummarySignedPayload, Topic, canonical_bincode,
};

mod helpers;

// ============================================================================
// Tests 1-2: sign/verify round-trip (pure-types, default flavor)
// ============================================================================

/// Covers: convergence.md §4.2
///
/// Sign a `HeadsSummary` exactly as the runtime does; reconstruct the
/// `HeadsSummarySignedPayload` on the verifier side; assert
/// `myrhiza_manifest::verify_signature` returns `Ok(())`.
#[test]
fn heads_summary_sign_then_verify_roundtrips() {
    let kp = PeerKeypair::deterministic(1);
    let topic = Topic::from_bytes([0xAA; 32]);
    let authors = vec![AuthorHead {
        author: AuthorPubkey::from_bytes([1; 32]),
        seq: 5,
        hash: EventHash::ZERO,
    }];
    let kernel_fuel_table_version = 1;

    // --- sign side (mirrors Runtime::publish_heads_summary) ---
    let signed_payload = HeadsSummarySignedPayload {
        authors: authors.clone(),
        kernel_fuel_table_version,
        topic,
    };
    let sign_bytes = canonical_bincode()
        .serialize(&signed_payload)
        .expect("encode signed payload");
    let signature = kp.sign(&sign_bytes);

    let msg = HeadsSummary {
        authors: authors.clone(),
        kernel_fuel_table_version,
        signed_by_peer: kp.public,
        signature,
    };

    // --- verify side (mirrors Runtime::verify_heads_summary) ---
    let verify_payload = HeadsSummarySignedPayload {
        authors: msg.authors.clone(),
        kernel_fuel_table_version: msg.kernel_fuel_table_version,
        topic,
    };
    let verify_bytes = canonical_bincode()
        .serialize(&verify_payload)
        .expect("encode verify payload");
    myrhiza_manifest::verify_signature(
        msg.signed_by_peer.as_bytes(),
        &verify_bytes,
        &msg.signature,
    )
    .expect("HeadsSummary signature must verify");
}

// ============================================================================
// Tests 3-4: bad-signature rejection (pure-types, default flavor)
// ============================================================================

/// Covers: convergence.md §4.6
///
/// Flip a single bit in a valid `HeadsSummary` signature and assert
/// `verify_signature` returns `Err(_)`.
#[test]
fn verify_rejects_bad_signature_heads_summary() {
    let kp = PeerKeypair::deterministic(3);
    let topic = Topic::from_bytes([0xCC; 32]);
    let authors = vec![AuthorHead {
        author: AuthorPubkey::from_bytes([3; 32]),
        seq: 1,
        hash: EventHash::ZERO,
    }];

    let payload = HeadsSummarySignedPayload {
        authors: authors.clone(),
        kernel_fuel_table_version: 1,
        topic,
    };
    let bytes = canonical_bincode().serialize(&payload).expect("encode");
    let mut signature = kp.sign(&bytes);

    // Flip a bit in the first byte of the signature.
    signature[0] ^= 0xFF;

    let msg = HeadsSummary {
        authors,
        kernel_fuel_table_version: 1,
        signed_by_peer: kp.public,
        signature,
    };

    let verify_payload = HeadsSummarySignedPayload {
        authors: msg.authors.clone(),
        kernel_fuel_table_version: msg.kernel_fuel_table_version,
        topic,
    };
    let verify_bytes = canonical_bincode()
        .serialize(&verify_payload)
        .expect("encode");
    let result = myrhiza_manifest::verify_signature(
        msg.signed_by_peer.as_bytes(),
        &verify_bytes,
        &msg.signature,
    );
    assert!(
        result.is_err(),
        "verify_signature must return Err on bit-flipped HeadsSummary signature"
    );
}

// ============================================================================
// Tests 5-6: cross-topic replay defense (pure-types, default flavor)
// ============================================================================

/// Covers: convergence.md §4.6
///
/// Sign a `HeadsSummary` for topic X. Attempt to verify the same
/// wire-bytes using the signed payload reconstructed with topic Y.
/// Verification must fail — the topic is inside the signed payload,
/// so a replay across topics breaks the signature by construction.
#[test]
fn verify_rejects_cross_topic_replay_heads_summary() {
    let kp = PeerKeypair::deterministic(5);
    let topic_x = Topic::from_bytes([0x11; 32]);
    let topic_y = Topic::from_bytes([0x22; 32]);
    assert_ne!(
        topic_x, topic_y,
        "topics must differ for the test to be meaningful"
    );

    let authors = vec![AuthorHead {
        author: AuthorPubkey::from_bytes([5; 32]),
        seq: 3,
        hash: EventHash::ZERO,
    }];

    // Sign under topic X.
    let payload_x = HeadsSummarySignedPayload {
        authors: authors.clone(),
        kernel_fuel_table_version: 1,
        topic: topic_x,
    };
    let sign_bytes = canonical_bincode().serialize(&payload_x).expect("encode");
    let signature = kp.sign(&sign_bytes);

    let msg = HeadsSummary {
        authors: authors.clone(),
        kernel_fuel_table_version: 1,
        signed_by_peer: kp.public,
        signature,
    };

    // Verify side: recipient on topic Y reconstructs the payload with topic Y.
    // The wire bytes (msg) are exactly what an attacker would replay —
    // they don't carry `topic`, so no re-encoding needed. The verifier
    // reconstructs the signed payload from the subscription's self.topic.
    let payload_y = HeadsSummarySignedPayload {
        authors: msg.authors.clone(),
        kernel_fuel_table_version: msg.kernel_fuel_table_version,
        topic: topic_y, // <-- attacker's target topic, not the signing topic
    };
    let verify_bytes = canonical_bincode().serialize(&payload_y).expect("encode");
    let result = myrhiza_manifest::verify_signature(
        msg.signed_by_peer.as_bytes(),
        &verify_bytes,
        &msg.signature,
    );
    assert!(
        result.is_err(),
        "cross-topic replay of HeadsSummary must fail verification: \
         signature was made under topic X but verifier reconstructed topic Y"
    );
}

// ============================================================================
// Tests 7-10: runtime-level dispatch (multi_thread, worker_threads = 2)
// ============================================================================

/// Covers: convergence.md §4.4
///
/// Two-peer `MemNetwork`. Peer A hand-forges a `HeadsSummary` with a
/// deliberately wrong signature and publishes it via `MemNetwork::publish`
/// directly (bypassing `Runtime::publish_heads_summary`, which signs
/// correctly). Peer B's runtime receives the message.
///
/// Assertion: `peer_warnings` on B accumulates `PeerWarning::SignatureInvalid`.
/// (The B-4.2 assertion that B does not issue a gossip-routed `HeadsRequest`
/// was removed by B-4.7 — that surface no longer exists.)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_drops_heads_summary_with_bad_signature() {
    let bus = MemBus::new(256);
    let app_bundle_hash = BundleHash::from_bytes([0xA1; 32]);
    let topic_name = "main".to_string();
    let seed = [0x01u8; 32];
    let topic = Topic::derive(&app_bundle_hash, &seed, &topic_name);

    let kp_a = PeerKeypair::deterministic(11);
    let pub_a = kp_a.public;
    let pub_b = PeerKeypair::deterministic(12).public;
    assert_ne!(pub_a, pub_b);

    // Spawn peer B (read-only).
    let peer_kp_b_t7 = PeerKeypair::deterministic(12);
    let net_b = MemNetwork::new(bus.clone(), peer_kp_b_t7.public);
    let runtime_b = Runtime::start(
        net_b,
        topic,
        app_bundle_hash,
        topic_name.clone(),
        helpers::counter_handle(),
        peer_kp_b_t7,
        None,
        helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
        vec![],
        vec![],
    )
    .await
    .expect("runtime_b start");

    // Give B's startup HeadsSummary a chance to flush.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Hand-forge a bad-sig HeadsSummary: signed_by_peer = A's pubkey
    // but signature = [0xFF; 64] (definitely wrong).
    let bad_sig_summary = HeadsSummary {
        authors: vec![AuthorHead {
            author: AuthorPubkey::from_bytes([0xAA; 32]),
            seq: 1,
            hash: EventHash::ZERO,
        }],
        kernel_fuel_table_version: 1,
        signed_by_peer: pub_a,
        signature: [0xFF; 64],
    };

    // Synthetic bus-injection MemNetwork — no Runtime attached, no
    // install_request_handler call from this MemNetwork. Pubkey choice
    // is arbitrary. Cited: B-4.5 spec §4.2 carryover audit.
    let net_a = MemNetwork::new(
        bus.clone(),
        myrhiza_types::PeerPubkey::from_bytes([0xE2; 32]),
    );
    net_a
        .publish(topic, GossipMessage::HeadsSummary(bad_sig_summary))
        .await
        .expect("A publish bad-sig HeadsSummary");

    // Give B time to receive and process the message.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // B must have accumulated a SignatureInvalid warning.
    let warnings = runtime_b
        .peer_warnings
        .lock()
        .expect("peer_warnings mutex")
        .clone();
    let sig_invalid_count = warnings
        .iter()
        .filter(|w| matches!(w, PeerWarning::SignatureInvalid { .. }))
        .count();
    assert_eq!(
        sig_invalid_count, 1,
        "B must record exactly one SignatureInvalid warning; saw warnings={warnings:?}"
    );

    // Cleanup.
    let _ = runtime_b
        .author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Shutdown)
        .await;
}

/// Covers: convergence.md §4.7 — bad-signature drift surfaces as `PeerWarning::SignatureInvalid` (B-4.8 carryover).
///
/// Mirrors the `runtime_drops_heads_summary_with_bad_signature` shape
/// but on the drift path. Peer A hand-forges a `DriftMessage` with a
/// deliberately wrong signature; peer B's `process_drift_message`
/// rejects the signature AND (post-B-4.8) pushes a `SignatureInvalid`
/// warning instead of silently dropping. Per B-4.2 §10 carryover.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_drops_drift_with_bad_signature() {
    let bus = MemBus::new(256);
    let app_bundle_hash = BundleHash::from_bytes([0xA1; 32]);
    let topic_name = "main".to_string();
    let seed = [0x01u8; 32];
    let topic = Topic::derive(&app_bundle_hash, &seed, &topic_name);

    let kp_a = PeerKeypair::deterministic(81);
    let pub_a = kp_a.public;
    let pub_b = PeerKeypair::deterministic(82).public;
    assert_ne!(pub_a, pub_b);

    let peer_kp_b = PeerKeypair::deterministic(82);
    let net_b = MemNetwork::new(bus.clone(), peer_kp_b.public);
    let runtime_b = Runtime::start(
        net_b,
        topic,
        app_bundle_hash,
        topic_name.clone(),
        helpers::counter_handle(),
        peer_kp_b,
        None,
        helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
        vec![],
        vec![],
    )
    .await
    .expect("runtime_b start");

    // Give B's startup HeadsSummary a chance to flush.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Hand-forge a bad-sig DriftMessage with an empty anchor (covers
    // vacuously since `author_seq_vec` is empty; means the anchor-covered
    // check passes trivially in process_drift_message). The signature is
    // [0xFF; 64] — definitely wrong against any real peer key.
    let bad_sig_drift = DriftMessage {
        anchor: DriftAnchor {
            event_hash: EventHash::ZERO,
            author_seq_vec: Vec::<AuthorSeq>::new(),
        },
        digest: [0xCC; 32],
        digest_format: "bincode-1.3".into(),
        signed_by_peer: pub_a,
        signature: [0xFF; 64],
    };

    // Synthetic bus-injection MemNetwork (no Runtime attached).
    let net_inject = MemNetwork::new(
        bus.clone(),
        myrhiza_types::PeerPubkey::from_bytes([0xE5; 32]),
    );
    net_inject
        .publish(topic, GossipMessage::Drift(bad_sig_drift))
        .await
        .expect("publish bad-sig drift");

    // Give B time to receive and process the message.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // B must have accumulated exactly one SignatureInvalid warning
    // attributed to the claimed peer pub_a (the verify failed; the
    // warning records the *claimed* identity for observability).
    let warnings = runtime_b
        .peer_warnings
        .lock()
        .expect("peer_warnings mutex")
        .clone();
    let sig_invalid_count = warnings
        .iter()
        .filter(|w| matches!(w, PeerWarning::SignatureInvalid { peer: Some(p) } if *p == pub_a))
        .count();
    assert_eq!(
        sig_invalid_count, 1,
        "B must record exactly one SignatureInvalid warning attributed to pub_a; saw warnings={warnings:?}"
    );

    let _ = runtime_b
        .author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Shutdown)
        .await;
}

/// Covers: convergence.md §4.4
///
/// Sanity-check counterpart to test 7: peer A's REAL
/// `publish_heads_summary` produces a properly-signed `HeadsSummary` that
/// peer B accepts (no `SignatureInvalid` warning surfaced). Ensures
/// B-4.2's verify-then-dispatch wiring doesn't break existing convergence.
///
/// Shape: two runtimes on a shared bus. A triggers a heads-summary by
/// using a short tick. After a brief wait, assert B has no
/// `SignatureInvalid` warnings.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_accepts_heads_summary_with_good_signature() {
    let bus = MemBus::new(256);
    let app_bundle_hash = BundleHash::from_bytes([0xA3; 32]);
    let topic_name = "main".to_string();
    let seed = [0x03u8; 32];
    let topic = Topic::derive(&app_bundle_hash, &seed, &topic_name);

    // Use a very short heads_summary_tick so A emits quickly.
    let cfg_a = RuntimeCfg {
        drift_interval: 1,
        drift_min_interval: Duration::from_secs(0),
        drift_daily_cap: u32::MAX,
        heads_summary_tick: Duration::from_millis(50),
        distribution_sync_tick: Duration::from_millis(50),
        pending_cfg: myrhiza_kernel::pending::PendingCfg::default(),
        broadcast_capacity: 256,
        kernel_fuel_table_version: 1,
        drift_stash_cap: 256,
        transport_error_halt_threshold: 5,
    };

    let pub_a = PeerKeypair::deterministic(15).public;
    let pub_b = PeerKeypair::deterministic(16).public;
    assert_ne!(pub_a, pub_b);

    // Spawn peer A (author-capable — we want it to emit HeadsSummary).
    let kp_a_for_runtime = PeerKeypair::deterministic(15);
    let net_a = MemNetwork::new(bus.clone(), kp_a_for_runtime.public);
    let runtime_a = Runtime::start(
        net_a,
        topic,
        app_bundle_hash,
        topic_name.clone(),
        helpers::counter_handle(),
        kp_a_for_runtime,
        Some(myrhiza_kernel::identity::AuthorKeypair::deterministic(15)),
        cfg_a,
        vec![],
        vec![],
    )
    .await
    .expect("runtime_a start");

    // Spawn peer B (read-only) with the default (long) tick.
    let kp_for_b_runtime = PeerKeypair::deterministic(16);
    let net_b = MemNetwork::new(bus.clone(), kp_for_b_runtime.public);
    let runtime_b = Runtime::start(
        net_b,
        topic,
        app_bundle_hash,
        topic_name.clone(),
        helpers::counter_handle(),
        kp_for_b_runtime,
        None,
        helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
        vec![],
        vec![],
    )
    .await
    .expect("runtime_b start");

    // Wait long enough for A's tick to fire and B to receive + process it.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // B must have no SignatureInvalid warnings.
    let warnings = runtime_b
        .peer_warnings
        .lock()
        .expect("peer_warnings mutex")
        .clone();
    let sig_invalid = warnings
        .iter()
        .any(|w| matches!(w, PeerWarning::SignatureInvalid { .. }));
    assert!(
        !sig_invalid,
        "B must NOT record SignatureInvalid for a correctly-signed HeadsSummary from A; \
         saw warnings={warnings:?}"
    );

    // Cleanup.
    let _ = runtime_b
        .author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Shutdown)
        .await;
    let _ = runtime_a
        .author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Shutdown)
        .await;
}

/// Covers: convergence.md §4.4 — User-visible contract: a single-peer
/// runtime publishing its own `HeadsSummary` does NOT accumulate
/// spurious `PeerWarning::SignatureInvalid` warnings.
///
/// NOTE on mechanism: this test verifies the USER-VISIBLE CONTRACT (no
/// spurious warnings on own publishes), NOT the loopback filter's
/// mechanism directly. A broken loopback filter would also pass this
/// test if the runtime's own signature happens to verify (which it
/// does in normal operation — self-publishes use real Ed25519 sigs).
/// The mechanism check belongs in a unit test of `verify_heads_summary`
/// that asserts the loopback equality branch fires; that's a follow-up
/// if observability becomes important.
///
/// What this test PROVES:
/// - One-peer runtime running over `MemNetwork` echoes own publishes
///   through the broadcast channel
/// - The runtime processes the echo without surfacing
///   `SignatureInvalid` (user-visible contract)
///
/// Per B-4.2 spec §2 "Loopback filter" row.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_loopback_filter_skips_own_heads_summary_verify() {
    let bus = MemBus::new(256);
    let app_bundle_hash = BundleHash::from_bytes([0xA4; 32]);
    let topic_name = "main".to_string();
    let seed = [0x04u8; 32];
    let topic = Topic::derive(&app_bundle_hash, &seed, &topic_name);

    // Short tick so the runtime emits its HeadsSummary quickly.
    let cfg = RuntimeCfg {
        drift_interval: 1,
        drift_min_interval: Duration::from_secs(0),
        drift_daily_cap: u32::MAX,
        heads_summary_tick: Duration::from_millis(50),
        distribution_sync_tick: Duration::from_millis(50),
        pending_cfg: myrhiza_kernel::pending::PendingCfg::default(),
        broadcast_capacity: 256,
        kernel_fuel_table_version: 1,
        drift_stash_cap: 256,
        transport_error_halt_threshold: 5,
    };

    let peer_kp_t10 = PeerKeypair::deterministic(17);
    let net = MemNetwork::new(bus.clone(), peer_kp_t10.public);
    let runtime = Runtime::start(
        net,
        topic,
        app_bundle_hash,
        topic_name.clone(),
        helpers::counter_handle(),
        peer_kp_t10,
        None,
        cfg,
        vec![],
        vec![],
    )
    .await
    .expect("runtime start");

    // Wait long enough for the tick to fire (50ms) + echo to arrive + runtime
    // to process. 300ms is generous.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // The loopback filter must have caught the echo — no SignatureInvalid.
    let warnings = runtime
        .peer_warnings
        .lock()
        .expect("peer_warnings mutex")
        .clone();
    let sig_invalid = warnings
        .iter()
        .any(|w| matches!(w, PeerWarning::SignatureInvalid { .. }));
    assert!(
        !sig_invalid,
        "loopback filter must prevent SignatureInvalid for own HeadsSummary echo on MemNetwork; \
         saw warnings={warnings:?}"
    );

    // Cleanup.
    let _ = runtime
        .author_tx
        .send(myrhiza_kernel::runtime::AuthorCommand::Shutdown)
        .await;
}

// ============================================================================
// Tests 11-12: IrohNetwork unsubscribe + drop-as-leave (network-iroh feature)
// ============================================================================

/// Covers: networking.md §11.1
///
/// Construct an `IrohNetwork`, call `unsubscribe(topic).await`, assert
/// `Ok(())`. Pure invariant test — does NOT require a real swarm.
/// Per spec §3.3: `IrohNetwork::unsubscribe` is semantically a no-op at
/// the method boundary (iroh-gossip 0.99.0 has no explicit "leave swarm"
/// API; cleanup happens via subscription drop).
#[cfg(feature = "network-iroh")]
#[tokio::test]
async fn unsubscribe_returns_ok() {
    use myrhiza_network::IrohNetwork;

    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .bind()
        .await
        .expect("iroh endpoint bind");
    let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
    let _router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();
    let network = IrohNetwork::new(endpoint, gossip);

    let topic = Topic::from_bytes([0xF1; 32]);
    let result = network.unsubscribe(topic).await;
    assert!(
        result.is_ok(),
        "IrohNetwork::unsubscribe must return Ok(()); got {result:?}"
    );
}

/// Covers: networking.md §11.1 — `IrohNetwork::publish` and the
/// underlying gossip actor remain functional after a peer drops its
/// `IrohSubscription`. This is a regression check for "drop +
/// publish-after-drop doesn't crash the gossip actor", not the
/// stronger property "B actually left the swarm at the actor level"
/// (which iroh-gossip 0.99.0 doesn't expose for assertion;
/// actor-internal swarm-state verification is B-4.3 cross-process
/// scope — see spec §11 future work).
///
/// What this test PROVES:
/// - Two iroh peers can form a swarm via real `IrohNetwork::subscribe`
/// - Dropping `IrohSubscription` doesn't panic
/// - `IrohNetwork::publish` after a peer drop completes Ok(_) — the
///   gossip actor on the publisher's side is not corrupted by the
///   neighbor's subscription drop.
///
/// What this test does NOT prove (gap acknowledged):
/// - The actor on B's side actually signaled `NeighborDown` to A
/// - A's swarm-membership state reflects B's leave
///
/// Per B-4.2 spec §3.3 + §10 (real-cross-process tests deferred).
#[cfg(feature = "network-iroh")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_publish_after_subscription_drop_does_not_error() {
    use iroh::address_lookup::MemoryLookup;
    use myrhiza_network::IrohNetwork;
    use myrhiza_types::PeerPubkey;

    let lookup = MemoryLookup::new();

    // Peer A.
    let ep_a = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .address_lookup(lookup.clone())
        .bind()
        .await
        .expect("endpoint A bind");
    lookup.add_endpoint_info(ep_a.addr());
    let gossip_a = iroh_gossip::Gossip::builder().spawn(ep_a.clone());
    let _router_a = iroh::protocol::Router::builder(ep_a.clone())
        .accept(iroh_gossip::ALPN, gossip_a.clone())
        .spawn();
    let net_a = IrohNetwork::new(ep_a.clone(), gossip_a.clone());

    // Peer B.
    let ep_b = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .address_lookup(lookup.clone())
        .bind()
        .await
        .expect("endpoint B bind");
    lookup.add_endpoint_info(ep_b.addr());
    let gossip_b = iroh_gossip::Gossip::builder().spawn(ep_b.clone());
    let _router_b = iroh::protocol::Router::builder(ep_b.clone())
        .accept(iroh_gossip::ALPN, gossip_b.clone())
        .spawn();
    let net_b = IrohNetwork::new(ep_b.clone(), gossip_b.clone());

    let a_pk = net_a.peer_pubkey();
    let topic = Topic::from_bytes([0xF2; 32]);

    // Subscribe both peers; B bootstraps from A.
    let _sub_a = net_a.subscribe(topic, vec![]).await.expect("A subscribe");
    let sub_b = net_b
        .subscribe(topic, vec![a_pk])
        .await
        .expect("B subscribe");

    // Wait for swarm formation.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // B drops its subscription. Per iroh-gossip 0.99.0 (`api.rs:355-363`),
    // drop is the only public path to "leave swarm" — but this test does
    // NOT verify the actor-internal leave signal (see docstring above).
    drop(sub_b);

    // Give the actor time to process the drop.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // A publishes after B's drop. The assertion is that `publish` returns
    // Ok(_) — the gossip actor on A's side is not corrupted by the
    // neighbor's drop. This is a regression check for "drop +
    // publish-after-drop doesn't crash the gossip actor"; it does NOT
    // prove B actually left the swarm at the actor level.
    let summary = HeadsSummary {
        authors: vec![],
        kernel_fuel_table_version: 1,
        signed_by_peer: PeerPubkey::from_bytes([0; 32]),
        signature: [0; 64],
    };
    net_a
        .publish(topic, GossipMessage::HeadsSummary(summary))
        .await
        .expect("A publish after B drop must complete Ok(_)");
}
