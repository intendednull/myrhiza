//! Filesystem-backed [`IdentityStore`] implementation + bech32m
//! filename helpers.
//!
//! Per plan B-2 spec §4 + §6. Secrets are raw 32-byte binary files
//! (Willow "no `wsecret` HRP, ever"); public keys embedded in
//! filenames use bech32m with the `wuser` HRP.
//!
//! [`IdentityStore`]: super::IdentityStore

use crate::identity::store::{IdentityError, IdentityStore};
use crate::identity::{AuthorKeypair, PeerKeypair};
use async_trait::async_trait;
use myrhiza_types::AuthorPubkey;
use rand_core::RngCore;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// HRP for an event-author identity public key, per spec §4.3.
///
/// Distinct from `wpub-author` (publisher identity, distribution.md
/// §10.2) so the role is unambiguous on inspection.
pub(super) const HRP_AUTHOR_PK: &str = "wuser";

/// Encode an [`AuthorPubkey`] as a bech32m string with the `wuser` HRP.
///
/// Takes `AuthorPubkey` by value because it is `Copy` (newtype around
/// `[u8; 32]`).
///
/// # Panics
///
/// Cannot panic in practice: bech32m encoding is infallible for a
/// 32-byte payload and the fixed 5-character `wuser` HRP (well below
/// the BIP-173/350 90-character HRP+data limit). A panic would
/// indicate an upstream `bech32` bug.
#[allow(clippy::expect_used)]
pub(super) fn encode_author_pubkey(pk: AuthorPubkey) -> String {
    let hrp = bech32::Hrp::parse_unchecked(HRP_AUTHOR_PK);
    bech32::encode::<bech32::Bech32m>(hrp, pk.as_bytes())
        .expect("bech32m encode of 32 bytes with valid HRP cannot fail")
}

/// Decode a `wuser1...` bech32m string back into an [`AuthorPubkey`].
pub(super) fn decode_author_pubkey(s: &str) -> Result<AuthorPubkey, IdentityError> {
    let (hrp, data) = bech32::decode(s).map_err(|source| IdentityError::Bech32Decode {
        input: s.to_owned(),
        source,
    })?;
    if hrp.as_str() != HRP_AUTHOR_PK {
        return Err(IdentityError::HrpMismatch {
            input: s.to_owned(),
            expected: HRP_AUTHOR_PK,
            actual: hrp.as_str().to_owned(),
        });
    }
    if data.len() != 32 {
        return Err(IdentityError::SeedLengthMismatch {
            path: std::path::PathBuf::from(s),
            actual: data.len(),
        });
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&data);
    Ok(AuthorPubkey::from_bytes(bytes))
}

/// Filesystem-backed [`super::IdentityStore`].
///
/// On-disk layout:
///
/// ```text
/// <dir>/
/// ├── peer.key                 (mode 0600 — raw 32 bytes)
/// └── authors/                 (mode 0700)
///     ├── wuser1<...>.key      (mode 0600 — raw 32 bytes)
///     └── ...
/// ```
///
/// Windows: mode-bit enforcement is `#[cfg(unix)]`; Windows builds
/// compile out the [`super::store::IdentityError::InsecurePermissions`]
/// branch. ACL story is deferred.
pub struct FilesystemIdentityStore {
    dir: PathBuf,
}

impl FilesystemIdentityStore {
    /// Open a filesystem-backed store rooted at `dir`. Creates the
    /// directory (and `authors/` subdirectory) if missing, with mode
    /// 0700 on Unix. On Unix, verifies that an existing `dir` has
    /// mode bits ≤ 0700.
    ///
    /// # Errors
    /// Returns [`IdentityError::DirCreate`] for filesystem failures
    /// during directory creation; [`IdentityError::InsecurePermissions`]
    /// if a pre-existing directory has world-readable bits set (Unix
    /// only).
    pub async fn open(dir: impl Into<PathBuf>) -> Result<Self, IdentityError> {
        let dir = dir.into();
        let dir_for_blocking = dir.clone();
        tokio::task::spawn_blocking(move || open_blocking(&dir_for_blocking))
            .await
            .map_err(|join_err| IdentityError::Io {
                path: dir.clone(),
                source: std::io::Error::other(format!("spawn_blocking panicked: {join_err}")),
            })??;
        Ok(Self { dir })
    }

    pub(super) fn authors_dir(&self) -> PathBuf {
        self.dir.join("authors")
    }

    pub(super) fn peer_key_path(&self) -> PathBuf {
        self.dir.join("peer.key")
    }

    pub(super) fn author_key_path(&self, pk: AuthorPubkey) -> PathBuf {
        self.authors_dir()
            .join(format!("{}.key", encode_author_pubkey(pk)))
    }
}

