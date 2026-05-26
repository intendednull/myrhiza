//! Wasmtime backend for the Myrhiza runtime.
//!
//! Implements [`myrhiza_backend::Backend`] using Wasmtime's component
//! model. Capability gating is enforced at linker construction time
//! (only allowed imports are bound) plus a per-call interception
//! wrapper for high-value ops.

#![deny(missing_docs)]

mod engine;
mod float_ban;
mod gating;
mod helpers;
mod instance;
mod interaction_instance;
mod propose_instance;

use myrhiza_types::limits::{
    INTERACTION_FUEL_BUDGET_V1, STATE_APPLY_FUEL_BUDGET_V1, STATE_PROPOSE_FUEL_BUDGET_V1,
};

pub use engine::{HostState, WasmtimeBackend, deterministic_config};
pub use float_ban::{scan_component_for_floats, scan_core_module_for_floats};
pub use gating::{ambient_set, bound_imports, validate_manifest, wire_linker};
pub use helpers::{
    LogLevel, LogSink, host_hash_impl, host_now_hlc_from_event_impl, host_verify_signature_impl,
};

/// Component profile selector for the v1 wasmtime backend.
///
/// The v1 backend implements three of the four profiles in the master
/// spec (state-apply, state-propose, interaction). The behavior profile
/// lands later; when it does, this enum gains a fourth variant. The
/// kernel-side abstraction in [`myrhiza_backend::Profile`] already
/// names all four for the trait layer — this enum exists separately
/// because (a) the wasmtime backend's match arms encode the v1
/// implementation surface, not the spec surface, and (b) decoupling
/// the two lets the backend extend its match arms as a deliberate
/// implementation step rather than via `unreachable!()` placeholders.
///
/// Each variant carries (via methods) the per-profile knobs that used
/// to live as three parallel constants / wrappers:
/// fuel budget, whether float-ban applies, whether the
/// `host-ui-surfaces@1.0.0` types-only instance is in the prewalk
/// allowlist.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Profile {
    /// Strict pure-fn state-apply profile. Float-ban applies; fuel
    /// budget [`STATE_APPLY_FUEL_BUDGET_V1`] (10M). Prewalk does NOT
    /// admit `host-ui-surfaces@1.0.0`.
    StateApply,
    /// Loose intent-to-event proposal profile. No float-ban (kernel
    /// re-checks via state-apply dry-run); fuel budget
    /// [`STATE_PROPOSE_FUEL_BUDGET_V1`] (50M). Prewalk does NOT admit
    /// `host-ui-surfaces@1.0.0`.
    StatePropose,
    /// Per-peer UI surface profile. No float-ban (non-deterministic
    /// profile); fuel budget [`INTERACTION_FUEL_BUDGET_V1`] (50M).
    /// Prewalk DOES admit the types-only `host-ui-surfaces@1.0.0`.
    Interaction,
}

impl Profile {
    /// Per-call fuel budget for this profile per determinism.md §5.3.
    #[must_use]
    pub fn fuel_budget(self) -> u64 {
        match self {
            Self::StateApply => STATE_APPLY_FUEL_BUDGET_V1,
            Self::StatePropose => STATE_PROPOSE_FUEL_BUDGET_V1,
            Self::Interaction => INTERACTION_FUEL_BUDGET_V1,
        }
    }

    /// Whether the byte-level float-ban lint runs on this profile's
    /// components. Only [`Self::StateApply`] enables it — propose and
    /// interaction are non-deterministic profiles per spec §3.3, so
    /// cross-peer determinism does not apply.
    #[must_use]
    pub fn float_ban_applies(self) -> bool {
        matches!(self, Self::StateApply)
    }

    /// Whether the prewalk admits the `host-ui-surfaces@1.0.0`
    /// types-only instance as a permitted import. Only
    /// [`Self::Interaction`] permits it; state-apply and state-propose
    /// reject any top-level instance other than
    /// `host-deterministic@1.0.0` and `types@1.0.0`.
    #[must_use]
    pub fn allow_ui_surfaces(self) -> bool {
        matches!(self, Self::Interaction)
    }
}
