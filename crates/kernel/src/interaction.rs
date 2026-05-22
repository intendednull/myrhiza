//! Interaction handle wrapping a backend `InteractionInstance`.
//!
//! The interaction profile is per-peer and non-deterministic. It owns the
//! UI surface: `view` projects current state as bytes (v1: UTF-8 text);
//! `dispatch` translates user actions into intent bytes for the propose
//! path.
//!
//! Per spec §3.1 / §3.6 / §3.2.

use myrhiza_backend::{BackendError, InteractionInstance};
use thiserror::Error;

/// Errors returned by the interaction handle.
#[derive(Debug, Error)]
pub enum InteractionError {
    /// The underlying backend reported a failure (trap, fuel, etc.).
    #[error("backend error: {0}")]
    Backend(#[from] BackendError),
    /// `dispatch` returned `Err(msg)` — unrecognised or invalid action.
    #[error("dispatch rejected: {0}")]
    DispatchRejected(String),
}

/// Owner of an `InteractionInstance` plus the `view` and `dispatch` entry
/// points.
pub struct InteractionHandle {
    instance: Box<dyn InteractionInstance>,
}

impl InteractionHandle {
    /// Wrap an instantiated interaction component.
    #[must_use]
    pub fn new(instance: Box<dyn InteractionInstance>) -> Self {
        Self { instance }
    }

    /// Project a view of `state` (and optional per-peer `peer_state`) as
    /// opaque bytes.  v1 contract: bytes are UTF-8 text the harness writes
    /// to stdout.
    ///
    /// # Errors
    ///
    /// Returns [`InteractionError::Backend`] if the underlying backend
    /// traps or otherwise fails to dispatch the call.
    pub fn view(&mut self, state: &[u8], peer_state: &[u8]) -> Result<Vec<u8>, InteractionError> {
        Ok(self.instance.call_view(state, peer_state)?)
    }

    /// Translate a user action string into intent bytes for the propose path.
    ///
    /// # Errors
    ///
    /// Returns [`InteractionError::Backend`] if the underlying backend
    /// traps. Returns [`InteractionError::DispatchRejected`] if the
    /// component returns `Err` (e.g. unknown action).
    pub fn dispatch(&mut self, action: &str) -> Result<Vec<u8>, InteractionError> {
        match self.instance.call_dispatch(action)? {
            Ok(bytes) => Ok(bytes),
            Err(msg) => Err(InteractionError::DispatchRejected(msg)),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use myrhiza_backend::BackendError;

    struct StubInteraction;
    impl InteractionInstance for StubInteraction {
        fn call_view(&mut self, state: &[u8], _peer_state: &[u8]) -> Result<Vec<u8>, BackendError> {
            Ok(state.to_vec())
        }

        fn call_dispatch(&mut self, action: &str) -> Result<Result<Vec<u8>, String>, BackendError> {
            if action == "ok" {
                Ok(Ok(vec![0x01]))
            } else {
                Ok(Err(format!("unknown: {action}")))
            }
        }

        fn call_on_broadcast_completion(
            &mut self,
            _token: &[u8],
            _ok: bool,
            _err: &str,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn call_on_blob_fetch_completion(
            &mut self,
            _token: &[u8],
            _ok: bool,
            _payload: &[u8],
            _err: &str,
        ) -> Result<(), BackendError> {
            Ok(())
        }
    }

    #[test]
    fn view_returns_state_bytes() {
        let mut handle = InteractionHandle::new(Box::new(StubInteraction));
        let v = handle.view(b"hello", &[]).unwrap();
        assert_eq!(v, b"hello");
    }

    #[test]
    fn dispatch_ok_returns_intent() {
        let mut handle = InteractionHandle::new(Box::new(StubInteraction));
        let intent = handle.dispatch("ok").unwrap();
        assert_eq!(intent, vec![0x01]);
    }

    #[test]
    fn dispatch_unknown_surfaces_as_rejected() {
        let mut handle = InteractionHandle::new(Box::new(StubInteraction));
        let err = handle.dispatch("bad").unwrap_err();
        assert!(matches!(err, InteractionError::DispatchRejected(_)));
    }
}
