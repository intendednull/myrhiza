//! iroh-blobs-backed bundle publish + fetch.
//!
//! Per B-10 spec §3.2 (fetch-side auth chain) + §3.5 (`IrohBlob → Disk`
//! materialization) + §4.3 (`BundleDistribution::publish` / `::fetch`
//! API + `MaterializedBundle` shape). Feature-gated on `network-iroh`.
//!
//! ## Store choice: `MemStore`
//!
//! B-10 wires `iroh_blobs::store::mem::MemStore` (per spec §12 open
//! question 1; `FsStore` wiring is a B-9-adjacent follow-up). `MemStore`
//! is faster for tests and avoids touching the filesystem during
//! publish. Production deployments will swap to `FsStore` through
//! embedder configuration without changing this crate's public API.
//!
//! ## API adaptations vs. the T7/T8 plan sketches
//!
//! Verified against `iroh-blobs 0.101.0` (docs.rs + local source, 2026-05-26):
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
//! - Fetch path: `Store::downloader(&Endpoint) -> Downloader`. The
//!   `Downloader::download(hash, providers)` call returns a
//!   `DownloadProgress` that `IntoFuture`s into `Result<(), n0_error>` —
//!   i.e. completion-only, no bytes. To read fetched bytes, call
//!   `Store::blobs().get_bytes(hash)` (via `Store: Deref<Target = Blobs>`)
//!   after the download completes. The downloader is hash-addressed and
//!   `iroh-blobs` verifies BLAKE3 + Bao framing internally as the bytes
//!   stream in (per `prior-art/iroh/blobs.md` "verified streaming"),
//!   so an `Ok(())` from `.into_future().await` is sufficient proof
//!   that the bytes in the local store hash to the requested address.
//! - Provider list shape: `Vec<iroh::EndpointId>` (the trait blanket
//!   `impl<I: Into<Hash>, T: IntoIterator<Item = I>> SupportedRequest`
//!   collides with iterables of `EndpointId`, so we convert
//!   `&[PeerPubkey]` into a `Vec<EndpointId>` ourselves via
//!   `myrhiza_network::iroh_transport::iroh_endpoint_id_from_peer_pubkey`).
//! - Empty `peers` list: `execute_get` in `iroh-blobs` iterates the
//!   provider stream; an empty stream yields immediately with
//!   "Unable to download" — i.e. the downloader cannot serve from the
//!   local store alone. The publish-then-fetch-on-same-store path
//!   short-circuits via `Store::has(hash)` and reads locally with
//!   `Store::get_bytes`, skipping the downloader entirely.

#![cfg(feature = "network-iroh")]

use bincode::Options;
use myrhiza_manifest::schema::Manifest;
use myrhiza_network::iroh_transport::iroh_endpoint_id_from_peer_pubkey;
use myrhiza_types::{BlobHash, BundleAddress, PeerPubkey, canonical_bincode};
use thiserror::Error;

use crate::conversions::{blob_hash_from_iroh, blob_hash_to_iroh};

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

