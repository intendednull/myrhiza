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

use myrhiza_backend::Backend;
use myrhiza_kernel::{ApplyOutcome, BundleAddress, InstallFlow, StateApplyHandle};
use myrhiza_test_utils::bundle::write_bundle;
use myrhiza_test_utils::manifest::{
    deterministic_signing_key, helpers_only_state_apply_manifest, sign_manifest,
};
use myrhiza_types::EventHash;
use myrhiza_wasmtime_backend::WasmtimeBackend;

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

/// Encode an Increment event for the counter fixture's hand-rolled
/// wire format (see `tests/fixtures/counter-state-apply/src/lib.rs`):
/// 1 tag byte + 8 bytes big-endian i64.
fn encode_increment(by: i64) -> Vec<u8> {
    let mut v = Vec::with_capacity(9);
    v.push(0u8); // TAG_INCREMENT
    v.extend_from_slice(&by.to_be_bytes());
    v
}

/// Decode the fixture's state bytes: empty (zero) or 8 bytes big-endian i64.
fn decode_state(bytes: &[u8]) -> i64 {
    if bytes.is_empty() {
        return 0;
    }
    let arr: [u8; 8] = bytes.try_into().expect("state bytes must be 0 or 8");
    i64::from_be_bytes(arr)
}

/// Build a signed counter-state-apply bundle on disk. Used by the load
/// + instantiate tests. Returns the test bundle (kept alive for tempdir
///   RAII) and the bundle address pointing into it.
fn build_signed_counter_bundle() -> (myrhiza_test_utils::bundle::TestBundle, BundleAddress) {
    let component_bytes = std::fs::read(counter_fixture_path()).unwrap_or_else(|e| {
        panic!(
            "counter fixture missing at {}: {e} — run `just build-fixtures`",
            counter_fixture_path().display()
        )
    });
    let content_hash = EventHash::blake3(&component_bytes);

    let mut manifest = helpers_only_state_apply_manifest();
    let key = deterministic_signing_key(7);
    sign_manifest(&mut manifest, &content_hash, &key);

    let test_bundle = write_bundle(&manifest, &component_bytes).expect("write bundle to tempdir");
    let addr = BundleAddress {
        bundle_dir: test_bundle.bundle_dir.clone(),
        manifest_path: test_bundle.manifest_path.clone(),
    };
    (test_bundle, addr)
}

/// Covers: mvp.md §15.1, verification.md §22.1
///
/// Plan-A criterion #1 part 1: kernel loads a signed bundle, verifies
/// the Ed25519 signature, returns the manifest + component bytes.
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
    // The recomputed content hash must match what the signing target
    // committed to (otherwise verify_signature would have errored).
    let recomputed = EventHash::blake3(&loaded.component_bytes);
    assert_eq!(recomputed, loaded.content_hash);
}

/// Covers: mvp.md §15.1, convergence.md §4.4
///
/// Plan-A criterion #1 part 2: the full loop. Load + instantiate via
/// `WasmtimeBackend`, drive a real state-apply call (Increment by 42),
/// decode the returned state bytes, assert the counter reads 42.
/// This exercises:
///   - `InstallFlow::load` (signature verify),
///   - `WasmtimeBackend::instantiate_state_apply` (linker + fuel + memcap),
///   - `StateApplyHandle::apply` (bindgen-typed call),
///   - canonical-bincode round-trip of app-defined state.
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

    let event = encode_increment(42);
    let result = handle
        .apply(b"", &event)
        .expect("apply Increment{by:42} succeeds");

    assert!(
        matches!(result.outcome, ApplyOutcome::Accepted),
        "expected Accepted verdict, got {:?}",
        result.outcome
    );
    let value = decode_state(&result.new_state);
    assert_eq!(value, 42);
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
    sign_manifest(&mut manifest, &EventHash::blake3(&component_bytes), &key);

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let Err(err) = backend.instantiate_state_apply(&component_bytes, &manifest) else {
        panic!("over-importer must be rejected at link time");
    };

    let msg = err.to_string();
    assert!(
        msg.contains("host-non-deterministic")
            || msg.contains("random")
            || msg.contains("import")
            || msg.contains("unknown"),
        "rejection error must mention the missing import; got: {msg}"
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
    let content_hash = EventHash::blake3(&component_bytes);

    let mut manifest = helpers_only_state_apply_manifest();
    let key = deterministic_signing_key(seed);
    sign_manifest(&mut manifest, &content_hash, &key);

    let test_bundle = write_bundle(&manifest, &component_bytes).expect("write bundle to tempdir");
    let addr = BundleAddress {
        bundle_dir: test_bundle.bundle_dir.clone(),
        manifest_path: test_bundle.manifest_path.clone(),
    };
    (test_bundle, addr)
}

/// Covers: convergence.md §4.4, mvp.md §15.1
///
/// Pre-check fail-closed: a state-apply that always returns Reject
/// must surface as `ApplyOutcome::Rejected` with the reject reason.
/// Caller convention (kernel originator path): on Rejected, do NOT
/// sign or broadcast. The handle returns the reject reason; the
/// kernel surfaces it as a user-visible error and stops.
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
}

/// Covers: determinism.md §5.3, mvp.md §15.1
///
/// Fuel exhaustion: the kernel sets a 10M-unit fuel budget per
/// `apply` call. A state-apply that spins forever must trap when
/// fuel is exhausted. The error must mention "fuel" or "trap" so
/// callers can categorize the failure as compute-budget rather than
/// a user-defined reject.
#[test]
fn fuel_exhaustion_traps_apply() {
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
    let msg = err.to_string();
    assert!(
        msg.contains("fuel") || msg.contains("trap"),
        "fuel-exhaustion error should mention fuel/trap; got: {msg}"
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
    let content_hash = EventHash::blake3(&component_bytes);

    let mut manifest = helpers_only_state_apply_manifest();
    let key = deterministic_signing_key(19);
    sign_manifest(&mut manifest, &content_hash, &key);

    let backend = WasmtimeBackend::new().expect("backend constructs");
    let Err(err) = backend.instantiate_state_apply(&component_bytes, &manifest) else {
        panic!("float-banned fixture must be rejected at install");
    };
    let msg = err.to_string();
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("float")
            || lower.contains("f32")
            || lower.contains("f64")
            || lower.contains("banned"),
        "float-ban error should mention float/f32/f64/banned; got: {msg}"
    );
}
