//! B-12 acceptance tests for the `MemNetwork` distribution-backfill
//! direct-stream surface.
//!
//! Per docs/specs/2026-05-29-b-12-stale-network-backfill-design.md
//! §14.2/§14.3. Mirrors `direct_streams_memory.rs` (the event-DAG
//! `request_heads` tests) — the distribution protocol is a NEW, parallel
//! direct-stream protocol disjoint from `request_heads`.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use myrhiza_distribution::{
    DistributionBackfillRequest, DistributionEnvelope, DistributionLogKind, RevocationEvent,
};
use myrhiza_network::distribution_request::{DistributionHandler, DistributionResponder};
use myrhiza_network::{MemBus, MemNetwork, NetError, Network};
use myrhiza_types::{AuthorPubkey, BlobHash, PeerPubkey};
use std::sync::{Arc, Mutex};

const PEER_A: [u8; 32] = [0xA1; 32];
const PEER_B: [u8; 32] = [0xB2; 32];

fn pk(bytes: [u8; 32]) -> PeerPubkey {
    PeerPubkey::from_bytes(bytes)
}

fn sample_revocation(seq: u64) -> RevocationEvent {
    RevocationEvent {
        revoked_bundle_hash: BlobHash::from_bytes([0xAB; 32]),
        reason: "compromised".into(),
        revoked_at: 0,
        revocation_seq: seq,
        signature: [0u8; 64],
    }
}

/// Handler that emits a fixed vector of envelopes on every call.
struct FixedEnvelopesHandler {
    envelopes: Vec<DistributionEnvelope>,
}

#[async_trait::async_trait]
impl DistributionHandler for FixedEnvelopesHandler {
    async fn handle(
        &self,
        _requester: PeerPubkey,
        _request: DistributionBackfillRequest,
        responder: DistributionResponder,
    ) {
        for env in &self.envelopes {
            if responder.send(env.clone()).await.is_err() {
                return;
            }
        }
    }
}

fn sample_request() -> DistributionBackfillRequest {
    DistributionBackfillRequest {
        author: AuthorPubkey::from_bytes([0x42; 32]),
        kind: DistributionLogKind::Revocation,
        from_seq: 0,
    }
}

// ---- tests ------------------------------------------------------------------

#[tokio::test]
async fn mem_request_distribution_delivers_envelopes() {
    let bus = MemBus::new(8);
    let net_a = MemNetwork::new(bus.clone(), pk(PEER_A));
    let net_b = MemNetwork::new(bus.clone(), pk(PEER_B));

    let envelopes = vec![
        DistributionEnvelope::Revocation(sample_revocation(1)),
        DistributionEnvelope::Revocation(sample_revocation(2)),
        DistributionEnvelope::Revocation(sample_revocation(3)),
    ];
    net_b.install_distribution_handler(Arc::new(FixedEnvelopesHandler {
        envelopes: envelopes.clone(),
    }));

    let mut stream = net_a
        .request_distribution(pk(PEER_B), sample_request())
        .await
        .expect("request_distribution");

    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.expect("envelope"));
    }
    assert_eq!(received, envelopes);
}

#[tokio::test]
async fn mem_request_distribution_unknown_peer_fails() {
    let bus = MemBus::new(8);
    let net_a = MemNetwork::new(bus.clone(), pk(PEER_A));
    // peer B never constructed -> no distribution handler in registry.

    let result = net_a
        .request_distribution(pk(PEER_B), sample_request())
        .await;
    match result {
        Err(NetError::RequestFailed { peer, reason: _ }) => {
            assert_eq!(peer, pk(PEER_B));
        }
        Ok(_) => panic!("expected RequestFailed, got Ok"),
        Err(e) => panic!("expected RequestFailed, got Err: {e}"),
    }
}

#[tokio::test]
async fn mem_request_distribution_handler_drops_responder_signals_eof() {
    let bus = MemBus::new(8);
    let net_a = MemNetwork::new(bus.clone(), pk(PEER_A));
    let net_b = MemNetwork::new(bus.clone(), pk(PEER_B));

    net_b.install_distribution_handler(Arc::new(FixedEnvelopesHandler { envelopes: vec![] }));

    let mut stream = net_a
        .request_distribution(pk(PEER_B), sample_request())
        .await
        .expect("request_distribution");
    assert!(stream.next().await.is_none(), "expected clean EOF");
}

