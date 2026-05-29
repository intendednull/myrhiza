//! B-12 (corrected transport, T3) acceptance tests for the
//! `IrohNetwork` distribution-backfill direct-stream surface.
//!
//! Per docs/specs/2026-05-29-b-12-stale-network-backfill-design.md §14.2.
//! Mirrors `direct_streams_iroh.rs` (the event-DAG `request_heads`
//! transport tests) — the distribution protocol is a NEW, parallel
//! direct-stream protocol on `DISTRIBUTION_REQUEST_ALPN`, disjoint from
//! `request_heads`.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use iroh::address_lookup::MemoryLookup;
use myrhiza_distribution::{
    DistributionBackfillRequest, DistributionEnvelope, DistributionLogKind, RevocationEvent,
};
use myrhiza_network::distribution_request::{DistributionHandler, DistributionResponder};
use myrhiza_network::{DISTRIBUTION_REQUEST_ALPN, IrohNetwork, NetError, Network};
use myrhiza_types::{AuthorPubkey, BlobHash, PeerPubkey};
use std::sync::{Arc, Mutex};

fn sample_revocation(seq: u64) -> RevocationEvent {
    RevocationEvent {
        revoked_bundle_hash: BlobHash::from_bytes([0xAB; 32]),
        reason: "compromised".into(),
        revoked_at: 0,
        revocation_seq: seq,
        signature: [0u8; 64],
    }
}

fn sample_request() -> DistributionBackfillRequest {
    DistributionBackfillRequest {
        author: AuthorPubkey::from_bytes([0x42; 32]),
        kind: DistributionLogKind::Revocation,
        from_seq: 0,
    }
}

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

/// Spawn an iroh peer with both iroh-gossip ALPN AND the
/// distribution-request ALPN registered on its Router. The
/// distribution-request ALPN's handler is
/// `network.distribution_protocol_handler()` (which shares the installed
/// `DistributionHandler` slot with the `IrohNetwork` via
/// `Arc<Mutex<Option<_>>>`).
async fn spawn_iroh_peer(
    lookup: &MemoryLookup,
    register_distribution_alpn: bool,
) -> (
    iroh::Endpoint,
    iroh_gossip::Gossip,
    iroh::protocol::Router,
    IrohNetwork,
) {
    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .address_lookup(lookup.clone())
        .bind()
        .await
        .expect("iroh endpoint bind");
    lookup.add_endpoint_info(endpoint.addr());
    let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
    let network = IrohNetwork::new(endpoint.clone(), gossip.clone());
    let mut builder =
        iroh::protocol::Router::builder(endpoint.clone()).accept(iroh_gossip::ALPN, gossip.clone());
    if register_distribution_alpn {
        builder = builder.accept(
            DISTRIBUTION_REQUEST_ALPN,
            network.distribution_protocol_handler(),
        );
    }
    let router = builder.spawn();
    (endpoint, gossip, router, network)
}

