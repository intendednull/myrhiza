//! V1 normative resource caps per determinism.md §5.3.
//!
//! Bumping any constant requires:
//! 1. A kernel-major version bump (convergence-breaking).
//! 2. Updating `crates/types/tests/limits_shadow.rs` to match.
//! 3. A spec amendment naming the new value.
//!
//! See verification.md §22.4 for the discipline.

/// Per-event apply fuel budget per determinism.md §5.3.
pub const STATE_APPLY_FUEL_BUDGET_V1: u64 = 10_000_000;

/// Per-event propose fuel budget per determinism.md §5.3 (5x apply).
pub const STATE_PROPOSE_FUEL_BUDGET_V1: u64 = 50_000_000;

/// Per-component memory cap per determinism.md §5.3.
pub const COMPONENT_MEMORY_CAP_V1: usize = 64 * 1024 * 1024;

/// Wasm operand stack ceiling per determinism.md §5.3, in bytes.
///
/// Pinned here so a future wasmtime LTS bump cannot silently change
/// the wasm stack size and shift trap boundaries on deeply recursive
/// components — see [`crate::limits`] preamble for the bump
/// discipline. 512 KiB matches wasmtime 36's default; pinning the
/// number means the value participates in convergence guarantees
/// rather than tracking upstream's whim.
pub const MAX_WASM_STACK_V1: usize = 512 * 1024;

/// Maximum event payload size per determinism.md §5.3.
pub const EVENT_PAYLOAD_CAP_V1: usize = 1024 * 1024;

/// Maximum DAG `deps` array size per determinism.md §5.3.
pub const DAG_DEPS_CAP_V1: usize = 64;

/// `host.hash(bytes)` cost: n * this constant per determinism.md §5.3.
pub const HOST_HASH_FUEL_PER_BYTE: u64 = 5;

/// `host.verify-signature` cost per determinism.md §5.3.
pub const HOST_VERIFY_SIGNATURE_FUEL: u64 = 5_000;

/// `host.verify-payload-mac` cost per determinism.md §5.3.
pub const HOST_VERIFY_PAYLOAD_MAC_FUEL: u64 = 1_000;

/// `host.install-key` cost per determinism.md §5.3.
pub const HOST_INSTALL_KEY_FUEL: u64 = 100;

/// `host.now-hlc-from-event` cost per determinism.md §5.3.
pub const HOST_NOW_HLC_FROM_EVENT_FUEL: u64 = 50;

/// `host.log` base cost (per-byte msg cost adds on top) per
/// determinism.md §5.3.
pub const HOST_LOG_FUEL_BASE: u64 = 100;
