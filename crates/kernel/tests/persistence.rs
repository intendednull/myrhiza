//! B-2 acceptance: filesystem-backed identity store round-trips.
//!
//! Per docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md §9.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use myrhiza_kernel::identity::{FilesystemIdentityStore, IdentityStore};

/// Covers: spec §6 round-trip happy path. plan-b-1 §10 (peer identity
/// — persistence replaces the in-memory stub).
#[tokio::test]
async fn peer_key_round_trip_persists_across_store_reopen() {
    let dir = tempfile::tempdir().expect("tempdir");
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

/// Covers: spec §6 round-trip happy path for `AuthorKeypair`.
/// plan-b-1 §11 (kernel runtime — author keypair handling).
#[tokio::test]
async fn author_key_round_trip_persists_across_store_reopen() {
    let dir = tempfile::tempdir().expect("tempdir");
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

/// Covers: spec §6 `list_authors`. plan-b-1 §11 (kernel runtime —
/// author keypair handling).
#[tokio::test]
async fn list_authors_returns_all_created_authors_sorted() {
    let dir = tempfile::tempdir().expect("tempdir");
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

/// Covers: spec §6 `load_or_create_peer` idempotence. plan-b-1 §10.
#[tokio::test]
async fn load_or_create_peer_is_idempotent_within_one_store() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().to_path_buf();

    let store = FilesystemIdentityStore::open(&path).await.expect("open");
    let peer1 = store.load_or_create_peer().await.expect("create");
    let pub1 = peer1.public;
    let peer2 = store.load_or_create_peer().await.expect("reload");
    assert_eq!(peer1.public, peer2.public);
    assert_eq!(pub1, peer2.public);
}
