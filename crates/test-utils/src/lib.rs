//! Shared test fixtures + doubles for the Myrhiza workspace.
//!
//! Per verification.md §22.8. Dev-only crate; never depend on
//! production paths. Plan A populates manifest + bundle helpers;
//! plan B adds mem-network double; plan C adds proptest generators.

pub mod bundle;
pub mod event_builder;
pub mod manifest;

pub use event_builder::{
    AuthorKeypair, EventBuilder, canonical_envelope, counter_increment_payload,
};
