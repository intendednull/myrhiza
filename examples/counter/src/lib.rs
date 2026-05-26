//! Counter example app — canonical first-app demo.
//!
//! Three component slots (state-apply, state-propose, interaction)
//! wired through `myrhiza-sdk`'s `myrhiza_app!` macro. See
//! `manifest.rs` for the bundle manifest authored via the
//! `manifest!` macro.
//!
//! This crate's lib is a host-side marker — it only carries the
//! `manifest` module (gated to non-wasm32 targets). The three
//! components are `[[bin]]` artifacts gated by `required-features`.
//! Build a component via
//! `cd examples/counter && cargo build --target wasm32-unknown-unknown --features <slot> --bin counter-<slot>`.
//!
//! Per docs/specs/2026-05-26-b-8-sdk-design.md §3.3.

#[cfg(not(target_arch = "wasm32"))]
#[path = "../manifest.rs"]
pub mod manifest;
