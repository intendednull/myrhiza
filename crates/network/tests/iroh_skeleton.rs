//! B-4.0 smoke tests: prove `IrohNetwork` compiles against iroh
//! 1.0.0-rc.0 and the `PeerPubkey` <-> `EndpointId` conversion
//! roundtrips through Bundle A's free-function adaptation.
//!
//! Per docs/specs/2026-05-20-plan-b-4-0-iroh-skeleton-design.md §4 and
//! the API adaptations Bundle A documented in
//! `crates/network/src/iroh_transport.rs` module-level docstring.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use myrhiza_network::IrohNetwork;
use myrhiza_network::iroh_transport::peer_pubkey_from_iroh;

/// Covers: identity.md §6, networking.md §11.1 — iroh's `NodeID` pubkey
/// (Ed25519 raw 32-byte key, per identity.md §6) is the same primitive
/// as Myrhiza's `PeerPubkey`; this verifies the conversion preserves
/// bytes through the `IrohNetwork` constructor / cached getter and
/// through Bundle A's free-function `peer_pubkey_from_iroh`.
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
