//! Wasmtime engine + Backend impl.
//!
//! `wasmtime::component::bindgen!` generates host trait skeletons
//! and a `StateApply` instantiation type from the state-apply WIT
//! world. Plan A binds the deterministic helper set manually via
//! per-method `linker.func_wrap` (see [`crate::gating::wire_linker`])
//! so the kernel can intersect the manifest-declared subset against
//! the ambient set at link time, rejecting components that import
//! anything outside that subset.

use std::sync::Arc;

use myrhiza_backend::{
    Backend, BackendError, ComponentInstance, InteractionInstance, ProposeInstance,
};
use myrhiza_manifest::Manifest;
use myrhiza_types::limits::{COMPONENT_MEMORY_CAP_V1, MAX_WASM_STACK_V1};
use wasmtime::{
    Config, Engine, Store,
    component::{Component, Linker, ResourceTable},
};

use crate::Profile;
use crate::float_ban::scan_component_for_floats;
use crate::gating::{bound_imports, validate_manifest, wire_linker};
use crate::helpers::LogSink;
use crate::instance::StateApplyInstance;
use crate::interaction_instance::InteractionInstanceImpl;
use crate::propose_instance::StateProposeInstance;

/// WIT instance name for the deterministic-helper interface bound to
/// state-apply per architecture.md §3.5. Pre-walking the component's
/// imports against this name is how we deterministically attribute
/// `UnauthorizedImport` errors before reaching the linker (which
/// otherwise surfaces those failures as a string-typed
/// `wasmtime::Error`).
const HOST_DETERMINISTIC_INSTANCE: &str = "myrhiza:kernel/host-deterministic@1.0.0";

/// WIT instance name for the shared types interface. The state-apply
/// world `use`s `verdict` / `hlc` / `key-handle` / `log-level` from
/// this interface, which surfaces as a top-level component import of
/// the types instance. The instance is types-only by design — no
/// callable functions — so the pre-walk permits its `Type` and
/// `Resource` items but rejects any callable (`ComponentFunc`) or
/// nested-instance / module / core-func item that would surface a
/// non-type capability via this instance. A future WIT bump that adds
/// a method to one of `types.wit`'s resources (`identity-handle`,
/// `peer-handle`, `key-handle`, `request-token`) would land that
/// method as a `ComponentFunc` item inside this instance with a
/// mangled name (e.g. `[method]key-handle.fingerprint`); rejecting all
/// callables here means such a future bump fails closed instead of
/// silently bypassing the deterministic-helper gate.
const SHARED_TYPES_INSTANCE: &str = "myrhiza:kernel/types@1.0.0";

/// WIT instance name for the ui-surfaces interface. Interaction world
/// `use`s ui-element / panel / button / etc. from this interface, which
/// surfaces as a top-level component import. The instance is types-only
/// in v1 — no callable functions — so the prewalk permits `Type` and
/// `Resource` items and rejects any `ComponentFunc`. Mirrors the rule
/// already applied to `myrhiza:kernel/types@1.0.0`.
pub(crate) const HOST_UI_SURFACES_INSTANCE: &str = "myrhiza:kernel/host-ui-surfaces@1.0.0";

/// `host.log` is unconditionally bound per determinism.md §5.1 — the
/// manifest does not need to declare it. The pre-walk treats it as
/// always-on so a manifest that omits it doesn't cause a spurious
/// `UnauthorizedImport`.
fn is_always_on_helper(cap: &str) -> bool {
    cap == "host.log"
}

