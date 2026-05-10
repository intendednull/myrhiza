//! State-apply ABI: apply mode and pre-check dry-run mode.
//!
//! Per convergence.md §4.4 + determinism.md §5.3: pre-check and
//! apply are the same WASM function called by the kernel in two
//! different modes. The fuel budget is shared per (event, peer)
//! pair — the kernel sets fuel to 10M at the start of each event
//! and lets pre-check + apply share that pool.
//!
//! Plan A delivers handle methods `pre_check` and `apply` returning
//! verdicts. The kernel client (originator path) calls `pre_check`
//! first; on Accept, the kernel signs the event and broadcasts; on
//! Reject, the kernel surfaces an error and does NOT sign. On the
//! receiving path each peer calls `apply` directly and commits the
//! returned state if the verdict is Accept.
//!
//! ## Event-byte contract
//!
//! The `event` argument to [`StateApplyHandle::apply`] and
//! [`StateApplyHandle::pre_check`] is the FULL canonical Event
//! envelope per convergence.md §4 — the same byte string two peers
//! see on the wire, encoded under the v1-pinned bincode 1.3.x
//! options chain per determinism.md §5.4. This is what
//! `host.now-hlc-from-event(event-bytes)` decodes
//! ([determinism.md §5.1]); the guest passes the same `event` slice
//! it received as the helper argument so every peer extracts the
//! identical HLC.
//!
//! Plan A's kernel does not yet have an event-DAG that produces
//! these envelopes — the acceptance suite passes hand-rolled bytes
//! to exercise the wasm boundary. That is acceptable only because
//! plan A's fixtures do not call `host.now-hlc-from-event`. Plan B's
//! event-ingestion path MUST construct a real
//! [`myrhiza_types::Event`] and feed
//! `canonical_bincode().serialize(&event).expect(...)` to this
//! handle. See the round-trip test
//! [`tests::apply_event_envelope_round_trip_decodes_hlc`] for the
//! load-bearing shape.

use myrhiza_backend::{BackendError, ComponentInstance, Verdict};
use thiserror::Error;

/// Errors returned by the state-apply handle.
#[derive(Debug, Error)]
pub enum ApplyError {
    /// The underlying backend reported a failure (trap, fuel, etc).
    #[error("backend error: {0}")]
    Backend(#[from] BackendError),
}

/// Verdict reported by the component, lifted into the kernel's
/// surface-level enum.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ApplyOutcome {
    /// Apply mode: commit the new state. Pre-check mode: kernel signs.
    Accepted,
    /// Apply mode: skip the commit. Pre-check mode: do NOT sign.
    Rejected(String),
}

/// Result of [`StateApplyHandle::apply`].
#[derive(Clone, Debug)]
pub struct ApplyResult {
    /// Component verdict, lifted.
    pub outcome: ApplyOutcome,
    /// Apply's resulting state. Empty on Reject. Per convergence.md
    /// §4.4, the kernel commits this iff `outcome == Accepted`.
    pub new_state: Vec<u8>,
}

/// Result of [`StateApplyHandle::pre_check`].
#[derive(Clone, Debug)]
pub struct PreCheckResult {
    /// Component verdict, lifted.
    pub outcome: ApplyOutcome,
    /// Pre-check's hypothetical post-state. Discarded by the kernel
    /// (originator never commits pre-check state; only the post-
    /// signing apply call mutates state). Returned for tests + drift
    /// inspection.
    pub candidate_state: Vec<u8>,
}

/// Owner of a `ComponentInstance` plus the apply / pre-check entry
/// points.
pub struct StateApplyHandle {
    instance: Box<dyn ComponentInstance>,
}

impl StateApplyHandle {
    /// Wrap an instantiated state-apply component.
    #[must_use]
    pub fn new(instance: Box<dyn ComponentInstance>) -> Self {
        Self { instance }
    }

    /// Apply mode: ingest an event, mutate state in place. Per
    /// convergence.md §4.4, called on every receiving peer.
    ///
    /// `event` is the FULL canonical Event envelope (author, seq,
    /// prev, deps, hlc, payload, signature) encoded under the v1-pinned
    /// bincode 1.3.x options chain per determinism.md §5.4 — see the
    /// module-level doc comment for the complete contract and the
    /// plan-A vs plan-B caveat.
    ///
    /// # Errors
    ///
    /// Returns [`ApplyError::Backend`] if the underlying backend traps,
    /// exhausts fuel, or otherwise fails to dispatch the call.
    pub fn apply(&mut self, prior_state: &[u8], event: &[u8]) -> Result<ApplyResult, ApplyError> {
        let (verdict, new_state) = self.instance.call_apply(prior_state, event)?;
        Ok(ApplyResult {
            outcome: lift_verdict(verdict),
            new_state,
        })
    }

