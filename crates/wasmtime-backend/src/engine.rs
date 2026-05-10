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
use myrhiza_types::limits::{COMPONENT_MEMORY_CAP_V1, STATE_APPLY_FUEL_BUDGET_V1};
use wasmtime::{
    Config, Engine, Store,
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

/// Backend impl using Wasmtime's component model.
///
/// Holds a single `wasmtime::Engine` configured for fuel + the
/// component model. Each `instantiate_state_apply` call builds a
/// fresh `Store` + `Linker` + `Component` from this engine.
pub struct WasmtimeBackend {
    engine: Engine,
}

/// Build the deterministic `wasmtime::Config` used for every engine
/// construction in the runtime per determinism.md §5.2 + §5.3.
///
/// Wasmtime's defaults shift across LTS bumps (SIMD, relaxed-SIMD,
/// threads, memory64, multi-memory have all flipped between releases).
/// Replay determinism requires that two peers running the same kernel
/// major produce identical post-state for the same event sequence —
/// which means every engine must accept and reject the same set of
/// instructions. We therefore pin each feature flag explicitly rather
/// than relying on whatever the wasmtime build defaults to.
///
/// Disabled (nondeterministic / NaN-bit-divergent / cross-peer drift):
/// - `wasm_simd` — `v128` ops include float lanes; bit patterns vary.
/// - `wasm_relaxed_simd` — explicitly nondeterministic by design.
/// - `wasm_memory64` — orthogonal pointer width; not in v1.
/// - `wasm_multi_memory` — orthogonal store layout; not in v1.
///
/// Off at build time (no `Config` runtime call needed):
/// - `wasm_threads` — the `wasm_threads` `Config` method is gated on
///   the wasmtime `threads` cargo feature, which we deliberately do
///   not enable in the workspace `wasmtime` dep. Threads are therefore
///   off at compile time, which is strictly stronger than a runtime
///   `wasm_threads(false)` call.
///
/// Enabled (component-model load-bearing — already on by default in
/// wasmtime 36's `WasmFeatures::WASM2` baseline; documented here so
/// that a future LTS bump dropping any of these lights up loudly):
/// - `wasm_bulk_memory`, `wasm_multi_value` — required by the
///   component-model lowering. Both default-on; we re-enable them
///   defensively.
/// - `wasm_reference_types` — required by the component-model
///   lowering. Default-on as part of WASM2. The `wasm_reference_types`
///   `Config` setter is gated on the wasmtime `gc` cargo feature
///   (which we don't enable), so we cannot call it explicitly. The
///   feature stays on by default; a future LTS that flips that default
///   would break component instantiation noisily — `Component::new`
///   would fail — which is the desired loudness.
///
/// Enabled (correctness):
/// - `cranelift_nan_canonicalization` — converts every produced NaN
///   to a single canonical bit pattern. f32/f64 are still byte-banned
///   by the float-ban lint, but NaN canonicalization defends in depth
///   in case a future host or cap admits a float-typed input/output.
/// - `consume_fuel` — required for the fuel-bounded execution gate.
/// - `wasm_component_model` — we only ever load components.
#[must_use]
pub fn deterministic_config() -> Config {
    let mut config = Config::new();
    config
        .wasm_simd(false)
        .wasm_relaxed_simd(false)
        .wasm_memory64(false)
        .wasm_multi_memory(false)
        .wasm_bulk_memory(true)
        .wasm_multi_value(true)
        .cranelift_nan_canonicalization(true)
        .consume_fuel(true)
        .wasm_component_model(true);
    config
}

impl WasmtimeBackend {
    /// Build a new backend with the deterministic engine config per
    /// determinism.md §5.2 + §5.3 (see [`deterministic_config`]).
    ///
    /// Float-ban is a byte-level lint enforced before instantiation
    /// (see [`scan_component_for_floats`]); we belt-and-brace it with
    /// `cranelift_nan_canonicalization(true)` so a future host that
    /// accepts a float-typed input cannot leak a non-canonical NaN.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Instantiation`] if the Wasmtime engine
    /// cannot be constructed with the requested config.
    pub fn new() -> Result<Self, BackendError> {
        let config = deterministic_config();
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
                .memory_size(COMPONENT_MEMORY_CAP_V1)
                .build(),
        };
        let mut store: Store<HostState> = Store::new(&self.engine, host_state);
        store
            .set_fuel(STATE_APPLY_FUEL_BUDGET_V1)
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