/// Walk the component's top-level instance imports against
/// `bound_imports` (the manifest-allowed subset of the deterministic
/// helper ambient set, plus the always-on `host.log`). Returns
/// [`BackendError::UnauthorizedImport`] for the first import that the
/// `profile`'s ambient set does not provide.
///
/// This pre-walk replaces the previous string-match fallback in the
/// bindgen-instantiate path: by enumerating
/// [`wasmtime::component::Component::component_type().imports()`]
/// before linker construction we can attribute capability rejections
/// deterministically without depending on the wording of wasmtime's
/// link-error messages.
///
/// Four kinds of unauthorized imports surface:
/// 1. An unknown WIT instance (anything other than
///    `myrhiza:kernel/host-deterministic@1.0.0`,
///    `myrhiza:kernel/types@1.0.0`, or — for the interaction profile
///    only — `myrhiza:kernel/host-ui-surfaces@1.0.0`). The error
///    carries the full versioned WIT instance name so logs are
///    unambiguous.
/// 2. A known-instance function whose vocabulary-mapped name
///    (`host.<wit-fn-name>`) is not in `bound_imports` and is not the
///    always-on helper (`host.log`). The error carries the
///    vocabulary-style name `host.<fn-name>` to match the manifest's
///    declaration vocabulary.
/// 3. Any non-`ComponentFunc` item appearing inside the
///    `host-deterministic` instance (e.g. a resource, type, or
///    nested instance). The WIT worlds surface only function imports
///    inside this instance, so anything else means a future WIT bump
///    has surfaced a non-function capability we haven't audited;
///    reject fail-closed rather than silently skipping it. The error
///    carries the qualified
///    `host-deterministic-instance.<item-name>` so the audit point is
///    obvious in logs.
/// 4. Any callable item appearing inside a types-only instance
///    (`types@1.0.0` or, for interaction, `host-ui-surfaces@1.0.0`).
///    The types-only instances are types-only by design — only
///    `Type` and `Resource` items are permitted. A future WIT bump
///    that adds a method to a resource would surface as a
///    `ComponentFunc` item inside this instance (with a mangled
///    `[method]resource.method-name` shape) and bypass the
///    deterministic-helper gate; rejecting all callables here closes
///    that hole. The error carries the qualified
///    `<instance-name>.<item-name>`.
///
///    A regression fixture exercising a synthetic component whose
///    `host-deterministic@1.0.0` instance carries a non-function
///    item (resource type / nested instance) is deferred to plan B
///    — building one requires a custom WIT package fork, since the
///    production WIT cannot be twisted into that shape. The
///    inverted predicate above is the load-bearing security change;
///    the existing test suite confirms legitimate components and the
///    types-only allowlist still instantiate.
pub(crate) fn prewalk_imports(
    engine: &Engine,
    component: &Component,
    bound_imports: &std::collections::BTreeSet<String>,
    profile: Profile,
) -> Result<(), BackendError> {
    use wasmtime::component::types::ComponentItem;

    let allow_ui_surfaces = profile.allow_ui_surfaces();
    let component_type = component.component_type();
    for (import_name, item) in component_type.imports(engine) {
        // types-only instances: SHARED_TYPES_INSTANCE and (for interaction)
        // HOST_UI_SURFACES_INSTANCE. Walk contents: Type/Resource allowed,
        // any callable rejected.
        let is_types_only = import_name == SHARED_TYPES_INSTANCE
            || (allow_ui_surfaces && import_name == HOST_UI_SURFACES_INSTANCE);

        if is_types_only {
            let ComponentItem::ComponentInstance(inst) = item else {
                return Err(BackendError::UnauthorizedImport(import_name.into()));
            };
            for (item_name, child_item) in inst.exports(engine) {
                match child_item {
                    ComponentItem::Type(_) | ComponentItem::Resource(_) => {}
                    ComponentItem::ComponentFunc(_)
                    | ComponentItem::CoreFunc(_)
                    | ComponentItem::Module(_)
                    | ComponentItem::Component(_)
                    | ComponentItem::ComponentInstance(_) => {
                        return Err(BackendError::UnauthorizedImport(format!(
                            "{import_name}.{item_name}"
                        )));
                    }
                }
            }
            continue;
        }

        if import_name != HOST_DETERMINISTIC_INSTANCE {
            return Err(BackendError::UnauthorizedImport(import_name.into()));
        }

        let ComponentItem::ComponentInstance(inst) = item else {
            return Err(BackendError::UnauthorizedImport(import_name.into()));
        };
        for (item_name, child_item) in inst.exports(engine) {
            if !matches!(child_item, ComponentItem::ComponentFunc(_)) {
                return Err(BackendError::UnauthorizedImport(format!(
                    "{import_name}.{item_name}"
                )));
            }
            let cap = format!("host.{item_name}");
            if !bound_imports.contains(&cap) && !is_always_on_helper(&cap) {
                return Err(BackendError::UnauthorizedImport(cap));
            }
        }
    }
    Ok(())
}

wasmtime::component::bindgen!({
    path: "../../wit/myrhiza-kernel/wit",
    world: "state-apply",
});

