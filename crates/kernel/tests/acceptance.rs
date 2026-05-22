//! Kernel-tier acceptance tests.
//!
//! These tests prove the load-bearing claims for plan A's foundation
//! slice: mvp.md §15.1 criteria #1 (kernel loads + instantiates a WASM
//! state component from a signed bundle) and #5 (capability declarations
//! gate access). Each test wires real artifacts: real wasm components
//! built by `just build-fixtures`, real Wasmtime instantiation, real
//! Ed25519 signature verification.
//!
//! The fixtures live under `tests/fixtures/built/` and are produced by
//! the workspace's Justfile recipe (manual `cargo build` for
//! `wasm32-unknown-unknown` plus `wasm-tools component embed/new` to
//! wrap into a component with the fixture's WIT). See the `Justfile`
//! comment block for why we deviate from cargo-component.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::PathBuf;

use myrhiza_backend::{Backend, BackendError};
use myrhiza_kernel::{ApplyOutcome, BundleAddress, InstallFlow, StateApplyHandle};
use myrhiza_manifest::bundle_content_hash;
use myrhiza_test_utils::bundle::{
    build_counter_bundle_with_extra_cap, build_signed_counter_bundle, write_bundle,
};
use myrhiza_test_utils::manifest::{
    deterministic_signing_key, helpers_only_state_apply_manifest, sign_manifest,
};
use myrhiza_types::EventHash;
use myrhiza_wasmtime_backend::WasmtimeBackend;

use bincode::Options;
use myrhiza_types::{AuthorPubkey, Event, GenesisV1, Hlc, canonical_bincode};
use std::collections::BTreeSet;

/// Path to the counter-state-apply fixture built by `just build-fixtures`.
fn counter_fixture_path() -> PathBuf {
    // Tests run with cwd = crate dir. Walk up to the workspace root
    // so the path resolves consistently from `cargo test` and from
    // editor-driven test runners.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(2)
        .expect("workspace root is two levels above kernel crate manifest")
        .join("tests/fixtures/built/counter-state-apply.wasm")
}

/// Covers: mvp.md §15.1, verification.md §22.1, distribution.md §10.5
///
/// Plan-A criterion #1 part 1: kernel loads a signed bundle, verifies
/// the Ed25519 signature, returns the manifest + component bytes.
/// Exercises the install flow per distribution.md §10.5.
#[test]
fn kernel_loads_signed_bundle() {
    let (_bundle, addr) = build_signed_counter_bundle();

    let flow = InstallFlow::new();
    let loaded = flow.load(&addr).expect("install flow loads + verifies");

    assert_eq!(loaded.manifest.app.name, "test-fixture");
    assert!(
        !loaded.component_bytes.is_empty(),
        "component bytes must be populated"
    );
    // The stored content_hash is the composite bundle-content-hash per
    // spec §3.4. For a single-component bundle (no propose/interaction/behavior)
    // it equals bundle_content_hash(Some(&bytes), None, None, None).
    let expected = bundle_content_hash(Some(&loaded.component_bytes), None, None, None);
    assert_eq!(expected, loaded.content_hash);
}

