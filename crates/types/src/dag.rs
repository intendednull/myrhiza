//! Event-DAG wire types per convergence.md §4 + §4.7.
//!
//! Each type here has a canonical-bincode byte layout pinned at v1
//! and validated by `crates/types/tests/wire_freeze.rs`. Field order
//! is normative — emitter and verifier MUST encode fields in
//! declaration order.

use serde::{Deserialize, Serialize};

use crate::{AuthorPubkey, EventHash, PeerPubkey, Topic};

/// Genesis event payload (the bytes inside `Event::payload` when
/// `event.seq == 1`).
///
/// Per convergence.md §4.6 + plan-B-1 spec §4.2 step 3: strictly
/// decoded via [`crate::decode_canonical`] — no trailing bytes
/// permitted. Apps embed app-specific initialization data inside
/// [`Self::app_payload`]; there is no "prefix" convention.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct GenesisV1 {
    /// 32-byte random seed contributed by the founder. Mixed into the
    /// app's deterministic RNG when the kernel applies the genesis
    /// event.
    pub seed: [u8; 32],
    /// Ed25519 pubkey of the founder — the author of this genesis
    /// event. Duplicated here (alongside `Event::author`) so the
    /// payload is self-describing for offline / archived inspection.
    pub founder_pubkey: AuthorPubkey,
    /// App-opaque initialization bytes. Interpreted exclusively by the
    /// app's `state-apply` component; the kernel treats this as
    /// opaque.
    #[serde(with = "serde_bytes")]
    pub app_payload: Vec<u8>,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::canonical_bincode;
    use bincode::Options;

    #[test]
    fn genesis_v1_round_trips_canonical() {
        let g = GenesisV1 {
            seed: [0x55; 32],
            founder_pubkey: AuthorPubkey::from_bytes([0x11; 32]),
            app_payload: vec![0xCA, 0xFE],
        };
        let bytes = canonical_bincode().serialize(&g).expect("encode");
        let decoded: GenesisV1 = canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(g, decoded);
    }

    #[test]
    fn genesis_v1_strict_decode_rejects_trailing_bytes() {
        let g = GenesisV1 {
            seed: [0x55; 32],
            founder_pubkey: AuthorPubkey::from_bytes([0x11; 32]),
            app_payload: vec![],
        };
        let mut bytes = canonical_bincode().serialize(&g).expect("encode");
        bytes.push(0xFF); // trailing byte
        let result = crate::decode_canonical::<GenesisV1>(&bytes);
        assert!(
            result.is_err(),
            "decode_canonical must reject trailing bytes"
        );
    }
}

/// (author, max-seq-seen-for-author) pair used in [`DriftAnchor`] and
/// `HeadsSummary` diff. Sorted by author pubkey byte-lex for canonical
/// encoding per spec §8.1.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct AuthorSeq {
    /// Ed25519 pubkey identifying the author this entry summarizes.
    pub author: AuthorPubkey,
    /// Highest sequence number observed for `author` at the point this
    /// anchor / summary was emitted.
    pub max_seq: u64,
}

/// Canonical "after this event" point for drift detection per
/// convergence.md §4.7.
///
/// `event_hash` is informative metadata (the event that triggered
/// emission) and MUST NOT participate in anchor equality — anchor
/// identity is `author_seq_vec` only (see plan-B-1 spec §8.4).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DriftAnchor {
    /// Informative metadata: the event that triggered emission of this
    /// anchor. Excluded from anchor equality (see type-level docs).
    pub event_hash: EventHash,
    /// Per-author max-seq tuples. MUST be sorted by author pubkey
    /// byte-lex for canonical encoding (spec §8.1).
    pub author_seq_vec: Vec<AuthorSeq>,
}

