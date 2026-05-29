//! Kernel-tier stale-network backfill over real iroh-gossip. Closes
//! `distribution.md` §10.7 (the stale-network attack) and B-12 spec §14.5.
//!
//! These tests exercise the corrected **direct-stream pull** transport
//! (spec §14), which supersedes the original gossip-push design. §13
//! records *why*: gossip re-broadcast cannot catch up a late joiner over
//! real iroh-gossip — a freshly-joined peer's broadcasts do not reach an
//! established peer in-window (Plumtree eager-push is established→joiner;
//! the joiner→established path is lazy), and identical periodic summaries
//! are content-deduplicated. Pull sidesteps both: the behind peer DIALS the
//! advertiser point-to-point (QUIC), which bypasses the Plumtree asymmetry.
//!
//! Topology: a raw publisher stack anchors the per-author distribution
//! topic's swarm. An AHEAD peer C (a real `Runtime`, `installed_authors=
//! [author]`) bootstraps to the anchor and is seeded with a signed
//! revocation/publication **while the late joiner is still offline** — C
//! applies it and archives the signed envelope. Only then is the LATE
//! JOINER B spawned (bootstrapped to C); B never saw the original gossip.
//! C's on-start + periodic `RevocationHeads`/`PublicationHeads` broadcast
//! (every `FAST_GOSSIP_TICK` = 100ms) reaches B (established→joiner
//! eager-push works — the one direction proven reliable in §13). On hearing
//! C's *above-its-head* summary, B dials C via `request_distribution` and
//! pulls the missing signed envelopes from C's archive; B applies them
//! through the existing `handle_revocation`/`handle_publication` path and
//! surfaces them on its poll-log.
//!
//! Why the periodic tick matters here (not just the on-start broadcast):
//! C's summary must reach B *after* B's iroh-gossip subscription has formed
//! a Plumtree route into the swarm; the 100ms re-broadcast guarantees a
//! summary lands once the swarm forms (the ~300ms empirical window from
//! `iroh_convergence.rs`), at which point B dials and catches up. The
//! bounded poll then observes the catch-up. Observed settle timing
//! (2026-05-29, 3× parallel + 3× serial, all green, non-flaky): each test
//! completes its full lifecycle (anchor+ahead 300ms settle → seed → 300ms
//! mesh → poll) well inside the 15s timeout; the post-mesh catch-up lands
//! within ~1s of the late joiner spawning. Parallel pair ~0.95s in-test
//! (~1.3s wall); serial pair ~1.85s in-test.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::time::Duration;

use ed25519_dalek::{Signer, SigningKey};
use myrhiza_distribution::topic::{derive_publication_topic, derive_revocation_topic};
use myrhiza_distribution::{PublicationEvent, RevocationEvent};
use myrhiza_network::{GossipMessage, Network};
use myrhiza_test_utils::harness::PeerHandle;
use myrhiza_test_utils::iroh_harness::{IrohHarness, IrohPeerStack, spawn_iroh_peer};
use myrhiza_types::{AuthorPubkey, BlobHash, PeerPubkey, Topic};

mod helpers;

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

