//! B-4.1 acceptance tests for the iroh-gossip-backed transport.
//!
//! Per docs/specs/2026-05-20-plan-b-4-1-iroh-gossip-design.md §4.
//!
//! All gossip-driven tests use `multi_thread, worker_threads = 2`
//! because iroh-gossip spawns internal tasks (`gossip/src/net.rs:207`);
//! single-threaded current-thread would deadlock waiting for those.
//! Test 4 and 5 don't drive gossip and use the default flavor.
//!
//! ## Address-lookup adaptation
//!
//! The plan's `spawn_iroh_peer()` was adapted to use a shared
//! [`iroh::address_lookup::MemoryLookup`] so that bootstrap by
//! `EndpointId` works without a relay or DNS lookup service.
//! Iroh's `Gossip::subscribe(topic, vec![EndpointId])` calls
//! `endpoint.connect(endpoint_id, alpn)` internally; that dial requires
//! address information for the target peer. With `Minimal` preset (no
//! relay, no DNS), the only way to supply that information
//! intra-process is via `MemoryLookup::add_endpoint_info(peer_addr)`.
//! Each test creates one shared `MemoryLookup`, passes it to every
//! endpoint builder, and registers each endpoint's `addr()` in it
//! before subscribing.
//!
//! This is a documented API adaptation from the plan's template code.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use bytes::Bytes;
use iroh::address_lookup::MemoryLookup;
use myrhiza_network::iroh_transport::iroh_topic_id_from_topic;
use myrhiza_network::{GossipMessage, IrohNetwork, Network, SubError, Subscription};
use myrhiza_types::{AuthorHead, AuthorPubkey, EventHash, HeadsSummary, PeerPubkey, Topic};
use std::time::Duration;
use tokio::time::timeout;

// ---- helpers ----------------------------------------------------------------

/// Spin up a fresh iroh endpoint + gossip + router for a test peer.
///
/// The caller provides a shared `MemoryLookup` — each peer's `addr()`
/// is registered into it after bind so that bootstrap by `EndpointId`
/// resolves to a real socket address. Without this, `endpoint.connect`
/// has no addressing information and the dial silently times out.
///
/// Returns the four handles in dependency order. The Router is the
/// caller-owned dispatcher that registers `iroh_gossip::ALPN` against
/// the gossip protocol — without it, inbound gossip streams won't
/// reach the Gossip handler.
async fn spawn_iroh_peer(
    lookup: &MemoryLookup,
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

    // Register this endpoint's address in the shared lookup so other
    // peers can dial us by EndpointId. `endpoint.addr()` returns the
    // local UDP socket address(es) iroh has bound.
    lookup.add_endpoint_info(endpoint.addr());

    let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
    let router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();
    let network = IrohNetwork::new(endpoint.clone(), gossip.clone());
    (endpoint, gossip, router, network)
}

fn sample_heads_summary() -> HeadsSummary {
    HeadsSummary {
        authors: vec![AuthorHead {
            author: AuthorPubkey::from_bytes([0x42; 32]),
            seq: 1,
            hash: EventHash::ZERO,
        }],
        kernel_fuel_table_version: 1,
        signed_by_peer: PeerPubkey::from_bytes([0; 32]),
        signature: [0; 64],
    }
}

/// Await the next decoded message from a subscription, with a 5-second
/// deadline. Panics on timeout, subscription close, or unexpected error.
///
/// `IrohSubscription::recv()` silently consumes membership events
/// (`NeighborUp` / `NeighborDown`) in its inner loop, so the first
/// value returned here is already a decoded `GossipMessage`.
async fn recv_first_decoded<S: Subscription>(sub: &mut S) -> GossipMessage {
    let recv_fut = sub.recv();
    let result = timeout(Duration::from_secs(5), recv_fut)
        .await
        .expect("timeout waiting for first decoded message");
    match result {
        Ok(Some(m)) => m,
        Ok(None) => panic!("subscription closed before message arrived"),
        Err(e) => panic!("recv returned unexpected error: {e:?}"),
    }
}

// ---- test 1 -----------------------------------------------------------------

/// Covers: networking.md §11.1 — two iroh peers form a swarm and
/// exchange one bincode-encoded `GossipMessage` end-to-end.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_peers_subscribe_and_exchange_a_single_event() {
    let lookup = MemoryLookup::new();

    let (_ep_a, _gossip_a, _router_a, net_a) = spawn_iroh_peer(&lookup).await;
    let (_ep_b, _gossip_b, _router_b, net_b) = spawn_iroh_peer(&lookup).await;

    let a_pk = net_a.peer_pubkey();
    let topic = Topic::from_bytes([0x01; 32]);

    // B subscribes with A as bootstrap; A subscribes with empty.
    let _sub_a = net_a.subscribe(topic, vec![]).await.expect("A subscribe");
    let mut sub_b = net_b
        .subscribe(topic, vec![a_pk])
        .await
        .expect("B subscribe");

    // Give the swarm a brief moment to form before A publishes.
    // Without this, the publish may arrive before B's join completes
    // and Plumtree silently drops it. Timing is best-effort — see
    // spec §6 "bootstrap peer not reachable".
    tokio::time::sleep(Duration::from_millis(200)).await;

    let msg = GossipMessage::HeadsSummary(sample_heads_summary());
    net_a.publish(topic, msg.clone()).await.expect("A publish");

    let received = recv_first_decoded(&mut sub_b).await;
    match received {
        GossipMessage::HeadsSummary(h) => assert_eq!(h.kernel_fuel_table_version, 1),
        other => panic!("expected HeadsSummary, got {other:?}"),
    }
}