    /// Pre-check dry-run mode: same WASM function, kernel discards
    /// the new state. Per convergence.md §4.4. Pre-check fails closed:
    /// the kernel does NOT sign and broadcast on Reject.
    ///
    /// `event` is the same canonical Event envelope shape required by
    /// [`StateApplyHandle::apply`] — see the module-level doc comment
    /// for the contract. Pre-check is mechanically the same WASM
    /// `apply` call run in dry-run mode (per architecture.md §3.5),
    /// so the byte-shape requirement is identical: pre-check / apply
    /// agreement is structurally impossible to violate via the kernel
    /// boundary.
    ///
    /// # Errors
    ///
    /// Returns [`ApplyError::Backend`] if the underlying backend traps,
    /// exhausts fuel, or otherwise fails to dispatch the call.
    pub fn pre_check(
        &mut self,
        prior_state: &[u8],
        event: &[u8],
    ) -> Result<PreCheckResult, ApplyError> {
        let (verdict, candidate_state) = self.instance.call_apply(prior_state, event)?;
        Ok(PreCheckResult {
            outcome: lift_verdict(verdict),
            candidate_state,
        })
    }

    /// Forward to the underlying instance's `state-digest` export.
    ///
    /// # Errors
    ///
    /// Returns [`ApplyError::Backend`] if the underlying backend traps
    /// or otherwise fails to dispatch the call.
    pub fn state_digest(&mut self, state: &[u8]) -> Result<Vec<u8>, ApplyError> {
        Ok(self.instance.call_state_digest(state)?)
    }
}