/// Covers: mvp.md §15.1, convergence.md §4.4
///
/// Plan-A criterion #1 part 2: the full loop. Load + instantiate via
/// `WasmtimeBackend`, drive real state-apply calls against canonical
/// `Event` envelopes (per plan-B-1 Task 20's fixture rewrite):
///   1. Apply a Genesis event whose `GenesisV1::app_payload` is the
///      8-byte BE i64 zero — the fixture returns that as the initial
///      state.
///   2. Apply a non-genesis +5 increment event whose `payload` is the
///      8-byte BE i64 delta — the fixture adds it to the prior state.
///
/// This exercises:
///   - `InstallFlow::load` (signature verify),
///   - `WasmtimeBackend::instantiate_state_apply` (linker + fuel + memcap),
///   - `StateApplyHandle::apply` (bindgen-typed call),
///   - canonical-bincode decode of the wire `Event` envelope inside the
///     fixture.
///
/// `signature` is left zero: `handle.apply` does not verify signatures
/// (kernel verifies at insert).
#[test]
fn kernel_instantiates_and_applies_increment() {
    let (_bundle, addr) = build_signed_counter_bundle();

    let flow = InstallFlow::new();
    let loaded = flow.load(&addr).expect("load + verify");

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let instance = backend
        .instantiate_state_apply(&loaded.component_bytes, &loaded.manifest)
        .expect("instantiate counter state-apply");
    let mut handle = StateApplyHandle::new(instance);

    let author = AuthorPubkey::from_bytes([1; 32]);

    // Build the Genesis event: app_payload is the canonical 8-byte BE
    // i64 zero, which the fixture returns verbatim as initial state.
    let initial_state = 0_i64.to_be_bytes().to_vec();
    let genesis_payload = GenesisV1 {
        seed: [0x11; 32],
        founder_pubkey: author,
        app_payload: initial_state.clone(),
    };
    let genesis_payload_bytes = canonical_bincode()
        .serialize(&genesis_payload)
        .expect("encode genesis payload");
    let genesis = Event {
        author,
        seq: 1,
        prev: EventHash::ZERO,
        deps: BTreeSet::new(),
        hlc: Hlc {
            wall_ms: 0,
            logical: 0,
        },
        payload: genesis_payload_bytes,
        signature: [0; 64],
    };
    let genesis_bytes = canonical_bincode()
        .serialize(&genesis)
        .expect("encode genesis event");
    let result = handle
        .apply(&[], &genesis_bytes)
        .expect("genesis apply succeeds");
    assert!(
        matches!(result.outcome, ApplyOutcome::Accepted),
        "expected Accepted verdict for genesis, got {:?}",
        result.outcome
    );
    assert_eq!(
        result.new_state, initial_state,
        "fixture must return genesis app_payload as initial state"
    );

    // Apply a +5 increment as a non-genesis event referencing genesis.
    let increment = Event {
        author,
        seq: 2,
        prev: genesis.wire_hash(),
        deps: BTreeSet::new(),
        hlc: Hlc {
            wall_ms: 0,
            logical: 0,
        },
        payload: 5_i64.to_be_bytes().to_vec(),
        signature: [0; 64],
    };
    let increment_bytes = canonical_bincode()
        .serialize(&increment)
        .expect("encode increment event");
    let result = handle
        .apply(&result.new_state, &increment_bytes)
        .expect("increment apply succeeds");
    assert!(
        matches!(result.outcome, ApplyOutcome::Accepted),
        "expected Accepted verdict for increment, got {:?}",
        result.outcome
    );
    assert_eq!(
        result.new_state,
        5_i64.to_be_bytes().to_vec(),
        "0 + 5 = 5 (8-byte BE i64)"
    );
}

/// Path to the over-importer fixture. See
/// `tests/fixtures/over-importer/wit/world.wit` for the import that
/// the state-apply linker does not bind.
fn over_importer_fixture_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(2)
        .expect("workspace root is two levels above kernel crate manifest")
        .join("tests/fixtures/built/over-importer.wasm")
}

/// Covers: mvp.md §15.1, capabilities.md §7.2
///
/// Plan-A criterion #5: capability declarations gate access. The
/// over-importer fixture imports `myrhiza:kernel/host-non-deterministic.random`,
/// which the state-apply linker per architecture.md §3.5 does NOT
/// bind. Component instantiation must therefore fail.
///
/// This is the structural defense — the linker has no binding for the
/// import, so the component fails to link. Independent of whether the
/// manifest happens to have over-declared (here it has not, so the
/// rejection cannot come from manifest validation).
#[test]
fn capability_gating_rejects_non_deterministic_import() {
    let component_bytes = std::fs::read(over_importer_fixture_path()).unwrap_or_else(|e| {
        panic!(
            "over-importer fixture missing at {}: {e} — run `just build-fixtures`",
            over_importer_fixture_path().display()
        )
    });

    // Use a minimal helper-set-only manifest: declares only host.hash
    // + host.log. The component's `host-non-deterministic.random`
    // import is therefore NOT validated up front (manifest gating
    // doesn't see it because the manifest doesn't list it). The
    // rejection comes structurally from the linker missing the
    // binding. That's the load-bearing claim per capabilities.md
    // §7.2: the linker is the gate, not just the manifest.
    let mut manifest = helpers_only_state_apply_manifest();
    let key = deterministic_signing_key(11);
    sign_manifest(&mut manifest, &component_bytes, &key);

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let Err(err) = backend.instantiate_state_apply(&component_bytes, &manifest) else {
        panic!("over-importer must be rejected at link time");
    };

    // Typed verdict categorization per Task 7 of plan
    // 2026-05-10-foundation-review-fixes: the engine pre-walks the
    // component's imports against the bound set before reaching the
    // linker, so an unauthorized import surfaces as
    // `BackendError::UnauthorizedImport(_)` deterministically. The
    // previous substring assertion (`host-non-deterministic` / `random` /
    // `import` / `unknown`) was load-bearing on wasmtime's link-error
    // wording — fragile across LTS bumps.
    assert!(
        matches!(err, BackendError::UnauthorizedImport(_)),
        "over-importer must surface UnauthorizedImport; got: {err:?}"
    );
}

