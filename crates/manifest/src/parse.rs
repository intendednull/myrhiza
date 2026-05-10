//! Manifest TOML parser.
//!
//! - Pinned to `toml_edit 0.22.x` (per distribution.md §10.2).
//! - Rejects unknown capability strings (per §10.2).
//! - Rejects non-Ed25519 signature algorithms (Cremers ETK 2025
//!   structural enforcement; per identity.md §6.1).
//! - Calls `Manifest::canonicalize()` before returning so callers
//!   always see canonical-form output.

use std::collections::BTreeMap;

use thiserror::Error;
use toml_edit::DocumentMut;

use crate::schema::{
    AbiSection, AppSection, AuthorIdentityClass, AuthorPolicy, CapabilitiesSection,
    ComponentsSection, DeterminismSection, DriftDetectionSection, HighValueOps, Manifest,
    ModuleDep, ModulesSection, Signature, SignatureAlgorithm, StateDigestFormat,
};
use crate::vocabulary::known_capability;

/// Errors returned by [`parse_manifest`].
#[derive(Debug, Error)]
pub enum ParseError {
    /// Underlying TOML parse error.
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml_edit::TomlError),
    /// A required field was absent.
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    /// A field's value was the wrong shape or out of range.
    #[error("invalid value at {field}: {detail}")]
    InvalidValue {
        /// Logical field path.
        field: &'static str,
        /// Human-readable detail.
        detail: String,
    },
    /// A capability string was not in the v1 vocabulary.
    #[error("unknown capability: {0}")]
    UnknownCapability(String),
    /// The signature block declared a non-Ed25519 algorithm.
    #[error("Cremers ETK 2025: only ed25519 is permitted, got {0}")]
    NonEd25519Signature(String),
    /// Hex decode failed for a hex-encoded field.
    #[error("invalid hex: {0}")]
    Hex(#[from] hex::FromHexError),
}

/// Parse a manifest TOML string and validate the result.
///
/// # Errors
/// Returns [`ParseError`] on any TOML, schema, or vocabulary violation.
pub fn parse_manifest(input: &str) -> Result<Manifest, ParseError> {
    let doc: DocumentMut = input.parse()?;

    let app = parse_app(&doc)?;
    let abi = parse_abi(&doc)?;
    let capabilities = parse_capabilities(&doc)?;
    let determinism = parse_determinism(&doc)?;
    let modules = parse_modules(&doc)?;
    let components = parse_components(&doc)?;
    let author_policy = parse_author_policy(&doc)?;
    let signature = parse_signature(&doc)?;

    let mut m = Manifest {
        app,
        abi,
        capabilities,
        determinism,
        modules,
        components,
        author_policy,
        signature,
    };
    m.canonicalize();
    Ok(m)
}

/// Look up a top-level table by name. `table` is `&'static str` so the
/// missing-field diagnostic can embed the literal directly into
/// `ParseError::MissingField` without leaking a heap allocation. Every
/// caller passes a string literal — the manifest schema's table set is
/// closed.
fn require<'a>(
    doc: &'a DocumentMut,
    table: &'static str,
) -> Result<&'a toml_edit::Table, ParseError> {
    doc.get(table)
        .and_then(|i| i.as_table())
        .ok_or(ParseError::MissingField(table))
}

fn require_str<'a>(t: &'a toml_edit::Table, key: &'static str) -> Result<&'a str, ParseError> {
    t.get(key)
        .and_then(|i| i.as_str())
        .ok_or(ParseError::MissingField(key))
}

fn require_int(t: &toml_edit::Table, key: &'static str) -> Result<i64, ParseError> {
    t.get(key)
        .and_then(toml_edit::Item::as_integer)
        .ok_or(ParseError::MissingField(key))
}

fn parse_app(doc: &DocumentMut) -> Result<AppSection, ParseError> {
    let t = require(doc, "app")?;
    let class = match require_str(t, "author-identity-class")? {
        "third-party" => AuthorIdentityClass::ThirdParty,
        "myrhiza-official" => AuthorIdentityClass::MyrhizaOfficial,
        other => {
            return Err(ParseError::InvalidValue {
                field: "app.author-identity-class",
                detail: format!("unknown class {other}"),
            });
        }
    };
    Ok(AppSection {
        name: require_str(t, "name")?.into(),
        version: require_str(t, "version")?.into(),
        description: require_str(t, "description")?.into(),
        author_pubkey: require_str(t, "author-pubkey")?.into(),
        author_identity_class: class,
    })
}

