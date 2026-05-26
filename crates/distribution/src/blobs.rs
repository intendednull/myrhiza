//! iroh-blobs-backed bundle publish (and, in T8, fetch).
//!
//! Per B-10 spec §3.2 + §4.3. Feature-gated on `network-iroh`.
//!
//! ## Store choice: `MemStore`
//!
//! B-10 wires `iroh_blobs::store::mem::MemStore` (per spec §12 open
//! question 1; `FsStore` wiring is a B-9-adjacent follow-up). `MemStore`
//! is faster for tests and avoids touching the filesystem during
//! publish. Production deployments will swap to `FsStore` through
//! embedder configuration without changing this crate's public API.
//!
//! ## API adaptations vs. the T7 plan sketch
//!
//! Verified against `iroh-blobs 0.101.0` (docs.rs, 2026-05-26):
//!
//! - `MemStore` lives at `iroh_blobs::store::mem::MemStore` (not
//!   `iroh_blobs::store::MemStore`). It is constructed via
//!   `MemStore::new()`, implements `Clone`/`Send`/`Sync` natively, and
//!   `Deref<Target = iroh_blobs::api::Store>`. No `Arc` wrap needed.
//! - `BlobsProtocol::new(&store, events)` takes `&iroh_blobs::api::Store`,
//!   not `&MemStore`. We rely on `MemStore: Deref<Target = Store>` and
//!   pass `&*self.store`.
//! - `MemStore::add_bytes` takes `impl Into<bytes::Bytes>` and returns
//!   `AddProgress<'_>`, which implements `IntoFuture<Output =
//!   RequestResult<TagInfo>>`. `TagInfo.hash` is a `pub` field of type
//!   `iroh_blobs::Hash` (direct field access, not a method).
//! - The ALPN constant is `iroh_blobs::ALPN` (re-exported from
//!   `iroh_blobs::protocol::ALPN`).

#![cfg(feature = "network-iroh")]

use bincode::Options;
use myrhiza_manifest::schema::Manifest;
use myrhiza_types::{BlobHash, canonical_bincode};
use thiserror::Error;

use crate::conversions::blob_hash_from_iroh;

/// Errors `BundleDistribution::publish` can return.
#[derive(Debug, Error)]
pub enum PublishError {
    /// Encoding the manifest to canonical bincode failed.
    #[error("encode manifest: {0}")]
    EncodeManifest(String),
    /// `iroh-blobs` `add_bytes` failed for one of the imported blobs.
    #[error("iroh-blobs add: {0}")]
    BlobsAdd(String),
    /// The manifest carries no `state_apply` component. At least
    /// state-apply must be present per distribution.md §10.2.
    #[error("manifest declares no state-apply component — invalid bundle")]
    MissingStateApply,
    /// A `*_hash` field in the manifest does not match the bytes
    /// supplied to `publish`. Publish-side defense-in-depth: the
    /// author wrote the hash into the manifest and signed it, so a
    /// mismatch is structurally invalid downstream — surface it now
    /// instead of letting the fetch path discover it.
    #[error("component hash mismatch: manifest declares {expected}, actual bytes hash to {actual}")]
    ComponentHashMismatch {
        /// The hash recorded in the manifest's `_hash` field.
        expected: BlobHash,
        /// The BLAKE3 of the bytes actually passed to `publish`.
        actual: BlobHash,
    },
}

/// Holds a local `iroh-blobs` store + protocol handler + the
/// `iroh::Endpoint` already constructed by the kernel embedder.
/// Constructed once at kernel boot, shared across all publish + fetch
/// operations.
///
/// Per B-10 spec §4.3.
///
/// The `endpoint` field is retained for T8 (`fetch`); we keep it here
/// so publish + fetch share the same iroh transport stack.
pub struct BundleDistribution {
    store: iroh_blobs::store::mem::MemStore,
    blobs_protocol: iroh_blobs::BlobsProtocol,
    #[allow(dead_code)]
    endpoint: iroh::Endpoint,
}

impl BundleDistribution {
    /// Construct from a pre-built `iroh::Endpoint`. Spins up a new
    /// `MemStore` + `BlobsProtocol`.
    ///
    /// The caller is responsible for registering
    /// `iroh_blobs::ALPN` against the router that owns this endpoint
    /// (per spec §4.3 wiring preconditions) — typically via
    /// `BundleDistribution::protocol_handler()`.
    ///
    /// `MemStore: Clone + Send + Sync` natively, so no `Arc` wrap is
    /// needed here.
    #[must_use]
    pub fn new(endpoint: iroh::Endpoint) -> Self {
        let store = iroh_blobs::store::mem::MemStore::new();
        // Pass `None` for the EventSender per spec §12 question 2 —
        // tests don't need progress notifications, and the production
        // observability path is out of scope for B-10.
        //
        // `MemStore: Deref<Target = iroh_blobs::api::Store>`, so
        // `&*store` produces the `&Store` `BlobsProtocol::new`
        // expects.
        let blobs_protocol = iroh_blobs::BlobsProtocol::new(&store, None);
        Self {
            store,
            blobs_protocol,
            endpoint,
        }
    }