/// Bindgen-generated bindings for the `state-propose` WIT world.
/// Scoped in a separate module to avoid collision with the
/// `myrhiza::kernel::types` module generated by the state-apply bindgen
/// above — both worlds share the `types` interface, which produces
/// identically-named items in each module. Scoping separates the two
/// generated namespaces; `propose_instance.rs` imports `StatePropose`
/// from here.
pub(crate) mod propose_bindings {
    wasmtime::component::bindgen!({
        path: "../../wit/myrhiza-kernel/wit",
        world: "state-propose",
    });
}

/// Bindgen-generated bindings for the `interaction` WIT world.
/// Scoped in its own module for the same reason as `propose_bindings`:
/// the interaction world shares the `types` interface with the other
/// worlds, producing identically-named items; isolation prevents
/// collisions. `interaction_instance.rs` imports `Interaction` from here.
pub(crate) mod interaction_bindings {
    wasmtime::component::bindgen!({
        path: "../../wit/myrhiza-kernel/wit",
        world: "interaction",
    });
}

/// Per-instance host state held in the Wasmtime `Store`.
///
/// Wired into the Wasmtime store via [`Store::new`] in
/// `WasmtimeBackend::prepare_instance` and consulted from inside
/// `func_wrap` closures in [`crate::gating::wire_linker`].
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
/// component model. Each `instantiate_*` call builds a fresh `Store`
/// + `Linker` + `Component` from this engine.
pub struct WasmtimeBackend {
    engine: Engine,
}

