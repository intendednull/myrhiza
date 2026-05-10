//! State-digest emission stub.
//!
//! Per convergence.md §4.3, the kernel hashes each app's
//! `state-digest()` output and gossips the hash on the drift-
//! detection topic. Plan A produces the (`event_index`, hash) pairs
//! into a peer-local log; plan B wires them onto the gossip topic
//! per `determinism.drift-detection.interval-events`.

use myrhiza_types::EventHash;

/// One observation by the digest emitter: the post-state digest hash
/// for the event at the canonical topo-sort index.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DigestEvent {
    /// Canonical topo-sort index of the event whose post-state this
    /// digests.
    pub event_index: u64,
    /// BLAKE3 of the app's `state-digest` output bytes.
    pub digest_hash: EventHash,
}

/// Emits digests every `interval_events` per
/// `determinism.drift-detection.interval-events`. Plan A's stub
/// records every observation it is given; plan B integrates with
/// the kernel's apply loop to call `observe` only at the configured
/// modulo cadence.
pub struct DigestEmitter {
    log: Vec<DigestEvent>,
    interval_events: u32,
}

impl DigestEmitter {
    /// Construct an emitter parameterized by the manifest's
    /// `determinism.drift-detection.interval-events`.
    #[must_use]
    pub fn new(interval_events: u32) -> Self {
        Self {
            log: Vec::new(),
            interval_events,
        }
    }

    /// The configured cadence — number of events between drift-detect
    /// observations once the modulo gate lands in plan B.
    #[must_use]
    pub fn interval_events(&self) -> u32 {
        self.interval_events
    }

    /// Record a digest observation for an event at the given index.
    pub fn observe(&mut self, event_index: u64, state_digest_bytes: &[u8]) {
        self.log.push(DigestEvent {
            event_index,
            digest_hash: EventHash::blake3(state_digest_bytes),
        });
    }

    /// Drain the recorded events; clears the log.
    pub fn drain(&mut self) -> Vec<DigestEvent> {
        std::mem::take(&mut self.log)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use myrhiza_types::EventHash;

    #[test]
    fn emitter_records_per_event_digest() {
        let mut emitter = DigestEmitter::new(1024);
        emitter.observe(0, b"state_v0");
        emitter.observe(1, b"state_v1");
        emitter.observe(1024, b"state_v1024");
        let log = emitter.drain();
        assert_eq!(log.len(), 3);
        assert_eq!(log[0].event_index, 0);
        assert_eq!(log[0].digest_hash, EventHash::blake3(b"state_v0"));
        assert_eq!(log[2].event_index, 1024);
    }

    #[test]
    fn emitter_uses_blake3_canonical() {
        let mut emitter = DigestEmitter::new(1);
        emitter.observe(0, b"abc");
        let log = emitter.drain();
        assert_eq!(log[0].digest_hash, EventHash::blake3(b"abc"));
    }

    #[test]
    fn drain_clears_log() {
        let mut emitter = DigestEmitter::new(1);
        emitter.observe(0, b"x");
        emitter.drain();
        let log = emitter.drain();
        assert!(log.is_empty());
    }
}
