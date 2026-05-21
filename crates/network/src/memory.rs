//! [`MemBus`] + [`MemNetwork`] — in-process [`Network`] impl for tests.

use crate::{GossipMessage, NetError, Network, subscription::MemSubscription};
use myrhiza_types::{PeerPubkey, Topic};
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
// `Ordering` is only referenced inside `MemBus::inject_lag`, which is
// itself gated behind `#[cfg(any(test, feature = "test-helpers"))]`.
// Without the matching gate here, a plain `cargo build -p myrhiza-kernel`
// (lib-only, no features) trips `-D unused-imports`.
#[cfg(any(test, feature = "test-helpers"))]
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, Weak};

/// Per-topic bus state: the broadcast sender + the set of live
/// per-subscription "force-lag" flags pinned by [`MemBus::inject_lag`].
///
/// The flags are held as `Weak<AtomicBool>` so a dropped
/// [`MemSubscription`] is naturally evicted on the next housekeeping
/// sweep — no separate unsubscribe bookkeeping needed.
struct TopicState {
    sender: tokio::sync::broadcast::Sender<GossipMessage>,
    /// One entry per live subscription on this topic. Strong refs
    /// live inside each [`MemSubscription`]; [`MemBus::inject_lag`]
    /// upgrades each weak and sets it to `true`.
    force_lag_flags: Vec<Weak<AtomicBool>>,
    /// Mirrors `force_lag_flags`. Set via [`MemBus::inject_transport_error`].
    /// One entry per live subscription on this topic. Per B-4.3 spec §3.4.
    force_transport_error_flags: Vec<Weak<AtomicBool>>,
}

/// In-process broadcast bus shared across multiple [`MemNetwork`]
/// handles. One tokio broadcast channel per topic; all subscribers
/// on a topic receive every publish.
pub struct MemBus {
    topics: Mutex<BTreeMap<Topic, TopicState>>,
    capacity_per_topic: usize,
}