fn parse_abi(doc: &DocumentMut) -> Result<AbiSection, ParseError> {
    let t = require(doc, "abi")?;
    let format = match require_str(t, "state-digest-format")? {
        "bincode-1.3" => StateDigestFormat::Bincode13,
        other => {
            return Err(ParseError::InvalidValue {
                field: "abi.state-digest-format",
                detail: format!("unknown format {other}"),
            });
        }
    };
    Ok(AbiSection {
        kernel_major: u32::try_from(require_int(t, "kernel-major")?).map_err(|_| {
            ParseError::InvalidValue {
                field: "abi.kernel-major",
                detail: "out of u32 range".into(),
            }
        })?,
        kernel_minor_min: u32::try_from(require_int(t, "kernel-minor-min")?).map_err(|_| {
            ParseError::InvalidValue {
                field: "abi.kernel-minor-min",
                detail: "out of u32 range".into(),
            }
        })?,
        state_digest_format: format,
    })
}

fn parse_capabilities(doc: &DocumentMut) -> Result<CapabilitiesSection, ParseError> {
    let mut host_imports = BTreeMap::new();
    let mut ui_surfaces = BTreeMap::new();
    let mut deterministic_helpers = BTreeMap::new();

    if let Some(t) = doc.get("capabilities").and_then(|i| i.as_table()) {
        if let Some(hi) = t.get("host-imports").and_then(|i| i.as_table()) {
            for (k, v) in hi {
                if !known_capability(k) {
                    return Err(ParseError::UnknownCapability(k.into()));
                }
                let b = v.as_bool().ok_or(ParseError::InvalidValue {
                    field: "capabilities.host-imports.<key>",
                    detail: format!("expected bool for {k}"),
                })?;
                host_imports.insert(k.into(), b);
            }
        }
        if let Some(ui) = t.get("ui-surfaces").and_then(|i| i.as_table()) {
            for (k, v) in ui {
                if !known_capability(k) {
                    return Err(ParseError::UnknownCapability(k.into()));
                }
                let b = v.as_bool().ok_or(ParseError::InvalidValue {
                    field: "capabilities.ui-surfaces.<key>",
                    detail: format!("expected bool for {k}"),
                })?;
                ui_surfaces.insert(k.into(), b);
            }
        }
        if let Some(dh) = t.get("deterministic-helpers").and_then(|i| i.as_table()) {
            for (k, v) in dh {
                if !known_capability(k) {
                    return Err(ParseError::UnknownCapability(k.into()));
                }
                let b = v.as_bool().ok_or(ParseError::InvalidValue {
                    field: "capabilities.deterministic-helpers.<key>",
                    detail: format!("expected bool for {k}"),
                })?;
                deterministic_helpers.insert(k.into(), b);
            }
        }
    }

    let high_value_ops = parse_hvo(doc)?;

    Ok(CapabilitiesSection {
        host_imports,
        ui_surfaces,
        high_value_ops,
        deterministic_helpers,
    })
}

fn parse_hvo(doc: &DocumentMut) -> Result<HighValueOps, ParseError> {
    let mut hvo = HighValueOps::default();
    let Some(t) = doc
        .get("capabilities")
        .and_then(|i| i.as_table())
        .and_then(|t| t.get("high-value-ops"))
        .and_then(|i| i.as_table())
    else {
        return Ok(hvo);
    };
    for (k, v) in t {
        if !known_capability(k) {
            return Err(ParseError::UnknownCapability(k.into()));
        }
        // Strict bool parse: silently coercing `"yes"` / `1` to `false`
        // would let a manifest typo flip a high-value-op declaration
        // closed without a parse error. Per distribution.md §10.2, HVO
        // entries are load-bearing for user-prompt decisions; reject
        // non-bool values rather than defaulting.
        let require_bool = |v: &toml_edit::Item, cap: &str| -> Result<bool, ParseError> {
            v.as_bool().ok_or_else(|| ParseError::InvalidValue {
                field: "capabilities.high-value-ops",
                detail: format!("expected bool for {cap}"),
            })
        };
        match k {
            "host.clipboard.write" => hvo.clipboard_write = require_bool(v, k)?,
            "host.file-picker.show" => hvo.file_picker_show = require_bool(v, k)?,
            "host.navigation.top-level" => hvo.navigation_top_level = require_bool(v, k)?,
            "host.push.register" => hvo.push_register = require_bool(v, k)?,
            "host.aead-seal" => hvo.aead_seal = parse_str_array(v)?,
            "host.aead-open" => hvo.aead_open = parse_str_array(v)?,
            "host.http.request" => hvo.http_request = parse_str_array(v)?,
            other => {
                return Err(ParseError::InvalidValue {
                    field: "capabilities.high-value-ops",
                    detail: format!("unsupported field {other}"),
                });
            }
        }
    }
    Ok(hvo)
}

