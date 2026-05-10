//! Manifest TOML schema, canonical encoding, and signature verification
//! for Myrhiza bundles.
//!
//! Per [distribution.md §10.2]:
//! - Parse manifest TOML with `toml_edit 0.22.x` (pinned).
//! - Convert to typed `Manifest` struct.
//! - Canonical encoding = bincode 1.3.x (via
//!   `myrhiza_types::canonical_bincode`) over the typed struct.
//! - BLAKE3 the encoded bytes → `manifest_canonical_hash`.
//! - Author signs `manifest_canonical_hash` + `content_hash` + `version`
//!   + `author_pubkey` (length-prefixed framing per §10.2).

#![deny(missing_docs)]

pub mod vocabulary;

pub mod schema;
pub use schema::*;

pub mod parse;
pub use parse::{ParseError, parse_manifest};

pub mod canonical;
pub use canonical::{
    DOMAIN_SEP, length_prefix_concat, manifest_canonical_hash, signed_body_bytes,
    signing_target_bytes,
};

pub mod signature;
pub use signature::{SignatureError, verify_signature};
