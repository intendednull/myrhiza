//! [`MemBus`] + [`MemNetwork`] — in-process [`Network`] impl for tests.

use crate::{GossipMessage, NetError, Network, subscription::MemSubscription};
use myrhiza_types::Topic;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

/// In-process broadcast bus shared across multiple [`MemNetwork`]
/// handles. One tokio broadcast channel per topic; all subscribers
/// on a topic receive every publish.
pub struct MemBus {
    topics: Mutex<BTreeMap<Topic, tokio::sync::broadcast::Sender<GossipMessage>>>,
    capacity_per_topic: usize,
    /// Set of topics whose next `recv` should deterministically return
    /// `Err(SubError::Lagged(1))` exactly once before resuming normal
    /// delivery. Populated by [`MemBus::inject_lag`]; consumed by
    /// [`MemSubscription::recv`].
    ///
    /// Test affordance per spec §6.3 (review-finding M-3). Lives on the
    /// bus rather than per-subscription so a single `inject_lag(topic)`
    /// call deterministically perturbs every subscriber on that topic.
    force_lag: Mutex<BTreeSet<Topic>>,
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
            force_lag: Mutex::new(BTreeSet::new()),
        })
    }

    fn sender_for(&self, topic: Topic) -> tokio::sync::broadcast::Sender<GossipMessage> {
        // Recover from poisoning: the BTreeMap is always left in a valid
        // state because all mutations under this lock are single atomic
        // BTreeMap operations. A panicked prior holder cannot have left
        // a torn data structure.
        let mut topics = self
            .topics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        topics
            .entry(topic)
            .or_insert_with(|| tokio::sync::broadcast::channel(self.capacity_per_topic).0)
            .clone()
    }

    /// `true` if `topic` is currently armed by [`MemBus::inject_lag`] and
    /// the next `recv` on a subscriber of `topic` should return
    /// `Err(SubError::Lagged(1))`. Atomically consumes the flag (clear
    /// on read), so a single inject fires exactly once.
    ///
    /// Public-in-crate only; the lag-forcing protocol is between
    /// [`MemBus`] and [`MemSubscription`]. The flag is also consulted
    /// even when [`MemBus::inject_lag`] is gated out at compile time
    /// (the set is always empty in that case, so the check is a single
    /// lock + empty-test and stays well off any hot path).
    pub(crate) fn take_force_lag(&self, topic: Topic) -> bool {
        let mut set = self
            .force_lag
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        set.remove(&topic)
    }

    /// Arm `topic` so that the next `recv` on any subscriber of `topic`
    /// returns `Err(SubError::Lagged(1))` exactly once, then resumes
    /// normal delivery (including any pending broadcast messages).
    ///
    /// This is the deterministic counterpart to natural broadcast-buffer
    /// overflow used by lag-recovery convergence tests (spec §6.3,
    /// review-finding M-3). The natural-overflow approach is
    /// consumer-speed-dependent and can't be used to assert that the
    /// recovery path *always* fires.
    ///
    /// Idempotent: calling twice for the same topic has the same effect
    /// as calling once (the set holds at most one entry per topic).
    ///
    /// # Panics
    /// Does not panic. Mutex poisoning is recovered transparently.
    #[cfg(any(test, feature = "test-helpers"))]
    pub fn inject_lag(&self, topic: Topic) {
        let mut set = self
            .force_lag
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        set.insert(topic);
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

    async fn subscribe(&self, topic: Topic) -> Result<MemSubscription, NetError> {
        let sender = self.bus.sender_for(topic);
        Ok(MemSubscription {
            rx: sender.subscribe(),
            bus: Arc::clone(&self.bus),
            topic,
        })
    }

    async fn publish(&self, topic: Topic, msg: GossipMessage) -> Result<(), NetError> {
        let sender = self.bus.sender_for(topic);
        // broadcast::send returns Err only if there are no receivers; not an error here.
        let _ = sender.send(msg);
        Ok(())
    }

    async fn unsubscribe(&self, _topic: Topic) -> Result<(), NetError> {
        // MemSubscription's Drop releases its receiver automatically;
        // no separate unsubscribe state to manage.
        Ok(())
    }
}
