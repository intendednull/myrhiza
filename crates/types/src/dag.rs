//! Event-DAG wire types per convergence.md §4 + §4.7.
//!
//! Each type here has a canonical-bincode byte layout pinned at v1
//! and validated by `crates/types/tests/wire_freeze.rs`. Field order
//! is normative — emitter and verifier MUST encode fields in
//! declaration order.

use serde::{Deserialize, Serialize};

use crate::{AuthorPubkey, EventHash};

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
