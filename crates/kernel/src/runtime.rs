//! Per-topic kernel runtime — drives event ingest, replay, drift emit.
//!
//! See `docs/specs/2026-05-10-plan-b-1-dag-memnet-design.md` §11.
//!
//! This module owns:
//! - configuration ([`RuntimeCfg`]) and error / observation types
//!   ([`RuntimeError`], [`EquivocationFlag`], [`PeerWarning`]);
//! - the per-topic [`Runtime`] task that ingests events from the
//!   network + author commands, runs the DAG / state-apply pipeline,
//!   and emits drift messages;
//! - the public-facing [`RuntimeHandle`] used by hosts and tests to
//!   author events, inspect observation logs, and watch state digests.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use bincode::Options;
use myrhiza_distribution::topic::{derive_publication_topic, derive_revocation_topic};
use myrhiza_distribution::{
    DistributionBackfillRequest, DistributionEnvelope, DistributionLogKind, PublicationEvent,
    PublicationHeads, PublicationLog, RevocationEvent, RevocationHeads, RevocationLog, dispatch,
};
use myrhiza_network::{
    ArcDistributionHandler, ArcRequestHandler, DistributionHandler, DistributionResponder,
    DistributionStream, GossipMessage, HeadsResponder, HeadsStream, NetError, Network,
    RequestHandler, SubError, Subscription,
};
use myrhiza_types::{
    AuthorPubkey, AuthorSeq, BlobHash, BundleHash, DirectHeadsRequest, DriftAnchor, DriftMessage,
    DriftSignedPayload, Event, EventHash, HeadsSummary, HeadsSummarySignedPayload, Hlc, PeerPubkey,
    Topic, canonical_bincode,
};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, watch};

use crate::dag::{DagError, EventDag, Inserted};
use crate::drift::{
    DriftDetected, DriftRateLimit, RateLimitKind, anchor_bound_map, anchor_covered, should_emit,
};
use crate::identity::{AuthorKeypair, PeerKeypair};
use crate::pending::{PendingBuffer, PendingCfg};
use crate::state_apply::{ApplyError, ApplyOutcome, StateApplyHandle};
use crate::state_propose::{ProposeError, StateProposeHandle};

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

    /// Cadence at which distribution-log head summaries
    /// (`RevocationHeads`/`PublicationHeads`) are broadcast for
    /// stale-network backfill. Distribution logs change far less often
    /// than the event DAG, so this is slower than `heads_summary_tick`.
    /// Per B-12 spec §3.5.
    pub distribution_sync_tick: Duration,

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

    /// Number of consecutive `SubError::TransportError` returns from
    /// `Subscription::recv` after which the runtime halts. Default 5.
    /// Set tighter (e.g. 2) in tests to validate the halt path.
    /// Per B-4.3 spec §3.2.
    pub transport_error_halt_threshold: usize,
}

