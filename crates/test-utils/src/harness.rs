//! In-process multi-peer harness for B-1 cross-peer convergence tests.
//!
//! Provides [`InProcessHarness`] — a fixture that constructs a shared
//! [`MemBus`] + [`Topic`] and spawns one or more peers — and
//! [`PeerHandle`] — a thin wrapper around the [`RuntimeHandle`] returned
//! by [`Runtime::start`] that exposes the operations convergence tests
//! need: authoring events, sampling the current state digest, and
//! awaiting digest convergence.
//!
//! ## Determinism
//!
//! Both peer + author keypairs are derived deterministically from `u64`
//! seeds via [`PeerKeypair::deterministic`] / [`AuthorKeypair::deterministic`],
//! and the harness owns the topic-derivation seed, so two test runs with
//! identical inputs produce byte-identical events.
//!
//! ## `await_digest` semantics
//!
//! [`PeerHandle::await_digest`] uses the round-1 plan-review M-5 fix: it
//! calls [`tokio::sync::watch::Receiver::mark_unchanged`] on entry so
//! that [`tokio::sync::watch::Receiver::changed`] only resolves on a
//! *new* value (not the snapshot already present at call time), and
//! uses a saturating deadline so callers can pass any [`Duration`]
//! without underflow risk.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use myrhiza_kernel::drift::DriftDetected;
use myrhiza_kernel::identity::{AuthorKeypair, PeerKeypair};
use myrhiza_kernel::runtime::{
    AuthorCommand, EquivocationFlag, PeerWarning, Runtime, RuntimeCfg, RuntimeHandle,
};
use myrhiza_kernel::state_apply::StateApplyHandle;
use myrhiza_network::{MemBus, MemNetwork};
use myrhiza_types::{BundleHash, EventHash, Topic};
use tokio::sync::oneshot;

/// Test-side handle to a single peer inside an [`InProcessHarness`].
///
/// Wraps the [`RuntimeHandle`] returned by [`Runtime::start`] and exposes
/// only the operations convergence tests need: authoring events,
/// sampling state digest, awaiting digest convergence, and reading the
/// per-peer drift / equivocation / warning logs.
pub struct PeerHandle {
    runtime: RuntimeHandle,
}

impl PeerHandle {
    /// Author a new event with the given payload + explicit dependency set.
    ///
    /// Forwards to [`AuthorCommand::Author`] on the runtime channel and
    /// awaits the reply. Returns the resulting [`EventHash`] on success,
    /// or a stringified error on either channel-send / channel-recv
    /// failure or runtime-side authoring failure.
    ///
    /// # Errors
    /// Returns a stringified error if the author-command channel is
    /// closed, the reply channel is dropped, or the runtime returns an
    /// authoring error (e.g., missing keypair, manifest mismatch).
    pub async fn author(
        &self,
        payload: Vec<u8>,
        deps: BTreeSet<EventHash>,
    ) -> Result<EventHash, String> {
        let (tx, rx) = oneshot::channel();
        self.runtime
            .author_tx
            .send(AuthorCommand::Author {
                payload,
                deps,
                reply: tx,
            })
            .await
            .map_err(|e| format!("author send: {e}"))?;
        rx.await
            .map_err(|e| format!("author recv: {e}"))?
            .map_err(|e| format!("author err: {e}"))
    }

    /// Snapshot the peer's current state digest without blocking.
    #[must_use]
    pub fn current_digest(&self) -> Vec<u8> {
        self.runtime.digest_watch.borrow().clone()
    }

