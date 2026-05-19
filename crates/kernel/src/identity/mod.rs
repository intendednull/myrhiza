//! Identity primitives — keypair structs + pluggable storage.
//!
//! This module owns the in-memory keypair types `PeerKeypair` and
//! `AuthorKeypair` (drift-message and event-author signing identities,
//! per plan B-1 spec §10 and convergence.md §4 / §4.7). It also exposes
//! the persistent-storage layer added by plan B-2:
//!
//! - [`IdentityStore`] — pluggable backend trait (this module's `store`).
//! - `FilesystemIdentityStore` — disk-backed impl with bech32m
//!   `wuser`-HRP author filenames and raw-bytes secret files (this
//!   module's `fs`).
//! - [`IdentityError`] — failure surface.
//!
//! Per plan B-2 spec §5 + §6 + §7. Keypair structs derive
//! `ZeroizeOnDrop` (Willow precedent — `prior-art/willow/identity.md`).
//!
//! `Runtime::start` consumes a `PeerKeypair` / `Option<AuthorKeypair>`
//! by value as in B-1; loading from a store is a caller-side concern.

use ed25519_dalek::{Signer, SigningKey};
use myrhiza_types::{AuthorPubkey, EventHash, PeerPubkey};
use zeroize::ZeroizeOnDrop;

/// Peer-scoped signing key (drift-message author identity).
#[derive(ZeroizeOnDrop)]
pub struct PeerKeypair {
    secret: SigningKey,
    /// The peer's Ed25519 verifying key as a [`PeerPubkey`].
    #[zeroize(skip)]
    pub public: PeerPubkey,
}

impl PeerKeypair {
    /// Construct from a 32-byte ed25519 secret-key seed.
    #[must_use]
    pub fn from_secret_bytes(b: [u8; 32]) -> Self {
        let secret = SigningKey::from_bytes(&b);
        let public = PeerPubkey::from_bytes(secret.verifying_key().to_bytes());
        Self { secret, public }
    }

    /// Generate a fresh peer keypair from a cryptographically-secure RNG.
    ///
    /// Per plan-B-1 spec §10. Production callers should pass a
    /// `rand::rngs::OsRng` or equivalent. Tests that need reproducibility
    /// should keep using [`Self::deterministic`] instead.
    ///
    /// Implementation: draws 32 bytes via `rng.fill_bytes` and routes
    /// through [`Self::from_secret_bytes`]. This avoids adding the
    /// `rand_core` feature flag to the workspace `ed25519-dalek`
    /// dependency just for `SigningKey::generate`; the entropy quality
    /// is identical when `R: CryptoRng`.
    pub fn generate<R: rand_core::CryptoRng + rand_core::RngCore>(rng: &mut R) -> Self {
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        Self::from_secret_bytes(bytes)
    }

    /// Deterministic generation for tests.
    #[must_use]
    pub fn deterministic(seed: u64) -> Self {
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&seed.to_be_bytes());
        Self::from_secret_bytes(bytes)
    }

    /// Sign an arbitrary message under the peer key.
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.secret.sign(message).to_bytes()
    }
}

/// Long-term author signing key (event author identity).
#[derive(ZeroizeOnDrop)]
pub struct AuthorKeypair {
    secret: SigningKey,
    /// The author's Ed25519 verifying key as an [`AuthorPubkey`].
    #[zeroize(skip)]
    pub author: AuthorPubkey,
}

impl AuthorKeypair {
    /// Construct from a 32-byte ed25519 secret-key seed.
    #[must_use]
    pub fn from_secret_bytes(b: [u8; 32]) -> Self {
        let secret = SigningKey::from_bytes(&b);
        let author = AuthorPubkey::from_bytes(secret.verifying_key().to_bytes());
        Self { secret, author }
    }

    /// Deterministic generation for tests.
    #[must_use]
    pub fn deterministic(seed: u64) -> Self {
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&seed.to_be_bytes());
        Self::from_secret_bytes(bytes)
    }

    /// Generate a fresh author keypair from a cryptographically-secure RNG.
    ///
    /// Production callers should pass a `rand::rngs::OsRng` or
    /// equivalent. Tests that need reproducibility should keep using
    /// [`Self::deterministic`].
    ///
    /// Implementation mirrors [`PeerKeypair::generate`]: draws 32 bytes
    /// via `rng.fill_bytes` and routes through [`Self::from_secret_bytes`].
    pub fn generate<R: rand_core::CryptoRng + rand_core::RngCore>(rng: &mut R) -> Self {
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        Self::from_secret_bytes(bytes)
    }

    /// Sign an event's BLAKE3-hash-of-signed-body.
    ///
    /// Per plan-B-1 spec §4.2 step 1 normative: Ed25519 signs the
    /// raw 32 bytes of `body_hash`, NOT the canonical-encoded
    /// `SignedBody` pre-image.
    #[must_use]
    pub fn sign_body_hash(&self, body_hash: EventHash) -> [u8; 64] {
        self.secret.sign(body_hash.as_bytes()).to_bytes()
    }
}

mod store;
pub use store::{IdentityError, IdentityStore};

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_peer_keypair_is_reproducible() {
        let p1 = PeerKeypair::deterministic(42);
        let p2 = PeerKeypair::deterministic(42);
        assert_eq!(p1.public, p2.public);
    }

    #[test]
    fn deterministic_author_keypair_is_reproducible() {
        let a1 = AuthorKeypair::deterministic(7);
        let a2 = AuthorKeypair::deterministic(7);
        assert_eq!(a1.author, a2.author);
    }

    /// Spec §10: `PeerKeypair::generate<R: CryptoRng + RngCore>(rng: &mut R) -> Self`.
    ///
    /// Two consecutive draws from a CSPRNG must produce distinct keys.
    /// Regression for review-finding I-4.
    #[test]
    fn peer_keypair_generate_with_csprng_produces_distinct_keys() {
        use rand::rngs::OsRng;
        let mut rng = OsRng;
        let a = PeerKeypair::generate(&mut rng);
        let b = PeerKeypair::generate(&mut rng);
        assert_ne!(a.public, b.public, "two CSPRNG draws must differ");
    }

    #[test]
    fn author_sign_body_hash_verifies_via_manifest_path() {
        let kp = AuthorKeypair::deterministic(99);
        let body_hash = EventHash::blake3(b"some-event-body");
        let sig = kp.sign_body_hash(body_hash);
        // Cross-check against the same path the DAG will use to verify.
        myrhiza_manifest::verify_signature(kp.author.as_bytes(), body_hash.as_bytes(), &sig)
            .expect("author-signed body_hash must verify via manifest verify_signature");
    }

    /// Compile-only check that both keypair types derive `ZeroizeOnDrop`.
    /// The zeroize crate's drop-time guarantee is upstream-tested; we
    /// only assert that our derive wiring is correct.
    #[test]
    fn keypair_types_derive_zeroize_on_drop() {
        fn assert_zeroize_on_drop<T: zeroize::ZeroizeOnDrop>() {}
        assert_zeroize_on_drop::<PeerKeypair>();
        assert_zeroize_on_drop::<AuthorKeypair>();
    }
}
