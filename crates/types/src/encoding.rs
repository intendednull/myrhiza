//! Canonical bincode configuration.
//!
//! `bincode 1.3.x` exposes both function-level entry points
//! (`bincode::serialize`) and an `Options` builder. The function-level
//! entry points use a different default config than the builder — two
//! correct implementations following different idioms produce different
//! bytes. Per [determinism.md §5.4] this divergence is convergence-
//! breaking.
//!
//! Every byte-stable encode in the Myrhiza runtime MUST go through
//! [`canonical_bincode`]. Direct calls to `bincode::serialize` /
//! `bincode::deserialize` are forbidden (clippy lint enforces this in
//! the workspace; reviewer enforces during code review).

use bincode::{
    DefaultOptions, Options,
    config::{BigEndian, FixintEncoding, WithOtherEndian, WithOtherIntEncoding},
};
use serde::{Serialize, de::DeserializeOwned};

/// The canonical bincode `Options` chain.
///
/// Equivalent to:
/// `DefaultOptions::new().with_fixint_encoding().with_big_endian()`.
pub type CanonicalOptions =
    WithOtherEndian<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, BigEndian>;

/// Returns the canonical bincode options chain pinned by the master spec.
#[must_use]
pub fn canonical_bincode() -> CanonicalOptions {
    DefaultOptions::new()
        .with_fixint_encoding()
        .with_big_endian()
}

/// Errors returned by canonical decode helpers.
///
/// Distinguishes a malformed encoding (`Bincode`) from a structurally
/// valid but non-canonical one (`NonCanonical`). The latter is the
/// convergence-divergence vector: two peers feeding "the same" bytes
/// into a state-apply must observe identical outcomes, and a decoder
/// that silently accepts trailing garbage / non-canonical varint
/// encodings would let one peer's "valid" decode disagree with another's
/// "rejected" decode for byte-equivalent inputs.
#[derive(Debug, thiserror::Error)]
pub enum EncodingError {
    /// Underlying bincode decode failed (malformed bytes).
    #[error("bincode decode error: {0}")]
    Bincode(String),
    /// Decode succeeded but re-encoding the value did not reproduce the
    /// input bytes exactly. The input is structurally valid but not
    /// the canonical encoding of the decoded value.
    #[error("non-canonical encoding (re-encode mismatch)")]
    NonCanonical,
}

