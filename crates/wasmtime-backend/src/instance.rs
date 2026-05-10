//! `ComponentInstance` impl for state-apply.
//!
//! Wraps a `Store<HostState>` plus the bindgen-generated
//! `StateApply` bindings type. Maps Wasmtime traps to
//! [`BackendError`] variants so the kernel can distinguish fuel
//! exhaustion, capability rejection, and verdict-reject from
//! generic instantiation failures.

use myrhiza_backend::{BackendError, ComponentInstance, Verdict};
use wasmtime::{
    Store,
    component::{Component, Linker},
};

use crate::engine::{HostState, StateApply, myrhiza::kernel::types::Verdict as WitVerdict};

/// Loaded state-apply instance. Owns its `Store` (so its fuel,
/// memory cap, and log sink live for the lifetime of the instance).
pub(crate) struct StateApplyInstance {
    store: Store<HostState>,
    bindings: StateApply,
}

impl StateApplyInstance {
    /// Instantiate a state-apply component against the given linker.
    ///
    /// Distinguishes capability-rejection (linker missing import) from
    /// other instantiation failures by string-matching the wasmtime
    /// error. This is brittle but acceptable for plan A — the kernel
    /// surfaces the original error message either way; the only
    /// observable difference is the `BackendError` variant the kernel
    /// uses to categorize the failure for diagnostics.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::UnauthorizedImport`] if the component
    /// imports a function the linker did not bind (i.e. a capability
    /// gating violation surfaced at link time). Returns
    /// [`BackendError::Instantiation`] for any other failure.
    pub(crate) fn instantiate(
        mut store: Store<HostState>,
        component: &Component,
        linker: &Linker<HostState>,
    ) -> Result<Self, BackendError> {
        let bindings = StateApply::instantiate(&mut store, component, linker).map_err(|e| {
            let s = e.to_string();
            // Wasmtime's link-error messages include phrases like
            // "import `...` not provided" / "unknown import"; treat
            // those as gating rejections.
            if s.contains("import") || s.contains("unknown") {
                BackendError::UnauthorizedImport { imported: s }
            } else {
                BackendError::Instantiation(s)
            }
        })?;
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
            .map_err(|e| {
                // Fuel exhaustion is a distinct failure category —
                // kernel may want to surface "compute budget exceeded"
                // rather than a generic trap. Wasmtime's
                // `Trap::OutOfFuel` formats with the substring "fuel".
                if e.to_string().contains("fuel") {
                    BackendError::Trap("fuel exhausted".into())
                } else {
                    BackendError::Trap(e.to_string())
                }
            })?;

        let v = match verdict {
            WitVerdict::Accept => Verdict::Accept,
            WitVerdict::Reject(s) => Verdict::Reject(s),
        };
        Ok((v, new_state))
    }

    fn call_state_digest(&mut self, state: &[u8]) -> Result<Vec<u8>, BackendError> {
        self.bindings
            .call_state_digest(&mut self.store, state)
            .map_err(|e| BackendError::Trap(e.to_string()))
    }
}