fn parse_str_array(item: &toml_edit::Item) -> Result<Vec<String>, ParseError> {
    let arr = item.as_array().ok_or(ParseError::InvalidValue {
        field: "high-value-ops.<list>",
        detail: "expected array".into(),
    })?;
    arr.iter()
        .map(|v| {
            v.as_str()
                .map(String::from)
                .ok_or(ParseError::InvalidValue {
                    field: "high-value-ops.<list>",
                    detail: "expected string element".into(),
                })
        })
        .collect()
}

fn parse_determinism(doc: &DocumentMut) -> Result<DeterminismSection, ParseError> {
    let t = require(doc, "determinism")?;
    let allow_floats = t
        .get("allow-floats")
        .and_then(toml_edit::Item::as_bool)
        .unwrap_or(false);
    let drift = t
        .get("drift-detection")
        .and_then(|i| i.as_table())
        .ok_or(ParseError::MissingField("determinism.drift-detection"))?;
    let interval_events = u32::try_from(require_int(drift, "interval-events")?).map_err(|_| {
        ParseError::InvalidValue {
            field: "determinism.drift-detection.interval-events",
            detail: "out of u32 range".into(),
        }
    })?;
    Ok(DeterminismSection {
        allow_floats,
        drift_detection: DriftDetectionSection { interval_events },
    })
}

fn parse_modules(doc: &DocumentMut) -> Result<ModulesSection, ParseError> {
    let mut deps = Vec::new();
    if let Some(arr) = doc
        .get("modules")
        .and_then(|i| i.as_table())
        .and_then(|t| t.get("dep"))
        .and_then(|i| i.as_array_of_tables())
    {
        for tbl in arr {
            let mut required = Vec::new();
            if let Some(rc) = tbl.get("required-capabilities").and_then(|i| i.as_array()) {
                for v in rc {
                    let s = v.as_str().ok_or(ParseError::InvalidValue {
                        field: "modules.dep.required-capabilities",
                        detail: "expected string element".into(),
                    })?;
                    if !known_capability(s) {
                        return Err(ParseError::UnknownCapability(s.into()));
                    }
                    required.push(s.into());
                }
            }
            deps.push(ModuleDep {
                name: require_str(tbl, "name")?.into(),
                content_hash: require_str(tbl, "content-hash")?.into(),
                expected_author: require_str(tbl, "expected-author")?.into(),
                required_capabilities: required,
            });
        }
    }
    Ok(ModulesSection { dep: deps })
}

fn parse_components(doc: &DocumentMut) -> Result<ComponentsSection, ParseError> {
    let t = require(doc, "components")?;
    let opt = |k: &str| t.get(k).and_then(|i| i.as_str()).map(String::from);
    Ok(ComponentsSection {
        state_apply: opt("state-apply"),
        state_propose: opt("state-propose"),
        interaction: opt("interaction"),
        behavior: opt("behavior"),
    })
}

fn parse_author_policy(doc: &DocumentMut) -> Result<AuthorPolicy, ParseError> {
    let Some(t) = doc.get("author-policy").and_then(|i| i.as_table()) else {
        return Ok(AuthorPolicy::default_deny());
    };
    if let Some(p) = t.get("policy").and_then(|i| i.as_str()) {
        return match p {
            "permissive" => Ok(AuthorPolicy::Permissive),
            "deny" => Ok(AuthorPolicy::Deny),
            other => Err(ParseError::InvalidValue {
                field: "author-policy.policy",
                detail: format!("unknown {other}"),
            }),
        };
    }
    let parse_list = |key: &str| -> Result<Vec<String>, ParseError> {
        let Some(arr) = t.get(key).and_then(|i| i.as_array()) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for v in arr {
            out.push(
                v.as_str()
                    .ok_or(ParseError::InvalidValue {
                        field: "author-policy.<list>",
                        detail: "expected string element".into(),
                    })?
                    .into(),
            );
        }
        Ok(out)
    };
    Ok(AuthorPolicy::Map {
        state_propose: parse_list("state-propose")?,
        behavior: parse_list("behavior")?,
    })
}

