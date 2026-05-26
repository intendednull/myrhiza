//! Myrhiza SDK — application author surface.
//!
//! See `myrhiza_sdk::prelude::*` for the common imports. App authors
//! depend on this crate exclusively; the kernel-internal crates
//! (`myrhiza-kernel`, `myrhiza-backend`, `myrhiza-wasmtime-backend`,
//! `myrhiza-network`) are NOT permitted dependencies for `examples/*`
//! members per the dep-direction CI check landed alongside this SDK.
//!
//! Per docs/specs/2026-05-26-b-8-sdk-design.md.

#![cfg_attr(target_arch = "wasm32", no_std)]
#![deny(missing_docs)]

#[cfg(target_arch = "wasm32")]
extern crate alloc;

pub mod macros;

// Manifest authoring surface (host-only): re-exports `myrhiza-manifest`
// schema types and the `manifest!` declarative macro's prelude. Gated
// to non-wasm32 because `myrhiza-manifest` is std-only — pulling its
// symbols into the SDK's rlib on wasm32 transitively loads `std`,
// which conflicts with the consumer's `#![no_std]` + `#[panic_handler]`.
#[cfg(not(target_arch = "wasm32"))]
pub mod manifest;
#[cfg(not(target_arch = "wasm32"))]
pub mod prelude;
#[cfg(not(target_arch = "wasm32"))]
pub mod types;

// Boilerplate is only relevant on wasm32 targets — re-exposed via the
// `myrhiza_app!` macro, not directly consumed. See spec §3.1.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub mod __boilerplate;
