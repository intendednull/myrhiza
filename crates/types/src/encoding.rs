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
}
