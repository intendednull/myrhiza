//! [`Subscription`] trait + [`MemSubscription`] impl.

use crate::{GossipMessage, SubError};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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
/// Carries a per-subscription `force_lag` flag whose strong ref lives
/// here and whose `Weak` is registered with the bus. [`MemBus::inject_lag`]
/// flips the flag to `true`; the next call to [`Self::recv`] swaps it
/// back to `false` and surfaces a synthetic `Lagged(1)`. See spec §6.3
/// / review-finding M-3 for the deterministic-lag affordance rationale.
pub struct MemSubscription {
    pub(crate) rx: tokio::sync::broadcast::Receiver<GossipMessage>,
    /// `true` iff [`MemBus::inject_lag`] has armed this subscription
    /// and the synthetic Lagged has not yet been delivered. Swapped
    /// to `false` on consumption — the affordance fires exactly once
    /// per arm.
    pub(crate) force_lag: Arc<AtomicBool>,
}

#[async_trait::async_trait]
impl Subscription for MemSubscription {
    async fn recv(&mut self) -> Result<Option<GossipMessage>, SubError> {
        // Deterministic-lag injection (spec §6.3 / review-finding M-3):
        // if the bus armed this subscription's flag, consume the flag
        // and surface a synthetic `Lagged(1)`. The underlying
        // broadcast receiver is left untouched, so any already-buffered
        // messages are still delivered by the next `recv` call —
        // matching the natural-overflow recovery shape.
        //
        // `swap` is sufficient: the flag is set/cleared only here and
        // by `inject_lag`, both as single atomic ops. No further
        // sequencing concerns.
        if self.force_lag.swap(false, Ordering::SeqCst) {
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
