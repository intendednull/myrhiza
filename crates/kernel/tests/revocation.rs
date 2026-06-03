//! Kernel-tier revocation/publication subscription acceptance tests
//! over `MemNetwork` (fast, no iroh).
//!
//! Proves the B-11 wiring end-to-end at the in-process tier: a peer
//! spawned with `installed_authors=[A]` auto-subscribes author A's
//! derived revocation/publication topics, and inbound gossip on those
//! topics flows through `dispatch::verify_*` → `RevocationLog`/
//! `PublicationLog::apply` → the `revocation_events()`/
//! `publication_events()` poll-log surface.
//!
//! Per B-11 spec §6.2 / plan T4. The real-iroh analogues live in
//! `iroh_revocation.rs` (spec §6.3, closes B-10 spec §6.4).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::time::Duration;

use ed25519_dalek::{Signer, SigningKey};
use myrhiza_distribution::topic::{derive_publication_topic, derive_revocation_topic};
use myrhiza_distribution::{PublicationEvent, RevocationEvent};
use myrhiza_network::{GossipMessage, MemNetwork, Network};
use myrhiza_test_utils::InProcessHarness;
use myrhiza_types::{AuthorPubkey, BlobHash, PeerPubkey, Topic};

mod helpers;

/// The author key under test. Seed 7 matches
/// `build_signed_counter_bundle`'s author so the fixture is consistent
/// with the iroh-tier test (spec §6.3).
fn author_key() -> (SigningKey, AuthorPubkey) {
    let sk = SigningKey::from_bytes(&[7u8; 32]);
    let pk = AuthorPubkey::from_bytes(sk.verifying_key().to_bytes());
    (sk, pk)
}

/// Build a signed `RevocationEvent`. Signing with `sk`; the kernel
/// cross-checks against the `author` topic-owner.
fn sign_revocation(sk: &SigningKey, revoked: BlobHash, reason: &str, seq: u64) -> RevocationEvent {
    let mut ev = RevocationEvent {
        revoked_bundle_hash: revoked,
        reason: reason.into(),
        revoked_at: 0,
        revocation_seq: seq,
        signature: [0u8; 64],
    };
    ev.signature = sk.sign(&ev.signing_target()).to_bytes();
    ev
}

/// Build a signed `PublicationEvent`.
fn sign_publication(
    sk: &SigningKey,
    manifest: BlobHash,
    version: &str,
    seq: u64,
) -> PublicationEvent {
    let mut ev = PublicationEvent {
        manifest_hash: manifest,
        version: version.into(),
        publication_seq: seq,
        signature: [0u8; 64],
    };
    ev.signature = sk.sign(&ev.signing_target()).to_bytes();
    ev
}