impl Default for RuntimeCfg {
    fn default() -> Self {
        Self {
            drift_interval: 1024,
            drift_min_interval: Duration::from_mins(1),
            drift_daily_cap: 1024,
            heads_summary_tick: Duration::from_secs(5),
            distribution_sync_tick: Duration::from_secs(30),
            pending_cfg: PendingCfg::default(),
            broadcast_capacity: 256,
            kernel_fuel_table_version: 1,
            drift_stash_cap: 256,
            transport_error_halt_threshold: 5,
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

    /// `propose_and_author` issued on a runtime started without a
    /// state-propose component (`propose: None`). Per B-13 spec §4.4.
    #[error("runtime has no propose component installed")]
    NoProposeComponent,

    /// The state-propose component rejected the intent
    /// (`ProposeError::Rejected`). Distinct from `PreCheckRejected`: this
    /// is the app's own propose logic declining to produce a payload, not
    /// the kernel's state-apply dry-run rejecting the produced payload.
    /// Per B-13 spec §4.4.
    #[error("propose rejected: {0}")]
    ProposeRejected(String),
}

impl From<bincode::Error> for RuntimeError {
    fn from(e: bincode::Error) -> Self {
        RuntimeError::Canonical(e.to_string())
    }
}

impl From<ProposeError> for RuntimeError {
    /// Map a state-propose failure into the runtime error surface (B-13
    /// spec §4.4). The component's explicit `Err(msg)`
    /// (`ProposeError::Rejected`) becomes [`RuntimeError::ProposeRejected`]
    /// carrying `msg` verbatim — not the `Display` string, which would
    /// double-prefix "propose rejected:". A backend trap / fuel exhaustion
    /// (`ProposeError::Backend`) has no dedicated runtime variant; it is
    /// surfaced through the same `ProposeRejected` channel with its
    /// diagnostic string (no event is authored either way).
    fn from(e: ProposeError) -> Self {
        match e {
            ProposeError::Rejected(msg) => RuntimeError::ProposeRejected(msg),
            ProposeError::Backend(b) => RuntimeError::ProposeRejected(b.to_string()),
        }
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

/// Observation-log record: a peer-author's revocation event verified at
/// the gossip edge and applied to that author's [`RevocationLog`].
///
/// Surfaced via the `RuntimeHandle::revocation_events` poll-log (wired in
/// B-11 T3). Carries the bundle hash an embedder correlates against
/// installed bundles to drive the uninstall prompt
/// ([`distribution.md`] §10.5 step 7). This is an
/// observation event, not a state-apply path — surfacing order is the
/// per-peer gossip-arrival order (non-converging by design, like
/// [`DriftDetected`]). Per B-11 spec §3.6 / §4.2.
///
/// [`RevocationLog`]: myrhiza_distribution::RevocationLog
/// [`distribution.md`]: ../../../docs/specs/2026-05-09-myrhiza-master-design/distribution.md
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevocationApplied {
    /// Author whose revocation log advanced.
    pub author: AuthorPubkey,
    /// Bundle hash the author revoked.
    pub revoked_bundle_hash: BlobHash,
    /// `revocation_seq` of the applied event (monotonic per author).
    pub revocation_seq: u64,
}

/// Observation-log record: a peer-author's publication event verified at
/// the gossip edge and applied to that author's [`PublicationLog`].
///
/// Surfaced via the `RuntimeHandle::publication_events` poll-log (wired
/// in B-11 T3). Per B-11 spec §3.6 / §4.2. Like [`RevocationApplied`],
/// an observation event with no determinism contract.
///
/// [`PublicationLog`]: myrhiza_distribution::PublicationLog
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicationAnnounced {
    /// Author whose publication log advanced.
    pub author: AuthorPubkey,
    /// Hash of the manifest the author published.
    pub manifest_hash: BlobHash,
    /// Version string the author announced.
    pub version: String,
    /// `publication_seq` of the applied event (monotonic per author).
    pub publication_seq: u64,
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
    DriftRateLimited {
        /// Which rate-limit rule rejected the emit.
        kind: RateLimitKind,
    },

    /// Subscription consumer lagged and dropped messages.
    BroadcastLagged {
        /// Number of dropped messages reported by the transport.
        dropped: u64,
    },

    /// Wire-decode failure from a single peer. Distinct from
    /// `BroadcastLagged` — does NOT trigger `HeadsSummary`
    /// republishing. Per B-4.1 spec §3.0.
    DecodeFailed {
        /// The iroh-gossip last-hop neighbor (Plumtree forwarder,
        /// not necessarily the original publisher; Q-4 is B-4.2).
        /// `None` for transports without per-message sender identity.
        peer: Option<PeerPubkey>,
    },

    /// Local-ahead branch of `handle_heads_summary` could not look up our
    /// hash at the remote's claimed seq, despite our `local_seq > remote_seq`.
    /// Under DAG invariants this is unreachable (an author chain with head
    /// at `local_seq` must populate `seq_to_hash` for every seq in
    /// `1..=local_seq`). Surfacing as a warning rather than swallowing the
    /// no-op makes the invariant break visible (CLAUDE.md: no swallowing
    /// errors).
    ChainHashLookupMissing {
        /// Author whose chain we were inspecting.
        author: AuthorPubkey,
        /// Seq the lookup failed at (the remote's claimed head seq).
        seq: u64,
    },

    /// Pending-drain loop observed an `Inserted::Pending(_)` or `Err(_)`
    /// outcome from the inner `dag.insert` call. Under honest input this
    /// is unreachable: `PendingBuffer::newly_satisfied` filters on
    /// `is_subset(known)`, so the deps are present when we re-insert.
    /// Reaching this arm implies a DAG / pending consistency drift (e.g.
    /// a future refactor of `newly_satisfied`) or a non-fatal `DagError`
    /// (sig / genesis / chain). Surfaced rather than silently dropped so
    /// the invariant break is observable (CLAUDE.md: no swallowing
    /// errors; review Q-2).
    PendingDrainAnomaly {
        /// Author of the event the anomaly was observed on.
        author: AuthorPubkey,
        /// Seq of the event the anomaly was observed on.
        seq: u64,
        /// Free-form description of the anomaly (variant name +
        /// `Debug`-formatted detail for `Err`).
        reason: String,
    },

    /// Wire-decode succeeded but the cryptographic signature did NOT
    /// verify against the claimed `signed_by_peer` key. Distinct from
    /// `DecodeFailed` (parse failure) — this is "parsed cleanly, but
    /// the attribution claim is fraudulent." Per B-4.2 spec §2.
    /// Routes to log + drop; the body-consuming handler does NOT run.
    SignatureInvalid {
        /// The claimed `signed_by_peer` from the message. Note: this
        /// is the *claimed* identity — the signature failed to verify
        /// against this key, so the claim itself may be forged. Useful
        /// for observability + correlation, not for trust decisions.
        peer: Option<PeerPubkey>,
    },

    /// `Subscription::recv` returned a `SubError::TransportError`.
    /// Pushed on every sub-threshold occurrence (each error
    /// increments `consecutive`). The runtime halts when
    /// `consecutive` reaches `cfg.transport_error_halt_threshold`.
    /// Per B-4.3 spec §3.0.
    TransportError {
        /// Description string from the underlying error.
        reason: String,
        /// Consecutive-error count at the time this warning was
        /// pushed (1, 2, 3, ... up to `transport_error_halt_threshold`).
        consecutive: usize,
    },

    /// A direct-stream `request_heads` call to a peer failed before any
    /// events were streamed back — typically because the peer is
    /// unreachable, hasn't registered the heads-request ALPN, or has no
    /// handler installed. The runtime continues; the next periodic
    /// `HeadsSummary` tick or fresh inbound `HeadsSummary` will trigger a
    /// retry. Per B-4.5 spec §3.2.
    DirectRequestFailed {
        /// The target peer the request was directed at.
        peer: PeerPubkey,
        /// Human-readable diagnostic from `NetError::RequestFailed`.
        reason: String,
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

    /// Drive the installed state-propose component on `intent`, then
    /// author + publish the produced payload through the existing
    /// [`Runtime::author`] engine (sign + pre-check + insert + broadcast).
    /// Per B-13 spec §4.1. The private key stays kernel-side; propose
    /// never signs.
    ProposeAndAuthor {
        /// App-internal intent bytes (propose-component-defined schema).
        intent: Vec<u8>,
        /// One-shot reply channel: hash of the published event, or the
        /// error that aborted propose / authoring.
        reply: oneshot::Sender<Result<EventHash, RuntimeError>>,
    },

    /// Cooperative shutdown — runtime task exits its select loop.
    Shutdown,
}

/// Command sent to the runtime task by [`KernelRequestHandler`] when
/// an inbound direct-stream `HeadsRequest` arrives.
///
/// Drained by the select loop's `heads_req_rx.recv()` arm and processed
/// via [`Runtime::serve_direct_heads_request`].
///
/// Per B-4.5 spec §3.1.
pub(crate) struct HeadsRequestCommand {
    /// QUIC-TLS-confirmed pubkey of the peer that issued the request.
    pub(crate) requester: PeerPubkey,
    /// The decoded request payload (already topic-validated by the
    /// handler shim).
    pub(crate) request: DirectHeadsRequest,
    /// Sender half of the response stream; the runtime pushes events
    /// through `responder.send(event)`; dropping the responder signals
    /// clean EOF to the requester.
    pub(crate) responder: HeadsResponder,
}

/// [`RequestHandler`] impl installed by [`Runtime::start`] on the
/// underlying [`Network`]. Forwards inbound direct-stream requests to
/// the runtime task via mpsc; does topic validation (defense in depth)
/// to prevent the same handler being misregistered on a different
/// topic.
///
/// Per B-4.5 spec §3.1.
pub(crate) struct KernelRequestHandler {
    /// Sender half of the runtime's inbound-direct-request mailbox.
    tx: mpsc::Sender<HeadsRequestCommand>,
    /// The topic this handler services. Inbound requests for any other
    /// topic are silently dropped (clean EOF).
    topic: Topic,
}

#[async_trait::async_trait]
impl RequestHandler for KernelRequestHandler {
    /// Topic-validate then forward the request into the runtime task's
    /// mailbox. Returns immediately (drops the responder, yielding
    /// clean EOF to the requester) when the request targets a topic
    /// this handler does not service. Otherwise moves the responder
    /// into the [`HeadsRequestCommand`] and sends; if the runtime task
    /// has already exited, the send fails silently — dropping the
    /// responder yields clean EOF on the requester side too. Per B-4.5
    /// spec §3.1.
    async fn handle(
        &self,
        requester: PeerPubkey,
        request: DirectHeadsRequest,
        responder: HeadsResponder,
    ) {
        // Defense in depth — confirm the request targets the topic
        // this runtime services. The IrohNetwork routes by peer+ALPN;
        // this guards against an embedder that registers the same
        // handler against multiple per-topic networks by mistake.
        if request.topic != self.topic {
            // Drop responder — requester sees clean EOF.
            return;
        }
        // Forward to runtime. If the runtime task has exited, the
        // send fails; dropping the responder yields EOF to the
        // requester. No diagnostic surfaced — the runtime has already
        // shut down, there is nothing to log into.
        let _ = self
            .tx
            .send(HeadsRequestCommand {
                requester,
                request,
                responder,
            })
            .await;
    }
}

/// Command sent to the runtime task by [`KernelDistributionHandler`] when
/// an inbound direct-stream `DistributionBackfillRequest` arrives.
///
/// Drained by the select loop's `distribution_req_rx.recv()` arm and
/// processed via [`Runtime::serve_distribution_request`]. This is the
/// serve-side twin of [`HeadsRequestCommand`] for the B-12 §14 corrected
/// transport: a behind peer dials this peer over
/// [`DISTRIBUTION_REQUEST_ALPN`](myrhiza_network::DISTRIBUTION_REQUEST_ALPN)
/// and pulls the missing signed envelopes from this peer's archive.
///
/// Per B-12 spec §14.4.
pub(crate) struct DistributionRequestCommand {
    /// QUIC-TLS-confirmed pubkey of the peer that issued the request.
    /// Captured for parity with [`HeadsRequestCommand`] and a future
    /// per-requester rate-limit hook; the serve path does not yet branch
    /// on it (the author gate already lives in the handler).
    pub(crate) requester: PeerPubkey,
    /// The decoded backfill request (already author-validated by the
    /// handler shim — see [`KernelDistributionHandler`]).
    pub(crate) request: DistributionBackfillRequest,
    /// Sender half of the response stream; the runtime pushes envelopes
    /// through `responder.send(envelope)`; dropping the responder signals
    /// clean EOF to the requester.
    pub(crate) responder: DistributionResponder,
}

/// [`DistributionHandler`] impl installed by [`Runtime::start`] on the
/// underlying [`Network`], alongside [`KernelRequestHandler`]. Forwards
/// inbound direct-stream distribution-backfill requests to the runtime
/// task via mpsc, after gating on the requested author.
///
/// **Author gate (defense in depth, spec §14.4):** the handler serves a
/// request only when its `author` is one this runtime is installed for.
/// A request for any other author is dropped (the responder is dropped →
/// clean EOF to the requester), so this peer never streams envelopes for
/// an author it does not track — symmetric with the topic gate on
/// [`KernelRequestHandler`]. The `IrohNetwork` already routes by peer+ALPN;
/// this guards against a confused-deputy pull for an unrelated author.
///
/// Per B-12 spec §14.4.
pub(crate) struct KernelDistributionHandler {
    /// Sender half of the runtime's inbound-distribution-request mailbox.
    tx: mpsc::Sender<DistributionRequestCommand>,
    /// The authors this runtime serves. Inbound requests for any other
    /// author are silently dropped (clean EOF).
    installed_authors: Vec<AuthorPubkey>,
}

#[async_trait::async_trait]
impl DistributionHandler for KernelDistributionHandler {
    /// Author-gate then forward the request into the runtime task's
    /// mailbox. Returns immediately (dropping the responder → clean EOF
    /// to the requester) when the request targets an author this handler
    /// does not serve. Otherwise moves the responder into the
    /// [`DistributionRequestCommand`] and sends; if the runtime task has
    /// already exited, the send fails silently and dropping the responder
    /// yields clean EOF on the requester side too. Per B-12 spec §14.4.
    async fn handle(
        &self,
        requester: PeerPubkey,
        request: DistributionBackfillRequest,
        responder: DistributionResponder,
    ) {
        // Author gate — only serve authors this runtime is installed for.
        // A request for any other author drops the responder here →
        // requester sees a clean EOF (empty stream).
        if !self.installed_authors.contains(&request.author) {
            return;
        }
        // Forward to runtime. If the runtime task has exited, the send
        // fails; dropping the responder yields EOF to the requester. No
        // diagnostic surfaced — the runtime has already shut down.
        let _ = self
            .tx
            .send(DistributionRequestCommand {
                requester,
                request,
                responder,
            })
            .await;
    }
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

    /// Map of events rejected by `state-apply` during `replay_full`,
    /// keyed by [`EventHash`] (the event's `wire_hash`) and valued with
    /// the reject reason returned by the component.
    ///
    /// Per spec §4.4 / §14 edge-case 8: a Reject at apply time does not
    /// remove the event from the DAG — it remains for future
    /// re-evaluation under a different topo ordering — but it is not
    /// committed into `state`. Surfacing the map here (review-finding
    /// M-4) makes those drops observable for diagnostics rather than
    /// silently swallowed.
    pub dropped_at_apply: Arc<Mutex<HashMap<EventHash, String>>>,

    /// Latest state-digest published by the runtime.
    pub digest_watch: watch::Receiver<Vec<u8>>,

    /// `Some(reason)` once the runtime task halts; `None` while alive.
    pub halt_watch: watch::Receiver<Option<String>>,

    /// Diagnostic counter of tip-fast-path engagements in
    /// [`Runtime::try_tip_incremental`]. Acceptance tests read this to
    /// verify the fast path is taken when expected and skipped when not
    /// (e.g. drain-loop multi-insert, re-topo). Per plan-B-2.1 spec §5.
    ///
    /// Always-on (matches the `dropped_at_apply` diagnostic pattern):
    /// the field's overhead is one `Arc<Mutex<usize>>` per runtime and
    /// one mutex lock per fast-path engagement — negligible compared to
    /// the apply call the increment is paired with. Surfacing this on
    /// `RuntimeHandle` rather than gating with `#[cfg(test)]` lets
    /// integration tests in `crates/kernel/tests/` read it (cargo
    /// compiles the kernel library without `--test` when building
    /// dependent integration test crates).
    pub tip_fast_path_hits: Arc<Mutex<usize>>,

    /// Append-only log of revocation events verified + applied for an
    /// installed author's revocation topic. Clone of the same `Arc` the
    /// runtime task writes (spec §3.5 poll-log pattern, twin of
    /// `drift_log`). An embedder polls this to learn which bundle hashes
    /// were flagged and drive the [`distribution.md`] §10.5 step-7
    /// uninstall prompt. Per B-11 spec §4.3.
    ///
    /// [`distribution.md`]: ../../../docs/specs/2026-05-09-myrhiza-master-design/distribution.md
    pub revocation_events: Arc<Mutex<Vec<RevocationApplied>>>,

    /// Append-only log of publication events verified + applied for an
    /// installed author's publication topic. Twin of `revocation_events`.
    /// Per B-11 spec §4.3.
    pub publication_events: Arc<Mutex<Vec<PublicationAnnounced>>>,

    /// Per-author wall-clock of the last received distribution message
    /// (event or summary), cloned from the runtime task's map. Poll it —
    /// or use [`RuntimeHandle::stale_authors`] — to surface the master
    /// `distribution.md` §10.7 "potentially stale" warning before
    /// installing a new version. Per B-12 spec §3.7.
    pub last_distribution_sync: Arc<Mutex<BTreeMap<AuthorPubkey, SystemTime>>>,

    /// The authors this runtime was started with (its distribution
    /// subscription set). Retained on the handle so [`Self::stale_authors`]
    /// knows the full installed set — including authors that have *never*
    /// synced and so have no `last_distribution_sync` entry. Per B-12 spec
    /// §3.7.
    pub installed_authors: Vec<AuthorPubkey>,
}

impl RuntimeHandle {
    /// Installed authors with no fresh distribution sync within
    /// `threshold` of `now` — i.e. those for which a "potentially stale"
    /// warning should be surfaced before installing a new version (master
    /// `distribution.md` §10.7; default threshold 24h). An author with no
    /// recorded sync is always stale. `now` is a parameter (not
    /// `SystemTime::now()`) so tests are deterministic. The installed set is
    /// taken from the handle (`installed_authors`), not a caller argument —
    /// the runtime already knows which authors it serves. Per B-12 spec
    /// §3.7 / §12 Q5.
    ///
    /// # Panics
    /// Panics if the `last_distribution_sync` mutex is poisoned — i.e. the
    /// runtime task panicked while holding it. Structurally unreachable in a
    /// healthy run (the runtime task is the only writer).
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn stale_authors(&self, now: SystemTime, threshold: Duration) -> Vec<AuthorPubkey> {
        let map = self
            .last_distribution_sync
            .lock()
            .expect("last_distribution_sync mutex poisoned");
        self.installed_authors
            .iter()
            .filter(|author| match map.get(author) {
                // Never synced ⇒ always stale.
                None => true,
                // Synced in the future (clock skew) counts as fresh; only
                // an elapsed gap strictly greater than the threshold is stale.
                Some(last) => now
                    .duration_since(*last)
                    .is_ok_and(|elapsed| elapsed > threshold),
            })
            .copied()
            .collect()
    }

    /// Drive the runtime's state-propose component on `intent`, author the
    /// produced payload, and return the published event's hash. Per B-13
    /// spec §4.1 / §4.2.
    ///
    /// Sends [`AuthorCommand::ProposeAndAuthor`] into the runtime task and
    /// awaits the one-shot reply — the same mpsc+oneshot round-trip shape
    /// as the author path. The runtime signs with its single installed
    /// `author_key`; propose never sees the key.
    ///
    /// # Errors
    /// Returns [`RuntimeError::ReadOnly`] (no author key),
    /// [`RuntimeError::NoProposeComponent`] (no propose component),
    /// [`RuntimeError::ProposeRejected`] (propose declined / trapped),
    /// [`RuntimeError::PreCheckRejected`] (produced payload failed the
    /// state-apply dry-run), or any other error [`Runtime::author`]
    /// surfaces. Returns [`RuntimeError::Canonical`] if the runtime task
    /// has shut down (command channel closed or reply dropped).
    pub async fn propose_and_author(&self, intent: Vec<u8>) -> Result<EventHash, RuntimeError> {
        let (reply, rx) = oneshot::channel();
        self.author_tx
            .send(AuthorCommand::ProposeAndAuthor { intent, reply })
            .await
            .map_err(|e| RuntimeError::Canonical(format!("runtime task closed: {e}")))?;
        rx.await
            .map_err(|e| RuntimeError::Canonical(format!("runtime reply dropped: {e}")))?
    }
}

/// Bounded count of peers tracked per author in the
/// peer-authority index. Older entries are evicted on overflow.
/// Per B-4.6 spec §2 (decision table).
pub(crate) const PEER_AUTHORITY_PER_AUTHOR_CAP: usize = 8;

/// Maximum number of backfill *dials* a peer will issue for a single
/// advertiser within the trailing-24h window (`distribution_dial_limit`
/// bucket capacity, spec §14.1 / §12 Q3).
///
/// In the corrected pull transport (spec §14) a behind peer that hears an
/// advertiser's above-our-head summary *dials* that advertiser to pull
/// the missing envelopes. A forged-high summary therefore costs at most
/// one wasted dial; this cap bounds how *often* a single advertiser can
/// goad us into dialing within the window, so a flood of forged-high
/// summaries cannot weaponise us into a dial storm against one peer. One
/// "dial" is one `request_distribution` call (which may stream a
/// contiguous range of envelopes back — the range size is bounded by the
/// author's own log length, not by this cap). Chosen generously
/// (distribution logs change a handful of times per author *ever*, so a
/// legitimate peer never approaches this) while still bounding a forged
/// flood. Per B-12 spec §12 Q3 ("start from `DriftRateLimit` defaults,
/// tune if the flood test needs it").
pub(crate) const DISTRIBUTION_DIAL_DAILY_CAP: u32 = 32;

/// Per-topic runtime — owns the event DAG, pending buffer, and
/// state-apply handle for a single `(topic, app_bundle_hash)` binding.
///
/// Constructed via [`Runtime::start`], which spawns a task that drives
/// the [`Runtime::run`] select loop and returns a [`RuntimeHandle`] for
/// commands + observation. The struct itself is not `Clone`; ownership
/// lives inside the spawned task.
///
/// The boxed `dyn Network` field with a uniform [`Subscription`] type is
/// established via the [`NetworkErased`] wrapper below — see its doc for
/// the rationale.
#[allow(dead_code)]
pub struct Runtime {
    /// Network handle, type-erased so multiple `Network` implementors
    /// can be swapped without re-parameterizing `Runtime`. See
    /// [`NetworkErased`].
    network: Arc<dyn Network<Subscription = Box<dyn Subscription + Send>>>,
    /// Topic this runtime operates on.
    topic: Topic,
    /// Bundle hash bound to this topic (genesis-derived).
    app_bundle_hash: BundleHash,
    /// Human-readable topic name (carried for genesis re-derivation).
    topic_name: String,
    /// Event DAG aggregator.
    dag: EventDag,
    /// Out-of-order event buffer.
    pending: PendingBuffer,
    /// State-apply ABI handle.
    handle: StateApplyHandle,
    /// Optional state-propose ABI handle. `Some` when the embedder
    /// installed a propose component for this app, enabling
    /// [`Self::propose_and_author`]; `None` for runtimes that only apply
    /// (consume) events. Per B-13 spec §4.2.
    propose: Option<StateProposeHandle>,
    /// Latest replayed state bytes.
    state: Vec<u8>,
    /// Cached topo order corresponding to `self.state`. Per
    /// plan-B-2.1 spec §3.1. Used by [`Self::try_tip_incremental`]
    /// to detect tip-fast-path eligibility — when a new topo extends
    /// this by exactly one tail element, the new event can be applied
    /// incrementally instead of replaying from scratch.
    last_topo_order: Vec<EventHash>,
    /// Peer identity (signs drift messages).
    peer_key: PeerKeypair,
    /// Author identity (signs authored events). `None` for read-only
    /// runtimes — author commands return [`RuntimeError::ReadOnly`].
    author_key: Option<AuthorKeypair>,
    /// Runtime configuration knobs.
    cfg: RuntimeCfg,
    /// Sliding-window rate limiter for drift-message emission.
    rate_limit: DriftRateLimit,
    /// Memoized own-digest at each `AuthorSeqVec` anchor we have
    /// computed (avoids replay on every incoming drift).
    own_digest_cache: BTreeMap<Vec<myrhiza_types::AuthorSeq>, [u8; 32]>,
    /// Incoming drift messages whose anchor is not yet covered locally.
    /// Keyed by the anchor `AuthorSeqVec`; drained as new events arrive.
    incoming_drift_pending:
        BTreeMap<Vec<myrhiza_types::AuthorSeq>, Vec<myrhiza_types::DriftMessage>>,
    /// Observation log — surfaced via [`RuntimeHandle::drift_log`].
    drift_log: Arc<Mutex<Vec<DriftDetected>>>,
    /// Observation log — surfaced via [`RuntimeHandle::equivocation_log`].
    equivocation_log: Arc<Mutex<Vec<EquivocationFlag>>>,
    /// Observation log — surfaced via [`RuntimeHandle::peer_warnings`].
    peer_warnings: Arc<Mutex<Vec<PeerWarning>>>,
    /// Observation log — surfaced via [`RuntimeHandle::dropped_at_apply`].
    ///
    /// Populated by [`Runtime::replay_full`] whenever `state-apply`
    /// returns [`ApplyOutcome::Rejected`] for an event already in the
    /// DAG. Per spec §4.4 / §14 edge-case 8: the event is *not* removed
    /// from the DAG (future state ordering may re-accept it); only the
    /// state-materialization step skips it. Recording the reject reason
    /// here (review-finding M-4) keeps the drop diagnosable.
    dropped_at_apply: Arc<Mutex<HashMap<EventHash, String>>>,
    /// Watch-side of the digest stream; published on every replay.
    digest_watch_tx: watch::Sender<Vec<u8>>,
    /// Watch-side of the halt signal; populated on fatal runtime error.
    halt_watch_tx: watch::Sender<Option<String>>,
    /// HLC logical-component counter (Tasks 18-19 use; declared here so
    /// the field set is stable across the scaffold commit).
    hlc_logical_counter: u32,
    /// Diagnostic instrumentation: counts engagements of the
    /// tip-fast-path in [`Self::try_tip_incremental`] (both `Accepted`
    /// and `Rejected` outcomes). Used by B-2.1 acceptance tests to
    /// assert the fast path is being taken when expected and skipped
    /// when not. Shared with [`RuntimeHandle::tip_fast_path_hits`] so
    /// the test thread can read the counter. Per plan-B-2.1 spec §5
    /// test 1. See [`RuntimeHandle::tip_fast_path_hits`] for the
    /// always-on rationale.
    tip_fast_path_hits: Arc<Mutex<usize>>,

    /// Count of consecutive `SubError::TransportError` returns from
    /// the active subscription. Resets to 0 on `Ok(Some(_))`.
    /// `Ok(None)` exits the task immediately (clean close path)
    /// without touching this counter. `Lagged` and `DecodeFailed`
    /// leave it unchanged (neutral). When this reaches
    /// `cfg.transport_error_halt_threshold`, the runtime signals
    /// halt and exits the task. Per B-4.3 spec §3.3.
    consecutive_transport_errors: usize,

    /// Mailbox for inbound direct-stream `HeadsRequest` commands. The
    /// [`KernelRequestHandler`] installed on `network` at startup is the
    /// only sender. Drained by the select loop's `heads_req_rx.recv()`
    /// arm and processed by [`Self::serve_direct_heads_request`].
    /// Per B-4.5 spec §3.3.
    heads_req_rx: mpsc::Receiver<HeadsRequestCommand>,

    /// Mailbox for inbound direct-stream distribution-backfill requests.
    /// The [`KernelDistributionHandler`] installed on `network` at startup
    /// (alongside the heads handler) is the only sender. Drained by the
    /// select loop's `distribution_req_rx.recv()` arm and processed by
    /// [`Self::serve_distribution_request`], which streams the requested
    /// envelopes back from `revocation_archive` / `publication_latest`.
    /// The serve-side twin of `heads_req_rx`. Per B-12 spec §14.4.
    distribution_req_rx: mpsc::Receiver<DistributionRequestCommand>,

    /// Mailbox for events arriving on direct-stream backfill responses.
    /// The drainer task spawned by [`Self::issue_direct_backfill`] is
    /// the only sender (cloned from `internal_event_tx`). Drained by
    /// the select loop's `internal_event_rx.recv()` arm and processed
    /// via [`Self::handle_event`] — identical path to gossip.
    /// Per B-4.5 spec §3.3.
    internal_event_rx: mpsc::Receiver<Event>,

    /// Sender half of `internal_event_rx`, retained so it can be
    /// cloned into drainer tasks. Cloning into the drainer (rather
    /// than passing the receiver) means multiple in-flight backfill
    /// responses can all feed events into the same channel.
    /// Per B-4.5 spec §3.3.
    internal_event_tx: mpsc::Sender<Event>,

    /// Peer-authority index: for each author, the list of peers
    /// observed to have signed a `HeadsSummary` advertising authority
    /// over that author. Ordered most-recently-observed first; capped
    /// at [`PEER_AUTHORITY_PER_AUTHOR_CAP`] entries per author
    /// (least-recently-observed evicted on overflow). Populated by
    /// [`Self::record_peer_authority`] from `handle_heads_summary`.
    /// Queried by [`Self::lookup_peer_for_author`] from
    /// `request_author_chain_gap` to pick a direct-stream target.
    /// Per B-4.6 spec §3.1.
    peer_authority_index: BTreeMap<AuthorPubkey, Vec<PeerPubkey>>,

    /// Per-author revocation-log state. One entry per installed author
    /// (lazily defaulted on first inbound event). Advanced by
    /// [`Self::handle_revocation`] after the gossip-edge signature check
    /// and the monotonic-seq / flood-cap checks in
    /// [`RevocationLog::apply`]. Per B-11 spec §4.3.
    revocation_logs: BTreeMap<AuthorPubkey, RevocationLog>,

    /// Per-author publication-log state. Structural twin of
    /// `revocation_logs`; advanced by [`Self::handle_publication`].
    /// Per B-11 spec §4.3.
    publication_logs: BTreeMap<AuthorPubkey, PublicationLog>,

    /// Archive of signed revocation envelopes, keyed by author then
    /// `revocation_seq`. The pure-tier [`RevocationLog`] folds events
    /// into a `revoked_bundles` set + `last_observed_seq` and discards
    /// the signatures, so it cannot *serve* a backfill; this archive
    /// retains the full per-author envelope sequence (revocation
    /// accumulates — every event contributes a distinct
    /// `revoked_bundle_hash`, so a complete set needs every event in
    /// range). Populated in [`Self::handle_revocation`] on a successful
    /// apply, so gossip-received and backfill-received events archive
    /// uniformly. A backfill-serving concern, deliberately kept in the
    /// kernel rather than the deterministic fold. Per B-12 spec
    /// §3.3 / §4.2.
    revocation_archive: BTreeMap<AuthorPubkey, BTreeMap<u64, RevocationEvent>>,

    /// Latest signed publication envelope per author. Publication is
    /// latest-wins, so a single newest envelope reconstructs the entire
    /// observable state and is all a served backfill needs (asymmetric
    /// with `revocation_archive`, which keeps the full range — see its
    /// doc). Populated in [`Self::handle_publication`] on a successful
    /// apply; read by [`Self::serve_distribution_request`] to answer a
    /// behind peer's pull. Per B-12 spec §3.3 / §4.2.
    publication_latest: BTreeMap<AuthorPubkey, PublicationEvent>,

    /// Dial guard for backfill pulls — one token bucket per advertiser.
    /// In the corrected pull transport (spec §14) a behind peer that hears
    /// an advertiser's above-our-head `RevocationHeads`/`PublicationHeads`
    /// summary *dials* that advertiser to pull the missing envelopes. Each
    /// such dial must first consume a bucket slot keyed by the advertiser;
    /// a burst of forged-high summaries from one advertiser is absorbed by
    /// the bucket rather than weaponising this peer into a dial storm
    /// against that advertiser
    /// ([`Self::handle_revocation_heads`] / [`Self::handle_publication_heads`]).
    /// Reuses the [`DriftRateLimit`] sliding-window shape (lazily created
    /// per advertiser on first dial opportunity). Per B-12 spec §14.1 / §14.4.
    distribution_dial_limit: BTreeMap<PeerPubkey, DriftRateLimit>,

    /// In-flight distribution pulls, keyed by `(author, kind)`. Set when
    /// [`Self::issue_distribution_backfill`] spawns a drainer, cleared by
    /// the drainer when the response stream ends. A fresh
    /// above-our-head summary for an `(author, kind)` already being pulled
    /// is a no-op, so a burst of advertisements (e.g. several ahead peers
    /// each re-advertising) does not fan out into redundant concurrent
    /// dials for the same gap. Shared with the drainer task via `Arc`.
    /// Per B-12 spec §14.4 (the optional in-flight guard).
    distribution_in_flight: Arc<Mutex<BTreeSet<(AuthorPubkey, DistributionLogKind)>>>,

    /// Sender half of the shared distribution channel (clone of the half
    /// [`subscribe_distribution_topics`] wired into the per-author drainer
    /// tasks). Retained so [`Self::issue_distribution_backfill`]'s drainer
    /// can re-inject pulled envelopes into the apply path as
    /// `GossipMessage::Revocation`/`Publication`, reusing
    /// [`Self::handle_distribution_message`] (verify-edge → apply →
    /// archive → surface), identical to the gossip-received path. Per
    /// B-12 spec §14.4.
    distribution_tx: mpsc::Sender<(AuthorPubkey, GossipMessage)>,

    /// Mailbox for inbound revocation/publication gossip. Each
    /// `(author, GossipMessage)` is forwarded here by a per-subscription
    /// [`drain_distribution_sub`] task (one per derived topic per
    /// installed author). Drained by the select loop's
    /// `distribution_rx.recv()` arm and dispatched via
    /// [`Self::handle_distribution_message`]. The single-channel fan-in
    /// (spec §3.2) mirrors the `internal_event_tx` drainer pattern.
    distribution_rx: mpsc::Receiver<(AuthorPubkey, GossipMessage)>,

    /// Observation log — surfaced via `RuntimeHandle::revocation_events`.
    /// Appended by [`Self::handle_revocation`] on a verified+applied
    /// event. Per B-11 spec §3.5 / §4.3.
    revocation_events: Arc<Mutex<Vec<RevocationApplied>>>,

    /// Observation log — surfaced via `RuntimeHandle::publication_events`.
    /// Structural twin of `revocation_events`. Per B-11 spec §3.5 / §4.3.
    publication_events: Arc<Mutex<Vec<PublicationAnnounced>>>,

    /// Authors whose distribution topics this runtime subscribed on start.
    /// Retained so [`Self::broadcast_distribution_heads`] can advertise a
    /// head summary per author on the on-start + periodic sync. Per B-12
    /// spec §3.5 / §4.4.
    installed_authors: Vec<AuthorPubkey>,

    /// Per-author wall-clock of the last distribution message (event or
    /// summary) received from the network — evidence the author's
    /// distribution topic is reachable. Surfaced via
    /// `RuntimeHandle::last_distribution_sync` and the 24h
    /// `RuntimeHandle::stale_authors` helper (spec §3.7). Wall-clock lives
    /// only in this kernel orchestration task, never in any deterministic
    /// state-apply path. Per B-12 spec §3.7 / §4.2.
    last_distribution_sync: Arc<Mutex<BTreeMap<AuthorPubkey, SystemTime>>>,
}

impl Runtime {
    /// Spawn a runtime task bound to `(topic, app_bundle_hash)`.
    ///
    /// The returned [`RuntimeHandle`] is the only owner-side surface
    /// for author commands and log observation. The spawned task
    /// terminates on [`AuthorCommand::Shutdown`], on subscription
    /// close, or on a fatal [`RuntimeError`] (which populates
    /// [`RuntimeHandle::halt_watch`]).
    ///
    /// # Errors
    /// Propagates [`NetError`] (wrapped in [`RuntimeError::Network`])
    /// raised by the initial [`Network::subscribe`] call.
    // The wide parameter list is intentional: every field is required
    // up front so the spawned task has no setters to race against.
    // Bundling these into a config struct would shift the same
    // arguments to a builder without removing them; not worth the
    // indirection at this layer.
    #[allow(clippy::too_many_arguments)]
    pub async fn start<N: Network>(
        network: N,
        topic: Topic,
        app_bundle_hash: BundleHash,
        topic_name: String,
        handle: StateApplyHandle,
        peer_key: PeerKeypair,
        author_key: Option<AuthorKeypair>,
        propose: Option<StateProposeHandle>,
        cfg: RuntimeCfg,
        bootstrap: Vec<PeerPubkey>,
        installed_authors: Vec<AuthorPubkey>,
    ) -> Result<RuntimeHandle, RuntimeError> {
        let erased = NetworkErased::new(network);
        let sub = erased.subscribe(topic, bootstrap.clone()).await?;

        // B-11 §3.3 / §4.1: auto-subscribe each installed author's
        // revocation + publication topics, returning the receive side of
        // the single shared channel the sixth select arm polls (§3.2 —
        // mirrors the `internal_event_tx` drainer→mpsc→select-arm
        // pattern). `installed_authors` empty ⇒ zero extra subscriptions,
        // zero behavior change.
        let DistributionChannel {
            tx: distribution_tx,
            rx: distribution_rx,
        } = subscribe_distribution_topics(&erased, &installed_authors, &bootstrap).await?;

        let (author_tx, author_rx) = mpsc::channel(64);

        // NEW (B-4.5): create the direct-stream channels BEFORE building
        // the handler. Capacities are sized to match HEADS_STREAM_CHANNEL_
        // CAPACITY (responder mailbox) and the 256-events-per-EventRequest
        // bound from B-4.2 (response mailbox provides 128 deep buffer for
        // backpressure).
        let (heads_req_tx, heads_req_rx) = mpsc::channel::<HeadsRequestCommand>(32);
        let (internal_event_tx, internal_event_rx) = mpsc::channel::<Event>(128);

        // NEW (B-12 §14.4): the serve-side mailbox for inbound
        // distribution-backfill requests, twin of `heads_req_rx`. Same
        // capacity as the heads-request mailbox — backfills are tiny and
        // rare, so 32 in-flight requests is ample.
        let (distribution_req_tx, distribution_req_rx) =
            mpsc::channel::<DistributionRequestCommand>(32);

        // NEW (B-4.5): construct + install the handler on the erased
        // network. The trait method takes `&self`; NetworkErased delegates
        // to the inner N. Must run BEFORE the `Arc::new(erased)` below
        // (the Arc consumes `erased` into the Runtime field).
        let handler = KernelRequestHandler {
            tx: heads_req_tx,
            topic,
        };
        erased.install_request_handler(Arc::new(handler));

        // NEW (B-12 §14.4): install the distribution-backfill serve handler
        // alongside the heads handler. It author-gates on `installed_authors`
        // (the set this runtime serves) and forwards admitted requests into
        // `distribution_req_rx` for `serve_distribution_request`. Installed
        // before `Arc::new(erased)` for the same reason as the heads handler.
        let distribution_handler = KernelDistributionHandler {
            tx: distribution_req_tx,
            installed_authors: installed_authors.clone(),
        };
        erased.install_distribution_handler(Arc::new(distribution_handler));

        let drift_log = Arc::new(Mutex::new(Vec::new()));
        let equivocation_log = Arc::new(Mutex::new(Vec::new()));
        let peer_warnings = Arc::new(Mutex::new(Vec::new()));
        let dropped_at_apply = Arc::new(Mutex::new(HashMap::new()));
        let tip_fast_path_hits = Arc::new(Mutex::new(0_usize));
        let revocation_events = Arc::new(Mutex::new(Vec::new()));
        let publication_events = Arc::new(Mutex::new(Vec::new()));
        let last_distribution_sync = Arc::new(Mutex::new(BTreeMap::new()));
        let (digest_watch_tx, digest_watch) = watch::channel(Vec::<u8>::new());
        let (halt_watch_tx, halt_watch) = watch::channel(None::<String>);

        let rate_limit = DriftRateLimit::new(cfg.drift_min_interval, cfg.drift_daily_cap);
        let dag = EventDag::new(topic, app_bundle_hash, topic_name.clone());
        let pending = PendingBuffer::new(cfg.pending_cfg.clone());

        let mut runtime = Runtime {
            network: Arc::new(erased),
            topic,
            app_bundle_hash,
            topic_name,
            dag,
            pending,
            handle,
            propose,
            state: Vec::new(),
            last_topo_order: Vec::new(),
            peer_key,
            author_key,
            cfg,
            rate_limit,
            own_digest_cache: BTreeMap::new(),
            incoming_drift_pending: BTreeMap::new(),
            drift_log: drift_log.clone(),
            equivocation_log: equivocation_log.clone(),
            peer_warnings: peer_warnings.clone(),
            dropped_at_apply: dropped_at_apply.clone(),
            digest_watch_tx,
            halt_watch_tx,
            hlc_logical_counter: 0,
            tip_fast_path_hits: tip_fast_path_hits.clone(),
            consecutive_transport_errors: 0,
            // NEW (B-4.5):
            heads_req_rx,
            internal_event_rx,
            internal_event_tx,
            // NEW (B-12 §14.4):
            distribution_req_rx,
            // NEW (B-4.6):
            peer_authority_index: BTreeMap::new(),
            // NEW (B-11):
            revocation_logs: BTreeMap::new(),
            publication_logs: BTreeMap::new(),
            // NEW (B-12):
            revocation_archive: BTreeMap::new(),
            publication_latest: BTreeMap::new(),
            distribution_dial_limit: BTreeMap::new(),
            distribution_in_flight: Arc::new(Mutex::new(BTreeSet::new())),
            distribution_tx,
            distribution_rx,
            revocation_events: revocation_events.clone(),
            publication_events: publication_events.clone(),
            installed_authors: installed_authors.clone(),
            last_distribution_sync: last_distribution_sync.clone(),
        };

        tokio::spawn(async move {
            let r = runtime.run(sub, author_rx).await;
            if let Err(e) = r {
                // The watch receiver lives in the RuntimeHandle the
                // caller holds; send only fails if the receiver was
                // dropped, in which case there is nothing to surface.
                let _ = runtime.halt_watch_tx.send(Some(format!("{e}")));
            }
        });

        Ok(RuntimeHandle {
            author_tx,
            drift_log,
            equivocation_log,
            peer_warnings,
            dropped_at_apply,
            digest_watch,
            halt_watch,
            tip_fast_path_hits,
            revocation_events,
            publication_events,
            last_distribution_sync,
            installed_authors,
        })
    }

    /// Main select loop driving the runtime task.
    ///
    /// `biased` ordering prioritizes author commands over inbound
    /// gossip — keeps locally-issued events ahead of remote backlog so
    /// reply latency stays low. The heads-summary ticker fires last.
    async fn run<S: Subscription>(
        &mut self,
        mut sub: S,
        mut author_rx: mpsc::Receiver<AuthorCommand>,
    ) -> Result<(), RuntimeError> {
        let mut ticker = tokio::time::interval(self.cfg.heads_summary_tick);
        let mut dist_ticker = tokio::time::interval(self.cfg.distribution_sync_tick);
        self.publish_heads_summary().await?;
        // B-12 §3.5 / §14.1: advertise our distribution-log heads on start
        // (the "sync on start" of master `distribution.md` §10.7) so a peer
        // that is *behind* hears our head and dials us to pull any missed
        // revocation/publication events (pull-on-behind). No-op when no
        // authors are installed.
        self.broadcast_distribution_heads().await?;
        loop {
            tokio::select! {
                biased;
                cmd = author_rx.recv() => match cmd {
                    Some(AuthorCommand::Author { payload, deps, reply }) => {
                        let r = self.author(payload, deps).await;
                        // oneshot send fails only if the caller dropped
                        // the receiver; nothing to do in that case.
                        let _ = reply.send(r);
                    }
                    // B-13 §4.1: run the propose component on the intent,
                    // then author the produced payload. Mirrors the Author
                    // arm above (same oneshot-drop tolerance).
                    Some(AuthorCommand::ProposeAndAuthor { intent, reply }) => {
                        let r = self.propose_and_author(intent).await;
                        let _ = reply.send(r);
                    }
                    Some(AuthorCommand::Shutdown) | None => return Ok(()),
                },
                Some(cmd) = self.heads_req_rx.recv() => {
                    self.serve_direct_heads_request(cmd).await;
                }
                // B-12 §14.4 (SELECT ARM 8): inbound direct-stream
                // distribution-backfill requests forwarded by the
                // KernelDistributionHandler. A behind peer dialed us; serve
                // the requested envelopes from the archive. Serve-side twin
                // of the heads-request arm above.
                Some(cmd) = self.distribution_req_rx.recv() => {
                    self.serve_distribution_request(cmd).await;
                }
                Some(event) = self.internal_event_rx.recv() => {
                    let _ = self.handle_event(event).await;
                }
                // B-11 §4.1 (SELECT ARM 6): inbound revocation/publication
                // gossip fanned in from the per-author drainer tasks. Also
                // carries pulled-backfill envelopes re-injected by
                // `drain_distribution_response` (spec §14.4).
                // B-12 §4.3 / §14.1: the handler is `async` — an above-our-head
                // `RevocationHeads`/`PublicationHeads` summary makes it dial
                // the advertiser and pull the missing envelopes
                // (pull-on-behind), so the arm must `.await` it. The dial is
                // fire-and-forget (spawns a drainer) and bounded by the
                // per-advertiser `distribution_dial_limit` bucket (§14.1), so
                // this `.await` cannot stall the loop on a forged-summary flood.
                Some((author, msg)) = self.distribution_rx.recv() => {
                    let _ = self.handle_distribution_message(author, msg).await;
                }
                recv_result = sub.recv() => match recv_result {
                    Ok(Some(m)) => {
                        self.consecutive_transport_errors = 0;
                        let _ = self.handle_message(m).await;
                    }
                    Ok(None) => return Ok(()),
                    Err(SubError::Lagged(n)) => {
                        // Mutex poisoning here would mean another task
                        // panicked while holding warnings — unreachable
                        // because the runtime task is the only writer.
                        #[allow(clippy::expect_used)]
                        self.peer_warnings
                            .lock()
                            .expect("peer_warnings mutex poisoned")
                            .push(PeerWarning::BroadcastLagged { dropped: n });
                        self.publish_heads_summary().await?;
                    }
                    Err(SubError::DecodeFailed { peer }) => {
                        // Wire-decode failure: a peer sent bytes that did not
                        // round-trip through canonical bincode. Distinct from
                        // Lagged on purpose — Lagged means "I missed messages,
                        // please backfill" and triggers publish_heads_summary;
                        // DecodeFailed means "this single peer sent garbage,
                        // discard it" and must NOT trigger backfill (a flood of
                        // garbage from one peer would otherwise spam HeadsSummary
                        // from every recipient).
                        //
                        // Per B-4.1 spec §2 (`SubError::DecodeFailed` row) +
                        // §3.0. The `peer` field is the iroh-gossip last-hop
                        // neighbor under Plumtree (not necessarily the original
                        // publisher; Q-4 is B-4.2's scope).
                        #[allow(clippy::expect_used)]
                        self.peer_warnings
                            .lock()
                            .expect("peer_warnings mutex poisoned")
                            .push(PeerWarning::DecodeFailed { peer });
                    }
                    Err(SubError::TransportError(reason)) => {
                        self.consecutive_transport_errors += 1;
                        if self.consecutive_transport_errors >= self.cfg.transport_error_halt_threshold {
                            let halt_msg = format!(
                                "transport halted: {} consecutive errors (latest: {})",
                                self.consecutive_transport_errors, reason
                            );
                            let _ = self.halt_watch_tx.send(Some(halt_msg));
                            return Ok(());
                        }
                        #[allow(clippy::expect_used)]
                        self.peer_warnings
                            .lock()
                            .expect("peer_warnings mutex poisoned")
                            .push(PeerWarning::TransportError {
                                reason,
                                consecutive: self.consecutive_transport_errors,
                            });
                    }
                },
                _ = ticker.tick() => { self.publish_heads_summary().await?; }
                // B-12 §3.5 (SELECT ARM 7): periodically re-advertise our
                // distribution-log heads, recovering from transient
                // partitions / `SubError::Lagged` on the distribution subs.
                _ = dist_ticker.tick() => { self.broadcast_distribution_heads().await?; }
            }
        }
    }

    /// Publish the current per-author tip summary on the topic.
    ///
    /// Called periodically by the heads-summary ticker, and on
    /// subscription-lag recovery to nudge peers into a backfill round.
    async fn publish_heads_summary(&mut self) -> Result<(), RuntimeError> {
        let authors = self.dag.author_heads();
        let signed_payload = HeadsSummarySignedPayload {
            authors: authors.clone(),
            kernel_fuel_table_version: self.cfg.kernel_fuel_table_version,
            topic: self.topic,
        };
        let signed_bytes = canonical_bincode()
            .serialize(&signed_payload)
            .map_err(|e| RuntimeError::Canonical(format!("HeadsSummarySignedPayload: {e}")))?;
        let signature = self.peer_key.sign(&signed_bytes);
        let summary = HeadsSummary {
            authors,
            kernel_fuel_table_version: self.cfg.kernel_fuel_table_version,
            signed_by_peer: self.peer_key.public,
            signature,
        };
        self.network
            .publish(self.topic, GossipMessage::HeadsSummary(summary))
            .await?;
        Ok(())
    }

    /// Broadcast a head summary (`RevocationHeads`/`PublicationHeads`)
    /// advertising our current `last_observed_seq` for each installed
    /// author on its derived distribution topics.
    ///
    /// Called once on start (the "sync on start" of master
    /// `distribution.md` §10.7) and from the `distribution_sync_tick` arm.
    /// Every peer advertises its head; a behind peer that hears an
    /// *above-our-head* summary dials the advertiser and pulls the delta
    /// (pull-on-behind, [`Self::handle_revocation_heads`]). The advertiser
    /// identity carried on each summary is the dial target. Convergence is
    /// the fixpoint of everyone advertising + behind-peers pulling
    /// (spec §14.1, the corrected transport — gossip-push is gone). An
    /// empty `installed_authors` makes this a no-op.
    ///
    /// # Errors
    /// Propagates a [`NetError`] (wrapped) from a summary `publish`.
    async fn broadcast_distribution_heads(&mut self) -> Result<(), RuntimeError> {
        // Our peer identity, stamped on each summary so receivers can filter
        // loopback (spec §3.2) and, in the corrected transport (spec §13),
        // know whom to dial for the pull.
        let advertiser = self.peer_key.public;
        // Clone the small author list so the publish calls below can take
        // `&mut self` without holding an immutable borrow of the field.
        for author in self.installed_authors.clone() {
            let rev_seq = self
                .revocation_logs
                .get(&author)
                .map_or(0, |log| log.last_observed_seq);
            self.network
                .publish(
                    Topic::from_bytes(derive_revocation_topic(author)),
                    GossipMessage::RevocationHeads(RevocationHeads {
                        author,
                        advertiser,
                        last_observed_seq: rev_seq,
                    }),
                )
                .await?;
            let pub_seq = self
                .publication_logs
                .get(&author)
                .map_or(0, |log| log.last_observed_seq);
            self.network
                .publish(
                    Topic::from_bytes(derive_publication_topic(author)),
                    GossipMessage::PublicationHeads(PublicationHeads {
                        author,
                        advertiser,
                        last_observed_seq: pub_seq,
                    }),
                )
                .await?;
        }
        Ok(())
    }

    /// Refresh the staleness clock for `author` — any inbound distribution
    /// message (event or summary, even a misroute) is evidence the topic
    /// is reachable (spec §3.7). The wall-clock read is confined to this
    /// kernel orchestration task; it never enters a state-apply path.
    #[allow(clippy::expect_used)]
    fn note_distribution_sync(&self, author: AuthorPubkey) {
        self.last_distribution_sync
            .lock()
            .expect("last_distribution_sync mutex poisoned")
            .insert(author, SystemTime::now());
    }

    /// Dispatch an inbound revocation/publication message (fanned in
    /// from a per-author drainer task) to the variant-specific handler.
    ///
    /// `author` is the topic-owner the message arrived for — carried
    /// alongside the message by [`drain_distribution_sub`] because the
    /// per-author topic is derived from this key and
    /// [`RevocationLog::apply`] / [`PublicationLog::apply`] need it to
    /// cross-check the signature.
    ///
    /// The runtime publishes `Revocation`, `Publication`, `RevocationHeads`,
    /// and `PublicationHeads` on these topics (the latter two added in
    /// B-12 T4's `broadcast_distribution_heads`). Any other
    /// [`GossipMessage`] variant on a distribution topic is structurally
    /// impossible from an honest peer; receiving one means a peer
    /// misrouted (or forged) wire traffic, so it is discarded with a
    /// [`PeerWarning::DecodeFailed`] — matching the spec §4.1 default arm
    /// and the app-topic `handle_message` mirror.
    ///
    /// This is `async` (B-12 §4.3 / §4.5): the `RevocationHeads` /
    /// `PublicationHeads` summary arms dispatch to the pull-on-behind
    /// handlers, which dial the advertiser and pull the missing signed
    /// envelopes when a summary lands *above* our head (spec §14.1). The
    /// returned `Result` is dropped by the sixth select arm (matching the
    /// `handle_event` / `handle_message` non-fatal-per-message pattern): a
    /// transient dial failure is surfaced as
    /// [`PeerWarning::DirectRequestFailed`] and recovered by the next
    /// `distribution_sync_tick` (spec §3.5), not a loop-halting fault.
    ///
    /// # Errors
    /// Currently infallible on the summary arms (the pull is fire-and-forget
    /// with errors surfaced as warnings); the `Result` is retained for the
    /// arm contract above.
    ///
    /// Per B-11 spec §3.4 / §4.1 and B-12 spec §3.2 / §14.1.
    async fn handle_distribution_message(
        &mut self,
        author: AuthorPubkey,
        msg: GossipMessage,
    ) -> Result<(), RuntimeError> {
        // B-12 §3.7: any inbound distribution message is evidence the
        // author's topic is reachable — refresh the staleness clock before
        // dispatch (covers events, summaries, and misroutes alike).
        //
        // Loopback filter (spec §3.2): a peer must ignore its own advertised
        // summaries. MemNetwork — and gossip overlays generally — may deliver
        // a peer its own broadcast; acting on it would make us "push to
        // ourselves" (then re-apply the push as a stale duplicate) and would
        // falsely refresh our own staleness clock. Mirrors the event-DAG
        // `HeadsSummary` loopback skip (`signed_by_peer != self`). Checked
        // BEFORE the sync-clock bump so a self-summary is not mistaken for
        // network reachability.
        match &msg {
            GossipMessage::RevocationHeads(h) if h.advertiser == self.peer_key.public => {
                return Ok(());
            }
            GossipMessage::PublicationHeads(h) if h.advertiser == self.peer_key.public => {
                return Ok(());
            }
            _ => {}
        }
        self.note_distribution_sync(author);
        match msg {
            GossipMessage::Revocation(ev) => self.handle_revocation(author, &ev),
            GossipMessage::Publication(ev) => self.handle_publication(author, ev),
            // RevocationHeads / PublicationHeads are legitimate B-12
            // backfill summaries that ride these distribution topics
            // (spec §3.2) — NOT misroutes, so they must NOT be classified
            // as DecodeFailed. They drive the pull-on-behind receive path
            // (dial the advertiser when its head is above ours, spec §14.1).
            GossipMessage::RevocationHeads(heads) => {
                self.handle_revocation_heads(author, heads).await?;
            }
            GossipMessage::PublicationHeads(heads) => {
                self.handle_publication_heads(author, heads).await?;
            }
            GossipMessage::Event(_) | GossipMessage::HeadsSummary(_) | GossipMessage::Drift(_) => {
                #[allow(clippy::expect_used)]
                self.peer_warnings
                    .lock()
                    .expect("peer_warnings mutex poisoned")
                    .push(PeerWarning::DecodeFailed { peer: None });
            }
        }
        Ok(())
    }

    /// Handle an inbound [`RevocationHeads`] summary: pull-on-behind for
    /// the revocation log (B-12 spec §14.1 / §14.4, the corrected pull
    /// transport).
    ///
    /// `topic_author` is the topic-owner the drainer tagged the message
    /// with (the key the per-author revocation topic was derived from);
    /// `heads.author` is the author the *summary* claims to describe. A
    /// mismatch means the summary arrived on the wrong per-author topic —
    /// a misroute or forgery — so it is discarded with
    /// [`PeerWarning::DecodeFailed`], mirroring the misrouted-`Revocation`
    /// guard (spec §3.2 / §4.3).
    ///
    /// On a matching author: if the advertised `last_observed_seq` is
    /// *above* our own log head (we are behind) and the per-advertiser
    /// dial-limit admits, we **dial the advertiser** and pull the missing
    /// envelopes via [`Self::issue_distribution_backfill`] — a
    /// point-to-point QUIC request, not a gossip re-broadcast, because the
    /// behind→ahead gossip path is unreliable for a late joiner (spec §13).
    /// The advertiser serves the contiguous `local+1..=remote` range from
    /// its archive; we feed each pulled envelope back through
    /// [`Self::handle_distribution_message`], idempotent via the
    /// monotonic-seq check. A summary at-or-below our head is a no-op
    /// beyond the (caller's) sync-clock bump — push-on-behind is gone, so
    /// hearing a *behind* peer's low summary no longer makes us act.
    ///
    /// # Errors
    /// Infallible in practice (the dial is fire-and-forget; request
    /// failures surface as [`PeerWarning::DirectRequestFailed`] inside
    /// [`Self::issue_distribution_backfill`]); the `Result` is retained for
    /// symmetry with the dispatch arm.
    #[allow(clippy::expect_used)]
    async fn handle_revocation_heads(
        &mut self,
        topic_author: AuthorPubkey,
        heads: RevocationHeads,
    ) -> Result<(), RuntimeError> {
        if heads.author != topic_author {
            self.peer_warnings
                .lock()
                .expect("peer_warnings mutex poisoned")
                .push(PeerWarning::DecodeFailed { peer: None });
            return Ok(());
        }
        let author = topic_author;
        let remote = heads.last_observed_seq;
        let local = self
            .revocation_logs
            .get(&author)
            .map_or(0, |log| log.last_observed_seq);
        // At or ahead of the advertiser — nothing to pull.
        if remote <= local {
            return Ok(());
        }
        // We are behind: pull the gap from the advertiser (spec §14.1). The
        // per-advertiser dial-limit + in-flight guard live in
        // `issue_distribution_backfill`.
        self.issue_distribution_backfill(
            heads.advertiser,
            author,
            DistributionLogKind::Revocation,
            local,
        )
        .await;
        Ok(())
    }

    /// Handle an inbound [`PublicationHeads`] summary: pull-on-behind for
    /// the publication log (B-12 spec §14.1 / §14.4).
    ///
    /// Structural twin of [`Self::handle_revocation_heads`]: on a summary
    /// *above* our head (matching author, dial-limit admits) we dial the
    /// advertiser and pull. Publication is latest-wins, so the advertiser
    /// serves the single newest envelope; the seq comparison and the
    /// pulled-envelope apply path are otherwise identical to the revocation
    /// twin. Mismatched-author summaries are discarded as
    /// [`PeerWarning::DecodeFailed`] exactly as in the revocation twin.
    ///
    /// # Errors
    /// Infallible in practice (see [`Self::handle_revocation_heads`]).
    #[allow(clippy::expect_used)]
    async fn handle_publication_heads(
        &mut self,
        topic_author: AuthorPubkey,
        heads: PublicationHeads,
    ) -> Result<(), RuntimeError> {
        if heads.author != topic_author {
            self.peer_warnings
                .lock()
                .expect("peer_warnings mutex poisoned")
                .push(PeerWarning::DecodeFailed { peer: None });
            return Ok(());
        }
        let author = topic_author;
        let remote = heads.last_observed_seq;
        let local = self
            .publication_logs
            .get(&author)
            .map_or(0, |log| log.last_observed_seq);
        if remote <= local {
            return Ok(());
        }
        self.issue_distribution_backfill(
            heads.advertiser,
            author,
            DistributionLogKind::Publication,
            local,
        )
        .await;
        Ok(())
    }

    /// Consume one slot from the per-advertiser backfill-dial token bucket
    /// (spec §14.1). Lazily creates the bucket on first use. Returns `true`
    /// if the dial is admitted, `false` if the bucket is exhausted (the
    /// dial-storm guard has tripped).
    ///
    /// `min_interval` is zero: the cap is purely the trailing-24h dial
    /// count (`DISTRIBUTION_DIAL_DAILY_CAP`), so legitimate back-to-back
    /// catch-up across distinct authors served by the same advertiser is
    /// not throttled while a forged-high-summary flood from one advertiser
    /// is still bounded.
    fn admit_distribution_dial(&mut self, advertiser: PeerPubkey) -> bool {
        let bucket = self
            .distribution_dial_limit
            .entry(advertiser)
            .or_insert_with(|| {
                DriftRateLimit::new(Duration::from_secs(0), DISTRIBUTION_DIAL_DAILY_CAP)
            });
        bucket.try_emit(std::time::Instant::now()).is_ok()
    }

    /// Dial `advertiser` and pull the missing distribution envelopes for
    /// `(author, kind)`, feeding each pulled envelope back into the apply
    /// path (spec §14.4, the pull counterpart to the deleted push branch).
    ///
    /// Gated twice before the dial: the per-advertiser dial-limit
    /// ([`Self::admit_distribution_dial`]) bounds how often one advertiser
    /// can goad us into dialing (a forged-high-summary defense), and the
    /// per-`(author, kind)` in-flight guard suppresses a redundant
    /// concurrent dial when a pull for the same gap is already running
    /// (e.g. several ahead peers re-advertising the same head). Either gate
    /// failing is a silent no-op.
    ///
    /// On admit, issues `network.request_distribution(advertiser, …)` for
    /// envelopes with `seq > from_seq`, then spawns
    /// [`drain_distribution_response`] to forward each received
    /// [`DistributionEnvelope`] into [`Self::distribution_tx`] as a
    /// `GossipMessage::Revocation`/`Publication` — the runtime then
    /// processes them through [`Self::handle_distribution_message`]
    /// exactly as if they had arrived over gossip (verify-edge → apply →
    /// archive → surface), idempotent via the monotonic-seq check. The
    /// drainer clears the in-flight marker when the stream ends.
    ///
    /// On `NetError::RequestFailed`, pushes a
    /// [`PeerWarning::DirectRequestFailed`] and returns (clearing the
    /// in-flight marker); the next periodic `distribution_sync_tick` or a
    /// fresh inbound summary triggers a retry. Mirrors
    /// [`Self::issue_direct_backfill`] (the event-DAG pull).
    #[allow(clippy::expect_used)]
    async fn issue_distribution_backfill(
        &mut self,
        advertiser: PeerPubkey,
        author: AuthorPubkey,
        kind: DistributionLogKind,
        from_seq: u64,
    ) {
        // Suppress a redundant concurrent pull for the same gap. Insert
        // returns false if the marker was already present.
        {
            let mut in_flight = self
                .distribution_in_flight
                .lock()
                .expect("distribution_in_flight mutex poisoned");
            if !in_flight.insert((author, kind)) {
                return;
            }
        }
        // Per-advertiser dial-limit (forged-high-summary dial-storm guard).
        // Checked AFTER claiming the in-flight slot so a denied dial must
        // release it again.
        if !self.admit_distribution_dial(advertiser) {
            self.clear_distribution_in_flight(author, kind);
            return;
        }
        let request = DistributionBackfillRequest {
            author,
            kind,
            from_seq,
        };
        let stream = match self.network.request_distribution(advertiser, request).await {
            Ok(s) => s,
            Err(e) => {
                self.peer_warnings
                    .lock()
                    .expect("peer_warnings mutex poisoned")
                    .push(PeerWarning::DirectRequestFailed {
                        peer: advertiser,
                        reason: format!("{e}"),
                    });
                self.clear_distribution_in_flight(author, kind);
                return;
            }
        };
        // Spawn a drainer that re-injects each pulled envelope into the
        // shared distribution channel (tagged with `author`, as the gossip
        // drainers do) and clears the in-flight marker on completion.
        let tx = self.distribution_tx.clone();
        let in_flight = self.distribution_in_flight.clone();
        tokio::spawn(drain_distribution_response(
            stream, author, kind, tx, in_flight,
        ));
    }

    /// Clear the `(author, kind)` in-flight-pull marker. Called on the
    /// early-return dial-denied / request-failed paths in
    /// [`Self::issue_distribution_backfill`]; the drainer clears it on the
    /// happy path. Idempotent (removing an absent key is a no-op).
    #[allow(clippy::expect_used)]
    fn clear_distribution_in_flight(&self, author: AuthorPubkey, kind: DistributionLogKind) {
        self.distribution_in_flight
            .lock()
            .expect("distribution_in_flight mutex poisoned")
            .remove(&(author, kind));
    }

    /// Verify + apply an inbound [`RevocationEvent`] for `author`.
    ///
    /// Edge-verification order is load-bearing (spec §3.4): the
    /// gossip-edge signature check ([`dispatch::verify_revocation`]) runs
    /// FIRST so a forged-signature event is classified as a forgery
    /// rather than mis-attributed as a benign stale-seq duplicate (which
    /// is what [`RevocationLog::apply`]'s internal `reason-len →
    /// seq-monotonic → seq-jump → signature` order would do). On a verify
    /// failure we push [`PeerWarning::SignatureInvalid`] and drop.
    ///
    /// On verify success we clone the prior log, call `apply` (which
    /// consumes the clone), and:
    /// - `Ok(new)` — install the advanced log and push a
    ///   [`RevocationApplied`] onto the poll-log surface.
    /// - `Err(_)` — a seq/length rejection (the signature already
    ///   verified). Push [`PeerWarning::DecodeFailed`] and re-insert the
    ///   unchanged prior log (the only correct pattern for `apply`'s
    ///   consume-and-return API; spec §5 last row).
    ///
    /// Per B-11 spec §3.4 / §4.1.
    ///
    /// `ev` is borrowed (not consumed): every field surfaced onto
    /// [`RevocationApplied`] is `Copy`, so the caller's owned event need
    /// not be moved — unlike [`Self::handle_publication`], which moves
    /// the `version` `String`.
    #[allow(clippy::expect_used)]
    fn handle_revocation(&mut self, author: AuthorPubkey, ev: &RevocationEvent) {
        if dispatch::verify_revocation(ev, &author).is_err() {
            self.peer_warnings
                .lock()
                .expect("peer_warnings mutex poisoned")
                .push(PeerWarning::SignatureInvalid { peer: None });
            return;
        }
        let prior = self
            .revocation_logs
            .get(&author)
            .cloned()
            .unwrap_or_default();
        if let Ok(new) = prior.clone().apply(ev, &author) {
            self.revocation_logs.insert(author, new);
            // B-12 §3.3 / §4.3: archive the signed envelope so this peer
            // can serve a backfill pull later (§14.4). Keyed by author then
            // `revocation_seq` (the contiguous range a behind-peer needs).
            // `ev` is borrowed, so clone into the archive.
            self.revocation_archive
                .entry(author)
                .or_default()
                .insert(ev.revocation_seq, ev.clone());
            self.revocation_events
                .lock()
                .expect("revocation_events mutex poisoned")
                .push(RevocationApplied {
                    author,
                    revoked_bundle_hash: ev.revoked_bundle_hash,
                    revocation_seq: ev.revocation_seq,
                });
        } else {
            // Signature already verified above, so this is a seq /
            // length rejection. Surface as DecodeFailed and restore the
            // unchanged prior log.
            self.peer_warnings
                .lock()
                .expect("peer_warnings mutex poisoned")
                .push(PeerWarning::DecodeFailed { peer: None });
            self.revocation_logs.insert(author, prior);
        }
    }

    /// Verify + apply an inbound [`PublicationEvent`] for `author`.
    ///
    /// Structural twin of [`Self::handle_revocation`] — same
    /// verify-edge-first ordering, same clone-prior / re-insert-on-Err
    /// pattern — over [`PublicationLog`] / [`PublicationAnnounced`]. Kept
    /// as a separate explicit method (rather than a shared generic) for
    /// clarity, per spec §12.4. Per B-11 spec §3.4 / §4.1.
    #[allow(clippy::expect_used)]
    fn handle_publication(&mut self, author: AuthorPubkey, ev: PublicationEvent) {
        if dispatch::verify_publication(&ev, &author).is_err() {
            self.peer_warnings
                .lock()
                .expect("peer_warnings mutex poisoned")
                .push(PeerWarning::SignatureInvalid { peer: None });
            return;
        }
        let prior = self
            .publication_logs
            .get(&author)
            .cloned()
            .unwrap_or_default();
        // `ev` is taken by value: the `Ok` arm moves `ev.version` (a
        // `String`) onto `PublicationAnnounced`. `apply` only borrows it,
        // so the move stays valid in the success branch.
        if let Ok(new) = prior.clone().apply(&ev, &author) {
            self.publication_logs.insert(author, new);
            // B-12 §3.3 / §4.3: archive the latest signed envelope so this
            // peer can serve a backfill pull (§14.4; publication is
            // latest-wins, so one envelope suffices). Clone BEFORE the
            // `PublicationAnnounced` push below moves `ev.version` out of `ev`.
            self.publication_latest.insert(author, ev.clone());
            self.publication_events
                .lock()
                .expect("publication_events mutex poisoned")
                .push(PublicationAnnounced {
                    author,
                    manifest_hash: ev.manifest_hash,
                    version: ev.version,
                    publication_seq: ev.publication_seq,
                });
        } else {
            self.peer_warnings
                .lock()
                .expect("peer_warnings mutex poisoned")
                .push(PeerWarning::DecodeFailed { peer: None });
            self.publication_logs.insert(author, prior);
        }
    }
}

/// Type-erasing [`Network`] wrapper.
///
/// `Runtime` holds an `Arc<dyn Network<Subscription = ...>>` so a single
/// struct definition can host any [`Network`] implementor. Doing that
/// directly is awkward because `Network::Subscription` is an associated
/// type that varies per impl — `dyn Network` would need a concrete
/// associated-type binding. `NetworkErased` resolves that by fixing
/// the wire-facing `Subscription` to a `Box<dyn Subscription + Send>`,
/// boxing whatever concrete subscription the inner impl returns.
///
/// A blanket [`Subscription`] impl on `Box<S>` (in the `myrhiza-network`
/// crate, where the trait is defined) lets the box satisfy
/// [`Subscription::recv`] without further indirection.
struct NetworkErased<N: Network> {
    inner: N,
}

impl<N: Network> NetworkErased<N> {
    fn new(inner: N) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl<N: Network> Network for NetworkErased<N> {
    type Subscription = Box<dyn Subscription + Send>;

    async fn subscribe(
        &self,
        topic: Topic,
        bootstrap: Vec<PeerPubkey>,
    ) -> Result<Self::Subscription, NetError> {
        let s = self.inner.subscribe(topic, bootstrap).await?;
        Ok(Box::new(s))
    }

    async fn publish(&self, topic: Topic, msg: GossipMessage) -> Result<(), NetError> {
        self.inner.publish(topic, msg).await
    }

    async fn unsubscribe(&self, topic: Topic) -> Result<(), NetError> {
        self.inner.unsubscribe(topic).await
    }

    async fn request_heads(
        &self,
        peer: PeerPubkey,
        request: DirectHeadsRequest,
    ) -> Result<HeadsStream, NetError> {
        self.inner.request_heads(peer, request).await
    }

    fn install_request_handler(&self, handler: ArcRequestHandler) {
        self.inner.install_request_handler(handler);
    }

    async fn request_distribution(
        &self,
        peer: PeerPubkey,
        request: DistributionBackfillRequest,
    ) -> Result<DistributionStream, NetError> {
        self.inner.request_distribution(peer, request).await
    }

    fn install_distribution_handler(&self, handler: ArcDistributionHandler) {
        self.inner.install_distribution_handler(handler);
    }
}

// The blanket `impl Subscription for Box<S>` lives in the
// `myrhiza-network` crate (next to the `Subscription` trait itself) to
// satisfy Rust's orphan rule. The erasure pattern in `NetworkErased`
// relies on that impl to forward `recv` through the box.

impl Runtime {
    /// Dispatch an inbound gossip message to the variant-specific
    /// handler. See plan-B-1 spec §11.5 (Event), §7.1 (`HeadsSummary`),
    /// §8.4 (Drift).
    async fn handle_message(&mut self, msg: GossipMessage) -> Result<(), RuntimeError> {
        match msg {
            GossipMessage::Event(e) => self.handle_event(e).await?,
            GossipMessage::HeadsSummary(h) => {
                if self.verify_heads_summary(&h) {
                    self.handle_heads_summary(h).await?;
                }
                // Else: SignatureInvalid was pushed (or loopback) — drop.
            }
            GossipMessage::Drift(d) => self.process_drift_message(d).await,
            // Revocation / Publication envelopes (B-11 §3.1) and the
            // RevocationHeads / PublicationHeads summaries (B-12 §3.2) all
            // ride the per-author distribution topics, NOT the app topic
            // this subscription consumes. Receiving one here means a peer
            // misrouted (or forged) wire traffic onto the app topic; treat
            // it like any other "parsed cleanly but doesn't belong here"
            // case — discard with a DecodeFailed warning, matching the
            // spec §4.1 default arm. The legitimate receive path is the
            // dedicated distribution select arm wired in B-11 T4.
            GossipMessage::Revocation(_)
            | GossipMessage::Publication(_)
            | GossipMessage::RevocationHeads(_)
            | GossipMessage::PublicationHeads(_) => {
                #[allow(clippy::expect_used)]
                self.peer_warnings
                    .lock()
                    .expect("peer_warnings mutex poisoned")
                    .push(PeerWarning::DecodeFailed { peer: None });
            }
        }
        Ok(())
    }

    /// Insert an inbound event into the DAG, drain any pending events
    /// that may now be insertable, then replay + drift-drain + maybe
    /// emit a single drift message for the entire batch.
    ///
    /// §7.2: equivocation is non-fatal — log + continue (both for the
    /// direct-receive insert and any during-drain inserts).
    /// §11.5 / plan-review C-3: at most one drift emission per batch,
    /// using the highest topological index observed.
    #[allow(clippy::too_many_lines)]
    async fn handle_event(&mut self, event: Event) -> Result<(), RuntimeError> {
        match self.dag.insert(event.clone()) {
            Ok(Inserted::NewlyApplied { topo_index, hash }) => {
                let mut last_emit_index = topo_index;
                let mut last_emit_hash = hash;
                // Count inserts performed by the drain loop below; the
                // outer NewlyApplied is the single guaranteed insert.
                // Per plan-B-2.1 spec §3.2: tip-fast-path is sound only
                // for single-insert calls (drain_insert_count == 0).
                // When the drain inserts ≥ 1 events, fall back to
                // replay_full to cover all N+1 inserts in one pass.
                let mut drain_insert_count: usize = 0;
                // Drain pending events that may now be insertable.
                loop {
                    let known = self.dag.known_hashes();
                    let ready = self.pending.newly_satisfied(&known);
                    if ready.is_empty() {
                        break;
                    }
                    for e in ready {
                        let drain_author = e.author;
                        let drain_seq = e.seq;
                        match self.dag.insert(e.clone()) {
                            Ok(Inserted::NewlyApplied {
                                topo_index: ti,
                                hash: h,
                            }) => {
                                last_emit_index = ti;
                                last_emit_hash = h;
                                drain_insert_count += 1;
                            }
                            Err(DagError::Equivocation {
                                author,
                                seq,
                                local_hash,
                                remote_hash,
                            }) => {
                                // TODO(B-2): pending events do not carry
                                // their originating peer identity, so
                                // equivocations surfaced during the
                                // pending-drain path log `peer: None`.
                                // Plan B-2 extends `PendingBuffer`
                                // entries to record the source peer so
                                // this log is fully attributable
                                // (carry-over from review-finding Q-4).
                                #[allow(clippy::expect_used)]
                                self.equivocation_log
                                    .lock()
                                    .expect("equivocation_log mutex poisoned")
                                    .push(EquivocationFlag {
                                        author,
                                        seq,
                                        local_hash,
                                        remote_hash,
                                        peer: None,
                                    });
                            }
                            Ok(Inserted::AlreadyKnown) => {
                                // Event was promoted by a parallel path
                                // (e.g. an earlier iteration of this same
                                // drain loop already inserted it). No-op.
                            }
                            Ok(Inserted::Pending(still_missing)) => {
                                // Spec invariant: `newly_satisfied`
                                // filters on `is_subset(known)`, so the
                                // deps are present at insert time.
                                // Reaching `Pending` here implies a
                                // DAG / pending consistency drift.
                                // Re-buffer with the current
                                // missing-set rather than silently drop
                                // so the event can be retried on the
                                // next round, and surface a warning
                                // (review Q-2; CLAUDE.md: no swallowing
                                // errors).
                                self.pending.insert(e, still_missing);
                                #[allow(clippy::expect_used)]
                                self.peer_warnings
                                    .lock()
                                    .expect("peer_warnings mutex poisoned")
                                    .push(PeerWarning::PendingDrainAnomaly {
                                        author: drain_author,
                                        seq: drain_seq,
                                        reason: "pending-drain produced Pending(_); re-buffered"
                                            .to_string(),
                                    });
                            }
                            Err(err) => {
                                // Non-fatal DagError (sig / genesis /
                                // chain): byzantine input, no
                                // convergence impact, but surface it
                                // rather than discard so the operator
                                // can see what the drain loop
                                // encountered (review Q-2).
                                #[allow(clippy::expect_used)]
                                self.peer_warnings
                                    .lock()
                                    .expect("peer_warnings mutex poisoned")
                                    .push(PeerWarning::PendingDrainAnomaly {
                                        author: drain_author,
                                        seq: drain_seq,
                                        reason: format!("pending-drain insert error: {err:?}"),
                                    });
                            }
                        }
                    }
                }
                // Per plan-B-2.1 spec §3.2: only the single-insert case
                // (drain loop produced no additional NewlyApplied
                // inserts) is eligible for the tip-fast-path. The
                // drain-count gate is the primary correctness check;
                // try_tip_incremental's prefix + tail comparison is
                // defense-in-depth.
                if drain_insert_count == 0 {
                    self.replay_or_incremental(last_emit_hash)?;
                } else {
                    self.replay_full()?;
                }
                self.drain_drift_stash().await;
                // Emit at most one drift per batch — using highest
                // topo_index seen during the drain (plan-review C-3).
                self.maybe_emit_drift(last_emit_index, last_emit_hash).await;
            }
            Ok(Inserted::Pending(missing)) => {
                self.pending.insert(event.clone(), missing);
                self.request_missing_for(&event).await;
            }
            Err(DagError::Equivocation {
                author,
                seq,
                local_hash,
                remote_hash,
            }) => {
                #[allow(clippy::expect_used)]
                self.equivocation_log
                    .lock()
                    .expect("equivocation_log mutex poisoned")
                    .push(EquivocationFlag {
                        author,
                        seq,
                        local_hash,
                        remote_hash,
                        peer: None,
                    });
            }
            // Same-author chain skip — we received an event whose seq is
            // ahead of our known head for that author. The event itself
            // is a byzantine-or-out-of-order signal; either way, the
            // recovery action is the same as the missing-deps path:
            // request the author's chain gap (spec §7.2). Without this,
            // a peer that joins mid-stream and observes only the tail
            // would never request the missing prefix.
            Err(DagError::InvalidChain {
                author,
                expected_seq,
                got_seq,
                ..
            }) if got_seq > expected_seq => {
                self.request_author_chain_gap(author, expected_seq, got_seq.saturating_sub(1))
                    .await;
            }
            // AlreadyKnown: no-op.
            // Other DagErrors (sig / genesis / equivocation w/ same seq,
            // or InvalidChain with seq <= expected which is past-only):
            // drop silently — byzantine or duplicate input, no
            // convergence impact.
            Ok(Inserted::AlreadyKnown) | Err(_) => {}
        }
        Ok(())
    }

    /// Recovery handler for [`Inserted::Pending`] (event well-formed but
    /// missing one or more declared deps).
    ///
    /// Per spec §7.2 (review-finding I-1): publish a targeted
    /// [`HeadsRequest`] for the *event's author* when we are behind on
    /// that author's chain (i.e., the missing deps are likely on the
    /// same author's history). Otherwise — the cross-author case where
    /// the missing deps belong to a different author whose chain we
    /// can't identify from a bare hash set — fall back to a
    /// [`HeadsSummary`] nudge so peers can diff their tips against ours
    /// and push what we're behind on.
    ///
    /// Design A (per `docs/plans/2026-05-11-plan-b-1-fixes.md` Task 4):
    /// no change to [`Inserted::Pending`] enum shape; runtime derives
    /// the author from `event.author`. Design B (carrying explicit
    /// `author_hints` on the Pending variant) is a B-2 ergonomic
    /// improvement.
    async fn request_missing_for(&mut self, event: &Event) {
        let known_head_seq = self
            .dag
            .author_chain(&event.author)
            .map_or(0, |c| c.head_seq);
        if event.seq > known_head_seq + 1 {
            // Same-author gap: ask for the author's chain range.
            self.request_author_chain_gap(
                event.author,
                known_head_seq + 1,
                event.seq.saturating_sub(1),
            )
            .await;
        } else {
            // Cross-author Pending (deps from a different author we
            // can't identify from a bare hash): nudge via HeadsSummary.
            let _ = self.publish_heads_summary().await;
        }
    }

    /// Issue a direct-stream backfill for `author`'s inclusive
    /// `from_seq..=to_seq` range, or soft-nudge when no peer is known.
    ///
    /// Used by [`Self::request_missing_for`] (Pending path) and the
    /// `InvalidChain` arm of [`Self::handle_event`] (same-author
    /// chain-skip path). No-op if the computed range is empty or
    /// inverted.
    async fn request_author_chain_gap(&mut self, author: AuthorPubkey, from_seq: u64, to_seq: u64) {
        if to_seq < from_seq || to_seq == 0 {
            return;
        }
        let Some(target_peer) = self.lookup_peer_for_author(&author) else {
            // Empty index — no peer known to have authority over this author.
            // Soft-nudge: publish our HeadsSummary so peers can diff and
            // either push their HeadsSummaries back (which will populate
            // our index for a future recovery attempt) or, if they're
            // also behind on this author, propagate the gap further.
            // Matches the cross-author Pending recovery in
            // `request_missing_for` (`runtime.rs:1101`).
            // Per B-4.7 spec §3.1.
            let _ = self.publish_heads_summary().await;
            return;
        };
        let mut requests = Vec::new();
        Self::paginate_into(author, from_seq, to_seq, &mut requests);
        if requests.is_empty() {
            return;
        }
        self.issue_direct_backfill(target_peer, requests).await;
    }

    /// Issue a direct-stream backfill request to `target_peer` for the
    /// given `requests`. Spawns a drainer task that forwards response
    /// events into the runtime's `internal_event_rx` mailbox; the
    /// runtime select loop picks them up and processes them via
    /// `handle_event` exactly as if they had arrived through gossip.
    ///
    /// On `NetError::RequestFailed`, pushes a
    /// [`PeerWarning::DirectRequestFailed`] and returns. The next
    /// periodic `HeadsSummary` tick or fresh inbound `HeadsSummary`
    /// triggers a retry.
    ///
    /// Per B-4.5 spec §3.6.
    async fn issue_direct_backfill(
        &mut self,
        target_peer: PeerPubkey,
        requests: Vec<myrhiza_types::EventRequest>,
    ) {
        let direct_req = DirectHeadsRequest {
            topic: self.topic,
            requests,
        };
        let stream = match self.network.request_heads(target_peer, direct_req).await {
            Ok(s) => s,
            Err(e) => {
                #[allow(clippy::expect_used)]
                self.peer_warnings
                    .lock()
                    .expect("peer_warnings mutex poisoned")
                    .push(PeerWarning::DirectRequestFailed {
                        peer: target_peer,
                        reason: format!("{e}"),
                    });
                return;
            }
        };
        // Spawn a drainer task that forwards each Event from the
        // stream into the runtime's internal_event_rx mailbox. The
        // select loop picks them up and calls handle_event.
        let tx = self.internal_event_tx.clone();
        tokio::spawn(drain_heads_response(stream, tx));
    }

    /// Process an inbound `HeadsSummary` from a remote peer. Compares
    /// authors and, per `AuthorDiff` classification, requests backfill,
    /// pushes our events, or flags equivocation.
    ///
    /// Also pushes our events for authors the remote doesn't know
    /// about, and surfaces a `KernelFuelTableMismatch` warning when
    /// fuel-table versions disagree (§11.7, non-fatal).
    async fn handle_heads_summary(&mut self, remote: HeadsSummary) -> Result<(), RuntimeError> {
        self.check_fuel_table_version(&remote);

        // B-4.6: populate the peer-authority index from the signed
        // HeadsSummary. The signer attests to having events for every
        // author in `remote.authors` (else it could not advertise
        // valid tip hashes). Future Pending/InvalidChain recoveries
        // on these authors will target `remote.signed_by_peer` via
        // direct-stream. The verify-side filter at
        // `verify_heads_summary` (line 1836) ensures
        // `remote.signed_by_peer != self.peer_key.public` here, so
        // we never record ourselves. Per B-4.6 spec §3.3.
        for head in &remote.authors {
            self.record_peer_authority(remote.signed_by_peer, head.author);
        }

        let local_map: BTreeMap<AuthorPubkey, (u64, EventHash)> = self
            .dag
            .author_heads()
            .iter()
            .map(|h| (h.author, (h.seq, h.hash)))
            .collect();
        let remote_authors: BTreeSet<AuthorPubkey> =
            remote.authors.iter().map(|h| h.author).collect();

        let mut requests = Vec::new();
        for remote_head in &remote.authors {
            let (local_seq, local_hash) = local_map
                .get(&remote_head.author)
                .copied()
                .unwrap_or((0, EventHash::ZERO));
            let diff = classify_author_diff(local_seq, local_hash, remote_head);
            match diff {
                AuthorDiff::Behind {
                    author,
                    local_seq,
                    remote_seq,
                } => {
                    Self::handle_heads_behind(author, local_seq, remote_seq, &mut requests);
                }
                AuthorDiff::Equal => {}
                AuthorDiff::EqualDivergent {
                    author,
                    seq,
                    local_hash,
                    remote_hash,
                } => {
                    self.handle_heads_equal_divergent(author, seq, local_hash, remote_hash);
                }
                AuthorDiff::Ahead {
                    author,
                    local_seq,
                    remote_seq,
                    remote_hash,
                } => {
                    self.handle_heads_ahead(author, local_seq, remote_seq, remote_hash)
                        .await;
                }
            }
        }

        // ORDERING NOTE: push_authors_remote_lacks must run BEFORE the
        // loopback-early-return + backfill block below. The loopback
        // branch returns early; if it ever moves above this push, our
        // own self-diff would skip pushing authors the remote lacks.
        // Currently safe because the verify-side filter (below) makes
        // the loopback branch unreachable in production.
        self.push_authors_remote_lacks(&local_map, &remote_authors)
            .await;

        if !requests.is_empty() {
            // Loopback guard — defense in depth.
            //
            // In production, this branch is currently unreachable:
            // `handle_heads_summary` is only called by `handle_message`
            // when `verify_heads_summary` returns true, and that
            // function (runtime.rs verify_heads_summary loopback
            // filter) already returns false when
            // `h.signed_by_peer == self.peer_key.public`. So the
            // verify-side filter is the primary gate.
            //
            // The guard remains here as defense in depth because (a)
            // future code paths might call handle_heads_summary
            // directly, bypassing the verify gate; (b) issuing a
            // self-targeted direct-stream `request_heads` would
            // deadlock — the handler runs on this very task, awaiting
            // the request would block forever. The cost is one
            // pubkey comparison + a possible early return. Per B-4.5
            // spec §6 (edge cases — loopback).
            if remote.signed_by_peer == self.peer_key.public {
                return Ok(());
            }
            self.issue_direct_backfill(remote.signed_by_peer, requests)
                .await;
        }
        Ok(())
    }

    /// Fuel-table version warning (non-fatal per §11.7).
    fn check_fuel_table_version(&self, remote: &HeadsSummary) {
        if remote.kernel_fuel_table_version == self.cfg.kernel_fuel_table_version {
            return;
        }
        #[allow(clippy::expect_used)]
        self.peer_warnings
            .lock()
            .expect("peer_warnings mutex poisoned")
            .push(PeerWarning::KernelFuelTableMismatch {
                peer: None,
                remote_version: remote.kernel_fuel_table_version,
                local_version: self.cfg.kernel_fuel_table_version,
            });
    }

    /// Sub-fn: local is behind — paginate a backfill request.
    //
    // Stateless: no `self` access, parallels the sibling
    // `Self::paginate_into` helper (which it delegates to).
    fn handle_heads_behind(
        author: AuthorPubkey,
        local_seq: u64,
        remote_seq: u64,
        requests: &mut Vec<myrhiza_types::EventRequest>,
    ) {
        Self::paginate_into(author, local_seq + 1, remote_seq, requests);
    }

    /// Sub-fn: same seq, divergent hash — equivocation.
    fn handle_heads_equal_divergent(
        &mut self,
        author: AuthorPubkey,
        seq: u64,
        local_hash: EventHash,
        remote_hash: EventHash,
    ) {
        #[allow(clippy::expect_used)]
        self.equivocation_log
            .lock()
            .expect("equivocation_log mutex poisoned")
            .push(EquivocationFlag {
                author,
                seq,
                local_hash,
                remote_hash,
                peer: None,
            });
    }

    /// Sub-fn: local ahead — verify remote-claimed hash at `remote_seq`
    /// matches our history, then push the gap. Mismatch is equivocation,
    /// not a backfill opportunity (spec C-3 fix from plan-review B-5).
    async fn handle_heads_ahead(
        &mut self,
        author: AuthorPubkey,
        local_seq: u64,
        remote_seq: u64,
        remote_hash: EventHash,
    ) {
        let chain = self.dag.author_chain(&author);
        let local_hash_at_remote = chain.and_then(|c| c.seq_to_hash.get(&remote_seq).copied());

        if local_hash_at_remote == Some(remote_hash) {
            // History agrees — push the gap.
            for seq in (remote_seq + 1)..=local_seq {
                if let Some(hash) = chain.and_then(|c| c.seq_to_hash.get(&seq).copied())
                    && let Some(e) = self.dag.get(&hash)
                {
                    let _ = self
                        .network
                        .publish(self.topic, GossipMessage::Event(e.clone()))
                        .await;
                }
            }
        } else if let Some(local_h) = local_hash_at_remote {
            // History disagrees at remote_seq → equivocation.
            #[allow(clippy::expect_used)]
            self.equivocation_log
                .lock()
                .expect("equivocation_log mutex poisoned")
                .push(EquivocationFlag {
                    author,
                    seq: remote_seq,
                    local_hash: local_h,
                    remote_hash,
                    peer: None,
                });
        } else {
            // local_hash_at_remote is None despite local_seq > remote_seq.
            // Under DAG invariants this is unreachable; surface as a
            // PeerWarning rather than silently no-op.
            #[allow(clippy::expect_used)]
            self.peer_warnings
                .lock()
                .expect("peer_warnings mutex poisoned")
                .push(PeerWarning::ChainHashLookupMissing {
                    author,
                    seq: remote_seq,
                });
        }
    }

    /// Push events for authors local has but remote does not.
    async fn push_authors_remote_lacks(
        &mut self,
        local_map: &BTreeMap<AuthorPubkey, (u64, EventHash)>,
        remote_authors: &BTreeSet<AuthorPubkey>,
    ) {
        for (author, (local_seq, _)) in local_map {
            if remote_authors.contains(author) {
                continue;
            }
            let chain = self.dag.author_chain(author);
            for seq in 1..=*local_seq {
                if let Some(hash) = chain.and_then(|c| c.seq_to_hash.get(&seq).copied())
                    && let Some(e) = self.dag.get(&hash)
                {
                    let _ = self
                        .network
                        .publish(self.topic, GossipMessage::Event(e.clone()))
                        .await;
                }
            }
        }
    }

    /// Split a `(from..=to)` range into pages of at most 256 events
    /// (inclusive bounds). Plan-review off-by-one fix: a page covers
    /// `(start, start+255)` — 256 events — and the next page starts at
    /// `start + 256`. Receiver enforces `to_seq - from_seq <= 255`.
    //
    // Stateless: takes no `self`. Free fn would also work, but keeping
    // it as an associated fn parallels the other DAG-message helpers on
    // `Runtime` and lets call sites read as `self.paginate_into(...)`
    // analogues without import churn.
    fn paginate_into(
        author: AuthorPubkey,
        from: u64,
        to: u64,
        out: &mut Vec<myrhiza_types::EventRequest>,
    ) {
        let mut cur = from;
        while cur <= to {
            let page_end = cur.saturating_add(255).min(to);
            out.push(myrhiza_types::EventRequest {
                author,
                from_seq: cur,
                to_seq: page_end,
            });
            if page_end == u64::MAX {
                break;
            }
            cur = page_end + 1;
        }
    }

    /// Record that `peer` is known to have authority over `author`
    /// — we just received a signed `HeadsSummary` from `peer` advertising
    /// it. Move `peer` to the front of the per-author Vec (MRU); on
    /// overflow, drop the tail. Per B-4.6 spec §3.2.
    fn record_peer_authority(&mut self, peer: PeerPubkey, author: AuthorPubkey) {
        let entry = self.peer_authority_index.entry(author).or_default();
        // Remove existing occurrence (if any) so the move-to-front is
        // a single push at the head.
        entry.retain(|p| *p != peer);
        entry.insert(0, peer);
        if entry.len() > PEER_AUTHORITY_PER_AUTHOR_CAP {
            entry.truncate(PEER_AUTHORITY_PER_AUTHOR_CAP);
        }
    }

    /// Look up the most-recently-observed peer with authority over
    /// `author`. Returns `None` if we have never seen a `HeadsSummary`
    /// advertising this author. Per B-4.6 spec §3.2.
    fn lookup_peer_for_author(&self, author: &AuthorPubkey) -> Option<PeerPubkey> {
        self.peer_authority_index
            .get(author)
            .and_then(|peers| peers.first().copied())
    }

    /// Service an inbound direct-stream `HeadsRequest`. Streams events
    /// to `cmd.responder` instead of broadcasting them on the gossip
    /// topic.
    ///
    /// Bound: a single `EventRequest` may cover at most 256 events
    /// (`to_seq - from_seq <= 255`); over-sized requests are silently
    /// dropped.
    ///
    /// If `cmd.responder.send(event).await` returns `Err`, the
    /// requester dropped the stream — stop processing further events.
    ///
    /// Per B-4.5 spec §3.5.
    async fn serve_direct_heads_request(&mut self, cmd: HeadsRequestCommand) {
        // `requester` is captured for future per-peer rate-limit hooks
        // (B-4.6+); currently unused but documented intent.
        let _requester = cmd.requester;
        let responder = cmd.responder;

        for r in cmd.request.requests {
            if r.to_seq < r.from_seq {
                continue;
            }
            if r.to_seq.saturating_sub(r.from_seq) > 255 {
                continue;
            }
            let Some(chain) = self.dag.author_chain(&r.author) else {
                continue;
            };
            // Snapshot the (seq, hash) pairs before any await so we
            // don't hold an immutable borrow of `self.dag` across
            // responder.send.
            let pairs: Vec<(u64, EventHash)> = (r.from_seq..=r.to_seq)
                .filter_map(|seq| chain.seq_to_hash.get(&seq).copied().map(|h| (seq, h)))
                .collect();
            for (_, hash) in pairs {
                if let Some(e) = self.dag.get(&hash).cloned()
                    && responder.send(e).await.is_err()
                {
                    // Requester dropped the stream — stop early.
                    return;
                }
            }
        }
        // Responder drops at end of function -> requester sees clean EOF.
    }

    /// Serve an inbound direct-stream distribution-backfill request from a
    /// behind peer (B-12 spec §14.4, the corrected pull transport).
    ///
    /// The request was already author-gated by [`KernelDistributionHandler`]
    /// (we only reach here for an author this runtime serves). Streams the
    /// missing signed envelopes back through `cmd.responder`:
    ///
    /// - [`DistributionLogKind::Revocation`] → the contiguous range
    ///   `from_seq+1..=max` from [`Self::revocation_archive`]. Revocation
    ///   accumulates (every envelope contributes a distinct revoked bundle
    ///   to the set), so a behind peer needs every envelope in the gap.
    /// - [`DistributionLogKind::Publication`] → the single
    ///   [`Self::publication_latest`] envelope, and only if its
    ///   `publication_seq` exceeds `from_seq` (latest-wins: one envelope
    ///   reconstructs the entire observable state, so the head alone is the
    ///   whole backfill).
    ///
    /// **Borrow discipline (mirrors [`Self::serve_direct_heads_request`]):**
    /// the envelopes are snapshotted (cloned) into an owned `Vec` *before*
    /// the first `responder.send(...).await`, so no immutable borrow of
    /// `self.revocation_archive` / `self.publication_latest` is held across
    /// an await point. A `send` error means the requester dropped the
    /// stream — we stop early. The responder drops at end of function,
    /// yielding clean EOF to the requester.
    async fn serve_distribution_request(&mut self, cmd: DistributionRequestCommand) {
        // `requester` is captured for parity with the heads serve path and
        // a future per-requester rate-limit hook; the author gate already
        // ran in the handler, so the serve path does not branch on it.
        let _requester = cmd.requester;
        let responder = cmd.responder;
        let author = cmd.request.author;
        let from_seq = cmd.request.from_seq;

        // Snapshot the envelopes to serve BEFORE any await so the immutable
        // borrow of the archive ends before `responder.send`.
        let to_send: Vec<DistributionEnvelope> = match cmd.request.kind {
            DistributionLogKind::Revocation => self
                .revocation_archive
                .get(&author)
                .map(|archive| {
                    archive
                        .range(from_seq.saturating_add(1)..)
                        .map(|(_, ev)| DistributionEnvelope::Revocation(ev.clone()))
                        .collect()
                })
                .unwrap_or_default(),
            DistributionLogKind::Publication => self
                .publication_latest
                .get(&author)
                .filter(|ev| ev.publication_seq > from_seq)
                .map(|ev| vec![DistributionEnvelope::Publication(ev.clone())])
                .unwrap_or_default(),
        };

        for envelope in to_send {
            if responder.send(envelope).await.is_err() {
                // Requester dropped the stream — stop early.
                return;
            }
        }
        // Responder drops at end of function -> requester sees clean EOF.
    }

    /// Author + sign + pre-check + self-insert + replay + maybe-emit-drift
    /// + broadcast. See plan-B-1 spec §11.3.
    ///
    /// # Errors
    /// Returns [`RuntimeError::ReadOnly`] when this runtime has no author
    /// keypair, [`RuntimeError::PreCheckRejected`] when the local
    /// pre-check rejects the event before signing, or propagates the
    /// underlying [`ApplyError`] / [`DagError`] / [`NetError`] /
    /// canonical-encoding error.
    async fn author(
        &mut self,
        payload: Vec<u8>,
        deps: BTreeSet<EventHash>,
    ) -> Result<EventHash, RuntimeError> {
        let author_key = self.author_key.as_ref().ok_or(RuntimeError::ReadOnly)?;

        // §11.3: author path is invoked from the run loop's biased select
        // arm, so received messages between authoring sessions have already
        // been processed by the time we reach here — the run loop drains
        // sub.recv() between author commands as part of the select. The
        // author_tx mpsc channel is single-consumer; the Runtime task owns
        // both the chain head and the apply state. No explicit drain needed.

        // Compute next slot from chain.
        let chain = self.dag.author_chain(&author_key.author);
        let (seq, prev) = match chain {
            None => (1u64, EventHash::ZERO),
            Some(c) if c.head_seq == 0 => (1, EventHash::ZERO),
            Some(c) => (c.head_seq + 1, c.head_hash),
        };

        // Advance HLC.
        self.hlc_logical_counter = self.hlc_logical_counter.saturating_add(1);
        let hlc = Hlc {
            wall_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX)),
            logical: self.hlc_logical_counter,
        };

        let body = Event {
            author: author_key.author,
            seq,
            prev,
            deps,
            hlc,
            payload,
            signature: [0; 64],
        };
        let body_hash = body.hash_signed_body();
        let signature = author_key.sign_body_hash(body_hash);
        let event = Event { signature, ..body };
        let envelope = canonical_bincode().serialize(&event)?;

        // Pre-check.
        let pre = self.handle.pre_check(&self.state, &envelope)?;
        if let ApplyOutcome::Rejected(reason) = pre.outcome {
            return Err(RuntimeError::PreCheckRejected(reason));
        }

        // Self-insert + replay + drift drain + maybe emit. The author
        // path is single-insert by construction (one event per call),
        // so the tip-fast-path is always eligible to attempt — gated
        // by [`Self::try_tip_incremental`]'s prefix + tail checks. Per
        // plan-B-2.1 spec §3.2.
        let inserted = self.dag.insert(event.clone())?;
        if let Inserted::NewlyApplied { topo_index, hash } = inserted {
            self.replay_or_incremental(hash)?;
            self.drain_drift_stash().await;
            self.maybe_emit_drift(topo_index, hash).await;
        }

        // Broadcast.
        self.network
            .publish(self.topic, GossipMessage::Event(event.clone()))
            .await?;
        Ok(event.wire_hash())
    }

    /// Drive the installed state-propose component on `intent`, then
    /// author the produced payload through [`Self::author`]. Per B-13
    /// spec §4.1.
    ///
    /// The flow is `propose → deps → author`: the propose component turns
    /// the app-internal `intent` into a candidate payload, and the
    /// existing [`Self::author`] engine signs, pre-checks (state-apply
    /// dry-run), inserts, replays, and broadcasts it. The private key
    /// never reaches propose — the kernel signs on its behalf (spec §2 /
    /// §6). A buggy or malicious propose cannot get an invalid event
    /// applied: `author`'s pre-check still gates (spec §4.4).
    ///
    /// # Errors
    /// - [`RuntimeError::ReadOnly`] when no author keypair is configured.
    ///   Checked *before* running propose (short-circuit — don't spend a
    ///   WASM call we can't act on; spec §4.4).
    /// - [`RuntimeError::NoProposeComponent`] when no propose component is
    ///   installed (`propose: None`).
    /// - [`RuntimeError::ProposeRejected`] when the propose component
    ///   declines the intent (`ProposeError::Rejected`) or its backend
    ///   traps (`ProposeError::Backend`).
    /// - Anything [`Self::author`] can return (e.g.
    ///   [`RuntimeError::PreCheckRejected`]) once the payload is produced.
    async fn propose_and_author(&mut self, intent: Vec<u8>) -> Result<EventHash, RuntimeError> {
        // Short-circuit a read-only runtime before spending a propose
        // call we could not act on (spec §4.4).
        if self.author_key.is_none() {
            return Err(RuntimeError::ReadOnly);
        }
        let propose = self
            .propose
            .as_mut()
            .ok_or(RuntimeError::NoProposeComponent)?;
        let payload = propose.propose(&self.state, &intent)?;

        // deps = current applied frontier. No global DAG frontier accessor
        // exists yet (dag.rs exposes per-author heads via `author_heads`,
        // not a global sink set); the per-author `prev`/`seq` chain computed
        // inside `author()` already orders same-author events for
        // correctness. Cross-author `deps` is a convergence-speed
        // optimization only (spec §4.3).
        // B-13: cross-author deps optimization — frontier accessor TODO.
        let deps = BTreeSet::new();
        self.author(payload, deps).await
    }

    /// Fast-path-then-fallback wrapper for [`Self::replay_full`]. Per
    /// plan-B-2.1 spec §3.2.
    ///
    /// Caller must guarantee that `inserted_hash` corresponds to a
    /// **single-insert** since the last replay. Multi-insert paths
    /// (drain loops, batch arrivals) must call [`Self::replay_full`]
    /// directly — the eligibility check inside
    /// [`Self::try_tip_incremental`] catches accidental misuse via the
    /// `new_order.len() == prior_len + 1` guard, but the caller-side
    /// guarantee is the primary correctness gate (defense-in-depth).
    ///
    /// # Errors
    /// Propagates canonical-encoding errors or [`ApplyError`] from the
    /// underlying state-apply handle.
    fn replay_or_incremental(&mut self, inserted_hash: EventHash) -> Result<(), RuntimeError> {
        if self.try_tip_incremental(inserted_hash)? {
            return Ok(());
        }
        self.replay_full()
    }

    /// Attempt incremental tip-extension apply. Returns `Ok(true)` if
    /// the fast path was taken (state, `digest_watch`, and topo cache
    /// updated); `Ok(false)` to signal the caller should fall back to
    /// [`Self::replay_full`]. Errors propagate.
    ///
    /// Per plan-B-2.1 spec §3.4. Eligibility:
    /// - The new topo order extends `last_topo_order` by exactly one.
    /// - The prefix of the new order matches the cached order.
    /// - The new tail element matches `inserted_hash`.
    ///
    /// On `Accepted`: state, topo cache, and `digest_watch` are updated.
    /// On `Rejected`: the event stays in the DAG (per spec §4.4 /
    /// §14 edge-case 8), `dropped_at_apply` records the reason,
    /// `last_topo_order` is refreshed (DAG-sourced; legitimately
    /// differs from state by the one rejected event), and
    /// `digest_watch` publishes unchanged state to match
    /// `replay_full`'s "always publish post-loop" contract.
    ///
    /// # Errors
    /// Propagates canonical-encoding errors or [`ApplyError`] from the
    /// underlying state-apply handle.
    fn try_tip_incremental(&mut self, inserted_hash: EventHash) -> Result<bool, RuntimeError> {
        let new_order = self.dag.topo_sort();
        if new_order.len() != self.last_topo_order.len() + 1 {
            return Ok(false);
        }
        if new_order[..self.last_topo_order.len()] != self.last_topo_order[..] {
            return Ok(false);
        }
        let Some(&last) = new_order.last() else {
            return Ok(false);
        };
        if last != inserted_hash {
            return Ok(false);
        }

        let Some(event) = self.dag.get(&inserted_hash) else {
            // `Inserted::NewlyApplied` should guarantee presence in
            // the DAG. Belt-and-suspenders fallback.
            return Ok(false);
        };
        let bytes = canonical_bincode().serialize(event)?;
        let result = self.handle.apply(&self.state, &bytes)?;
        {
            #[allow(clippy::expect_used)]
            let mut guard = self
                .tip_fast_path_hits
                .lock()
                .expect("tip_fast_path_hits mutex poisoned");
            *guard += 1;
        }
        match result.outcome {
            ApplyOutcome::Accepted => {
                self.state = result.new_state;
                self.last_topo_order = new_order;
                let _ = self.digest_watch_tx.send(self.state.clone());
                Ok(true)
            }
            ApplyOutcome::Rejected(reason) => {
                // Per spec §4.4 / §14 edge-case 8 + B-2.1 spec §3.4 +
                // §3.5: event stays in DAG; state ignores it. Topo
                // cache reflects DAG; can legitimately differ from
                // state by this one rejected event. The next replay
                // (full or incremental-success) re-aligns them.
                self.last_topo_order = new_order;
                #[allow(clippy::expect_used)]
                self.dropped_at_apply
                    .lock()
                    .expect("dropped_at_apply mutex poisoned")
                    .insert(inserted_hash, reason);
                // Match replay_full's digest_watch semantics: publish
                // even when state is unchanged (B-2.1 spec §3.5).
                let _ = self.digest_watch_tx.send(self.state.clone());
                Ok(true)
            }
        }
    }

    /// Re-run state-apply over the full DAG topological order and
    /// publish the resulting state on the digest watch channel.
    ///
    /// Authoritative full-recompute path. Used as the fallback for
    /// [`Self::replay_or_incremental`] when the tip-fast-path is
    /// ineligible (re-topo, multi-insert drain loops, etc.). Always
    /// refreshes [`Self::last_topo_order`] to keep the tip-fast-path
    /// eligibility cache aligned with `self.state`.
    ///
    /// # Errors
    /// Propagates canonical-encoding errors or [`ApplyError`] from the
    /// underlying state-apply handle.
    fn replay_full(&mut self) -> Result<(), RuntimeError> {
        let order = self.dag.topo_sort();
        let mut state = Vec::new();
        // Drops accumulated this replay. We clear `dropped_at_apply`
        // wholesale at the end rather than during the loop so observers
        // never see a transient empty map mid-replay; the publish is
        // atomic from the consumer's perspective (single mutex op). Per
        // spec §4.4 / §14 edge-case 8 a reject is per-replay, not
        // sticky — future replays with a different topo ordering may
        // accept the same event.
        let mut drops: HashMap<EventHash, String> = HashMap::new();
        for hash in &order {
            if let Some(event) = self.dag.get(hash) {
                let bytes = canonical_bincode().serialize(event)?;
                let r = self.handle.apply(&state, &bytes)?;
                match r.outcome {
                    ApplyOutcome::Accepted => state = r.new_state,
                    ApplyOutcome::Rejected(reason) => {
                        drops.insert(*hash, reason);
                    }
                }
            }
        }
        state.clone_into(&mut self.state);
        self.last_topo_order = order;
        // Atomic publish of the new drops snapshot. Mutex poisoning
        // would mean another task panicked while holding the map —
        // unreachable because the runtime task is the only writer.
        #[allow(clippy::expect_used)]
        {
            let mut guard = self
                .dropped_at_apply
                .lock()
                .expect("dropped_at_apply mutex poisoned");
            *guard = drops;
        }
        let _ = self.digest_watch_tx.send(state);
        Ok(())
    }

    /// Drain any stashed incoming drift messages whose anchor is now
    /// covered by the local DAG state.
    async fn drain_drift_stash(&mut self) {
        let current_seq_map: BTreeMap<AuthorPubkey, u64> = self
            .dag
            .author_seq_vec()
            .into_iter()
            .map(|a| (a.author, a.max_seq))
            .collect();
        let covered_keys: Vec<Vec<AuthorSeq>> = self
            .incoming_drift_pending
            .keys()
            .filter(|asv| {
                let anchor = DriftAnchor {
                    event_hash: EventHash::ZERO,
                    author_seq_vec: (*asv).clone(),
                };
                anchor_covered(&anchor, &current_seq_map)
            })
            .cloned()
            .collect();
        for key in covered_keys {
            if let Some(msgs) = self.incoming_drift_pending.remove(&key) {
                for m in msgs {
                    self.process_drift_message(m).await;
                }
            }
        }
    }

    /// Consider emitting a drift-message at the given topo-index +
    /// anchor event hash. Subject to interval cadence and rate limits
    /// (§11.4, §8.1).
    async fn maybe_emit_drift(&mut self, topo_index: u64, anchor_event_hash: EventHash) {
        if !should_emit(topo_index, self.cfg.drift_interval) {
            return;
        }
        let now = std::time::Instant::now();
        if let Err(kind) = self.rate_limit.try_emit(now) {
            #[allow(clippy::expect_used)]
            self.peer_warnings
                .lock()
                .expect("peer_warnings mutex poisoned")
                .push(PeerWarning::DriftRateLimited { kind });
            return;
        }

        let Ok(state_digest_bytes) = self.handle.state_digest(&self.state) else {
            return;
        };
        let digest: [u8; 32] = blake3::hash(&state_digest_bytes).into();
        let author_seq_vec = self.dag.author_seq_vec();
        let anchor = DriftAnchor {
            event_hash: anchor_event_hash,
            author_seq_vec: author_seq_vec.clone(),
        };
        self.own_digest_cache.insert(author_seq_vec, digest);

        let signed_payload = DriftSignedPayload {
            anchor: anchor.clone(),
            digest,
            digest_format: "bincode-1.3".into(),
        };
        // canonical_bincode of a fixed-schema struct cannot fail; expect for
        // clippy compliance per CLAUDE.md "no panics in non-test code" escape hatch.
        #[allow(clippy::expect_used)]
        let sign_bytes = canonical_bincode()
            .serialize(&signed_payload)
            .expect("canonical bincode of DriftSignedPayload is infallible");
        let signature = self.peer_key.sign(&sign_bytes);
        let msg = DriftMessage {
            anchor,
            digest,
            digest_format: "bincode-1.3".into(),
            signed_by_peer: self.peer_key.public,
            signature,
        };
        let _ = self
            .network
            .publish(self.topic, GossipMessage::Drift(msg))
            .await;
    }

    /// Verify a `HeadsSummary`'s peer signature against its claimed
    /// `signed_by_peer`. Returns `true` if the message should be
    /// processed, `false` if it should be dropped.
    ///
    /// Follows the structural shape of `process_drift_message`
    /// (`runtime.rs:1450-1466`) but extends it: the drift handler
    /// **silently drops** on bad sig; this fn pushes
    /// `PeerWarning::SignatureInvalid { peer }` at the same decision
    /// point, then returns `false` so the body-consuming handler skips.
    /// (Backfilling `PeerWarning::SignatureInvalid` into
    /// `process_drift_message` is a follow-up — see spec §10.)
    ///
    /// Loopback: returns `false` for own-published messages, causing
    /// the dispatch site to skip the body handler. `MemNetwork` echoes
    /// own publishes (broadcast channel); `IrohNetwork` does NOT
    /// (Plumtree). See spec §2 "Loopback filter" row.
    ///
    /// Per B-4.2 spec §3.2.
    fn verify_heads_summary(&self, h: &HeadsSummary) -> bool {
        // Loopback filter — MemNetwork echoes own publishes through its
        // tokio broadcast channel; IrohNetwork does not (Plumtree).
        // Either way, our own HeadsSummary is a self-diff no-op; skip.
        if h.signed_by_peer == self.peer_key.public {
            return false;
        }
        let signed_payload = HeadsSummarySignedPayload {
            authors: h.authors.clone(),
            kernel_fuel_table_version: h.kernel_fuel_table_version,
            topic: self.topic,
        };
        let Ok(bytes) = canonical_bincode().serialize(&signed_payload) else {
            // Encode of a payload we just constructed cannot fail in
            // practice; defensive return matches the drift handler shape.
            return false;
        };
        if myrhiza_manifest::verify_signature(h.signed_by_peer.as_bytes(), &bytes, &h.signature)
            .is_err()
        {
            #[allow(clippy::expect_used)]
            self.peer_warnings
                .lock()
                .expect("peer_warnings mutex poisoned")
                .push(PeerWarning::SignatureInvalid {
                    peer: Some(h.signed_by_peer),
                });
            return false;
        }
        true
    }

    /// Process an incoming drift message per spec §8.4: loopback
    /// filter, signature verification, anchor coverage (stash on miss),
    /// digest lookup / compute, compare + log on mismatch.
    ///
    /// Kept `async` to match the plan-B-1 §8.4 signature and the
    /// `drain_drift_stash` call site (which awaits this fn); the body
    /// happens to be synchronous in B-1 but downstream tasks may need
    /// to await network operations during processing.
    #[allow(clippy::unused_async)]
    async fn process_drift_message(&mut self, d: DriftMessage) {
        // §8.4 step 0: loopback filter.
        if d.signed_by_peer == self.peer_key.public {
            return;
        }
        // Step 1: verify signature.
        let signed_payload = DriftSignedPayload {
            anchor: d.anchor.clone(),
            digest: d.digest,
            digest_format: d.digest_format.clone(),
        };
        let Ok(bytes) = canonical_bincode().serialize(&signed_payload) else {
            return;
        };
        if myrhiza_manifest::verify_signature(d.signed_by_peer.as_bytes(), &bytes, &d.signature)
            .is_err()
        {
            // B-4.8: surface bad-sig drift messages as a PeerWarning,
            // matching the equivalent shape in `verify_heads_summary`
            // (B-4.2 §10 carryover). Note the claimed peer is *claimed*
            // (the sig failed to verify against this key), useful for
            // observability + correlation, not for trust decisions.
            #[allow(clippy::expect_used)]
            self.peer_warnings
                .lock()
                .expect("peer_warnings mutex poisoned")
                .push(PeerWarning::SignatureInvalid {
                    peer: Some(d.signed_by_peer),
                });
            return;
        }
        // Step 2: anchor coverage check.
        let current_seq_map: BTreeMap<AuthorPubkey, u64> = self
            .dag
            .author_seq_vec()
            .into_iter()
            .map(|a| (a.author, a.max_seq))
            .collect();
        if !anchor_covered(&d.anchor, &current_seq_map) {
            // Stash for later.
            if self.incoming_drift_pending.len() >= self.cfg.drift_stash_cap
                && let Some((k, _)) = self.incoming_drift_pending.iter().next()
            {
                let k = k.clone();
                self.incoming_drift_pending.remove(&k);
            }
            self.incoming_drift_pending
                .entry(d.anchor.author_seq_vec.clone())
                .or_default()
                .push(d);
            return;
        }
        // Step 3: lookup own digest at this anchor; compute on cache miss.
        let local_digest =
            if let Some(dg) = self.own_digest_cache.get(&d.anchor.author_seq_vec).copied() {
                dg
            } else {
                let Some(dg) = self.compute_anchor_digest_off_loop(&d.anchor).await else {
                    return;
                };
                self.own_digest_cache
                    .insert(d.anchor.author_seq_vec.clone(), dg);
                dg
            };
        // Step 4: compare.
        if local_digest != d.digest {
            let detected = DriftDetected {
                peer: d.signed_by_peer,
                anchor: d.anchor.clone(),
                local_digest,
                remote_digest: d.digest,
            };
            #[allow(clippy::expect_used)]
            self.drift_log
                .lock()
                .expect("drift_log mutex poisoned")
                .push(detected);
        }
    }

    /// Compute our own state-digest at the given anchor by replaying
    /// the topo-subset of events bounded by `anchor.author_seq_vec`.
    ///
    /// Moves `self.handle` into a `tokio::task::spawn_blocking` worker
    /// for the duration of the subset replay so the runtime select loop
    /// can drive other tokio tasks (`MemBus` publishes, network
    /// subscription forwarding, downstream `digest_watch` consumers).
    /// Per plan-B-2.1 spec §4.2.
    ///
    /// On task panic the handle is permanently lost (left as a
    /// tombstone) and any subsequent `self.handle` use will panic via
    /// the tombstone's `unreachable!()`. v1 recovery path is runtime
    /// halt — see plan-B-2.1 spec §4.2.1.
    async fn compute_anchor_digest_off_loop(&mut self, anchor: &DriftAnchor) -> Option<[u8; 32]> {
        let bound = anchor_bound_map(&anchor.author_seq_vec);
        // Snapshot the subset events before moving the handle. The DAG
        // is a BTreeMap so this is a clone of the relevant slice.
        let subset_hashes = self.dag.topo_sort_subset(|e| {
            bound
                .get(&e.author)
                .copied()
                .is_some_and(|max| e.seq <= max)
        });
        let mut subset_events: Vec<Event> = Vec::with_capacity(subset_hashes.len());
        for h in &subset_hashes {
            let event = self.dag.get(h)?;
            subset_events.push(event.clone());
        }

        // SAFETY (B-2.1 §4.3): swap the real handle for a tombstone
        // for the duration of the `spawn_blocking` call. The tombstone
        // panics on any method invocation; the only code path that
        // could trigger a panic is a concurrent runtime task — but
        // the runtime is single-task by construction (plan-B-1 §11),
        // so no such concurrent invocation exists. The handle is
        // restored from the task's return tuple before any subsequent
        // method on `self` is reached. On task panic the tombstone
        // persists and the next handle use will surface the bug
        // deterministically — v1 recovery is runtime halt (§4.2.1).
        let mut handle = std::mem::replace(&mut self.handle, StateApplyHandle::tombstone());

        let join_result =
            tokio::task::spawn_blocking(move || -> (StateApplyHandle, Option<[u8; 32]>) {
                let digest = compute_subset_digest(&mut handle, &subset_events);
                (handle, digest)
            })
            .await;

        match join_result {
            Ok((handle, digest)) => {
                self.handle = handle;
                digest
            }
            Err(_join_err) => {
                // Task panicked. Handle is gone; tombstone persists.
                // Per B-2.1 spec §4.2.1: any subsequent self.handle
                // use will trigger the tombstone's unreachable!().
                // v1 acceptable; recovery requires runtime halt.
                None
            }
        }
    }
}

/// Per-author classification of remote heads vs local heads, used by
/// `handle_heads_summary` to dispatch to the appropriate sub-function.
#[derive(Debug)]
enum AuthorDiff {
    /// Local is behind remote — request backfill.
    Behind {
        author: AuthorPubkey,
        local_seq: u64,
        remote_seq: u64,
    },
    /// Same seq, same hash — no action.
    Equal,
    /// Same seq, divergent hash — equivocation.
    EqualDivergent {
        author: AuthorPubkey,
        seq: u64,
        local_hash: EventHash,
        remote_hash: EventHash,
    },
    /// Local is ahead — push the gap (or flag equivocation if the
    /// remote-claimed hash at `remote_seq` disagrees with local history).
    Ahead {
        author: AuthorPubkey,
        local_seq: u64,
        remote_seq: u64,
        remote_hash: EventHash,
    },
}

/// Classify a single remote author head against local state. Pure
/// function; lifted out of `handle_heads_summary` to keep that
/// dispatcher small.
fn classify_author_diff(
    local_seq: u64,
    local_hash: EventHash,
    remote_head: &myrhiza_types::AuthorHead,
) -> AuthorDiff {
    use std::cmp::Ordering;
    match local_seq.cmp(&remote_head.seq) {
        Ordering::Less => AuthorDiff::Behind {
            author: remote_head.author,
            local_seq,
            remote_seq: remote_head.seq,
        },
        Ordering::Equal => {
            if local_hash == remote_head.hash {
                AuthorDiff::Equal
            } else {
                AuthorDiff::EqualDivergent {
                    author: remote_head.author,
                    seq: remote_head.seq,
                    local_hash,
                    remote_hash: remote_head.hash,
                }
            }
        }
        Ordering::Greater => AuthorDiff::Ahead {
            author: remote_head.author,
            local_seq,
            remote_seq: remote_head.seq,
            remote_hash: remote_head.hash,
        },
    }
}

/// Pure-compute helper for [`Runtime::compute_anchor_digest_off_loop`].
/// Runs on a `tokio::task::spawn_blocking` worker thread.
///
/// Returns `None` on any state-apply backend error (matching the
/// removed synchronous `compute_anchor_digest`'s `.ok()?` semantics).
/// State-apply rejects (non-error verdict) are silently dropped from
/// the materialized state — the event still contributes to the
/// `prior_state` chain only when accepted. Per plan-B-2.1 spec §4.2.
///
/// Exposed `pub` so B-2.1 acceptance test 7
/// (`anchor_digest_correctness_after_off_loop_move`) can compute the
/// same digest via a fresh-handle inline path and verify byte equality
/// with the off-loop path. Marked `#[doc(hidden)]` because it is an
/// implementation detail of the off-loop drift digest path, not a
/// stable public surface.
#[doc(hidden)]
pub fn compute_subset_digest(handle: &mut StateApplyHandle, subset: &[Event]) -> Option<[u8; 32]> {
    let mut state = Vec::<u8>::new();
    for event in subset {
        let bytes = canonical_bincode().serialize(event).ok()?;
        let r = handle.apply(&state, &bytes).ok()?;
        if let ApplyOutcome::Accepted = r.outcome {
            state = r.new_state;
        }
    }
    let digest_bytes = handle.state_digest(&state).ok()?;
    Some(blake3::hash(&digest_bytes).into())
}

/// Drainer task that consumes a [`HeadsStream`] from a direct-stream
/// backfill response and forwards Events into the runtime's
/// `internal_event_rx` mailbox. The runtime processes them via
/// [`Runtime::handle_event`].
///
/// Errors from the stream
/// ([`HeadsStreamError::Transport`] / `::Decode` / `::Handler`)
/// terminate the drainer silently; the missing events surface as gaps
/// on the next `HeadsSummary` cycle.
///
/// Per B-4.5 spec §3.7.
async fn drain_heads_response(mut stream: HeadsStream, tx: mpsc::Sender<Event>) {
    while let Some(item) = stream.next().await {
        match item {
            Ok(event) => {
                if tx.send(event).await.is_err() {
                    // Runtime task gone; stop draining.
                    return;
                }
            }
            Err(_e) => {
                // Stream-level error — terminate. The next
                // HeadsSummary cycle will surface the same gap and
                // retry.
                return;
            }
        }
    }
}

/// Drain a [`DistributionStream`] pulled from an advertiser, re-injecting
/// each [`DistributionEnvelope`] into the shared distribution channel as a
/// `GossipMessage::Revocation`/`Publication` tagged with `author`, so the
/// runtime applies it through [`Runtime::handle_distribution_message`]
/// exactly as if it had arrived over gossip (verify-edge → apply → archive
/// → surface). The distribution counterpart of [`drain_heads_response`]
/// (B-12 spec §14.4).
///
/// Lifecycle: forwards until the stream ends (`None`, clean EOF), a
/// stream-level error arrives (terminate — the next summary retries the
/// gap), or the channel receiver is dropped (runtime task gone). On exit —
/// by any path — the `(author, kind)` in-flight marker is cleared so a
/// later summary for the same gap can dial again. The
/// [`DistributionEnvelope`] variant is mapped to the matching
/// `GossipMessage` variant; `kind` is carried only to key the in-flight
/// clear (the envelope variant alone determines the dispatch).
#[allow(clippy::expect_used)]
async fn drain_distribution_response(
    mut stream: DistributionStream,
    author: AuthorPubkey,
    kind: DistributionLogKind,
    tx: mpsc::Sender<(AuthorPubkey, GossipMessage)>,
    in_flight: Arc<Mutex<BTreeSet<(AuthorPubkey, DistributionLogKind)>>>,
) {
    while let Some(item) = stream.next().await {
        match item {
            Ok(DistributionEnvelope::Revocation(ev)) => {
                if tx
                    .send((author, GossipMessage::Revocation(ev)))
                    .await
                    .is_err()
                {
                    break; // runtime task gone
                }
            }
            Ok(DistributionEnvelope::Publication(ev)) => {
                if tx
                    .send((author, GossipMessage::Publication(ev)))
                    .await
                    .is_err()
                {
                    break; // runtime task gone
                }
            }
            Err(_e) => {
                // Stream-level error — terminate. The next summary for
                // this gap retries.
                break;
            }
        }
    }
    in_flight
        .lock()
        .expect("distribution_in_flight mutex poisoned")
        .remove(&(author, kind));
}

/// Auto-subscribe each installed author's revocation + publication
/// topics on `network`, spawning a [`drain_distribution_sub`] task per
/// subscription that forwards inbound `(author, GossipMessage)` into a
/// single shared channel. Returns the receive half of that channel for
/// the runtime's sixth select arm.
///
/// Extracted from [`Runtime::start`] (B-11 §3.3 / §4.1) so `start` stays
/// within the line budget; the subscribe-and-spawn loop is the whole of
/// the distribution wiring and reads as one unit here. Bounded at 256
/// like the other runtime mailboxes. An empty `installed_authors` makes
/// this a no-op beyond constructing the (then-idle) channel.
///
/// # Errors
/// Propagates any [`NetError`] from a per-topic [`Network::subscribe`]
/// (wrapped in [`RuntimeError::Network`]).
async fn subscribe_distribution_topics<N: Network>(
    network: &N,
    installed_authors: &[AuthorPubkey],
    bootstrap: &[PeerPubkey],
) -> Result<DistributionChannel, RuntimeError>
where
    N::Subscription: Send + 'static,
{
    let (tx, rx) = mpsc::channel::<(AuthorPubkey, GossipMessage)>(256);
    for &author in installed_authors {
        let rsub = network
            .subscribe(
                Topic::from_bytes(derive_revocation_topic(author)),
                bootstrap.to_vec(),
            )
            .await?;
        let psub = network
            .subscribe(
                Topic::from_bytes(derive_publication_topic(author)),
                bootstrap.to_vec(),
            )
            .await?;
        tokio::spawn(drain_distribution_sub(author, rsub, tx.clone()));
        tokio::spawn(drain_distribution_sub(author, psub, tx.clone()));
    }
    // The `tx` is returned alongside `rx` (rather than dropped after the
    // drainer spawns) so the runtime can retain a clone: a distribution
    // *pull* drainer ([`Runtime::issue_distribution_backfill`]) re-injects
    // pulled envelopes into this same channel as
    // `GossipMessage::Revocation`/`Publication`, reusing
    // [`Runtime::handle_distribution_message`]. Per B-12 spec §14.4.
    Ok(DistributionChannel { tx, rx })
}

/// Both halves of the shared distribution channel returned by
/// [`subscribe_distribution_topics`]. The runtime keeps `rx` for the
/// inbound-gossip select arm and retains `tx` as the re-injection point
/// for pulled-backfill envelopes (spec §14.4).
struct DistributionChannel {
    tx: mpsc::Sender<(AuthorPubkey, GossipMessage)>,
    rx: mpsc::Receiver<(AuthorPubkey, GossipMessage)>,
}

/// Per-subscription drainer for an installed author's revocation OR
/// publication topic. Spawned once per derived topic per installed
/// author by [`Runtime::start`]; forwards each inbound message, tagged
/// with the `author` whose topic it arrived on, into the shared
/// distribution channel polled by the runtime's sixth select arm.
///
/// `author` is carried because the per-author topic is derived from it
/// and the downstream `dispatch::verify_*` / `*Log::apply` calls need
/// it; the drainer cannot recover it from the message alone.
///
/// Lifecycle (spec §4.4): loops until the subscription closes
/// (`Ok(None)`) or the channel receiver is dropped (runtime task gone).
/// A `recv` error ([`SubError::Lagged`] / `DecodeFailed` /
/// `TransportError`) is non-fatal here: the loop continues so a single
/// transient error on the distribution topic does not tear down the
/// drainer. Forwarding these sub-errors to the runtime as structured
/// [`PeerWarning`]s is an explicit deferred follow-up (spec §2) — the
/// drainer runs outside the select loop with no `&mut self` handle to
/// the warnings log, and the workspace carries no logging facade to emit
/// into, so the error is dropped rather than surfaced. The generic
/// `S: Subscription` bound accepts the erased `Box<dyn Subscription +
/// Send>` the runtime's `subscribe` returns (via the blanket
/// `Subscription for Box<S>` impl).
///
/// Per B-11 spec §3.2 / §4.4.
async fn drain_distribution_sub<S: Subscription + Send + 'static>(
    author: AuthorPubkey,
    mut sub: S,
    tx: mpsc::Sender<(AuthorPubkey, GossipMessage)>,
) {
    loop {
        match sub.recv().await {
            Ok(Some(msg)) => {
                if tx.send((author, msg)).await.is_err() {
                    // Runtime task gone; stop draining.
                    break;
                }
            }
            Ok(None) => break, // subscription closed
            // Non-fatal: continue draining (see fn-level doc — sub-error
            // forwarding is deferred per spec §2).
            Err(_e) => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use myrhiza_types::BlobHash;

    /// Build a minimal in-process [`Runtime`] for unit-testing the
    /// distribution handlers' archive population (B-12 §3.3 / §4.3).
    ///
    /// The revocation/publication handlers never touch the state-apply
    /// component (they verify the envelope signature, fold the pure-tier
    /// log, append to the poll-log, and — new in B-12 — populate the
    /// kernel archive), so a [`StateApplyHandle::tombstone`] is a sound,
    /// never-invoked stand-in: it would panic only if a state-apply
    /// method were called, which these code paths never do. Building the
    /// full struct directly (rather than going through
    /// [`Runtime::start`], which spawns the value into a task and only
    /// returns a `RuntimeHandle`) is what lets the test read the private
    /// `revocation_archive` / `publication_latest` fields.
    fn test_runtime() -> Runtime {
        use myrhiza_network::{MemBus, MemNetwork};

        let bus = MemBus::new(64);
        let net = MemNetwork::new(bus, PeerPubkey::from_bytes([0xA1; 32]));
        let erased = NetworkErased::new(net);

        let topic = Topic::from_bytes([0u8; 32]);
        let app_bundle_hash = BundleHash::from_bytes([0u8; 32]);
        let topic_name = "test".to_string();
        let cfg = RuntimeCfg::default();

        let (digest_watch_tx, _digest_watch) = watch::channel(Vec::<u8>::new());
        let (halt_watch_tx, _halt_watch) = watch::channel(None::<String>);
        let (_heads_req_tx, heads_req_rx) = mpsc::channel::<HeadsRequestCommand>(1);
        let (internal_event_tx, internal_event_rx) = mpsc::channel::<Event>(1);
        let (_dist_req_tx, distribution_req_rx) = mpsc::channel::<DistributionRequestCommand>(1);
        let (distribution_tx, distribution_rx) = mpsc::channel::<(AuthorPubkey, GossipMessage)>(1);

        let rate_limit = DriftRateLimit::new(cfg.drift_min_interval, cfg.drift_daily_cap);
        let dag = EventDag::new(topic, app_bundle_hash, topic_name.clone());
        let pending = PendingBuffer::new(cfg.pending_cfg.clone());

        Runtime {
            network: Arc::new(erased),
            topic,
            app_bundle_hash,
            topic_name,
            dag,
            pending,
            handle: StateApplyHandle::tombstone(),
            propose: None,
            state: Vec::new(),
            last_topo_order: Vec::new(),
            peer_key: PeerKeypair::deterministic(1),
            author_key: None,
            cfg,
            rate_limit,
            own_digest_cache: BTreeMap::new(),
            incoming_drift_pending: BTreeMap::new(),
            drift_log: Arc::new(Mutex::new(Vec::new())),
            equivocation_log: Arc::new(Mutex::new(Vec::new())),
            peer_warnings: Arc::new(Mutex::new(Vec::new())),
            dropped_at_apply: Arc::new(Mutex::new(HashMap::new())),
            digest_watch_tx,
            halt_watch_tx,
            hlc_logical_counter: 0,
            tip_fast_path_hits: Arc::new(Mutex::new(0)),
            consecutive_transport_errors: 0,
            heads_req_rx,
            distribution_req_rx,
            internal_event_rx,
            internal_event_tx,
            peer_authority_index: BTreeMap::new(),
            revocation_logs: BTreeMap::new(),
            publication_logs: BTreeMap::new(),
            revocation_archive: BTreeMap::new(),
            publication_latest: BTreeMap::new(),
            distribution_dial_limit: BTreeMap::new(),
            distribution_in_flight: Arc::new(Mutex::new(BTreeSet::new())),
            distribution_tx,
            distribution_rx,
            revocation_events: Arc::new(Mutex::new(Vec::new())),
            publication_events: Arc::new(Mutex::new(Vec::new())),
            installed_authors: Vec::new(),
            last_distribution_sync: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Sign a genuine [`RevocationEvent`] for `author` at `seq`.
    fn signed_revocation(
        sk: &ed25519_dalek::SigningKey,
        revoked: BlobHash,
        seq: u64,
    ) -> RevocationEvent {
        use ed25519_dalek::Signer;
        let mut ev = RevocationEvent {
            revoked_bundle_hash: revoked,
            reason: "compromised".to_string(),
            revoked_at: 0,
            revocation_seq: seq,
            signature: [0u8; 64],
        };
        ev.signature = sk.sign(&ev.signing_target()).to_bytes();
        ev
    }

    /// Sign a genuine [`PublicationEvent`] for `author` at `seq`.
    fn signed_publication(
        sk: &ed25519_dalek::SigningKey,
        manifest: BlobHash,
        version: &str,
        seq: u64,
    ) -> PublicationEvent {
        use ed25519_dalek::Signer;
        let mut ev = PublicationEvent {
            manifest_hash: manifest,
            version: version.to_string(),
            publication_seq: seq,
            signature: [0u8; 64],
        };
        ev.signature = sk.sign(&ev.signing_target()).to_bytes();
        ev
    }

    /// B-12 spec §3.3 / §4.3: a verified+applied revocation is archived
    /// into `revocation_archive[author][revocation_seq]` (the full signed
    /// envelope), so this peer can later serve a backfill pull that the
    /// signature-discarding pure-tier `RevocationLog` cannot reconstruct.
    #[test]
    fn revocation_archived_on_valid_apply() {
        use ed25519_dalek::SigningKey;

        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let author = AuthorPubkey::from_bytes(sk.verifying_key().to_bytes());
        let revoked = BlobHash::from_bytes([0xAA; 32]);
        let ev = signed_revocation(&sk, revoked, 1);

        let mut rt = test_runtime();
        rt.handle_revocation(author, &ev);

        // The pure-tier log advanced (sanity — the archive write is gated
        // on the same apply-Ok).
        assert_eq!(
            rt.revocation_logs.get(&author).unwrap().last_observed_seq,
            1,
        );
        // The full signed envelope is in the archive at its seq.
        let archived = rt
            .revocation_archive
            .get(&author)
            .expect("author archive present after a valid revocation")
            .get(&1)
            .expect("seq-1 envelope archived");
        assert_eq!(*archived, ev, "the exact signed envelope is retained");
    }

    /// B-12 spec §3.3 / §4.3: `publication_latest[author]` holds the
    /// NEWER of two applied publications (latest-wins). Critically, the
    /// envelope is cloned into the archive BEFORE `ev.version` is moved
    /// onto `PublicationAnnounced`, so the archive carries the full
    /// envelope (version included) for backfill.
    #[test]
    fn publication_latest_holds_newer_of_two() {
        use ed25519_dalek::SigningKey;

        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let author = AuthorPubkey::from_bytes(sk.verifying_key().to_bytes());

        let ev1 = signed_publication(&sk, BlobHash::from_bytes([0x11; 32]), "1.0.0", 1);
        let ev2 = signed_publication(&sk, BlobHash::from_bytes([0x22; 32]), "2.0.0", 2);

        let mut rt = test_runtime();
        rt.handle_publication(author, ev1);
        rt.handle_publication(author, ev2.clone());

        let latest = rt
            .publication_latest
            .get(&author)
            .expect("author publication archived");
        assert_eq!(
            *latest, ev2,
            "publication_latest must hold the newer (seq-2) envelope",
        );
        assert_eq!(latest.version, "2.0.0", "version retained for backfill");
        assert_eq!(latest.publication_seq, 2);
    }

    /// B-11 spec §4.2: `RevocationApplied` is the outward surface record
    /// the kernel pushes after a verified+applied revocation event. Pins
    /// the field set + accessibility so the surface stays stable.
    #[test]
    fn revocation_applied_fields_accessible() {
        let author = AuthorPubkey::from_bytes([7u8; 32]);
        let revoked_bundle_hash = BlobHash::from_bytes([9u8; 32]);
        let revocation_seq = 3u64;

        let applied = RevocationApplied {
            author,
            revoked_bundle_hash,
            revocation_seq,
        };

        assert_eq!(applied.author, author);
        assert_eq!(applied.revoked_bundle_hash, revoked_bundle_hash);
        assert_eq!(applied.revocation_seq, revocation_seq);

        // Clone + PartialEq + Debug derives (spec §4.2) are part of the
        // surface contract — the poll-log pattern clones records out.
        assert_eq!(applied.clone(), applied);
        let _ = format!("{applied:?}");
    }

    /// B-11 spec §4.2: `PublicationAnnounced` is the outward surface
    /// record for a verified+applied publication event.
    #[test]
    fn publication_announced_fields_accessible() {
        let author = AuthorPubkey::from_bytes([7u8; 32]);
        let manifest_hash = BlobHash::from_bytes([11u8; 32]);
        let version = "1.2.3".to_string();
        let publication_seq = 5u64;

        let announced = PublicationAnnounced {
            author,
            manifest_hash,
            version: version.clone(),
            publication_seq,
        };

        assert_eq!(announced.author, author);
        assert_eq!(announced.manifest_hash, manifest_hash);
        assert_eq!(announced.version, version);
        assert_eq!(announced.publication_seq, publication_seq);

        assert_eq!(announced.clone(), announced);
        let _ = format!("{announced:?}");
    }

    /// B-11 spec §4.4 (plan T3): a [`drain_distribution_sub`] task
    /// forwards an inbound `GossipMessage::Revocation` published on the
    /// author's derived revocation topic into the shared distribution
    /// channel, tagged with the author. Exercises the drainer + the
    /// per-author topic isolation (`MemBus` routes by exact topic bytes,
    /// spec §12.3) end-to-end without spinning a full `Runtime`.
    #[tokio::test]
    async fn distribution_rx_receives_forwarded_message() {
        use ed25519_dalek::{Signer, SigningKey};
        use myrhiza_network::{MemBus, MemNetwork, Network};

        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let author = AuthorPubkey::from_bytes(sk.verifying_key().to_bytes());

        // Sign a genuine revocation event (seq 1).
        let mut ev = RevocationEvent {
            revoked_bundle_hash: BlobHash::from_bytes([0xAA; 32]),
            reason: "compromised".to_string(),
            revoked_at: 0,
            revocation_seq: 1,
            signature: [0u8; 64],
        };
        ev.signature = sk.sign(&ev.signing_target()).to_bytes();

        let bus = MemBus::new(256);
        let revocation_topic = Topic::from_bytes(derive_revocation_topic(author));

        // Receiver side: subscribe the revocation topic + spawn the
        // drainer the runtime uses.
        let recv_net = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0x01; 32]));
        let sub = recv_net
            .subscribe(revocation_topic, vec![])
            .await
            .expect("subscribe revocation topic");
        let (tx, mut rx) = mpsc::channel::<(AuthorPubkey, GossipMessage)>(8);
        tokio::spawn(drain_distribution_sub(author, sub, tx));

        // Publisher side: a separate MemNetwork on the same bus emits the
        // revocation envelope on the author's revocation topic.
        let pub_net = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0x02; 32]));
        pub_net
            .publish(revocation_topic, GossipMessage::Revocation(ev.clone()))
            .await
            .expect("publish revocation");

        let received = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("drainer forwarded within timeout")
            .expect("channel open");

        assert_eq!(received.0, author, "tagged with the author topic-owner");
        assert!(
            matches!(received.1, GossipMessage::Revocation(got) if got == ev),
            "drainer must forward the exact Revocation envelope",
        );
    }

    /// Drain a [`DistributionStream`] to exhaustion, collecting every
    /// envelope the server streamed before clean EOF (`next` → `None`).
    /// Bounded by a per-poll timeout so a serve-side hang fails the test
    /// rather than hanging the suite.
    async fn drain_stream(mut stream: DistributionStream) -> Vec<DistributionEnvelope> {
        let mut out = Vec::new();
        loop {
            let item = tokio::time::timeout(Duration::from_secs(2), stream.next())
                .await
                .expect("distribution stream stalled (no item within timeout)");
            match item {
                Some(result) => out.push(result.expect("distribution stream errored")),
                None => break, // clean EOF
            }
        }
        out
    }

    /// Install a [`KernelDistributionHandler`] on `server_net` for
    /// `installed_authors`, returning the mailbox receiver the runtime
    /// task would drain. Mirrors the real `Runtime::start` wiring so the
    /// author gate is exercised exactly as in production.
    fn install_kernel_distribution_handler(
        server_net: &myrhiza_network::MemNetwork,
        installed_authors: Vec<AuthorPubkey>,
    ) -> mpsc::Receiver<DistributionRequestCommand> {
        use myrhiza_network::Network;
        let (tx, rx) = mpsc::channel::<DistributionRequestCommand>(32);
        server_net.install_distribution_handler(Arc::new(KernelDistributionHandler {
            tx,
            installed_authors,
        }));
        rx
    }

    /// B-12 spec §14.4: `serve_distribution_request` streams the
    /// **contiguous range** `from_seq+1..=max` of archived revocation
    /// envelopes back to a behind requester. Exercises the real
    /// `KernelDistributionHandler` author gate + the real serve path + the
    /// `MemNetwork` direct-stream round-trip: the requester dials, the
    /// handler forwards a `DistributionRequestCommand`, the runtime serves
    /// it, and the requester reads the exact envelopes off its stream.
    #[tokio::test]
    async fn serve_distribution_request_streams_revocation_range() {
        use ed25519_dalek::SigningKey;
        use myrhiza_network::{MemBus, MemNetwork, Network};

        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let author = AuthorPubkey::from_bytes(sk.verifying_key().to_bytes());

        // Server runtime with seqs 1..=3 archived.
        let mut rt = test_runtime();
        let revs: Vec<RevocationEvent> = [1u64, 2, 3]
            .into_iter()
            .map(|seq| {
                let marker = u8::try_from(seq).expect("test seq fits in u8");
                signed_revocation(&sk, BlobHash::from_bytes([marker; 32]), seq)
            })
            .collect();
        for ev in &revs {
            rt.handle_revocation(author, ev);
        }

        let bus = MemBus::new(64);
        let server_net = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0xB0; 32]));
        let mut req_rx = install_kernel_distribution_handler(&server_net, vec![author]);

        // Requester dials with from_seq=1 → expects seqs 2 and 3.
        let client_net = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0xC0; 32]));
        let stream = client_net
            .request_distribution(
                server_net.peer_pubkey(),
                DistributionBackfillRequest {
                    author,
                    kind: DistributionLogKind::Revocation,
                    from_seq: 1,
                },
            )
            .await
            .expect("request_distribution");

        // Serve the forwarded command on the runtime (the select arm's body).
        let cmd = tokio::time::timeout(Duration::from_secs(2), req_rx.recv())
            .await
            .expect("handler forwarded a command within timeout")
            .expect("mailbox open");
        rt.serve_distribution_request(cmd).await;

        let got = drain_stream(stream).await;
        assert_eq!(
            got.len(),
            2,
            "from_seq=1 over a 1..=3 archive yields seqs 2,3"
        );
        assert_eq!(
            got,
            vec![
                DistributionEnvelope::Revocation(revs[1].clone()),
                DistributionEnvelope::Revocation(revs[2].clone()),
            ],
            "exact signed envelopes, in ascending seq order",
        );
    }

    /// B-12 spec §14.4: the publication serve path is latest-wins — it
    /// streams the single newest envelope, and ONLY when its
    /// `publication_seq` strictly exceeds the requester's `from_seq`. A
    /// requester at-or-ahead of the latest gets a clean empty EOF.
    #[tokio::test]
    async fn serve_distribution_request_publication_latest_wins() {
        use ed25519_dalek::SigningKey;
        use myrhiza_network::{MemBus, MemNetwork, Network};

        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let author = AuthorPubkey::from_bytes(sk.verifying_key().to_bytes());

        let mut rt = test_runtime();
        let pub1 = signed_publication(&sk, BlobHash::from_bytes([0x11; 32]), "1.0.0", 1);
        let pub2 = signed_publication(&sk, BlobHash::from_bytes([0x22; 32]), "2.0.0", 2);
        rt.handle_publication(author, pub1);
        rt.handle_publication(author, pub2.clone());

        let bus = MemBus::new(64);
        let server_net = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0xB1; 32]));
        let mut req_rx = install_kernel_distribution_handler(&server_net, vec![author]);
        let client_net = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0xC1; 32]));

        // Behind requester (from_seq=0) → gets the single latest (seq 2).
        let stream = client_net
            .request_distribution(
                server_net.peer_pubkey(),
                DistributionBackfillRequest {
                    author,
                    kind: DistributionLogKind::Publication,
                    from_seq: 0,
                },
            )
            .await
            .expect("request_distribution");
        let cmd = req_rx.recv().await.expect("command forwarded");
        rt.serve_distribution_request(cmd).await;
        let got = drain_stream(stream).await;
        assert_eq!(
            got,
            vec![DistributionEnvelope::Publication(pub2.clone())],
            "latest-wins: behind requester gets the single newest envelope",
        );

        // At-the-head requester (from_seq=2) → empty stream (seq 2 !> 2).
        let stream = client_net
            .request_distribution(
                server_net.peer_pubkey(),
                DistributionBackfillRequest {
                    author,
                    kind: DistributionLogKind::Publication,
                    from_seq: 2,
                },
            )
            .await
            .expect("request_distribution");
        let cmd = req_rx.recv().await.expect("command forwarded");
        rt.serve_distribution_request(cmd).await;
        let got = drain_stream(stream).await;
        assert!(
            got.is_empty(),
            "at-or-ahead requester gets a clean empty EOF (seq must strictly exceed from_seq)",
        );
    }

    /// B-12 spec §14.4: the `KernelDistributionHandler` author gate drops a
    /// request for an author this peer does not serve — the requester sees
    /// a clean empty EOF and the runtime mailbox receives nothing (the
    /// serve path is never reached for an un-served author).
    #[tokio::test]
    async fn distribution_handler_drops_request_for_unserved_author() {
        use myrhiza_network::{MemBus, MemNetwork, Network};

        let served = AuthorPubkey::from_bytes([0x01; 32]);
        let unserved = AuthorPubkey::from_bytes([0x02; 32]);

        let bus = MemBus::new(64);
        let server_net = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0xB2; 32]));
        let mut req_rx = install_kernel_distribution_handler(&server_net, vec![served]);
        let client_net = MemNetwork::new(bus.clone(), PeerPubkey::from_bytes([0xC2; 32]));

        let stream = client_net
            .request_distribution(
                server_net.peer_pubkey(),
                DistributionBackfillRequest {
                    author: unserved,
                    kind: DistributionLogKind::Revocation,
                    from_seq: 0,
                },
            )
            .await
            .expect("request_distribution");

        // Clean empty EOF on the requester side.
        let got = drain_stream(stream).await;
        assert!(got.is_empty(), "un-served author yields an empty stream");

        // And the runtime mailbox saw no command — the gate dropped it.
        assert!(
            req_rx.try_recv().is_err(),
            "author gate must not forward a request for an un-served author",
        );
    }
}
