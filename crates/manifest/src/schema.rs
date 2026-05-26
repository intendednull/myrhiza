//! Typed manifest struct.
//!
//! Mirrors distribution.md §10.2's TOML schema. The struct is the
//! canonical-encoding target — the signature signs bincode of the
//! struct's signed-body view, NOT the TOML text.

use std::collections::BTreeMap;

use myrhiza_types::BlobHash;
use serde::{Deserialize, Serialize};

/// Top-level manifest. Mirrors distribution.md §10.2.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Manifest {
    /// `[app]` section.
    pub app: AppSection,
    /// `[abi]` section.
    pub abi: AbiSection,
    /// `[capabilities.*]` aggregated.
    pub capabilities: CapabilitiesSection,
    /// `[determinism]` section.
    pub determinism: DeterminismSection,
    /// `[[modules.dep]]` array.
    pub modules: ModulesSection,
    /// `[components]` section.
    pub components: ComponentsSection,
    /// Author-policy is required at parse time per identity.md §6.1.
    /// `default_deny()` produces `Deny`, which forbids
    /// `host.author-event` from non-state-propose profiles.
    pub author_policy: AuthorPolicy,
    /// `Some` only after signing. The serialized signed-body excludes
    /// this field.
    pub signature: Option<Signature>,
}

/// `[app]` section: identity, version, author binding.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AppSection {
    /// App name.
    pub name: String,
    /// `SemVer` string.
    pub version: String,
    /// Short human-readable description.
    pub description: String,
    /// Author public key (Bech32-style string).
    pub author_pubkey: String,
    /// Whether author is third-party or Myrhiza-official.
    pub author_identity_class: AuthorIdentityClass,
}

/// Class of author identity per identity.md §6.1.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AuthorIdentityClass {
    /// Independent author.
    ThirdParty,
    /// Myrhiza-blessed author.
    MyrhizaOfficial,
}

/// `[abi]` section: kernel ABI compatibility.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AbiSection {
    /// Required kernel major version.
    pub kernel_major: u32,
    /// Minimum acceptable kernel minor version.
    pub kernel_minor_min: u32,
    /// Canonical state-digest format.
    pub state_digest_format: StateDigestFormat,
}

/// State-digest canonical encoding format identifier.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum StateDigestFormat {
    /// The only v1 value.
    Bincode13,
}

/// `[capabilities.*]` aggregated.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CapabilitiesSection {
    /// `[capabilities.host-imports]` map (name → bool).
    pub host_imports: BTreeMap<String, bool>,
    /// `[capabilities.ui-surfaces]` map (name → bool).
    pub ui_surfaces: BTreeMap<String, bool>,
    /// `[capabilities.high-value-ops]` typed view.
    pub high_value_ops: HighValueOps,
    /// `[capabilities.deterministic-helpers]` map (name → bool).
    pub deterministic_helpers: BTreeMap<String, bool>,
}

/// Typed view of `[capabilities.high-value-ops]`.
///
/// The bool fields mirror the TOML schema fixed by distribution.md §10.2,
/// so the field-shape is part of the public ABI rather than a code-smell.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct HighValueOps {
    /// Whether `host.clipboard.write` is allowed.
    pub clipboard_write: bool,
    /// Whether `host.file-picker.show` is allowed.
    pub file_picker_show: bool,
    /// Whether `host.navigation.top-level` is allowed.
    pub navigation_top_level: bool,
    /// Whether `host.push.register` is allowed.
    pub push_register: bool,
    /// List of key-handle namespaces app may seal under.
    pub aead_seal: Vec<String>,
    /// List of key-handle namespaces app may open from.
    pub aead_open: Vec<String>,
    /// RFC 6454 exact origins; empty = denied. No glob/wildcard at v1.
    pub http_request: Vec<String>,
}

/// `[determinism]` section.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DeterminismSection {
    /// Whether the state-apply path is permitted to use floats.
    pub allow_floats: bool,
    /// Drift-detection cadence config.
    pub drift_detection: DriftDetectionSection,
}

/// `[determinism.drift-detection]` section.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DriftDetectionSection {
    /// Number of events between drift-detection probes.
    pub interval_events: u32,
}

/// `[[modules.dep]]` array section.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ModulesSection {
    /// One entry per declared module dependency.
    pub dep: Vec<ModuleDep>,
}

/// A single declared module dependency.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ModuleDep {
    /// Logical module name.
    pub name: String,
    /// Content-addressed hash of the module artifact.
    pub content_hash: String,
    /// Author pubkey the kernel must verify the module against.
    pub expected_author: String,
    /// Capabilities the module requires (re-checked against host vocabulary).
    pub required_capabilities: Vec<String>,
}

