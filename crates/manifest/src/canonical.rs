//! Canonical encoding and signing-target framing.
//!
//! Per distribution.md §10.2:
//! - `manifest_canonical_hash` = BLAKE3(`canonical_bincode`(`signed_body`))
//! - `signing_target` = `length_prefix_concat`(
//!   BLAKE3("myrhiza/manifest/v1"),
//!   `manifest_canonical_hash`,
//!   `content_hash`,
//!   `version_string_bytes`,
//!   `author_pubkey_bytes`)
//! - Length prefixes are 4-byte little-endian per the section text.

use bincode::Options;
use myrhiza_types::{EventHash, canonical_bincode};
use serde::Serialize;

use crate::schema::Manifest;

/// Domain-separator string per §10.2.
pub const DOMAIN_SEP: &[u8] = b"myrhiza/manifest/v1";

/// Encode the manifest's signed body (everything except `signature`)
/// via `canonical_bincode`.
///
/// # Panics
/// Infallible in practice: the inner `serialize` call only fails if
/// the bincode `Options` fail — they don't, for the fixed
/// `SignedBody` shape — or if the underlying `Vec` allocator fails,
/// which we treat as an unrecoverable abort. The narrow `expect`
/// allow is justified because every failure is a kernel bug, not a
/// user-facing condition.
#[allow(clippy::expect_used)]
#[must_use]
pub fn signed_body_bytes(m: &Manifest) -> Vec<u8> {
    #[derive(Serialize)]
    struct SignedBody<'a> {
        app: &'a crate::schema::AppSection,
        abi: &'a crate::schema::AbiSection,
        capabilities: &'a crate::schema::CapabilitiesSection,
        determinism: &'a crate::schema::DeterminismSection,
        modules: &'a crate::schema::ModulesSection,
        components: &'a crate::schema::ComponentsSection,
        author_policy: &'a crate::schema::AuthorPolicy,
    }

    let body = SignedBody {
        app: &m.app,
        abi: &m.abi,
        capabilities: &m.capabilities,
        determinism: &m.determinism,
        modules: &m.modules,
        components: &m.components,
        author_policy: &m.author_policy,
    };

    canonical_bincode()
        .serialize(&body)
        .expect("canonical bincode of SignedBody never fails")
}

/// BLAKE3 of the canonical signed body.
#[must_use]
pub fn manifest_canonical_hash(m: &Manifest) -> EventHash {
    EventHash::blake3(&signed_body_bytes(m))
}

/// `length_prefix_concat(fields)` returns
/// `for each f: u32_le(f.len()) || f`.
///
/// # Panics
/// Panics only if a single field exceeds `u32::MAX` bytes (~4 GiB).
/// Manifest schema bounds (per distribution.md §10.2) make this
/// structurally unreachable; the narrow `expect` allow is justified.
#[allow(clippy::expect_used)]
#[must_use]
pub fn length_prefix_concat(fields: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(fields.iter().map(|f| 4 + f.len()).sum());
    for f in fields {
        let len = u32::try_from(f.len()).expect("manifest field length > u32::MAX");
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(f);
    }
    out
}

/// Compute the byte string the author signs.
///
/// Layout per §10.2: `length_prefix_concat` over the five fields
/// (`DOMAIN_SEP`, `manifest_canonical_hash`, `content_hash`,
/// `version_string_bytes`, `author_pubkey_bytes`).
#[must_use]
pub fn signing_target_bytes(m: &Manifest, content_hash: &EventHash) -> Vec<u8> {
    let canonical_hash = manifest_canonical_hash(m);
    let version_bytes = m.app.version.as_bytes();
    let author_bytes = m.app.author_pubkey.as_bytes();
    length_prefix_concat(&[
        DOMAIN_SEP,
        canonical_hash.as_bytes(),
        content_hash.as_bytes(),
        version_bytes,
        author_bytes,
    ])
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use myrhiza_types::EventHash;

    #[test]
    fn signed_body_excludes_signature() {
        let m_no_sig = sample_manifest(None);
        let m_with_sig = sample_manifest(Some([0xFF; 64]));
        let h1 = manifest_canonical_hash(&m_no_sig);
        let h2 = manifest_canonical_hash(&m_with_sig);
        assert_eq!(h1, h2, "canonical hash must not depend on signature bytes");
    }

    #[test]
    fn length_prefix_layout() {
        // Single field with bytes [1, 2, 3] should produce
        // [3, 0, 0, 0, 1, 2, 3].
        let out = length_prefix_concat(&[&[1, 2, 3]]);
        assert_eq!(out, vec![3, 0, 0, 0, 1, 2, 3]);
    }

    #[test]
    fn signing_target_layout() {
        let m = sample_manifest(None);
        let content = EventHash::blake3(b"some-content");
        let target = signing_target_bytes(&m, &content);
        // 4 length prefixes for 4 fields.
        assert!(target.len() >= 16);
    }

    fn sample_manifest(sig: Option<[u8; 64]>) -> crate::schema::Manifest {
        use crate::schema::{
            AbiSection, AppSection, AuthorIdentityClass, AuthorPolicy, CapabilitiesSection,
            ComponentsSection, DeterminismSection, DriftDetectionSection, HighValueOps, Manifest,
            ModulesSection, Signature, SignatureAlgorithm, StateDigestFormat,
        };
        Manifest {
            app: AppSection {
                name: "x".into(),
                version: "0.1.0".into(),
                description: "x".into(),
                author_pubkey: "wpub-author1xxx".into(),
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
            modules: ModulesSection { dep: vec![] },
            components: ComponentsSection {
                state_apply: Some("components/state-apply.wasm".into()),
                state_propose: None,
                interaction: None,
                behavior: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: sig.map(|v| Signature {
                algorithm: SignatureAlgorithm::Ed25519,
                value: v,
            }),
        }
    }
}
