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

/// The state-apply ambient set is the v1 subset of the deterministic
/// helper set per architecture.md §3.5.
///
/// `host.install-key` and `host.verify-payload-mac` are vocabulary-
/// registered authored capabilities (still `DeterministicHelper`
/// classified) but are intentionally NOT in the v1 ambient set: their
/// signatures take a `key-handle` resource whose infrastructure lands
/// in plan B (per determinism.md §5.1 v1 deferral). Manifests that
/// declare them for state-apply are rejected at install with
/// [`BackendError::DeferredToPlanB`].
#[must_use]
pub fn state_apply_ambient_set() -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    s.insert("host.hash".into());
    s.insert("host.verify-signature".into());
    s.insert("host.now-hlc-from-event".into());
    s.insert("host.log".into());
    s
}

/// Capabilities deferred to plan B — registered in the vocabulary as
/// `DeterministicHelper` but not bindable in v1 state-apply. Declared
/// here so `validate_state_apply_manifest` can short-circuit before
/// the class check rejects them as "non-deterministic".
const DEFERRED_TO_PLAN_B: &[&str] = &["host.install-key", "host.verify-payload-mac"];

/// Validate that the manifest's declared imports are a subset of the
/// state-apply ambient set. Any declared host-imports row that is not
/// a `DeterministicHelper` rejects.
///
/// `host.install-key` and `host.verify-payload-mac` are vocabulary-
/// registered (still `DeterministicHelper`) but deferred to plan B;
/// declaring either short-circuits to [`BackendError::DeferredToPlanB`]
/// before the generic class check fires (so users see the specific
/// "deferred" message rather than a misleading "unauthorized non-det
/// import" — both are `DeterministicHelper`-classified).
///
/// # Errors
///
/// Returns [`BackendError::DeferredToPlanB`] if a declared capability
/// is registered but not bindable in v1 state-apply. Returns
/// [`BackendError::UnknownImport`] if a declared capability is not in
/// the v1 vocabulary. Returns [`BackendError::UnauthorizedImport`] if a
/// declared capability is in the vocabulary but not part of the
/// state-apply ambient set (i.e. any non-deterministic import).
pub fn validate_state_apply_manifest(m: &Manifest) -> Result<(), BackendError> {
    let ambient = state_apply_ambient_set();

    // Any value in capabilities.host_imports = true that is not a
    // DeterministicHelper is a hard error (state-apply cannot bind it).
    for (cap, &enabled) in &m.capabilities.host_imports {
        if !enabled {
            continue;
        }
        // Defer-check fires first so install-key / verify-payload-mac
        // get the specific "deferred" verdict even though the class
        // check below would otherwise treat them as accepted helpers.
        if DEFERRED_TO_PLAN_B.contains(&cap.as_str()) {
            return Err(BackendError::DeferredToPlanB(cap.clone()));
        }
        match classify(cap) {
            None => return Err(BackendError::UnknownImport(cap.clone())),
            Some(CapabilityClass::DeterministicHelper) => {
                // The deterministic-helper class minus the deferred
                // entries is exactly the state-apply ambient set (see
                // `state_apply_ambient_set`), so no further membership
                // check is needed.
            }
            Some(_) => {
                return Err(BackendError::UnauthorizedImport(cap.clone()));
            }
        }
    }

    // capabilities.deterministic_helpers = true entries are
    // self-documenting only — they must all be in the ambient set.
    // Deferred entries are checked first for the same reason as above.
    for (cap, &enabled) in &m.capabilities.deterministic_helpers {
        if !enabled {
            continue;
        }
        if DEFERRED_TO_PLAN_B.contains(&cap.as_str()) {
            return Err(BackendError::DeferredToPlanB(cap.clone()));
        }
        if !ambient.contains(cap) {
            return Err(BackendError::UnknownImport(cap.clone()));
        }
    }

    Ok(())
}