/// Handler that records the requester pubkey it was invoked with.
struct CapturingHandler {
    seen_requester: Arc<Mutex<Option<PeerPubkey>>>,
}

#[async_trait::async_trait]
impl DistributionHandler for CapturingHandler {
    async fn handle(
        &self,
        requester: PeerPubkey,
        _request: DistributionBackfillRequest,
        _responder: DistributionResponder,
    ) {
        *self
            .seen_requester
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(requester);
    }
}

#[tokio::test]
async fn mem_request_distribution_handler_sees_requester_pubkey() {
    let bus = MemBus::new(8);
    let net_a = MemNetwork::new(bus.clone(), pk(PEER_A));
    let net_b = MemNetwork::new(bus.clone(), pk(PEER_B));

    let seen = Arc::new(Mutex::new(None));
    net_b.install_distribution_handler(Arc::new(CapturingHandler {
        seen_requester: seen.clone(),
    }));

    let mut stream = net_a
        .request_distribution(pk(PEER_B), sample_request())
        .await
        .expect("request_distribution");
    // Drain to EOF so the handler task has run.
    while stream.next().await.is_some() {}

    assert_eq!(
        *seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
        Some(pk(PEER_A)),
    );
}

#[tokio::test]
async fn mem_install_distribution_handler_last_call_wins() {
    let bus = MemBus::new(8);
    let net_a = MemNetwork::new(bus.clone(), pk(PEER_A));
    let net_b = MemNetwork::new(bus.clone(), pk(PEER_B));

    net_b.install_distribution_handler(Arc::new(FixedEnvelopesHandler {
        envelopes: vec![DistributionEnvelope::Revocation(sample_revocation(1))],
    }));
    net_b.install_distribution_handler(Arc::new(FixedEnvelopesHandler {
        envelopes: vec![
            DistributionEnvelope::Revocation(sample_revocation(7)),
            DistributionEnvelope::Revocation(sample_revocation(8)),
        ],
    }));

    let mut stream = net_a
        .request_distribution(pk(PEER_B), sample_request())
        .await
        .expect("request_distribution");

    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.expect("envelope"));
    }
    // Second install wins: two envelopes (seq 7, 8).
    assert_eq!(received.len(), 2);
    match (&received[0], &received[1]) {
        (DistributionEnvelope::Revocation(a), DistributionEnvelope::Revocation(b)) => {
            assert_eq!(a.revocation_seq, 7);
            assert_eq!(b.revocation_seq, 8);
        }
        other => panic!("expected two revocation envelopes, got {other:?}"),
    }
}

/// The distribution protocol and the event-DAG `request_heads` protocol
/// are disjoint (spec §14.2): installing a distribution handler does NOT
/// satisfy a `request_heads` call, and vice versa.
#[tokio::test]
async fn distribution_and_heads_handlers_are_disjoint() {
    use myrhiza_network::request::{HeadsResponder, RequestHandler};
    use myrhiza_types::DirectHeadsRequest;

    struct NoopHeadsHandler;
    #[async_trait::async_trait]
    impl RequestHandler for NoopHeadsHandler {
        async fn handle(
            &self,
            _requester: PeerPubkey,
            _request: DirectHeadsRequest,
            _responder: HeadsResponder,
        ) {
        }
    }

    let bus = MemBus::new(8);
    let net_a = MemNetwork::new(bus.clone(), pk(PEER_A));
    let net_b = MemNetwork::new(bus.clone(), pk(PEER_B));

    // Install ONLY a heads handler on B.
    net_b.install_request_handler(Arc::new(NoopHeadsHandler));

    // A distribution request to B must still fail (no distribution handler).
    let result = net_a
        .request_distribution(pk(PEER_B), sample_request())
        .await;
    assert!(
        matches!(result, Err(NetError::RequestFailed { .. })),
        "distribution request must not be served by a heads-only handler",
    );
}