/// Strict canonical decode: deserialize `bytes` as `T`, then re-encode
/// `T` and assert the re-encoded bytes are byte-identical to the input.
///
/// This is the only correct decoder for inputs that cross the
/// convergence surface (state-apply event bytes, manifest bytes, etc.):
/// it rejects trailing garbage, non-canonical varint encodings, and
/// any other byte-distinct-but-decoder-equivalent input that would let
/// two honest peers disagree on whether the bytes "decode."
///
/// # Errors
/// - [`EncodingError::Bincode`] if `bytes` is not a valid canonical
///   bincode encoding of any `T` value.
/// - [`EncodingError::NonCanonical`] if `bytes` decodes successfully
///   but re-encoding the resulting value does not reproduce `bytes`
///   byte-for-byte.
pub fn decode_canonical<T>(bytes: &[u8]) -> Result<T, EncodingError>
where
    T: Serialize + DeserializeOwned,
{
    let value: T = canonical_bincode()
        .deserialize(bytes)
        .map_err(|e| EncodingError::Bincode(e.to_string()))?;
    let re_encoded = canonical_bincode()
        .serialize(&value)
        .map_err(|e| EncodingError::Bincode(e.to_string()))?;
    if re_encoded.as_slice() != bytes {
        return Err(EncodingError::NonCanonical);
    }
    Ok(value)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn empty_btreemap_encodes_to_zero_length() {
        let map: BTreeMap<String, u32> = BTreeMap::new();
        let bytes = canonical_bincode()
            .serialize(&map)
            .expect("encode empty btreemap");
        // bincode 1.3 with fixint big-endian encodes a length prefix as u64 BE.
        assert_eq!(bytes, vec![0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn u32_encodes_big_endian_fixint() {
        let bytes = canonical_bincode()
            .serialize(&0x0102_0304_u32)
            .expect("encode u32");
        assert_eq!(bytes, vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn btreemap_round_trip_preserves_order() {
        let mut map: BTreeMap<String, u32> = BTreeMap::new();
        map.insert("zebra".into(), 1);
        map.insert("apple".into(), 2);
        let bytes = canonical_bincode().serialize(&map).expect("encode");
        let decoded: BTreeMap<String, u32> =
            canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(map, decoded);
        // Re-encoding the decoded map must produce identical bytes (canonical).
        let bytes2 = canonical_bincode().serialize(&decoded).expect("re-encode");
        assert_eq!(bytes, bytes2);
    }

    #[test]
    fn decode_canonical_round_trips() {
        let mut map: BTreeMap<String, u32> = BTreeMap::new();
        map.insert("apple".into(), 1);
        map.insert("zebra".into(), 2);
        let bytes = canonical_bincode().serialize(&map).expect("encode");
        let decoded: BTreeMap<String, u32> =
            decode_canonical(&bytes).expect("decode_canonical accepts canonical bytes");
        assert_eq!(map, decoded);
    }

    #[test]
    fn decode_canonical_rejects_trailing_garbage() {
        let value: u32 = 0x0102_0304;
        let mut bytes = canonical_bincode().serialize(&value).expect("encode");
        bytes.push(0x00); // append trailing byte
        // Either error variant is acceptable — the convergence-relevant
        // property is "non-canonical bytes are rejected." bincode 1.3
        // surfaces trailing-garbage at the deserialize layer (Bincode);
        // hypothetical future encodings where decode succeeds but
        // re-encode disagrees would surface as NonCanonical. Both block
        // the convergence-divergence vector.
        let err = decode_canonical::<u32>(&bytes)
            .expect_err("decode_canonical must reject trailing garbage");
        match err {
            EncodingError::Bincode(_) | EncodingError::NonCanonical => {}
        }
    }

    /// Manually constructs bytes that decode to a known value but are
    /// NOT the canonical encoding of that value, to exercise the
    /// re-encode branch of [`decode_canonical`].
    ///
    /// `bincode 1.3` with fixint big-endian writes a `BTreeSet<u8>` as:
    ///   `u64 BE length | element bytes (already sorted)`
    /// Bincode tolerates an out-of-order (or duplicate) input on
    /// deserialize — it just iterates the bytes — but the `BTreeSet`
    /// re-encoding will sort, so the round-trip bytes diverge.
    #[test]
    fn decode_canonical_rejects_non_canonical_set_order() {
        use std::collections::BTreeSet;
        // Construct a canonical encoding of {1, 2, 3}, then reorder
        // the element bytes to {3, 1, 2}. Both sequences decode to the
        // same set, but only the first is canonical.
        let canonical_bytes: Vec<u8> = {
            let mut s = BTreeSet::new();
            s.insert(1u8);
            s.insert(2u8);
            s.insert(3u8);
            canonical_bincode().serialize(&s).expect("encode")
        };
        // Length prefix is 8 bytes (u64 BE), then 3 element bytes.
        assert_eq!(canonical_bytes.len(), 8 + 3);
        assert_eq!(&canonical_bytes[8..], &[1u8, 2, 3]);

        // Build a byte-distinct but decoder-equivalent encoding by
        // reordering the element bytes.
        let mut non_canonical = canonical_bytes.clone();
        non_canonical[8..].copy_from_slice(&[3u8, 1, 2]);

        // Sanity: this still decodes to the same set.
        let decoded: BTreeSet<u8> = canonical_bincode()
            .deserialize(&non_canonical)
            .expect("non-canonical bytes still decode");
        assert_eq!(decoded.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3]);

        // But strict canonical decode must reject the re-encode mismatch.
        let err = decode_canonical::<BTreeSet<u8>>(&non_canonical)
            .expect_err("decode_canonical must reject non-canonical set order");
        assert!(
            matches!(err, EncodingError::NonCanonical),
            "expected NonCanonical, got {err:?}"
        );
    }

    #[test]
    fn decode_canonical_rejects_truncated_bytes() {
        let mut map: BTreeMap<String, u32> = BTreeMap::new();
        map.insert("apple".into(), 1);
        let mut bytes = canonical_bincode().serialize(&map).expect("encode");
        bytes.truncate(bytes.len() - 1); // drop last byte
        let err = decode_canonical::<BTreeMap<String, u32>>(&bytes)
            .expect_err("decode_canonical must reject truncated bytes");
        // Truncation surfaces as an underlying bincode decode failure,
        // not a re-encode mismatch.
        assert!(
            matches!(err, EncodingError::Bincode(_)),
            "expected Bincode error, got {err:?}"
        );
    }
}
