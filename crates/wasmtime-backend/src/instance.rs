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
/// - [`wasmtime::Trap::StackOverflow`] → [`BackendError::StackExhausted`]
///   (the wasm stack is pinned to
///   [`myrhiza_types::limits::MAX_WASM_STACK_V1`] (512 KiB) on every
///   peer per determinism.md §5.3, so a recursion-bomb traps at the
///   same call depth on every peer; surfacing the failure as a typed
///   variant lets the kernel deterministically quarantine the
///   component rather than carrying an opaque trap string).
/// - any other [`wasmtime::Trap`] variant → [`BackendError::Trap`]
///   carrying the trap's `Display` form for diagnostics.
/// - non-trap errors (host-call panics, etc.) →
///   [`BackendError::Instantiation`] carrying the error chain.
///
/// Takes `&wasmtime::Error` rather than the value so callers can keep
/// the original error around if they need to log it; we only need to
/// inspect the downcast and format.
pub(crate) fn map_wasmtime_error(e: &wasmtime::Error) -> BackendError {
    if let Some(trap) = e.downcast_ref::<wasmtime::Trap>() {
        return match trap {
            wasmtime::Trap::OutOfFuel => BackendError::FuelExhausted,
            wasmtime::Trap::MemoryOutOfBounds => BackendError::MemoryExhausted,
            wasmtime::Trap::StackOverflow => BackendError::StackExhausted,
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    //! Unit coverage for [`map_wasmtime_error`].
    //!
    //! Every state-apply trap categorization the kernel relies on
    //! flows through this function, so each typed variant gets a
    //! direct mapping test. `wasmtime::Error: From<wasmtime::Trap>`
    //! lets us synthesize traps without running a real wasm fixture
    //! — the alternative is to author a recursion-bomb component for
    //! `Trap::StackOverflow`, which adds a wasm-build dependency for
    //! coverage that is mechanically equivalent to the typed downcast.
    //! Fuel and memory exhaustion already have integration coverage
    //! via the kernel acceptance suite (`fuel_exhaustion_traps_apply`
    //! and friends), so we only mirror their direct-mapping check
    //! here for parity.
    use super::map_wasmtime_error;
    use myrhiza_backend::BackendError;
    use wasmtime::Error as WasmtimeError;
    use wasmtime::Trap;

    #[test]
    fn stack_overflow_trap_maps_to_stack_exhausted() {
        let err: WasmtimeError = Trap::StackOverflow.into();
        assert!(
            matches!(map_wasmtime_error(&err), BackendError::StackExhausted),
            "Trap::StackOverflow must surface as BackendError::StackExhausted; \
             got: {:?}",
            map_wasmtime_error(&err),
        );
    }

    #[test]
    fn out_of_fuel_trap_maps_to_fuel_exhausted() {
        let err: WasmtimeError = Trap::OutOfFuel.into();
        assert!(
            matches!(map_wasmtime_error(&err), BackendError::FuelExhausted),
            "Trap::OutOfFuel must surface as BackendError::FuelExhausted; \
             got: {:?}",
            map_wasmtime_error(&err),
        );
    }

    #[test]
    fn memory_out_of_bounds_trap_maps_to_memory_exhausted() {
        let err: WasmtimeError = Trap::MemoryOutOfBounds.into();
        assert!(
            matches!(map_wasmtime_error(&err), BackendError::MemoryExhausted),
            "Trap::MemoryOutOfBounds must surface as BackendError::MemoryExhausted; \
             got: {:?}",
            map_wasmtime_error(&err),
        );
    }

    #[test]
    fn other_trap_falls_through_to_generic_trap() {
        // Any variant that isn't one of the three typed cases must
        // fall through to BackendError::Trap. `IntegerOverflow` is
        // representative — it is not specialized and so should land
        // in the generic bucket carrying the trap's Display form.
        let err: WasmtimeError = Trap::IntegerOverflow.into();
        match map_wasmtime_error(&err) {
            BackendError::Trap(msg) => {
                assert!(
                    !msg.is_empty(),
                    "BackendError::Trap must carry a non-empty diagnostic"
                );
            }
            other => panic!("non-specialized trap must map to BackendError::Trap; got: {other:?}"),
        }
    }
}
