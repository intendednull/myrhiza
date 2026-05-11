//! Crate-public serde helpers shared across canonical-encoded types.

/// Serde shim for fixed-length 64-byte signatures.
///
/// Used by [`crate::Event::signature`] and [`crate::dag::DriftMessage::signature`].
/// Serializes as `serde_bytes::Bytes` so the canonical bincode encoding
/// is `8-byte u64 length-prefix + 64 raw bytes` (length is always 64
/// under fixint big-endian).
pub mod serde_signature_64 {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// # Errors
    /// Returns a serializer error if the underlying serializer rejects the byte sequence.
    pub fn serialize<S: Serializer>(b: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        serde_bytes::Bytes::new(b).serialize(s)
    }

    /// # Errors
    /// Returns a deserializer error if the byte sequence is not exactly 64 bytes.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v: serde_bytes::ByteBuf = serde_bytes::ByteBuf::deserialize(d)?;
        let arr: [u8; 64] = v
            .as_ref()
            .try_into()
            .map_err(|_| serde::de::Error::invalid_length(v.len(), &"64 bytes"))?;
        Ok(arr)
    }
}