fn lift_verdict(v: Verdict) -> ApplyOutcome {
    match v {
        Verdict::Accept => ApplyOutcome::Accepted,
        Verdict::Reject(s) => ApplyOutcome::Rejected(s),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// Mock instance that just echoes `prior_state || event` back as
    /// new state and returns Accept. No fuel accounting in this mock.
    struct Echo;

    impl myrhiza_backend::ComponentInstance for Echo {
        fn call_apply(
            &mut self,
            prior: &[u8],
            event: &[u8],
        ) -> Result<(myrhiza_backend::Verdict, Vec<u8>), myrhiza_backend::BackendError> {
            let mut out = Vec::with_capacity(prior.len() + event.len());
            out.extend_from_slice(prior);
            out.extend_from_slice(event);
            Ok((myrhiza_backend::Verdict::Accept, out))
        }

        fn call_state_digest(
            &mut self,
            state: &[u8],
        ) -> Result<Vec<u8>, myrhiza_backend::BackendError> {
            Ok(state.to_vec())
        }
    }

    #[test]
    fn pre_check_does_not_commit_state() {
        let mut handle = StateApplyHandle::new(Box::new(Echo));
        let prior = vec![1, 2, 3];
        let event = vec![4, 5];
        let r = handle.pre_check(&prior, &event).unwrap();
        assert!(matches!(r.outcome, ApplyOutcome::Accepted));
        // pre_check returns the candidate state but does NOT mutate
        // the handle's view of "current state" — it has none.
        assert_eq!(r.candidate_state, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn apply_returns_accept_with_new_state() {
        let mut handle = StateApplyHandle::new(Box::new(Echo));
        let prior = vec![10];
        let event = vec![20];
        let r = handle.apply(&prior, &event).unwrap();
        assert!(matches!(r.outcome, ApplyOutcome::Accepted));
        assert_eq!(r.new_state, vec![10, 20]);
    }

    /// Covers: convergence.md §4, determinism.md §5.1, determinism.md §5.4
    ///
    /// Pins the load-bearing event-byte contract at the kernel/wasm
    /// boundary: when the kernel passes a canonical [`myrhiza_types::Event`]
    /// envelope through `apply`, the same byte slice routed back to
    /// `decode_canonical::<Event>` (the exact path
    /// `host_now_hlc_from_event_impl` takes inside the wasmtime backend)
    /// recovers the original HLC.
    ///
    /// This test does NOT cross the wasm boundary — that is exercised
    /// by the wasmtime-backend's helper unit tests
    /// (`host_now_hlc_accepts_canonical_event_bytes`). Instead this
    /// test pins the *contract surface*: the bytes the kernel would
    /// hand to `apply` are the bytes the helper expects. If a future
    /// refactor accidentally has the kernel pass payload-only bytes
    /// (or any other shape), the round-trip here breaks — surfacing a
    /// contract drift before plan B's event-DAG path is wired up.
    ///
    /// The plan-A acceptance suite passes hand-rolled bytes to
    /// fixtures that do not call `host.now-hlc-from-event`; that is
    /// internally consistent only because the fixtures don't exercise
    /// the helper. This unit test is the canonical reference for what
    /// plan-B's event-ingestion must build.
    #[test]
    fn apply_event_envelope_round_trip_decodes_hlc() {
        use bincode::Options;
        use myrhiza_types::{
            AuthorPubkey, Event, EventHash, Hlc, canonical_bincode, decode_canonical,
        };
        use std::collections::BTreeSet;

        // Construct an Event with a distinctive HLC so the round-trip
        // assertion is unambiguous (no zero-default coincidence).
        let event = Event {
            author: AuthorPubkey::from_bytes([0x11; 32]),
            seq: 1,
            prev: EventHash::ZERO,
            deps: BTreeSet::new(),
            hlc: Hlc {
                wall_ms: 12_345,
                logical: 7,
            },
            payload: vec![0xAA, 0xBB],
            signature: [0xFF; 64],
        };

        // Kernel side: serialize via the v1-pinned canonical bincode
        // (determinism.md §5.4). These are the exact bytes plan B
        // will pass to `apply`.
        let envelope_bytes = canonical_bincode()
            .serialize(&event)
            .expect("canonical bincode of Event never fails");

        // Helper side: same call shape as
        // `wasmtime_backend::helpers::host_now_hlc_from_event_impl`
        // (`decode_canonical::<Event>(event_bytes).ok()?.hlc`). If the
        // wire shape ever drifts, this decode fails and the contract
        // breaks here rather than at runtime.
        let decoded: Event =
            decode_canonical(&envelope_bytes).expect("canonical envelope must decode");
        assert_eq!(
            decoded.hlc, event.hlc,
            "host.now-hlc-from-event contract: kernel-supplied envelope must round-trip the originator's HLC"
        );

        // Drive the same bytes through the apply-handle plumbing to
        // prove the kernel side does not need to reshape the bytes
        // before calling the wasm export.
        let mut handle = StateApplyHandle::new(Box::new(Echo));
        let r = handle
            .apply(b"", &envelope_bytes)
            .expect("apply accepts a canonical Event envelope");
        assert!(matches!(r.outcome, ApplyOutcome::Accepted));
        // Echo's new_state == prior || event == "" || envelope_bytes,
        // so we can re-decode out of the returned state to prove the
        // bytes are passed through verbatim (no copy-mangling, no
        // length prefix injection at the kernel layer).
        let echoed: Event = decode_canonical(&r.new_state)
            .expect("echoed bytes must still be a canonical envelope");
        assert_eq!(echoed.hlc, event.hlc);
    }

    /// Verifies pre-check fail-closed semantics: on Reject the kernel
    /// must NOT sign or broadcast. The handle's `pre_check` returns
    /// `outcome=Rejected`; calling code is responsible for NOT calling
    /// `apply`.
    struct AlwaysReject;
    impl myrhiza_backend::ComponentInstance for AlwaysReject {
        fn call_apply(
            &mut self,
            _: &[u8],
            _: &[u8],
        ) -> Result<(myrhiza_backend::Verdict, Vec<u8>), myrhiza_backend::BackendError> {
            Ok((myrhiza_backend::Verdict::Reject("nope".into()), vec![]))
        }
        fn call_state_digest(
            &mut self,
            _: &[u8],
        ) -> Result<Vec<u8>, myrhiza_backend::BackendError> {
            Ok(vec![])
        }
    }

    #[test]
    fn pre_check_fail_closed_on_reject() {
        let mut handle = StateApplyHandle::new(Box::new(AlwaysReject));
        let r = handle.pre_check(&[], &[]).unwrap();
        match r.outcome {
            ApplyOutcome::Rejected(reason) => assert_eq!(reason, "nope"),
            ApplyOutcome::Accepted => panic!("must not accept"),
        }
    }
}
