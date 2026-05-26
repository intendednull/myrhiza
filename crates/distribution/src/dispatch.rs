//! Subscription dispatch for revocation + publication envelopes.
//!
//! Per B-10 spec §3.3 ¶3 + §6.4. Two stateless verify helpers run at
//! the gossip-receive boundary BEFORE the envelope reaches the state
//! machine. Forged envelopes (bad Ed25519 signatures) are dropped here
//! so [`crate::revocation::RevocationLog::apply`] /
//! [`crate::publication::PublicationLog::apply`] never see them.
//!
//! The state machines themselves still re-verify the signature on
//! `apply` — that's intentional defense-in-depth: the state-tier
//! contract is "pure function of `(prior state, event, author)`",
//! and a state-apply that *trusts* its caller to have already
//! verified is brittle. Verifying twice is cheap (Ed25519 is fast)
//! and removes a class of bug where a future refactor accidentally
//! bypasses the dispatch helper.
//!
//! Field-length checks (`MAX_REASON_LEN`, `MAX_VERSION_LEN`) are
//! mirrored here as a cheap pre-state-machine filter — same rationale
//! as the signature check: the state machine enforces these too, but
//! we don't want a 10 MB `reason` field reaching deserialize-validate
//! at the state tier if we can drop it at the wire boundary.
//!
//! ## Shared verify path
//!
//! Both helpers delegate to [`crate::signed_envelope::verify`]; their
//! distinct existence at this surface is just a typed-call-site
//! convenience (so callers can't accidentally hand a `RevocationEvent`
//! to the publication path or vice versa) — the per-envelope
//! `MAX_*_LEN` cap and the `DOMAIN_SEP_*` constant travel with each
//! type via its `SignedEnvelope` impl and `signing_target()`.
//!
//! `DispatchReject` variants map 1:1 onto `PeerWarning::SignatureInvalid
//! { reason }` consistent with B-4.8 — the kernel-tier subscription
//! wiring (a future task — see spec §6.4) will translate these into
//! `PeerWarning` and emit them via the warnings channel.

use myrhiza_types::AuthorPubkey;

use crate::publication::PublicationEvent;
use crate::revocation::RevocationEvent;
use crate::signed_envelope;

/// Reasons the dispatch layer dropped an inbound envelope before it
/// reached the state machine. Maps 1:1 onto `PeerWarning::SignatureInvalid
/// { reason }` consistent with B-4.8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchReject {
    /// Ed25519 signature verification failed (forged or corrupted).
    SignatureInvalid,
    /// Author pubkey was not a valid Ed25519 curve point.
    AuthorPubkeyMalformed,
    /// `reason` / `version` field exceeded the per-envelope cap.
    FieldTooLong,
}

/// Verify a `RevocationEvent` envelope at the gossip-receive boundary.
///
/// Pure function of `(event, author)`. Performs a cheap field-length
/// check followed by `verify_strict` Ed25519 verification. Returns
/// `Ok(())` if the envelope is safe to hand to
/// [`crate::revocation::RevocationLog::apply`]; returns
/// `Err(DispatchReject::*)` to surface as `PeerWarning::SignatureInvalid`.
///
/// Thin typed wrapper over [`crate::signed_envelope::verify`] —
/// preserved at this surface so callers can't accidentally route a
/// `RevocationEvent` through the publication path.
///
/// # Errors
///
/// - [`DispatchReject::FieldTooLong`] — `reason` exceeds
///   [`crate::revocation::MAX_REASON_LEN`].
/// - [`DispatchReject::AuthorPubkeyMalformed`] — `author` is not a
///   valid Ed25519 curve point.
/// - [`DispatchReject::SignatureInvalid`] — Ed25519 verification failed
///   (forged signature, wrong author, or corrupted envelope).
pub fn verify_revocation(
    event: &RevocationEvent,
    author: &AuthorPubkey,
) -> Result<(), DispatchReject> {
    signed_envelope::verify(event, author)
}