    /// Borrow the `BlobsProtocol` for router registration.
    ///
    /// Caller registers `iroh_blobs::ALPN` against this handler in
    /// their `iroh::protocol::Router::builder` before spawning the
    /// router.
    #[must_use]
    pub fn protocol_handler(&self) -> &iroh_blobs::BlobsProtocol {
        &self.blobs_protocol
    }

    /// Borrow the inner `MemStore`.
    ///
    /// Exposed for T8 (`fetch`) and integration tests that need to
    /// inspect what was imported. The public surface is intentionally
    /// minimal — callers should prefer `publish` / `protocol_handler`.
    #[must_use]
    pub fn store(&self) -> &iroh_blobs::store::mem::MemStore {
        &self.store
    }

    /// Publish: import the manifest + all declared component blobs
    /// into the local `iroh-blobs` store. Returns the manifest's
    /// `BlobHash`, which is the `BundleAddress::IrohBlob` identifier.
    ///
    /// Defense-in-depth: cross-checks that each provided component
    /// blob's actual BLAKE3 hash matches the `*_hash` field declared
    /// in the manifest. Mismatch is a publish-side author error (the
    /// signature would not validate downstream anyway).
    ///
    /// Per B-10 spec §3.2 + §4.3.
    ///
    /// # Errors
    ///
    /// - [`PublishError::MissingStateApply`] if the manifest declares
    ///   no `state_apply` component.
    /// - [`PublishError::ComponentHashMismatch`] if a provided bytes /
    ///   declared hash pair disagrees.
    /// - [`PublishError::BlobsAdd`] if `iroh-blobs` rejects an import.
    /// - [`PublishError::EncodeManifest`] is reserved for callers that
    ///   want to construct manifest bytes via this crate; the public
    ///   `publish` API takes pre-encoded bytes so this variant is not
    ///   raised from here today. (Retained for future symmetry with a
    ///   `publish_manifest(&Manifest)` convenience.)
    pub async fn publish(
        &self,
        manifest: &Manifest,
        manifest_bytes: &[u8],
        state_apply_bytes: &[u8],
        state_propose_bytes: Option<&[u8]>,
        interaction_bytes: Option<&[u8]>,
        behavior_bytes: Option<&[u8]>,
    ) -> Result<BlobHash, PublishError> {
        // state-apply is mandatory. Treat either the path slot OR the
        // hash slot being populated as "declared" — disk-only bundles
        // populate path only, iroh-publish populates both.
        if manifest.components.state_apply.is_none()
            && manifest.components.state_apply_hash.is_none()
        {
            return Err(PublishError::MissingStateApply);
        }

        // Defense-in-depth: every provided bytes/hash pair must agree.
        Self::check_hash(state_apply_bytes, manifest.components.state_apply_hash)?;
        if let Some(b) = state_propose_bytes {
            Self::check_hash(b, manifest.components.state_propose_hash)?;
        }
        if let Some(b) = interaction_bytes {
            Self::check_hash(b, manifest.components.interaction_hash)?;
        }
        if let Some(b) = behavior_bytes {
            Self::check_hash(b, manifest.components.behavior_hash)?;
        }

        // Import each component blob into the local store.
        self.add_bytes(state_apply_bytes).await?;
        if let Some(b) = state_propose_bytes {
            self.add_bytes(b).await?;
        }
        if let Some(b) = interaction_bytes {
            self.add_bytes(b).await?;
        }
        if let Some(b) = behavior_bytes {
            self.add_bytes(b).await?;
        }

        // Import the manifest blob last — its hash is what we return
        // (the `BundleAddress::IrohBlob` identifier per spec §4.3).
        let manifest_iroh_hash = self.add_bytes(manifest_bytes).await?;
        Ok(blob_hash_from_iroh(manifest_iroh_hash))
    }

    /// Canonical-encode a `Manifest` to bytes for `publish`.
    ///
    /// Helper for callers that want to compute the canonical bytes
    /// without a second `bincode` import. Not used internally because
    /// `publish` accepts pre-encoded bytes (the caller usually already
    /// has them — they're what the manifest's signature commits to).
    ///
    /// # Errors
    ///
    /// Returns [`PublishError::EncodeManifest`] if canonical-bincode
    /// fails (in practice: only on `Vec` OOM).
    pub fn encode_manifest(manifest: &Manifest) -> Result<Vec<u8>, PublishError> {
        canonical_bincode()
            .serialize(manifest)
            .map_err(|e| PublishError::EncodeManifest(e.to_string()))
    }

