//! [`Subscription`] trait + [`MemSubscription`] impl.

use crate::{GossipMessage, SubError};

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
pub struct MemSubscription {
    pub(crate) rx: tokio::sync::broadcast::Receiver<GossipMessage>,
}

#[async_trait::async_trait]
impl Subscription for MemSubscription {
    async fn recv(&mut self) -> Result<Option<GossipMessage>, SubError> {
        match self.rx.recv().await {
            Ok(msg) => Ok(Some(msg)),
            Err(tokio::sync::broadcast::error::RecvError::Closed) => Ok(None),
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => Err(SubError::Lagged(n)),
        }
    }
}
