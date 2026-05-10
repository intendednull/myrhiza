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