/// Errors `BundleDistribution::fetch` can return.
///
/// Per B-10 spec §3.2 (fetch-side auth chain).
#[derive(Debug, Error)]
pub enum FetchError {
    /// `iroh-blobs` fetch failed (no peer hosting the requested hash;
    /// connectivity failure; verified-streaming integrity error). The
    /// BLAKE3 + Bao framing in `iroh-blobs` (per
    /// `prior-art/iroh/blobs.md`) treats an integrity mismatch as a
    /// transport error — the bytes that reached the local store would
    /// not match the requested hash, so the download is rejected by
    /// `iroh-blobs` itself before the bytes are visible.
    #[error("iroh-blobs fetch: {0}")]
    BlobsFetch(String),
    /// Decoding the canonical-bincode manifest failed. The manifest
    /// blob bytes hash matched the requested address (iroh-blobs
    /// verified that), but their structure does not decode under
    /// canonical bincode — i.e. either the author published a
    /// non-canonical encoding, or the manifest schema has drifted.
    #[error("decode manifest: {0}")]
    DecodeManifest(String),
    /// The decoded manifest references a component profile but no
    /// `*_hash` field was populated for it — the iroh-blobs fetch
    /// path requires the hash to address the blob.
    ///
    /// This is structurally impossible for manifests signed by a
    /// well-formed `BundleDistribution::publish` call (publish populates
    /// the hash from the bytes before signing). Reaching this error
    /// implies the author signed a malformed manifest by hand or a
    /// tool other than `BundleDistribution::publish`.
    #[error("manifest component {profile} missing iroh-blobs hash — required for IrohBlob fetch")]
    ComponentMissingHash {
        /// Profile name: `"state-apply"`, `"state-propose"`,
        /// `"interaction"`, or `"behavior"`.
        profile: &'static str,
    },
    /// `PeerPubkey` could not be converted to an `iroh::EndpointId`
    /// because the underlying 32 bytes do not form a valid Ed25519
    /// curve point. In normal use this is unreachable: Myrhiza's
    /// internal `PeerPubkey` construction paths all originate from
    /// verified signatures.
    #[error("invalid peer pubkey (not a valid Ed25519 curve point): {0}")]
    InvalidPeer(String),
    /// I/O writing the materialized bundle layout to the tempdir failed.
    #[error("write tempdir: {0}")]
    WriteTempdir(#[from] std::io::Error),
}

/// Output of `BundleDistribution::fetch`: a verified bundle materialized
/// into a tempdir, addressable as `BundleAddress::Disk` for
/// `myrhiza_kernel::InstallFlow::load`.
///
/// The tempdir is owned via RAII — the bundle bytes live only as long
/// as `MaterializedBundle` lives. The kernel embedder must keep the
/// `MaterializedBundle` alive across the call to `InstallFlow::load`.
///
/// Per B-10 spec §3.5 (`IrohBlob → Disk` materialization) + §4.3
/// (`MaterializedBundle` shape).
///
/// The `_tempdir` field carries a leading underscore to signal "owned
/// via RAII; reads are unusual" — the same pattern as
/// `myrhiza_test_utils::bundle::TestBundle::_dir`. The targeted
/// `clippy::pub_underscore_fields` allow exists to accept that
/// nominal convention while keeping the field public so embedders
/// that genuinely need to inspect the tempdir (e.g. for debugging)
/// can reach in.
#[derive(Debug)]
#[allow(clippy::pub_underscore_fields)]
pub struct MaterializedBundle {
    /// RAII tempdir holding the materialized bundle layout
    /// (`manifest.bincode` at the root + `components/state-apply.wasm`,
    /// etc.). Drop-tied to the lifetime of this struct: dropping
    /// `MaterializedBundle` cleans up the tempdir.
    pub _tempdir: tempfile::TempDir,
    /// `BundleAddress::Disk` pointing into `_tempdir`. Pass to
    /// `myrhiza_kernel::InstallFlow::load(&address)`.
    pub address: BundleAddress,
}

/// Holds a local `iroh-blobs` store + protocol handler + the
/// `iroh::Endpoint` already constructed by the kernel embedder.
/// Constructed once at kernel boot, shared across all publish + fetch
/// operations.
///
/// Per B-10 spec §4.3.
pub struct BundleDistribution {
    store: iroh_blobs::store::mem::MemStore,
    blobs_protocol: iroh_blobs::BlobsProtocol,
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

