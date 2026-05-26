//! Capability gating logic.
//!
//! Per architecture.md §3.5: state-apply may bind ONLY the
//! deterministic helper set. Per capabilities.md §7.2: the kernel
//! intersects the app's declared set with what the component needs.
//! For state-apply specifically, "what the app declares" must be a
//! subset of the deterministic helper set — declaring any
//! non-deterministic import is a hard error regardless of kernel-major.
//!
//! In v1, state-propose and interaction share the same callable-function
//! ambient set as state-apply (the deterministic helpers). The three
//! profiles diverge only in (a) prewalk's `host-ui-surfaces@1.0.0`
//! types-only allowlist (interaction only), (b) float-ban gating
//! (state-apply only), (c) fuel budget — none of which are this
//! module's concern. The functions here therefore take a [`Profile`]
//! parameter but in v1 do not actually branch on it; the parameter is
//! the documented extension point for when the surfaces diverge.

use std::collections::BTreeSet;

use myrhiza_backend::BackendError;
use myrhiza_manifest::Manifest;
use myrhiza_manifest::vocabulary::{CapabilityClass, classify};

use crate::Profile;

/// The ambient host-callable-function set for `profile`.
///
/// In v1 all three profiles share the same callable-function ambient
/// set: the deterministic-helper subset of the vocabulary per
/// architecture.md §3.5 / spec §3.1.
///
/// `host.install-key` and `host.verify-payload-mac` are vocabulary-
/// registered authored capabilities (still `DeterministicHelper`
/// classified) but are intentionally NOT in the v1 ambient set: their
/// signatures take a `key-handle` resource whose infrastructure lands
/// in plan B (per determinism.md §5.1 v1 deferral). Manifests that
/// declare them are rejected at install with
/// [`BackendError::DeferredToPlanB`].
///
/// The `host-ui-surfaces@1.0.0` instance carries only type definitions
/// and is not a callable-function import — the prewalk handles its
/// types-only audit separately (per [`Profile::allow_ui_surfaces`]).
#[must_use]
pub fn ambient_set(profile: Profile) -> BTreeSet<String> {
    // v1: all three profiles share the deterministic-helper set. A
    // future profile-divergent ambient (e.g. propose getting
    // `host.author-event` when the author-event resource lands) would
    // match on `profile` here.
    let _ = profile;
    let mut s = BTreeSet::new();
    s.insert("host.hash".into());
    s.insert("host.verify-signature".into());
    s.insert("host.now-hlc-from-event".into());
    s.insert("host.log".into());
    s
}

/// Capabilities deferred to plan B — registered in the vocabulary as
/// `DeterministicHelper` but not bindable in v1. Declared here so
/// [`validate_manifest`] can short-circuit before the class check
/// rejects them as "non-deterministic".
const DEFERRED_TO_PLAN_B: &[&str] = &["host.install-key", "host.verify-payload-mac"];

