//! Event-DAG wire types per convergence.md §4 + §4.7.
//!
//! Each type here has a canonical-bincode byte layout pinned at v1
//! and validated by `crates/types/tests/wire_freeze.rs`. Field order
//! is normative — emitter and verifier MUST encode fields in
//! declaration order.

use serde::{Deserialize, Serialize};

use crate::AuthorPubkey;

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
