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
//! [`PeerHandle::await_digest`] combines the round-1 plan-review M-5 fix
//! with the round-2 review-Q-3 fix:
//!
//! - If the watch already has an UNOBSERVED update pending at call time
//!   ([`tokio::sync::watch::Receiver::has_changed`] is true), that
//!   update is consumed via
//!   [`tokio::sync::watch::Receiver::borrow_and_update`] and compared
//!   to `expected` — a match returns `true` immediately, because the
//!   pending update IS the evidence of delivery.
//! - Otherwise [`tokio::sync::watch::Receiver::mark_unchanged`] marks
//!   the current snapshot as seen, and the function then loops on
//!   [`tokio::sync::watch::Receiver::changed`], comparing against
//!   `expected` only after each fresh signal resolves. This rules out
//!   the Q-3 "vacuous pass" where state coincidentally equals
//!   `expected` (e.g. both at the construction-default empty
//!   `Vec<u8>`) without any real delivery having occurred.
//!
//! A saturating deadline lets callers pass any [`Duration`] without
//! underflow risk.

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
    /// Implements the round-1 plan-review M-5 fix combined with the
    /// round-2 review-Q-3 fix:
    ///
    /// 1. [`mark_unchanged`](tokio::sync::watch::Receiver::mark_unchanged)
    ///    is called on entry only when no real update is pending — so
    ///    that [`changed`](tokio::sync::watch::Receiver::changed) only
    ///    resolves on a genuinely *new* value rather than busy-waiting
    ///    on the construction snapshot.
    /// 2. The pre-wait equality check is gated on
    ///    [`has_changed`](tokio::sync::watch::Receiver::has_changed)
    ///    being `true` (review Q-3). A bare pre-wait `*borrow() ==
    ///    expected` admitted vacuous passes: any test where the watch
    ///    happened to already hold `expected` at call time — including
    ///    the construction-default value, e.g. an empty `Vec<u8>` — got
    ///    `true` back without any delivery having actually occurred.
    ///    By requiring an unobserved update to be present, we
    ///    distinguish "state coincidentally matches construction
    ///    default" (no update ever fired → block) from "state was
    ///    updated to `expected` just before the call" (an update is
    ///    pending → return true, that update IS the evidence of
    ///    delivery).
    /// 3. A saturating deadline (`Instant::now() + timeout` plus
    ///    `Instant::saturating_duration_since`) guards against overflow
    ///    on extreme `Duration` inputs.
    ///
    /// Returns `true` either when an already-pending update equals
    /// `expected` (consumed via
    /// [`borrow_and_update`](tokio::sync::watch::Receiver::borrow_and_update))
    /// or when a subsequent `changed()` notification arrives whose
    /// post-mutation value equals `expected`. Returns `false` on
    /// timeout. Sender-drop is treated as a final-equality check.
    pub async fn await_digest(&mut self, expected: Vec<u8>, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        // Pre-wait check: only honored if the watch has an UNOBSERVED
        // update pending. `has_changed()` returns `Err` only if the
        // sender is dropped — treat that as "no pending change" and
        // fall through to the loop, which will observe sender-drop on
        // the next changed().await.
        if self.runtime.digest_watch.has_changed().unwrap_or(false) {
            // borrow_and_update both reads the current value and marks
            // it as observed; combined with the has_changed guard
            // above, this returns true only when a real update brought
            // the digest to `expected` (review Q-3).
            if *self.runtime.digest_watch.borrow_and_update() == expected {
                return true;
            }
            // Otherwise the pending update did not match; the next
            // loop iteration waits for the next changed() signal.
        } else {
            // No pending change — mark the current snapshot as seen so
            // changed() only resolves on a genuinely new value (avoids
            // the busy-wait that would occur if a never-updated
            // receiver returned immediately from changed()).
            self.runtime.digest_watch.mark_unchanged();
        }
        loop {
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
            match r {
                Ok(Ok(())) => {
                    // Fresh signal observed — now the equality check is meaningful.
                    if *self.runtime.digest_watch.borrow() == expected {
                        return true;
                    }
                    // Different value — keep waiting for the next change.
                }
                Ok(Err(_)) => {
                    // Sender dropped (Runtime halted). One final
                    // equality check against the last-published value;
                    // safe to consult `borrow()` here because no
                    // further change can race in.
                    return *self.runtime.digest_watch.borrow() == expected;
                }
                Err(_) => {
                    // Per-iteration poll timeout — fall through to the
                    // top of the loop, where the deadline is re-checked.
                }
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

    /// Snapshot the peer's map of events rejected by `state-apply`,
    /// keyed by event `wire_hash` and valued with the reject reason.
    ///
    /// Per plan-B-2.1 spec §3.4 / §5 test 4: surfaces drops recorded by
    /// either `replay_full` (full-recompute path) or the tip-fast-path's
    /// Rejected branch (in `try_tip_incremental`).
    ///
    /// # Panics
    /// Panics if the underlying `dropped_at_apply` mutex is poisoned —
    /// i.e., if the runtime task panicked while holding the lock. In a
    /// healthy test run this is structurally unreachable.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn dropped_at_apply(&self) -> std::collections::HashMap<EventHash, String> {
        self.runtime
            .dropped_at_apply
            .lock()
            .expect("dropped_at_apply mutex")
            .clone()
    }

    /// Read the current value of the tip-fast-path engagement counter
    /// on the underlying [`Runtime`]. Per plan-B-2.1 spec §5.
    ///
    /// Returns the count of times [`Runtime::try_tip_incremental`]
    /// engaged (combined `Accepted` + `Rejected` outcomes per spec §3.4).
    ///
    /// # Panics
    /// Panics if the underlying `tip_fast_path_hits` mutex is poisoned —
    /// i.e., if the runtime task panicked while holding the lock. In a
    /// healthy test run this is structurally unreachable.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn tip_fast_path_hits(&self) -> usize {
        *self
            .runtime
            .tip_fast_path_hits
            .lock()
            .expect("tip_fast_path_hits mutex poisoned")
    }

    /// Send a shutdown command to the peer's runtime task, allowing
    /// tests to exit cleanly without leaking the spawned task.
    pub async fn shutdown(&self) {
        let _ = self.runtime.author_tx.send(AuthorCommand::Shutdown).await;
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
