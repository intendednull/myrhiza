//! Kernel-tier stale-network backfill acceptance tests over `MemNetwork`
//! (fast, no iroh).
//!
//! Proves the B-12 catch-up end-to-end at the in-process tier using the
//! corrected **direct-stream pull** transport (spec §14, which supersedes
//! the original gossip-push design — see §13 for why push could not catch
//! up a late joiner over real iroh-gossip). A peer that was offline while
//! an author broadcast a revocation/publication later joins, hears an
//! ahead-peer (advertiser) summary whose head is *above* its own, and
//! **dials the advertiser** (`request_distribution`) to pull the missing
//! signed envelopes from the advertiser's archive — revocation as a
//! contiguous range, publication as the single latest-wins envelope. The
//! pulled envelopes re-enter the existing `handle_revocation` /
//! `handle_publication` apply path. Also covers the mismatched-author
//! guard, the per-advertiser dial-limit (replacing the deleted
//! amplification rate-limit), and the 24h staleness surface.
//!
//! Per B-12 spec §14.5 (kernel tier) / plan T4/T5/T7. The real-iroh
//! analogue lives in `iroh_stale_backfill.rs` (closes `distribution.md`
//! §10.7).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime};

use ed25519_dalek::{Signer, SigningKey};
use myrhiza_distribution::topic::{derive_publication_topic, derive_revocation_topic};
use myrhiza_distribution::{
    DistributionBackfillRequest, PublicationEvent, RevocationEvent, RevocationHeads,
};
use myrhiza_network::distribution_request::{DistributionHandler, DistributionResponder};
use myrhiza_network::{GossipMessage, MemNetwork, Network};
use myrhiza_test_utils::InProcessHarness;
use myrhiza_types::{AuthorPubkey, BlobHash, PeerPubkey, Topic};

mod helpers;

/// The author key under test. Seed 7 matches the B-11 fixture author so
/// the kernel cross-check against the topic-owner is consistent.
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