/// Validate that the manifest's declared imports are a subset of the
/// `profile`'s ambient set. Any declared host-imports row that is not
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
/// is registered but not bindable in v1. Returns
/// [`BackendError::UnknownImport`] if a declared capability is not in
/// the v1 vocabulary. Returns [`BackendError::UnauthorizedImport`] if a
/// declared capability is in the vocabulary but not part of the
/// `profile`'s ambient set (i.e. any non-deterministic import).
pub fn validate_manifest(m: &Manifest, profile: Profile) -> Result<(), BackendError> {
    let ambient = ambient_set(profile);
    for (cap, &enabled) in &m.capabilities.host_imports {
        if !enabled {
            continue;
        }
        if DEFERRED_TO_PLAN_B.contains(&cap.as_str()) {
            return Err(BackendError::DeferredToPlanB(cap.clone()));
        }
        match classify(cap) {
            None => return Err(BackendError::UnknownImport(cap.clone())),
            Some(CapabilityClass::DeterministicHelper) => {}
            Some(_) => {
                return Err(BackendError::UnauthorizedImport(cap.clone()));
            }
        }
    }
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

/// Compute the set of host imports that should be bound on `profile`'s
/// linker for `manifest`. Returns the manifest-declared subset of the
/// profile's ambient set. Imports outside this set are NOT bound, so a
/// component attempting to import them fails to link.
///
/// In v1, this is profile-invariant (all three profiles share the
/// deterministic-helper ambient set). The `profile` parameter is the
/// documented extension point for when ambient sets diverge.
#[must_use]
pub fn bound_imports(m: &Manifest, profile: Profile) -> BTreeSet<String> {
    let ambient = ambient_set(profile);
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

    // host.log is always available (output-only sink per
    // determinism.md §5.1; no peer-divergence risk).
    bound.insert("host.log".into());

    bound
}

/// Register host functions on the Wasmtime linker for the imports
/// listed in `bound_imports`. Imports not listed are not registered;
/// a component attempting to import them will fail to link (this is
/// the load-bearing gating mechanic for plan A's acceptance criterion
/// #5 per capabilities.md §7.2).
///
/// In v1 the wiring is profile-invariant: all three profiles bind the
/// same deterministic-helper instance. The `profile` parameter is the
/// documented extension point for when wiring diverges. For interaction
/// the `host-ui-surfaces@1.0.0` instance is types-only — no `func_wrap`
/// calls are needed; bindgen treats types-only imports as auto-satisfied.
///
/// `host.install-key` and `host.verify-payload-mac` are intentionally
/// not bound here — they take a `key-handle` resource whose
/// infrastructure lands in plan B. They are also rejected up-front by
/// [`validate_manifest`] with [`BackendError::DeferredToPlanB`] when
/// declared in a manifest, so a v1 component that imports them never
/// reaches link time on the happy path.
///
/// # Errors
///
/// Returns [`BackendError::Instantiation`] if the linker rejects an
/// `instance(...)` lookup or `func_wrap(...)` registration. This
/// surface should not fail in practice; it would indicate a bindgen
/// / WIT mismatch at workspace build time.
pub fn wire_linker(
    linker: &mut wasmtime::component::Linker<crate::engine::HostState>,
    bound_imports: &BTreeSet<String>,
    profile: Profile,
) -> Result<(), BackendError> {
    use crate::engine::myrhiza::kernel::types::{Hlc as WitHlc, LogLevel as WitLogLevel};
    use crate::helpers::{
        LogLevel, host_hash_impl, host_now_hlc_from_event_impl, host_verify_signature_impl,
    };

    // v1: identical wiring for all three profiles. The match arm
    // exists to keep the parameter live (and to be the place where a
    // future profile-specific instance lands).
    let _ = profile;

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

    // host.log is always available per determinism.md §5.1 (output-only
    // sink; not part of state-digest). The bound set computed by
    // `bound_imports` always injects "host.log"; the assert locks that
    // invariant so a future refactor that drops the unconditional
    // insertion fails fast in debug builds rather than silently
    // regressing always-on logging.
    debug_assert!(
        bound_imports.contains("host.log"),
        "host.log must be in bound_imports — bound_imports always injects it",
    );
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use myrhiza_manifest::vocabulary::CapabilityClass;

    // ── B-7.2.1 ──────────────────────────────────────────────────────────────

    /// Covers: architecture.md §3.5, capabilities.md §7.1
    ///
    /// State-propose ambient set equals state-apply's ambient set
    /// (deterministic helpers only). Non-deterministic helpers declared in
    /// the state-propose WIT are NOT in the ambient set in v1.
    #[test]
    fn state_propose_ambient_set_contains_deterministic_helpers_only() {
        let propose_ambient = ambient_set(Profile::StatePropose);
        let apply_ambient = ambient_set(Profile::StateApply);
        assert_eq!(
            propose_ambient, apply_ambient,
            "v1 state-propose ambient set must equal state-apply ambient set"
        );
        // Every member must be DeterministicHelper.
        for cap in &propose_ambient {
            let class = myrhiza_manifest::vocabulary::classify(cap)
                .expect("ambient cap must be in vocabulary");
            assert_eq!(
                class,
                CapabilityClass::DeterministicHelper,
                "{cap} must be DeterministicHelper for state-propose ambient"
            );
        }
        // Known non-det helpers must not appear.
        assert!(
            !propose_ambient.contains("host.author-event"),
            "host.author-event (HostImport) must not be in propose ambient"
        );
        assert!(
            !propose_ambient.contains("host.broadcast"),
            "host.broadcast (HostImport) must not be in propose ambient"
        );
    }

    // ── B-7.2.6 ──────────────────────────────────────────────────────────────

    /// Covers: architecture.md §3.5, capabilities.md §7.2
    ///
    /// A manifest declaring `host.author-event` (classified `HostImport`,
    /// non-deterministic) under `host_imports` must be rejected by
    /// `validate_manifest(_, StatePropose)` with `UnauthorizedImport`.
    /// Verifies the gating fires on the propose path, not just state-apply.
    #[test]
    fn manifest_with_apply_only_capability_declared_for_propose_rejects() {
        // Pre-condition: verify host.author-event is classified as
        // HostImport (non-deterministic) so the test documents the
        // vocabulary contract it relies on.
        assert_eq!(
            myrhiza_manifest::vocabulary::classify("host.author-event"),
            Some(CapabilityClass::HostImport),
            "host.author-event must be HostImport for this test to be meaningful"
        );

        let mut m = sample_state_apply_manifest();
        m.capabilities
            .host_imports
            .insert("host.author-event".into(), true);
        let res = validate_manifest(&m, Profile::StatePropose);
        assert!(
            matches!(res, Err(BackendError::UnauthorizedImport(_))),
            "non-det host.author-event must be rejected as unauthorized for propose: {res:?}"
        );
    }

    // ── existing state-apply tests ────────────────────────────────────────────

    /// Covers: determinism.md §5.1, capabilities.md §7.1
    ///
    /// The state-apply ambient set is the deterministic-helper subset
    /// per architecture.md §3.5 / determinism.md §5.1; this test
    /// verifies every member of the ambient set is in fact classified
    /// as a `DeterministicHelper` in the vocabulary, locking the
    /// "ambient set = deterministic helpers, nothing else" invariant
    /// that capabilities.md §7.1 builds on.
    #[test]
    fn state_apply_ambient_is_only_deterministic_helpers() {
        let ambient = ambient_set(Profile::StateApply);
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
        // Manifest declares non-deterministic broadcast — invalid for
        // a state-apply-only bundle.
        m.capabilities
            .host_imports
            .insert("host.broadcast".into(), true);
        let res = validate_manifest(&m, Profile::StateApply);
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
        let res = validate_manifest(&m, Profile::StateApply);
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
        let res = validate_manifest(&m, Profile::StateApply);
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
        let res = validate_manifest(&m, Profile::StateApply);
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
        validate_manifest(&m, Profile::StateApply).expect("helper-set-only must validate");
    }

    /// Covers: determinism.md §5.1, capabilities.md §7.1
    ///
    /// `host.log` is always-on for state-apply per determinism.md §5.1
    /// — manifests need not (and historically did not) declare it. The
    /// `bound_imports` function unconditionally inserts it even when
    /// the manifest omits both `host_imports` and
    /// `deterministic_helpers` entries for it. This test locks that
    /// behavior so a future refactor that drops the unconditional
    /// insertion (or makes log declaration mandatory) fails fast.
    #[test]
    fn manifest_omitting_host_log_still_binds_it() {
        let m = sample_state_apply_manifest();
        // Pre-condition: the manifest declares neither host_imports
        // nor deterministic_helpers entries for host.log.
        assert!(!m.capabilities.host_imports.contains_key("host.log"));
        assert!(
            !m.capabilities
                .deterministic_helpers
                .contains_key("host.log")
        );

        let bound = bound_imports(&m, Profile::StateApply);
        assert!(
            bound.contains("host.log"),
            "host.log must be bound even when manifest omits it (always-on per §5.1): {bound:?}"
        );
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
        // Smoke check — `wire_linker` accepts the call without
        // erroring. Component-level link failure of an unbound import
        // is exercised in the e2e test (Task 37+).
        wire_linker(&mut linker, &bound, Profile::StateApply).expect("wire OK");
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
        // by wire_linker (plan B), so excluding them.
        let mut bound = BTreeSet::new();
        bound.insert("host.hash".into());
        bound.insert("host.verify-signature".into());
        bound.insert("host.now-hlc-from-event".into());
        bound.insert("host.log".into());
        wire_linker(&mut linker, &bound, Profile::StateApply).expect("wire OK");
    }

    // ── B-7.2.3 — propose/interaction linker smoke tests ─────────────────────

    /// Covers spec §3.1: `wire_linker(_, _, Profile::StatePropose)` accepts
    /// the same bound set as state-apply. The two linkers share the
    /// deterministic helper set in v1.
    #[test]
    fn wire_state_propose_linker_accepts_deterministic_set() {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config).expect("engine");
        let mut linker: wasmtime::component::Linker<HostState> =
            wasmtime::component::Linker::new(&engine);
        let mut bound = BTreeSet::new();
        bound.insert("host.hash".into());
        bound.insert("host.log".into());
        wire_linker(&mut linker, &bound, Profile::StatePropose).expect("wire propose OK");
    }

    /// Covers spec §3.1: `wire_linker(_, _, Profile::Interaction)` accepts
    /// the deterministic set. `host-ui-surfaces@1.0.0` is types-only — no
    /// `func_wrap` calls are registered for it, so the linker wiring is
    /// identical to propose.
    #[test]
    fn wire_interaction_linker_accepts_deterministic_set() {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config).expect("engine");
        let mut linker: wasmtime::component::Linker<HostState> =
            wasmtime::component::Linker::new(&engine);
        let mut bound = BTreeSet::new();
        bound.insert("host.hash".into());
        bound.insert("host.log".into());
        wire_linker(&mut linker, &bound, Profile::Interaction).expect("wire interaction OK");
    }

    /// Covers spec §3.1: `HOST_UI_SURFACES_INSTANCE` constant matches the
    /// WIT package/interface name `myrhiza:kernel/host-ui-surfaces@1.0.0`.
    /// The prewalk matches by string; this test locks the constant so a
    /// typo surfaces immediately.
    #[test]
    fn host_ui_surfaces_instance_constant_matches_wit_name() {
        assert_eq!(
            crate::engine::HOST_UI_SURFACES_INSTANCE,
            "myrhiza:kernel/host-ui-surfaces@1.0.0",
        );
    }
}