    /// Block until this peer's `digest_watch` reports `expected`, or
    /// `timeout` elapses.
    ///
    /// Implements the round-1 plan-review M-5 fix:
    ///
    /// 1. [`mark_unchanged`](tokio::sync::watch::Receiver::mark_unchanged)
    ///    is called on entry so that [`changed`](tokio::sync::watch::Receiver::changed)
    ///    only resolves on a genuinely *new* value — without this, the
    ///    receiver always sees the snapshot present at construction as
    ///    "changed" and the loop busy-waits.
    /// 2. A saturating deadline (`Instant::now() + timeout` plus
    ///    `Instant::saturating_duration_since`) guards against overflow
    ///    on extreme `Duration` inputs.
    ///
    /// Returns `true` if the expected digest was observed before the
    /// deadline (including the final check after the sender drops),
    /// `false` on timeout.
    pub async fn await_digest(&mut self, expected: Vec<u8>, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        // Mark current snapshot as seen so changed() only resolves on
        // genuinely new values (avoids busy-wait when value hasn't changed).
        self.runtime.digest_watch.mark_unchanged();
        loop {
            if *self.runtime.digest_watch.borrow() == expected {
                return true;
            }
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return false;
            }
            let r = tokio::time::timeout(
                remaining.min(Duration::from_millis(50)),
                self.runtime.digest_watch.changed(),
            )
            .await;
            // r is Ok(Ok(())) on change, Ok(Err(_)) on sender dropped, Err(_) on timeout.
            if let Ok(Err(_)) = r {
                // Sender dropped (Runtime halted); final check then exit.
                return *self.runtime.digest_watch.borrow() == expected;
            }
        }
    }

    /// Snapshot the peer's drift-event log.
    ///
    /// # Panics
    /// Panics if the underlying `drift_log` mutex is poisoned — i.e., if
    /// the runtime task panicked while holding the lock. In a healthy
    /// test run this is structurally unreachable.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn drift_log(&self) -> Vec<DriftDetected> {
        self.runtime
            .drift_log
            .lock()
            .expect("drift_log mutex")
            .clone()
    }

    /// Snapshot the peer's equivocation-flag log.
    ///
    /// # Panics
    /// Panics if the underlying `equivocation_log` mutex is poisoned —
    /// i.e., if the runtime task panicked while holding the lock. In a
    /// healthy test run this is structurally unreachable.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn equivocation_log(&self) -> Vec<EquivocationFlag> {
        self.runtime
            .equivocation_log
            .lock()
            .expect("equiv mutex")
            .clone()
    }

    /// Snapshot the peer's non-fatal warning log.
    ///
    /// # Panics
    /// Panics if the underlying `peer_warnings` mutex is poisoned —
    /// i.e., if the runtime task panicked while holding the lock. In a
    /// healthy test run this is structurally unreachable.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn peer_warnings(&self) -> Vec<PeerWarning> {
        self.runtime
            .peer_warnings
            .lock()
            .expect("warnings mutex")
            .clone()
    }
}

/// Fixture binding a shared [`MemBus`] + topic for in-process,
/// multi-peer convergence tests.
///
/// One `InProcessHarness` corresponds to one topic: every peer spawned
/// via [`InProcessHarness::spawn_peer`] shares the same `bus`,
/// `app_bundle_hash`, `topic_name`, `seed`, and derived [`Topic`] —
/// which is exactly the scope of the convergence-property under test.
pub struct InProcessHarness {
    /// Shared in-memory broadcast bus all peers subscribe to.
    pub bus: Arc<MemBus>,
    /// Application bundle hash bound to this topic (genesis-derived).
    pub app_bundle_hash: BundleHash,
    /// Human-readable topic name (re-used when deriving genesis).
    pub topic_name: String,
    /// 32-byte topic-derivation seed.
    pub seed: [u8; 32],
    /// Pre-computed topic id (derived from `app_bundle_hash`, `seed`, `topic_name`).
    pub topic: Topic,
}

impl InProcessHarness {
    /// Construct a fresh harness with the given bus capacity + topic seed.
    ///
    /// `bus_capacity` is forwarded to [`MemBus::new`] (per-subscription
    /// channel size). `seed` is the topic-derivation seed; the bundle
    /// hash is fixed to `[0xAB; 32]` and the topic name to `"main"` so
    /// every test run uses the same `(bundle, seed, name)` triple unless
    /// the test overrides those fields after construction.
    #[must_use]
    pub fn new(bus_capacity: usize, seed: [u8; 32]) -> Self {
        let bus = MemBus::new(bus_capacity);
        let app_bundle_hash = BundleHash::from_bytes([0xAB; 32]);
        let topic_name = "main".to_string();
        let topic = Topic::derive(&app_bundle_hash, &seed, &topic_name);
        Self {
            bus,
            app_bundle_hash,
            topic_name,
            seed,
            topic,
        }
    }

    /// Spawn a peer with the given keypair seeds + state-apply handle.
    /// `author_seed = None` makes the peer read-only.
    ///
    /// # Panics
    /// Panics if [`Runtime::start`] fails — which only occurs if the
    /// initial topic subscription fails on the in-memory network. The
    /// [`MemNetwork`] subscription path is infallible in practice, so
    /// this is structurally unreachable for in-process tests.
    #[allow(clippy::expect_used)]
    pub async fn spawn_peer(
        &self,
        peer_seed: u64,
        author_seed: Option<u64>,
        handle: StateApplyHandle,
        cfg: RuntimeCfg,
    ) -> PeerHandle {
        let net = MemNetwork::new(self.bus.clone());
        let peer_key = PeerKeypair::deterministic(peer_seed);
        let author_key = author_seed.map(AuthorKeypair::deterministic);
        let runtime = Runtime::start(
            net,
            self.topic,
            self.app_bundle_hash,
            self.topic_name.clone(),
            handle,
            peer_key,
            author_key,
            cfg,
        )
        .await
        .expect("Runtime::start");
        PeerHandle { runtime }
    }
}
