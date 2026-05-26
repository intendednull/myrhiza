//! Drift detection per convergence.md §4.7 — anchor model + emit rate cap.
//!
//! Drift-messages are TUTTI-style: at each anchor (a canonical
//! "after-this-event" point in DAG topo-order), the local peer emits a
//! signed digest of state. Remote peers compare against their own
//! digest at the same anchor; mismatch implies divergence.
//!
//! This module owns:
//! - [`DriftRateLimit`] — min-interval + daily-cap gate on emit frequency
//! - [`should_emit`] — predicate on topo-index
//! - [`anchor_bound_map`] / [`anchor_covered`] — anchor-bounded replay support
//! - [`DriftDetected`] — observation log record

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use myrhiza_types::{AuthorPubkey, AuthorSeq, DriftAnchor, PeerPubkey};

/// Why a drift-emit was rate-limited.
#[derive(Clone, Debug)]
pub enum RateLimitKind {
    /// `min_interval` not yet elapsed since last emit.
    MinInterval,
    /// `daily_cap` reached for the rolling 24h window.
    DailyCap,
}

/// Token-bucket-ish rate gate for drift-message emission.
///
/// Two-axis gate per spec §4.7:
/// - Minimum interval between emits (smoothing).
/// - Hard daily cap (`DoS` budget).
///
/// Not thread-safe; the kernel runtime owns a single instance per
/// topic and calls `try_emit` from the run loop.
pub struct DriftRateLimit {
    /// Minimum wall-clock interval between successful emits.
    pub min_interval: Duration,
    /// Maximum emits per rolling 24h window.
    pub daily_cap: u32,
    last_emit: Option<Instant>,
    /// Timestamps of emits within the trailing 24h sliding window.
    ///
    /// Front is the oldest still-relevant emit; back is the most recent.
    /// Eviction is lazy: on every `try_emit` call, we pop from the front
    /// while the head instant sits at or before `now - 24h`.
    recent_emits: std::collections::VecDeque<Instant>,
}

impl DriftRateLimit {
    /// Construct a rate-limit with the given minimum interval and daily cap.
    #[must_use]
    pub fn new(min_interval: Duration, daily_cap: u32) -> Self {
        Self {
            min_interval,
            daily_cap,
            last_emit: None,
            // No pre-allocation: callers may pass very large caps
            // (`u32::MAX` in tests to disable the cap entirely), so we
            // grow lazily as emits accumulate.
            recent_emits: std::collections::VecDeque::new(),
        }
    }

    /// Try to consume a rate-limit slot. Returns `Ok(())` on grant,
    /// `Err(kind)` if rate-limited.
    ///
    /// Implements a true trailing-24h sliding window: an emit is rejected
    /// iff the count of prior emits within the last 24h is at or above
    /// `daily_cap`. Prior emits older than 24h are evicted on every call.
    ///
    /// # Errors
    ///
    /// - [`RateLimitKind::DailyCap`] if the rolling 24h emit count is at cap.
    /// - [`RateLimitKind::MinInterval`] if `min_interval` has not elapsed
    ///   since the most recent successful emit.
    pub fn try_emit(&mut self, now: Instant) -> Result<(), RateLimitKind> {
        let day = Duration::from_hours(24);
        // Evict timestamps that no longer sit in the trailing 24h window.
        while let Some(&front) = self.recent_emits.front() {
            if now.duration_since(front) >= day {
                self.recent_emits.pop_front();
            } else {
                break;
            }
        }
        if self.recent_emits.len() >= self.daily_cap as usize {
            return Err(RateLimitKind::DailyCap);
        }
        if let Some(last) = self.last_emit
            && now.duration_since(last) < self.min_interval
        {
            return Err(RateLimitKind::MinInterval);
        }
        self.recent_emits.push_back(now);
        self.last_emit = Some(now);
        Ok(())
    }
}