/// Verify a `PublicationEvent` envelope at the gossip-receive boundary.
///
/// Pure function of `(event, author)`. Performs a cheap field-length
/// check followed by `verify_strict` Ed25519 verification. Returns
/// `Ok(())` if the envelope is safe to hand to
/// [`crate::publication::PublicationLog::apply`]; returns
/// `Err(DispatchReject::*)` to surface as `PeerWarning::SignatureInvalid`.
///
/// Thin typed wrapper over [`crate::signed_envelope::verify`] —
/// preserved at this surface so callers can't accidentally route a
/// `PublicationEvent` through the revocation path.
///
/// # Errors
///
/// - [`DispatchReject::FieldTooLong`] — `version` exceeds
///   [`crate::publication::MAX_VERSION_LEN`].
/// - [`DispatchReject::AuthorPubkeyMalformed`] — `author` is not a
///   valid Ed25519 curve point.
/// - [`DispatchReject::SignatureInvalid`] — Ed25519 verification failed
///   (forged signature, wrong author, or corrupted envelope).
pub fn verify_publication(
    event: &PublicationEvent,
    author: &AuthorPubkey,
) -> Result<(), DispatchReject> {
    signed_envelope::verify(event, author)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use myrhiza_types::BlobHash;

    use crate::publication::MAX_VERSION_LEN;
    use crate::revocation::MAX_REASON_LEN;

    fn fixture() -> (SigningKey, AuthorPubkey) {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = AuthorPubkey::from_bytes(sk.verifying_key().to_bytes());
        (sk, pk)
    }

    fn sign_revocation(
        sk: &SigningKey,
        revoked: BlobHash,
        reason: &str,
        seq: u64,
    ) -> RevocationEvent {
        let mut ev = RevocationEvent {
            revoked_bundle_hash: revoked,
            reason: reason.into(),
            revoked_at: 0,
            revocation_seq: seq,
            signature: [0u8; 64],
        };
        let target = ev.signing_target();
        let sig = sk.sign(&target);
        ev.signature = sig.to_bytes();
        ev
    }

    fn sign_publication(
        sk: &SigningKey,
        manifest: BlobHash,
        version: &str,
        seq: u64,
    ) -> PublicationEvent {
        let mut ev = PublicationEvent {
            manifest_hash: manifest,
            version: version.into(),
            publication_seq: seq,
            signature: [0u8; 64],
        };
        let target = ev.signing_target();
        let sig = sk.sign(&target);
        ev.signature = sig.to_bytes();
        ev
    }

    // ---- revocation ----

    #[test]
    fn verify_revocation_accepts_genuine_signature() {
        let (sk, pk) = fixture();
        let ev = sign_revocation(&sk, BlobHash::from_bytes([0xAA; 32]), "compromised", 1);
        verify_revocation(&ev, &pk).expect("genuine signature must verify");
    }

    #[test]
    fn verify_revocation_rejects_bad_signature() {
        let (_, pk) = fixture();
        let wrong_sk = SigningKey::from_bytes(&[42u8; 32]);
        let ev = sign_revocation(&wrong_sk, BlobHash::from_bytes([0xAA; 32]), "x", 1);
        let err = verify_revocation(&ev, &pk).expect_err("must reject bad sig");
        assert_eq!(err, DispatchReject::SignatureInvalid);
    }

    /// A 32-byte sequence guaranteed to fail `VerifyingKey::from_bytes`:
    /// 32 bytes of 0x01 with bit 1 (the second-lowest) set in the
    /// trailing byte — the resulting y-coordinate is a quadratic
    /// non-residue, so curve-point decompression fails. Confirmed
    /// empirically against ed25519-dalek 2.x (see the
    /// `malformed_pk_constant_is_actually_malformed` self-check
    /// below — meta-test guarding against ed25519-dalek changing
    /// its rejection set in a future release).
    const MALFORMED_PK_BYTES: [u8; 32] = {
        let mut b = [0x01u8; 32];
        b[31] = 0x03;
        b
    };

    #[test]
    fn malformed_pk_constant_is_actually_malformed() {
        // Meta-test: if ed25519-dalek ever starts accepting this
        // encoding, the AuthorPubkeyMalformed-path tests below would
        // silently become SignatureInvalid-path tests. This guards.
        use ed25519_dalek::VerifyingKey;
        assert!(VerifyingKey::from_bytes(&MALFORMED_PK_BYTES).is_err());
    }

    #[test]
    fn verify_revocation_rejects_malformed_author_pubkey() {
        let (sk, _) = fixture();
        let ev = sign_revocation(&sk, BlobHash::from_bytes([0xAA; 32]), "x", 1);
        let bad_pk = AuthorPubkey::from_bytes(MALFORMED_PK_BYTES);
        let err = verify_revocation(&ev, &bad_pk).expect_err("must reject malformed pk");
        assert_eq!(err, DispatchReject::AuthorPubkeyMalformed);
    }

    #[test]
    fn verify_revocation_rejects_reason_too_long() {
        let (sk, pk) = fixture();
        let too_long = "x".repeat(MAX_REASON_LEN + 1);
        let ev = sign_revocation(&sk, BlobHash::from_bytes([0xAA; 32]), &too_long, 1);
        let err = verify_revocation(&ev, &pk).expect_err("must reject oversize reason");
        assert_eq!(err, DispatchReject::FieldTooLong);
    }

    // ---- publication ----

    #[test]
    fn verify_publication_accepts_genuine_signature() {
        let (sk, pk) = fixture();
        let ev = sign_publication(&sk, BlobHash::from_bytes([0xAA; 32]), "1.0.0", 1);
        verify_publication(&ev, &pk).expect("genuine signature must verify");
    }

    #[test]
    fn verify_publication_rejects_bad_signature() {
        let (_, pk) = fixture();
        let wrong_sk = SigningKey::from_bytes(&[42u8; 32]);
        let ev = sign_publication(&wrong_sk, BlobHash::from_bytes([0xAA; 32]), "1.0.0", 1);
        let err = verify_publication(&ev, &pk).expect_err("must reject bad sig");
        assert_eq!(err, DispatchReject::SignatureInvalid);
    }

    #[test]
    fn verify_publication_rejects_malformed_author_pubkey() {
        let (sk, _) = fixture();
        let ev = sign_publication(&sk, BlobHash::from_bytes([0xAA; 32]), "1.0.0", 1);
        let bad_pk = AuthorPubkey::from_bytes(MALFORMED_PK_BYTES);
        let err = verify_publication(&ev, &bad_pk).expect_err("must reject malformed pk");
        assert_eq!(err, DispatchReject::AuthorPubkeyMalformed);
    }

    #[test]
    fn verify_publication_rejects_version_too_long() {
        let (sk, pk) = fixture();
        let too_long = "x".repeat(MAX_VERSION_LEN + 1);
        let ev = sign_publication(&sk, BlobHash::from_bytes([0xAA; 32]), &too_long, 1);
        let err = verify_publication(&ev, &pk).expect_err("must reject oversize version");
        assert_eq!(err, DispatchReject::FieldTooLong);
    }
}
