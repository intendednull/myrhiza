//! `IdentityStore` trait + `IdentityError` enum.
//!
//! Pluggable identity backends. The kernel runtime is filesystem-
//! unaware: callers obtain a `PeerKeypair` / `AuthorKeypair` from
//! whichever `IdentityStore` they want (filesystem, future HSM,
//! future encrypted-at-rest) and hand them to [`crate::Runtime::start`].
//!
//! Per plan B-2 spec §5 + §7.

use crate::identity::{AuthorKeypair, PeerKeypair};
use async_trait::async_trait;
use myrhiza_types::AuthorPubkey;
use std::path::PathBuf;

/// Errors surfaced by [`IdentityStore`] implementations.
///
/// `HrpMismatch` / `Bech32Decode` apply to filename parsing only —
/// secret-file content is raw bytes with no HRP (per spec §4.1).
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// A filesystem read or write failed.
    #[error("io error at {path}: {source}")]
    Io {
        /// Path the operation targeted.
        path: PathBuf,
        /// Underlying [`std::io::Error`].
        #[source]
        source: std::io::Error,
    },

    /// A bech32m-encoded filename failed to decode.
    #[error("bech32 decode failed for {input:?}: {source}")]
    Bech32Decode {
        /// The string that failed to decode.
        input: String,
        /// Underlying [`bech32::DecodeError`].
        #[source]
        source: bech32::DecodeError,
    },

    /// A bech32m string decoded but used the wrong human-readable part.
    #[error("HRP mismatch in {input:?}: expected '{expected}', got '{actual}'")]
    HrpMismatch {
        /// The string that decoded successfully but had the wrong HRP.
        input: String,
        /// The HRP this context required (e.g. `"wuser"`).
        expected: &'static str,
        /// The HRP actually present in `input`.
        actual: String,
    },

    /// A secret-file's content was not exactly 32 bytes.
    #[error("expected 32-byte seed at {path}, got {actual} bytes")]
    SeedLengthMismatch {
        /// Path of the offending secret file.
        path: PathBuf,
        /// Actual length read.
        actual: usize,
    },

    /// The pubkey embedded in an author filename does not match the
    /// pubkey derived from the secret-file content (tampered filename).
    #[error("author pubkey mismatch at {path}: filename {requested}, derived {actual}")]
    AuthorPubkeyMismatch {
        /// Path of the offending file.
        path: PathBuf,
        /// Bech32m pubkey embedded in the filename.
        requested: String,
        /// Bech32m pubkey derived from the file's secret bytes.
        actual: String,
    },

    /// On Unix, a secret file's mode bits were looser than the
    /// store's requirement (typically 0600).
    #[error("insecure permissions on {path}: mode 0{mode:o}, expected ≤ 0{expected:o}")]
    InsecurePermissions {
        /// Path of the offending file.
        path: PathBuf,
        /// Actual mode bits observed.
        mode: u32,
        /// Maximum mode bits this store accepts.
        expected: u32,
    },

    /// A file in `authors/` did not match the expected
    /// `wuser1*.key` pattern.
    #[error("invalid filename in authors/: {0}")]
    InvalidAuthorFilename(String),

    /// The identity directory did not exist and could not be created.
    #[error("identity dir does not exist and could not be created: {path}: {source}")]
    DirCreate {
        /// Directory path that could not be created.
        path: PathBuf,
        /// Underlying [`std::io::Error`].
        #[source]
        source: std::io::Error,
    },
}

/// A pluggable identity backend.
///
/// Production = filesystem (`super::FilesystemIdentityStore`).
/// Tests may use the in-memory keypair constructors directly without
/// going through a store. Future impls: encrypted-at-rest, HSM, OS
/// keyring — added without breaking changes per spec §2.
///
/// The trait is intentionally small: B-2 only needs load/create.
/// Author-key deletion, peer-key rotation, and audit-log surface are
/// out of scope (spec §12).
#[async_trait]
pub trait IdentityStore: Send + Sync {
    /// Load the peer's keypair, generating + persisting a fresh one
    /// if no peer key exists in the store.
    ///
    /// # Errors
    /// Returns [`IdentityError::Io`] for filesystem failures,
    /// [`IdentityError::InsecurePermissions`] on Unix if `peer.key`
    /// has looser-than-0600 mode bits, or
    /// [`IdentityError::SeedLengthMismatch`] if an existing key file
    /// is not exactly 32 bytes.
    async fn load_or_create_peer(&self) -> Result<PeerKeypair, IdentityError>;

    /// Load an author keypair by its public key.
    ///
    /// # Errors
    /// Returns [`IdentityError::Io`] if the file is missing,
    /// [`IdentityError::AuthorPubkeyMismatch`] if the filename's
    /// embedded pubkey does not match the derived pubkey (tampered
    /// filename), or [`IdentityError::SeedLengthMismatch`] /
    /// [`IdentityError::InsecurePermissions`] as for `load_or_create_peer`.
    async fn load_author(&self, pk: &AuthorPubkey) -> Result<AuthorKeypair, IdentityError>;

    /// Generate + persist a fresh author keypair.
    ///
    /// # Errors
    /// Returns [`IdentityError::Io`] if the file cannot be written.
    async fn create_author(&self) -> Result<AuthorKeypair, IdentityError>;

    /// List all author public keys in the store, sorted.
    ///
    /// # Errors
    /// Returns [`IdentityError::Io`] for directory-read failures or
    /// [`IdentityError::InvalidAuthorFilename`] if a file in
    /// `authors/` does not match the `wuser1*.key` pattern.
    async fn list_authors(&self) -> Result<Vec<AuthorPubkey>, IdentityError>;
}
