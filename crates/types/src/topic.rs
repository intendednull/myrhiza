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

    /// Derive a topic ID from a pre-normalized name + bundle hash + app instance seed
    /// per convergence.md §4.6.
    ///
    /// Caller MUST pass an NFC-normalized name. Use
    /// `myrhiza_manifest::derive_topic_normalized` if the name comes
    /// from an unnormalized source.
    #[must_use]
    pub fn derive(app_bundle_hash: &crate::BundleHash, seed: &[u8; 32], name: &str) -> Topic {
        let mut h = blake3::Hasher::new();
        h.update(b"myrhiza/topic/v1");
        h.update(app_bundle_hash.as_bytes());
        h.update(seed);
        h.update(name.as_bytes());
        Topic::from_bytes(h.finalize().into())
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

    #[test]
    fn topic_derive_is_deterministic() {
        let bh = crate::BundleHash::from_bytes([0xAA; 32]);
        let seed = [0x11; 32];
        let t1 = Topic::derive(&bh, &seed, "main");
        let t2 = Topic::derive(&bh, &seed, "main");
        assert_eq!(t1, t2);
    }

    #[test]
    fn topic_derive_differs_by_name() {
        let bh = crate::BundleHash::from_bytes([0xAA; 32]);
        let seed = [0x11; 32];
        assert_ne!(
            Topic::derive(&bh, &seed, "main"),
            Topic::derive(&bh, &seed, "other")
        );
    }

    #[test]
    fn topic_derive_differs_by_seed() {
        let bh = crate::BundleHash::from_bytes([0xAA; 32]);
        assert_ne!(
            Topic::derive(&bh, &[0x11; 32], "main"),
            Topic::derive(&bh, &[0x22; 32], "main")
        );
    }

    #[test]
    fn topic_derive_differs_by_bundle_hash() {
        let seed = [0x11; 32];
        assert_ne!(
            Topic::derive(&crate::BundleHash::from_bytes([0xAA; 32]), &seed, "main"),
            Topic::derive(&crate::BundleHash::from_bytes([0xBB; 32]), &seed, "main")
        );
    }

    #[test]
    fn topic_derive_includes_domain_separator() {
        // Hand-compute BLAKE3 with the domain separator to verify the formula is
        // not accidentally changed.
        let bh = crate::BundleHash::from_bytes([0; 32]);
        let seed = [0; 32];
        let mut h = blake3::Hasher::new();
        h.update(b"myrhiza/topic/v1");
        h.update(bh.as_bytes());
        h.update(&seed);
        h.update(b"main");
        let expected: [u8; 32] = h.finalize().into();
        assert_eq!(Topic::derive(&bh, &seed, "main").as_bytes(), &expected);
    }
}
