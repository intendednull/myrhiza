//! Content-addressed hashes used across the runtime.
//!
//! All hashes are BLAKE3 with canonical 32-byte output per
//! [determinism.md §5.1]. The byte ordering of `EventHash` is the
//! topological tie-break key per [convergence.md §4.1] — `Ord` on
//! `EventHash` is byte-lex over the inner array.

use core::fmt;

use serde::{Deserialize, Serialize};

/// 32-byte BLAKE3 hash of an event envelope.
///
/// `Ord` is byte-lex; this is normative for topo-sort tie-break per
/// [convergence.md §4.1].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventHash(#[serde(with = "crate::hash::serde_bytes_32_pub")] [u8; 32]);

/// 32-byte BLAKE3 hash of a bundle's content+manifest pair.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BundleHash(#[serde(with = "crate::hash::serde_bytes_32_pub")] [u8; 32]);

pub(crate) mod serde_bytes_32_pub {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(b: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        serde_bytes::Bytes::new(b).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let v: serde_bytes::ByteBuf = serde_bytes::ByteBuf::deserialize(d)?;
        let arr: [u8; 32] = v
            .as_ref()
            .try_into()
            .map_err(|_| serde::de::Error::invalid_length(v.len(), &"32 bytes"))?;
        Ok(arr)
    }
}

impl EventHash {
    /// All-zero hash sentinel used for the `prev` field of a genesis
    /// event (`seq == 1`) per [convergence.md §4]. Genesis events MUST
    /// encode `prev` as 32 raw zero bytes; non-genesis events MUST
    /// reference a real prior `EventHash` (zero-collision is
    /// astronomically improbable for BLAKE3).
    pub const ZERO: EventHash = EventHash([0u8; 32]);

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

    /// Hash arbitrary bytes via BLAKE3.
    #[must_use]
    pub fn blake3(input: &[u8]) -> Self {
        Self(*blake3::hash(input).as_bytes())
    }
}

impl BundleHash {
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

    /// Hash arbitrary bytes via BLAKE3.
    #[must_use]
    pub fn blake3(input: &[u8]) -> Self {
        Self(*blake3::hash(input).as_bytes())
    }
}

impl fmt::Display for EventHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for EventHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EventHash({self})")
    }
}

impl fmt::Display for BundleHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for BundleHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BundleHash({self})")
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use bincode::Options;

    #[test]
    fn event_hash_is_thirty_two_bytes() {
        assert_eq!(core::mem::size_of::<EventHash>(), 32);
    }

    #[test]
    fn event_hash_from_bytes_round_trip() {
        let raw = [0xAB; 32];
        let h = EventHash::from_bytes(raw);
        assert_eq!(h.as_bytes(), &raw);
    }

    #[test]
    fn event_hash_blake3_of_empty_is_canonical() {
        // BLAKE3 of empty input — published canonical vector.
        let h = EventHash::blake3(b"");
        assert_eq!(
            hex::encode(h.as_bytes()),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn event_hash_lex_ord_matches_byte_ord() {
        let a = EventHash::from_bytes([0x00; 32]);
        let mut b_bytes = [0x00; 32];
        b_bytes[31] = 0x01;
        let b = EventHash::from_bytes(b_bytes);
        assert!(a < b);
    }

    #[test]
    fn event_hash_serde_round_trip() {
        let h = EventHash::blake3(b"hello");
        let bytes = crate::canonical_bincode().serialize(&h).expect("encode");
        let decoded: EventHash = crate::canonical_bincode()
            .deserialize(&bytes)
            .expect("decode");
        assert_eq!(h, decoded);
    }

    #[test]
    fn event_hash_display_is_lowercase_hex() {
        let h = EventHash::from_bytes([0xDE; 32]);
        let s = format!("{h}");
        assert_eq!(s, "de".repeat(32));
    }
}
