//! B-4.0 smoke tests: prove `IrohNetwork` compiles against iroh
//! 1.0.0-rc.0 and the `PeerPubkey` <-> `EndpointId` conversion
//! roundtrips through Bundle A's free-function adaptation.
//!
//! Per docs/specs/2026-05-20-plan-b-4-0-iroh-skeleton-design.md §4 and
//! the API adaptations Bundle A documented in
//! `crates/network/src/iroh_transport.rs` module-level docstring.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use myrhiza_network::iroh_transport::peer_pubkey_from_iroh;
use myrhiza_network::{IrohNetwork, NetError, Network};
use myrhiza_types::Topic;

/// Covers: identity.md §6, plan-b-1 §6 — iroh's `NodeID` pubkey routes
/// through Myrhiza's `PeerPubkey` cleanly. B-4.0 spec §3.2.
///
/// ## API adaptations from plan's hypothetical names
///
/// - `Endpoint::builder()` requires a `Preset` argument in 1.0.0-rc.0;
///   we use `endpoint::presets::Minimal` — the smallest preset that
///   sets the mandatory crypto provider (mandatory per
///   `endpoint/presets.rs` doc: "the only mandatory option to set on
///   the endpoint builder is `Builder::crypto_provider`"). `Empty` is
///   intentionally avoided because it does not set the crypto provider
///   and therefore would always fail to bind.
/// - `Gossip::builder().spawn(endpoint) -> Gossip` is synchronous in
///   `iroh-gossip` 0.99.0 — no `.await`, no `Result`.
/// - `endpoint.id()` replaces the plan's hypothetical `node_id()`.
/// - `PeerPubkey::from(endpoint_id)` is not available (orphan rule).
///   We call Bundle A's free function `peer_pubkey_from_iroh`.
#[tokio::test]
async fn iroh_network_constructs_and_exposes_endpoint_id_as_peer_pubkey() {
    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .bind()
        .await
        .expect("iroh endpoint bind");

    let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());

    let network = IrohNetwork::new(endpoint.clone(), gossip);
    let peer_pk_via_struct = network.peer_pubkey();

    let endpoint_id = endpoint.id();
    let peer_pk_via_conversion = peer_pubkey_from_iroh(endpoint_id);

    assert_eq!(
        peer_pk_via_struct, peer_pk_via_conversion,
        "IrohNetwork::peer_pubkey() must match peer_pubkey_from_iroh() conversion"
    );
    assert_eq!(
        peer_pk_via_struct.as_bytes(),
        endpoint_id.as_bytes(),
        "PeerPubkey bytes must match EndpointId bytes (both 32-byte Ed25519)"
    );
}

/// Covers: B-4.0 §3.2 — skeleton methods return structured errors,
/// not panics. Regression for "skeleton should not crash CI."
#[tokio::test]
async fn iroh_network_subscribe_returns_unimplemented() {
    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .bind()
        .await
        .expect("iroh endpoint bind");
    let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
    let network = IrohNetwork::new(endpoint, gossip);

    let topic = Topic::from_bytes([0xAB; 32]);
    let result = network.subscribe(topic).await;
    // Let-else (not `.expect_err()`) because `IrohSubscription`
    // intentionally does not derive `Debug` in the skeleton.
    let Err(err) = result else {
        panic!("expected Err(NetError::Unimplemented), got Ok(_)");
    };
    match err {
        NetError::Unimplemented { method, planned_in } => {
            assert_eq!(method, "Network::subscribe");
            assert_eq!(planned_in, "B-4.1");
        }
        other => panic!("expected NetError::Unimplemented, got {other:?}"),
    }
}
