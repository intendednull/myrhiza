//! Wasmtime engine + Backend impl skeleton.
//!
//! `wasmtime::component::bindgen!` generates host trait skeletons
//! and a `StateApply` instantiation type from the state-apply WIT
//! world. The actual `Backend` trait impl lands in Task 27 once
//! [`crate::instance::StateApplyInstance`] and
//! [`crate::gating::wire_state_apply_linker`] are in place.

use std::sync::Arc;

use myrhiza_backend::BackendError;
use wasmtime::{Engine, component::ResourceTable};

use crate::helpers::LogSink;

wasmtime::component::bindgen!({
    path: "../../wit/myrhiza-kernel/wit",
    world: "state-apply",
});

/// Per-instance host state held in the Wasmtime `Store`.
///
/// Wired into the Wasmtime store via [`wasmtime::Store::new`] in
/// `WasmtimeBackend::instantiate_state_apply` (Task 27) and
/// consulted from inside `func_wrap` closures registered in
/// [`crate::gating::wire_state_apply_linker`] (Task 26).
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
    /// Wired via [`wasmtime::Store::limiter`] so any `memory.grow`
    /// past 64 MB traps inside the wasm component.
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
/// component model. Each `instantiate_state_apply` call (added in
/// Task 27) builds a fresh `Store` + `Linker` + `Component` from
/// this engine.
pub struct WasmtimeBackend {
    engine: Engine,
}

impl WasmtimeBackend {
    /// Build a new backend with fuel + component-model enabled per
    /// determinism.md §5.3.
    ///
    /// Float-ban is a byte-level lint enforced before instantiation
    /// (see [`crate::float_ban::scan_component_for_floats`]); we
    /// deliberately do *not* disable Wasmtime's float support
    /// because the lint runs first and the WIT package does not
    /// declare any float-typed exports.
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
    /// instance type (Task 27) to build typed function calls.
    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.engine
    }
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
