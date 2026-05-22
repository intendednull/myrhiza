//! State-propose handle wrapping a backend `ProposeInstance`.
//!
//! The propose profile is non-deterministic (fuel = 50M, no float-ban).
//! The kernel re-checks the returned payload via `state-apply` in dry-run
//! (pre-check) before signing, so cross-peer determinism does not require
//! the propose call itself to be pure.
//!
//! Per spec §3.1 / §3.6: `propose(prior_state, intent) -> result<list<u8>, string>`.
//! `Ok` bytes are the candidate event payload; `Err` is surfaced as
//! [`ProposeError::Rejected`].

use myrhiza_backend::{BackendError, ProposeInstance};
use thiserror::Error;

/// Errors returned by the state-propose handle.
#[derive(Debug, Error)]
pub enum ProposeError {
    /// The underlying backend reported a failure (trap, fuel, etc.).
    #[error("backend error: {0}")]
    Backend(#[from] BackendError),
    /// The propose component returned `Err(msg)`.
    #[error("propose rejected: {0}")]
    Rejected(String),
}

/// Owner of a `ProposeInstance` plus the `propose` entry point.
pub struct StateProposeHandle {
    instance: Box<dyn ProposeInstance>,
}

impl StateProposeHandle {
    /// Wrap an instantiated state-propose component.
    #[must_use]
    pub fn new(instance: Box<dyn ProposeInstance>) -> Self {
        Self { instance }
    }

    /// Run `propose(prior_state, intent)`.
    ///
    /// Returns payload bytes on `Ok`; on the component's `Err` the
    /// message is surfaced as [`ProposeError::Rejected`].
    ///
    /// # Errors
    ///
    /// Returns [`ProposeError::Backend`] if the underlying backend traps,
    /// exhausts fuel, or otherwise fails to dispatch the call.
    /// Returns [`ProposeError::Rejected`] if the component itself returns
    /// an `Err` variant (e.g. invalid intent, delta == 0).
    pub fn propose(&mut self, prior_state: &[u8], intent: &[u8]) -> Result<Vec<u8>, ProposeError> {
        match self.instance.call_propose(prior_state, intent)? {
            Ok(bytes) => Ok(bytes),
            Err(msg) => Err(ProposeError::Rejected(msg)),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use myrhiza_backend::BackendError;

    struct OkPropose(Vec<u8>);
    impl ProposeInstance for OkPropose {
        fn call_propose(
            &mut self,
            _prior: &[u8],
            _intent: &[u8],
        ) -> Result<Result<Vec<u8>, String>, BackendError> {
            Ok(Ok(self.0.clone()))
        }
    }

    struct ErrPropose(String);
    impl ProposeInstance for ErrPropose {
        fn call_propose(
            &mut self,
            _prior: &[u8],
            _intent: &[u8],
        ) -> Result<Result<Vec<u8>, String>, BackendError> {
            Ok(Err(self.0.clone()))
        }
    }

    #[test]
    fn propose_ok_returns_bytes() {
        let mut handle = StateProposeHandle::new(Box::new(OkPropose(vec![1, 2, 3])));
        let result = handle.propose(&[], &[]).unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn propose_err_surfaces_as_rejected() {
        let mut handle = StateProposeHandle::new(Box::new(ErrPropose("bad intent".into())));
        let err = handle.propose(&[], &[]).unwrap_err();
        assert!(matches!(err, ProposeError::Rejected(msg) if msg == "bad intent"));
    }
}
