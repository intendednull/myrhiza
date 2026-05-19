//! `IdentityStore` trait + `IdentityError` enum.
//!
//! Pluggable identity backends. The kernel runtime is filesystem-
//! unaware: callers obtain a `PeerKeypair` / `AuthorKeypair` from
//! whichever `IdentityStore` they want (filesystem, future HSM,
//! future encrypted-at-rest) and hand them to [`crate::Runtime::start`].
//!
//! Per plan B-2 spec §5 + §7.

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
