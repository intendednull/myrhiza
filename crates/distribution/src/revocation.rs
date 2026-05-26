//! Revocation event schema + log state machine.
//!
//! Per B-10 spec §4.4. Pure-function `apply` — no clock, no network.
//! Mirrors the determinism discipline from `state-apply` components
//! per CLAUDE.md ("State-apply components must be pure functions of
//! `(prior state, event)` plus the deterministic helper set"). The
//! revocation log is a kernel-resident analog with the same purity
//! contract.

use std::collections::BTreeSet;

use bincode::Options;
use myrhiza_types::{AuthorPubkey, BlobHash, canonical_bincode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::dispatch::DispatchReject;
use crate::signed_envelope::{self, SignedEnvelope, serde_bytes_64};

/// Domain-separator string for revocation signatures. Mirrors the
/// manifest-signature framing in `crates/manifest/src/canonical.rs`
/// (where `DOMAIN_SEP = "myrhiza/manifest/v1"`); the domain prefix
/// defends against key-reuse across envelope types if the same author
/// key ever signs heterogeneous payloads. Per B-10 spec §4.4.
pub const DOMAIN_SEP_REVOCATION: &[u8] = b"myrhiza/revocation/v1";

/// Maximum revocation-seq jump per 24h window. Per
/// `docs/specs/2026-05-09-myrhiza-master-design/distribution.md` §10.7.
/// Acts as a flood-protection bound; legitimate authors should never
/// approach this in normal use.
pub const MAX_REVOCATION_JUMP: u64 = 1024;

/// Maximum bytes of `reason` text. Per B-10 spec §4.4. Truncated
/// (not rejected) on encode at the publish side; the receive side
/// just enforces the bound on decode.
pub const MAX_REASON_LEN: usize = 256;

/// Signed revocation envelope.
///
/// Per B-10 spec §4.4. Gossipped on the per-author revocation topic
/// derived by [`derive_revocation_topic`](crate::topic::derive_revocation_topic).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevocationEvent {
    /// The bundle hash being revoked.
    pub revoked_bundle_hash: BlobHash,
    /// Human-readable reason (≤ `MAX_REASON_LEN` bytes).
    pub reason: String,
    /// Author-asserted timestamp (informational; NOT trusted for
    /// ordering — `revocation_seq` is the authority).
    pub revoked_at: u64,
    /// Monotonic-per-author. Kernel rejects out-of-order or duplicate.
    pub revocation_seq: u64,
    /// Ed25519 signature by the author over `DOMAIN_SEP_REVOCATION ||
    /// canonical_bincode(SignedFields)` where `SignedFields` is the
    /// envelope minus the `signature` field.
    #[serde(with = "serde_bytes_64")]
    pub signature: [u8; 64],
}

#[derive(Serialize)]
struct RevocationSignedFields<'a> {
    revoked_bundle_hash: &'a BlobHash,
    reason: &'a str,
    revoked_at: u64,
    revocation_seq: u64,
}

impl RevocationEvent {
    /// Bytes the signature commits to: `DOMAIN_SEP_REVOCATION ||
    /// canonical_bincode(signed_fields)`. Public so publish-side
    /// authors can construct the same bytes for signing.
    ///
    /// # Panics
    ///
    /// `canonical_bincode().serialize` of `RevocationSignedFields`
    /// is infallible in practice — the only failure modes are
    /// `Vec` allocation OOM (which the panic-handler in `panic_abort`
    /// would have caught upstream anyway) and length overflow of
    /// `u32::MAX` (structurally unreachable for a 32-byte hash plus
    /// a ≤256-byte string plus two `u64`s). Mirrors the precedent in
    /// `crates/manifest/src/canonical.rs::signed_body_bytes`.
    #[allow(clippy::expect_used)]
    #[must_use]
    pub fn signing_target(&self) -> Vec<u8> {
        let signed = RevocationSignedFields {
            revoked_bundle_hash: &self.revoked_bundle_hash,
            reason: &self.reason,
            revoked_at: self.revoked_at,
            revocation_seq: self.revocation_seq,
        };
        let mut out = Vec::with_capacity(DOMAIN_SEP_REVOCATION.len() + 128);
        out.extend_from_slice(DOMAIN_SEP_REVOCATION);
        let body = canonical_bincode()
            .serialize(&signed)
            .expect("canonical bincode of RevocationSignedFields never fails");
        out.extend_from_slice(&body);
        out
    }
}

impl SignedEnvelope for RevocationEvent {
    fn signing_target(&self) -> Vec<u8> {
        RevocationEvent::signing_target(self)
    }

