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
