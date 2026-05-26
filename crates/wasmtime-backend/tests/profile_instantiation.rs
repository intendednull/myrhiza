//! Per-profile instantiation tests for the state-propose and interaction profiles.
//!
//! Covers: architecture.md §3.2, architecture.md §3.3.
//!
//! Counter WASM artifacts under `tests/fixtures/built/counter-*.wasm`
//! are produced from `examples/counter/` (per
//! docs/specs/2026-05-26-b-8-sdk-design.md §3.3) by the Justfile's
//! `_build-example` recipe. The output paths stayed stable through
//! the B-8 T6 cutover; only the source dir moved.
//!
//! B-7.3.4 — `instantiate_state_propose_loads_counter_propose_fixture`:
//! loads the counter-state-propose WASM artifact (built from
//! `examples/counter/src/propose.rs` per B-8 T6), instantiates via
//! `WasmtimeBackend::instantiate_state_propose`, calls `propose` with
//! an Increment-5 intent, and asserts the returned payload equals
//! `5_i64.to_be_bytes()`.
//!
//! B-7.3.5 — `instantiate_state_apply_rejects_propose_fixture_with_instantiation_error`:
//! reads the same propose artifact but calls `instantiate_state_apply`.
//! The component exports `propose` but not `apply`/`state-digest`, so the
//! state-apply instantiation must fail with `BackendError::Instantiation`.
//!
//! B-7.4.4 — `instantiate_interaction_loads_counter_interaction_fixture`:
//! loads the counter-interaction WASM artifact (built from
//! `examples/counter/src/interaction.rs` per B-8 T6), instantiates via
//! `WasmtimeBackend::instantiate_interaction`, calls `view` with an
//! 8-byte BE i64 state, asserts the rendered text, then calls
//! `dispatch("inc 3")` and asserts the intent bytes.
//!
//! B-7.4.5 — `instantiate_interaction_rejects_state_apply_fixture`:
//! reads the counter-state-apply artifact but calls `instantiate_interaction`.
//! The component exports `apply`/`state-digest` but not `view`/`dispatch`,
//! so instantiation must fail with `BackendError::Instantiation`.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;

use myrhiza_backend::{Backend, BackendError};
use myrhiza_manifest::schema::{
    AbiSection, AppSection, AuthorIdentityClass, AuthorPolicy, CapabilitiesSection,
    ComponentsSection, DeterminismSection, DriftDetectionSection, HighValueOps, Manifest,
    ModulesSection, StateDigestFormat,
};
use myrhiza_wasmtime_backend::WasmtimeBackend;

/// Build a minimal valid manifest with the `interaction` slot set so
/// `validate_interaction_manifest` does not short-circuit on a missing
/// interaction path. No capability helpers are declared.
fn empty_interaction_manifest() -> Manifest {
    let mut m = Manifest {
        app: AppSection {
            name: "test".into(),
            version: "0.1.0".into(),
            description: "test".into(),
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
            state_apply: None,
            state_propose: None,
            interaction: Some("dummy".into()),
            behavior: None,
        },
        author_policy: AuthorPolicy::default_deny(),
        signature: None,
    };
    m.canonicalize();
    m
}

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

/// Covers: architecture.md §3.2, architecture.md §3.3
///
/// The counter-interaction fixture exports `view(state, peer-state) ->
/// list<u8>` and `dispatch(action) -> result<list<u8>, string>`. Calls
/// must round-trip correctly through the Wasmtime component-model ABI.
///
/// - `view([0,0,0,0,0,0,0,5], [])` must return `b"counter: 5\n"`.
/// - `dispatch("inc 3")` must return `Ok([0x00, 0,0,0,0,0,0,0,3])` — the
///   intent bytes consumed by `state-propose.propose`.
#[test]
fn instantiate_interaction_loads_counter_interaction_fixture() {
    let bytes = std::fs::read("../../tests/fixtures/built/counter-interaction.wasm")
        .expect("counter-interaction fixture missing — run `just build-fixtures`");

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let manifest = empty_interaction_manifest();

    let mut instance = backend
        .instantiate_interaction(&bytes, &manifest)
        .expect("instantiate_interaction should succeed for counter fixture");

    // view of state = 5 (i64 BE) → "counter: 5\n"
    let state = 5_i64.to_be_bytes().to_vec();
    let view_bytes = instance
        .call_view(&state, &[])
        .expect("call_view must not return a BackendError");
    assert_eq!(
        view_bytes, b"counter: 5\n",
        "view of state=5 must render as 'counter: 5\\n'"
    );

    // dispatch("inc 3") → Ok([0x00] ++ 3_i64.to_be_bytes())
    let intent = instance
        .call_dispatch("inc 3")
        .expect("call_dispatch must not return a BackendError");
    let mut expected_intent = vec![0x00_u8];
    expected_intent.extend_from_slice(&3_i64.to_be_bytes());
    assert_eq!(
        intent,
        Ok(expected_intent),
        "dispatch('inc 3') must return Ok([0x00] ++ 3_i64.to_be_bytes())"
    );
}

/// Covers: architecture.md §3.2
///
/// `instantiate_interaction` on a state-apply fixture must fail because the
/// fixture exports `apply`/`state-digest` but not `view`/`dispatch`. The
/// failure must be `BackendError::Instantiation` (link-time world mismatch).
#[test]
fn instantiate_interaction_rejects_state_apply_fixture() {
    let bytes = std::fs::read("../../tests/fixtures/built/counter-state-apply.wasm")
        .expect("counter-state-apply fixture missing — run `just build-fixtures`");

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let manifest = empty_interaction_manifest();

    let result = backend.instantiate_interaction(&bytes, &manifest);

    match result {
        Ok(_) => panic!("instantiate_interaction must reject a state-apply fixture"),
        Err(err) => assert!(
            matches!(err, BackendError::Instantiation(_)),
            "expected BackendError::Instantiation for wrong-world fixture, got {err:?}"
        ),
    }
}
