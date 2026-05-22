//! Shared test fixtures + doubles for the Myrhiza workspace.
//!
//! Per verification.md §22.8. Dev-only crate; never depend on
//! production paths. Plan A populates manifest + bundle helpers;
//! plan B adds mem-network double; plan C adds proptest generators.

pub mod bundle;
pub mod event_builder;
pub mod harness;
pub mod manifest;

// Re-export from canonical home in myrhiza_kernel for backward-compat.
pub use harness::{InProcessHarness, PeerHandle};
pub use myrhiza_kernel::event_builder::{
    EventBuilder, canonical_envelope, counter_increment_payload,
};
pub use myrhiza_kernel::identity::AuthorKeypair;
