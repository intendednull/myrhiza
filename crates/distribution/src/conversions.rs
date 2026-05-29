//! Orphan-rule conversions between Myrhiza's `BlobHash` /
//! `PeerPubkey` (in `crates/types`) and the corresponding `iroh-blobs`
//! / `iroh` newtypes.
//!
//! Free fns because the orphan rule prevents `impl From<...> for ...`
//! when neither type is local to this crate. Same pattern as
//! `peer_pubkey_from_iroh` in B-4.0.
//!
//! Per B-10 spec §4.2 + §4.6.

#[cfg(feature = "network-iroh")]
use myrhiza_types::{BlobHash, PeerPubkey};

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

/// Convert a Myrhiza `PeerPubkey` to an `iroh::EndpointId`.
///
/// Both are 32-byte Ed25519 public keys; `EndpointId::from_bytes`
/// validates the bytes form a valid curve point and is fallible at
/// the trait level (in normal use it never fails — Myrhiza's internal
/// `PeerPubkey` construction paths all originate from verified
/// signatures). Mirrors `iroh_endpoint_id_from_peer_pubkey` in
/// `crates/network`; duplicated here (a one-line newtype unwrap) so
/// `crates/distribution` carries NO dependency on `crates/network` —
/// that keeps the package graph a DAG once `crates/network` gains its
/// unconditional dep on `crates/distribution` for the
/// `GossipMessage::{Revocation,Publication}` variants (B-11 §3.1).
///
/// # Errors
///
/// Returns [`iroh::KeyParsingError`] if `peer`'s bytes do not form a
/// valid Ed25519 curve point.
#[cfg(feature = "network-iroh")]
pub fn endpoint_id_from_peer_pubkey(
    peer: PeerPubkey,
) -> Result<iroh::EndpointId, iroh::KeyParsingError> {
    iroh::EndpointId::from_bytes(peer.as_bytes())
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
