//! Shared test helpers for kernel-tier acceptance + convergence tests.
//!
//! Replicates plan-A's `build_signed_counter_bundle` pattern from
//! `crates/kernel/tests/acceptance.rs` (now hoisted to
//! `crates/test-utils/src/bundle.rs` in Task 24.3a) and adds
//! B-1-specific plumbing (`CorruptingDecorator`).
//!
//! API names verified against plan-A code (2026-05-10):
//! - `myrhiza_test_utils::bundle::TestBundle`
//! - `myrhiza_test_utils::bundle::build_signed_counter_bundle`
//! - `myrhiza_kernel::{InstallFlow, BundleAddress}`
//! - `myrhiza_wasmtime_backend::WasmtimeBackend`
//! - `WasmtimeBackend::instantiate_state_apply(&bytes, &manifest)`

#![allow(dead_code)] // not every test consumes every helper
#![allow(clippy::expect_used)] // test-only module

use std::path::PathBuf;

use myrhiza_backend::{Backend, ComponentInstance};
use myrhiza_kernel::{BundleAddress, InstallFlow, StateApplyHandle};
use myrhiza_test_utils::bundle::{
    TestBundle, build_signed_counter_bundle, build_signed_echo_bundle, write_bundle,
};
use myrhiza_test_utils::manifest::{
    deterministic_signing_key, helpers_only_state_apply_manifest, sign_manifest,
};
use myrhiza_wasmtime_backend::WasmtimeBackend;

/// Install + instantiate the counter-state-apply fixture and return a
/// fresh `StateApplyHandle`. Each call returns an independent wasmtime
/// instance with its own Store.
#[must_use]
pub fn counter_handle() -> StateApplyHandle {
    let inner = counter_component_instance();
    StateApplyHandle::new(inner)
}

/// Install + instantiate the echo-state-apply fixture and return a
/// fresh `StateApplyHandle`. Each call returns an independent wasmtime
/// instance with its own Store.
#[must_use]
pub fn echo_handle() -> StateApplyHandle {
    let (_bundle, addr) = build_signed_echo_bundle();
    let flow = InstallFlow::new();
    let loaded = flow.load(&addr).expect("InstallFlow::load");
    let backend = WasmtimeBackend::new().expect("WasmtimeBackend::new");
    let instance = backend
        .instantiate_state_apply(&loaded.component_bytes, &loaded.manifest)
        .expect("instantiate_state_apply");
    StateApplyHandle::new(instance)
}

/// Same as `counter_handle()` but returns the unwrapped
/// `Box<dyn ComponentInstance>` (used by `CorruptingDecorator`).
#[must_use]
pub fn counter_component_instance() -> Box<dyn ComponentInstance> {
    let (_bundle, addr) = build_signed_counter_bundle();
    let flow = InstallFlow::new();
    let loaded = flow.load(&addr).expect("InstallFlow::load");
    let backend = WasmtimeBackend::new().expect("WasmtimeBackend::new");
    backend
        .instantiate_state_apply(&loaded.component_bytes, &loaded.manifest)
        .expect("instantiate_state_apply")
}

/// Wrap `counter_component_instance()` in a `CorruptingDecorator`.
#[must_use]
pub fn corrupting_counter_handle(corrupt_at: u32) -> StateApplyHandle {
    let inner = counter_component_instance();
    let wrapped = CorruptingDecorator::new(inner, corrupt_at);
    StateApplyHandle::new(Box::new(wrapped))
}

/// Path to the pre-check-rejector fixture built by `just build-fixtures`.
///
/// Resolves to `<workspace_root>/tests/fixtures/built/pre-check-rejector.wasm`
/// via `CARGO_MANIFEST_DIR`. The kernel crate sits at `crates/kernel/`,
/// so walking up two ancestors reaches the workspace root.
#[must_use]
fn pre_check_rejector_fixture_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(2)
        .expect("workspace root is two levels above kernel crate manifest")
        .join("tests/fixtures/built/pre-check-rejector.wasm")
}

/// Build a signed bundle wrapping the pre-check-rejector fixture.
///
/// Used by `dropped_at_apply_records_rejected_events` in
/// `tests/convergence.rs`. The pre-check-rejector returns
/// `Reject("not allowed")` from BOTH `apply` and `pre_check` (they are
/// the same wasm function in dry-run / canonical modes per spec §4.4),
/// which is the load-bearing property that drives the test: events
/// hand-injected onto a peer running this handle reach `replay_full`,
/// hit the Reject branch, and land in `dropped_at_apply`.
pub fn build_signed_pre_check_rejector_bundle() -> (TestBundle, BundleAddress) {
    let component_bytes = std::fs::read(pre_check_rejector_fixture_path()).unwrap_or_else(|e| {
        panic!(
            "pre-check-rejector fixture missing at {}: {e} — run `just build-fixtures`",
            pre_check_rejector_fixture_path().display()
        )
    });
    let mut manifest = helpers_only_state_apply_manifest();
    // Seed 13 mirrors the seed used in `acceptance::pre_check_returns_reject_and_does_not_commit`
    // — cosmetic, not load-bearing for the test.
    let key = deterministic_signing_key(13);
    sign_manifest(&mut manifest, &component_bytes, &key);

    let test_bundle = write_bundle(&manifest, &component_bytes).expect("write bundle to tempdir");
    let addr = BundleAddress {
        bundle_dir: test_bundle.bundle_dir.clone(),
        manifest_path: test_bundle.manifest_path.clone(),
    };
    (test_bundle, addr)
}

/// Install + instantiate the pre-check-rejector fixture and return a
/// fresh `StateApplyHandle`. The handle's `apply` always returns
/// `Reject("not allowed")`.
#[must_use]
pub fn pre_check_rejector_handle() -> StateApplyHandle {
    let (_bundle, addr) = build_signed_pre_check_rejector_bundle();
    let flow = InstallFlow::new();
    let loaded = flow.load(&addr).expect("InstallFlow::load");
    let backend = WasmtimeBackend::new().expect("WasmtimeBackend::new");
    let instance = backend
        .instantiate_state_apply(&loaded.component_bytes, &loaded.manifest)
        .expect("instantiate pre-check-rejector");
    StateApplyHandle::new(instance)
}

/// Wraps a `ComponentInstance` and flips one byte in `state_digest()`
/// output after `corrupt_at` applies. Used to drive a synthetic digest
/// divergence between peers in `drift_detected_when_state_apply_corrupted`.
pub struct CorruptingDecorator {
    inner: Box<dyn ComponentInstance>,
    apply_count: u32,
    corrupt_at: u32,
}

impl CorruptingDecorator {
    #[must_use]
    pub fn new(inner: Box<dyn ComponentInstance>, corrupt_at: u32) -> Self {
        Self {
            inner,
            apply_count: 0,
            corrupt_at,
        }
    }
}

impl ComponentInstance for CorruptingDecorator {
    fn call_apply(
        &mut self,
        prior: &[u8],
        event: &[u8],
    ) -> Result<(myrhiza_backend::Verdict, Vec<u8>), myrhiza_backend::BackendError> {
        self.apply_count += 1;
        self.inner.call_apply(prior, event)
    }

    fn call_state_digest(
        &mut self,
        state: &[u8],
    ) -> Result<Vec<u8>, myrhiza_backend::BackendError> {
        let mut d = self.inner.call_state_digest(state)?;
        if self.apply_count >= self.corrupt_at && !d.is_empty() {
            d[0] ^= 0xFF;
        }
        Ok(d)
    }
}
