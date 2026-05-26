//! Publication event schema + log state machine.
//!
//! Per B-10 spec §3.4. Pure-function `apply` — no clock, no network.
//! Structurally parallel to [`crate::revocation::RevocationLog`]
//! (same monotonic-seq + signature-verified per-author envelope
//! shape); the divergence is at the state-shape level only — this
//! log tracks `latest_announcement: Option<(BlobHash, String)>`
//! (an informational pointer for the kernel UI surface) rather
//! than a `BTreeSet<BlobHash>` of presence-or-absence.
//!
//! Mirrors the determinism discipline from `state-apply` components
//! per CLAUDE.md ("State-apply components must be pure functions of
//! `(prior state, event)` plus the deterministic helper set"). The
//! publication log is a kernel-resident analog with the same purity
//! contract.

use bincode::Options;
use ed25519_dalek::{Signature as DalekSignature, VerifyingKey};
use myrhiza_types::{AuthorPubkey, BlobHash, canonical_bincode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Domain-separator string for publication signatures. Mirrors the
/// manifest-signature framing in `crates/manifest/src/canonical.rs`
/// (where `DOMAIN_SEP = "myrhiza/manifest/v1"`) and the revocation
/// framing in [`crate::revocation::DOMAIN_SEP_REVOCATION`]; the
/// domain prefix defends against key-reuse across envelope types
/// if the same author key ever signs heterogeneous payloads.
/// Per B-10 spec §3.4.
pub const DOMAIN_SEP_PUBLICATION: &[u8] = b"myrhiza/publication/v1";

/// Maximum publication-seq jump per author. Acts as a flood-protection
/// bound; legitimate authors should never approach this in normal use.
/// Mirrors `MAX_REVOCATION_JUMP` (per B-10 spec §3.4 — publication and
/// revocation use the same monotonic-seq semantics).
pub const MAX_PUBLICATION_JUMP: u64 = 1024;

/// Maximum bytes of `version` text. Per B-10 spec §3.4 + §6.2
/// ("version-string truncation policy"). Truncated (not rejected)
/// on encode at the publish side; the receive side just enforces
/// the bound on decode.
pub const MAX_VERSION_LEN: usize = 64;

/// Signed publication envelope.
///
/// Per B-10 spec §3.4. Gossipped on the per-author publication topic
/// derived by [`derive_publication_topic`](crate::topic::derive_publication_topic).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationEvent {
    /// iroh-blobs hash of the canonical-bincode manifest being announced.
    pub manifest_hash: BlobHash,
    /// Version string (≤ `MAX_VERSION_LEN` bytes). Same value embedded
    /// in the manifest's `[app]` section; carried here so peers can
    /// render a release notification before fetching the manifest bytes.
    pub version: String,
    /// Monotonic-per-author. Kernel rejects out-of-order or duplicate.
    pub publication_seq: u64,
    /// Ed25519 signature by the author over `DOMAIN_SEP_PUBLICATION ||
    /// canonical_bincode(SignedFields)` where `SignedFields` is the
    /// envelope minus the `signature` field.
    #[serde(with = "serde_bytes_64")]
    pub signature: [u8; 64],
}

mod serde_bytes_64 {
    use core::fmt;

    use serde::{
        Deserializer, Serializer,
        de::{SeqAccess, Visitor},
        ser::SerializeTuple,
    };

    pub fn serialize<S: Serializer>(bytes: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        let mut t = s.serialize_tuple(64)?;
        for b in bytes {
            t.serialize_element(b)?;
        }
        t.end()
    }

    struct ArrayVisitor;

    impl<'de> Visitor<'de> for ArrayVisitor {
        type Value = [u8; 64];

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "an array of 64 bytes")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = [0u8; 64];
            for (i, slot) in out.iter_mut().enumerate() {
                *slot = seq
                    .next_element::<u8>()?
                    .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
            }
            Ok(out)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        d.deserialize_tuple(64, ArrayVisitor)
    }
}

#[derive(Serialize)]
struct PublicationSignedFields<'a> {
    manifest_hash: &'a BlobHash,
    version: &'a str,
    publication_seq: u64,
}

