//! `ProposeInstance` impl for state-propose.
//!
//! Wraps a `Store<HostState>` plus the bindgen-generated `StatePropose`
//! bindings type. Maps Wasmtime traps to [`BackendError`] variants via
//! the shared [`crate::instance::map_wasmtime_error`] helper. Mirrors
//! the shape of [`crate::instance::StateApplyInstance`]; state-propose
//! is a non-deterministic profile so no float-ban is applied at
//! instantiation time, and the fuel budget differs (see
//! [`myrhiza_types::limits::STATE_PROPOSE_FUEL_BUDGET_V1`]).

use myrhiza_backend::{BackendError, ProposeInstance};
use wasmtime::{
    Store,
    component::{Component, Linker},
};

use crate::engine::{HostState, propose_bindings::StatePropose};
use crate::instance::map_wasmtime_error;

/// Loaded state-propose instance. Owns its `Store` (so its fuel,
/// memory cap, and log sink live for the lifetime of the instance).
pub(crate) struct StateProposeInstance {
    store: Store<HostState>,
    bindings: StatePropose,
}

impl StateProposeInstance {
    /// Instantiate a state-propose component against the given linker.
    ///
    /// Capability-rejection is detected by the prewalk in
    /// [`crate::engine::WasmtimeBackend::instantiate_state_propose`]
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
        let bindings = StatePropose::instantiate(&mut store, component, linker)
            .map_err(|e| map_wasmtime_error(&e))?;
        Ok(Self { store, bindings })
    }
}

impl ProposeInstance for StateProposeInstance {
    fn call_propose(
        &mut self,
        prior_state: &[u8],
        intent: &[u8],
    ) -> Result<Result<Vec<u8>, String>, BackendError> {
        self.bindings
            .call_propose(&mut self.store, prior_state, intent)
            .map_err(|e| map_wasmtime_error(&e))
    }
}
