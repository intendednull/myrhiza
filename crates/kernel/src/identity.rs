//! In-memory keypair stubs for B-1.
//!
//! Per plan-B-1 spec §10: full bech32m-encoded persistent identity
//! is B-2. B-1 lives entirely in-memory; keypairs generated at
//! `Runtime::start` time and discarded on shutdown.
//!
//! - [`PeerKeypair`] signs drift-messages per convergence.md §4.7.
//! - [`AuthorKeypair`] signs events per convergence.md §4.
//!
//! Both are nominally distinct but mechanically the same (Ed25519 `SigningKey`).

use ed25519_dalek::{Signer, SigningKey};
use myrhiza_types::{AuthorPubkey, EventHash, PeerPubkey};

/// Peer-scoped signing key (drift-message author identity).
pub struct PeerKeypair {
    secret: SigningKey,
    /// The peer's Ed25519 verifying key as a [`PeerPubkey`].
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
pub struct AuthorKeypair {
    secret: SigningKey,
    /// The author's Ed25519 verifying key as an [`AuthorPubkey`].
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

    #[test]
    fn author_sign_body_hash_verifies_via_manifest_path() {
        let kp = AuthorKeypair::deterministic(99);
        let body_hash = EventHash::blake3(b"some-event-body");
        let sig = kp.sign_body_hash(body_hash);
        // Cross-check against the same path the DAG will use to verify.
        myrhiza_manifest::verify_signature(kp.author.as_bytes(), body_hash.as_bytes(), &sig)
            .expect("author-signed body_hash must verify via manifest verify_signature");
    }
}
