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

pub use engine::WasmtimeBackend;
pub use float_ban::{scan_component_for_floats, scan_core_module_for_floats};
pub use gating::{
    state_apply_ambient_set, state_apply_bound_imports, validate_state_apply_manifest,
};
pub use helpers::{
    LogLevel, LogSink, host_hash_impl, host_now_hlc_from_event_impl, host_verify_signature_impl,
};