/// Decide whether to emit a drift-anchor at `topo_index`.
///
/// Returns `true` iff `drift_interval > 0`, `topo_index > 0`, and
/// `topo_index` is a multiple of `drift_interval`. `drift_interval == 0`
/// disables drift emission entirely.
///
/// `topo_index == 0` is excluded because that is the Genesis / empty-state
/// startup point: state is just-initialized, the digest is barely
/// meaningful, and remote peers all carry the same trivial digest there.
/// Emitting at Genesis would burn the rate-limit budget on a no-information
/// message.
#[must_use]
pub fn should_emit(topo_index: u64, drift_interval: u64) -> bool {
    drift_interval > 0 && topo_index > 0 && topo_index.checked_rem(drift_interval) == Some(0)
}

/// Build the `BTreeMap<AuthorPubkey, u64>` lookup map a `DriftMessage` needs for
/// anchor-bounded replay (used by `Runtime::drain_drift_stash`).
#[must_use]
pub fn anchor_bound_map(author_seq_vec: &[AuthorSeq]) -> BTreeMap<AuthorPubkey, u64> {
    author_seq_vec
        .iter()
        .map(|a| (a.author, a.max_seq))
        .collect()
}

/// Predicate: is `anchor` fully covered by `current_seq_map`?
///
/// True iff every `(author, max_seq)` in the anchor is at or below the
/// peer's current chain head for that author. An author missing from
/// `current_seq_map` is treated as max-seq 0.
#[must_use]
pub fn anchor_covered(anchor: &DriftAnchor, current_seq_map: &BTreeMap<AuthorPubkey, u64>) -> bool {
    anchor
        .author_seq_vec
        .iter()
        .all(|a| current_seq_map.get(&a.author).copied().unwrap_or(0) >= a.max_seq)
}

// Re-export concrete types so downstream Runtime can `use crate::drift::{...}`.
pub use myrhiza_types::DriftSignedPayload;

/// Observation log record produced when a remote drift-message digest
/// disagrees with the local digest at the same anchor.
#[derive(Clone, Debug)]
pub struct DriftDetected {
    /// Peer that emitted the disagreeing drift-message.
    pub peer: PeerPubkey,
    /// Anchor at which the disagreement was observed.
    pub anchor: DriftAnchor,
    /// Local state-digest at the anchor.
    pub local_digest: [u8; 32],
    /// Remote state-digest claimed by `peer`.
    pub remote_digest: [u8; 32],
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use myrhiza_types::EventHash;

    #[test]
    fn should_emit_at_multiples_of_interval() {
        // topo_index=0 is the Genesis / empty-state startup point: state is
        // just-initialized, the digest is barely meaningful, and all remote
        // peers carry the same trivial digest there. Drift-emit at Genesis
        // would burn rate-limit budget on a no-information message, so we
        // exclude topo_index=0 even though 0 % N == 0.
        assert!(!should_emit(0, 4));
        assert!(should_emit(4, 4));
        assert!(should_emit(8, 4));
        assert!(!should_emit(1, 4));
        assert!(!should_emit(3, 4));
    }

    #[test]
    fn should_emit_zero_interval_never_fires() {
        assert!(!should_emit(0, 0));
        assert!(!should_emit(100, 0));
    }

    #[test]
    fn rate_limit_blocks_on_min_interval() {
        let mut rl = DriftRateLimit::new(Duration::from_mins(1), 1024);
        let t0 = Instant::now();
        rl.try_emit(t0).expect("first ok");
        let r = rl.try_emit(t0 + Duration::from_secs(30));
        assert!(matches!(r, Err(RateLimitKind::MinInterval)));
    }

    #[test]
    fn rate_limit_blocks_on_daily_cap() {
        let mut rl = DriftRateLimit::new(Duration::from_secs(0), 2);
        let t0 = Instant::now();
        rl.try_emit(t0).expect("1");
        rl.try_emit(t0).expect("2");
        let r = rl.try_emit(t0);
        assert!(matches!(r, Err(RateLimitKind::DailyCap)));
    }