impl PublicationEvent {
    /// Bytes the signature commits to: `DOMAIN_SEP_PUBLICATION ||
    /// canonical_bincode(signed_fields)`. Public so publish-side
    /// authors can construct the same bytes for signing.
    ///
    /// # Panics
    ///
    /// `canonical_bincode().serialize` of `PublicationSignedFields`
    /// is infallible in practice — the only failure modes are
    /// `Vec` allocation OOM (which the panic-handler in `panic_abort`
    /// would have caught upstream anyway) and length overflow of
    /// `u32::MAX` (structurally unreachable for a 32-byte hash plus
    /// a ≤64-byte string plus a `u64`). Mirrors the precedent in
    /// `crates/manifest/src/canonical.rs::signed_body_bytes`.
    #[allow(clippy::expect_used)]
    #[must_use]
    pub fn signing_target(&self) -> Vec<u8> {
        let signed = PublicationSignedFields {
            manifest_hash: &self.manifest_hash,
            version: &self.version,
            publication_seq: self.publication_seq,
        };
        let mut out = Vec::with_capacity(DOMAIN_SEP_PUBLICATION.len() + 128);
        out.extend_from_slice(DOMAIN_SEP_PUBLICATION);
        let body = canonical_bincode()
            .serialize(&signed)
            .expect("canonical bincode of PublicationSignedFields never fails");
        out.extend_from_slice(&body);
        out
    }
}

/// Errors `PublicationLog::apply` can return.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PublicationError {
    /// Ed25519 signature verification failed.
    #[error("publication signature verification failed")]
    SignatureInvalid,
    /// Author pubkey was not a valid Ed25519 curve point.
    #[error("author pubkey is not a valid Ed25519 verifying key")]
    AuthorPubkeyMalformed,
    /// `publication_seq` is not strictly greater than `last_observed_seq`.
    #[error("publication_seq {got} not greater than last_observed_seq {last_observed}")]
    SeqNotMonotonic {
        /// The seq in the event.
        got: u64,
        /// The last accepted seq for this author.
        last_observed: u64,
    },
    /// `publication_seq` exceeds `last_observed_seq + MAX_PUBLICATION_JUMP`.
    #[error("publication_seq jump {jump} exceeds MAX_PUBLICATION_JUMP={MAX_PUBLICATION_JUMP}")]
    SeqJumpTooLarge {
        /// `event.publication_seq - last_observed_seq`.
        jump: u64,
    },
    /// `version` exceeds `MAX_VERSION_LEN` bytes.
    #[error("version length {got} exceeds MAX_VERSION_LEN={MAX_VERSION_LEN}")]
    VersionTooLong {
        /// Observed length.
        got: usize,
    },
}

/// Per-author publication log state.
///
/// Per B-10 spec §3.4. Pure-function state machine — `apply`
/// consumes `(self, event)` and returns the next state with no
/// side effects beyond the structurally returned diff.
///
/// **State-shape divergence from `RevocationLog`** (intentional):
/// publications track only the *latest* announcement
/// (`latest_announcement: Option<(BlobHash, String)>`) — the
/// kernel UI surface needs "what's the newest version this author
/// has shipped?" not "every version they ever shipped". This is
/// cumulative-pointer semantics, distinct from revocation's
/// `BTreeSet<BlobHash>` presence-or-absence semantics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PublicationLog {
    /// Highest `publication_seq` accepted so far. Starts at 0;
    /// first accepted event must have `publication_seq >= 1`.
    pub last_observed_seq: u64,
    /// The most recently announced `(manifest_hash, version)` for
    /// this author, or `None` if no events have been applied.
    /// Updated on every accepted event.
    pub latest_announcement: Option<(BlobHash, String)>,
}

