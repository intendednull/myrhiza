//! Wasmtime backend for the Myrhiza runtime.
//!
//! Implements [`myrhiza_backend::Backend`] using Wasmtime's component
//! model. Capability gating is enforced at linker construction time
//! (only allowed imports are bound) plus a per-call interception
//! wrapper for high-value ops.

#![deny(missing_docs)]

mod engine;
mod float_ban;
mod gating;
mod helpers;
mod instance;
mod interaction_instance;
mod propose_instance;

pub use engine::{HostState, WasmtimeBackend, deterministic_config};
pub use float_ban::{scan_component_for_floats, scan_core_module_for_floats};
pub use gating::{
    interaction_ambient_set, interaction_bound_imports, state_apply_ambient_set,
    state_apply_bound_imports, state_propose_ambient_set, state_propose_bound_imports,
    validate_interaction_manifest, validate_state_apply_manifest, validate_state_propose_manifest,
    wire_interaction_linker, wire_state_apply_linker, wire_state_propose_linker,
};
pub use helpers::{
    LogLevel, LogSink, host_hash_impl, host_now_hlc_from_event_impl, host_verify_signature_impl,
};
