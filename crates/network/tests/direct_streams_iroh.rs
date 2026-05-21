//! B-4.4 acceptance tests for the `IrohNetwork` direct-stream surface.
//!
//! Per docs/specs/2026-05-21-plan-b-4-4-direct-streams-design.md §4.2.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use iroh::address_lookup::MemoryLookup;
use myrhiza_network::request::{HEADS_REQUEST_ALPN, HeadsResponder, RequestHandler};
use myrhiza_network::{IrohNetwork, NetError, Network};
use myrhiza_types::{
    AuthorPubkey, DirectHeadsRequest, Event, EventHash, EventRequest, Hlc, PeerPubkey, Topic,
};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

fn sample_event(seq: u64) -> Event {
    Event {
        author: AuthorPubkey::from_bytes([0x42; 32]),
        seq,
        prev: EventHash::ZERO,
        deps: BTreeSet::new(),
        hlc: Hlc {
            wall_ms: 0,
            logical: u32::try_from(seq).expect("seq fits in u32 for test events"),
        },
        payload: vec![],
        signature: [0; 64],
    }
}

fn sample_request() -> DirectHeadsRequest {
    DirectHeadsRequest {
        topic: Topic::from_bytes([0xAB; 32]),
        requests: vec![EventRequest {
            author: AuthorPubkey::from_bytes([0x42; 32]),
            from_seq: 1,
            to_seq: 3,
        }],
    }
}

struct FixedEventsHandler {
    events: Vec<Event>,
}
#[async_trait::async_trait]
impl RequestHandler for FixedEventsHandler {
    async fn handle(
        &self,
        _requester: PeerPubkey,
        _request: DirectHeadsRequest,
        responder: HeadsResponder,
    ) {
        for e in &self.events {
            if responder.send(e.clone()).await.is_err() {
                return;
            }
        }
    }
}

/// Spawn an iroh peer with both iroh-gossip ALPN AND the heads-request
/// ALPN registered on its Router. The heads-request ALPN's handler is
/// `network.protocol_handler()` (which shares state with the
/// `IrohNetwork` via Arc<Mutex<Option<_>>>).
async fn spawn_iroh_peer(
    lookup: &MemoryLookup,
    register_heads_alpn: bool,
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
    if register_heads_alpn {
        builder = builder.accept(HEADS_REQUEST_ALPN, network.protocol_handler());
    }
    let router = builder.spawn();
    (endpoint, gossip, router, network)
}

// ---- tests ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_request_heads_delivers_events() {
    let lookup = MemoryLookup::default();
    let (_ep_a, _g_a, _r_a, net_a) = spawn_iroh_peer(&lookup, true).await;
    let (_ep_b, _g_b, _r_b, net_b) = spawn_iroh_peer(&lookup, true).await;

    let events = vec![sample_event(1), sample_event(2), sample_event(3)];
    net_b.install_request_handler(Arc::new(FixedEventsHandler {
        events: events.clone(),
    }));

    let mut stream = net_a
        .request_heads(net_b.peer_pubkey(), sample_request())
        .await
        .expect("request_heads");

    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.expect("event"));
    }
    assert_eq!(received.len(), 3, "expected 3 events from B");
    for (i, e) in received.iter().enumerate() {
        assert_eq!(e.seq, (i as u64) + 1);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_request_heads_unknown_alpn_fails() {
    let lookup = MemoryLookup::default();
    let (_ep_a, _g_a, _r_a, net_a) = spawn_iroh_peer(&lookup, true).await;
    // Peer B does NOT register the heads-request ALPN.
    let (_ep_b, _g_b, _r_b, net_b) = spawn_iroh_peer(&lookup, false).await;

    let result = net_a
        .request_heads(net_b.peer_pubkey(), sample_request())
        .await;
    match result {
        Err(NetError::RequestFailed { .. }) => {} // expected
        Ok(_) => panic!("expected RequestFailed (ALPN mismatch), got Ok"),
        Err(e) => panic!("expected RequestFailed (ALPN mismatch), got Err: {e}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_request_heads_no_handler_installed_clean_eof() {
    let lookup = MemoryLookup::default();
    let (_ep_a, _g_a, _r_a, net_a) = spawn_iroh_peer(&lookup, true).await;
    let (_ep_b, _g_b, _r_b, net_b) = spawn_iroh_peer(&lookup, true).await;
    // Peer B registers the ALPN but does NOT install a handler.

    let mut stream = net_a
        .request_heads(net_b.peer_pubkey(), sample_request())
        .await
        .expect("request_heads");
    assert!(stream.next().await.is_none(), "expected clean EOF");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_request_heads_handler_sees_requester_pubkey() {
    struct CapturingHandler {
        seen: Arc<Mutex<Option<PeerPubkey>>>,
    }
    #[async_trait::async_trait]
    impl RequestHandler for CapturingHandler {
        async fn handle(
            &self,
            requester: PeerPubkey,
            _r: DirectHeadsRequest,
            _resp: HeadsResponder,
        ) {
            *self.seen.lock().unwrap() = Some(requester);
        }
    }

    let lookup = MemoryLookup::default();
    let (_ep_a, _g_a, _r_a, net_a) = spawn_iroh_peer(&lookup, true).await;
    let (_ep_b, _g_b, _r_b, net_b) = spawn_iroh_peer(&lookup, true).await;
    let captured = Arc::new(Mutex::new(None));
    net_b.install_request_handler(Arc::new(CapturingHandler {
        seen: captured.clone(),
    }));

    let mut stream = net_a
        .request_heads(net_b.peer_pubkey(), sample_request())
        .await
        .expect("request_heads");
    while stream.next().await.is_some() {}

    // Give handler task time to record the pubkey.
    for _ in 0..20 {
        if captured.lock().unwrap().is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(*captured.lock().unwrap(), Some(net_a.peer_pubkey()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn iroh_install_request_handler_last_call_wins() {
    let lookup = MemoryLookup::default();
    let (_ep_a, _g_a, _r_a, net_a) = spawn_iroh_peer(&lookup, true).await;
    let (_ep_b, _g_b, _r_b, net_b) = spawn_iroh_peer(&lookup, true).await;

    net_b.install_request_handler(Arc::new(FixedEventsHandler {
        events: vec![sample_event(1)],
    }));
    net_b.install_request_handler(Arc::new(FixedEventsHandler {
        events: vec![sample_event(99)],
    }));

    let mut stream = net_a
        .request_heads(net_b.peer_pubkey(), sample_request())
        .await
        .expect("request_heads");
    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.expect("event"));
    }
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].seq, 99);
}
