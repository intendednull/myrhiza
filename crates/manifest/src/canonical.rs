//! Canonical encoding and signing-target framing.
//!
//! Per distribution.md §10.2:
//! - `manifest_canonical_hash` = BLAKE3(`canonical_bincode`(`signed_body`))
//! - `signing_target` is the concatenation of five length-prefixed
//!   fields, in order:
//!   1. `length_prefix`("myrhiza/manifest/v1") — domain separator
//!   2. `length_prefix`(`manifest_canonical_hash`)
//!   3. `length_prefix`(`content_hash`)
//!   4. `length_prefix`(`version_string_bytes`)
//!   5. `length_prefix`(`author_pubkey_bytes`)
//! - The domain separator `"myrhiza/manifest/v1"` is framed as a raw
//!   length-prefixed field (NOT hashed). Framing it eliminates
//!   prefix/suffix collision risk; verifiers MUST reject signatures
//!   computed over a 4-field framing.
//! - Length prefixes are 4-byte little-endian, matching the rest of
//!   the wire format per §10.2.

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

/// Compute the bundle-content-hash per spec §2 Choice G + §3.4.
///
/// Fixed slot order: `state_apply`, `state_propose`, `interaction`, `behavior`.
/// Absent components contribute the 32-byte literal `[0; 32]` (sentinel),
/// NOT `BLAKE3(&[])`. Outer hash input is always 128 bytes.
#[must_use]
pub fn bundle_content_hash(
    state_apply: Option<&[u8]>,
    state_propose: Option<&[u8]>,
    interaction: Option<&[u8]>,
    behavior: Option<&[u8]>,
) -> EventHash {
    let slot = |opt: Option<&[u8]>| -> [u8; 32] {
        match opt {
            Some(bytes) => *EventHash::blake3(bytes).as_bytes(),
            None => [0u8; 32],
        }
    };
    let mut concat = Vec::with_capacity(128);
    concat.extend_from_slice(&slot(state_apply));
    concat.extend_from_slice(&slot(state_propose));
    concat.extend_from_slice(&slot(interaction));
    concat.extend_from_slice(&slot(behavior));
    EventHash::blake3(&concat)
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
    fn bundle_content_hash_for_single_component_differs_from_raw_blake3() {
        // Composite formula wraps each slot in its own BLAKE3 and adds
        // [0;32] sentinels for absent slots — result differs from raw
        // BLAKE3(state_apply_bytes).
        let raw = EventHash::blake3(b"x");
        let composite = bundle_content_hash(Some(b"x"), None, None, None);
        assert_ne!(
            composite, raw,
            "composite hash must differ from single-slot raw BLAKE3"
        );
    }

    #[test]
    fn bundle_content_hash_is_canonical_order_sa_propose_interaction_behavior() {
        // Swapping propose and interaction bytes produces a different
        // hash, confirming the order is enforced by the function signature
        // (not caller-supplied field names). We verify by computing two
        // distinct 4-slot combinations.
        let h1 = bundle_content_hash(Some(b"sa"), Some(b"sp"), Some(b"ix"), None);
        let h2 = bundle_content_hash(Some(b"sa"), Some(b"ix"), Some(b"sp"), None);
        assert_ne!(
            h1, h2,
            "swapping propose and interaction bytes must yield a different hash"
        );
    }

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
    fn signing_target_is_five_length_prefixed_fields() {
        // Walk the signing target byte-by-byte, asserting it is exactly
        // 5 length-prefixed fields per spec §10.2 with no trailing
        // garbage. Field lengths are pinned to the fixture so any
        // structural drift (extra field, missing prefix, wrong field
        // order) trips the test.
        let m = sample_manifest(None);
        let content = EventHash::blake3(b"some-content");
        let bytes = signing_target_bytes(&m, &content);

        let version_len = m.app.version.len();
        let author_len = m.app.author_pubkey.len();
        let expected_lens = [
            (0, DOMAIN_SEP.len()),
            (1, 32), // canonical_hash (BLAKE3 = 32 bytes)
            (2, 32), // content_hash (BLAKE3 = 32 bytes)
            (3, version_len),
            (4, author_len),
        ];

        let mut cursor = 0usize;
        for (idx, expected_len) in expected_lens {
            assert!(
                cursor + 4 <= bytes.len(),
                "field {idx}: signing target truncated before length prefix at offset {cursor}",
            );
            let len = u32::from_le_bytes(
                bytes[cursor..cursor + 4]
                    .try_into()
                    .expect("4 bytes fit a [u8; 4]"),
            ) as usize;
            assert_eq!(len, expected_len, "field {idx} length mismatch");
            cursor += 4 + len;
            assert!(
                cursor <= bytes.len(),
                "field {idx}: declared length {len} runs past end of signing target",
            );
        }
        assert_eq!(cursor, bytes.len(), "trailing garbage in signing target");
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
                state_apply_hash: None,
                state_propose: None,
                state_propose_hash: None,
                interaction: None,
                interaction_hash: None,
                behavior: None,
                behavior_hash: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: sig.map(|v| Signature {
                algorithm: SignatureAlgorithm::Ed25519,
                value: v,
            }),
        }
    }

    #[test]
    fn components_section_hash_fields_roundtrip() {
        use crate::schema::ComponentsSection;
        use myrhiza_types::BlobHash;

        let cs = ComponentsSection {
            state_apply: Some("components/state-apply.wasm".into()),
            state_apply_hash: Some(BlobHash::from_bytes([0xAA; 32])),
            state_propose: None,
            state_propose_hash: None,
            interaction: Some("components/interaction.wasm".into()),
            interaction_hash: Some(BlobHash::from_bytes([0xBB; 32])),
            behavior: None,
            behavior_hash: None,
        };
        let bytes = myrhiza_types::canonical_bincode()
            .serialize(&cs)
            .expect("encode");
        let decoded: ComponentsSection = myrhiza_types::canonical_bincode()
            .deserialize(&bytes)
            .expect("decode");
        assert_eq!(cs, decoded);
    }
}
