//! Hybrid Logical Clock.
//!
//! Signed into events by the originator. Extracted by every peer
//! deterministically via `host.now-hlc-from-event` per
//! [determinism.md §5.1]. NOT used for DAG ordering or topo-sort
//! tie-break (per [convergence.md §4.1]); materialized into derived
//! state where useful.

use serde::{Deserialize, Serialize};

/// Hybrid logical clock signed into events by the originator.
///
/// `Ord` is lexicographic on `(wall_ms, logical)`. This is convenient
/// for derived state but is *not* normative for DAG ordering — see
/// [convergence.md §4.1] which uses `EventHash` byte-lex for tie-break.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct Hlc {
    /// Wall-clock milliseconds since UNIX epoch (signed by originator).
    pub wall_ms: u64,
    /// Per-(peer, ms) logical counter. Resets to 0 each ms.
    pub logical: u32,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use bincode::Options;

    #[test]
    fn hlc_round_trip_via_canonical_bincode() {
        let hlc = Hlc {
            wall_ms: 1_700_000_000_000,
            logical: 7,
        };
        let bytes = crate::canonical_bincode().serialize(&hlc).expect("encode");
        // 8 bytes wall_ms BE + 4 bytes logical BE = 12 bytes.
        assert_eq!(bytes.len(), 12);
        let decoded: Hlc = crate::canonical_bincode()
            .deserialize(&bytes)
            .expect("decode");
        assert_eq!(hlc, decoded);
    }

    #[test]
    fn hlc_ord_is_lex_wall_then_logical() {
        let a = Hlc {
            wall_ms: 100,
            logical: 5,
        };
        let b = Hlc {
            wall_ms: 100,
            logical: 6,
        };
        let c = Hlc {
            wall_ms: 101,
            logical: 0,
        };
        assert!(a < b);
        assert!(b < c);
    }
}