    /// Trailing-24h sliding window: a pre-rollover burst that fills the
    /// daily cap must still block the first post-rollover emit, because
    /// all four prior emits still sit inside the trailing 24h window
    /// from the perspective of `after_rollover`.
    ///
    /// Regression for review-finding Q-5 (old implementation reset the
    /// counter at the 24h boundary, admitting a `2 * daily_cap` burst).
    #[test]
    fn daily_cap_does_not_admit_double_burst_across_rollover() {
        let mut rl = DriftRateLimit::new(
            Duration::from_mins(1), // min_interval
            4,                      // daily_cap
        );
        let t0 = Instant::now();

        // Burn the daily cap right before rollover.
        let near_rollover = (t0 + Duration::from_hours(24))
            .checked_sub(Duration::from_secs(1))
            .expect("instant minus 1s is well-defined");
        let mut t = near_rollover;
        for _ in 0..4 {
            rl.try_emit(t).expect("within daily cap");
            t += Duration::from_mins(1);
        }
        // Immediately after rollover, true sliding window should still reject —
        // because the prior 4 emits all sit inside the trailing 24h window.
        let after_rollover = t0 + Duration::from_hours(24) + Duration::from_secs(1);
        let r = rl.try_emit(after_rollover);
        assert!(
            matches!(r, Err(RateLimitKind::DailyCap)),
            "sliding window must still reject across rollover, got {r:?}",
        );
    }

    #[test]
    fn anchor_covered_returns_true_when_all_authors_satisfied() {
        let anchor = DriftAnchor {
            event_hash: EventHash::ZERO,
            author_seq_vec: vec![
                AuthorSeq {
                    author: AuthorPubkey::from_bytes([1; 32]),
                    max_seq: 3,
                },
                AuthorSeq {
                    author: AuthorPubkey::from_bytes([2; 32]),
                    max_seq: 5,
                },
            ],
        };
        let mut map = BTreeMap::new();
        map.insert(AuthorPubkey::from_bytes([1; 32]), 4);
        map.insert(AuthorPubkey::from_bytes([2; 32]), 5);
        assert!(anchor_covered(&anchor, &map));
    }

    #[test]
    fn anchor_covered_returns_false_when_author_missing() {
        let anchor = DriftAnchor {
            event_hash: EventHash::ZERO,
            author_seq_vec: vec![AuthorSeq {
                author: AuthorPubkey::from_bytes([1; 32]),
                max_seq: 3,
            }],
        };
        let map = BTreeMap::new();
        assert!(!anchor_covered(&anchor, &map));
    }

    /// Critical integration test: the drift sign + verify round-trip
    /// MUST work over canonical `DriftSignedPayload` bytes directly
    /// (no intermediate BLAKE3 hash). This pins the assumption that
    /// `myrhiza_manifest::verify_signature` passes its `message` argument
    /// straight to `verify_strict` without re-hashing. If a future
    /// refactor introduces a hash step inside `verify_signature`, this
    /// test catches it and prevents silently-broken drift detection.
    #[test]
    fn drift_signed_payload_round_trip_signs_and_verifies_over_raw_bytes() {
        use bincode::Options;
        use myrhiza_types::{DriftSignedPayload, canonical_bincode};

        let kp = crate::identity::PeerKeypair::deterministic(0xCAFE);
        let payload = DriftSignedPayload {
            anchor: DriftAnchor {
                event_hash: EventHash::blake3(b"event"),
                author_seq_vec: vec![AuthorSeq {
                    author: AuthorPubkey::from_bytes([0x11; 32]),
                    max_seq: 5,
                }],
            },
            digest: [0xDD; 32],
            digest_format: "bincode-1.3".into(),
        };
        let bytes = canonical_bincode().serialize(&payload).expect("encode");
        let sig = kp.sign(&bytes);
        myrhiza_manifest::verify_signature(kp.public.as_bytes(), &bytes, &sig)
            .expect("drift payload round-trip MUST verify");
    }
}