/// Path to a built fixture under `tests/fixtures/built/`.
fn fixture_path(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(2)
        .expect("workspace root is two levels above kernel crate manifest")
        .join("tests/fixtures/built")
        .join(format!("{name}.wasm"))
}

/// Build a signed bundle for a fixture wasm + helpers-only manifest.
/// The manifest is signed with `seed` so different fixtures get
/// distinct keys (cosmetic; not load-bearing for the test).
fn build_signed_bundle_for(
    fixture_name: &str,
    seed: u8,
) -> (myrhiza_test_utils::bundle::TestBundle, BundleAddress) {
    let component_bytes = std::fs::read(fixture_path(fixture_name)).unwrap_or_else(|e| {
        panic!(
            "fixture {fixture_name} missing at {}: {e} — run `just build-fixtures`",
            fixture_path(fixture_name).display()
        )
    });
    let mut manifest = helpers_only_state_apply_manifest();
    let key = deterministic_signing_key(seed);
    sign_manifest(&mut manifest, &component_bytes, &key);

    let test_bundle = write_bundle(&manifest, &component_bytes).expect("write bundle to tempdir");
    let addr = BundleAddress {
        bundle_dir: test_bundle.bundle_dir.clone(),
        manifest_path: test_bundle.manifest_path.clone(),
    };
    (test_bundle, addr)
}

/// Covers: convergence.md §4.4, mvp.md §15.1, verification.md §22.5
///
/// Pre-check fail-closed: a state-apply that always returns Reject
/// must surface as `ApplyOutcome::Rejected` with the reject reason.
/// Caller convention (kernel originator path): on Rejected, do NOT
/// sign or broadcast. The handle returns the reject reason; the
/// kernel surfaces it as a user-visible error and stops.
///
/// The second half of this test asserts the §22.5 pre-check / apply
/// agreement invariant: the same `(prior_state, event)` pair must
/// produce the same verdict through `apply` as through `pre_check`.
#[test]
fn pre_check_returns_reject_and_does_not_commit() {
    let (_bundle, addr) = build_signed_bundle_for("pre-check-rejector", 13);

    let flow = InstallFlow::new();
    let loaded = flow.load(&addr).expect("load + verify");

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let instance = backend
        .instantiate_state_apply(&loaded.component_bytes, &loaded.manifest)
        .expect("instantiate pre-check-rejector");
    let mut handle = StateApplyHandle::new(instance);

    let result = handle.pre_check(b"", b"any-event").expect("pre_check OK");
    match result.outcome {
        ApplyOutcome::Rejected(reason) => {
            assert_eq!(reason, "not allowed");
        }
        ApplyOutcome::Accepted => {
            panic!("pre-check-rejector must reject; caller would otherwise sign+broadcast");
        }
    }
    // The fixture also returns an empty candidate state on reject —
    // the kernel discards it either way, but the contract is that
    // pre-check on Reject returns no useful state.
    assert!(result.candidate_state.is_empty());

    // §22.5 pre-check / apply agreement invariant: the same
    // `(prior_state, event)` pair must produce the same verdict
    // through `apply` as through `pre_check`. Pre-check is
    // mechanically the same WASM `apply` call run in dry-run mode
    // (per architecture.md §3.5 / determinism.md §5.1) — if the
    // verdicts diverge, a state-apply is non-deterministic and
    // cross-peer convergence is at risk. Asserting both paths return
    // `Reject("not allowed")` documents that contract.
    let apply_result = handle.apply(b"", b"any-event").expect("apply OK");
    match apply_result.outcome {
        ApplyOutcome::Rejected(reason) => {
            assert_eq!(reason, "not allowed");
        }
        ApplyOutcome::Accepted => {
            panic!(
                "pre-check / apply agreement: apply must reject when pre-check rejected the same input"
            );
        }
    }
}

/// Covers: determinism.md §5.3, mvp.md §15.1
///
/// Fuel exhaustion: the kernel sets a 10M-unit fuel budget per
/// `apply` call. A state-apply that spins forever must trap when
/// fuel is exhausted, and the failure must surface as the typed
/// [`BackendError::FuelExhausted`] variant (Task 7 of the foundation
/// review fixes plan) so callers can categorize it as
/// compute-budget rather than a generic trap or user reject.
#[test]
fn fuel_exhaustion_traps_apply() {
    use myrhiza_kernel::ApplyError;

    let (_bundle, addr) = build_signed_bundle_for("infinite-loop", 17);

    let flow = InstallFlow::new();
    let loaded = flow.load(&addr).expect("load + verify");

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let instance = backend
        .instantiate_state_apply(&loaded.component_bytes, &loaded.manifest)
        .expect("instantiate infinite-loop");
    let mut handle = StateApplyHandle::new(instance);

    let Err(err) = handle.apply(b"", b"any-event") else {
        panic!("infinite-loop must trap on fuel exhaustion");
    };
    // ApplyError::Backend wraps the underlying BackendError; downcast
    // is via the existing `From<BackendError>` impl. The typed-variant
    // assertion replaces the previous `msg.contains("fuel"|"trap")`
    // substring check, which depended on wasmtime's `Trap::OutOfFuel`
    // Display impl wording.
    let ApplyError::Backend(backend_err) = err;
    assert!(
        matches!(backend_err, BackendError::FuelExhausted),
        "fuel exhaustion must surface as BackendError::FuelExhausted; got: {backend_err:?}"
    );
}

