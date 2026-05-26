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

pub mod manifest;
pub mod prelude;
pub mod types;