// ---- tests ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_request_distribution_delivers_envelopes() {
    let lookup = MemoryLookup::default();
    let (_ep_a, _g_a, _r_a, net_a) = spawn_iroh_peer(&lookup, true).await;
    let (_ep_b, _g_b, _r_b, net_b) = spawn_iroh_peer(&lookup, true).await;

    let envelopes = vec![
        DistributionEnvelope::Revocation(sample_revocation(1)),
        DistributionEnvelope::Revocation(sample_revocation(2)),
        DistributionEnvelope::Revocation(sample_revocation(3)),
    ];
    net_b.install_distribution_handler(Arc::new(FixedEnvelopesHandler {
        envelopes: envelopes.clone(),
    }));

    let mut stream = net_a
        .request_distribution(net_b.peer_pubkey(), sample_request())
        .await
        .expect("request_distribution");

    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.expect("envelope"));
    }
    assert_eq!(received, envelopes, "expected 3 envelopes from B");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_request_distribution_unknown_alpn_fails() {
    let lookup = MemoryLookup::default();
    let (_ep_a, _g_a, _r_a, net_a) = spawn_iroh_peer(&lookup, true).await;
    // Peer B does NOT register the distribution-request ALPN.
    let (_ep_b, _g_b, _r_b, net_b) = spawn_iroh_peer(&lookup, false).await;

    let result = net_a
        .request_distribution(net_b.peer_pubkey(), sample_request())
        .await;
    match result {
        Err(NetError::RequestFailed { .. }) => {} // expected
        Ok(_) => panic!("expected RequestFailed (ALPN mismatch), got Ok"),
        Err(e) => panic!("expected RequestFailed (ALPN mismatch), got Err: {e}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_request_distribution_no_handler_installed_clean_eof() {
    let lookup = MemoryLookup::default();
    let (_ep_a, _g_a, _r_a, net_a) = spawn_iroh_peer(&lookup, true).await;
    let (_ep_b, _g_b, _r_b, net_b) = spawn_iroh_peer(&lookup, true).await;
    // Peer B registers the ALPN but does NOT install a handler.

    let mut stream = net_a
        .request_distribution(net_b.peer_pubkey(), sample_request())
        .await
        .expect("request_distribution");
    assert!(stream.next().await.is_none(), "expected clean EOF");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_request_distribution_handler_sees_requester_pubkey() {
    struct CapturingHandler {
        seen: Arc<Mutex<Option<PeerPubkey>>>,
    }
    #[async_trait::async_trait]
    impl DistributionHandler for CapturingHandler {
        async fn handle(
            &self,
            requester: PeerPubkey,
            _r: DistributionBackfillRequest,
            _resp: DistributionResponder,
        ) {
            *self.seen.lock().unwrap() = Some(requester);
        }
    }

    let lookup = MemoryLookup::default();
    let (_ep_a, _g_a, _r_a, net_a) = spawn_iroh_peer(&lookup, true).await;
    let (_ep_b, _g_b, _r_b, net_b) = spawn_iroh_peer(&lookup, true).await;
    let captured = Arc::new(Mutex::new(None));
    net_b.install_distribution_handler(Arc::new(CapturingHandler {
        seen: captured.clone(),
    }));

    let mut stream = net_a
        .request_distribution(net_b.peer_pubkey(), sample_request())
        .await
        .expect("request_distribution");
    // Drain to EOF; the drop-order discipline in
    // DistributionRequestProtocol::accept guarantees the handler task's
    // `seen` write commits before FIN, which is what unblocks EOF here.
    while stream.next().await.is_some() {}
    assert_eq!(*captured.lock().unwrap(), Some(net_a.peer_pubkey()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_install_distribution_handler_last_call_wins() {
    let lookup = MemoryLookup::default();
    let (_ep_a, _g_a, _r_a, net_a) = spawn_iroh_peer(&lookup, true).await;
    let (_ep_b, _g_b, _r_b, net_b) = spawn_iroh_peer(&lookup, true).await;

    net_b.install_distribution_handler(Arc::new(FixedEnvelopesHandler {
        envelopes: vec![DistributionEnvelope::Revocation(sample_revocation(1))],
    }));
    net_b.install_distribution_handler(Arc::new(FixedEnvelopesHandler {
        envelopes: vec![DistributionEnvelope::Revocation(sample_revocation(99))],
    }));

    let mut stream = net_a
        .request_distribution(net_b.peer_pubkey(), sample_request())
        .await
        .expect("request_distribution");
    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.expect("envelope"));
    }
    assert_eq!(received.len(), 1);
    match &received[0] {
        DistributionEnvelope::Revocation(ev) => assert_eq!(ev.revocation_seq, 99),
        other @ DistributionEnvelope::Publication(_) => {
            panic!("expected a revocation envelope, got {other:?}")
        }
    }
}

/// The distribution protocol and the event-DAG `request_heads` protocol
/// are disjoint (spec §14.2): a peer that registers ONLY the
/// distribution ALPN does not satisfy a `request_heads` dial, and a peer
/// that registers ONLY the heads ALPN does not satisfy a
/// `request_distribution` dial. Here B registers only the distribution
/// ALPN, so a `request_heads` to it fails on ALPN mismatch.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_distribution_alpn_does_not_serve_heads() {
    use myrhiza_types::{DirectHeadsRequest, Topic};

    let lookup = MemoryLookup::default();
    let (_ep_a, _g_a, _r_a, net_a) = spawn_iroh_peer(&lookup, true).await;
    // B registers ONLY the distribution ALPN (not the heads ALPN).
    let (_ep_b, _g_b, _r_b, net_b) = spawn_iroh_peer(&lookup, true).await;

    let result = net_a
        .request_heads(
            net_b.peer_pubkey(),
            DirectHeadsRequest {
                topic: Topic::from_bytes([0xAB; 32]),
                requests: vec![],
            },
        )
        .await;
    match result {
        Err(NetError::RequestFailed { .. }) => {} // expected ALPN mismatch
        Ok(_) => panic!("expected RequestFailed (heads ALPN not registered), got Ok"),
        Err(e) => panic!("expected RequestFailed, got Err: {e}"),
    }
}
