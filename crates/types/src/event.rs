//! Event envelope.
//!
//! Per [convergence.md §4]: per-author signed Merkle DAG. The
//! signature signs the BLAKE3 hash of the SIGNED BODY (every field
//! except `signature`), exposed via [`Event::hash_signed_body`]. The
//! DAG node identifier is the BLAKE3 hash of the FULL canonical
//! envelope (including signature), exposed via [`Event::wire_hash`].
//!
//! Rejecting events whose `signature` bytes are not 64 bytes is
//! enforced at decode time by bincode's fixint length prefix via the
//! 64-byte `try_into` in [`serde_signature::deserialize`].

use std::collections::BTreeSet;

use bincode::Options;
use serde::{Deserialize, Serialize};

use crate::{AuthorPubkey, EventHash, Hlc, canonical_bincode};

/// The full event envelope, including signature.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Event {
    /// Author's Ed25519 pubkey.
    pub author: AuthorPubkey,
    /// Monotonic per-author, starting at 1.
    pub seq: u64,
    /// Hash of this author's previous event. `None` iff `seq == 1`.
    pub prev: Option<EventHash>,
    /// Cross-author causal heads. `BTreeSet` for canonical encoding
    /// order (sorted by `EventHash` byte-lex).
    pub deps: BTreeSet<EventHash>,
    /// Hybrid logical clock at origination.
    pub hlc: Hlc,
    /// App-opaque payload bytes.
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
    /// Ed25519 signature over BLAKE3 of the signed body.
    #[serde(with = "serde_signature")]
    pub signature: [u8; 64],
}

mod serde_signature {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(b: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        serde_bytes::Bytes::new(b).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v: serde_bytes::ByteBuf = serde_bytes::ByteBuf::deserialize(d)?;
        let arr: [u8; 64] = v
            .as_ref()
            .try_into()
            .map_err(|_| serde::de::Error::invalid_length(v.len(), &"64 bytes"))?;
        Ok(arr)
    }
}

/// Mirror of [`Event`] minus `signature`. Used to derive the BLAKE3
/// hash that the signature signs. Field order MUST match `Event`'s
/// declared field order so that the canonical bincode encoding of
/// `SignedBody` is exactly the prefix of the canonical encoding of
/// `Event` up to (but not including) the signature bytes.
#[derive(Serialize)]
struct SignedBody<'a> {
    author: &'a AuthorPubkey,
    seq: u64,
    prev: &'a Option<EventHash>,
    deps: &'a BTreeSet<EventHash>,
    hlc: &'a Hlc,
    #[serde(with = "serde_bytes")]
    payload: &'a [u8],
}

impl Event {
    /// Returns BLAKE3 of the canonical encoding of every field except
    /// `signature`. This is the value that `signature` is over.
    ///
    /// # Panics
    ///
    /// Cannot panic in practice: canonical bincode is infallible for
    /// the fixed, fully-owned schema of `SignedBody` (no `io::Write`,
    /// no recursion, no untyped sizes). A panic would indicate an
    /// upstream bincode bug.
    #[allow(clippy::expect_used)]
    #[must_use]
    pub fn hash_signed_body(&self) -> EventHash {
        let body = SignedBody {
            author: &self.author,
            seq: self.seq,
            prev: &self.prev,
            deps: &self.deps,
            hlc: &self.hlc,
            payload: &self.payload,
        };
        let bytes = canonical_bincode()
            .serialize(&body)
            .expect("canonical bincode of SignedBody never fails");
        EventHash::blake3(&bytes)
    }

    /// Returns BLAKE3 of the canonical encoding of the FULL event
    /// (including signature). This is the wire-content hash used as
    /// the DAG node identifier.
    ///
    /// # Panics
    ///
    /// Cannot panic in practice: see [`Event::hash_signed_body`] —
    /// same infallibility argument. A panic would indicate an
    /// upstream bincode bug.
    #[allow(clippy::expect_used)]
    #[must_use]
    pub fn wire_hash(&self) -> EventHash {
        let bytes = canonical_bincode()
            .serialize(self)
            .expect("canonical bincode of Event never fails");
        EventHash::blake3(&bytes)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use bincode::Options;
    use std::collections::BTreeSet;

    fn sample_event() -> Event {
        Event {
            author: AuthorPubkey::from_bytes([1; 32]),
            seq: 1,
            prev: None,
            deps: BTreeSet::new(),
            hlc: Hlc {
                wall_ms: 1_700_000_000_000,
                logical: 0,
            },
            payload: vec![0x01, 0x02, 0x03],
            signature: [0xFF; 64],
        }
    }

    #[test]
    fn event_round_trips_via_canonical_bincode() {
        let e = sample_event();
        let bytes = crate::canonical_bincode().serialize(&e).expect("encode");
        let decoded: Event = crate::canonical_bincode()
            .deserialize(&bytes)
            .expect("decode");
        assert_eq!(e, decoded);
    }

    #[test]
    fn event_hash_excludes_signature() {
        let mut e1 = sample_event();
        let h1 = e1.hash_signed_body();
        e1.signature = [0x00; 64];
        let h2 = e1.hash_signed_body();
        assert_eq!(
            h1, h2,
            "hash_signed_body must NOT depend on signature bytes"
        );
    }

    #[test]
    fn event_hash_distinct_for_distinct_payload() {
        let e1 = sample_event();
        let mut e2 = e1.clone();
        e2.payload = vec![0xFF, 0xFF];
        assert_ne!(e1.hash_signed_body(), e2.hash_signed_body());
    }

    #[test]
    fn deps_sorted_by_btreeset_iteration() {
        let mut deps = BTreeSet::new();
        deps.insert(EventHash::from_bytes([2; 32]));
        deps.insert(EventHash::from_bytes([1; 32]));
        let collected: Vec<_> = deps.iter().collect();
        assert!(collected[0] < collected[1]);
    }
}