/// Build the deterministic `wasmtime::Config` used for every engine
/// construction in the runtime per determinism.md §5.2 + §5.3.
///
/// Wasmtime's defaults shift across LTS bumps (SIMD, relaxed-SIMD,
/// threads, memory64, multi-memory have all flipped between releases),
/// and several proposal flags also vary across architectures inside a
/// single LTS — `wasm_tail_call` is on by default for `x86_64` /
/// `aarch64` / `riscv64` cranelift but off for s390x and Winch, which is a silent
/// cross-arch divergence vector. Replay determinism requires that two
/// peers running the same kernel major produce identical post-state for
/// the same event sequence — which means every engine must accept and
/// reject the same set of instructions. We therefore pin each feature
/// flag explicitly rather than relying on whatever the wasmtime build
/// defaults to, and we exhaustively pin every feature method wasmtime
/// 36 exposes that the workspace `wasmtime` cargo features admit.
///
/// Disabled (nondeterministic / NaN-bit-divergent / cross-peer drift):
/// - `wasm_simd` — `v128` ops include float lanes; bit patterns vary.
/// - `wasm_relaxed_simd` — explicitly nondeterministic by design.
/// - `wasm_memory64` — orthogonal pointer width; not in v1.
/// - `wasm_multi_memory` — orthogonal store layout; not in v1.
///
/// Disabled (default differs across arch / LTS — pinning closes the
/// silent divergence window):
/// - `wasm_tail_call` — default is on for `x86_64` / `aarch64` /
///   `riscv64` cranelift but off for s390x / Winch in wasmtime 36; the
///   float-ban whitelist also rejects `return_call` /
///   `return_call_indirect` so this is belt-and-brace. Tail calls are
///   not in v1.
/// - `wasm_extended_const` — default is on in wasmtime 36's WASM2
///   baseline; pin off because v1 components don't use the extended
///   constant-expression vocabulary in globals / data segments and we
///   want a tight surface.
///
/// Disabled (proposals not in v1; pinned defensively even though
/// either off-by-default or already covered by the float-ban):
/// - `wasm_custom_page_sizes` — proposal not in v1.
/// - `wasm_wide_arithmetic` — integer-only and deterministic, but not
///   in the v1 ABI; pin off so adding it later is a deliberate spec
///   bump rather than a default flip.
/// - `wasm_stack_switching` — control-flow proposal not in v1.
/// - `wasm_exceptions` — exception handling is a non-determinism
///   vector and is `doc(hidden)` in wasmtime 36; pin off explicitly.
///
/// Off at build time (no `Config` runtime call needed — strictly
/// stronger than a runtime pin):
/// - `wasm_threads` — the `wasm_threads` `Config` method is gated on
///   the wasmtime `threads` cargo feature, which we deliberately do
///   not enable in the workspace `wasmtime` dep. Threads are therefore
///   off at compile time.
/// - `wasm_function_references` and `wasm_gc` — both `Config` setters
///   are gated on the wasmtime `gc` cargo feature, which the workspace
///   does not enable. Both proposals are therefore off at compile
///   time. (Their feature bits — `FUNCTION_REFERENCES`, `GC` — also
///   participate in `WasmFeatures::WASM2`'s conditional set tied to
///   `cfg!(feature = "gc")`, so the engine's accepted feature set
///   does not include them.) Adding them later would require enabling
///   the `gc` cargo feature, which is itself a deliberate spec bump.
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
/// Pinned (resource):
/// - `max_wasm_stack` — pinned to [`MAX_WASM_STACK_V1`] (512 KiB) per
///   determinism.md §5.3. Wasmtime 36's default also happens to be
///   512 KiB, but pinning the bytes here means a future LTS bump
///   cannot silently change the wasm stack ceiling and shift trap
///   boundaries on deeply recursive components.
///
/// Enabled (correctness):
/// - `cranelift_nan_canonicalization` — converts every produced NaN
///   to a single canonical bit pattern. f32/f64 are still byte-banned
///   by the float-ban lint, but NaN canonicalization defends in depth
///   in case a future host or cap admits a float-typed input/output.
/// - `consume_fuel` — required for the fuel-bounded execution gate.
/// - `wasm_component_model` — we only ever load components.
///
/// Pinned (codegen):
/// - `cranelift_opt_level=Speed` — wasmtime 36's default also happens
///   to be `Speed`, but opt-level participates in cranelift's
///   instruction-selection pipeline (constant folding can elide trap
///   sites, register allocation order can shift fault-instruction
///   positions). A future LTS bump that flips the default to
///   `SpeedAndSize` (or any peer building with a non-default
///   `WASMTIME_OPT_LEVEL` env override that filters into Config
///   construction) would shift trap boundaries in pathological
///   components. Pinning here closes that silent divergence window.
/// - `strategy=Cranelift` — wasmtime's `Strategy::Auto` default
///   prefers Cranelift when the `cranelift` cargo feature is present,
///   which the workspace currently enables. A future workspace edit
///   dropping the `cranelift` cargo feature (or adding a Pulley-only
///   browser path that's not properly cargo-feature-isolated) would
///   silently switch the codegen backend, producing a different
///   trap-instruction set and potentially different trap boundaries.
///   Explicitly pinning the strategy defends in depth: the engine
///   either compiles with Cranelift or fails to construct, never
///   falling through to a different backend by default.
#[must_use]
pub fn deterministic_config() -> Config {
    let mut config = Config::new();
    config
        // Nondeterministic / NaN-divergent / not-in-v1.
        .wasm_simd(false)
        .wasm_relaxed_simd(false)
        .wasm_memory64(false)
        .wasm_multi_memory(false)
        // Cross-arch / LTS-default-flip surface — pin explicitly.
        .wasm_tail_call(false)
        .wasm_extended_const(false)
        // Proposals not in v1; pinned defensively.
        .wasm_custom_page_sizes(false)
        .wasm_wide_arithmetic(false)
        .wasm_stack_switching(false)
        .wasm_exceptions(false)
        // Component-model load-bearing — keep on defensively.
        .wasm_bulk_memory(true)
        .wasm_multi_value(true)
        // Resource pin per determinism.md §5.3.
        .max_wasm_stack(MAX_WASM_STACK_V1)
        // Correctness.
        .cranelift_nan_canonicalization(true)
        .consume_fuel(true)
        .wasm_component_model(true)
        // Codegen pin — defaults match today, but opt-level
        // participates in trap-site placement so pin to close the
        // cross-LTS / env-override divergence window.
        .cranelift_opt_level(wasmtime::OptLevel::Speed)
        // Strategy pin — defends against a future workspace edit
        // dropping the `cranelift` cargo feature or adding a non-
        // Cranelift backend; Strategy::Auto would silently switch,
        // shifting the trap-instruction set across peers.
        .strategy(wasmtime::Strategy::Cranelift);
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

    /// Shared instantiation prep for all three profiles.
    ///
    /// Runs the mechanically identical steps each `instantiate_*` trait
    /// method needs before invoking the profile-specific bindgen
    /// `Instance::instantiate` constructor:
    ///
    /// 1. Manifest gating check (`validate_manifest`).
    /// 2. Float-ban lint — only when [`Profile::float_ban_applies`].
    /// 3. Compute the bound import set.
    /// 4. Decode the component.
    /// 5. Pre-walk imports against the bound set.
    /// 6. Build the linker, binding ONLY the allowed imports.
    /// 7. Build the store with profile-specific fuel budget + memory cap.
    ///
    /// Returns the prepared `(Store, Component, Linker)` triple ready
    /// to be fed into the profile-specific bindgen
    /// `Instance::instantiate(...)` call. That final step differs by
    /// profile because the bindgen-generated types differ
    /// (`StateApply` vs `StatePropose` vs `Interaction`); the rest of
    /// the prep is byte-identical across profiles modulo the
    /// `Profile`-derived knobs above.
    fn prepare_instance(
        &self,
        profile: Profile,
        component_bytes: &[u8],
        manifest: &Manifest,
    ) -> Result<(Store<HostState>, Component, Linker<HostState>), BackendError> {
        // 1. Manifest gating check — declared imports must be a
        //    subset of the profile's ambient set.
        validate_manifest(manifest, profile)?;

        // 2. Float-ban lint — only applied for state-apply.
        if profile.float_ban_applies() {
            scan_component_for_floats(component_bytes).map_err(BackendError::BannedInstruction)?;
        }

        // 3. Compute the bound import set (manifest ∩ ambient).
        let bound = bound_imports(manifest, profile);

        // 4. Decode the component.
        let component = Component::from_binary(&self.engine, component_bytes)
            .map_err(|e| BackendError::Instantiation(format!("decode component: {e}")))?;

        // 5. Pre-walk component imports against the bound set so
        //    unauthorized imports surface as a typed
        //    `BackendError::UnauthorizedImport` instead of relying on
        //    the wording of the linker's instantiation error.
        prewalk_imports(&self.engine, &component, &bound, profile)?;

        // 6. Build the linker, binding ONLY the allowed imports.
        let mut linker: Linker<HostState> = Linker::new(&self.engine);
        wire_linker(&mut linker, &bound, profile)?;

        // 7. Build the store with profile-specific fuel budget +
        //    memory cap per determinism.md §5.3.
        let host_state = HostState {
            log_sink: Arc::new(LogSink::default()),
            bound_imports: bound,
            table: ResourceTable::new(),
            limits: wasmtime::StoreLimitsBuilder::new()
                .memory_size(COMPONENT_MEMORY_CAP_V1)
                .build(),
        };
        let mut store: Store<HostState> = Store::new(&self.engine, host_state);
        store
            .set_fuel(profile.fuel_budget())
            .map_err(|e| BackendError::Instantiation(format!("set_fuel: {e}")))?;
        // Enforce the memory cap via the StoreLimits the host_state
        // already carries. `memory.grow` past the cap returns the
        // wasm-level "out of memory" sentinel, which surfaces as a
        // trap in typed function calls.
        store.limiter(|s| &mut s.limits);

        Ok((store, component, linker))
    }
}

