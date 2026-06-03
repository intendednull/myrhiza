//! Kernel-tier revocation/publication propagation over real
//! iroh-gossip. Closes B-10 spec §6.4 (the deferred revocation
//! propagation test) and B-11 spec §6.3.
//!
//! Topology mirrors `iroh_convergence.rs`'s proven originator/receiver
//! shape: the PUBLISHER is the topic originator, and the RECEIVER (peer
//! A) joins the publisher's swarm by bootstrapping to it. Concretely the
//! publisher stack is spawned first and subscribes the per-author topic;
//! then the `IrohHarness` spawns peer A with `bootstrap=[publisher_pk]`
//! and `installed_authors=[author]`, so the `Runtime`'s auto-subscription
//! of author A's revocation/publication topic dials the publisher and
//! Plumtree forms a delivery route. The publisher then publishes the
//! signed envelope and the test polls peer A's poll-log surface.
//!
//! Why the receiver bootstraps to the publisher (not vice versa): a
//! freshly-subscribed iroh-gossip peer needs a bootstrap into an existing
//! swarm member to receive forwarded messages. `iroh_convergence.rs` has
//! the late/receiving peer bootstrap to the originator; we keep the same
//! direction so the swarm forms reliably.
//!
//! Timing (spec §12.2): a 300ms settle after peer A is spawned (matches
//! `iroh_convergence.rs`'s empirical swarm-formation window), then a 5s
//! bounded poll loop with 25ms ticks via `tokio::time::timeout`. Observed
//! locally: deliveries land within ~1s of publish once the swarm forms.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::time::Duration;

use ed25519_dalek::{Signer, SigningKey};
use myrhiza_distribution::topic::{derive_publication_topic, derive_revocation_topic};
use myrhiza_distribution::{PublicationEvent, RevocationEvent};
use myrhiza_network::{GossipMessage, Network};
use myrhiza_test_utils::harness::PeerHandle;
use myrhiza_test_utils::iroh_harness::{IrohHarness, IrohPeerStack, spawn_iroh_peer};
use myrhiza_types::{AuthorPubkey, BlobHash, Topic};

mod helpers;

/// Author under test. Seed 7 matches `deterministic_signing_key(7)` /
/// `build_signed_counter_bundle`'s author (spec §6.3).
fn author_key() -> (SigningKey, AuthorPubkey) {
    let sk = SigningKey::from_bytes(&[7u8; 32]);
    let pk = AuthorPubkey::from_bytes(sk.verifying_key().to_bytes());
    (sk, pk)
}

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

/// Spawn the publisher stack + receiving peer A, joined on `topic` and
/// settled. Returns `(publisher_stack, peer_a, retained_pub_sub)`.
///
/// Order is load-bearing (see module doc): publisher first + subscribed,
/// then peer A bootstrapped to the publisher so its auto-subscription
/// dials an existing swarm member. The publisher's `Subscription` is
/// returned so the caller keeps it alive — dropping it would tear down
/// the publisher's topic membership before the publish lands.
async fn setup(
    harness: &mut IrohHarness,
    author: AuthorPubkey,
    topic: Topic,
) -> (
    IrohPeerStack,
    PeerHandle,
    Box<dyn myrhiza_network::Subscription + Send>,
) {
    let publisher = spawn_iroh_peer(&harness.lookup, Some([200u8; 32]), true).await;
    let publisher_pk = publisher.network.peer_pubkey();
    let pub_sub = publisher
        .network
        .subscribe(topic, vec![])
        .await
        .expect("publisher subscribe topic");

    let peer_a = harness
        .spawn_peer(
            1,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::FAST_GOSSIP_TICK),
            vec![publisher_pk],
            vec![author],
        )
        .await;

    // Settle: let peer A's auto-subscription dial the publisher and the
    // Plumtree swarm form before the first publish.
    tokio::time::sleep(Duration::from_millis(300)).await;
    (publisher, peer_a, Box::new(pub_sub))
}

