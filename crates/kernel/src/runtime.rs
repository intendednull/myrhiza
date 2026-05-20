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
use std::time::Duration;

use bincode::Options;
use myrhiza_network::{GossipMessage, NetError, Network, SubError, Subscription};
use myrhiza_types::{
    AuthorPubkey, AuthorSeq, BundleHash, DriftAnchor, DriftMessage, DriftSignedPayload, Event,
    EventHash, HeadsRequest, HeadsRequestSignedPayload, HeadsSummary, HeadsSummarySignedPayload,
    Hlc, PeerPubkey, Topic, canonical_bincode,
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
}

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
        cfg: RuntimeCfg,
    ) -> Result<RuntimeHandle, RuntimeError> {
        let erased = NetworkErased::new(network);
        // B-4.* will plumb peer-discovery into Runtime::start; for
        // now pass an empty bootstrap. MemNetwork ignores it;
        // IrohNetwork (B-4.1) accepts it and waits for inbound joins.
        let sub = erased.subscribe(topic, vec![]).await?;

        let (author_tx, author_rx) = mpsc::channel(64);
        let drift_log = Arc::new(Mutex::new(Vec::new()));
        let equivocation_log = Arc::new(Mutex::new(Vec::new()));
        let peer_warnings = Arc::new(Mutex::new(Vec::new()));
        let dropped_at_apply = Arc::new(Mutex::new(HashMap::new()));
        let tip_fast_path_hits = Arc::new(Mutex::new(0_usize));
        let (digest_watch_tx, digest_watch) = watch::channel(Vec::<u8>::new());
        let (halt_watch_tx, halt_watch) = watch::channel(None::<String>);

        let rate_limit = DriftRateLimit::new(
            std::time::Instant::now(),
            cfg.drift_min_interval,
            cfg.drift_daily_cap,
        );
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
        self.publish_heads_summary().await?;
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
                    Some(AuthorCommand::Shutdown) | None => return Ok(()),
                },
                recv_result = sub.recv() => match recv_result {
                    Ok(Some(m)) => { let _ = self.handle_message(m).await; }
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
                },
                _ = ticker.tick() => { self.publish_heads_summary().await?; }
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

    /// Build a signed [`HeadsRequest`] from a list of [`EventRequest`]s.
    ///
    /// Constructs [`HeadsRequestSignedPayload`] with `topic` bound in the
    /// signed payload (NOT on the wire — prevents cross-topic replay per
    /// B-4.2 spec §3.0 + §3.1). Pattern mirrors `publish_heads_summary`
    /// and `maybe_emit_drift` (`runtime.rs:1414-1432`).
    fn build_signed_heads_request(
        &self,
        requests: Vec<myrhiza_types::EventRequest>,
    ) -> Result<HeadsRequest, RuntimeError> {
        let signed_payload = HeadsRequestSignedPayload {
            requests: requests.clone(),
            topic: self.topic,
        };
        let signed_bytes = canonical_bincode()
            .serialize(&signed_payload)
            .map_err(|e| RuntimeError::Canonical(format!("HeadsRequestSignedPayload: {e}")))?;
        let signature = self.peer_key.sign(&signed_bytes);
        Ok(HeadsRequest {
            requests,
            signed_by_peer: self.peer_key.public,
            signature,
        })
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
}

// The blanket `impl Subscription for Box<S>` lives in the
// `myrhiza-network` crate (next to the `Subscription` trait itself) to
// satisfy Rust's orphan rule. The erasure pattern in `NetworkErased`
// relies on that impl to forward `recv` through the box.

impl Runtime {
    /// Dispatch an inbound gossip message to the variant-specific
    /// handler. See plan-B-1 spec §11.5 (Event), §7.1
    /// (`HeadsSummary` + `HeadsRequest`), §8.4 (Drift).
    async fn handle_message(&mut self, msg: GossipMessage) -> Result<(), RuntimeError> {
        match msg {
            GossipMessage::Event(e) => self.handle_event(e).await?,
            GossipMessage::HeadsSummary(h) => self.handle_heads_summary(h).await?,
            GossipMessage::HeadsRequest(r) => self.handle_heads_request(r).await?,
            GossipMessage::Drift(d) => self.process_drift_message(d).await,
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

    /// Publish a [`HeadsRequest`] asking `author` for the inclusive
    /// `from_seq..=to_seq` range of their chain.
    ///
    /// Used by [`Self::request_missing_for`] (Pending path) and the
    /// `InvalidChain` arm of [`Self::handle_event`] (same-author
    /// chain-skip path). No-op if the computed range is empty or
    /// inverted.
    async fn request_author_chain_gap(&mut self, author: AuthorPubkey, from_seq: u64, to_seq: u64) {
        if to_seq < from_seq || to_seq == 0 {
            return;
        }
        let mut requests = Vec::new();
        Self::paginate_into(author, from_seq, to_seq, &mut requests);
        if requests.is_empty() {
            return;
        }
        let Ok(req) = self.build_signed_heads_request(requests) else {
            return;
        };
        let _ = self
            .network
            .publish(self.topic, GossipMessage::HeadsRequest(req))
            .await;
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

        self.push_authors_remote_lacks(&local_map, &remote_authors)
            .await;

        if !requests.is_empty() {
            let req = self.build_signed_heads_request(requests)?;
            let _ = self
                .network
                .publish(self.topic, GossipMessage::HeadsRequest(req))
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

    /// Service inbound range requests by publishing the requested
    /// events back to the topic.
    ///
    /// Bound: a single request may cover at most 256 events
    /// (`to_seq - from_seq <= 255`); over-sized requests are silently
    /// dropped (plan-review off-by-one fix).
    async fn handle_heads_request(&mut self, req: HeadsRequest) -> Result<(), RuntimeError> {
        for r in req.requests {
            if r.to_seq < r.from_seq {
                continue;
            }
            if r.to_seq.saturating_sub(r.from_seq) > 255 {
                continue;
            }
            let Some(chain) = self.dag.author_chain(&r.author) else {
                continue;
            };
            // Snapshot the (seq, hash) pairs before any await so we don't
            // hold an immutable borrow of `self.dag` across .await.
            let pairs: Vec<(u64, EventHash)> = (r.from_seq..=r.to_seq)
                .filter_map(|seq| chain.seq_to_hash.get(&seq).copied().map(|h| (seq, h)))
                .collect();
            for (_, hash) in pairs {
                if let Some(e) = self.dag.get(&hash).cloned() {
                    let _ = self
                        .network
                        .publish(self.topic, GossipMessage::Event(e))
                        .await;
                }
            }
        }
        Ok(())
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
