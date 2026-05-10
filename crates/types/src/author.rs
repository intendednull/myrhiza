//! Ed25519 author public key (32 raw bytes per RFC 8032).

use serde::{Deserialize, Serialize};

/// Ed25519 author public key.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AuthorPubkey(#[serde(with = "crate::hash::serde_bytes_32_pub")] [u8; 32]);

impl AuthorPubkey {
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
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use bincode::Options;

    #[test]
    fn author_pubkey_round_trip() {
        let pk = AuthorPubkey::from_bytes([0xEE; 32]);
        let bytes = crate::canonical_bincode().serialize(&pk).expect("encode");
        let decoded: AuthorPubkey = crate::canonical_bincode()
            .deserialize(&bytes)
            .expect("decode");
        assert_eq!(pk, decoded);
    }
}
