//! Manifest builders for tests. Returns canonicalized + signed
//! manifests for common test shapes.

use std::collections::BTreeMap;

use ed25519_dalek::{Signer, SigningKey};
use myrhiza_manifest::{
    bundle_content_hash,
    canonical::signing_target_bytes,
    schema::{
        AbiSection, AppSection, AuthorIdentityClass, AuthorPolicy, CapabilitiesSection,
        ComponentsSection, DeterminismSection, DriftDetectionSection, HighValueOps, Manifest,
        ModulesSection, Signature, SignatureAlgorithm, StateDigestFormat,
    },
};
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
    };
    m.canonicalize();
    m
}

/// Build a manifest declaring three component slots (state-apply,
/// state-propose, interaction) with the same helper set as
/// [`helpers_only_state_apply_manifest`].
///
/// Used by [`crate::bundle::build_signed_counter_bundle_three_components`]
/// to create a fully-populated counter bundle covering the
/// `view → dispatch → propose → apply` loop required by spec §3.6.
/// The bundle's component bytes come from `examples/counter/` (per
/// docs/specs/2026-05-26-b-8-sdk-design.md §3.3) built via the
/// Justfile's `_build-example` recipe.
#[must_use]
pub fn helpers_only_three_component_manifest() -> Manifest {
    let mut helpers = BTreeMap::new();
    helpers.insert("host.hash".into(), true);
    helpers.insert("host.log".into(), true);

    let mut m = Manifest {
        app: AppSection {
            name: "test-fixture".into(),
            version: "0.1.0".into(),
            description: "test".into(),
            author_pubkey: "0x".into(), // filled by sign_manifest_three_components
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
            state_apply_hash: None,
            state_propose: Some("components/state-propose.wasm".into()),
            state_propose_hash: None,
            interaction: Some("components/interaction.wasm".into()),
            interaction_hash: None,
            behavior: None,
            behavior_hash: None,
        },
        author_policy: AuthorPolicy::default_deny(),
        signature: None,
    };
    m.canonicalize();
    m
}

/// Sign `m` against the composite bundle-content-hash of all three component
/// byte slices (state-apply, state-propose, interaction; behavior absent →
/// `[0; 32]` sentinel per spec §3.4). Mutates `m.signature` and
/// `m.app.author_pubkey` in place.
pub fn sign_manifest_three_components(
    m: &mut Manifest,
    apply_bytes: &[u8],
    propose_bytes: &[u8],
    interaction_bytes: &[u8],
    key: &SigningKey,
) {
    let composite = bundle_content_hash(
        Some(apply_bytes),
        Some(propose_bytes),
        Some(interaction_bytes),
        None,
    );
    let pk = key.verifying_key().to_bytes();
    m.app.author_pubkey = format!("0x{}", hex::encode(pk));
    m.canonicalize();
    let target = signing_target_bytes(m, &composite);
    let sig = key.sign(&target);
    m.signature = Some(Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        value: sig.to_bytes(),
    });
}

/// Like [`helpers_only_state_apply_manifest`] but augmented with one
/// extra entry under `capabilities.host_imports` set to `true`.
///
/// Used by acceptance tests for `mvp.md §15.1 #5` manifest-arm: a
/// state-apply bundle whose manifest declares a non-deterministic
/// capability (e.g. `host.broadcast`) must be rejected at install
/// regardless of what the underlying component actually imports.
/// The counter fixture itself does not import `host.broadcast`; the
/// rejection comes from the manifest gating step in
/// `validate_manifest`, not from the linker.
#[must_use]
pub fn helpers_only_state_apply_manifest_with_extra_cap(extra_cap: &str) -> Manifest {
    let mut m = helpers_only_state_apply_manifest();
    m.capabilities
        .host_imports
        .insert(extra_cap.to_owned(), true);
    m.canonicalize();
    m
}

/// Return a fixed test signing key. Same seed across runs — handy for
/// deterministic test fixtures.
#[must_use]
pub fn deterministic_signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

/// Sign `m` against the composite bundle-content-hash of `component_bytes`
/// (state-apply only; absent slots contribute `[0; 32]` sentinels per spec §3.4).
/// Mutates `m.signature` and `m.app.author_pubkey` in place.
pub fn sign_manifest(m: &mut Manifest, component_bytes: &[u8], key: &SigningKey) {
    let composite = bundle_content_hash(Some(component_bytes), None, None, None);
    let pk = key.verifying_key().to_bytes();
    m.app.author_pubkey = format!("0x{}", hex::encode(pk));
    m.canonicalize();
    let target = signing_target_bytes(m, &composite);
    let sig = key.sign(&target);
    m.signature = Some(Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        value: sig.to_bytes(),
    });
}
