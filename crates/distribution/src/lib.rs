//! Bundle distribution: iroh-blobs-backed publish + fetch + per-author
//! revocation and publication topic schema + monotonic-seq state
//! machines.
//!
//! Per B-10 design at
//! `docs/specs/2026-05-26-b-10-bundle-distribution-design.md`.
//!
//! ## Feature gates
//!
//! - `network-iroh` (default-off): pulls in `iroh` + `iroh-blobs`
//!   and unlocks `BundleDistribution` (publish + fetch). The pure-
//!   function state machines (`RevocationLog`, `PublicationLog`)
//!   compile feature-free — they're used by every install regardless
//!   of transport.

#![deny(missing_docs)]

pub mod conversions;
pub mod revocation;
pub mod topic;

pub use revocation::{
    DOMAIN_SEP_REVOCATION, MAX_REASON_LEN, MAX_REVOCATION_JUMP, RevocationError, RevocationEvent,
    RevocationLog,
};

// PublicationLog lands in T6.
// Iroh-blobs publish + fetch lands in T7.
