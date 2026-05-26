//! `use myrhiza_sdk::prelude::*;` brings the common SDK surface into
//! scope.
//!
//! Includes manifest schema types, common Myrhiza types, and the
//! `manifest!` declarative macro. (`myrhiza_app!` lands in T3.)

pub use crate::manifest;
pub use crate::manifest::{
    AbiSection, AppSection, AuthorIdentityClass, AuthorPolicy, CapabilitiesSection,
    ComponentsSection, DeterminismSection, DriftDetectionSection, HighValueOps, Manifest,
    ModuleDep, ModulesSection, Signature, SignatureAlgorithm, StateDigestFormat,
};
pub use crate::types::{AuthorPubkey, BundleHash, EventHash, Hlc, PeerPubkey};