impl Backend for WasmtimeBackend {
    fn instantiate_state_apply(
        &self,
        component_bytes: &[u8],
        manifest: &Manifest,
    ) -> Result<Box<dyn ComponentInstance>, BackendError> {
        let (store, component, linker) =
            self.prepare_instance(Profile::StateApply, component_bytes, manifest)?;
        let instance = StateApplyInstance::instantiate(store, &component, &linker)?;
        Ok(Box::new(instance))
    }

    fn instantiate_state_propose(
        &self,
        component_bytes: &[u8],
        manifest: &Manifest,
    ) -> Result<Box<dyn ProposeInstance>, BackendError> {
        let (store, component, linker) =
            self.prepare_instance(Profile::StatePropose, component_bytes, manifest)?;
        let instance = StateProposeInstance::instantiate(store, &component, &linker)?;
        Ok(Box::new(instance))
    }

    fn instantiate_interaction(
        &self,
        component_bytes: &[u8],
        manifest: &Manifest,
    ) -> Result<Box<dyn InteractionInstance>, BackendError> {
        let (store, component, linker) =
            self.prepare_instance(Profile::Interaction, component_bytes, manifest)?;
        let instance = InteractionInstanceImpl::instantiate(store, &component, &linker)?;
        Ok(Box::new(instance))
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