/// Per-author DAG-tip used in `HeadsSummary` (spec §4.2 §7.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AuthorHead {
    /// Ed25519 pubkey identifying the author whose tip this records.
    pub author: AuthorPubkey,
    /// Sequence number of the tip event for `author`.
    pub seq: u64,
    /// Content hash of the tip event for `author`.
    pub hash: EventHash,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests_anchor {
    use super::*;
    use crate::canonical_bincode;
    use bincode::Options;

    #[test]
    fn drift_anchor_round_trips_canonical() {
        let a = DriftAnchor {
            event_hash: EventHash::blake3(b"e"),
            author_seq_vec: vec![
                AuthorSeq {
                    author: AuthorPubkey::from_bytes([1; 32]),
                    max_seq: 5,
                },
                AuthorSeq {
                    author: AuthorPubkey::from_bytes([2; 32]),
                    max_seq: 3,
                },
            ],
        };
        let bytes = canonical_bincode().serialize(&a).expect("encode");
        let decoded: DriftAnchor = canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(a, decoded);
    }

    #[test]
    fn author_head_round_trips_canonical() {
        let h = AuthorHead {
            author: AuthorPubkey::from_bytes([7; 32]),
            seq: 42,
            hash: EventHash::blake3(b"head"),
        };
        let bytes = canonical_bincode().serialize(&h).expect("encode");
        let decoded: AuthorHead = canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(h, decoded);
    }
}

/// Drift-message wire shape per convergence.md §4.7 + plan-B-1 spec §8.1.
///
/// The `signature` field covers [`DriftSignedPayload`] canonical bytes
/// (NOT the full `DriftMessage`); see spec §8.1.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriftMessage {
    /// "After this event" point being announced; informative
    /// `event_hash` plus per-author max-seq tuples.
    pub anchor: DriftAnchor,
    /// Digest of the state-equivalence summary at `anchor`. Format
    /// identified by [`Self::digest_format`].
    pub digest: [u8; 32],
    /// Format identifier for [`Self::digest`] (e.g. `"bincode-1.3"`).
    /// Versioned to allow future digest-scheme migration.
    pub digest_format: String,
    /// Ed25519 pubkey of the peer that emitted this drift message.
    /// Excluded from the signed payload — the signer asserts the
    /// (`anchor`, `digest`, `digest_format`) triple, not the emitter
    /// identity.
    pub signed_by_peer: PeerPubkey,
    /// Ed25519 signature over the canonical bincode encoding of
    /// [`DriftSignedPayload`] constructed from this message's first
    /// three fields. See spec §8.1.
    #[serde(with = "crate::serde_helpers::serde_signature_64")]
    pub signature: [u8; 64],
}

/// Exact byte target signed by [`DriftMessage::signature`].
///
/// Field order matches the first three fields of [`DriftMessage`] —
/// emit-side and verify-side MUST construct this struct identically
/// to produce the same canonical-bincode bytes (spec §8.1 normative).
/// `signed_by_peer` and `signature` are excluded from the signed
/// payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriftSignedPayload {
    /// Mirrors [`DriftMessage::anchor`].
    pub anchor: DriftAnchor,
    /// Mirrors [`DriftMessage::digest`].
    pub digest: [u8; 32],
    /// Mirrors [`DriftMessage::digest_format`].
    pub digest_format: String,
}

