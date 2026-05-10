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
