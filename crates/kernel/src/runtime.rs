//! Per-topic kernel runtime — drives event ingest, replay, drift emit.
//!
//! See `docs/specs/2026-05-10-plan-b-1-dag-memnet-design.md` §11.
//!
//! ## Scope of this file (Task 16)
//!
//! Types only — [`RuntimeCfg`], [`RuntimeError`], [`EquivocationFlag`],
//! [`PeerWarning`], [`AuthorCommand`], [`RuntimeHandle`]. The `Runtime`
//! struct, its `start` constructor, the `run` loop, and message
//! handlers land in Tasks 17-19. The import list here is intentionally
//! limited to symbols referenced by these type definitions; later
//! tasks extend it as the impl lands.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use myrhiza_network::NetError;
use myrhiza_types::{AuthorPubkey, EventHash, PeerPubkey};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, watch};

use crate::dag::DagError;
use crate::drift::{DriftDetected, RateLimitKind};
use crate::pending::PendingCfg;
use crate::state_apply::ApplyError;

/// Per-topic runtime configuration knobs.
///
/// Defaults match the spec recommendations (§11.4). Each field is
/// tunable per-deployment; the structure itself is wire-stable only in
/// that future fields must be additive with a [`Default`] supplier.
#[derive(Clone)]
pub struct RuntimeCfg {
    /// Number of events between candidate drift-message emits.
    ///
    /// A drift message is *considered* (subject to rate limits) when
    /// the topological index hits a multiple of this value.
    pub drift_interval: u64,

    /// Minimum wall-clock gap between successive drift emits.
    pub drift_min_interval: Duration,

    /// Rolling 24h cap on emitted drift messages.
    pub drift_daily_cap: u32,

    /// Cadence at which heads-summary is broadcast for late-joiner
    /// backfill.
    pub heads_summary_tick: Duration,

    /// Out-of-order event buffer configuration.
    pub pending_cfg: PendingCfg,

    /// Capacity hint for any bounded broadcast channels the runtime
    /// creates internally.
    pub broadcast_capacity: usize,

    /// Local kernel-fuel table version; surfaced as a [`PeerWarning`]
    /// when remote drift messages disagree (no halt).
    pub kernel_fuel_table_version: u32,

    /// Maximum number of incoming drift messages stashed pending an
    /// anchor-covering replay. Defaults to 256.
    pub drift_stash_cap: usize,
}

impl Default for RuntimeCfg {
    fn default() -> Self {
        Self {
            drift_interval: 1024,
            drift_min_interval: Duration::from_mins(1),
            drift_daily_cap: 1024,
            heads_summary_tick: Duration::from_secs(5),
            pending_cfg: PendingCfg::default(),
            broadcast_capacity: 256,
            kernel_fuel_table_version: 1,
            drift_stash_cap: 256,
        }
    }
}

/// Errors surfaced from the per-topic runtime.
///
/// Non-fatal anomalies (rate limits, equivocation evidence, peer
/// disagreement on the fuel table) flow through observation logs
/// instead — see [`RuntimeHandle`]. Variants here represent failures
/// that abort an operation (author / replay / publish).
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Wrapping [`NetError`] from the underlying transport.
    #[error("network: {0}")]
    Network(#[from] NetError),

    /// Wrapping [`ApplyError`] from state-apply / pre-check.
    #[error("apply: {0}")]
    Apply(#[from] ApplyError),

    /// Wrapping [`DagError`] from event-DAG insertion.
    #[error("dag: {0}")]
    Dag(#[from] DagError),

    /// Local pre-check rejected an authored event before broadcast.
    #[error("pre-check rejected: {0}")]
    PreCheckRejected(String),

    /// Canonical bincode encoding failure (signature / digest path).
    #[error("canonical encoding error: {0}")]
    Canonical(String),

    /// Author command issued on a runtime started without an author
    /// keypair.
    #[error("runtime is read-only (no author key configured)")]
    ReadOnly,
}

impl From<bincode::Error> for RuntimeError {
    fn from(e: bincode::Error) -> Self {
        RuntimeError::Canonical(e.to_string())
    }
}

/// Observation-log record: same-author-and-seq event seen twice with
/// differing hashes.
///
/// Surfaced via [`RuntimeHandle::equivocation_log`]. Detection +
/// recording is non-fatal at this layer; consumers decide on policy.
#[derive(Clone, Debug)]
pub struct EquivocationFlag {
    /// Author whose chain forked.
    pub author: AuthorPubkey,

    /// Sequence number at which the fork occurred.
    pub seq: u64,

    /// Hash of the event we observed first / accepted locally.
    pub local_hash: EventHash,

    /// Hash of the conflicting event observed later.
    pub remote_hash: EventHash,

    /// Peer that delivered the conflicting event, when known.
    pub peer: Option<PeerPubkey>,
}

/// Observation-log record: non-fatal peer / runtime warning.
///
/// Surfaced via [`RuntimeHandle::peer_warnings`]. Variants are
/// open-ended; new kinds are additive.
#[derive(Clone, Debug)]
pub enum PeerWarning {
    /// Remote drift message declared a kernel-fuel table version that
    /// differs from ours. Logged, not halted (§11.7).
    KernelFuelTableMismatch {
        /// Remote peer, when the message carried a peer identity.
        peer: Option<PeerPubkey>,
        /// Version claimed by the remote.
        remote_version: u32,
        /// Local kernel-fuel table version.
        local_version: u32,
    },

    /// Local drift-emit suppressed by the rate limiter.
    DriftRateLimited(RateLimitKind),

    /// Subscription consumer lagged and dropped messages.
    BroadcastLagged {
        /// Number of dropped messages reported by the transport.
        dropped: u64,
    },
}

/// Command sent into the runtime task via [`RuntimeHandle::author_tx`].
pub enum AuthorCommand {
    /// Author + publish a new event with `payload` and `deps`.
    Author {
        /// Canonical event payload bytes (state-apply-defined schema).
        payload: Vec<u8>,
        /// Causal dependencies for the new event.
        deps: BTreeSet<EventHash>,
        /// One-shot reply channel: hash of the published event, or
        /// the error that aborted authoring.
        reply: oneshot::Sender<Result<EventHash, RuntimeError>>,
    },

    /// Cooperative shutdown — runtime task exits its select loop.
    Shutdown,
}

/// Owner-side handle to a spawned per-topic runtime task.
///
/// Holding this handle is the only way to issue author commands or
/// observe the runtime's logs / digest stream / halt signal. Dropping
/// the handle does not halt the task; send [`AuthorCommand::Shutdown`]
/// or drop the underlying network for a clean exit.
pub struct RuntimeHandle {
    /// Send author / shutdown commands into the runtime task.
    pub author_tx: mpsc::Sender<AuthorCommand>,

    /// Append-only log of detected drift events.
    pub drift_log: Arc<Mutex<Vec<DriftDetected>>>,

    /// Append-only log of detected equivocations.
    pub equivocation_log: Arc<Mutex<Vec<EquivocationFlag>>>,

    /// Append-only log of non-fatal peer warnings.
    pub peer_warnings: Arc<Mutex<Vec<PeerWarning>>>,

    /// Latest state-digest published by the runtime.
    pub digest_watch: watch::Receiver<Vec<u8>>,

    /// `Some(reason)` once the runtime task halts; `None` while alive.
    pub halt_watch: watch::Receiver<Option<String>>,
}
