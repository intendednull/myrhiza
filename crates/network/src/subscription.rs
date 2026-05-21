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
    /// - `Err(SubError::DecodeFailed { peer })` — received bytes failed
    ///   canonical bincode decoding; log + discard. Per B-4.1 spec §3.0,
    ///   the runtime MUST NOT trigger `HeadsSummary` backfill on this
    ///   variant (distinct from `Lagged`). See [`SubError::DecodeFailed`]
    ///   for the full rationale.
    /// - `Err(SubError::TransportError(reason))` — the underlying
    ///   transport reported an error mid-stream. The runtime counts
    ///   consecutive occurrences and halts after a configurable
    ///   threshold; see [`SubError::TransportError`] for the full
    ///   rationale. Per B-4.3 spec §3.0.
    /// - `Ok(None)` — subscription closed
    ///
    /// # Errors
    /// `Lagged`, `DecodeFailed`, and `TransportError` are all non-fatal
    /// at the trait surface. Policy decisions (backfill vs discard vs
    /// halt) live in the consumer (`Runtime`). Per B-4.3 spec §3.0.
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
    /// Per-subscription one-shot transport-error flag. Set via
    /// [`MemBus::inject_transport_error`]. `pub(crate)` so
    /// `MemBus::make_subscription` in `memory.rs` can initialize it
    /// directly (same pattern as `rx` + `force_lag`).
    ///
    /// Per B-4.3 spec §3.4.
    pub(crate) force_transport_error: Arc<AtomicBool>,
}

#[async_trait::async_trait]
impl Subscription for MemSubscription {
    async fn recv(&mut self) -> Result<Option<GossipMessage>, SubError> {
        // Transport-error injection (B-4.3 spec §3.4): checked BEFORE the
        // lag flag because transport-error represents a more severe failure
        // mode. Both flags can be set independently; transport-error has
        // priority. Fires exactly once per arm (swap to false on consume).
        if self.force_transport_error.swap(false, Ordering::SeqCst) {
            return Err(SubError::TransportError(
                "injected by MemBus::inject_transport_error".to_string(),
            ));
        }
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
