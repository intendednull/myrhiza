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
/// Returns `None` if `event_bytes` is not a valid canonical-bincode
/// encoding of [`myrhiza_types::Event`].
#[must_use]
pub fn host_now_hlc_from_event_impl(event_bytes: &[u8]) -> Option<Hlc> {
    use bincode::Options;
    use myrhiza_types::canonical_bincode;
    let event: myrhiza_types::Event = canonical_bincode().deserialize(event_bytes).ok()?;
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
    pub fn drain(&self) -> Vec<(LogLevel, String)> {
        self.entries
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default()
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
