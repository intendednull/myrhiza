//! [`Subscription`] trait + [`MemSubscription`] impl.

use crate::{GossipMessage, SubError, memory::MemBus};
use myrhiza_types::Topic;
use std::sync::Arc;

/// Receive-side of a topic subscription.
#[async_trait::async_trait]
pub trait Subscription: Send {
    /// Receive the next message, lag signal, or end-of-stream.
    ///
    /// Returns:
    /// - `Ok(Some(msg))` — delivered message
    /// - `Err(SubError::Lagged(n))` — underlying transport dropped `n`
    ///   messages; non-fatal, runtime should publish a `HeadsSummary`
    ///   to recover via backfill, then continue calling `recv`
    /// - `Ok(None)` — subscription closed
    ///
    /// # Errors
    /// Lag is the only non-fatal error variant.
    async fn recv(&mut self) -> Result<Option<GossipMessage>, SubError>;
}

/// In-process subscription backed by a tokio broadcast receiver.
///
/// Carries an `Arc<MemBus>` + `topic` solely so [`Self::recv`] can check
/// the bus-side `force_lag` flag set by [`MemBus::inject_lag`] (spec
/// §6.3 deterministic-lag affordance). The bus reference adds a single
/// `Arc` clone per subscribe and a single `Mutex` lock + empty-set
/// check per `recv` — negligible on the hot path and `force_lag` is
/// empty in non-test builds.
pub struct MemSubscription {
    pub(crate) rx: tokio::sync::broadcast::Receiver<GossipMessage>,
    pub(crate) bus: Arc<MemBus>,
    pub(crate) topic: Topic,
}

#[async_trait::async_trait]
impl Subscription for MemSubscription {
    async fn recv(&mut self) -> Result<Option<GossipMessage>, SubError> {
        // Deterministic-lag injection (spec §6.3 / review-finding M-3):
        // if the bus has armed `force_lag` for this topic, consume the
        // flag and surface a synthetic `Lagged(1)` exactly once. The
        // underlying broadcast receiver is left untouched, so any
        // already-buffered messages are still delivered by the next
        // `recv` call — matching the natural-overflow recovery shape.
        if self.bus.take_force_lag(self.topic) {
            return Err(SubError::Lagged(1));
        }
        match self.rx.recv().await {
            Ok(msg) => Ok(Some(msg)),
            Err(tokio::sync::broadcast::error::RecvError::Closed) => Ok(None),
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => Err(SubError::Lagged(n)),
        }
    }
}

/// Blanket impl so a `Box<dyn Subscription + Send>` satisfies
/// [`Subscription`] without re-boxing per `recv` call.
///
/// This is the receive-side complement to the `Network::Subscription`
/// erasure pattern used by callers that hold a `dyn Network` with a
/// uniform concrete subscription type: they box the inner impl's
/// `Subscription` as `Box<dyn Subscription + Send>`, and rely on this
/// blanket impl to forward `recv` through the box. Lives here rather
/// than in a downstream crate because of the orphan rule.
#[async_trait::async_trait]
impl<S: Subscription + Send + ?Sized> Subscription for Box<S> {
    async fn recv(&mut self) -> Result<Option<GossipMessage>, SubError> {
        (**self).recv().await
    }
}
