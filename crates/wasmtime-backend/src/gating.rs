//! Capability gating logic for state-apply.
//!
//! Per architecture.md §3.5: state-apply may bind ONLY the
//! deterministic helper set. Per capabilities.md §7.2: the kernel
//! intersects the app's declared set with what the component needs.
//! For state-apply specifically, "what the app declares" must be a
//! subset of the deterministic helper set — declaring any
//! non-deterministic import is a hard error regardless of kernel-major.

use std::collections::BTreeSet;

use myrhiza_backend::BackendError;
use myrhiza_manifest::Manifest;
use myrhiza_manifest::vocabulary::{CapabilityClass, classify};

/// The state-apply ambient set is the deterministic helper set per
/// architecture.md §3.5 (every cell marked "permitted" in the
/// state-apply column).
#[must_use]
pub fn state_apply_ambient_set() -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    s.insert("host.hash".into());
    s.insert("host.verify-signature".into());
    s.insert("host.verify-payload-mac".into());
    s.insert("host.install-key".into());
    s.insert("host.now-hlc-from-event".into());
    s.insert("host.log".into());
    s
}

/// Validate that the manifest's declared imports are a subset of the
/// state-apply ambient set. Any declared host-imports row that is not
/// a `DeterministicHelper` rejects.
///
/// # Errors
///
/// Returns [`BackendError::UnknownImport`] if a declared capability is
/// not in the v1 vocabulary. Returns [`BackendError::UnauthorizedImport`]
/// if a declared capability is in the vocabulary but not part of the
/// state-apply ambient set (i.e. any non-deterministic import).
pub fn validate_state_apply_manifest(m: &Manifest) -> Result<(), BackendError> {
    let ambient = state_apply_ambient_set();

    // Any value in capabilities.host_imports = true that is not a
    // DeterministicHelper is a hard error (state-apply cannot bind it).
    for (cap, &enabled) in &m.capabilities.host_imports {
        if !enabled {
            continue;
        }
        match classify(cap) {
            None => return Err(BackendError::UnknownImport(cap.clone())),
            Some(CapabilityClass::DeterministicHelper) => {
                if !ambient.contains(cap) {
                    return Err(BackendError::UnauthorizedImport {
                        imported: cap.clone(),
                    });
                }
            }
            Some(_) => {
                return Err(BackendError::UnauthorizedImport {
                    imported: cap.clone(),
                });
            }
        }
    }

    // capabilities.deterministic_helpers = true entries are
    // self-documenting only — they must all be in the ambient set.
    for (cap, &enabled) in &m.capabilities.deterministic_helpers {
        if !enabled {
            continue;
        }
        if !ambient.contains(cap) {
            return Err(BackendError::UnknownImport(cap.clone()));
        }
    }

    Ok(())
}

/// Compute the set of host imports that should be bound on the
/// state-apply linker for `manifest`. Returns the manifest-declared
/// subset of the ambient set. Imports outside this set are NOT bound,
/// so a component attempting to import them fails to link.
#[must_use]
pub fn state_apply_bound_imports(m: &Manifest) -> BTreeSet<String> {
    let ambient = state_apply_ambient_set();
    let mut bound = BTreeSet::new();

    // Declared deterministic_helpers entries are merged in.
    for (cap, &enabled) in &m.capabilities.deterministic_helpers {
        if enabled && ambient.contains(cap) {
            bound.insert(cap.clone());
        }
    }
    // Declared host_imports entries (validated DeterministicHelper) too.
    for (cap, &enabled) in &m.capabilities.host_imports {
        if enabled && ambient.contains(cap) {
            bound.insert(cap.clone());
        }
    }

    // host.log is always available to state-apply (output-only sink
    // per determinism.md §5.1; no peer-divergence risk).
    bound.insert("host.log".into());

    bound
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use myrhiza_manifest::vocabulary::CapabilityClass;

    #[test]
    fn state_apply_ambient_is_only_deterministic_helpers() {
        let ambient = state_apply_ambient_set();
        for cap in &ambient {
            let class = myrhiza_manifest::vocabulary::classify(cap)
                .expect("ambient cap must be in vocabulary");
            assert_eq!(
                class,
                CapabilityClass::DeterministicHelper,
                "{cap} must be DeterministicHelper for state-apply ambient"
            );
        }
        assert!(ambient.contains("host.hash"));
        assert!(ambient.contains("host.verify-signature"));
        assert!(ambient.contains("host.now-hlc-from-event"));
        assert!(ambient.contains("host.log"));
    }

    #[test]
    fn validate_state_apply_manifest_rejects_non_deterministic_imports() {
        use myrhiza_manifest::schema::{
            AbiSection, AppSection, AuthorIdentityClass, AuthorPolicy, CapabilitiesSection,
            ComponentsSection, DeterminismSection, DriftDetectionSection, HighValueOps, Manifest,
            ModulesSection, StateDigestFormat,
        };
        let mut m = Manifest {
            app: AppSection {
                name: "x".into(),
                version: "0.1.0".into(),
                description: "x".into(),
                author_pubkey: "wpub-x".into(),
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
                state_apply: Some("c.wasm".into()),
                state_propose: None,
                interaction: None,
                behavior: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: None,
        };
        // Manifest declares non-deterministic broadcast — invalid for
        // a state-apply-only bundle.
        m.capabilities
            .host_imports
            .insert("host.broadcast-submit".into(), true);
        let res = validate_state_apply_manifest(&m);
        assert!(res.is_err(), "non-det import must be rejected");
    }

    #[test]
    fn validate_state_apply_manifest_accepts_helper_set_only() {
        let mut m = sample_state_apply_manifest();
        m.capabilities
            .deterministic_helpers
            .insert("host.hash".into(), true);
        m.capabilities
            .deterministic_helpers
            .insert("host.log".into(), true);
        validate_state_apply_manifest(&m).expect("helper-set-only must validate");
    }

    fn sample_state_apply_manifest() -> myrhiza_manifest::schema::Manifest {
        use myrhiza_manifest::schema::{
            AbiSection, AppSection, AuthorIdentityClass, AuthorPolicy, CapabilitiesSection,
            ComponentsSection, DeterminismSection, DriftDetectionSection, HighValueOps, Manifest,
            ModulesSection, StateDigestFormat,
        };
        Manifest {
            app: AppSection {
                name: "x".into(),
                version: "0.1.0".into(),
                description: "x".into(),
                author_pubkey: "wpub-x".into(),
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
                state_apply: Some("c.wasm".into()),
                state_propose: None,
                interaction: None,
                behavior: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: None,
        }
    }
}
