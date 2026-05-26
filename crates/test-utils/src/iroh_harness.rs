//! Iroh-backed multi-peer test harness for kernel-tier acceptance
//! tests. Mirrors the shape of `InProcessHarness` (`MemNetwork`) but
//! wires `Runtime::start` to a real `IrohNetwork` over loopback UDP
//! via a shared `iroh::address_lookup::MemoryLookup`.
//!
//! Per docs/specs/2026-05-22-e2e-test-coverage-design.md §3.2.

#![cfg(feature = "network-iroh")]

use iroh::address_lookup::MemoryLookup;
use myrhiza_kernel::identity::{AuthorKeypair, PeerKeypair};
use myrhiza_kernel::runtime::{Runtime, RuntimeCfg};
use myrhiza_kernel::state_apply::StateApplyHandle;
use myrhiza_network::{HEADS_REQUEST_ALPN, IrohNetwork};
use myrhiza_types::{BundleHash, PeerPubkey, Topic};

use crate::harness::PeerHandle;

/// One iroh peer's complete stack: endpoint, gossip handle, router,
/// the `IrohNetwork`, and the `BundleDistribution` (iroh-blobs
/// publish/fetch surface). Ownership lives on the harness so
/// endpoints are not dropped mid-test (dropping the endpoint tears
/// down the UDP socket and silently breaks every running peer).
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
    /// Protocol router accepting `iroh_gossip::ALPN` (always),
    /// `HEADS_REQUEST_ALPN` (when requested), and `iroh_blobs::ALPN`
    /// (always — for B-10 publish/fetch) against this endpoint.
    pub router: iroh::protocol::Router,
    /// The `IrohNetwork` wired to `endpoint` + `gossip`; pass into
    /// `Runtime::start` as the kernel's `Network` impl.
    pub network: IrohNetwork,
    /// The `BundleDistribution` wired to `endpoint`; owns the local
    /// `MemStore` and the `BlobsProtocol` handler registered on the
    /// router for `iroh_blobs::ALPN`. Tests reach in to publish
    /// bundles and pass the resulting `BlobHash` to peers for fetch.
    /// Per B-10 spec §3.6 + §6.3.
    pub distribution: myrhiza_distribution::BundleDistribution,
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
/// derives `secret = SigningKey::from_bytes(seed.to_be_bytes()-padded
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
    // Per B-10 spec §3.6 ¶5 + §6.3: every iroh peer also carries a
    // `BundleDistribution` and registers `iroh_blobs::ALPN` against
    // the same router. Constructed after gossip + endpoint so the
    // `BlobsProtocol` is wired to the same endpoint as the rest of
    // the per-peer stack.
    let distribution = myrhiza_distribution::BundleDistribution::new(endpoint.clone());
    // `BlobsProtocol: Clone` (derived in `iroh_blobs::net_protocol`),
    // so a `.clone()` on the borrowed handler is the natural way to
    // hand an owned handler to the router.
    let mut builder = iroh::protocol::Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .accept(iroh_blobs::ALPN, distribution.protocol_handler().clone());
    if register_heads_alpn {
        builder = builder.accept(HEADS_REQUEST_ALPN, network.protocol_handler());
    }
    let router = builder.spawn();
    IrohPeerStack {
        endpoint,
        gossip,
        router,
        network,
        distribution,
    }
}

/// Multi-peer fixture for iroh-backed convergence + coexistence tests.
///
/// Owns the shared `MemoryLookup` so each spawned peer's address is
/// discoverable by every other peer. Peer stacks are owned by the
/// harness; dropping the harness tears them all down together,
/// avoiding the "endpoint died mid-test" hazard from manual
/// lifecycle management.
///
/// Constructor difference from `InProcessHarness`: there is no
/// `bus_capacity` arg — iroh has no bus. Otherwise the field set
/// matches `InProcessHarness` exactly so test bodies remain
/// near-identical between `MemNetwork` and `IrohNetwork` variants.
pub struct IrohHarness {
    /// Shared address-lookup table all peers register their `addr()` into.
    pub lookup: MemoryLookup,
    /// Application bundle hash bound to this topic (genesis-derived).
    pub app_bundle_hash: BundleHash,
    /// Human-readable topic name (re-used when deriving genesis).
    pub topic_name: String,
    /// 32-byte topic-derivation seed.
    pub seed: [u8; 32],
    /// Pre-computed topic id (derived from `app_bundle_hash`, `seed`, `topic_name`).
    pub topic: Topic,
    /// Owned per-peer stacks. Retained so endpoints and routers
    /// outlive the runtimes they back; dropping the harness drops
    /// all of them together.
    peers: Vec<IrohPeerStack>,
}

impl IrohHarness {
    /// Construct a fresh harness with a private `MemoryLookup`.
    ///
    /// Bundle hash + topic name are fixed at construction to match
    /// `InProcessHarness::new`'s defaults so test bodies stay
    /// near-identical between `MemNetwork` and `IrohNetwork` variants.
    #[must_use]
    pub fn new(seed: [u8; 32]) -> Self {
        let lookup = MemoryLookup::default();
        let app_bundle_hash = BundleHash::from_bytes([0xAB; 32]);
        let topic_name = "main".to_string();
        let topic = Topic::derive(&app_bundle_hash, &seed, &topic_name);
        Self {
            lookup,
            app_bundle_hash,
            topic_name,
            seed,
            topic,
            peers: Vec::new(),
        }
    }

