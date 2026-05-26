//! Orphan-rule conversions between Myrhiza's `BlobHash` (in
//! `crates/types`) and `iroh_blobs::Hash` (in `iroh-blobs`).
//!
//! Free fns because the orphan rule prevents `impl From<...> for ...`
//! when neither type is local to this crate. Same pattern as
//! `peer_pubkey_from_iroh` in B-4.0.
//!
//! Per B-10 spec §4.2 + §4.6.

#[cfg(feature = "network-iroh")]
use myrhiza_types::BlobHash;

/// Convert a Myrhiza `BlobHash` to an `iroh_blobs::Hash`.
///
/// Both are 32-byte BLAKE3; this is a memcpy + wrap.
#[cfg(feature = "network-iroh")]
#[must_use]
pub fn blob_hash_to_iroh(h: BlobHash) -> iroh_blobs::Hash {
    iroh_blobs::Hash::from_bytes(*h.as_bytes())
}

/// Convert an `iroh_blobs::Hash` to a Myrhiza `BlobHash`.
#[cfg(feature = "network-iroh")]
#[must_use]
pub fn blob_hash_from_iroh(h: iroh_blobs::Hash) -> BlobHash {
    BlobHash::from_bytes(*h.as_bytes())
}

#[cfg(all(test, feature = "network-iroh"))]
mod tests {
    use super::*;

    #[test]
    fn blob_hash_roundtrips_through_iroh_hash() {
        let raw = [0x42u8; 32];
        let mh = BlobHash::from_bytes(raw);
        let ih = blob_hash_to_iroh(mh);
        let back = blob_hash_from_iroh(ih);
        assert_eq!(mh, back);
        assert_eq!(*ih.as_bytes(), raw);
    }
}