/// `[components]` section: per-profile artifact paths + content hashes.
///
/// Per B-10 spec §4.1, each profile carries two parallel fields: the
/// disk-relative path (for disk-bundle layout) and the `BlobHash` of
/// the component's raw bytes (load-bearing for iroh-blobs addressing).
/// Disk bundles set `*_hash` to `None`; iroh-publish populates them
/// before signing so the signature commits to the hash claim.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ComponentsSection {
    /// Path to state-apply component, if present.
    pub state_apply: Option<String>,
    /// BLAKE3 hash of the state-apply component's raw bytes (iroh-
    /// blobs addressing). `None` for disk-only bundles. Populated by
    /// publish before signing; cross-checked at install when fetched
    /// over iroh-blobs.
    pub state_apply_hash: Option<BlobHash>,
    /// Path to state-propose component, if present.
    pub state_propose: Option<String>,
    /// BLAKE3 hash of the state-propose component's raw bytes (iroh-
    /// blobs addressing). `None` for disk-only bundles.
    pub state_propose_hash: Option<BlobHash>,
    /// Path to interaction component, if present.
    pub interaction: Option<String>,
    /// BLAKE3 hash of the interaction component's raw bytes (iroh-
    /// blobs addressing). `None` for disk-only bundles.
    pub interaction_hash: Option<BlobHash>,
    /// Path to behavior component, if present.
    pub behavior: Option<String>,
    /// BLAKE3 hash of the behavior component's raw bytes (iroh-blobs
    /// addressing). `None` for disk-only bundles.
    pub behavior_hash: Option<BlobHash>,
}

/// Author-policy per identity.md §6.1. v1 default is `Deny`.
/// `Permissive` is opt-in; `Map` is per-profile-per-variant.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AuthorPolicy {
    /// Forbid `host.author-event` from any profile.
    Deny,
    /// Permit `host.author-event` from any profile that imports it.
    Permissive,
    /// Per-profile allow-list of event variant names.
    Map {
        /// Variants `state-propose` may author.
        state_propose: Vec<String>,
        /// Variants `behavior` may author.
        behavior: Vec<String>,
    },
}

impl AuthorPolicy {
    /// Construct the v1 default policy: `Deny`.
    #[must_use]
    pub fn default_deny() -> Self {
        Self::Deny
    }
}

/// Detached signature over the manifest signed-body.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Signature {
    /// Algorithm identifier — only `Ed25519` is permitted at v1.
    pub algorithm: SignatureAlgorithm,
    /// Raw 64-byte Ed25519 signature.
    #[serde(with = "crate::schema::serde_sig_bytes")]
    pub value: [u8; 64],
}

/// Permitted signature algorithm.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    /// The only v1 value. Cremers ETK 2025 forbids ECDSA on the
    /// kernel surface; manifest cannot declare alternative algorithms.
    Ed25519,
}

mod serde_sig_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(b: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        serde_bytes::Bytes::new(b).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v: serde_bytes::ByteBuf = serde_bytes::ByteBuf::deserialize(d)?;
        let arr: [u8; 64] = v
            .as_ref()
            .try_into()
            .map_err(|_| serde::de::Error::invalid_length(v.len(), &"64 bytes"))?;
        Ok(arr)
    }
}

impl Manifest {
    /// Apply canonical-form normalizations per distribution.md §10.2:
    /// - `modules.dep` sorted by `content_hash` ascending.
    /// - String fields NFC-normalized.
    pub fn canonicalize(&mut self) {
        use unicode_normalization::UnicodeNormalization;

        self.modules
            .dep
            .sort_by(|a, b| a.content_hash.cmp(&b.content_hash));

        // NFC normalize every string field at the schema boundary.
        let nfc = |s: &str| s.nfc().collect::<String>();
        self.app.name = nfc(&self.app.name);
        self.app.version = nfc(&self.app.version);
        self.app.description = nfc(&self.app.description);
        self.app.author_pubkey = nfc(&self.app.author_pubkey);
        for dep in &mut self.modules.dep {
            dep.name = nfc(&dep.name);
            dep.content_hash = nfc(&dep.content_hash);
            dep.expected_author = nfc(&dep.expected_author);
            for cap in &mut dep.required_capabilities {
                *cap = nfc(cap);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use bincode::Options;
    use myrhiza_types::canonical_bincode;

    fn minimal() -> Manifest {
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

    #[test]
    fn manifest_round_trips_via_canonical_bincode() {
        let m = minimal();
        let bytes = canonical_bincode().serialize(&m).expect("encode");
        let decoded: Manifest = canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(m, decoded);
    }

    #[test]
    fn author_policy_default_is_deny() {
        let p = AuthorPolicy::default_deny();
        assert!(matches!(p, AuthorPolicy::Deny));
    }

    #[test]
    fn modules_dep_canonical_sort_by_content_hash() {
        let mut m = minimal();
        m.modules.dep = vec![
            ModuleDep {
                name: "z-mod".into(),
                content_hash: "blake3:fff".into(),
                expected_author: "wpub-myrhiza1xyz".into(),
                required_capabilities: vec![],
            },
            ModuleDep {
                name: "a-mod".into(),
                content_hash: "blake3:aaa".into(),
                expected_author: "wpub-myrhiza1xyz".into(),
                required_capabilities: vec![],
            },
        ];
        m.canonicalize();
        assert_eq!(m.modules.dep[0].content_hash, "blake3:aaa");
        assert_eq!(m.modules.dep[1].content_hash, "blake3:fff");
    }
}