/// `HeadsSummary` per convergence.md §4.2 + B-4.2 Q-4 attribution.
///
/// Periodic per-author DAG-tip snapshot used by peers to detect when
/// they are behind on some authors and need to issue [`HeadsRequest`].
///
/// **B-4.2 wire shape**: `signed_by_peer` + `signature` carry
/// peer-level attribution (mirroring [`DriftMessage`]'s pattern at
/// `dag.rs:150-193`). The signature covers
/// [`HeadsSummarySignedPayload`] canonical bytes — NOT the message
/// itself — and includes a `topic` field (signed-only, NOT on the
/// wire) to prevent cross-topic replay.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadsSummary {
    /// Per-author DAG-tip entries. Order is significant for canonical
    /// encoding; consult §7.1 for the sorting rule.
    pub authors: Vec<AuthorHead>,
    /// Version of the kernel's fuel table at the time of emission.
    /// Recipients with a different version know their pre-check
    /// metering may diverge from the authority verdict.
    pub kernel_fuel_table_version: u32,
    /// Ed25519 pubkey of the peer that emitted this summary. Excluded
    /// from the signed payload — the signer asserts the
    /// (`authors`, `kernel_fuel_table_version`, `topic`) triple, not
    /// the emitter identity. Per B-4.2 spec §3.0.
    pub signed_by_peer: PeerPubkey,
    /// Ed25519 signature over the canonical bincode encoding of
    /// [`HeadsSummarySignedPayload`] constructed from this message's
    /// first two fields plus the subscription's `topic`. See spec
    /// §3.0 for the topic-binding rationale.
    #[serde(with = "crate::serde_helpers::serde_signature_64")]
    pub signature: [u8; 64],
}

/// Exact byte target signed by [`HeadsSummary::signature`].
///
/// Field order: the first two fields mirror [`HeadsSummary`] (so
/// emit-side and verify-side construct identical leading bytes);
/// `topic` is appended last and is integrity-protected-only (NOT
/// carried on the wire — the recipient reconstructs `topic` from
/// the subscription context). Per B-4.2 spec §2 "Topic-binding
/// location" + §3.0.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadsSummarySignedPayload {
    /// Mirrors [`HeadsSummary::authors`].
    pub authors: Vec<AuthorHead>,
    /// Mirrors [`HeadsSummary::kernel_fuel_table_version`].
    pub kernel_fuel_table_version: u32,
    /// Topic this summary applies to. Integrity-protected only —
    /// NOT on the wire (the recipient reconstructs from subscription
    /// context). Per spec §3.0.
    pub topic: Topic,
}

/// Range request issued by a peer that detected it is behind on some
/// authors via a [`HeadsSummary`] diff.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventRequest {
    /// Author whose event-range the requester wants to receive.
    pub author: AuthorPubkey,
    /// Inclusive lower bound on the requested sequence range.
    pub from_seq: u64,
    /// Inclusive upper bound on the requested sequence range.
    pub to_seq: u64,
}

/// Bundle of [`EventRequest`] values sent in a single wire message.
///
/// **B-4.2 wire shape**: `signed_by_peer` + `signature` carry
/// peer-level attribution. The signature covers
/// [`HeadsRequestSignedPayload`] canonical bytes including a `topic`
/// field (signed-only, NOT on the wire) to prevent cross-topic
/// replay. Per B-4.2 spec §3.0.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadsRequest {
    /// Range requests included in this bundle. Recipients SHOULD treat
    /// the bundle as a unit but MAY service entries independently.
    pub requests: Vec<EventRequest>,
    /// Ed25519 pubkey of the peer that emitted this request. Excluded
    /// from the signed payload. Per B-4.2 spec §3.0.
    pub signed_by_peer: PeerPubkey,
    /// Ed25519 signature over the canonical bincode encoding of
    /// [`HeadsRequestSignedPayload`].
    #[serde(with = "crate::serde_helpers::serde_signature_64")]
    pub signature: [u8; 64],
}

