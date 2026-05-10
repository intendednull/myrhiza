//! Shared core types for the Myrhiza runtime.
//!
//! This crate is a leaf in the workspace dependency graph and contains
//! no I/O, no crypto beyond BLAKE3 hashing, and no host bindings.

pub mod encoding;
pub use encoding::{CanonicalOptions, canonical_bincode};

pub mod hash;
pub use hash::{BundleHash, EventHash};

pub mod hlc;
pub use hlc::Hlc;

pub mod author;
pub mod topic;
pub use author::AuthorPubkey;
pub use topic::Topic;
