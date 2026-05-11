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

use myrhiza_backend::{Backend, ComponentInstance};
use myrhiza_kernel::{InstallFlow, StateApplyHandle};
use myrhiza_test_utils::bundle::build_signed_counter_bundle;
use myrhiza_wasmtime_backend::WasmtimeBackend;

/// Install + instantiate the counter-state-apply fixture and return a
/// fresh `StateApplyHandle`. Each call returns an independent wasmtime
/// instance with its own Store.
#[must_use]
pub fn counter_handle() -> StateApplyHandle {
    let inner = counter_component_instance();
    StateApplyHandle::new(inner)
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