impl PublicationLog {
    /// Construct an empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a signed `PublicationEvent` to the log.
    ///
    /// Pure function of `(self, event, author)`. Returns either the
    /// updated log (which structurally includes the new latest
    /// announcement) or a `PublicationError` indicating why the
    /// event was rejected. The author pubkey is taken as a parameter
    /// rather than carried in the event because the gossip dispatch
    /// layer already knows which author's topic the event arrived
    /// on (per spec §3.4 — topic is `BLAKE3("myrhiza/publications/v1"
    /// || author_pubkey)`); requiring the caller to pass it makes
    /// signature-cross-checking impossible to forget.
    ///
    /// # Errors
    ///
    /// Returns `PublicationError::SignatureInvalid`,
    /// `PublicationError::AuthorPubkeyMalformed`,
    /// `PublicationError::SeqNotMonotonic`,
    /// `PublicationError::SeqJumpTooLarge`, or
    /// `PublicationError::VersionTooLong`. None mutate state — on
    /// `Err`, the caller's `PublicationLog` is unchanged.
    pub fn apply(
        mut self,
        event: &PublicationEvent,
        author: &AuthorPubkey,
    ) -> Result<Self, PublicationError> {
        if event.version.len() > MAX_VERSION_LEN {
            return Err(PublicationError::VersionTooLong {
                got: event.version.len(),
            });
        }
        if event.publication_seq <= self.last_observed_seq {
            return Err(PublicationError::SeqNotMonotonic {
                got: event.publication_seq,
                last_observed: self.last_observed_seq,
            });
        }
        let jump = event.publication_seq - self.last_observed_seq;
        if jump > MAX_PUBLICATION_JUMP {
            return Err(PublicationError::SeqJumpTooLarge { jump });
        }

        let vk = VerifyingKey::from_bytes(author.as_bytes())
            .map_err(|_| PublicationError::AuthorPubkeyMalformed)?;
        let sig = DalekSignature::from_bytes(&event.signature);
        let target = event.signing_target();
        vk.verify_strict(&target, &sig)
            .map_err(|_| PublicationError::SignatureInvalid)?;

        self.latest_announcement = Some((event.manifest_hash, event.version.clone()));
        self.last_observed_seq = event.publication_seq;
        Ok(self)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn sign_event(
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

    fn fixture() -> (SigningKey, AuthorPubkey) {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = AuthorPubkey::from_bytes(sk.verifying_key().to_bytes());
        (sk, pk)
    }

    #[test]
    fn applies_first_publication() {
        let (sk, pk) = fixture();
        let ev = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "1.0.0", 1);
        let log = PublicationLog::new().apply(&ev, &pk).expect("apply ok");
        assert_eq!(log.last_observed_seq, 1);
        assert_eq!(
            log.latest_announcement,
            Some((BlobHash::from_bytes([0xAA; 32]), "1.0.0".into()))
        );
    }

    #[test]
    fn rejects_signature_mismatch() {
        let (_, pk) = fixture();
        let wrong_sk = SigningKey::from_bytes(&[42u8; 32]);
        let ev = sign_event(&wrong_sk, BlobHash::from_bytes([0xAA; 32]), "1.0.0", 1);
        let err = PublicationLog::new()
            .apply(&ev, &pk)
            .expect_err("must reject");
        assert!(matches!(err, PublicationError::SignatureInvalid));
    }

    #[test]
    fn rejects_out_of_order_seq() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "1.0.0", 3);
        let ev2 = sign_event(&sk, BlobHash::from_bytes([0xBB; 32]), "1.0.1", 2);
        let log = PublicationLog::new().apply(&ev1, &pk).expect("apply 1");
        let err = log.apply(&ev2, &pk).expect_err("seq 2 < 3 must reject");
        assert!(matches!(err, PublicationError::SeqNotMonotonic { .. }));
    }

    #[test]
    fn rejects_duplicate_seq() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "1.0.0", 2);
        let ev2 = sign_event(&sk, BlobHash::from_bytes([0xBB; 32]), "1.0.1", 2);
        let log = PublicationLog::new().apply(&ev1, &pk).expect("apply 1");
        let err = log.apply(&ev2, &pk).expect_err("duplicate seq must reject");
        assert!(matches!(err, PublicationError::SeqNotMonotonic { .. }));
    }

    #[test]
    fn rejects_jump_exceeds_max() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "1.0.0", 1);
        let ev2 = sign_event(
            &sk,
            BlobHash::from_bytes([0xBB; 32]),
            "1.0.1",
            1 + MAX_PUBLICATION_JUMP + 1,
        );
        let log = PublicationLog::new().apply(&ev1, &pk).expect("apply 1");
        let err = log.apply(&ev2, &pk).expect_err("jump+1 must reject");
        assert!(matches!(err, PublicationError::SeqJumpTooLarge { .. }));
    }

    #[test]
    fn accepts_jump_at_max() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "1.0.0", 1);
        let ev2 = sign_event(
            &sk,
            BlobHash::from_bytes([0xBB; 32]),
            "1.0.1",
            1 + MAX_PUBLICATION_JUMP,
        );
        let log = PublicationLog::new().apply(&ev1, &pk).expect("apply 1");
        let log = log.apply(&ev2, &pk).expect("max-jump must accept");
        assert_eq!(log.last_observed_seq, 1 + MAX_PUBLICATION_JUMP);
    }

    #[test]
    fn rejects_version_too_long() {
        let (sk, pk) = fixture();
        let too_long = "x".repeat(MAX_VERSION_LEN + 1);
        let ev = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), &too_long, 1);
        let err = PublicationLog::new()
            .apply(&ev, &pk)
            .expect_err("must reject");
        assert!(matches!(err, PublicationError::VersionTooLong { .. }));
    }

    #[test]
    fn latest_announcement_updates_on_accept() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "1.0.0", 1);
        let ev2 = sign_event(&sk, BlobHash::from_bytes([0xBB; 32]), "1.0.1", 2);
        let log = PublicationLog::new().apply(&ev1, &pk).expect("apply 1");
        assert_eq!(
            log.latest_announcement,
            Some((BlobHash::from_bytes([0xAA; 32]), "1.0.0".into()))
        );
        let log = log.apply(&ev2, &pk).expect("apply 2");
        assert_eq!(
            log.latest_announcement,
            Some((BlobHash::from_bytes([0xBB; 32]), "1.0.1".into()))
        );
        assert_eq!(log.last_observed_seq, 2);
    }
}
