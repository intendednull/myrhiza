//! `ComponentInstance` impl for state-apply.
//!
//! Wraps a `Store<HostState>` plus the bindgen-generated
//! `StateApply` bindings type. Maps Wasmtime traps to
//! [`BackendError`] variants so the kernel can distinguish fuel
//! exhaustion, memory-cap exhaustion, capability rejection, and
//! verdict-reject from generic instantiation failures.
//!
//! Trap categorization uses typed downcast against
//! [`wasmtime::Trap`] (an enum) — not error-string matching. The
//! string-match approach was brittle across wasmtime LTS bumps:
//! `Trap::OutOfFuel`'s `Display` impl has shifted between releases,
//! and a future bump could silently break the categorization. The
//! downcast variant compiles against the wasmtime API directly so
//! the typecheck catches drift.

use myrhiza_backend::{BackendError, ComponentInstance, Verdict};
use wasmtime::{
    Store,
    component::{Component, Linker},
};

use crate::engine::{HostState, StateApply, myrhiza::kernel::types::Verdict as WitVerdict};

/// Map a `wasmtime::Error` returned from a state-apply call into the
/// matching [`BackendError`] variant via typed downcast.
///
/// - [`wasmtime::Trap::OutOfFuel`] → [`BackendError::FuelExhausted`]
/// - [`wasmtime::Trap::MemoryOutOfBounds`] → [`BackendError::MemoryExhausted`]
///   (the `wasmtime::StoreLimits` memory cap surfaces as an
///   out-of-bounds trap when `memory.grow` would exceed
///   [`myrhiza_types::limits::COMPONENT_MEMORY_CAP_V1`]).
/// - any other [`wasmtime::Trap`] variant → [`BackendError::Trap`]
///   carrying the trap's `Display` form for diagnostics.
/// - non-trap errors (host-call panics, etc.) →
///   [`BackendError::Instantiation`] carrying the error chain.
///
/// Takes `&wasmtime::Error` rather than the value so callers can keep
/// the original error around if they need to log it; we only need to
/// inspect the downcast and format.
fn map_wasmtime_error(e: &wasmtime::Error) -> BackendError {
    if let Some(trap) = e.downcast_ref::<wasmtime::Trap>() {
        return match trap {
            wasmtime::Trap::OutOfFuel => BackendError::FuelExhausted,
            wasmtime::Trap::MemoryOutOfBounds => BackendError::MemoryExhausted,
            other => BackendError::Trap(other.to_string()),
        };
    }
    BackendError::Instantiation(e.to_string())
}

/// Loaded state-apply instance. Owns its `Store` (so its fuel,
/// memory cap, and log sink live for the lifetime of the instance).
pub(crate) struct StateApplyInstance {
    store: Store<HostState>,
    bindings: StateApply,
}

impl StateApplyInstance {
    /// Instantiate a state-apply component against the given linker.
    ///
    /// Capability-rejection (an import the manifest did not declare or
    /// the state-apply ambient set does not provide) is detected by
    /// the engine-level pre-walk in
    /// [`crate::engine::WasmtimeBackend::instantiate_state_apply`]
    /// before this function is called, so a linker error here is
    /// always a non-capability failure.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Instantiation`] if instantiation fails.
    pub(crate) fn instantiate(
        mut store: Store<HostState>,
        component: &Component,
        linker: &Linker<HostState>,
    ) -> Result<Self, BackendError> {
        let bindings = StateApply::instantiate(&mut store, component, linker)
            .map_err(|e| map_wasmtime_error(&e))?;
        Ok(Self { store, bindings })
    }
}

impl ComponentInstance for StateApplyInstance {
    fn call_apply(
        &mut self,
        prior_state: &[u8],
        event: &[u8],
    ) -> Result<(Verdict, Vec<u8>), BackendError> {
        let (verdict, new_state) = self
            .bindings
            .call_apply(&mut self.store, prior_state, event)
            .map_err(|e| map_wasmtime_error(&e))?;

        let v = match verdict {
            WitVerdict::Accept => Verdict::Accept,
            WitVerdict::Reject(s) => Verdict::Reject(s),
        };
        Ok((v, new_state))
    }

    fn call_state_digest(&mut self, state: &[u8]) -> Result<Vec<u8>, BackendError> {
        self.bindings
            .call_state_digest(&mut self.store, state)
            .map_err(|e| map_wasmtime_error(&e))
    }
}
