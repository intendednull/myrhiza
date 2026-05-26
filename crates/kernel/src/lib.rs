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
pub use install::{InstallError, InstallFlow, LoadedBundle};

// `BundleAddress` moved to `crates/types/` in B-10 T8 per spec §4.6
// declared dep direction (so `crates/distribution::BundleDistribution::fetch`
// can return `MaterializedBundle { address: BundleAddress::Disk { .. } }`
// without inducing a `distribution -> kernel` dep cycle).
// Re-exported from this location for backwards compatibility.
pub use myrhiza_types::BundleAddress;

pub mod state_apply;
pub use state_apply::{ApplyError, ApplyOutcome, ApplyResult, PreCheckResult, StateApplyHandle};

pub mod digest;
pub use digest::{DigestEmitter, DigestEvent};

pub mod identity;
pub use identity::{
    AuthorKeypair, FilesystemIdentityStore, IdentityError, IdentityStore, PeerKeypair,
};

pub mod dag;
pub use dag::{AuthorChain, DagError, EventDag, Inserted};

pub mod pending;

pub mod drift;

pub mod runtime;

pub mod event_builder;
pub use event_builder::{EventBuilder, canonical_envelope, counter_increment_payload};

pub mod state_propose;
pub use state_propose::{ProposeError, StateProposeHandle};

pub mod interaction;
pub use interaction::{InteractionError, InteractionHandle};

// Re-export the `BundleDistribution::fetch` API surface so embedders
// (including the CLI) reach iroh-blob bundle materialization through
// `myrhiza_kernel::...`. The kernel itself does not call `fetch` — the
// embedder threads a `MaterializedBundle` into `InstallFlow::load`
// per spec §3.5 (the kernel's role is to dispatch on `BundleAddress`
// and run the existing disk-load path).
#[cfg(feature = "network-iroh")]
pub use myrhiza_distribution::{BundleDistribution, FetchError, MaterializedBundle, PublishError};