/// Poll `f` until it returns a non-empty `Vec`, or `timeout` elapses.
/// Returns whatever the final poll observed (possibly empty on timeout).
async fn poll_until_nonempty<T>(timeout: Duration, mut f: impl FnMut() -> Vec<T>) -> Vec<T> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let v = f();
        if !v.is_empty() || std::time::Instant::now() >= deadline {
            return v;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// A publisher `MemNetwork` on the harness bus, identified by an
/// arbitrary peer key distinct from any spawned peer.
fn publisher(harness: &InProcessHarness) -> MemNetwork {
    MemNetwork::new(harness.bus.clone(), PeerPubkey::from_bytes([0xFE; 32]))
}

#[tokio::test]
async fn revocation_applied_on_valid_event() {
    let harness = InProcessHarness::new(256, [0x31; 32]);
    let (sk, author) = author_key();

    let peer = harness
        .spawn_peer(
            1,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
            vec![author],
        )
        .await;

    let revoked = BlobHash::from_bytes([0xAA; 32]);
    let ev = sign_revocation(&sk, revoked, "compromised", 1);

    publisher(&harness)
        .publish(
            Topic::from_bytes(derive_revocation_topic(author)),
            GossipMessage::Revocation(ev),
        )
        .await
        .expect("publish revocation");

    let events = poll_until_nonempty(Duration::from_secs(2), || peer.revocation_events()).await;
    assert_eq!(events.len(), 1, "exactly one revocation applied");
    assert_eq!(events[0].author, author);
    assert_eq!(events[0].revoked_bundle_hash, revoked);
    assert_eq!(events[0].revocation_seq, 1);
    assert!(
        peer.peer_warnings().is_empty(),
        "valid revocation must not produce any warning"
    );

    peer.shutdown().await;
}

#[tokio::test]
async fn invalid_sig_revocation_becomes_peer_warning() {
    let harness = InProcessHarness::new(256, [0x32; 32]);
    let (_sk, author) = author_key();
    // Sign with the WRONG key — the edge verify (dispatch::verify_revocation)
    // must reject before apply, classifying it as SignatureInvalid (not a
    // benign stale-seq drop). Spec §3.4.
    let wrong_sk = SigningKey::from_bytes(&[42u8; 32]);

    let peer = harness
        .spawn_peer(
            1,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
            vec![author],
        )
        .await;

    let ev = sign_revocation(&wrong_sk, BlobHash::from_bytes([0xAA; 32]), "x", 1);
    publisher(&harness)
        .publish(
            Topic::from_bytes(derive_revocation_topic(author)),
            GossipMessage::Revocation(ev),
        )
        .await
        .expect("publish revocation");

    let warnings = poll_until_nonempty(Duration::from_secs(2), || peer.peer_warnings()).await;
    assert!(
        warnings.iter().any(|w| matches!(
            w,
            myrhiza_kernel::runtime::PeerWarning::SignatureInvalid { peer: None }
        )),
        "forged-signature revocation must surface SignatureInvalid, got {warnings:?}"
    );
    assert!(
        peer.revocation_events().is_empty(),
        "forged revocation must not advance the log"
    );

    peer.shutdown().await;
}

#[tokio::test]
async fn seq_not_monotonic_second_event_dropped() {
    let harness = InProcessHarness::new(256, [0x33; 32]);
    let (sk, author) = author_key();

    let peer = harness
        .spawn_peer(
            1,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
            vec![author],
        )
        .await;

    let topic = Topic::from_bytes(derive_revocation_topic(author));
    let net = publisher(&harness);

    // Two events both at seq=1. The first applies; the second is a
    // non-monotonic duplicate (signature valid → classified as a
    // seq/length drop = DecodeFailed, not SignatureInvalid).
    let ev1 = sign_revocation(&sk, BlobHash::from_bytes([0xAA; 32]), "first", 1);
    let ev2 = sign_revocation(&sk, BlobHash::from_bytes([0xBB; 32]), "second", 1);
    net.publish(topic, GossipMessage::Revocation(ev1))
        .await
        .expect("publish ev1");
    net.publish(topic, GossipMessage::Revocation(ev2))
        .await
        .expect("publish ev2");

    // Wait for the first to apply, then confirm the second never adds a
    // second RevocationApplied even after a settle margin.
    let events = poll_until_nonempty(Duration::from_secs(2), || peer.revocation_events()).await;
    assert_eq!(events.len(), 1, "first event applies");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        peer.revocation_events().len(),
        1,
        "non-monotonic second event must be dropped (exactly one applied)"
    );
    assert_eq!(events[0].revocation_seq, 1);

    peer.shutdown().await;
}

#[tokio::test]
async fn publication_applied_on_valid_event() {
    let harness = InProcessHarness::new(256, [0x34; 32]);
    let (sk, author) = author_key();

    let peer = harness
        .spawn_peer(
            1,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
            vec![author],
        )
        .await;

    let manifest = BlobHash::from_bytes([0xCC; 32]);
    let ev = sign_publication(&sk, manifest, "1.2.3", 1);

    publisher(&harness)
        .publish(
            Topic::from_bytes(derive_publication_topic(author)),
            GossipMessage::Publication(ev),
        )
        .await
        .expect("publish publication");

    let events = poll_until_nonempty(Duration::from_secs(2), || peer.publication_events()).await;
    assert_eq!(events.len(), 1, "exactly one publication announced");
    assert_eq!(events[0].author, author);
    assert_eq!(events[0].manifest_hash, manifest);
    assert_eq!(events[0].version, "1.2.3");
    assert_eq!(events[0].publication_seq, 1);
    assert!(peer.peer_warnings().is_empty());

    peer.shutdown().await;
}

#[tokio::test]
async fn invalid_sig_publication_becomes_peer_warning() {
    let harness = InProcessHarness::new(256, [0x35; 32]);
    let (_sk, author) = author_key();
    let wrong_sk = SigningKey::from_bytes(&[42u8; 32]);

    let peer = harness
        .spawn_peer(
            1,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
            vec![author],
        )
        .await;

    let ev = sign_publication(&wrong_sk, BlobHash::from_bytes([0xCC; 32]), "1.0.0", 1);
    publisher(&harness)
        .publish(
            Topic::from_bytes(derive_publication_topic(author)),
            GossipMessage::Publication(ev),
        )
        .await
        .expect("publish publication");

    let warnings = poll_until_nonempty(Duration::from_secs(2), || peer.peer_warnings()).await;
    assert!(
        warnings.iter().any(|w| matches!(
            w,
            myrhiza_kernel::runtime::PeerWarning::SignatureInvalid { peer: None }
        )),
        "forged-signature publication must surface SignatureInvalid, got {warnings:?}"
    );
    assert!(peer.publication_events().is_empty());

    peer.shutdown().await;
}
