//! Backward-compat shim. `EventBuilder` now lives at
//! `myrhiza_kernel::event_builder`.

pub use myrhiza_kernel::event_builder::{
    EventBuilder, canonical_envelope, counter_increment_payload,
};
pub use myrhiza_kernel::identity::AuthorKeypair;
