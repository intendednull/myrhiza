//! Filesystem-backed [`IdentityStore`] implementation + bech32m
//! filename helpers.
//!
//! Per plan B-2 spec §4 + §6. Secrets are raw 32-byte binary files
//! (Willow "no `wsecret` HRP, ever"); public keys embedded in
//! filenames use bech32m with the `wuser` HRP.
//!
//! [`IdentityStore`]: super::IdentityStore

use crate::identity::store::IdentityError;
use myrhiza_types::AuthorPubkey;

/// HRP for an event-author identity public key, per spec §4.3.
///
/// Distinct from `wpub-author` (publisher identity, distribution.md
/// §10.2) so the role is unambiguous on inspection.
// Bundle B introduces this; Bundle C wires `FilesystemIdentityStore`
// to call it. Tests in this file exercise it now.
#[allow(dead_code)]
pub(super) const HRP_AUTHOR_PK: &str = "wuser";

/// Encode an [`AuthorPubkey`] as a bech32m string with the `wuser` HRP.
///
/// Takes `AuthorPubkey` by value because it is `Copy` (newtype around
/// `[u8; 32]`).
///
/// # Panics
///
/// Cannot panic in practice: bech32m encoding is infallible for a
/// 32-byte payload and the fixed 5-character `wuser` HRP (well below
/// the BIP-173/350 90-character HRP+data limit). A panic would
/// indicate an upstream `bech32` bug.
// Bundle C wires this into `FilesystemIdentityStore`; covered by unit
// tests in this file.
#[allow(dead_code)]
#[allow(clippy::expect_used)]
pub(super) fn encode_author_pubkey(pk: AuthorPubkey) -> String {
    let hrp = bech32::Hrp::parse_unchecked(HRP_AUTHOR_PK);
    bech32::encode::<bech32::Bech32m>(hrp, pk.as_bytes())
        .expect("bech32m encode of 32 bytes with valid HRP cannot fail")
}

/// Decode a `wuser1...` bech32m string back into an [`AuthorPubkey`].
// Bundle C wires this into `FilesystemIdentityStore`; covered by unit
// tests in this file.
#[allow(dead_code)]
pub(super) fn decode_author_pubkey(s: &str) -> Result<AuthorPubkey, IdentityError> {
    let (hrp, data) = bech32::decode(s).map_err(|source| IdentityError::Bech32Decode {
        input: s.to_owned(),
        source,
    })?;
    if hrp.as_str() != HRP_AUTHOR_PK {
        return Err(IdentityError::HrpMismatch {
            input: s.to_owned(),
            expected: HRP_AUTHOR_PK,
            actual: hrp.as_str().to_owned(),
        });
    }
    if data.len() != 32 {
        return Err(IdentityError::SeedLengthMismatch {
            path: std::path::PathBuf::from(s),
            actual: data.len(),
        });
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&data);
    Ok(AuthorPubkey::from_bytes(bytes))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn encode_then_decode_roundtrips() {
        let pk = AuthorPubkey::from_bytes([0xAB; 32]);
        let s = encode_author_pubkey(pk);
        assert!(s.starts_with("wuser1"), "must start with wuser1, got {s}");
        let pk2 = decode_author_pubkey(&s).expect("decode roundtrip");
        assert_eq!(pk, pk2);
    }

    #[test]
    fn decode_rejects_wrong_hrp() {
        let hrp = bech32::Hrp::parse_unchecked("wpub-author");
        let s = bech32::encode::<bech32::Bech32m>(hrp, &[0u8; 32]).unwrap();
        match decode_author_pubkey(&s) {
            Err(IdentityError::HrpMismatch { actual, .. }) => assert_eq!(actual, "wpub-author"),
            other => panic!("expected HrpMismatch, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_garbage() {
        match decode_author_pubkey("definitely-not-bech32m") {
            Err(IdentityError::Bech32Decode { .. }) => {}
            other => panic!("expected Bech32Decode, got {other:?}"),
        }
    }
}
