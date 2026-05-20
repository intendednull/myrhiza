//! B-2 acceptance: filesystem-backed identity store round-trips.
//!
//! Per docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md §9.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use myrhiza_kernel::identity::{FilesystemIdentityStore, IdentityStore};

/// Create a tempdir with mode 0o700 on Unix — required because
/// `FilesystemIdentityStore::open` refuses tempdirs created with a
/// permissive umask (0o755 under default GitHub Actions umask).
fn secure_tempdir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("chmod tempdir 0o700");
    }
    dir
}

/// Covers: identity.md §6, crypto.md §9.1
///
/// B-2 design §6 round-trip happy path; replaces plan-B-1 §10's
/// in-memory peer-identity stub.
#[tokio::test]
async fn peer_key_round_trip_persists_across_store_reopen() {
    let dir = secure_tempdir();
    let path = dir.path().to_path_buf();

    let store1 = FilesystemIdentityStore::open(&path).await.expect("open 1");
    let peer1 = store1.load_or_create_peer().await.expect("create");
    let pub1 = peer1.public;
    let sig1 = peer1.sign(b"hello");
    drop(peer1);
    drop(store1);

    let store2 = FilesystemIdentityStore::open(&path).await.expect("open 2");
    let peer2 = store2.load_or_create_peer().await.expect("load");
    assert_eq!(peer2.public, pub1, "pubkey must persist");
    let sig2 = peer2.sign(b"hello");
    assert_eq!(sig1, sig2, "deterministic sig under same key must match");
}

/// Covers: identity.md §6, crypto.md §9.1
///
/// B-2 design §6 round-trip happy path for `AuthorKeypair`;
/// extends plan-B-1 §11 author keypair handling with persistence.
#[tokio::test]
async fn author_key_round_trip_persists_across_store_reopen() {
    let dir = secure_tempdir();
    let path = dir.path().to_path_buf();

    let store1 = FilesystemIdentityStore::open(&path).await.expect("open 1");
    let auth1 = store1.create_author().await.expect("create author");
    let pk = auth1.author;
    drop(auth1);
    drop(store1);

    let store2 = FilesystemIdentityStore::open(&path).await.expect("open 2");
    let auth2 = store2.load_author(&pk).await.expect("load author");
    assert_eq!(auth2.author, pk);
}

/// Covers: identity.md §6, crypto.md §9.1
///
/// B-2 design §6 `list_authors`; extends plan-B-1 §11 kernel-runtime
/// author keypair handling.
#[tokio::test]
async fn list_authors_returns_all_created_authors_sorted() {
    let dir = secure_tempdir();
    let path = dir.path().to_path_buf();

    let store = FilesystemIdentityStore::open(&path).await.expect("open");
    let mut created = Vec::new();
    for _ in 0..3 {
        created.push(store.create_author().await.expect("create author").author);
    }
    drop(store);

    let store2 = FilesystemIdentityStore::open(&path).await.expect("reopen");
    let listed = store2.list_authors().await.expect("list");
    assert_eq!(listed.len(), 3);

    let mut expected = created.clone();
    expected.sort();
    assert_eq!(listed, expected, "list_authors must return sorted pubkeys");
}

/// Covers: identity.md §6, crypto.md §9.1
///
/// B-2 design §6 `load_or_create_peer` idempotence; complements
/// plan-B-1 §10 peer-identity semantics.
#[tokio::test]
async fn load_or_create_peer_is_idempotent_within_one_store() {
    let dir = secure_tempdir();
    let path = dir.path().to_path_buf();

    let store = FilesystemIdentityStore::open(&path).await.expect("open");
    let peer1 = store.load_or_create_peer().await.expect("create");
    let pub1 = peer1.public;
    let peer2 = store.load_or_create_peer().await.expect("reload");
    assert_eq!(peer1.public, peer2.public);
    assert_eq!(pub1, peer2.public);
}

/// Covers: identity.md §6, crypto.md §9.1
///
/// B-2 design §6.3 permission enforcement; kernel-custody discipline.
#[cfg(unix)]
#[tokio::test]
async fn load_rejects_loose_unix_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = secure_tempdir();
    let path = dir.path().to_path_buf();

    let store = FilesystemIdentityStore::open(&path).await.expect("open");
    let _peer = store.load_or_create_peer().await.expect("create");

    // Loosen permissions on peer.key to world-readable.
    let peer_key = path.join("peer.key");
    std::fs::set_permissions(&peer_key, std::fs::Permissions::from_mode(0o644)).expect("chmod 644");

    // Reopen and try to load.
    let store2 = FilesystemIdentityStore::open(&path).await.expect("reopen");
    // `expect_err` requires `T: Debug`, but `PeerKeypair` intentionally
    // does not derive `Debug` (secret material — `ZeroizeOnDrop`). Use a
    // let-else to bind the error without needing `Debug` on the `Ok` arm.
    let Err(err) = store2.load_or_create_peer().await else {
        panic!("must reject (got Ok)");
    };
    match err {
        myrhiza_kernel::identity::IdentityError::InsecurePermissions { mode, expected, .. } => {
            assert_eq!(mode, 0o644);
            assert_eq!(expected, 0o600);
        }
        other => panic!("expected InsecurePermissions, got {other:?}"),
    }
}

