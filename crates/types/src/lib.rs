//! Shared core types for the Myrhiza runtime.
//!
//! This crate is a leaf in the workspace dependency graph and contains
//! no I/O, no crypto beyond BLAKE3 hashing, and no host bindings.

pub mod encoding;
pub use encoding::{CanonicalOptions, EncodingError, canonical_bincode, decode_canonical};

pub mod serde_helpers;

pub mod hash;
pub use hash::{BundleHash, EventHash};

pub mod hlc;
pub use hlc::Hlc;

pub mod author;
pub mod peer;
pub mod topic;
pub use author::AuthorPubkey;
pub use peer::PeerPubkey;
pub use topic::Topic;

pub mod identity;
pub use identity::{CallingProfile, IdentityScope, InstanceBinding, InstanceKind};

pub mod event;
pub use event::Event;

pub mod dag;
pub use dag::{
    AuthorHead, AuthorSeq, DriftAnchor, DriftMessage, DriftSignedPayload, EventRequest, GenesisV1,
    HeadsRequest, HeadsSummary,
};

pub mod limits;