    /// Fetch: pull the manifest + all declared components from peers,
    /// verify each blob's BLAKE3 hash via `iroh-blobs` verified
    /// streaming, materialize into a tempdir mirroring the disk-bundle
    /// layout, return a [`MaterializedBundle`] ready for
    /// `myrhiza_kernel::InstallFlow::load`.
    ///
    /// Per B-10 spec §3.2 (6-step fetch-side auth chain) + §3.5
    /// (`IrohBlob → Disk` materialization story) + §4.3
    /// (`MaterializedBundle` shape).
    ///
    /// `peers` provides bootstrap hints — at least one peer in this
    /// list MUST host the bundle's blobs over `iroh_blobs::ALPN` for
    /// the fetch to succeed. An empty `peers` slice is legal **only**
    /// if the local store already has the bundle (e.g. the
    /// publish-then-fetch-on-same-instance test path).
    ///
    /// Auth chain (per spec §3.2):
    ///
    /// 1. Caller hands kernel a `BlobHash` (out-of-band share).
    /// 2. We pull manifest bytes; iroh-blobs BLAKE3+Bao verifies the
    ///    fetched bytes hash to `manifest_hash`.
    /// 3. Decode manifest via canonical bincode.
    /// 4. For each declared component slot, fetch by the `*_hash`
    ///    recorded in the manifest; iroh-blobs verifies each.
    /// 5. Materialize the bundle into a tempdir mirroring the disk
    ///    bundle layout (`manifest.bincode` + `components/state-apply.wasm`
    ///    etc.).
    /// 6. Caller hands the tempdir's `BundleAddress::Disk` to
    ///    `myrhiza_kernel::InstallFlow::load`, which re-derives
    ///    `bundle_content_hash` over the component bytes and verifies
    ///    the manifest signature — that step is unchanged from the
    ///    disk path.
    ///
    /// # Errors
    ///
    /// - [`FetchError::BlobsFetch`] for transport-layer failures
    ///   (no provider can serve the blob; QUIC connection error;
    ///   verified-streaming integrity error reported as transport).
    /// - [`FetchError::DecodeManifest`] if the manifest blob bytes
    ///   do not decode under canonical bincode.
    /// - [`FetchError::ComponentMissingHash`] if the manifest
    ///   references a profile (e.g. `state-propose`) without
    ///   populating its `*_hash` field.
    /// - [`FetchError::InvalidPeer`] if a `PeerPubkey` in `peers`
    ///   does not form a valid Ed25519 curve point.
    /// - [`FetchError::WriteTempdir`] for I/O errors writing the
    ///   materialized layout.
    pub async fn fetch(
        &self,
        manifest_hash: BlobHash,
        peers: &[PeerPubkey],
    ) -> Result<MaterializedBundle, FetchError> {
        // Convert peers to iroh endpoint IDs once for all blob fetches.
        let endpoint_ids = peer_pubkeys_to_endpoint_ids(peers)?;

        // 1. Pull manifest bytes. iroh-blobs BLAKE3+Bao verifies the
        //    fetched bytes hash to `manifest_hash`; if the bytes that
        //    arrive in the local store do not, the download itself
        //    fails — i.e. an `Ok` here is sufficient proof that the
        //    bytes we read back have the requested hash.
        let manifest_bytes = self.fetch_blob(manifest_hash, &endpoint_ids).await?;

        // 2. Decode the canonical-bincode manifest.
        let manifest: Manifest = canonical_bincode()
            .deserialize(&manifest_bytes)
            .map_err(|e| FetchError::DecodeManifest(e.to_string()))?;

        // 3. For each declared component slot, fetch the referenced blob.
        //
        // state-apply is mandatory per distribution.md §10.2. For the
        // iroh-blobs path, the hash MUST be populated (publish writes
        // it before signing). If not, the manifest is malformed.
        let state_apply_hash =
            manifest
                .components
                .state_apply_hash
                .ok_or(FetchError::ComponentMissingHash {
                    profile: "state-apply",
                })?;
        let state_apply_bytes = self.fetch_blob(state_apply_hash, &endpoint_ids).await?;

        let state_propose_bytes = match (
            manifest.components.state_propose.as_deref(),
            manifest.components.state_propose_hash,
        ) {
            (Some(_), Some(h)) => Some(self.fetch_blob(h, &endpoint_ids).await?),
            (Some(_), None) => {
                return Err(FetchError::ComponentMissingHash {
                    profile: "state-propose",
                });
            }
            _ => None,
        };
        let interaction_bytes = match (
            manifest.components.interaction.as_deref(),
            manifest.components.interaction_hash,
        ) {
            (Some(_), Some(h)) => Some(self.fetch_blob(h, &endpoint_ids).await?),
            (Some(_), None) => {
                return Err(FetchError::ComponentMissingHash {
                    profile: "interaction",
                });
            }
            _ => None,
        };
        let behavior_bytes = match (
            manifest.components.behavior.as_deref(),
            manifest.components.behavior_hash,
        ) {
            (Some(_), Some(h)) => Some(self.fetch_blob(h, &endpoint_ids).await?),
            (Some(_), None) => {
                return Err(FetchError::ComponentMissingHash {
                    profile: "behavior",
                });
            }
            _ => None,
        };

        // 4. Write the materialized layout into a tempdir mirroring
        //    the disk-bundle layout (see
        //    `crates/test-utils/src/bundle.rs::write_bundle`).
        let tempdir = tempfile::TempDir::new()?;
        let bundle_dir = tempdir.path().to_path_buf();
        let manifest_path = std::path::PathBuf::from("manifest.bincode");
        std::fs::write(bundle_dir.join(&manifest_path), &manifest_bytes)?;

        // The `components/` dir is created lazily for the slots that
        // have paths declared. We don't create it unconditionally:
        // a state-apply-only bundle with `state_apply = "state-apply.wasm"`
        // (no leading `components/`) wouldn't need the directory.
        // In practice every bundle puts components under `components/`,
        // but follow the path the manifest declares.
        if let Some(rel) = manifest.components.state_apply.as_deref() {
            write_component(&bundle_dir, rel, &state_apply_bytes)?;
        }
        if let (Some(rel), Some(bytes)) = (
            manifest.components.state_propose.as_deref(),
            state_propose_bytes.as_ref(),
        ) {
            write_component(&bundle_dir, rel, bytes)?;
        }
        if let (Some(rel), Some(bytes)) = (
            manifest.components.interaction.as_deref(),
            interaction_bytes.as_ref(),
        ) {
            write_component(&bundle_dir, rel, bytes)?;
        }
        if let (Some(rel), Some(bytes)) = (
            manifest.components.behavior.as_deref(),
            behavior_bytes.as_ref(),
        ) {
            write_component(&bundle_dir, rel, bytes)?;
        }

        let address = BundleAddress::Disk {
            bundle_dir,
            manifest_path,
        };
        Ok(MaterializedBundle {
            _tempdir: tempdir,
            address,
        })
    }

