//! Manifest builders for tests. Returns canonicalized + signed
//! manifests for common test shapes.

use std::collections::BTreeMap;

use ed25519_dalek::{Signer, SigningKey};
use myrhiza_manifest::{
    canonical::signing_target_bytes,
    schema::{
        AbiSection, AppSection, AuthorIdentityClass, AuthorPolicy, CapabilitiesSection,
        ComponentsSection, DeterminismSection, DriftDetectionSection, HighValueOps, Manifest,
        ModulesSection, Signature, SignatureAlgorithm, StateDigestFormat,
    },
};
use myrhiza_types::EventHash;

/// Build a state-apply manifest declaring just `host.hash` + `host.log`
/// (the minimum useful set for plan A's counter fixture).
#[must_use]
pub fn helpers_only_state_apply_manifest() -> Manifest {
    let mut helpers = BTreeMap::new();
    helpers.insert("host.hash".into(), true);
    helpers.insert("host.log".into(), true);

    let mut m = Manifest {
        app: AppSection {
            name: "test-fixture".into(),
            version: "0.1.0".into(),
            description: "test".into(),
            author_pubkey: "0x".into(), // filled by sign_manifest
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
    m
}

/// Return a fixed test signing key. Same seed across runs — handy for
/// deterministic test fixtures.
#[must_use]
pub fn deterministic_signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

/// Sign `m` against `content_hash` using `key`. Mutates `m.signature`
/// and `m.app.author_pubkey` in place.
pub fn sign_manifest(m: &mut Manifest, content_hash: &EventHash, key: &SigningKey) {
    let pk = key.verifying_key().to_bytes();
    m.app.author_pubkey = format!("0x{}", hex::encode(pk));
    m.canonicalize();
    let target = signing_target_bytes(m, content_hash);
    let sig = key.sign(&target);
    m.signature = Some(Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        value: sig.to_bytes(),
    });
}