/// Blocking helper that runs under `spawn_blocking`.
///
/// Creates the dir + `authors/` subdir with mode 0700 (Unix) and
/// verifies a pre-existing dir's mode bits.
fn open_blocking(dir: &Path) -> Result<(), IdentityError> {
    if dir.exists() {
        #[cfg(unix)]
        verify_dir_mode(dir, 0o700)?;
    } else {
        create_dir_with_mode(dir, 0o700)?;
    }
    let authors = dir.join("authors");
    if authors.exists() {
        #[cfg(unix)]
        verify_dir_mode(&authors, 0o700)?;
    } else {
        create_dir_with_mode(&authors, 0o700)?;
    }
    Ok(())
}

// `mode` is consumed only on Unix; the `cfg(unix)` block below sets
// permissions from it. On Windows the parameter is intentionally
// unused but kept in the signature so the call sites stay portable.
#[cfg_attr(not(unix), allow(unused_variables))]
fn create_dir_with_mode(path: &Path, mode: u32) -> Result<(), IdentityError> {
    std::fs::create_dir_all(path).map_err(|source| IdentityError::DirCreate {
        path: path.to_path_buf(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, perms).map_err(|source| IdentityError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

#[cfg(unix)]
fn verify_dir_mode(path: &Path, expected_max: u32) -> Result<(), IdentityError> {
    use std::os::unix::fs::PermissionsExt;
    let md = std::fs::metadata(path).map_err(|source| IdentityError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mode = md.permissions().mode() & 0o777;
    if mode & !expected_max != 0 {
        return Err(IdentityError::InsecurePermissions {
            path: path.to_path_buf(),
            mode,
            expected: expected_max,
        });
    }
    Ok(())
}

/// Atomic write of `seed_bytes` to `path` via a `<path>.tmp` sibling
/// + rename. Mode 0600 on Unix.
///
/// # Panics
///
/// Panics if `path` has no file-name component (e.g. is `/` or empty).
/// All call sites in this module construct `path` from
/// [`FilesystemIdentityStore::peer_key_path`] /
/// [`FilesystemIdentityStore::author_key_path`] /
/// `authors_dir().join(<filename>)`, all of which always yield a path
/// with a file-name component — so this is an infallible-by-construction
/// invariant, not a runtime concern.
#[allow(clippy::expect_used)]
fn write_secret(path: &Path, seed_bytes: &[u8; 32]) -> Result<(), IdentityError> {
    // Sibling .tmp path. `with_extension` would replace .key — we want
    // peer.key.tmp instead.
    let tmp = {
        let mut s = path
            .file_name()
            .expect("write_secret: path must have a filename — see fn doc")
            .to_os_string();
        s.push(".tmp");
        path.with_file_name(s)
    };

    let mut opts = OpenOptions::new();
    opts.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&tmp).map_err(|source| IdentityError::Io {
        path: tmp.clone(),
        source,
    })?;
    f.write_all(seed_bytes)
        .map_err(|source| IdentityError::Io {
            path: tmp.clone(),
            source,
        })?;
    f.sync_all().map_err(|source| IdentityError::Io {
        path: tmp.clone(),
        source,
    })?;
    drop(f);
    std::fs::rename(&tmp, path).map_err(|source| IdentityError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

/// Read exactly 32 secret bytes from `path`, checking mode bits.
fn read_secret(path: &Path) -> Result<[u8; 32], IdentityError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let md = std::fs::metadata(path).map_err(|source| IdentityError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mode = md.permissions().mode() & 0o777;
        if mode & !0o600 != 0 {
            return Err(IdentityError::InsecurePermissions {
                path: path.to_path_buf(),
                mode,
                expected: 0o600,
            });
        }
    }
    let bytes = std::fs::read(path).map_err(|source| IdentityError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if bytes.len() != 32 {
        return Err(IdentityError::SeedLengthMismatch {
            path: path.to_path_buf(),
            actual: bytes.len(),
        });
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

#[async_trait]
impl IdentityStore for FilesystemIdentityStore {
    async fn load_or_create_peer(&self) -> Result<PeerKeypair, IdentityError> {
        let path = self.peer_key_path();
        let path_for_blocking = path.clone();
        let bytes = tokio::task::spawn_blocking(move || -> Result<[u8; 32], IdentityError> {
            if path_for_blocking.exists() {
                read_secret(&path_for_blocking)
            } else {
                let mut seed = [0u8; 32];
                rand::rngs::OsRng.fill_bytes(&mut seed);
                write_secret(&path_for_blocking, &seed)?;
                Ok(seed)
            }
        })
        .await
        .map_err(|join_err| IdentityError::Io {
            path: path.clone(),
            source: std::io::Error::other(format!("spawn_blocking panicked: {join_err}")),
        })??;
        Ok(PeerKeypair::from_secret_bytes(bytes))
    }

    async fn load_author(&self, pk: &AuthorPubkey) -> Result<AuthorKeypair, IdentityError> {
        let pk_copy = *pk;
        let path = self.author_key_path(pk_copy);
        let path_for_blocking = path.clone();
        let kp = tokio::task::spawn_blocking(move || -> Result<AuthorKeypair, IdentityError> {
            let seed = read_secret(&path_for_blocking)?;
            let kp = AuthorKeypair::from_secret_bytes(seed);
            if kp.author != pk_copy {
                return Err(IdentityError::AuthorPubkeyMismatch {
                    path: path_for_blocking.clone(),
                    requested: encode_author_pubkey(pk_copy),
                    actual: encode_author_pubkey(kp.author),
                });
            }
            Ok(kp)
        })
        .await
        .map_err(|join_err| IdentityError::Io {
            path: path.clone(),
            source: std::io::Error::other(format!("spawn_blocking panicked: {join_err}")),
        })??;
        Ok(kp)
    }

    async fn create_author(&self) -> Result<AuthorKeypair, IdentityError> {
        // Generate a fresh authority off the worker thread to avoid
        // blocking the runtime executor on the fs write.
        let dir = self.authors_dir();
        let kp = tokio::task::spawn_blocking(move || -> Result<AuthorKeypair, IdentityError> {
            let mut seed = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut seed);
            let kp = AuthorKeypair::from_secret_bytes(seed);
            let filename = format!("{}.key", encode_author_pubkey(kp.author));
            let path = dir.join(filename);
            write_secret(&path, &seed)?;
            Ok(kp)
        })
        .await
        .map_err(|join_err| IdentityError::Io {
            path: self.authors_dir(),
            source: std::io::Error::other(format!("spawn_blocking panicked: {join_err}")),
        })??;
        Ok(kp)
    }

    async fn list_authors(&self) -> Result<Vec<AuthorPubkey>, IdentityError> {
        let dir = self.authors_dir();
        let dir_for_blocking = dir.clone();
        let pks =
            tokio::task::spawn_blocking(move || -> Result<Vec<AuthorPubkey>, IdentityError> {
                let mut out: Vec<AuthorPubkey> = Vec::new();
                let entries =
                    std::fs::read_dir(&dir_for_blocking).map_err(|source| IdentityError::Io {
                        path: dir_for_blocking.clone(),
                        source,
                    })?;
                for entry in entries {
                    let entry = entry.map_err(|source| IdentityError::Io {
                        path: dir_for_blocking.clone(),
                        source,
                    })?;
                    let name = entry.file_name();
                    let name_str = name.to_str().ok_or_else(|| {
                        IdentityError::InvalidAuthorFilename(name.to_string_lossy().into_owned())
                    })?;
                    // Pattern: wuser1<...>.key
                    let pk_str = name_str
                        .strip_suffix(".key")
                        .ok_or_else(|| IdentityError::InvalidAuthorFilename(name_str.to_owned()))?;
                    let pk = decode_author_pubkey(pk_str)?;
                    out.push(pk);
                }
                out.sort();
                Ok(out)
            })
            .await
            .map_err(|join_err| IdentityError::Io {
                path: dir.clone(),
                source: std::io::Error::other(format!("spawn_blocking panicked: {join_err}")),
            })??;
        Ok(pks)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn encode_then_decode_roundtrips() {
        let pk = AuthorPubkey::from_bytes([0xAB; 32]);
        let s = encode_author_pubkey(pk);
        assert!(s.starts_with("wuser1"), "must start with wuser1, got {s}");
        let pk2 = decode_author_pubkey(&s).expect("decode roundtrip");
        assert_eq!(pk, pk2);
    }

    #[test]
    fn decode_rejects_wrong_hrp() {
        let hrp = bech32::Hrp::parse_unchecked("wpub-author");
        let s = bech32::encode::<bech32::Bech32m>(hrp, &[0u8; 32]).unwrap();
        match decode_author_pubkey(&s) {
            Err(IdentityError::HrpMismatch { actual, .. }) => assert_eq!(actual, "wpub-author"),
            other => panic!("expected HrpMismatch, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_garbage() {
        match decode_author_pubkey("definitely-not-bech32m") {
            Err(IdentityError::Bech32Decode { .. }) => {}
            other => panic!("expected Bech32Decode, got {other:?}"),
        }
    }
}