/// Register host functions on the Wasmtime linker for the imports
/// listed in `bound_imports`. Imports not listed are not registered;
/// a state-apply component attempting to import them will fail to
/// link (this is the load-bearing gating mechanic for plan A's
/// acceptance criterion #5 per capabilities.md §7.2).
///
/// `host.install-key` and `host.verify-payload-mac` are intentionally
/// not bound here — they take a `key-handle` resource whose
/// infrastructure lands in plan B. They are also rejected up-front by
/// [`validate_state_apply_manifest`] with [`BackendError::DeferredToPlanB`]
/// when declared in a manifest, so a state-apply component that imports
/// them never reaches link time on the v1 happy path.
///
/// # Errors
///
/// Returns [`BackendError::Instantiation`] if the linker rejects an
/// `instance(...)` lookup or `func_wrap(...)` registration. This
/// surface should not fail in practice; it would indicate a bindgen
/// / WIT mismatch at workspace build time.
pub fn wire_state_apply_linker(
    linker: &mut wasmtime::component::Linker<crate::engine::HostState>,
    bound_imports: &BTreeSet<String>,
) -> Result<(), BackendError> {
    use crate::engine::myrhiza::kernel::types::{Hlc as WitHlc, LogLevel as WitLogLevel};
    use crate::helpers::{
        LogLevel, host_hash_impl, host_now_hlc_from_event_impl, host_verify_signature_impl,
    };

    // Per the bindgen-generated `add_to_linker` for the
    // `host-deterministic` interface the WIT instance name is
    // versioned: `myrhiza:kernel/host-deterministic@1.0.0`.
    let mut iface = linker
        .instance("myrhiza:kernel/host-deterministic@1.0.0")
        .map_err(|e| BackendError::Instantiation(format!("linker instance: {e}")))?;

    if bound_imports.contains("host.hash") {
        iface
            .func_wrap(
                "hash",
                |_caller: wasmtime::StoreContextMut<'_, crate::engine::HostState>,
                 (bytes,): (Vec<u8>,)|
                 -> wasmtime::Result<(Vec<u8>,)> { Ok((host_hash_impl(&bytes),)) },
            )
            .map_err(|e| BackendError::Instantiation(format!("wire host.hash: {e}")))?;
    }

    if bound_imports.contains("host.verify-signature") {
        iface
            .func_wrap(
                "verify-signature",
                |_caller: wasmtime::StoreContextMut<'_, crate::engine::HostState>,
                 (pk, msg, sig): (Vec<u8>, Vec<u8>, Vec<u8>)|
                 -> wasmtime::Result<(bool,)> {
                    Ok((host_verify_signature_impl(&pk, &msg, &sig),))
                },
            )
            .map_err(|e| BackendError::Instantiation(format!("wire host.verify-signature: {e}")))?;
    }

    if bound_imports.contains("host.now-hlc-from-event") {
        iface
            .func_wrap(
                "now-hlc-from-event",
                |_caller: wasmtime::StoreContextMut<'_, crate::engine::HostState>,
                 (event_bytes,): (Vec<u8>,)|
                 -> wasmtime::Result<(WitHlc,)> {
                    let hlc = host_now_hlc_from_event_impl(&event_bytes).ok_or_else(|| {
                        wasmtime::Error::msg("now-hlc-from-event: invalid canonical event bytes")
                    })?;
                    Ok((WitHlc {
                        wall_ms: hlc.wall_ms,
                        logical: hlc.logical,
                    },))
                },
            )
            .map_err(|e| {
                BackendError::Instantiation(format!("wire host.now-hlc-from-event: {e}"))
            })?;
    }

    // host.log is always available to state-apply per
    // determinism.md §5.1 (output-only sink; not part of state-digest).
    iface
        .func_wrap(
            "log",
            |caller: wasmtime::StoreContextMut<'_, crate::engine::HostState>,
             (level, msg): (WitLogLevel, String)|
             -> wasmtime::Result<()> {
                let level = match level {
                    WitLogLevel::Trace => LogLevel::Trace,
                    WitLogLevel::Debug => LogLevel::Debug,
                    WitLogLevel::Info => LogLevel::Info,
                    WitLogLevel::Warn => LogLevel::Warn,
                    WitLogLevel::Error => LogLevel::Error,
                };
                caller.data().log_sink.record(level, msg);
                Ok(())
            },
        )
        .map_err(|e| BackendError::Instantiation(format!("wire host.log: {e}")))?;

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
            .insert("host.broadcast".into(), true);
        let res = validate_state_apply_manifest(&m);
        assert!(
            matches!(res, Err(BackendError::UnauthorizedImport(_))),
            "non-det import must be rejected as unauthorized: {res:?}"
        );
    }

    #[test]
    fn install_key_in_manifest_returns_deferred_to_plan_b() {
        let mut m = sample_state_apply_manifest();
        m.capabilities
            .host_imports
            .insert("host.install-key".into(), true);
        let res = validate_state_apply_manifest(&m);
        match res {
            Err(BackendError::DeferredToPlanB(name)) => {
                assert_eq!(name, "host.install-key");
            }
            other => panic!("expected DeferredToPlanB(\"host.install-key\"), got {other:?}"),
        }
    }

    #[test]
    fn verify_payload_mac_in_manifest_returns_deferred_to_plan_b() {
        let mut m = sample_state_apply_manifest();
        m.capabilities
            .host_imports
            .insert("host.verify-payload-mac".into(), true);
        let res = validate_state_apply_manifest(&m);
        match res {
            Err(BackendError::DeferredToPlanB(name)) => {
                assert_eq!(name, "host.verify-payload-mac");
            }
            other => panic!("expected DeferredToPlanB(\"host.verify-payload-mac\"), got {other:?}"),
        }
    }

    #[test]
    fn deferred_caps_also_rejected_when_declared_in_deterministic_helpers() {
        // The defer-check must fire on either declaration site
        // (host_imports or deterministic_helpers) — they are still
        // `DeterministicHelper`-classified in the vocabulary so a user
        // could put them anywhere.
        let mut m = sample_state_apply_manifest();
        m.capabilities
            .deterministic_helpers
            .insert("host.install-key".into(), true);
        let res = validate_state_apply_manifest(&m);
        assert!(
            matches!(&res, Err(BackendError::DeferredToPlanB(name)) if name == "host.install-key"),
            "deterministic_helpers declaration must also defer: {res:?}"
        );
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests_wire {
    use super::*;
    use crate::engine::HostState;
    use std::collections::BTreeSet;

    #[test]
    fn wire_binds_only_listed_imports() {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config).expect("engine");
        let mut linker: wasmtime::component::Linker<HostState> =
            wasmtime::component::Linker::new(&engine);
        let mut bound = BTreeSet::new();
        bound.insert("host.hash".into());
        bound.insert("host.log".into());
        // Smoke check — `wire_state_apply_linker` accepts the call
        // without erroring. Component-level link failure of an
        // unbound import is exercised in the e2e test (Task 37+).
        wire_state_apply_linker(&mut linker, &bound).expect("wire OK");
    }

    #[test]
    fn wire_accepts_full_ambient_set() {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config).expect("engine");
        let mut linker: wasmtime::component::Linker<HostState> =
            wasmtime::component::Linker::new(&engine);
        // host.install-key and host.verify-payload-mac are not bound
        // by wire_state_apply_linker (plan B), so excluding them.
        let mut bound = BTreeSet::new();
        bound.insert("host.hash".into());
        bound.insert("host.verify-signature".into());
        bound.insert("host.now-hlc-from-event".into());
        bound.insert("host.log".into());
        wire_state_apply_linker(&mut linker, &bound).expect("wire OK");
    }
}