    /// Fetch a single blob by hash. Reads from the local store if
    /// already present (publish-then-fetch-on-same-store path); else
    /// runs the iroh-blobs downloader against `peers` and then reads.
    ///
    /// `peers` is pre-converted to `iroh::EndpointId` to avoid
    /// re-validating curve points per call.
    async fn fetch_blob(
        &self,
        hash: BlobHash,
        peers: &[iroh::EndpointId],
    ) -> Result<Vec<u8>, FetchError> {
        let iroh_hash = blob_hash_to_iroh(hash);

        // Fast path: if the blob is already in the local store (e.g.
        // publish-then-fetch on the same `BundleDistribution`),
        // `iroh-blobs` `Downloader::download` with no providers
        // returns "Unable to download" because `execute_get` iterates
        // the providers stream before consulting the local store
        // for completeness. Skip the downloader entirely on local hits.
        let has = self
            .store
            .has(iroh_hash)
            .await
            .map_err(|e| FetchError::BlobsFetch(format!("store.has: {e}")))?;

        if !has {
            if peers.is_empty() {
                return Err(FetchError::BlobsFetch(format!(
                    "blob {hash} not in local store and no peers provided"
                )));
            }
            // `Store::downloader(&Endpoint)` constructs an actor-backed
            // `Downloader`; ad-hoc construction per fetch is acceptable
            // for B-10 (one bundle = a few blobs). Future plan may
            // cache a long-lived `Downloader` on `BundleDistribution`
            // (the docstring on `Store::downloader` recommends this for
            // hot paths, but a bundle fetch is a one-shot operation,
            // not a hot loop).
            let downloader = self.store.downloader(&self.endpoint);
            // `Downloader::download(hash, providers)` returns a
            // `DownloadProgress`; `IntoFuture::Output = Result<()>` —
            // see `iroh-blobs::api::downloader::DownloadProgress`. The
            // `providers: impl ContentDiscovery` blanket impl over
            // `IntoIterator<Item = Into<EndpointId>>` accepts a
            // `Vec<EndpointId>` directly.
            downloader
                .download(iroh_hash, peers.to_vec())
                .await
                .map_err(|e| FetchError::BlobsFetch(format!("download {hash}: {e}")))?;
        }

        // Read fetched bytes back from the local store.
        //
        // `Store: Deref<Target = Blobs>`, so `self.store.get_bytes`
        // resolves via Deref. Returns `RequestResult<bytes::Bytes>`.
        let bytes = self
            .store
            .get_bytes(iroh_hash)
            .await
            .map_err(|e| FetchError::BlobsFetch(format!("get_bytes {hash}: {e}")))?;
        Ok(bytes.to_vec())
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

/// Convert a slice of Myrhiza `PeerPubkey` into a `Vec<iroh::EndpointId>`.
///
/// `EndpointId::from_bytes` validates the bytes form a valid Ed25519
/// curve point; in normal use this never fails (Myrhiza's internal
/// `PeerPubkey` construction paths all originate from verified
/// signatures), but the conversion is fallible at the trait level —
/// we surface that as [`FetchError::InvalidPeer`].
fn peer_pubkeys_to_endpoint_ids(peers: &[PeerPubkey]) -> Result<Vec<iroh::EndpointId>, FetchError> {
    peers
        .iter()
        .copied()
        .map(|pk| {
            iroh_endpoint_id_from_peer_pubkey(pk)
                .map_err(|e| FetchError::InvalidPeer(e.to_string()))
        })
        .collect()
}

/// Write a component blob to `bundle_dir/rel`, creating parent
/// directories as needed.
///
/// Mirrors the path layout in `crates/test-utils/src/bundle.rs`
/// (`components/state-apply.wasm`, etc.). Returns an I/O error
/// surfaced as `FetchError::WriteTempdir` via the `?` operator at
/// the call site.
fn write_component(bundle_dir: &std::path::Path, rel: &str, bytes: &[u8]) -> std::io::Result<()> {
    let target = bundle_dir.join(rel);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, bytes)
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

    // ---- T8 state-tier `fetch()` tests ----
    //
    // Exercises the full auth chain (spec §3.2) on a single
    // `BundleDistribution`. Cross-peer fetch (real QUIC dial,
    // `Shuffled` provider list, ALPN registration) lands at kernel-tier
    // in T11's iroh acceptance test — that's the right test layer for
    // the wire-shape coverage, per spec §3.6.

    /// T8 acceptance: publish a counter-style bundle on one
    /// `BundleDistribution`, then fetch via the same instance.
    /// Verifies the full publish → fetch → materialize chain produces
    /// a tempdir layout that matches what `crates/test-utils/src/bundle.rs`
    /// emits — i.e. `InstallFlow::load(Disk)` can consume it unchanged.
    ///
    /// The `peers` slice is empty: same-store fetch uses the fast path
    /// (skip downloader, read locally via `Store::get_bytes`). This is
    /// the path the kernel exercises when the embedder publishes
    /// locally for dev / single-author workflows.
    #[tokio::test]
    async fn publish_then_fetch_roundtrip() {
        let dist = fixture().await;

        // Counter-style state-apply bytes (raw wasm magic for the
        // canonical-bincode + BLAKE3 path — we never instantiate;
        // T11 does the real wasm round-trip).
        let state_apply_bytes: Vec<u8> = b"\x00asm\x01\x00\x00\x00state-apply-v1".to_vec();
        let state_apply_hash = BlobHash::blake3(&state_apply_bytes);

        let mut manifest = manifest_with_state_apply();
        manifest.components.state_apply_hash = Some(state_apply_hash);

        let manifest_bytes = BundleDistribution::encode_manifest(&manifest).expect("encode");
        let manifest_hash = BlobHash::blake3(&manifest_bytes);

        // 1. Publish into the local store.
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
        assert_eq!(returned, manifest_hash);

        // 2. Fetch by manifest hash. Empty peers ⇒ same-store fast path.
        let materialized = dist
            .fetch(manifest_hash, &[])
            .await
            .expect("fetch ok (same-store path)");

        // 3. The address is `Disk`, pointing into the tempdir.
        let (bundle_dir, manifest_rel) = match &materialized.address {
            BundleAddress::Disk {
                bundle_dir,
                manifest_path,
            } => (bundle_dir.clone(), manifest_path.clone()),
            other @ BundleAddress::IrohBlob { .. } => {
                panic!("expected BundleAddress::Disk, got {other:?}")
            }
        };
        assert_eq!(manifest_rel, std::path::PathBuf::from("manifest.bincode"));
        assert!(bundle_dir.is_dir(), "bundle_dir must exist on disk");

        // 4. Re-read the materialized files and assert byte equality
        //    to what publish saw — this is the structural "InstallFlow
        //    can load it" check without dragging the kernel into a
        //    state-tier test. The fact that the read succeeds also
        //    proves the tempdir is alive (i.e. the RAII discipline is
        //    holding the dir open while `materialized` is owned).
        let materialized_manifest_bytes =
            std::fs::read(bundle_dir.join(&manifest_rel)).expect("read manifest");
        assert_eq!(
            materialized_manifest_bytes, manifest_bytes,
            "materialized manifest bytes must equal published canonical bytes"
        );

        let state_apply_rel = manifest
            .components
            .state_apply
            .as_deref()
            .expect("manifest declares state-apply path");
        let materialized_apply_bytes =
            std::fs::read(bundle_dir.join(state_apply_rel)).expect("read state-apply");
        assert_eq!(
            materialized_apply_bytes, state_apply_bytes,
            "materialized state-apply bytes must equal published bytes"
        );

        // `materialized` (and thus the tempdir) is dropped at the end
        // of this fn; `tempfile::TempDir`'s `Drop` removes the dir on
        // best-effort. Asserting post-drop removal would test
        // `tempfile`'s contract, not ours, so we skip it.
    }

    /// T8 acceptance: a manifest declaring `state_propose: Some(path)`
    /// but `state_propose_hash: None` is malformed for the iroh path
    /// (publish populates the hash before signing). Fetch must surface
    /// this as `ComponentMissingHash { profile: "state-propose" }`
    /// rather than panicking or returning a transport error.
    ///
    /// We construct the manifest by hand (not via `publish`) so we
    /// reach the fetch-side gate. In practice, a manifest like this
    /// would also fail signature verification at `InstallFlow::load`
    /// — but that's a second-order check; the fetch path must catch
    /// this structural defect first.
    #[tokio::test]
    async fn fetch_rejects_manifest_with_missing_component_hash() {
        let dist = fixture().await;

        let state_apply_bytes: Vec<u8> = b"\x00asm\x01\x00\x00\x00state-apply".to_vec();
        let state_apply_hash = BlobHash::blake3(&state_apply_bytes);

        let mut manifest = manifest_with_state_apply();
        manifest.components.state_apply_hash = Some(state_apply_hash);
        // Declare state-propose path but DELIBERATELY OMIT the hash —
        // this is the "manifest is malformed for iroh fetch" case.
        manifest.components.state_propose = Some("components/state-propose.wasm".into());
        manifest.components.state_propose_hash = None;

        // We bypass `BundleDistribution::publish` here because publish's
        // defense-in-depth (`check_hash`) would catch the mismatch
        // and reject before importing — but the mismatch is between
        // `bytes` and `declared`, not "declared is None." The publish
        // path with no declared state-propose hash and no bytes
        // succeeds (state-propose is optional). To reach the fetch
        // gate, we add the manifest blob directly via the inner store
        // and skip the component import — the fetch path will look
        // for state-propose-hash, find None, and reject.
        let manifest_bytes = BundleDistribution::encode_manifest(&manifest).expect("encode");
        let manifest_hash = BlobHash::blake3(&manifest_bytes);

        // Import only the manifest + state-apply blobs; deliberately
        // leave state-propose out (the fetch path errors before it
        // would try to fetch a state-propose blob anyway).
        let _ = dist.add_bytes(&state_apply_bytes).await.expect("add");
        let returned = dist.add_bytes(&manifest_bytes).await.expect("add");
        assert_eq!(blob_hash_from_iroh(returned), manifest_hash);

        let err = dist
            .fetch(manifest_hash, &[])
            .await
            .expect_err("must reject");

        match err {
            FetchError::ComponentMissingHash { profile } => {
                assert_eq!(
                    profile, "state-propose",
                    "the missing-hash slot is state-propose"
                );
            }
            other => panic!("expected ComponentMissingHash, got {other:?}"),
        }
    }
}