    /// Import a single blob into the local store. Returns the
    /// `iroh_blobs::Hash` the store assigned (which, since both
    /// `BlobHash` and `iroh-blobs` use BLAKE3 over the same input,
    /// equals `BlobHash::blake3(bytes)` byte-for-byte).
    async fn add_bytes(&self, bytes: &[u8]) -> Result<iroh_blobs::Hash, PublishError> {
        // `MemStore::add_bytes` takes `impl Into<bytes::Bytes>`. Both
        // `Vec<u8>` and `&[u8]` satisfy that via the `bytes` crate's
        // `From` impls; we pass an owned `Vec` to avoid the extra
        // copy that `&[u8] -> Bytes` would force (`Bytes::from(slice)`
        // allocates internally either way).
        //
        // `AddProgress: IntoFuture<Output = RequestResult<TagInfo>>`,
        // so `.await` resolves to `Result<TagInfo, RequestError>`.
        // `TagInfo.hash: iroh_blobs::Hash` is a public field.
        let tag = self
            .store
            .add_bytes(bytes.to_vec())
            .await
            .map_err(|e| PublishError::BlobsAdd(format!("add_bytes: {e}")))?;
        Ok(tag.hash)
    }

    /// Cross-check that `bytes` actually hash to `declared` (if a
    /// hash was declared in the manifest).
    ///
    /// `declared = None` is the disk-only case — no iroh hash claim,
    /// so trivially OK. For iroh-publish, `declared` must be `Some`
    /// for every component the caller supplies (the author populates
    /// these before signing the manifest), and the actual bytes must
    /// hash to that exact value.
    fn check_hash(bytes: &[u8], declared: Option<BlobHash>) -> Result<(), PublishError> {
        let actual = BlobHash::blake3(bytes);
        match declared {
            None => Ok(()),
            Some(d) if d == actual => Ok(()),
            Some(d) => Err(PublishError::ComponentHashMismatch {
                expected: d,
                actual,
            }),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    use myrhiza_manifest::schema::{
        AbiSection, AppSection, AuthorIdentityClass, AuthorPolicy, CapabilitiesSection,
        ComponentsSection, DeterminismSection, DriftDetectionSection, HighValueOps, Manifest,
        ModulesSection, StateDigestFormat,
    };

    /// Minimal manifest with a `state_apply` slot populated. Caller
    /// fills `*_hash` fields per test.
    fn manifest_with_state_apply() -> Manifest {
        Manifest {
            app: AppSection {
                name: "counter".into(),
                version: "0.1.0".into(),
                description: "Simple shared counter".into(),
                author_pubkey: "wpub-author1q9q...xy".into(),
                author_identity_class: AuthorIdentityClass::ThirdParty,
            },
            abi: AbiSection {
                kernel_major: 1,
                kernel_minor_min: 0,
                state_digest_format: StateDigestFormat::Bincode13,
            },
            capabilities: CapabilitiesSection {
                host_imports: std::collections::BTreeMap::new(),
                ui_surfaces: std::collections::BTreeMap::new(),
                high_value_ops: HighValueOps::default(),
                deterministic_helpers: std::collections::BTreeMap::new(),
            },
            determinism: DeterminismSection {
                allow_floats: false,
                drift_detection: DriftDetectionSection {
                    interval_events: 1024,
                },
            },
            modules: ModulesSection { dep: Vec::new() },
            components: ComponentsSection {
                state_apply: Some("components/state-apply.wasm".into()),
                state_apply_hash: None,
                state_propose: None,
                state_propose_hash: None,
                interaction: None,
                interaction_hash: None,
                behavior: None,
                behavior_hash: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: None,
        }
    }

    /// Manifest that declares NO state-apply (neither path nor hash).
    /// Used to drive the `MissingStateApply` branch.
    fn manifest_without_state_apply() -> Manifest {
        let mut m = manifest_with_state_apply();
        m.components.state_apply = None;
        m.components.state_apply_hash = None;
        m
    }

    /// Build a `BundleDistribution` on a fresh loopback endpoint.
    /// Borrowed from `crates/network/tests/iroh_skeleton.rs` — the
    /// `Minimal` preset is the only one that sets the mandatory
    /// crypto provider.
    async fn fixture() -> BundleDistribution {
        let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
            .bind()
            .await
            .expect("iroh endpoint bind");
        BundleDistribution::new(endpoint)
    }

    // ---- Pure helper tests (no endpoint required) ----

    #[test]
    fn check_hash_accepts_none_declared() {
        // Disk-only bundles: no declared hash → trivially OK.
        assert!(BundleDistribution::check_hash(b"any", None).is_ok());
    }

    #[test]
    fn check_hash_accepts_matching_declared() {
        let bytes = b"\x00asm\x01\x00\x00\x00";
        let h = BlobHash::blake3(bytes);
        assert!(BundleDistribution::check_hash(bytes, Some(h)).is_ok());
    }

    #[test]
    fn check_hash_rejects_mismatched_declared() {
        let bytes = b"\x00asm\x01\x00\x00\x00";
        let wrong = BlobHash::from_bytes([0xAA; 32]);
        let err = BundleDistribution::check_hash(bytes, Some(wrong)).expect_err("must reject");
        match err {
            PublishError::ComponentHashMismatch { expected, actual } => {
                assert_eq!(expected, wrong);
                assert_eq!(actual, BlobHash::blake3(bytes));
            }
            other => panic!("expected ComponentHashMismatch, got {other:?}"),
        }
    }

    #[test]
    fn encode_manifest_round_trips_to_canonical_bincode() {
        let m = manifest_with_state_apply();
        let via_helper = BundleDistribution::encode_manifest(&m).expect("encode");
        let via_canonical = canonical_bincode().serialize(&m).expect("encode direct");
        assert_eq!(via_helper, via_canonical);
    }

    // ---- State-tier publish() tests (require a tokio runtime +
    // a real iroh::Endpoint; per spec §4.6 we lean on Minimal preset
    // here rather than dragging in the IrohHarness from
    // crates/test-utils, which would create a cyclic dev-dep).

    /// T7 acceptance: success path — publish a well-formed bundle,
    /// verify the returned `BlobHash` equals `BlobHash::blake3` over
    /// the manifest bytes. Demonstrates that `iroh-blobs` and our
    /// `BlobHash` are BLAKE3-compatible byte-for-byte (B-10 spec §4.2).
    #[tokio::test]
    async fn publish_success_returns_manifest_hash() {
        let dist = fixture().await;

        let state_apply_bytes: Vec<u8> = b"\x00asm\x01\x00\x00\x00state-apply".to_vec();
        let state_apply_hash = BlobHash::blake3(&state_apply_bytes);

        let mut manifest = manifest_with_state_apply();
        manifest.components.state_apply_hash = Some(state_apply_hash);

        let manifest_bytes = BundleDistribution::encode_manifest(&manifest).expect("encode");
        let expected = BlobHash::blake3(&manifest_bytes);

        let returned = dist
            .publish(
                &manifest,
                &manifest_bytes,
                &state_apply_bytes,
                None,
                None,
                None,
            )
            .await
            .expect("publish ok");

        assert_eq!(
            returned, expected,
            "returned hash must equal BLAKE3 of manifest bytes"
        );
    }

    /// T7 acceptance: reject manifests with no `state_apply`. Per
    /// distribution.md §10.2 every bundle must carry at least the
    /// state-apply component (it's the authority for the app's
    /// event-application logic).
    #[tokio::test]
    async fn publish_rejects_missing_state_apply() {
        let dist = fixture().await;

        let manifest = manifest_without_state_apply();
        let manifest_bytes = BundleDistribution::encode_manifest(&manifest).expect("encode");

        let err = dist
            .publish(&manifest, &manifest_bytes, b"unused", None, None, None)
            .await
            .expect_err("must reject");
        assert!(
            matches!(err, PublishError::MissingStateApply),
            "expected MissingStateApply, got {err:?}",
        );
    }

    /// T7 acceptance: reject bytes whose BLAKE3 doesn't match the
    /// `*_hash` field in the manifest. Publish-side defense-in-depth
    /// (the author would have signed garbage otherwise).
    #[tokio::test]
    async fn publish_rejects_hash_mismatch() {
        let dist = fixture().await;

        let actual_bytes: Vec<u8> = b"\x00asm\x01\x00\x00\x00actual".to_vec();
        let actual_hash = BlobHash::blake3(&actual_bytes);
        let wrong_hash = BlobHash::from_bytes([0xCC; 32]);
        assert_ne!(actual_hash, wrong_hash);

        let mut manifest = manifest_with_state_apply();
        // Manifest declares the wrong hash for state-apply — author
        // error or tampering. Publish must catch this.
        manifest.components.state_apply_hash = Some(wrong_hash);

        let manifest_bytes = BundleDistribution::encode_manifest(&manifest).expect("encode");

        let err = dist
            .publish(&manifest, &manifest_bytes, &actual_bytes, None, None, None)
            .await
            .expect_err("must reject");
        match err {
            PublishError::ComponentHashMismatch { expected, actual } => {
                assert_eq!(expected, wrong_hash);
                assert_eq!(actual, actual_hash);
            }
            other => panic!("expected ComponentHashMismatch, got {other:?}"),
        }
    }
}
