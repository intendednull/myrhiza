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

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use myrhiza_network::{GossipMessage, NetError, Network, SubError, Subscription};
use myrhiza_types::{AuthorPubkey, BundleHash, EventHash, HeadsSummary, PeerPubkey, Topic};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, watch};

use crate::dag::{DagError, EventDag};
use crate::drift::{DriftDetected, DriftRateLimit, RateLimitKind};
use crate::identity::{AuthorKeypair, PeerKeypair};
use crate::pending::{PendingBuffer, PendingCfg};
use crate::state_apply::{ApplyError, StateApplyHandle};

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
//
// Fields below are populated here so the struct layout is stable across
// the Task 17 / 18 / 19 commit boundary. The handle / state / cache
// fields are read only by `Runtime::author` (Task 18) and
// `Runtime::handle_message` (Task 19); the stubs in this commit leave
// them inert.
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
    /// Watch-side of the digest stream; published on every replay.
    digest_watch_tx: watch::Sender<Vec<u8>>,
    /// Watch-side of the halt signal; populated on fatal runtime error.
    halt_watch_tx: watch::Sender<Option<String>>,
    /// HLC logical-component counter (Tasks 18-19 use; declared here so
    /// the field set is stable across the scaffold commit).
    hlc_logical_counter: u32,
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
        let sub = erased.subscribe(topic).await?;

        let (author_tx, author_rx) = mpsc::channel(64);
        let drift_log = Arc::new(Mutex::new(Vec::new()));
        let equivocation_log = Arc::new(Mutex::new(Vec::new()));
        let peer_warnings = Arc::new(Mutex::new(Vec::new()));
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
            state: Vec::new(),
            peer_key,
            author_key,
            cfg,
            rate_limit,
            own_digest_cache: BTreeMap::new(),
            incoming_drift_pending: BTreeMap::new(),
            drift_log: drift_log.clone(),
            equivocation_log: equivocation_log.clone(),
            peer_warnings: peer_warnings.clone(),
            digest_watch_tx,
            halt_watch_tx,
            hlc_logical_counter: 0,
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
            digest_watch,
            halt_watch,
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
        let summary = HeadsSummary {
            authors: self.dag.author_heads(),
            kernel_fuel_table_version: self.cfg.kernel_fuel_table_version,
        };
        self.network
            .publish(self.topic, GossipMessage::HeadsSummary(summary))
            .await?;
        Ok(())
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

    async fn subscribe(&self, topic: Topic) -> Result<Self::Subscription, NetError> {
        let s = self.inner.subscribe(topic).await?;
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
    // Tasks 18-19 replace these stubs with the real implementations.
    // Keeping them here as no-ops lets the Task 17 scaffold compile
    // without the author / handle-message bodies being in scope yet.
    // The `async` is load-bearing on the real impls (network publish,
    // state-apply call into a Send future) so we keep the signature
    // stable across the Task 17 / 18 / 19 commit boundary.

    /// Stub: drops the inbound message. Replaced in Task 19.
    #[allow(clippy::unused_async)]
    async fn handle_message(&mut self, _msg: GossipMessage) -> Result<(), RuntimeError> {
        Ok(())
    }

    /// Stub: rejects every author command with [`RuntimeError::ReadOnly`].
    /// Replaced in Task 18.
    #[allow(clippy::unused_async)]
    async fn author(
        &mut self,
        _payload: Vec<u8>,
        _deps: BTreeSet<EventHash>,
    ) -> Result<EventHash, RuntimeError> {
        Err(RuntimeError::ReadOnly)
    }
}