/// Poll `f` until it returns a non-empty `Vec` or 5s elapses, returning
/// the final observation. Tick is 25ms; the outer bound is enforced via
/// `tokio::time::timeout` so a wedged transport fails fast.
async fn poll_events<T>(mut f: impl FnMut() -> Vec<T>) -> Vec<T> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let v = f();
            if !v.is_empty() {
                return v;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revocation_gossip_applies_and_surfaces() {
    let mut harness = IrohHarness::new([0x41; 32]);
    let (sk, author) = author_key();
    let topic = Topic::from_bytes(derive_revocation_topic(author));
    let (publisher, peer_a, _pub_sub) = setup(&mut harness, author, topic).await;

    let revoked = BlobHash::from_bytes([0xAA; 32]);
    let ev = sign_revocation(&sk, revoked, "compromised", 1);
    publisher
        .network
        .publish(topic, GossipMessage::Revocation(ev))
        .await
        .expect("publish revocation");

    let events = poll_events(|| peer_a.revocation_events()).await;
    assert_eq!(events.len(), 1, "exactly one RevocationApplied over iroh");
    assert_eq!(events[0].author, author);
    assert_eq!(events[0].revoked_bundle_hash, revoked);
    assert_eq!(events[0].revocation_seq, 1);

    peer_a.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publication_gossip_applies_and_surfaces() {
    let mut harness = IrohHarness::new([0x42; 32]);
    let (sk, author) = author_key();
    let topic = Topic::from_bytes(derive_publication_topic(author));
    let (publisher, peer_a, _pub_sub) = setup(&mut harness, author, topic).await;

    let manifest = BlobHash::from_bytes([0xCC; 32]);
    let ev = sign_publication(&sk, manifest, "1.2.3", 1);
    publisher
        .network
        .publish(topic, GossipMessage::Publication(ev))
        .await
        .expect("publish publication");

    let events = poll_events(|| peer_a.publication_events()).await;
    assert_eq!(
        events.len(),
        1,
        "exactly one PublicationAnnounced over iroh"
    );
    assert_eq!(events[0].author, author);
    assert_eq!(events[0].manifest_hash, manifest);
    assert_eq!(events[0].version, "1.2.3");
    assert_eq!(events[0].publication_seq, 1);

    peer_a.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_signature_becomes_peer_warning() {
    let mut harness = IrohHarness::new([0x43; 32]);
    let (_sk, author) = author_key();
    // Sign with the wrong key — edge verify rejects before apply.
    let wrong_sk = SigningKey::from_bytes(&[42u8; 32]);
    let topic = Topic::from_bytes(derive_revocation_topic(author));
    let (publisher, peer_a, _pub_sub) = setup(&mut harness, author, topic).await;

    let ev = sign_revocation(&wrong_sk, BlobHash::from_bytes([0xAA; 32]), "x", 1);
    publisher
        .network
        .publish(topic, GossipMessage::Revocation(ev))
        .await
        .expect("publish forged revocation");

    let warnings = poll_events(|| peer_a.peer_warnings()).await;
    assert!(
        warnings.iter().any(|w| matches!(
            w,
            myrhiza_kernel::runtime::PeerWarning::SignatureInvalid { peer: None }
        )),
        "forged revocation must surface SignatureInvalid over iroh, got {warnings:?}"
    );
    assert!(
        peer_a.revocation_events().is_empty(),
        "forged revocation must not advance the log"
    );

    peer_a.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seq_not_monotonic_second_event_dropped() {
    let mut harness = IrohHarness::new([0x44; 32]);
    let (sk, author) = author_key();
    let topic = Topic::from_bytes(derive_revocation_topic(author));
    let (publisher, peer_a, _pub_sub) = setup(&mut harness, author, topic).await;

    // First event applies; second (same seq=1) is non-monotonic.
    let ev1 = sign_revocation(&sk, BlobHash::from_bytes([0xAA; 32]), "first", 1);
    publisher
        .network
        .publish(topic, GossipMessage::Revocation(ev1))
        .await
        .expect("publish ev1");

    let events = poll_events(|| peer_a.revocation_events()).await;
    assert_eq!(events.len(), 1, "first event applies over iroh");

    let ev2 = sign_revocation(&sk, BlobHash::from_bytes([0xBB; 32]), "second", 1);
    publisher
        .network
        .publish(topic, GossipMessage::Revocation(ev2))
        .await
        .expect("publish ev2");

    // Settle margin to let the second event arrive and be dropped.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        peer_a.revocation_events().len(),
        1,
        "non-monotonic second event must be dropped (exactly one applied)"
    );

    peer_a.shutdown().await;
}