/// Exact byte target signed by [`HeadsRequest::signature`].
///
/// Field order: `requests` mirrors [`HeadsRequest`]; `topic` is
/// appended last and is integrity-protected-only (NOT on the wire).
/// Per B-4.2 spec §3.0.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadsRequestSignedPayload {
    /// Mirrors [`HeadsRequest::requests`].
    pub requests: Vec<EventRequest>,
    /// Topic this request applies to. Integrity-protected only.
    pub topic: Topic,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests_drift_heads {
    use super::*;
    use crate::canonical_bincode;
    use bincode::Options;

    #[test]
    fn drift_signed_payload_round_trips() {
        let p = DriftSignedPayload {
            anchor: DriftAnchor {
                event_hash: EventHash::blake3(b"e"),
                author_seq_vec: vec![],
            },
            digest: [0xAA; 32],
            digest_format: "bincode-1.3".into(),
        };
        let bytes = canonical_bincode().serialize(&p).expect("encode");
        let decoded: DriftSignedPayload = canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(decoded.digest, p.digest);
        assert_eq!(decoded.digest_format, p.digest_format);
    }

    #[test]
    fn drift_message_round_trips() {
        let m = DriftMessage {
            anchor: DriftAnchor {
                event_hash: EventHash::ZERO,
                author_seq_vec: vec![],
            },
            digest: [0x33; 32],
            digest_format: "bincode-1.3".into(),
            signed_by_peer: PeerPubkey::from_bytes([0x44; 32]),
            signature: [0x55; 64],
        };
        let bytes = canonical_bincode().serialize(&m).expect("encode");
        let _decoded: DriftMessage = canonical_bincode().deserialize(&bytes).expect("decode");
    }

    #[test]
    fn heads_summary_round_trips() {
        let h = HeadsSummary {
            authors: vec![AuthorHead {
                author: AuthorPubkey::from_bytes([1; 32]),
                seq: 7,
                hash: EventHash::ZERO,
            }],
            kernel_fuel_table_version: 1,
            signed_by_peer: PeerPubkey::from_bytes([0; 32]),
            signature: [0; 64],
        };
        let bytes = canonical_bincode().serialize(&h).expect("encode");
        let decoded: HeadsSummary = canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(decoded.kernel_fuel_table_version, 1);
        assert_eq!(decoded.authors.len(), 1);
    }

    #[test]
    fn heads_request_round_trips() {
        let r = HeadsRequest {
            requests: vec![EventRequest {
                author: AuthorPubkey::from_bytes([8; 32]),
                from_seq: 1,
                to_seq: 10,
            }],
            signed_by_peer: PeerPubkey::from_bytes([0; 32]),
            signature: [0; 64],
        };
        let bytes = canonical_bincode().serialize(&r).expect("encode");
        let decoded: HeadsRequest = canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(decoded.requests.len(), 1);
    }

    #[test]
    fn heads_summary_signed_payload_round_trips() {
        let p = HeadsSummarySignedPayload {
            authors: vec![AuthorHead {
                author: AuthorPubkey::from_bytes([7; 32]),
                seq: 42,
                hash: EventHash::blake3(b"head"),
            }],
            kernel_fuel_table_version: 1,
            topic: crate::Topic::from_bytes([0xAB; 32]),
        };
        let bytes = canonical_bincode().serialize(&p).expect("encode");
        let decoded: HeadsSummarySignedPayload =
            canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(p.authors, decoded.authors);
        assert_eq!(
            p.kernel_fuel_table_version,
            decoded.kernel_fuel_table_version
        );
        assert_eq!(p.topic, decoded.topic);
    }

    #[test]
    fn heads_request_signed_payload_round_trips() {
        let p = HeadsRequestSignedPayload {
            requests: vec![EventRequest {
                author: AuthorPubkey::from_bytes([8; 32]),
                from_seq: 1,
                to_seq: 10,
            }],
            topic: crate::Topic::from_bytes([0xCD; 32]),
        };
        let bytes = canonical_bincode().serialize(&p).expect("encode");
        let decoded: HeadsRequestSignedPayload =
            canonical_bincode().deserialize(&bytes).expect("decode");
        // EventRequest does not derive PartialEq — compare fields individually.
        assert_eq!(decoded.requests.len(), p.requests.len());
        assert_eq!(decoded.requests[0].author, p.requests[0].author);
        assert_eq!(decoded.requests[0].from_seq, p.requests[0].from_seq);
        assert_eq!(decoded.requests[0].to_seq, p.requests[0].to_seq);
        assert_eq!(p.topic, decoded.topic);
    }
}
