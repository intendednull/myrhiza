//! Install flow: load bundle directory, verify Ed25519 signature
//! against the author pubkey embedded in the manifest, return
//! manifest + component bytes ready for backend instantiation.
//!
//! Plan A scope:
//! - Single-component state-apply bundles from a local directory.
//! - No recursive module-dep resolution (plan B).
//! - No user prompt (plan C: kernel-controlled UI surface).
//! - No revocation topic check (plan B).
//!
//! Tampering detection is folded into the signature check: the
//! signing target produced by [`signing_target_bytes`] commits to
//! the BLAKE3 hash of the component bytes, so any byte-level mutation
//! of the artifact post-signing fails as a signature error rather
//! than a separate content-hash mismatch.

use std::path::PathBuf;

use bincode::Options;
use myrhiza_manifest::{
    ParseError, SignatureError, bundle_content_hash,
    canonical::signing_target_bytes,
    schema::{Manifest, Signature},
    verify_signature,
};
use myrhiza_types::{EventHash, canonical_bincode};
use thiserror::Error;

/// The kernel-major value this build of the runtime implements. The
/// install gate rejects any manifest declaring a different major per
/// distribution.md §10.5: a major-bump is a hard ABI change, not a
/// soft kernel-minor extension. Plan A pins `KERNEL_MAJOR_V1 = 1`.
const KERNEL_MAJOR_V1: u32 = 1;

/// Locator for an on-disk bundle. Plan A reads bundles from local
/// directories; plan B fetches them via iroh-blobs and materializes
/// them under a temp dir before passing them through this struct.
#[derive(Debug, Clone)]
pub struct BundleAddress {
    /// Root of the bundle directory (contains `manifest.bincode` and
    /// the `components/` artifact tree).
    pub bundle_dir: PathBuf,
    /// Path of the manifest file (canonical-bincode-encoded) relative
    /// to `bundle_dir`. v1 file naming is `manifest.bincode`. The TOML
    /// human-readable form is canonicalized at publish time; the
    /// kernel only consumes the canonical bytes.
    pub manifest_path: PathBuf,
}