    fn signature(&self) -> &[u8; 64] {
        &self.signature
    }

    fn field_too_long(&self) -> bool {
        self.reason.len() > MAX_REASON_LEN
    }
}

/// Errors `RevocationLog::apply` can return.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RevocationError {
    /// Ed25519 signature verification failed.
    #[error("revocation signature verification failed")]
    SignatureInvalid,
    /// Author pubkey was not a valid Ed25519 curve point.
    #[error("author pubkey is not a valid Ed25519 verifying key")]
    AuthorPubkeyMalformed,
    /// `revocation_seq` is not strictly greater than `last_observed_seq`.
    #[error("revocation_seq {got} not greater than last_observed_seq {last_observed}")]
    SeqNotMonotonic {
        /// The seq in the event.
        got: u64,
        /// The last accepted seq for this author.
        last_observed: u64,
    },
    /// `revocation_seq` exceeds `last_observed_seq + MAX_REVOCATION_JUMP`.
    #[error("revocation_seq jump {jump} exceeds MAX_REVOCATION_JUMP={MAX_REVOCATION_JUMP}")]
    SeqJumpTooLarge {
        /// `event.revocation_seq - last_observed_seq`.
        jump: u64,
    },
    /// `reason` exceeds `MAX_REASON_LEN` bytes.
    #[error("reason length {got} exceeds MAX_REASON_LEN={MAX_REASON_LEN}")]
    ReasonTooLong {
        /// Observed length.
        got: usize,
    },
}

/// Per-author revocation log state.
///
/// Per B-10 spec §4.4. Pure-function state machine — `apply`
/// consumes `(self, event)` and returns the next state with no
/// side effects beyond the structurally returned diff.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RevocationLog {
    /// Highest `revocation_seq` accepted so far. Starts at 0;
    /// first accepted event must have `revocation_seq >= 1`.
    pub last_observed_seq: u64,
    /// Set of bundle hashes revoked by this author.
    pub revoked_bundles: BTreeSet<BlobHash>,
}

impl RevocationLog {
    /// Construct an empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a signed `RevocationEvent` to the log.
    ///
    /// Pure function of `(self, event, author)`. Returns either the
    /// updated log (which structurally includes the new revoked
    /// hash) or a `RevocationError` indicating why the event was
    /// rejected. The author pubkey is taken as a parameter rather
    /// than carried in the event because the gossip dispatch layer
    /// already knows which author's topic the event arrived on
    /// (per spec §3.3 — topic is `BLAKE3("myrhiza/revocations/v1"
    /// || author_pubkey)`); requiring the caller to pass it makes
    /// signature-cross-checking impossible to forget.
    ///
    /// # Errors
    ///
    /// Returns `RevocationError::SignatureInvalid`,
    /// `RevocationError::AuthorPubkeyMalformed`,
    /// `RevocationError::SeqNotMonotonic`,
    /// `RevocationError::SeqJumpTooLarge`, or
    /// `RevocationError::ReasonTooLong`. None mutate state — on
    /// `Err`, the caller's `RevocationLog` is unchanged.
    pub fn apply(
        mut self,
        event: &RevocationEvent,
        author: &AuthorPubkey,
    ) -> Result<Self, RevocationError> {
        // Validation sequence per B-10 spec §4.4: length check →
        // seq-monotonic → seq-jump cap → pubkey-decode → verify_strict.
        // The length check is repeated by `signed_envelope::verify`
        // below but must run *first* so its typed error precedes any
        // seq-check error; the verify-side check is a guaranteed
        // no-op once we've passed this gate.
        if event.reason.len() > MAX_REASON_LEN {
            return Err(RevocationError::ReasonTooLong {
                got: event.reason.len(),
            });
        }
        if event.revocation_seq <= self.last_observed_seq {
            return Err(RevocationError::SeqNotMonotonic {
                got: event.revocation_seq,
                last_observed: self.last_observed_seq,
            });
        }
        let jump = event.revocation_seq - self.last_observed_seq;
        if jump > MAX_REVOCATION_JUMP {
            return Err(RevocationError::SeqJumpTooLarge { jump });
        }

        signed_envelope::verify(event, author).map_err(|reject| match reject {
            // FieldTooLong is unreachable here — already gated above
            // with the typed `ReasonTooLong` error. Map defensively
            // to the typed variant so a future reorder of the gates
            // doesn't silently drop the field-length signal.
            DispatchReject::FieldTooLong => RevocationError::ReasonTooLong {
                got: event.reason.len(),
            },
            DispatchReject::AuthorPubkeyMalformed => RevocationError::AuthorPubkeyMalformed,
            DispatchReject::SignatureInvalid => RevocationError::SignatureInvalid,
        })?;

        self.revoked_bundles.insert(event.revoked_bundle_hash);
        self.last_observed_seq = event.revocation_seq;
        Ok(self)
    }

