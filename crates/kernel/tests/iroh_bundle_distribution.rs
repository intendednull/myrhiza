//! Kernel-tier acceptance — closes `mvp.md §15.1 #1` against
//! real iroh-blobs over loopback QUIC.
//!
//! Per `docs/specs/2026-05-26-b-10-bundle-distribution-design.md` §6.3
//! (the load-bearing test) + §3.6 (real iroh-blobs over loopback QUIC).
//!
//! ## What this test proves
//!
//! Criterion #1 (`mvp.md §15.1 #1`) — "kernel loads + instantiates a
//! WASM state component from a bundle fetched via iroh-blobs" — was
//! previously satisfied only against disk-loaded bundles (see
//! `acceptance::kernel_instantiates_and_applies_increment`). This test
//! exercises the full iroh-blobs fetch path end-to-end:
//!
//! 1. Peer A publishes the three-component counter bundle via
//!    `BundleDistribution::publish` (manifest + state-apply +
//!    state-propose + interaction into iroh-blobs `MemStore`).
//! 2. Peer B fetches by `manifest_hash` via `BundleDistribution::fetch`,
//!    bootstrapping against peer A — the blob is NOT in peer B's local
//!    store, so the fetch genuinely traverses the wire via the
//!    iroh-blobs downloader over `iroh_blobs::ALPN`.
//! 3. The materialized `BundleAddress::Disk` feeds `InstallFlow::load`
//!    (signature verify + canonical bytes round-trip).
//! 4. `WasmtimeBackend::instantiate_state_apply` instantiates the
//!    loaded component bytes.
//! 5. Applying counter genesis (empty `app_payload` zero-state seed,
//!    `seq = 1`) yields `0_i64.to_be_bytes()` — the canonical counter
//!    genesis state.
//!
//! ## Cross-peer fetch invariant (load-bearing)
//!
//! Peer B's `BundleDistribution` is constructed against its own
//! `MemStore` — the bundle bytes published on peer A are NOT replicated
//! into peer B at publish time. Step 2 must therefore traverse the
//! iroh-blobs ALPN over loopback QUIC, validating the wire shape
//! (provider lookup via `endpoint.id()`, ALPN dial, BLAKE3 + Bao
//! verified streaming, store-side decode). The same-instance fast path
//! exercised in `crates/distribution/src/blobs.rs::publish_then_fetch_roundtrip`
//! does NOT apply here.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod helpers;

use std::collections::BTreeSet;

use bincode::Options;
use iroh::address_lookup::MemoryLookup;
use myrhiza_backend::Backend;
use myrhiza_kernel::{ApplyOutcome, BundleAddress, InstallFlow, StateApplyHandle};
use myrhiza_network::iroh_transport::peer_pubkey_from_iroh;
use myrhiza_test_utils::bundle::build_signed_counter_bundle_three_components;
use myrhiza_test_utils::iroh_harness::spawn_iroh_peer;
use myrhiza_types::{AuthorPubkey, Event, EventHash, GenesisV1, Hlc, canonical_bincode};
use myrhiza_wasmtime_backend::WasmtimeBackend;