/// Errors returned by [`InstallFlow::load`].
#[derive(Debug, Error)]
pub enum InstallError {
    /// Reading a bundle file from disk failed.
    #[error("io error reading bundle: {0}")]
    Io(#[from] std::io::Error),
    /// Decoding the canonical-bincode-encoded manifest failed. Carries
    /// the underlying message as a `String` so `bincode::Error` does
    /// not leak into the public API (kernel callers should not need to
    /// match against bincode's error variants — install errors are
    /// terminal).
    #[error("bincode decode error reading manifest: {0}")]
    Decode(String),
    /// Manifest text-form parse error (reserved for future TOML-on-disk
    /// support; plan A consumes canonical bincode only).
    #[error("manifest parse error: {0}")]
    Parse(#[from] ParseError),
    /// Manifest is missing the signature section.
    #[error("manifest is missing the signature section")]
    MissingSignature,
    /// Manifest declares a `kernel-major` this runtime does not
    /// implement. Plan A implements `kernel-major = 1`; a manifest
    /// declaring any other major is rejected at install per
    /// distribution.md §10.5 (major-bumps are hard ABI changes).
    /// Carries the manifest-declared value for diagnostic surfaces.
    #[error("incompatible kernel major: manifest declares {0}, this runtime implements 1")]
    IncompatibleKernelMajor(u32),
    /// Ed25519 signature verification failed (covers tampered content
    /// and tampered manifest body alike, since the signing target
    /// commits to both).
    #[error("Ed25519 signature verification failed: {0}")]
    Signature(#[from] SignatureError),
    /// Author pubkey field could not be decoded as 32 raw bytes.
    #[error("author-pubkey field could not be decoded as 32 raw bytes")]
    AuthorPubkeyDecode,
    /// Manifest references components.state-apply but the file is
    /// absent on disk.
    #[error("manifest references components/state-apply but file is absent")]
    ComponentMissing,
}

/// Output of [`InstallFlow::load`]: a verified manifest plus the
/// component bytes ready for backend instantiation.
#[derive(Debug)]
pub struct LoadedBundle {
    /// Canonicalized manifest with signature attached.
    pub manifest: Manifest,
    /// The state-apply component bytes referenced by the manifest.
    pub component_bytes: Vec<u8>,
    /// Composite bundle-content-hash per spec §2 Choice G + §3.4.
    /// Covers all declared component slots; absent slots contribute `[0; 32]`.
    pub content_hash: EventHash,
    /// The state-propose component bytes, if declared in the manifest.
    pub state_propose_bytes: Option<Vec<u8>>,
    /// The interaction component bytes, if declared in the manifest.
    pub interaction_bytes: Option<Vec<u8>>,
    /// The behavior component bytes, if declared in the manifest.
    pub behavior_bytes: Option<Vec<u8>>,
}

/// Stateless install flow. Held by the kernel; calls into it for each
/// bundle that needs loading + verifying.
#[derive(Default)]
pub struct InstallFlow;

impl InstallFlow {
    /// Construct a fresh install flow.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Load and verify the bundle at `addr`.
    ///
    /// # Errors
    ///
    /// Returns [`InstallError::Io`] if a bundle file cannot be read,
    /// [`InstallError::Decode`] if the manifest fails to decode,
    /// [`InstallError::IncompatibleKernelMajor`] if the manifest declares
    /// a `kernel-major` other than the one this runtime implements,
    /// [`InstallError::MissingSignature`] if the manifest is unsigned,
    /// [`InstallError::ComponentMissing`] if the manifest references a
    /// component file that is absent, [`InstallError::AuthorPubkeyDecode`]
    /// if the `author_pubkey` field is not a `0x<hex>` 32-byte string,
    /// or [`InstallError::Signature`] if the Ed25519 signature does not
    /// verify (this also covers the tampered-component case, since
    /// `signing_target_bytes` commits to the component's content hash).
    pub fn load(&self, addr: &BundleAddress) -> Result<LoadedBundle, InstallError> {
        let manifest_bytes = std::fs::read(addr.bundle_dir.join(&addr.manifest_path))?;
        let mut manifest: Manifest = canonical_bincode()
            .deserialize(&manifest_bytes)
            .map_err(|e| InstallError::Decode(e.to_string()))?;
        manifest.canonicalize();

        // Reject incompatible kernel-major up front per distribution.md
        // §10.5. Done before signature verify so a v2-major manifest
        // never reaches the crypto path — keeps the error surface
        // honest about *why* a bundle was rejected.
        if manifest.abi.kernel_major != KERNEL_MAJOR_V1 {
            return Err(InstallError::IncompatibleKernelMajor(
                manifest.abi.kernel_major,
            ));
        }

        let signature: Signature = manifest
            .signature
            .clone()
            .ok_or(InstallError::MissingSignature)?;

        let component_rel = manifest
            .components
            .state_apply
            .clone()
            .ok_or(InstallError::ComponentMissing)?;
        let component_bytes = std::fs::read(addr.bundle_dir.join(&component_rel))?;

        let state_propose_bytes = manifest
            .components
            .state_propose
            .as_ref()
            .map(|rel| std::fs::read(addr.bundle_dir.join(rel)))
            .transpose()?;
        let interaction_bytes = manifest
            .components
            .interaction
            .as_ref()
            .map(|rel| std::fs::read(addr.bundle_dir.join(rel)))
            .transpose()?;
        let behavior_bytes = manifest
            .components
            .behavior
            .as_ref()
            .map(|rel| std::fs::read(addr.bundle_dir.join(rel)))
            .transpose()?;

        let content_hash = bundle_content_hash(
            Some(&component_bytes),
            state_propose_bytes.as_deref(),
            interaction_bytes.as_deref(),
            behavior_bytes.as_deref(),
        );

        // Decode author pubkey from `0x<hex>` form for plan A.
        // Plan B replaces this with bech32m decoding (per
        // distribution.md §10.2 wpub-author HRP).
        let pk = decode_author_pubkey_hex(&manifest.app.author_pubkey)?;

        let target = signing_target_bytes(&manifest, &content_hash);
        verify_signature(&pk, &target, &signature.value)?;

        Ok(LoadedBundle {
            manifest,
            component_bytes,
            content_hash,
            state_propose_bytes,
            interaction_bytes,
            behavior_bytes,
        })
    }
}

fn decode_author_pubkey_hex(s: &str) -> Result<[u8; 32], InstallError> {
    let stripped = s
        .strip_prefix("0x")
        .ok_or(InstallError::AuthorPubkeyDecode)?;
    let bytes = hex::decode(stripped).map_err(|_| InstallError::AuthorPubkeyDecode)?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| InstallError::AuthorPubkeyDecode)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use bincode::Options;
    use ed25519_dalek::{Signer, SigningKey};
    use myrhiza_manifest::{bundle_content_hash, canonical::signing_target_bytes, schema::*};
    use myrhiza_types::canonical_bincode;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    fn write_fixture_bundle(dir: &std::path::Path) -> (BundleAddress, [u8; 32]) {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = sk.verifying_key().to_bytes();
        let pk_hex = hex::encode(pk);

        let component_path = dir.join("components/state-apply.wasm");
        std::fs::create_dir_all(component_path.parent().unwrap()).unwrap();
        // Minimal wasm magic bytes; real component bytes provided by
        // tests/fixtures/built/counter-state-apply.wasm in the e2e test.
        std::fs::write(&component_path, b"\x00asm\x01\x00\x00\x00").unwrap();
        let component_bytes = std::fs::read(&component_path).unwrap();
        let content_hash = bundle_content_hash(Some(&component_bytes), None, None, None);

        let mut helpers = BTreeMap::new();
        helpers.insert("host.hash".into(), true);
        helpers.insert("host.log".into(), true);

        let mut m = Manifest {
            app: AppSection {
                name: "counter".into(),
                version: "0.1.0".into(),
                description: "test".into(),
                author_pubkey: format!("0x{pk_hex}"),
                author_identity_class: AuthorIdentityClass::ThirdParty,
            },
            abi: AbiSection {
                kernel_major: 1,
                kernel_minor_min: 0,
                state_digest_format: StateDigestFormat::Bincode13,
            },
            capabilities: CapabilitiesSection {
                host_imports: BTreeMap::new(),
                ui_surfaces: BTreeMap::new(),
                high_value_ops: HighValueOps::default(),
                deterministic_helpers: helpers,
            },
            determinism: DeterminismSection {
                allow_floats: false,
                drift_detection: DriftDetectionSection {
                    interval_events: 1024,
                },
            },
            modules: ModulesSection { dep: vec![] },
            components: ComponentsSection {
                state_apply: Some("components/state-apply.wasm".into()),
                state_propose: None,
                interaction: None,
                behavior: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: None,
        };
        m.canonicalize();

        let target = signing_target_bytes(&m, &content_hash);
        let sig = sk.sign(&target);
        m.signature = Some(Signature {
            algorithm: SignatureAlgorithm::Ed25519,
            value: sig.to_bytes(),
        });

        let manifest_bytes = canonical_bincode().serialize(&m).unwrap();
        std::fs::write(dir.join("manifest.bincode"), manifest_bytes).unwrap();

        (
            BundleAddress {
                bundle_dir: dir.to_path_buf(),
                manifest_path: "manifest.bincode".into(),
            },
            pk,
        )
    }

    /// Write a fixture bundle with state-apply AND state-propose components.
    /// Signs with the composite [`bundle_content_hash`]. Returns (addr, pk).
    fn write_two_component_fixture_bundle(
        dir: &std::path::Path,
        apply_bytes: &[u8],
        propose_bytes: &[u8],
    ) -> (BundleAddress, [u8; 32]) {
        use myrhiza_manifest::bundle_content_hash;

        let sk = SigningKey::from_bytes(&[9u8; 32]);
        let pk = sk.verifying_key().to_bytes();
        let pk_hex = hex::encode(pk);

        let apply_path = dir.join("components/state-apply.wasm");
        std::fs::create_dir_all(apply_path.parent().unwrap()).unwrap();
        std::fs::write(&apply_path, apply_bytes).unwrap();

        let propose_path = dir.join("components/state-propose.wasm");
        std::fs::write(&propose_path, propose_bytes).unwrap();

        let composite = bundle_content_hash(Some(apply_bytes), Some(propose_bytes), None, None);

        let mut m = Manifest {
            app: AppSection {
                name: "counter".into(),
                version: "0.1.0".into(),
                description: "test".into(),
                author_pubkey: format!("0x{pk_hex}"),
                author_identity_class: AuthorIdentityClass::ThirdParty,
            },
            abi: AbiSection {
                kernel_major: 1,
                kernel_minor_min: 0,
                state_digest_format: StateDigestFormat::Bincode13,
            },
            capabilities: CapabilitiesSection {
                host_imports: BTreeMap::new(),
                ui_surfaces: BTreeMap::new(),
                high_value_ops: HighValueOps::default(),
                deterministic_helpers: {
                    let mut h = BTreeMap::new();
                    h.insert("host.log".into(), true);
                    h
                },
            },
            determinism: DeterminismSection {
                allow_floats: false,
                drift_detection: DriftDetectionSection {
                    interval_events: 1024,
                },
            },
            modules: ModulesSection { dep: vec![] },
            components: ComponentsSection {
                state_apply: Some("components/state-apply.wasm".into()),
                state_propose: Some("components/state-propose.wasm".into()),
                interaction: None,
                behavior: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: None,
        };
        m.canonicalize();

        let target = signing_target_bytes(&m, &composite);
        let sig = sk.sign(&target);
        m.signature = Some(Signature {
            algorithm: SignatureAlgorithm::Ed25519,
            value: sig.to_bytes(),
        });

        let manifest_bytes = canonical_bincode().serialize(&m).unwrap();
        std::fs::write(dir.join("manifest.bincode"), manifest_bytes).unwrap();

        (
            BundleAddress {
                bundle_dir: dir.to_path_buf(),
                manifest_path: "manifest.bincode".into(),
            },
            pk,
        )
    }

    #[test]
    fn install_rejects_tampered_propose_bytes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let apply_bytes = b"\x00asm\x01\x00\x00\x00";
        let propose_bytes = b"\x00asm\x02\x00\x00\x00";
        let (addr, _) = write_two_component_fixture_bundle(tmp.path(), apply_bytes, propose_bytes);

        // Tamper with the propose component after signing.
        std::fs::write(
            tmp.path().join("components/state-propose.wasm"),
            b"\x00asmTAMPERED",
        )
        .unwrap();

        let flow = InstallFlow::new();
        let err = flow.load(&addr).expect_err("tampered propose must reject");
        assert!(
            matches!(err, InstallError::Signature(_)),
            "expected Signature error, got {err:?}"
        );
    }

    #[test]
    fn loads_and_verifies_a_signed_bundle() {
        let tmp = TempDir::new().unwrap();
        let (addr, _pk) = write_fixture_bundle(tmp.path());

        let flow = InstallFlow::new();
        let loaded = flow.load(&addr).expect("load OK");
        assert_eq!(loaded.manifest.app.name, "counter");
        assert!(!loaded.component_bytes.is_empty());
    }

    #[test]
    fn rejects_tampered_component_bytes() {
        let tmp = TempDir::new().unwrap();
        let (addr, _) = write_fixture_bundle(tmp.path());
        // Tamper with the component file post-signing. The signing
        // target commits to BLAKE3(component_bytes), so any byte
        // mutation surfaces as a signature failure rather than a
        // separate content-hash check.
        std::fs::write(
            tmp.path().join("components/state-apply.wasm"),
            b"\x00asmTAMPERED",
        )
        .unwrap();
        let flow = InstallFlow::new();
        let err = flow.load(&addr).expect_err("tampered must reject");
        assert!(matches!(err, InstallError::Signature(_)));
    }

    #[test]
    fn rejects_incompatible_kernel_major() {
        let tmp = TempDir::new().unwrap();
        let (addr, _) = write_fixture_bundle(tmp.path());
        // Re-decode, bump kernel-major to a value this runtime does
        // not implement (2 — reserved for the next ABI break), and
        // re-serialize. The signature is over the v1 signed body, so
        // it would no longer verify either; the kernel-major gate must
        // fire *before* the signature check so the error surface
        // attributes the rejection correctly.
        let mut m: Manifest = canonical_bincode()
            .deserialize(&std::fs::read(tmp.path().join("manifest.bincode")).unwrap())
            .unwrap();
        m.abi.kernel_major = 2;
        std::fs::write(
            tmp.path().join("manifest.bincode"),
            canonical_bincode().serialize(&m).unwrap(),
        )
        .unwrap();
        let flow = InstallFlow::new();
        let err = flow
            .load(&addr)
            .expect_err("kernel-major mismatch must reject");
        match err {
            InstallError::IncompatibleKernelMajor(v) => assert_eq!(v, 2),
            other => panic!("expected IncompatibleKernelMajor(2), got {other:?}"),
        }
    }

    #[test]
    fn rejects_unsigned_manifest() {
        let tmp = TempDir::new().unwrap();
        let (addr, _) = write_fixture_bundle(tmp.path());
        // Strip the signature.
        let mut m: Manifest = canonical_bincode()
            .deserialize(&std::fs::read(tmp.path().join("manifest.bincode")).unwrap())
            .unwrap();
        m.signature = None;
        std::fs::write(
            tmp.path().join("manifest.bincode"),
            canonical_bincode().serialize(&m).unwrap(),
        )
        .unwrap();
        let flow = InstallFlow::new();
        let err = flow.load(&addr).expect_err("missing sig must reject");
        assert!(matches!(err, InstallError::MissingSignature));
    }
}