fn parse_signature(doc: &DocumentMut) -> Result<Option<Signature>, ParseError> {
    let Some(t) = doc.get("signature").and_then(|i| i.as_table()) else {
        return Ok(None);
    };
    let alg = require_str(t, "algorithm")?;
    if alg != "ed25519" {
        return Err(ParseError::NonEd25519Signature(alg.into()));
    }
    let raw = require_str(t, "value")?;
    let stripped = raw.strip_prefix("0x").unwrap_or(raw);
    let bytes = hex::decode(stripped)?;
    let arr: [u8; 64] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| ParseError::InvalidValue {
            field: "signature.value",
            detail: "expected 64 bytes".into(),
        })?;
    Ok(Some(Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        value: arr,
    }))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn parse_counter_fixture() {
        let toml = include_str!("../tests/fixtures/counter-manifest.toml");
        let m = parse_manifest(toml).expect("parse counter fixture");
        assert_eq!(m.app.name, "counter");
        assert_eq!(m.abi.kernel_major, 1);
        assert!(m.capabilities.host_imports.contains_key("host.broadcast"));
        // canonicalize should have run during parse.
        let mut prev = String::new();
        for dep in &m.modules.dep {
            assert!(
                dep.content_hash >= prev,
                "modules.dep must be sorted by content_hash"
            );
            prev = dep.content_hash.clone();
        }
    }

    #[test]
    fn parse_rejects_unknown_capability() {
        let toml = r#"
[app]
name = "x"
version = "0.1.0"
description = "x"
author-pubkey = "wpub-author1xxx"
author-identity-class = "third-party"

[abi]
kernel-major = 1
kernel-minor-min = 0
state-digest-format = "bincode-1.3"

[capabilities.host-imports]
"host.invented-by-app" = true

[capabilities.high-value-ops]

[determinism]
allow-floats = false

[determinism.drift-detection]
interval-events = 1024

[components]
state-apply = "components/state-apply.wasm"
"#;
        let err = parse_manifest(toml).expect_err("must reject unknown capability");
        assert!(
            err.to_string().contains("host.invented-by-app"),
            "error must name the offending capability: {err}"
        );
    }

    #[test]
    fn parse_rejects_non_bool_high_value_op() {
        // `clipboard.write = "yes"` previously silently coerced to
        // `false` via `.as_bool().unwrap_or(false)`. After the strict-
        // parse fix it must surface as `InvalidValue` so a manifest
        // typo cannot flip a high-value-op declaration closed without
        // a parse error.
        let toml = r#"
[app]
name = "x"
version = "0.1.0"
description = "x"
author-pubkey = "wpub-author1xxx"
author-identity-class = "third-party"

[abi]
kernel-major = 1
kernel-minor-min = 0
state-digest-format = "bincode-1.3"

[capabilities.host-imports]

[capabilities.high-value-ops]
"host.clipboard.write" = "yes"

[determinism]
allow-floats = false

[determinism.drift-detection]
interval-events = 1024

[components]
state-apply = "components/state-apply.wasm"
"#;
        let err = parse_manifest(toml).expect_err("non-bool HVO must surface InvalidValue");
        match err {
            ParseError::InvalidValue { field, detail } => {
                assert_eq!(field, "capabilities.high-value-ops");
                assert!(
                    detail.contains("host.clipboard.write"),
                    "detail must name offending key: {detail}"
                );
            }
            other => panic!("expected InvalidValue, got {other:?}"),
        }
    }

    #[test]
    fn parse_rejects_non_ed25519_signature_algorithm() {
        let toml = r#"
[app]
name = "x"
version = "0.1.0"
description = "x"
author-pubkey = "wpub-author1xxx"
author-identity-class = "third-party"

[abi]
kernel-major = 1
kernel-minor-min = 0
state-digest-format = "bincode-1.3"

[capabilities.host-imports]
[capabilities.high-value-ops]

[determinism]
allow-floats = false

[determinism.drift-detection]
interval-events = 1024

[components]
state-apply = "components/state-apply.wasm"

[signature]
algorithm = "ecdsa"
value = "0x00"
"#;
        let err = parse_manifest(toml).expect_err("ECDSA must be rejected");
        assert!(err.to_string().to_lowercase().contains("ed25519"));
    }
}