/// Covers: identity.md §6, crypto.md §9.1
///
/// B-2 design §6.3 seed-length enforcement; kernel-custody discipline.
#[tokio::test]
async fn load_rejects_seed_length_mismatch() {
    let dir = secure_tempdir();
    let path = dir.path().to_path_buf();

    let store = FilesystemIdentityStore::open(&path).await.expect("open");
    // Pre-place a malformed file BEFORE calling load_or_create_peer
    // so the auto-generate path doesn't fire.
    let peer_key = path.join("peer.key");
    std::fs::write(&peer_key, b"too short").expect("write");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&peer_key, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    }

    // `expect_err` would require `PeerKeypair: Debug`, which we
    // deliberately do not derive (secret material). Use let-else.
    let Err(err) = store.load_or_create_peer().await else {
        panic!("must reject (got Ok)");
    };
    match err {
        myrhiza_kernel::identity::IdentityError::SeedLengthMismatch { actual, .. } => {
            assert_eq!(actual, b"too short".len());
        }
        other => panic!("expected SeedLengthMismatch, got {other:?}"),
    }
}

/// Covers: identity.md §6, crypto.md §9.1
///
/// B-2 design §6.3 / §4 filename discipline; kernel-custody hygiene
/// against tampered or mis-named author files.
#[tokio::test]
async fn load_rejects_corrupted_filename_bech32m() {
    let dir = secure_tempdir();
    let path = dir.path().to_path_buf();

    let store = FilesystemIdentityStore::open(&path).await.expect("open");

    // Pre-place a file in authors/ with a non-wuser1 filename. It
    // should fail list_authors with InvalidAuthorFilename (no .key
    // suffix or bad HRP).
    let bad = path.join("authors").join("garbage.key");
    std::fs::write(&bad, [0u8; 32]).expect("write");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    }

    let err = store.list_authors().await.expect_err("must reject");
    match err {
        myrhiza_kernel::identity::IdentityError::Bech32Decode { .. }
        | myrhiza_kernel::identity::IdentityError::InvalidAuthorFilename(_) => {}
        other => panic!("expected Bech32Decode or InvalidAuthorFilename, got {other:?}"),
    }
}

/// Covers: identity.md §6, crypto.md §9.1
///
/// B-2 design §6.3 pubkey-filename cross-check; kernel-custody guard
/// against tampered filenames.
#[tokio::test]
async fn load_author_rejects_pubkey_filename_mismatch() {
    let dir = secure_tempdir();
    let path = dir.path().to_path_buf();

    let store = FilesystemIdentityStore::open(&path).await.expect("open");

    // Create a real author so we get a valid filename.
    let auth_real = store.create_author().await.expect("create");
    let real_path = path
        .join("authors")
        .read_dir()
        .expect("readdir")
        .next()
        .expect("entry")
        .expect("entry ok")
        .path();

    // Overwrite the file content with a *different* seed so the
    // derived pubkey no longer matches the filename's embedded one.
    std::fs::write(&real_path, [0xFEu8; 32]).expect("overwrite");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&real_path, std::fs::Permissions::from_mode(0o600))
            .expect("chmod");
    }

    // `expect_err` would require `AuthorKeypair: Debug`, which we
    // deliberately do not derive (secret material). Use let-else.
    let Err(err) = store.load_author(&auth_real.author).await else {
        panic!("must reject (got Ok)");
    };
    assert!(
        matches!(
            err,
            myrhiza_kernel::identity::IdentityError::AuthorPubkeyMismatch { .. }
        ),
        "expected AuthorPubkeyMismatch, got {err:?}"
    );
}

/// Covers: identity.md §6, crypto.md §9.1
///
/// B-2 design §6.3 dir creation mode; kernel-custody discipline at
/// store-open time.
#[cfg(unix)]
#[tokio::test]
async fn open_creates_directory_with_0700_mode() {
    use std::os::unix::fs::PermissionsExt;

    let outer = tempfile::tempdir().expect("tempdir");
    let inner = outer.path().join("new-identity-dir");
    assert!(!inner.exists());

    let _store = FilesystemIdentityStore::open(&inner).await.expect("open");
    assert!(inner.exists());

    let mode = std::fs::metadata(&inner)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o700);

    let authors_mode = std::fs::metadata(inner.join("authors"))
        .expect("authors metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(authors_mode, 0o700);
}

/// Covers: identity.md §6, crypto.md §9.1
///
/// B-2 design §6.4 atomic write under concurrency; same atomic-write
/// idiom protects peer and author keys identically.
#[tokio::test]
async fn concurrent_store_writes_do_not_corrupt_key() {
    use std::sync::Arc;

    let dir = secure_tempdir();
    let path = dir.path().to_path_buf();

    let store = Arc::new(FilesystemIdentityStore::open(&path).await.expect("open"));

    let s1 = store.clone();
    let s2 = store.clone();
    let h1 = tokio::spawn(async move { s1.create_author().await });
    let h2 = tokio::spawn(async move { s2.create_author().await });

    let r1 = h1.await.expect("join 1").expect("create 1");
    let r2 = h2.await.expect("join 2").expect("create 2");
    assert_ne!(
        r1.author, r2.author,
        "two creates must produce distinct authors"
    );

    let listed = store.list_authors().await.expect("list");
    assert_eq!(listed.len(), 2);
    assert!(listed.contains(&r1.author));
    assert!(listed.contains(&r2.author));

    // Both files must be exactly 32 bytes.
    for entry in std::fs::read_dir(path.join("authors")).expect("readdir") {
        let entry = entry.expect("entry");
        let md = std::fs::metadata(entry.path()).expect("metadata");
        assert_eq!(md.len(), 32, "every author key file is exactly 32 bytes");
    }
}