/// Poll `f` until `done` holds or `timeout` elapses; returns the final
/// observation (condition-based waiting, not a fixed sleep).
async fn poll_until<T>(
    timeout: Duration,
    mut f: impl FnMut() -> Vec<T>,
    done: impl Fn(&[T]) -> bool,
) -> Vec<T> {
    let deadline = Instant::now() + timeout;
    loop {
        let v = f();
        if done(&v) || Instant::now() >= deadline {
            return v;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// A raw publisher `MemNetwork` on the harness bus (distinct peer key).
fn publisher(harness: &InProcessHarness, tag: u8) -> MemNetwork {
    MemNetwork::new(harness.bus.clone(), PeerPubkey::from_bytes([tag; 32]))
}

/// HEADLINE (spec §14.1): a peer that missed a *range* of revocations
/// while offline catches up the full contiguous range after it joins,
/// hears the ahead-peer's advertised head (above its own), and **pulls**
/// the gap by dialing the advertiser. Proves revocation's
/// accumulate-the-whole-range backfill, not just head-only.
#[tokio::test]
async fn late_joiner_catches_up_revocation_range() {
    let harness = InProcessHarness::new(256, [0x41; 32]);
    let (sk, author) = author_key();
    let rev_topic = Topic::from_bytes(derive_revocation_topic(author));

    // Ahead peer C: subscribed, applies + archives revocations seq 1..=3.
    let c = harness
        .spawn_peer(
            1,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
            vec![author],
        )
        .await;

    let net = publisher(&harness, 0xFE);
    let bundles = [
        BlobHash::from_bytes([0xA1; 32]),
        BlobHash::from_bytes([0xA2; 32]),
        BlobHash::from_bytes([0xA3; 32]),
    ];
    for (i, b) in bundles.iter().enumerate() {
        let seq = (i + 1) as u64;
        net.publish(
            rev_topic,
            GossipMessage::Revocation(sign_revocation(&sk, *b, "x", seq)),
        )
        .await
        .expect("publish revocation");
    }
    let c_events = poll_until(
        Duration::from_secs(2),
        || c.revocation_events(),
        |v| v.len() >= 3,
    )
    .await;
    assert_eq!(
        c_events.len(),
        3,
        "ahead peer applied all three revocations"
    );

    // Late joiner B: empty log (local head 0). It hears C's advertised
    // `RevocationHeads{seq=3}` (remote 3 > local 0 ⇒ B is behind), dials C
    // via `request_distribution`, and pulls the contiguous range 1..=3 from
    // C's archive.
    let b = harness
        .spawn_peer(
            2,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
            vec![author],
        )
        .await;

    let b_events = poll_until(
        Duration::from_secs(5),
        || b.revocation_events(),
        |v| v.len() >= 3,
    )
    .await;
    assert_eq!(
        b_events.len(),
        3,
        "late joiner caught up all three missed revocations, got {b_events:?}"
    );
    let mut seqs: Vec<u64> = b_events.iter().map(|e| e.revocation_seq).collect();
    seqs.sort_unstable();
    assert_eq!(seqs, vec![1, 2, 3], "contiguous range, no gaps");

    b.shutdown().await;
    c.shutdown().await;
}

/// Publication is latest-wins (spec §3.3): a late joiner catches up the
/// *single newest* announcement by pulling one envelope from the
/// advertiser, not the whole history.
#[tokio::test]
async fn late_joiner_catches_up_latest_publication() {
    let harness = InProcessHarness::new(256, [0x42; 32]);
    let (sk, author) = author_key();
    let pub_topic = Topic::from_bytes(derive_publication_topic(author));

    let c = harness
        .spawn_peer(
            1,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
            vec![author],
        )
        .await;

    let net = publisher(&harness, 0xFE);
    net.publish(
        pub_topic,
        GossipMessage::Publication(sign_publication(
            &sk,
            BlobHash::from_bytes([0xB1; 32]),
            "1.0.0",
            1,
        )),
    )
    .await
    .expect("publish v1");
    net.publish(
        pub_topic,
        GossipMessage::Publication(sign_publication(
            &sk,
            BlobHash::from_bytes([0xB2; 32]),
            "2.0.0",
            2,
        )),
    )
    .await
    .expect("publish v2");
    poll_until(
        Duration::from_secs(2),
        || c.publication_events(),
        |v| v.len() >= 2,
    )
    .await;

    let b = harness
        .spawn_peer(
            2,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
            vec![author],
        )
        .await;

    let b_events = poll_until(
        Duration::from_secs(5),
        || b.publication_events(),
        |v| !v.is_empty(),
    )
    .await;
    assert_eq!(b_events.len(), 1, "latest-wins: a single envelope pulled");
    assert_eq!(b_events[0].publication_seq, 2);
    assert_eq!(
        b_events[0].version, "2.0.0",
        "caught up to the newest version"
    );

    b.shutdown().await;
    c.shutdown().await;
}

/// A summary whose carried `author` disagrees with the topic it arrived
/// on is a misroute/forgery → `DecodeFailed`, never acted on (spec §3.2).
#[tokio::test]
async fn mismatched_author_summary_warns() {
    let harness = InProcessHarness::new(256, [0x43; 32]);
    let (_sk, topic_owner) = author_key();
    // A different author than the topic owner.
    let other = AuthorPubkey::from_bytes(
        SigningKey::from_bytes(&[9u8; 32])
            .verifying_key()
            .to_bytes(),
    );

    let peer = harness
        .spawn_peer(
            1,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
            vec![topic_owner],
        )
        .await;

    publisher(&harness, 0xFE)
        .publish(
            Topic::from_bytes(derive_revocation_topic(topic_owner)),
            GossipMessage::RevocationHeads(RevocationHeads {
                author: other, // mismatched: claims `other` on `topic_owner`'s topic
                advertiser: PeerPubkey::from_bytes([0xFE; 32]), // the publisher, not the peer-under-test
                last_observed_seq: 0,
            }),
        )
        .await
        .expect("publish mismatched heads");

    let warnings = poll_until(
        Duration::from_secs(2),
        || peer.peer_warnings(),
        |v| !v.is_empty(),
    )
    .await;
    assert!(
        warnings.iter().any(|w| matches!(
            w,
            myrhiza_kernel::runtime::PeerWarning::DecodeFailed { peer: None }
        )),
        "mismatched-author summary must surface DecodeFailed, got {warnings:?}"
    );

    peer.shutdown().await;
}

/// Counting `DistributionHandler`: tallies every inbound
/// `request_distribution` dial it receives and serves *nothing* (clean
/// EOF). Modelling a *forged-high* advertiser — it claims a head it cannot
/// actually back, so a behind peer that dials it pulls zero envelopes and
/// stays behind, wanting to dial again. The tally is the observable the
/// dial-limit test asserts on.
struct CountingDistributionHandler {
    dials: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl DistributionHandler for CountingDistributionHandler {
    async fn handle(
        &self,
        _requester: PeerPubkey,
        _request: DistributionBackfillRequest,
        _responder: DistributionResponder,
    ) {
        // Count the dial; drop the responder (serve nothing → clean EOF).
        // The behind peer's local head never advances, so the next forged
        // summary it hears makes it want to dial again — bounded only by
        // the per-advertiser dial-limit.
        self.dials.fetch_add(1, Ordering::SeqCst);
    }
}

/// Dial-limit guard (spec §14.1, replaces the deleted amplification
/// rate-limit): under the corrected pull transport a behind peer that
/// hears an *above-our-head* summary **dials the advertiser** to pull the
/// gap. A forged-high summary costs at most one wasted dial; the
/// per-advertiser dial-limit caps how *often* one advertiser can goad us
/// into dialing within the trailing-24h window, so a flood of forged-high
/// summaries from one advertiser cannot weaponise us into an unbounded
/// dial storm against it. A 64-summary flood from one forged advertiser
/// yields at most `DISTRIBUTION_DIAL_DAILY_CAP` (32) dials.
#[tokio::test]
async fn forged_high_summary_flood_is_dial_limited() {
    // DISTRIBUTION_DIAL_DAILY_CAP is `pub(crate)`; mirror its value here.
    const CAP: usize = 32;
    // Flood more summaries than the cap so the cap is the binding limit.
    const FLOOD: usize = CAP + 16;
    // Forged advertiser key — distinct from the behind peer's own key so
    // the loopback filter does not suppress its summaries.
    const ADVERTISER: [u8; 32] = [0xFE; 32];

    let harness = InProcessHarness::new(1024, [0x44; 32]);
    let (_sk, author) = author_key();
    let rev_topic = Topic::from_bytes(derive_revocation_topic(author));

    // The forged advertiser is a `MemNetwork` presence on the bus with a
    // counting distribution handler installed under its own pubkey: when
    // the behind peer dials `request_distribution(ADVERTISER, …)`, the bus
    // routes the dial here and the tally increments.
    let advertiser_net = MemNetwork::new(harness.bus.clone(), PeerPubkey::from_bytes(ADVERTISER));
    let dials = Arc::new(AtomicUsize::new(0));
    advertiser_net.install_distribution_handler(Arc::new(CountingDistributionHandler {
        dials: dials.clone(),
    }));

    // Behind peer B: empty revocation log (local head 0), installed for
    // `author`. It will hear the forged-high summaries on the revocation
    // topic and dial the advertiser to pull the (non-existent) gap.
    let b = harness
        .spawn_peer(
            1,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
            vec![author],
        )
        .await;

    // The summary publisher (a third bus presence) broadcasts forged-high
    // summaries advertising the forged advertiser. Each `remote=9999 >
    // local=0` makes B want to pull from ADVERTISER.
    let summary_pub = publisher(&harness, 0xFD);
    let forged = GossipMessage::RevocationHeads(RevocationHeads {
        author,
        advertiser: PeerPubkey::from_bytes(ADVERTISER),
        last_observed_seq: 9999,
    });

    // Flood more summaries than the cap. Each dial completes immediately
    // (the counting handler serves nothing → the in-flight `(author,
    // Revocation)` marker clears), so each subsequent summary triggers a
    // fresh admitted dial until the per-advertiser bucket is exhausted at
    // CAP. Pace the flood on the observed dial count (condition-based, not
    // a fixed sleep): wait for each summary to land a dial before sending
    // the next, so the in-flight guard never silently swallows a summary
    // we are counting on. Once the bucket saturates, dials stop advancing
    // and the loop falls through on the per-summary timeout.
    for _ in 0..FLOOD {
        let before = dials.load(Ordering::SeqCst);
        summary_pub
            .publish(rev_topic, forged.clone())
            .await
            .expect("publish forged-high heads");
        // Wait until this summary either lands a dial or the bucket is
        // clearly exhausted (no advance within the window).
        let deadline = Instant::now() + Duration::from_millis(300);
        while dials.load(Ordering::SeqCst) == before && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    // Let any final in-flight dial settle, then assert the cap held.
    let observed = poll_until(
        Duration::from_secs(1),
        || vec![dials.load(Ordering::SeqCst)],
        |v| v[0] >= CAP,
    )
    .await[0];

    assert_eq!(
        observed, CAP,
        "forged-high flood from one advertiser must be dial-limited to exactly \
         DISTRIBUTION_DIAL_DAILY_CAP ({CAP}), got {observed}"
    );

    b.shutdown().await;
}

/// Staleness surface (spec §3.7): an installed author is stale until a
/// distribution message is received, fresh right after, and stale again
/// once the threshold elapses (probed with a future `now`).
#[tokio::test]
async fn staleness_surface_tracks_sync() {
    let harness = InProcessHarness::new(256, [0x45; 32]);
    let (sk, author) = author_key();
    let day = Duration::from_hours(24);

    let peer = harness
        .spawn_peer(
            1,
            None,
            helpers::counter_handle(),
            helpers::fast_cfg(helpers::BACKGROUND_QUIET_TICK),
            vec![author],
        )
        .await;

    // No inbound distribution traffic yet ⇒ never synced ⇒ stale.
    assert_eq!(
        peer.stale_authors(SystemTime::now(), day),
        vec![author],
        "author with no sync must be reported stale"
    );

    // Receive a valid revocation ⇒ sync clock refreshes.
    publisher(&harness, 0xFE)
        .publish(
            Topic::from_bytes(derive_revocation_topic(author)),
            GossipMessage::Revocation(sign_revocation(
                &sk,
                BlobHash::from_bytes([0xD1; 32]),
                "x",
                1,
            )),
        )
        .await
        .expect("publish revocation");
    poll_until(
        Duration::from_secs(2),
        || peer.revocation_events(),
        |v| !v.is_empty(),
    )
    .await;

    // Just synced ⇒ not stale within the 24h window.
    assert!(
        peer.stale_authors(SystemTime::now(), day).is_empty(),
        "freshly-synced author must not be stale"
    );

    // Probe 25h in the future ⇒ stale again.
    let future = SystemTime::now() + Duration::from_hours(25);
    assert_eq!(
        peer.stale_authors(future, day),
        vec![author],
        "author past the staleness threshold must be reported stale"
    );

    peer.shutdown().await;
}
