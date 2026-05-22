//! `InteractionInstance` impl for the interaction profile.
//!
//! Wraps a `Store<HostState>` plus the bindgen-generated `Interaction`
//! bindings type. Maps Wasmtime traps to [`BackendError`] variants via
//! the shared [`crate::instance::map_wasmtime_error`] helper. Mirrors
//! the shape of [`crate::propose_instance::StateProposeInstance`];
//! interaction is a non-deterministic profile so no float-ban is applied
//! at instantiation time, and the fuel budget differs (see
//! [`myrhiza_types::limits::INTERACTION_FUEL_BUDGET_V1`]).

use myrhiza_backend::{BackendError, InteractionInstance};
use wasmtime::{
    Store,
    component::{Component, Linker},
};

use crate::engine::{HostState, interaction_bindings::Interaction};
use crate::instance::map_wasmtime_error;

/// Loaded interaction instance. Owns its `Store` (so its fuel,
/// memory cap, and log sink live for the lifetime of the instance).
pub(crate) struct InteractionInstanceImpl {
    store: Store<HostState>,
    bindings: Interaction,
}

impl InteractionInstanceImpl {
    /// Instantiate an interaction component against the given linker.
    ///
    /// Capability-rejection is detected by the prewalk in
    /// [`crate::engine::WasmtimeBackend::instantiate_interaction`]
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
        let bindings = Interaction::instantiate(&mut store, component, linker)
            .map_err(|e| map_wasmtime_error(&e))?;
        Ok(Self { store, bindings })
    }
}

impl InteractionInstance for InteractionInstanceImpl {
    fn call_view(&mut self, state: &[u8], peer_state: &[u8]) -> Result<Vec<u8>, BackendError> {
        self.bindings
            .call_view(&mut self.store, state, peer_state)
            .map_err(|e| map_wasmtime_error(&e))
    }

    fn call_dispatch(&mut self, action: &str) -> Result<Result<Vec<u8>, String>, BackendError> {
        self.bindings
            .call_dispatch(&mut self.store, action)
            .map_err(|e| map_wasmtime_error(&e))
    }

    fn call_on_broadcast_completion(
        &mut self,
        token: &[u8],
        ok: bool,
        err: &str,
    ) -> Result<(), BackendError> {
        self.bindings
            .call_on_broadcast_completion(&mut self.store, token, ok, err)
            .map_err(|e| map_wasmtime_error(&e))
    }

    fn call_on_blob_fetch_completion(
        &mut self,
        token: &[u8],
        ok: bool,
        payload: &[u8],
        err: &str,
    ) -> Result<(), BackendError> {
        self.bindings
            .call_on_blob_fetch_completion(&mut self.store, token, ok, payload, err)
            .map_err(|e| map_wasmtime_error(&e))
    }
}
