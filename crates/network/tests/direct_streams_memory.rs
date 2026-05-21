//! B-4.4 acceptance tests for the `MemNetwork` direct-stream surface.
//!
//! Per docs/specs/2026-05-21-plan-b-4-4-direct-streams-design.md §4.1.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use myrhiza_network::request::{HeadsResponder, RequestHandler};
use myrhiza_network::{MemBus, MemNetwork, NetError, Network};
use myrhiza_types::{
    AuthorPubkey, DirectHeadsRequest, Event, EventHash, EventRequest, Hlc, PeerPubkey, Topic,
};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

const PEER_A: [u8; 32] = [0xA1; 32];
const PEER_B: [u8; 32] = [0xB2; 32];

fn pk(bytes: [u8; 32]) -> PeerPubkey {
    PeerPubkey::from_bytes(bytes)
}

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

/// Handler that emits a fixed vector of events on every call.
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

// ---- tests ------------------------------------------------------------------

#[tokio::test]
async fn mem_request_heads_delivers_events() {
    let bus = MemBus::new(8);
    let net_a = MemNetwork::new(bus.clone(), pk(PEER_A));
    let net_b = MemNetwork::new(bus.clone(), pk(PEER_B));

    let events = vec![sample_event(1), sample_event(2), sample_event(3)];
    net_b.install_request_handler(Arc::new(FixedEventsHandler {
        events: events.clone(),
    }));

    let mut stream = net_a
        .request_heads(pk(PEER_B), sample_request())
        .await
        .expect("request_heads");

    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.expect("event"));
    }
    assert_eq!(received.len(), 3);
    for (i, e) in received.iter().enumerate() {
        assert_eq!(e.seq, (i as u64) + 1);
    }
}

#[tokio::test]
async fn mem_request_heads_unknown_peer_fails() {
    let bus = MemBus::new(8);
    let net_a = MemNetwork::new(bus.clone(), pk(PEER_A));
    // peer B never constructed -> no handler in registry.

    let result = net_a.request_heads(pk(PEER_B), sample_request()).await;
    match result {
        Err(NetError::RequestFailed { peer, reason: _ }) => {
            assert_eq!(peer, pk(PEER_B));
        }
        Ok(_) => panic!("expected RequestFailed, got Ok"),
        Err(e) => panic!("expected RequestFailed, got Err: {e}"),
    }
}

#[tokio::test]
async fn mem_request_heads_handler_drops_responder_signals_eof() {
    let bus = MemBus::new(8);
    let net_a = MemNetwork::new(bus.clone(), pk(PEER_A));
    let net_b = MemNetwork::new(bus.clone(), pk(PEER_B));

    net_b.install_request_handler(Arc::new(FixedEventsHandler { events: vec![] }));

    let mut stream = net_a
        .request_heads(pk(PEER_B), sample_request())
        .await
        .expect("request_heads");
    assert!(stream.next().await.is_none(), "expected clean EOF");
}

#[tokio::test]
async fn mem_request_heads_multiple_concurrent() {
    let bus = MemBus::new(8);
    let net_a = MemNetwork::new(bus.clone(), pk(PEER_A));
    let net_b = MemNetwork::new(bus.clone(), pk(PEER_B));

    let events = vec![sample_event(1), sample_event(2)];
    net_b.install_request_handler(Arc::new(FixedEventsHandler {
        events: events.clone(),
    }));

    let (mut s1, mut s2) = tokio::join!(
        async {
            net_a
                .request_heads(pk(PEER_B), sample_request())
                .await
                .expect("request_heads 1")
        },
        async {
            net_a
                .request_heads(pk(PEER_B), sample_request())
                .await
                .expect("request_heads 2")
        }
    );

    // Drain sequentially (cleaner than spawning two tasks for one assert)
    let r1: Vec<_> = {
        let mut out = Vec::new();
        while let Some(it) = s1.next().await {
            out.push(it.expect("evt"));
        }
        out
    };
    let r2: Vec<_> = {
        let mut out = Vec::new();
        while let Some(it) = s2.next().await {
            out.push(it.expect("evt"));
        }
        out
    };
    assert_eq!(r1.len(), 2);
    assert_eq!(r2.len(), 2);
}

#[tokio::test]
async fn mem_request_handler_sees_requester_pubkey() {
    struct CapturingHandler {
        seen_requester: Arc<Mutex<Option<PeerPubkey>>>,
    }
    #[async_trait::async_trait]
    impl RequestHandler for CapturingHandler {
        async fn handle(
            &self,
            requester: PeerPubkey,
            _request: DirectHeadsRequest,
            _responder: HeadsResponder,
        ) {
            *self.seen_requester.lock().unwrap() = Some(requester);
        }
    }

    let bus = MemBus::new(8);
    let net_a = MemNetwork::new(bus.clone(), pk(PEER_A));
    let net_b = MemNetwork::new(bus.clone(), pk(PEER_B));

    let captured = Arc::new(Mutex::new(None));
    net_b.install_request_handler(Arc::new(CapturingHandler {
        seen_requester: captured.clone(),
    }));

    let mut stream = net_a
        .request_heads(pk(PEER_B), sample_request())
        .await
        .expect("request_heads");
    // Drain the stream — `stream.next().await` returns `None` only
    // after the spawned handler task's `HeadsResponder` is dropped,
    // which happens when `CapturingHandler::handle` returns. The
    // function body writes `seen_requester` BEFORE returning, so the
    // mutex value is guaranteed observable by the time EOF fires.
    // No polling needed.
    while stream.next().await.is_some() {}
    assert_eq!(*captured.lock().unwrap(), Some(pk(PEER_A)));
}

#[tokio::test]
async fn mem_install_request_handler_last_call_wins() {
    let bus = MemBus::new(8);
    let net_a = MemNetwork::new(bus.clone(), pk(PEER_A));
    let net_b = MemNetwork::new(bus.clone(), pk(PEER_B));

    net_b.install_request_handler(Arc::new(FixedEventsHandler {
        events: vec![sample_event(1)],
    }));
    // Install a second handler — last call wins.
    net_b.install_request_handler(Arc::new(FixedEventsHandler {
        events: vec![sample_event(99)],
    }));

    let mut stream = net_a
        .request_heads(pk(PEER_B), sample_request())
        .await
        .expect("request_heads");
    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.expect("event"));
    }
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].seq, 99, "last-installed handler should run");
}
