//! Iroh-backed multi-peer test harness for kernel-tier acceptance
//! tests. Mirrors the shape of `InProcessHarness` (`MemNetwork`) but
//! wires `Runtime::start` to a real `IrohNetwork` over loopback UDP
//! via a shared `iroh::address_lookup::MemoryLookup`.
//!
//! Per docs/specs/2026-05-22-e2e-test-coverage-design.md §3.2.

#![cfg(feature = "network-iroh")]

use iroh::address_lookup::MemoryLookup;
use myrhiza_network::{HEADS_REQUEST_ALPN, IrohNetwork};

/// One iroh peer's complete stack: endpoint, gossip handle, router,
/// and the `IrohNetwork`. Ownership lives on the harness so endpoints
/// are not dropped mid-test (dropping the endpoint tears down the
/// UDP socket and silently breaks every running peer).
///
/// Fields are pub for the rare test that needs to reach below the
/// harness API (e.g. publishing raw bytes through `gossip` to
/// exercise decode failure). Prefer the harness API where it suffices.
pub struct IrohPeerStack {
    /// The iroh endpoint bound to a loopback UDP socket. Dropping
    /// this tears down the socket and breaks every running peer.
    pub endpoint: iroh::Endpoint,
    /// Gossip handle on top of `endpoint`, used by `IrohNetwork` for
    /// topic publish/subscribe.
    pub gossip: iroh_gossip::Gossip,
    /// Protocol router accepting `iroh_gossip::ALPN` (always) and
    /// `HEADS_REQUEST_ALPN` (when requested) against this endpoint.
    pub router: iroh::protocol::Router,
    /// The `IrohNetwork` wired to `endpoint` + `gossip`; pass into
    /// `Runtime::start` as the kernel's `Network` impl.
    pub network: IrohNetwork,
}

/// Spin up a fresh iroh endpoint + gossip + router for a test peer.
///
/// The caller provides a shared `MemoryLookup` — each peer's `addr()`
/// is registered into it after bind so that bootstrap by `EndpointId`
/// resolves to a real socket address. Without this, `endpoint.connect`
/// has no addressing information and the dial silently times out.
///
/// If `iroh_secret` is `Some(bytes)`, the endpoint is constructed with
/// that Ed25519 secret. The kernel's `PeerKeypair::deterministic(seed)`
/// derives `secret = SigningKey::from_bytes(seed.to_le_bytes()-padded
/// to 32 bytes)`. Passing the same bytes here makes
/// `network.peer_pubkey() == peer_key.public`, which is required for
/// kernel-issued `request_heads(target, ...)` to dial the correct iroh
/// endpoint (the target identifier comes from `signed_by_peer` in a
/// `HeadsSummary`, which is `peer_key.public`).
///
/// If `register_heads_alpn` is true, the Router also accepts
/// `HEADS_REQUEST_ALPN` against `network.protocol_handler()`. Kernel-
/// tier tests always need this, so the `IrohHarness` always passes true.
///
/// Mirrors `crates/network/tests/direct_streams_iroh.rs::spawn_iroh_peer`.
/// Duplication accepted per spec §2 Choice A (avoiding `network →
/// test-utils` dev-dep cycle).
///
/// # Panics
///
/// Panics if binding the iroh endpoint fails. This is a test-only
/// helper; a bind failure indicates broken loopback or a conflicting
/// socket, neither of which a test can recover from.
#[allow(clippy::expect_used)]
pub async fn spawn_iroh_peer(
    lookup: &MemoryLookup,
    iroh_secret: Option<[u8; 32]>,
    register_heads_alpn: bool,
) -> IrohPeerStack {
    let mut endpoint_builder =
        iroh::Endpoint::builder(iroh::endpoint::presets::Minimal).address_lookup(lookup.clone());
    if let Some(bytes) = iroh_secret {
        endpoint_builder = endpoint_builder.secret_key(iroh::SecretKey::from_bytes(&bytes));
    }
    let endpoint = endpoint_builder.bind().await.expect("iroh endpoint bind");
    lookup.add_endpoint_info(endpoint.addr());
    let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
    let network = IrohNetwork::new(endpoint.clone(), gossip.clone());
    let mut builder =
        iroh::protocol::Router::builder(endpoint.clone()).accept(iroh_gossip::ALPN, gossip.clone());
    if register_heads_alpn {
        builder = builder.accept(HEADS_REQUEST_ALPN, network.protocol_handler());
    }
    let router = builder.spawn();
    IrohPeerStack {
        endpoint,
        gossip,
        router,
        network,
    }
}
