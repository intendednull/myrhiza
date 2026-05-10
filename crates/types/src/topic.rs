//! Topic identity.
//!
//! 32-byte ID derived from `BLAKE3("myrhiza/topic/v1" |
//! app_bundle_hash | per-topic-data)` (derivation lands in plan B
//! when bundle distribution wires up). This crate stores the ID
//! opaquely.

use serde::{Deserialize, Serialize};

/// Opaque 32-byte topic identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Topic(#[serde(with = "crate::hash::serde_bytes_32_pub")] [u8; 32]);

impl Topic {
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
    fn topic_id_is_thirty_two_bytes() {
        let t = Topic::from_bytes([0u8; 32]);
        assert_eq!(t.as_bytes().len(), 32);
    }

    #[test]
    fn topic_round_trip_via_canonical_bincode() {
        let t = Topic::from_bytes([0xCD; 32]);
        let bytes = crate::canonical_bincode().serialize(&t).expect("encode");
        let decoded: Topic = crate::canonical_bincode()
            .deserialize(&bytes)
            .expect("decode");
        assert_eq!(t, decoded);
    }
}
