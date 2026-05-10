//! Backend trait abstraction.
//!
//! Plan A's `myrhiza-wasmtime-backend` is the v1 native impl.
//! Plan C's `myrhiza-jco-backend` will satisfy the same trait.
//! Designed in from the start so jco doesn't require kernel
//! retrofitting per implementation.md §20.

#![deny(missing_docs)]

use myrhiza_manifest::Manifest;
use myrhiza_types::IdentityScope;
use thiserror::Error;

/// Errors a backend may return when loading or running a component.
#[derive(Debug, Error)]
pub enum BackendError {
    /// Component bytes failed to decode or instantiate.
    #[error("component instantiation failed: {0}")]
    Instantiation(String),
    /// Component imported a host import its manifest does not declare,
    /// or the component imports an instance the state-apply ambient set
    /// does not provide. Carries the offending import name (vocabulary-
    /// style `host.X` for unknown functions on the deterministic-helper
    /// instance, or the WIT instance name for unknown instances).
    #[error("capability check failed: component imports {0:?} not in manifest grants")]
    UnauthorizedImport(String),
    /// Component imported a function not in the v1 vocabulary.
    #[error("component imports unknown capability: {0}")]
    UnknownImport(String),
    /// State-apply call exhausted its fuel budget per determinism.md §5.3.
    /// Distinguished from a generic trap so the kernel can surface
    /// "compute budget exceeded" rather than a generic instantiation
    /// failure.
    #[error("fuel exhausted during state-apply call")]
    FuelExhausted,
    /// State-apply call exceeded its 64 MB memory cap per determinism.md §5.3.
    /// Distinguished from a generic trap so the kernel can surface
    /// "memory cap exceeded" rather than a generic instantiation failure.
    #[error("memory cap exceeded during state-apply call")]
    MemoryExhausted,
    /// Capability is registered in the v1 vocabulary but its
    /// state-apply binding is deferred to plan B (e.g. `host.install-key`,
    /// `host.verify-payload-mac` — both require the `key-handle` resource
    /// infrastructure that plan B introduces). Manifests declaring these
    /// for state-apply are rejected at install per determinism.md §5.1.
    #[error("capability {0:?} declared but deferred to plan B")]
    DeferredToPlanB(String),
    /// State-apply WASM contains a banned float instruction. Carries
    /// the scanner's diagnostic message (op name plus location). The
    /// concrete shape is owned `String` rather than a `&'static str`
    /// because the float-ban scanner produces dynamic messages
    /// (e.g. embedded in nested core modules); leaking each into a
    /// `'static` reference would accumulate per-failure allocations
    /// for the lifetime of the backend.
    #[error("float-ban lint: component contains banned instruction {0}")]
    BannedInstruction(String),
    /// Fuel exhaustion or other trap during apply.
    #[error("trap during apply: {0}")]
    Trap(String),
    /// Pre-check or apply returned a Reject verdict.
    #[error("verdict reject: {0}")]
    Verdict(String),
    /// Calling profile attempted operation not authorized for it.
    #[error("profile {profile:?} forbidden from operation: {op}")]
    ProfileForbidden {
        /// Profile name (one of state-apply, state-propose, interaction, behavior).
        profile: &'static str,
        /// Operation attempted.
        op: String,
    },
}

/// A loaded, capability-gated component instance ready to be called.
pub trait ComponentInstance: Send + 'static {
    /// Invoke `apply(prior_state, event)` returning verdict + new state.
    /// Pre-check (dry-run) is the same call; the kernel decides whether
    /// to commit `new_state` based on the returned verdict.
    ///
    /// # Errors
    ///
    /// Returns `BackendError` if the component traps, exhausts fuel,
    /// or the call cannot be dispatched.
    fn call_apply(
        &mut self,
        prior_state: &[u8],
        event: &[u8],
    ) -> Result<(Verdict, Vec<u8>), BackendError>;

    /// Invoke `state-digest(state)`.
    ///
    /// # Errors
    ///
    /// Returns `BackendError` if the component traps or the call cannot
    /// be dispatched.
    fn call_state_digest(&mut self, state: &[u8]) -> Result<Vec<u8>, BackendError>;
}

/// The verdict returned by state-apply.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// Apply mode: commit the new state. Pre-check mode: kernel signs.
    Accept,
    /// Apply mode: reject and surface message. Pre-check: no signing.
    Reject(String),
}

/// Profile being instantiated. Determines which sub-interface is bound.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Profile {
    /// Strict pure-fn state-apply profile.
    StateApply,
    /// Loose intent-to-event proposal profile.
    StatePropose,
    /// Per-peer UI surface profile.
    Interaction,
    /// Bots, bridges, automations.
    Behavior,
}

/// Identity context bound at instance creation. Plan A only uses this
/// for state-apply (which has no `host.author-event`); plans B/C use
/// it for non-deterministic profiles.
#[derive(Clone, Debug)]
pub struct InstanceIdentity {
    /// The kernel-side identity scope; backend uses opaque WIT handles
    /// derived from this when binding the WIT resource at instantiation.
    pub scope: IdentityScope,
}

/// A backend creates `ComponentInstance`s from component bytes + manifest.
pub trait Backend: Send + Sync + 'static {
    /// Instantiate a state-apply component, applying capability gating
    /// per the manifest. Returns an instance ready for `call_apply`.
    ///
    /// # Errors
    ///
    /// Returns `BackendError` if the component bytes fail to decode,
    /// the manifest declares unauthorized imports, the float-ban lint
    /// rejects the WASM, or instantiation fails for any other reason.
    fn instantiate_state_apply(
        &self,
        component_bytes: &[u8],
        manifest: &Manifest,
    ) -> Result<Box<dyn ComponentInstance>, BackendError>;
}