/// Poll `f` until non-empty or 5s elapses (25ms ticks; hard-bounded via
/// `tokio::time::timeout` so a wedged transport fails fast).
async fn poll_events<T>(mut f: impl FnMut() -> Vec<T>) -> Vec<T> {
    tokio::time::timeout(Duration::from_secs(15), async {
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

/// Spawn the anchor publisher + the ahead peer C (real `Runtime`), joined
/// and settled on `topic`. The publisher's subscription is returned so the
/// caller keeps it alive (dropping it tears down the swarm anchor).
async fn anchor_and_ahead(
    harness: &mut IrohHarness,
    author: AuthorPubkey,
    topic: Topic,
) -> (
    IrohPeerStack,
    Box<dyn myrhiza_network::Subscription + Send>,
    PeerHandle,
    PeerPubkey,
) {
    let publisher = spawn_iroh_peer(&harness.lookup, Some([200u8; 32]), true).await;
    let publisher_pk = publisher.network.peer_pubkey();
    let pub_sub = publisher
        .network
        .subscribe(topic, vec![])
        .await
        .expect("anchor subscribe topic");

    let peer_c = harness
        .spawn_peer(
            1,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::FAST_GOSSIP_TICK),
            vec![publisher_pk],
            vec![author],
        )
        .await;

    // Let peer C's auto-subscription dial the anchor and form a route.
    tokio::time::sleep(Duration::from_millis(300)).await;
    (publisher, Box::new(pub_sub), peer_c, publisher_pk)
}

// Closes `distribution.md` §10.7 over real iroh-gossip via the direct-stream
// pull transport (spec §14): the late joiner B hears C's above-its-head
// `RevocationHeads` summary (established→joiner eager-push, the direction
// proven reliable in §13) and DIALS C via `request_distribution` to pull the
// missing signed envelopes — point-to-point QUIC, which bypasses the Plumtree
// joiner→established asymmetry that defeated the original gossip-push design.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn late_joiner_catches_up_missed_revocation_over_iroh() {
    let mut harness = IrohHarness::new([0x51; 32]);
    let (sk, author) = author_key();
    let topic = Topic::from_bytes(derive_revocation_topic(author));
    let (publisher, _pub_sub, peer_c, _anchor_pk) =
        anchor_and_ahead(&mut harness, author, topic).await;

    // Seed C while the late joiner is OFFLINE.
    let revoked = BlobHash::from_bytes([0xAA; 32]);
    publisher
        .network
        .publish(
            topic,
            GossipMessage::Revocation(sign_revocation(&sk, revoked, "compromised", 1)),
        )
        .await
        .expect("publish seed revocation");
    let c_events = poll_events(|| peer_c.revocation_events()).await;
    assert_eq!(
        c_events.len(),
        1,
        "ahead peer C applied the seed revocation"
    );

    // Late joiner B comes online — it missed the gossip and must catch up
    // by hearing C's summary then pulling from C. Bootstrap to C directly
    // (C is the advertiser whose summary B hears and the peer B dials to
    // serve the pull), so B's distribution-topic subscription meshes with C
    // without relying on the anchor to relay.
    let c_pk = harness.peer_pubkey(0);
    let peer_b = harness
        .spawn_peer(
            2,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::FAST_GOSSIP_TICK),
            vec![c_pk],
            vec![author],
        )
        .await;
    // Let B's distribution-topic subscription mesh with C before relying on
    // the periodic head re-broadcast to drive the catch-up.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let b_events = poll_events(|| peer_b.revocation_events()).await;
    assert_eq!(
        b_events.len(),
        1,
        "late joiner caught up the missed revocation over iroh-gossip, got {b_events:?}"
    );
    assert_eq!(b_events[0].author, author);
    assert_eq!(b_events[0].revoked_bundle_hash, revoked);
    assert_eq!(b_events[0].revocation_seq, 1);

    peer_b.shutdown().await;
    peer_c.shutdown().await;
}

// Publication twin of the revocation test above: latest-wins, so B pulls the
// single newest envelope from C's `publication_latest` archive. Same pull
// mechanism (hear C's above-its-head summary → dial C → apply).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn late_joiner_catches_up_missed_publication_over_iroh() {
    let mut harness = IrohHarness::new([0x52; 32]);
    let (sk, author) = author_key();
    let topic = Topic::from_bytes(derive_publication_topic(author));
    let (publisher, _pub_sub, peer_c, _anchor_pk) =
        anchor_and_ahead(&mut harness, author, topic).await;

    let manifest = BlobHash::from_bytes([0xCC; 32]);
    publisher
        .network
        .publish(
            topic,
            GossipMessage::Publication(sign_publication(&sk, manifest, "1.2.3", 1)),
        )
        .await
        .expect("publish seed publication");
    let c_events = poll_events(|| peer_c.publication_events()).await;
    assert_eq!(
        c_events.len(),
        1,
        "ahead peer C applied the seed publication"
    );

    let c_pk = harness.peer_pubkey(0);
    let peer_b = harness
        .spawn_peer(
            2,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::FAST_GOSSIP_TICK),
            vec![c_pk],
            vec![author],
        )
        .await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    let b_events = poll_events(|| peer_b.publication_events()).await;
    assert_eq!(
        b_events.len(),
        1,
        "late joiner caught up the latest publication over iroh-gossip, got {b_events:?}"
    );
    assert_eq!(b_events[0].author, author);
    assert_eq!(b_events[0].manifest_hash, manifest);
    assert_eq!(b_events[0].version, "1.2.3");
    assert_eq!(b_events[0].publication_seq, 1);

    peer_b.shutdown().await;
    peer_c.shutdown().await;
}
