//! Peer-scoped identity (drift-message signing per convergence.md §4.7).
//!
//! Distinct from [`crate::AuthorPubkey`]: `AuthorPubkey` is the long-term
//! user/author identity that signs events; `PeerPubkey` is the per-peer
//! instance identity that signs drift-messages. Same underlying
//! primitive (Ed25519 32-byte pubkey) but the nominal type boundary
//! prevents accidental cross-use.

use serde::{Deserialize, Serialize};

/// Ed25519 public key of a peer instance.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PeerPubkey(#[serde(with = "crate::hash::serde_bytes_32_pub")] [u8; 32]);

impl PeerPubkey {
    /// Construct from a raw 32-byte array.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the inner 32-byte array.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bincode::Options;

    #[test]
    fn peer_pubkey_round_trips_canonical_bincode() {
        let p = PeerPubkey::from_bytes([0xAB; 32]);
        let bytes = crate::canonical_bincode().serialize(&p).expect("encode");
        let decoded: PeerPubkey = crate::canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(p, decoded);
    }
}
