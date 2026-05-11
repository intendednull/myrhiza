//! Myrhiza runtime kernel.
//!
//! Plan A scope: install flow scaffold, state-apply ABI, state-digest
//! emission stub. No iroh, no event DAG, no apps — those live in
//! plans B and C.
//!
//! Modules land incrementally across tasks 29-31; each task adds its
//! `pub mod` + re-exports so every commit builds clean.

#![deny(missing_docs)]

pub mod install;
pub use install::{BundleAddress, InstallError, InstallFlow, LoadedBundle};

pub mod state_apply;
pub use state_apply::{ApplyError, ApplyOutcome, ApplyResult, PreCheckResult, StateApplyHandle};

pub mod digest;
pub use digest::{DigestEmitter, DigestEvent};

pub mod identity;
pub use identity::{AuthorKeypair, PeerKeypair};