// ---- test 2 -----------------------------------------------------------------

/// Covers: networking.md §11.1 — three-peer chain A↔B↔C. A publishes;
/// C must receive via one-hop Plumtree forwarding through B.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subscribe_publishes_propagate_via_gossip_to_three_peers() {
    let lookup = MemoryLookup::new();

    let (_ep_a, _gossip_a, _router_a, net_a) = spawn_iroh_peer(&lookup).await;
    let (_ep_b, _gossip_b, _router_b, net_b) = spawn_iroh_peer(&lookup).await;
    let (_ep_c, _gossip_c, _router_c, net_c) = spawn_iroh_peer(&lookup).await;

    let a_pk = net_a.peer_pubkey();
    let b_pk = net_b.peer_pubkey();
    let topic = Topic::from_bytes([0x02; 32]);

    let _sub_a = net_a.subscribe(topic, vec![]).await.expect("A subscribe");
    let _sub_b = net_b
        .subscribe(topic, vec![a_pk])
        .await
        .expect("B subscribe");
    let mut sub_c = net_c
        .subscribe(topic, vec![b_pk])
        .await
        .expect("C subscribe");

    // Larger sleep — three-peer swarm needs more time to converge.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let msg = GossipMessage::HeadsSummary(sample_heads_summary());
    net_a.publish(topic, msg).await.expect("A publish");

    let received = recv_first_decoded(&mut sub_c).await;
    match received {
        GossipMessage::HeadsSummary(_) => {}
        other => panic!("expected HeadsSummary at C, got {other:?}"),
    }
}

// ---- test 3 -----------------------------------------------------------------

/// Covers: convergence.md §4.4 — wire-decode failure surfaces as
/// `SubError::DecodeFailed`, NOT `SubError::Lagged`. The runtime's
/// distinct handler paths depend on this routing: Lagged triggers
/// `HeadsSummary` backfill; `DecodeFailed` does not (preventing a flood
/// of backfill traffic from a single bad-bytes peer).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn decode_failure_surfaces_as_subscribe_decode_failed() {
    let lookup = MemoryLookup::new();

    // `gossip_a` is threaded through for the backdoor broadcast — we
    // must use the SAME gossip instance as A so the bytes reach B
    // through the gossip overlay rather than a fresh out-of-band peer.
    let (_ep_a, gossip_a, _router_a, net_a) = spawn_iroh_peer(&lookup).await;
    let (_ep_b, _gossip_b, _router_b, net_b) = spawn_iroh_peer(&lookup).await;

    let a_pk = net_a.peer_pubkey();
    let topic = Topic::from_bytes([0x03; 32]);

    let _sub_a = net_a.subscribe(topic, vec![]).await.expect("A subscribe");
    let mut sub_b = net_b
        .subscribe(topic, vec![a_pk])
        .await
        .expect("B subscribe");

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Publish garbage bytes via the gossip backdoor — bypass the
    // IrohNetwork::publish path which would canonical-bincode-encode.
    let topic_id = iroh_topic_id_from_topic(topic);
    let gossip_topic = gossip_a
        .subscribe(topic_id, vec![])
        .await
        .expect("A backdoor subscribe");
    let (sender, _receiver) = gossip_topic.split();
    sender
        .broadcast(Bytes::from_static(b"garbage-not-bincode"))
        .await
        .expect("A backdoor broadcast");

    // B's recv() should now return DecodeFailed with A as the last-hop
    // (in a two-peer setup the last-hop neighbor IS the publisher).
    let result = timeout(Duration::from_secs(5), async {
        loop {
            match sub_b.recv().await {
                Ok(Some(_)) => panic!("unexpectedly got a decoded message"),
                Ok(None) => panic!("subscription closed unexpectedly"),
                Err(SubError::DecodeFailed { peer }) => return peer,
                Err(SubError::Lagged(_) | SubError::TransportError(_)) => {
                    // Tolerated during swarm formation (membership churn
                    // can surface Lagged before the swarm settles). If
                    // this persists until the outer 5s timeout fires,
                    // that is a routing-regression signal worth
                    // investigating: the garbage broadcast SHOULD
                    // produce DecodeFailed; persistent Lagged here
                    // would mean SubError::DecodeFailed has been
                    // collapsed back into SubError::Lagged at the
                    // runtime boundary, undermining the load-bearing
                    // routing distinction per spec §3.0 (and would
                    // re-enable the bad-bytes-peer backfill flood
                    // that the distinct variant exists to prevent).
                    //
                    // TransportError (B-4.3): not expected on this path;
                    // tolerate and continue — persistent TransportError
                    // before timeout would indicate ApiError → DecodeFailed
                    // conflation regression.
                }
            }
        }
    })
    .await
    .expect("timeout waiting for DecodeFailed");

    match result {
        Some(p) => assert_eq!(p, a_pk, "decode-failure peer must be A's pubkey"),
        None => panic!("expected Some(peer) on DecodeFailed, got None"),
    }
}

// ---- test 5 -----------------------------------------------------------------

/// Covers: networking.md §11.1 — the topic-id conversion free
/// function is a pure byte-transparent map (Topic and `TopicId` are
/// both 32-byte newtypes).
#[tokio::test]
async fn topic_id_from_topic_roundtrips() {
    let topic = Topic::from_bytes([0xAA; 32]);
    let topic_id = iroh_topic_id_from_topic(topic);
    assert_eq!(
        topic_id.as_bytes(),
        &[0xAA; 32],
        "TopicId bytes must equal Topic bytes — both are 32-byte transparent newtypes"
    );
}
