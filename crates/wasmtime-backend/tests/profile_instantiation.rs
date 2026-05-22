//! Per-profile instantiation tests for the state-propose profile.
//!
//! Covers: architecture.md §3.2, architecture.md §3.3.
//!
//! B-7.3.4 — `instantiate_state_propose_loads_counter_propose_fixture`:
//! loads the counter-state-propose WASM fixture (built by B-7.5),
//! instantiates via `WasmtimeBackend::instantiate_state_propose`, calls
//! `propose` with an Increment-5 intent, and asserts the returned payload
//! equals `5_i64.to_be_bytes()`.
//!
//! B-7.3.5 — `instantiate_state_apply_rejects_propose_fixture_with_instantiation_error`:
//! reads the same propose fixture but calls `instantiate_state_apply`.
//! The fixture exports `propose` but not `apply`/`state-digest`, so the
//! state-apply instantiation must fail with `BackendError::Instantiation`.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;

use myrhiza_backend::{Backend, BackendError};
use myrhiza_manifest::schema::{
    AbiSection, AppSection, AuthorIdentityClass, AuthorPolicy, CapabilitiesSection,
    ComponentsSection, DeterminismSection, DriftDetectionSection, HighValueOps, Manifest,
    ModulesSection, StateDigestFormat,
};
use myrhiza_wasmtime_backend::WasmtimeBackend;

/// Build a minimal valid manifest for a component that declares no
/// propose-specific helpers. The `state_propose` field is set so
/// `validate_state_propose_manifest` does not short-circuit.
fn empty_manifest() -> Manifest {
    let mut m = Manifest {
        app: AppSection {
            name: "test".into(),
            version: "0.1.0".into(),
            description: "test".into(),
            // author_pubkey must be non-empty but is not validated
            // against a real key here.
            author_pubkey: format!("0x{}", "00".repeat(32)),
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
            deterministic_helpers: BTreeMap::new(),
        },
        determinism: DeterminismSection {
            allow_floats: false,
            drift_detection: DriftDetectionSection {
                interval_events: 1024,
            },
        },
        modules: ModulesSection { dep: vec![] },
        components: ComponentsSection {
            state_apply: Some("dummy".into()),
            state_propose: Some("dummy".into()),
            interaction: None,
            behavior: None,
        },
        author_policy: AuthorPolicy::default_deny(),
        signature: None,
    };
    m.canonicalize();
    m
}

/// Covers: architecture.md §3.2, architecture.md §3.3
///
/// The counter-state-propose fixture exports `propose(prior-state, intent)
/// -> result<list<u8>, string>`. An Increment-5 intent (`[0x00] ++ 5_i64
/// .to_be_bytes()`) must return `Ok(Ok(5_i64.to_be_bytes().to_vec()))`.
/// Asserts the full instantiate→call→result round-trip succeeds.
#[test]
fn instantiate_state_propose_loads_counter_propose_fixture() {
    let bytes = std::fs::read("../../tests/fixtures/built/counter-state-propose.wasm")
        .expect("counter-state-propose fixture missing — run `just build-fixtures`");

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let manifest = empty_manifest();

    let mut instance = backend
        .instantiate_state_propose(&bytes, &manifest)
        .expect("instantiate_state_propose should succeed for counter fixture");

    // Increment-5 intent: discriminant 0x00 followed by delta as i64 big-endian.
    let mut intent = vec![0x00_u8];
    intent.extend_from_slice(&5_i64.to_be_bytes());

    let result = instance
        .call_propose(&[], &intent)
        .expect("call_propose must not return a BackendError");

    assert_eq!(
        result,
        Ok(5_i64.to_be_bytes().to_vec()),
        "Increment-5 propose must return Ok(5_i64.to_be_bytes())"
    );
}

/// Covers: architecture.md §3.2
///
/// `instantiate_state_apply` on a state-propose fixture must fail because
/// the fixture exports `propose` but not `apply`/`state-digest`. The failure
/// must be `BackendError::Instantiation` (link-time world mismatch), not a
/// capability or float-ban error.
#[test]
fn instantiate_state_apply_rejects_propose_fixture_with_instantiation_error() {
    let bytes = std::fs::read("../../tests/fixtures/built/counter-state-propose.wasm")
        .expect("counter-state-propose fixture missing — run `just build-fixtures`");

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let manifest = empty_manifest();

    let result = backend.instantiate_state_apply(&bytes, &manifest);

    // `Box<dyn ComponentInstance>` does not implement Debug, so we cannot
    // use `unwrap_err()` or `expect_err()`. Extract the error via pattern
    // match instead.
    match result {
        Ok(_) => panic!("instantiate_state_apply must reject a state-propose fixture"),
        Err(err) => assert!(
            matches!(err, BackendError::Instantiation(_)),
            "expected BackendError::Instantiation for wrong-world fixture, got {err:?}"
        ),
    }
}
