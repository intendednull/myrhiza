//! Deterministic helper imports.
//!
//! Each impl is a pure function of its bytes input. Side effects
//! (`host.log`) write to a peer-local sink that is NOT part of
//! state-digest per determinism.md §5.1.

use std::sync::Mutex;

use ed25519_dalek::{Signature, VerifyingKey};
use myrhiza_types::Hlc;

/// `host.hash(bytes)` returns BLAKE3(bytes) as 32 raw bytes.
#[must_use]
pub fn host_hash_impl(bytes: &[u8]) -> Vec<u8> {
    blake3::hash(bytes).as_bytes().to_vec()
}

/// `host.verify-signature(pubkey, msg, sig)` using `verify_strict`
/// per determinism.md §5.1. Plain `verify` is forbidden.
///
/// Returns `false` for any malformed pubkey (non-32 bytes or invalid
/// curve point) or signature (non-64 bytes); returns `true` only when
/// `verify_strict` accepts the signature.
#[must_use]
pub fn host_verify_signature_impl(pubkey: &[u8], msg: &[u8], sig: &[u8]) -> bool {
    let Ok(pk_arr): Result<&[u8; 32], _> = pubkey.try_into() else {
        return false;
    };
    let Ok(sig_arr): Result<&[u8; 64], _> = sig.try_into() else {
        return false;
    };
    let Ok(key) = VerifyingKey::from_bytes(pk_arr) else {
        return false;
    };
    let signature = Signature::from_bytes(sig_arr);
    key.verify_strict(msg, &signature).is_ok()
}

/// `host.now-hlc-from-event(event-bytes)` decodes the HLC out of a
/// canonical event envelope. Pure decoder per determinism.md §5.1.
///
/// Strict canonical decode: returns `None` if `event_bytes` is not the
/// exact canonical-bincode encoding of [`myrhiza_types::Event`]. This
/// rejects trailing garbage and any non-canonical encoding that two
/// honest peers might disagree on — which would otherwise be a
/// convergence-divergence vector inside `state-apply`.
///
/// The wasmtime binding (see `gating.rs`) maps `None` to a
/// `wasmtime::Error`, which traps the guest call. There is no silent
/// "zeroed Hlc" path: the WIT return type is `hlc` (no option/result),
/// so non-canonical input MUST fail loudly.
#[must_use]
pub fn host_now_hlc_from_event_impl(event_bytes: &[u8]) -> Option<Hlc> {
    let event: myrhiza_types::Event = myrhiza_types::decode_canonical(event_bytes).ok()?;
    Some(event.hlc)
}

/// `host.log` levels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogLevel {
    /// Tracing detail; lowest verbosity.
    Trace,
    /// Debug-level diagnostic.
    Debug,
    /// Informational message.
    Info,
    /// Warning — recoverable anomaly.
    Warn,
    /// Error — call did not produce expected outcome.
    Error,
}

/// Per-peer log sink. `record` is the only API state-apply sees;
/// `drain` is host-side. Drained content is NOT part of state-digest.
#[derive(Default)]
pub struct LogSink {
    entries: Mutex<Vec<(LogLevel, String)>>,
}

impl LogSink {
    /// Record a log line. State-apply gets a `()` return — cannot read
    /// back the log content (would be peer-local nondeterminism).
    pub fn record(&self, level: LogLevel, msg: String) {
        if let Ok(mut g) = self.entries.lock() {
            g.push((level, msg));
        }
    }

    /// Drain accumulated log entries. Host-side only.
    ///
    /// Recovers from a poisoned mutex via `into_inner()` rather than
    /// dropping log entries on the floor. A poisoned `entries` mutex
    /// means a `record` panicked mid-push — the existing entries are
    /// still well-formed (`Vec` is exception-safe through `push`), so
    /// returning them is preferable to silently swallowing diagnostic
    /// output. The peer-local sink is not part of state-digest, so
    /// surfacing the partial log has no convergence implications.
    pub fn drain(&self) -> Vec<(LogLevel, String)> {
        let mut guard = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *guard)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn host_hash_returns_blake3_canonical() {
        let out = host_hash_impl(b"");
        let expected =
            hex::decode("af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262")
                .unwrap();
        assert_eq!(out, expected);
    }

    #[test]
    fn host_hash_deterministic() {
        let a = host_hash_impl(b"hello");
        let b = host_hash_impl(b"hello");
        assert_eq!(a, b);
    }

    fn sample_event() -> myrhiza_types::Event {
        use std::collections::BTreeSet;
        myrhiza_types::Event {
            author: myrhiza_types::AuthorPubkey::from_bytes([1; 32]),
            seq: 1,
            prev: myrhiza_types::EventHash::ZERO,
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
    fn host_now_hlc_accepts_canonical_event_bytes() {
        use bincode::Options;
        use myrhiza_types::canonical_bincode;
        let event = sample_event();
        let bytes = canonical_bincode().serialize(&event).unwrap();
        let hlc = host_now_hlc_from_event_impl(&bytes).expect("canonical event bytes must decode");
        assert_eq!(hlc, event.hlc);
    }

    #[test]
    fn host_now_hlc_rejects_non_canonical_event_bytes() {
        use bincode::Options;
        use myrhiza_types::canonical_bincode;
        let event = sample_event();
        let mut bytes = canonical_bincode().serialize(&event).unwrap();
        bytes.push(0); // trailing garbage — non-canonical
        let result = host_now_hlc_from_event_impl(&bytes);
        assert!(
            result.is_none(),
            "non-canonical event bytes must be rejected (got Some(hlc))"
        );
    }

    #[test]
    fn host_now_hlc_rejects_malformed_event_bytes() {
        // Wholly malformed bytes — cannot decode at all.
        let bytes = vec![0xFFu8; 8];
        assert!(host_now_hlc_from_event_impl(&bytes).is_none());
    }

    #[test]
    fn log_sink_records_messages() {
        let sink = LogSink::default();
        sink.record(LogLevel::Info, "first".into());
        sink.record(LogLevel::Warn, "second".into());
        let lines = sink.drain();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], (LogLevel::Info, "first".into()));
        assert_eq!(lines[1], (LogLevel::Warn, "second".into()));
    }

    #[test]
    fn log_sink_not_part_of_state() {
        let sink = LogSink::default();
        sink.record(LogLevel::Info, "x".into());
        // The drain returns content; record returns nothing — by
        // construction state-apply cannot read what it logged. Asserts
        // the API shape required by determinism.md §5.1's "log content
        // is NOT part of the cross-peer convergence surface."
        let _: () = sink.record(LogLevel::Info, "y".into());
    }
}