/// Closes `mvp.md §15.1 #1` against real iroh-blobs over loopback QUIC.
///
/// See module docstring for the full auth chain. The test name encodes
/// the criterion reference so a future reader running `cargo test
/// --test iroh_bundle_distribution` immediately sees which spec claim
/// the test backs.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b10_fetch_via_iroh_blobs_closes_mvp_15_1_criterion_1() {
    // Two iroh peers sharing a `MemoryLookup` so peer B can resolve
    // peer A's bound socket without DNS. The seed bytes are
    // arbitrary (no `PeerKeypair` alignment required for the
    // distribution path — that's only load-bearing for the gossip /
    // heads-request paths, which this test does not exercise).
    let lookup = MemoryLookup::default();
    let stack_a = spawn_iroh_peer(&lookup, Some([1; 32]), true).await;
    let stack_b = spawn_iroh_peer(&lookup, Some([2; 32]), true).await;

    // Build the three-component counter bundle on disk, then read it
    // back into memory so we can hand the bytes to
    // `BundleDistribution::publish`. The tempdir is kept alive via the
    // `_bundle` binding so the on-disk files remain readable while we
    // load them. After the bytes are in memory we no longer depend on
    // the disk layout.
    let (test_bundle, _disk_addr) = build_signed_counter_bundle_three_components();
    let manifest_bytes = std::fs::read(test_bundle.bundle_dir.join(&test_bundle.manifest_path))
        .expect("read manifest");
    let manifest: myrhiza_manifest::schema::Manifest = canonical_bincode()
        .deserialize(&manifest_bytes)
        .expect("decode manifest");
    let apply_bytes = std::fs::read(
        test_bundle
            .bundle_dir
            .join(manifest.components.state_apply.as_deref().unwrap()),
    )
    .expect("read state-apply");
    let propose_bytes = manifest
        .components
        .state_propose
        .as_deref()
        .map(|p| std::fs::read(test_bundle.bundle_dir.join(p)).expect("read state-propose"));
    let interaction_bytes = manifest
        .components
        .interaction
        .as_deref()
        .map(|p| std::fs::read(test_bundle.bundle_dir.join(p)).expect("read interaction"));

    // 1. PUBLISH on peer A. The returned hash IS the manifest's
    //    `BundleAddress::IrohBlob` identifier (per `BundleDistribution`
    //    contract) — BLAKE3 over the canonical-bincode manifest bytes.
    let manifest_hash = stack_a
        .distribution
        .publish(
            &manifest,
            &manifest_bytes,
            &apply_bytes,
            propose_bytes.as_deref(),
            interaction_bytes.as_deref(),
            None,
        )
        .await
        .expect("publish");

    // 2. FETCH on peer B using peer A as the sole bootstrap provider.
    //    Peer B's local store does NOT have the manifest hash, so the
    //    fetch MUST traverse `iroh_blobs::ALPN` to peer A's
    //    `BlobsProtocol` over loopback QUIC. The peer ID is derived
    //    from `endpoint.id()` (verified API per
    //    `crates/network/tests/iroh_skeleton.rs`) and converted to
    //    `PeerPubkey` via `peer_pubkey_from_iroh` (Bundle A free
    //    function — orphan rule prevents a `From` impl).
    let peer_a_pubkey = peer_pubkey_from_iroh(stack_a.endpoint.id());
    let materialized = stack_b
        .distribution
        .fetch(manifest_hash, &[peer_a_pubkey])
        .await
        .expect("fetch via real iroh-blobs over loopback QUIC");

    // 3. Verify the materialized address is the `Disk` variant —
    //    `BundleDistribution::fetch` always materializes into a
    //    tempdir, regardless of whether the bundle was originally
    //    published with an `IrohBlob` or `Disk` address.
    let disk_addr = match &materialized.address {
        BundleAddress::Disk { .. } => &materialized.address,
        other @ BundleAddress::IrohBlob { .. } => {
            panic!("expected BundleAddress::Disk, got {other:?}")
        }
    };

    // 4. Load + instantiate. `InstallFlow::load` re-derives
    //    `bundle_content_hash` over the materialized component bytes
    //    and verifies the manifest signature — same path the disk-only
    //    tests exercise. The fact that this succeeds proves the
    //    iroh-blobs fetch produced byte-identical bytes to what was
    //    published (BLAKE3 + Bao verified streaming is load-bearing).
    //
    //    NOTE: `materialized` MUST outlive `flow.load(...)` because
    //    `MaterializedBundle` owns the tempdir via RAII. Dropping
    //    `materialized` before `load` reads the bytes would delete the
    //    tempdir mid-read. Binding it to a `let` above keeps it alive
    //    through the end of the function.
    let flow = InstallFlow::new();
    let loaded = flow.load(disk_addr).expect("install flow loads + verifies");

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let instance = backend
        .instantiate_state_apply(&loaded.component_bytes, &loaded.manifest)
        .expect("instantiate counter state-apply");
    let mut handle = StateApplyHandle::new(instance);

    // 5. Apply counter genesis. Mirrors the shape of
    //    `acceptance::kernel_instantiates_and_applies_increment`:
    //    `GenesisV1::app_payload` is the canonical 8-byte BE i64 zero,
    //    which the counter fixture returns verbatim as the initial
    //    state. The `author`, `seed`, and `signature` fields are
    //    test-shaped (not load-bearing for this acceptance — the
    //    signature is verified at the network insert layer, not by
    //    `handle.apply`).
    let author = AuthorPubkey::from_bytes([1; 32]);
    let initial_state = 0_i64.to_be_bytes().to_vec();
    let genesis_payload = GenesisV1 {
        seed: [0x11; 32],
        founder_pubkey: author,
        app_payload: initial_state.clone(),
    };
    let genesis_payload_bytes = canonical_bincode()
        .serialize(&genesis_payload)
        .expect("encode genesis payload");
    let genesis = Event {
        author,
        seq: 1,
        prev: EventHash::ZERO,
        deps: BTreeSet::new(),
        hlc: Hlc {
            wall_ms: 0,
            logical: 0,
        },
        payload: genesis_payload_bytes,
        signature: [0; 64],
    };
    let genesis_bytes = canonical_bincode()
        .serialize(&genesis)
        .expect("encode genesis event");
    let result = handle
        .apply(&[], &genesis_bytes)
        .expect("genesis apply succeeds against iroh-fetched component");

    assert!(
        matches!(result.outcome, ApplyOutcome::Accepted),
        "expected Accepted verdict for genesis, got {:?}",
        result.outcome
    );
    assert_eq!(
        result.new_state, initial_state,
        "iroh-fetched counter must produce canonical genesis state (0_i64.to_be_bytes())",
    );
}