    /// True if `bundle` has been revoked.
    #[must_use]
    pub fn is_revoked(&self, bundle: &BlobHash) -> bool {
        self.revoked_bundles.contains(bundle)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn sign_event(sk: &SigningKey, revoked: BlobHash, reason: &str, seq: u64) -> RevocationEvent {
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

    fn fixture() -> (SigningKey, AuthorPubkey) {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = AuthorPubkey::from_bytes(sk.verifying_key().to_bytes());
        (sk, pk)
    }

    #[test]
    fn applies_first_revocation() {
        let (sk, pk) = fixture();
        let ev = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "compromised", 1);
        let log = RevocationLog::new().apply(&ev, &pk).expect("apply ok");
        assert_eq!(log.last_observed_seq, 1);
        assert!(log.is_revoked(&BlobHash::from_bytes([0xAA; 32])));
    }

    #[test]
    fn rejects_signature_mismatch() {
        let (_, pk) = fixture();
        let wrong_sk = SigningKey::from_bytes(&[42u8; 32]);
        let ev = sign_event(&wrong_sk, BlobHash::from_bytes([0xAA; 32]), "x", 1);
        let err = RevocationLog::new()
            .apply(&ev, &pk)
            .expect_err("must reject");
        assert!(matches!(err, RevocationError::SignatureInvalid));
    }

    #[test]
    fn rejects_out_of_order_seq() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "x", 3);
        let ev2 = sign_event(&sk, BlobHash::from_bytes([0xBB; 32]), "y", 2);
        let log = RevocationLog::new().apply(&ev1, &pk).expect("apply 1");
        let err = log.apply(&ev2, &pk).expect_err("seq 2 < 3 must reject");
        assert!(matches!(err, RevocationError::SeqNotMonotonic { .. }));
    }

    #[test]
    fn rejects_duplicate_seq() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "x", 2);
        let ev2 = sign_event(&sk, BlobHash::from_bytes([0xBB; 32]), "y", 2);
        let log = RevocationLog::new().apply(&ev1, &pk).expect("apply 1");
        let err = log.apply(&ev2, &pk).expect_err("duplicate seq must reject");
        assert!(matches!(err, RevocationError::SeqNotMonotonic { .. }));
    }

    #[test]
    fn rejects_jump_exceeds_max() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "x", 1);
        let ev2 = sign_event(
            &sk,
            BlobHash::from_bytes([0xBB; 32]),
            "y",
            1 + MAX_REVOCATION_JUMP + 1,
        );
        let log = RevocationLog::new().apply(&ev1, &pk).expect("apply 1");
        let err = log.apply(&ev2, &pk).expect_err("jump+1 must reject");
        assert!(matches!(err, RevocationError::SeqJumpTooLarge { .. }));
    }

    #[test]
    fn accepts_jump_at_max() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "x", 1);
        let ev2 = sign_event(
            &sk,
            BlobHash::from_bytes([0xBB; 32]),
            "y",
            1 + MAX_REVOCATION_JUMP,
        );
        let log = RevocationLog::new().apply(&ev1, &pk).expect("apply 1");
        let log = log.apply(&ev2, &pk).expect("max-jump must accept");
        assert_eq!(log.last_observed_seq, 1 + MAX_REVOCATION_JUMP);
    }

    #[test]
    fn idempotent_double_revoke_same_bundle() {
        let (sk, pk) = fixture();
        let hash = BlobHash::from_bytes([0xAA; 32]);
        let ev1 = sign_event(&sk, hash, "first", 1);
        let ev2 = sign_event(&sk, hash, "second", 2);
        let log = RevocationLog::new().apply(&ev1, &pk).expect("apply 1");
        let log = log.apply(&ev2, &pk).expect("apply 2");
        // Bundle still revoked (semantic idempotence — the set
        // doesn't track multi-revoke; a re-revoke is a no-op for
        // membership but still bumps seq).
        assert!(log.is_revoked(&hash));
        assert_eq!(log.last_observed_seq, 2);
    }

    #[test]
    fn rejects_reason_too_long() {
        let (sk, pk) = fixture();
        let too_long = "x".repeat(MAX_REASON_LEN + 1);
        let ev = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), &too_long, 1);
        let err = RevocationLog::new()
            .apply(&ev, &pk)
            .expect_err("must reject");
        assert!(matches!(err, RevocationError::ReasonTooLong { .. }));
    }
}
