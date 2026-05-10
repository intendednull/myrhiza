//! Wasmtime engine + Backend impl.
//!
//! `wasmtime::component::bindgen!` generates host trait skeletons
//! and a `StateApply` instantiation type from the state-apply WIT
//! world. Plan A binds the deterministic helper set manually via
//! per-method `linker.func_wrap` (see [`crate::gating::wire_state_apply_linker`])
//! so the kernel can intersect the manifest-declared subset against
//! the ambient set at link time, rejecting components that import
//! anything outside that subset.

use std::sync::Arc;

use myrhiza_backend::{Backend, BackendError, ComponentInstance};
use myrhiza_manifest::Manifest;
use wasmtime::{
    Engine, Store,
    component::{Component, Linker, ResourceTable},
};

use crate::float_ban::scan_component_for_floats;
use crate::gating::{
    state_apply_bound_imports, validate_state_apply_manifest, wire_state_apply_linker,
};
use crate::helpers::LogSink;
use crate::instance::StateApplyInstance;

wasmtime::component::bindgen!({
    path: "../../wit/myrhiza-kernel/wit",
    world: "state-apply",
});

/// Per-instance host state held in the Wasmtime `Store`.
///
/// Wired into the Wasmtime store via [`Store::new`] in
/// [`WasmtimeBackend::instantiate_state_apply`] and consulted from
/// inside `func_wrap` closures in [`crate::gating::wire_state_apply_linker`].
pub struct HostState {
    /// Per-peer log sink for `host.log` records. Drained by the
    /// kernel; not part of state-digest per determinism.md §5.1.
    pub log_sink: Arc<LogSink>,
    /// Set of capability names bound on this instance's linker.
    /// Currently informational; the linker itself enforces the gate.
    pub bound_imports: std::collections::BTreeSet<String>,
    /// Wasmtime resource table for component-model resources.
    /// Plan A's state-apply does not actually instantiate any
    /// resources (key-handle bindings land in plan B), but the
    /// table is required by the bindgen-generated host trait shape.
    pub table: ResourceTable,
    /// Memory cap limiter per determinism.md §5.3 (64 MB).
    /// Wired via [`Store::limiter`] so any `memory.grow` past 64 MB
    /// traps inside the wasm component.
    pub limits: wasmtime::StoreLimits,
}

/// State-apply fuel budget per determinism.md §5.3.
///
/// 10M units; the deterministic helper costs in
/// `wit/host-deterministic.wit` are chosen so a typical apply
/// fits well within this envelope.
pub const STATE_APPLY_FUEL_BUDGET: u64 = 10_000_000;

/// State-apply memory cap per determinism.md §5.3 (64 MiB).
pub const STATE_APPLY_MEMORY_CAP: usize = 64 * 1024 * 1024;

/// Backend impl using Wasmtime's component model.
///
/// Holds a single `wasmtime::Engine` configured for fuel + the
/// component model. Each `instantiate_state_apply` call builds a
/// fresh `Store` + `Linker` + `Component` from this engine.
pub struct WasmtimeBackend {
    engine: Engine,
}

impl WasmtimeBackend {
    /// Build a new backend with fuel + component-model enabled per
    /// determinism.md §5.3.
    ///
    /// Float-ban is a byte-level lint enforced before instantiation
    /// (see [`scan_component_for_floats`]); we deliberately do *not*
    /// disable Wasmtime's float support because the lint runs first
    /// and the WIT package does not declare any float-typed exports.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Instantiation`] if the Wasmtime engine
    /// cannot be constructed with the requested config.
    pub fn new() -> Result<Self, BackendError> {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        config.wasm_component_model(true);
        let engine =
            Engine::new(&config).map_err(|e| BackendError::Instantiation(e.to_string()))?;
        Ok(Self { engine })
    }

    /// Reference to the underlying Wasmtime engine. Used by the
    /// instance type to build typed function calls.
    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}

impl Backend for WasmtimeBackend {
    fn instantiate_state_apply(
        &self,
        component_bytes: &[u8],
        manifest: &Manifest,
    ) -> Result<Box<dyn ComponentInstance>, BackendError> {
        // 1. Manifest gating check — declared imports must be a
        //    subset of the deterministic helper ambient set.
        validate_state_apply_manifest(manifest)?;

        // 2. Float-ban lint — reject the component up front if any
        //    function body uses an f32/f64/SIMD-float instruction.
        scan_component_for_floats(component_bytes).map_err(banned_instruction_from_string)?;

        // 3. Compute the bound import set (manifest ∩ ambient).
        let bound_imports = state_apply_bound_imports(manifest);

        // 4. Decode the component.
        let component = Component::from_binary(&self.engine, component_bytes)
            .map_err(|e| BackendError::Instantiation(format!("decode component: {e}")))?;

        // 5. Build the linker, binding ONLY the allowed imports.
        let mut linker: Linker<HostState> = Linker::new(&self.engine);
        wire_state_apply_linker(&mut linker, &bound_imports)?;

        // 6. Build the store with fuel budget + memory cap per
        //    determinism.md §5.3.
        let host_state = HostState {
            log_sink: Arc::new(LogSink::default()),
            bound_imports,
            table: ResourceTable::new(),
            limits: wasmtime::StoreLimitsBuilder::new()
                .memory_size(STATE_APPLY_MEMORY_CAP)
                .build(),
        };
        let mut store: Store<HostState> = Store::new(&self.engine, host_state);
        store
            .set_fuel(STATE_APPLY_FUEL_BUDGET)
            .map_err(|e| BackendError::Instantiation(format!("set_fuel: {e}")))?;
        // Enforce the 64 MB memory cap via the StoreLimits the
        // host_state already carries. `memory.grow` past the cap
        // returns the wasm-level "out of memory" sentinel, which
        // surfaces as a trap in typed function calls.
        store.limiter(|s| &mut s.limits);

        // 7. Instantiate via the bindgen-generated `StateApply` type.
        let instance = StateApplyInstance::instantiate(store, &component, &linker)?;
        Ok(Box::new(instance))
    }
}

/// Wrap a dynamic float-ban error string into [`BackendError::BannedInstruction`].
///
/// `BackendError::BannedInstruction` carries a `&'static str`; the
/// scan returns an owned `String`. We leak the string into a
/// `'static` reference. Cardinality is bounded — at most one banned
/// op per component scan — so the leak is a non-issue for the
/// lifetime of a backend (one leak per failed manifest gating).
fn banned_instruction_from_string(s: String) -> BackendError {
    let leaked: &'static str = Box::leak(s.into_boxed_str());
    BackendError::BannedInstruction(leaked)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn new_backend_constructs() {
        let _b = WasmtimeBackend::new().expect("backend constructs");
    }
}