impl MemBus {
    /// Construct a new bus with `capacity` slots per topic broadcast
    /// channel. Returns an [`Arc`] so the bus can be shared across
    /// multiple [`MemNetwork`] handles.
    #[must_use]
    pub fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            topics: Mutex::new(BTreeMap::new()),
            capacity_per_topic: capacity,
        })
    }

    /// Look up (or lazily create) the topic state, then build a new
    /// [`MemSubscription`] backed by a fresh broadcast receiver and a
    /// fresh `Arc<AtomicBool>` lag-flag that the bus also tracks via
    /// `Weak`. The strong ref lives on the subscription; bus only
    /// holds weaks so subscription drop naturally evicts.
    fn make_subscription(self: &Arc<Self>, topic: Topic) -> MemSubscription {
        let mut topics = self
            .topics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let state = topics.entry(topic).or_insert_with(|| TopicState {
            sender: tokio::sync::broadcast::channel(self.capacity_per_topic).0,
            force_lag_flags: Vec::new(),
            force_transport_error_flags: Vec::new(),
        });
        let lag_flag = Arc::new(AtomicBool::new(false));
        let transport_flag = Arc::new(AtomicBool::new(false));
        // GC dead weaks while we're holding the lock anyway.
        state.force_lag_flags.retain(|w| w.strong_count() > 0);
        state
            .force_transport_error_flags
            .retain(|w| w.strong_count() > 0);
        state.force_lag_flags.push(Arc::downgrade(&lag_flag));
        state
            .force_transport_error_flags
            .push(Arc::downgrade(&transport_flag));
        MemSubscription {
            rx: state.sender.subscribe(),
            force_lag: lag_flag,
            force_transport_error: transport_flag,
        }
    }

    fn sender_for(&self, topic: Topic) -> tokio::sync::broadcast::Sender<GossipMessage> {
        let mut topics = self
            .topics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        topics
            .entry(topic)
            .or_insert_with(|| TopicState {
                sender: tokio::sync::broadcast::channel(self.capacity_per_topic).0,
                force_lag_flags: Vec::new(),
                force_transport_error_flags: Vec::new(),
            })
            .sender
            .clone()
    }

    /// Arm every live subscription on `topic` so that its next `recv`
    /// returns `Err(SubError::Lagged(1))` exactly once, then resumes
    /// normal delivery (any already-buffered broadcast messages still
    /// arrive on the recv call after the synthetic lag).
    ///
    /// This is the deterministic counterpart to natural broadcast-buffer
    /// overflow used by lag-recovery convergence tests (spec §6.3,
    /// review-finding M-3). The natural-overflow approach is
    /// consumer-speed-dependent and can't be used to assert that the
    /// recovery path *always* fires on a specific subscriber.
    ///
    /// Per-subscription flags (rather than a single bus-wide flag per
    /// topic) ensure that when several peers share a topic — as in
    /// every kernel convergence test — *each* peer surfaces the
    /// synthetic Lagged, instead of whichever peer happens to poll
    /// `recv` first racing to consume a shared bit.
    ///
    /// Idempotent: an already-armed flag stays armed; calling twice
    /// has the same effect as calling once.
    ///
    /// # Panics
    /// Does not panic. Mutex poisoning is recovered transparently.
    #[cfg(any(test, feature = "test-helpers"))]
    pub fn inject_lag(&self, topic: Topic) {
        let mut topics = self
            .topics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(state) = topics.get_mut(&topic) {
            state.force_lag_flags.retain(|w| w.strong_count() > 0);
            for weak in &state.force_lag_flags {
                if let Some(flag) = weak.upgrade() {
                    flag.store(true, Ordering::SeqCst);
                }
            }
        }
        // If `topic` has no live subscriptions, the call is a no-op.
        // Tests that need the flag to fire must subscribe before
        // calling inject_lag — same precondition as natural overflow.
    }

    /// Arm every live subscription on `topic` so that its next `recv`
    /// returns `Err(SubError::TransportError("injected by ..."))`
    /// exactly once, then resumes normal delivery.
    ///
    /// Gate-paired with [`MemBus::inject_lag`]: both are deterministic
    /// test affordances that bypass the natural-overflow / natural-actor-
    /// death paths which are timing-dependent and hard to assert against.
    ///
    /// Per B-4.3 spec §3.4.
    ///
    /// # Panics
    /// Does not panic. Mutex poisoning is recovered transparently.
    #[cfg(any(test, feature = "test-helpers"))]
    pub fn inject_transport_error(&self, topic: Topic) {
        let mut topics = self
            .topics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(state) = topics.get_mut(&topic) {
            state
                .force_transport_error_flags
                .retain(|w| w.strong_count() > 0);
            for weak in &state.force_transport_error_flags {
                if let Some(flag) = weak.upgrade() {
                    flag.store(true, Ordering::SeqCst);
                }
            }
        }
    }
}

/// In-process [`Network`] implementation backed by a shared [`MemBus`].
///
/// Clones share the same underlying bus, so multiple `MemNetwork`
/// handles on the same bus model a fully-connected gossip mesh for
/// kernel-tier acceptance tests.
#[derive(Clone)]
pub struct MemNetwork {
    bus: Arc<MemBus>,
}

impl MemNetwork {
    /// Construct a new handle on the given shared [`MemBus`].
    #[must_use]
    pub fn new(bus: Arc<MemBus>) -> Self {
        Self { bus }
    }
}

#[async_trait::async_trait]
impl Network for MemNetwork {
    type Subscription = MemSubscription;

    async fn subscribe(
        &self,
        topic: Topic,
        _bootstrap: Vec<PeerPubkey>,
    ) -> Result<MemSubscription, NetError> {
        // In-process broadcast has no peer-discovery semantics;
        // bootstrap is intentionally ignored. Per B-4.1 spec §3.3.
        Ok(self.bus.make_subscription(topic))
    }

    async fn publish(&self, topic: Topic, msg: GossipMessage) -> Result<(), NetError> {
        let sender = self.bus.sender_for(topic);
        // broadcast::send returns Err only if there are no receivers; not an error here.
        let _ = sender.send(msg);
        Ok(())
    }

    async fn unsubscribe(&self, _topic: Topic) -> Result<(), NetError> {
        // MemSubscription's Drop releases its receiver automatically;
        // the Weak<AtomicBool> in TopicState::force_lag_flags is
        // collected on the next make_subscription / inject_lag sweep.
        Ok(())
    }
}