/// Covers: determinism.md §5.2, mvp.md §15.1
///
/// Float-ban: the byte-level lint per determinism.md §5.2 scans
/// every embedded core module's function bodies and rejects any
/// component containing a banned float instruction. The float-banned
/// fixture's `apply` body contains `f32.add`; instantiation must
/// fail before the wasm runs.
#[test]
fn float_banned_fixture_rejected_at_install() {
    let component_bytes = std::fs::read(fixture_path("float-banned")).unwrap_or_else(|e| {
        panic!(
            "float-banned fixture missing at {}: {e} — run `just build-fixtures`",
            fixture_path("float-banned").display()
        )
    });
    let mut manifest = helpers_only_state_apply_manifest();
    let key = deterministic_signing_key(19);
    sign_manifest(&mut manifest, &component_bytes, &key);

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let Err(err) = backend.instantiate_state_apply(&component_bytes, &manifest) else {
        panic!("float-banned fixture must be rejected at install");
    };
    // Typed verdict categorization per Task 7: the float-ban scanner
    // surfaces `BackendError::BannedInstruction(_)` directly, so we
    // assert the variant rather than substring-matching the formatted
    // error message.
    assert!(
        matches!(err, BackendError::BannedInstruction(_)),
        "float-ban must surface BannedInstruction; got: {err:?}"
    );
}

/// Covers: capabilities.md §7.2, distribution.md §10.5, mvp.md §15.1
///
/// Manifest-arm of plan-A criterion #5: a state-apply bundle whose
/// manifest declares a non-deterministic capability (here `host.broadcast`,
/// classified as `HostImport` in the v1 vocabulary) must be rejected
/// at install. The complementary linker-arm
/// (`capability_gating_rejects_non_deterministic_import`) covers the
/// case where a component imports a non-deterministic function the
/// linker refuses to bind. This test covers the manifest-side gate:
/// the counter fixture itself does not import `host.broadcast`, but
/// declaring it in `capabilities.host_imports` is still rejected
/// up-front by [`validate_state_apply_manifest`] before the linker
/// runs. Bundle signing is intact (signature verifies) — the failure
/// surfaces from the backend's manifest gating step.
#[test]
fn manifest_declaring_non_deterministic_cap_rejects_at_install() {
    let component_bytes = std::fs::read(counter_fixture_path()).unwrap_or_else(|e| {
        panic!(
            "counter fixture missing at {}: {e} — run `just build-fixtures`",
            counter_fixture_path().display()
        )
    });

    let bundle = build_counter_bundle_with_extra_cap(&component_bytes, "host.broadcast", 23)
        .expect("write bundle to tempdir");

    // Sanity: the install flow itself still accepts the bundle —
    // signature verification is independent of capability gating.
    // The manifest-gating rejection comes from the backend.
    let addr = BundleAddress {
        bundle_dir: bundle.bundle_dir.clone(),
        manifest_path: bundle.manifest_path.clone(),
    };
    let flow = InstallFlow::new();
    let loaded = flow
        .load(&addr)
        .expect("install flow accepts the signed bundle (cap gating is downstream)");

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let Err(err) = backend.instantiate_state_apply(&loaded.component_bytes, &loaded.manifest)
    else {
        panic!("manifest declaring host.broadcast must be rejected at install");
    };

    // `host.broadcast` is `HostImport`-classified in the v1 vocabulary,
    // so `validate_state_apply_manifest` rejects it as
    // `UnauthorizedImport`. Plan-B-deferred caps (e.g. `host.install-key`)
    // would surface as `DeferredToPlanB` instead — both arms are
    // load-bearing for §15.1 #5 manifest gating, so we accept either.
    assert!(
        matches!(
            err,
            BackendError::UnauthorizedImport(_) | BackendError::DeferredToPlanB(_)
        ),
        "manifest-arm gating must surface UnauthorizedImport or DeferredToPlanB; got: {err:?}"
    );
}
