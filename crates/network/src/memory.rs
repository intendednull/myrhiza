//! [`MemBus`] + [`MemNetwork`] — in-process [`Network`] impl for tests.

use crate::{GossipMessage, NetError, Network, subscription::MemSubscription};
use myrhiza_types::Topic;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// In-process broadcast bus shared across multiple [`MemNetwork`]
/// handles. One tokio broadcast channel per topic; all subscribers
/// on a topic receive every publish.
pub struct MemBus {
    topics: Mutex<BTreeMap<Topic, tokio::sync::broadcast::Sender<GossipMessage>>>,
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
