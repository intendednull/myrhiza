//! Shared envelope verification machinery for signed gossip events.
//!
//! Per B-10 spec §3.4 (publication) + §4.4 (revocation). Both
//! `PublicationEvent` and `RevocationEvent` share a near-identical
//! shape — a typed body plus a domain-separated Ed25519 signature
//! over `canonical_bincode(SignedFields)`. This module collapses the
//! cross-cutting parts (the shared `[u8; 64]` serde adapter and the
//! field-length + pubkey-decode + `verify_strict` sequence) into one
//! place; the *signing target* bytes themselves still live next to
//! each event's `SignedFields` struct because their field order is
//! part of the on-wire signature contract and must not move.
//!
//! ## Wire-format invariant
//!
//! This module only abstracts *in-memory access patterns* for the
//! verify path. The serialized bytes of `PublicationEvent` and
//! `RevocationEvent` are unchanged by this extraction — the
//! `serde_bytes_64` adapter here is byte-identical to the previously
//! duplicated copies in `publication.rs` and `revocation.rs`, and
//! each event continues to drive its own `SignedFields` shape.

use ed25519_dalek::{Signature as DalekSignature, VerifyingKey};
use myrhiza_types::AuthorPubkey;

use crate::dispatch::DispatchReject;

/// Shared `[u8; 64]` serde adapter, used by both `PublicationEvent`
/// and `RevocationEvent` for their `signature` field.
///
/// Byte-identical to the previously duplicated copies in
/// `publication.rs` and `revocation.rs` — hoisted unchanged so the
/// on-wire encoding of either event type does not shift.
pub(crate) mod serde_bytes_64 {
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

/// Abstraction over an event carrying a domain-separated Ed25519
/// signature plus a variable-length text field with an envelope-
/// specific length cap. Both `PublicationEvent` and `RevocationEvent`
/// implement this; the [`verify`] free function drives them through
/// the identical field-length + pubkey-decode + `verify_strict`
/// sequence that used to be duplicated in `dispatch.rs` and again in
/// each `apply` method.
///
/// **Domain separators stay distinct per impl** — that's the entire
/// point of domain separation. `signing_target()` is responsible for
/// prepending the type-specific `DOMAIN_SEP_*` constant; this trait
/// only abstracts the *call site*, never the *bytes*.
pub trait SignedEnvelope {
    /// Bytes the signature commits to. Implementors prepend their
    /// envelope-specific `DOMAIN_SEP_*` to `canonical_bincode` of
    /// their typed `SignedFields`.
    fn signing_target(&self) -> Vec<u8>;

    /// The 64-byte Ed25519 signature carried by the envelope.
    fn signature(&self) -> &[u8; 64];

    /// True if the envelope's variable-length text field exceeds its
    /// envelope-specific cap (`MAX_VERSION_LEN` for publication,
    /// `MAX_REASON_LEN` for revocation). The caps stay distinct per
    /// envelope per B-10 spec §3.4 + §4.4 — different upper bounds on
    /// different semantic fields.
    fn field_too_long(&self) -> bool;
}

/// Verify a signed envelope at the gossip-receive boundary.
///
/// Pure function of `(event, author)`. Runs the same three gates that
/// used to be inlined in `dispatch::verify_revocation`,
/// `dispatch::verify_publication`, and both `apply` methods:
///
/// 1. Field-length cap (delegated to [`SignedEnvelope::field_too_long`]).
/// 2. Author pubkey decode (Ed25519 curve-point validity).
/// 3. `verify_strict` signature check against the envelope's
///    `signing_target()` bytes.
///
/// Returns `Ok(())` if the envelope is safe to hand to the state
/// machine; returns `Err(DispatchReject::*)` otherwise. Each `apply`
/// in turn maps this into its envelope-specific typed error.
///
/// # Errors
///
/// - [`DispatchReject::FieldTooLong`] — envelope-specific length cap exceeded.
/// - [`DispatchReject::AuthorPubkeyMalformed`] — `author` not a valid
///   Ed25519 curve point.
/// - [`DispatchReject::SignatureInvalid`] — `verify_strict` rejected
///   the signature (forged, wrong author, or corrupted envelope).
pub fn verify<E: SignedEnvelope>(event: &E, author: &AuthorPubkey) -> Result<(), DispatchReject> {
    if event.field_too_long() {
        return Err(DispatchReject::FieldTooLong);
    }
    let vk = VerifyingKey::from_bytes(author.as_bytes())
        .map_err(|_| DispatchReject::AuthorPubkeyMalformed)?;
    let sig = DalekSignature::from_bytes(event.signature());
    let target = event.signing_target();
    vk.verify_strict(&target, &sig)
        .map_err(|_| DispatchReject::SignatureInvalid)?;
    Ok(())
}