    /// Pubkey of the i-th peer spawned via `spawn_peer`.
    ///
    /// Provided so tests can pass a previously-spawned peer's pubkey
    /// as the `bootstrap` arg of a later `spawn_peer` call without
    /// exposing the internal `peers` vec.
    ///
    /// # Panics
    /// Panics if `index` is out of range (i.e. fewer than `index + 1`
    /// peers have been spawned). Test-only helper; callers are expected
    /// to pass an index they know is valid.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn peer_pubkey(&self, index: usize) -> PeerPubkey {
        self.peers
            .get(index)
            .expect("peer index out of range")
            .network
            .peer_pubkey()
    }

    /// Spawn a peer with the given identity seeds. `bootstrap` is the
    /// pubkey of an already-spawned peer this one should dial; pass an
    /// empty vec for the first peer (it waits for inbound joins).
    ///
    /// Internally derives the iroh endpoint's secret key from the same
    /// `peer_seed` so the endpoint identity equals `peer_key.public` —
    /// required for direct-stream backfill since kernel-issued
    /// `request_heads(target, ...)` dials `target` as an iroh endpoint id,
    /// where `target` is `peer_key.public` from a peer `HeadsSummary`
    /// signature (see plan T3.0 prep). The seed-byte formula here MUST
    /// stay aligned with `PeerKeypair::deterministic` in
    /// `crates/kernel/src/identity/mod.rs` — both use `to_be_bytes`.
    ///
    /// Always registers the heads-request ALPN on every peer because
    /// kernel-tier tests rely on `Runtime`'s `install_request_handler`
    /// call to wire the responder (spec §3.2 load-bearing detail).
    ///
    /// # Panics
    /// Panics if `Runtime::start` fails. The iroh subscribe path can
    /// fail in principle (e.g. invalid bootstrap pubkey), but every
    /// test fixture passes well-formed bootstrap data; a panic here
    /// is a test-infrastructure bug, not a runtime error.
    #[allow(clippy::expect_used)]
    pub async fn spawn_peer(
        &mut self,
        peer_seed: u64,
        author_seed: Option<u64>,
        handle: StateApplyHandle,
        cfg: RuntimeCfg,
        bootstrap: Vec<PeerPubkey>,
    ) -> PeerHandle {
        // Recompute the same seed bytes that `PeerKeypair::deterministic`
        // uses internally (crates/kernel/src/identity/mod.rs:61-65). Both
        // `PeerKeypair` and the iroh endpoint then derive from the same
        // Ed25519 secret, so `network.peer_pubkey() == peer_key.public`.
        // Endianness must match the kernel formula: it uses `to_be_bytes`.
        let mut iroh_secret_bytes = [0u8; 32];
        iroh_secret_bytes[..8].copy_from_slice(&peer_seed.to_be_bytes());

        let stack = spawn_iroh_peer(&self.lookup, Some(iroh_secret_bytes), true).await;
        let peer_key = PeerKeypair::deterministic(peer_seed);
        let author_key = author_seed.map(AuthorKeypair::deterministic);

        // Clone the IrohNetwork handle out of the stack so we can pass
        // it to Runtime::start. The original is retained on the stack
        // for cleanup ordering. Clone is structurally sound — every
        // field is either `Arc`-backed or `Copy`; see the doc comment
        // on `IrohNetwork` in `crates/network/src/iroh_transport.rs`.
        let network = stack.network.clone();

        let runtime = Runtime::start(
            network,
            self.topic,
            self.app_bundle_hash,
            self.topic_name.clone(),
            handle,
            peer_key,
            author_key,
            cfg,
            bootstrap,
        )
        .await
        .expect("Runtime::start (iroh)");

        self.peers.push(stack);
        PeerHandle::from_runtime(runtime)
    }
}

// Suppress unused-import warnings for items used only when running the
// `cargo build` path with no tests; everything below is `#[cfg(test)]`.
#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Sanity: structural check that `iroh_secret`-bytes derived from a
    /// `u64` peer seed via the same formula `PeerKeypair::deterministic`
    /// uses produce `network.peer_pubkey() == PeerKeypair::deterministic(seed).public`.
    /// No `Runtime`, no swarm — this isolates the load-bearing identity
    /// alignment from any networking failure modes. Full integration
    /// validation lives in T4.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn iroh_secret_aligns_network_pubkey_with_peer_key() {
        let lookup = MemoryLookup::default();
        let seed = 7_u64;
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&seed.to_be_bytes());
        let stack = spawn_iroh_peer(&lookup, Some(bytes), true).await;
        let pk = PeerKeypair::deterministic(seed);
        assert_eq!(
            stack.network.peer_pubkey(),
            pk.public,
            "iroh endpoint identity must equal PeerKeypair::deterministic(seed).public",
        );
    }
}
