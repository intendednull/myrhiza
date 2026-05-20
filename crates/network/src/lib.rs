//! Network transport abstraction for the Myrhiza runtime.
//!
//! The [`Network`] trait + [`Subscription`] trait define an
//! executor-agnostic async surface. The in-process [`MemNetwork`]
//! double (in [`memory`]) satisfies both for kernel-tier acceptance
//! tests; B-4 will add an iroh implementor behind the same trait.
//!
//! ## Why `async_trait` macro
//!
//! Native async-fn-in-trait (stable in 1.75) requires `Send` future
//! bounds that the compiler currently can't express ergonomically
//! across `dyn`. `async_trait` adds a `Box<Future>` allocation per
//! call; B-4 may revisit once the `Send`-bound ergonomics stabilize.

#![doc(html_no_source)]

use myrhiza_types::{DriftMessage, Event, HeadsRequest, HeadsSummary, Topic};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod memory;
pub mod subscription;

pub use memory::{MemBus, MemNetwork};
pub use subscription::{MemSubscription, Subscription};

/// Gossip message envelope — the only thing that crosses the wire.
///
/// Variant tags are u32 fixint big-endian per the v1 canonical bincode
/// options chain (determinism.md §5.4). Wire-frozen by a snapshot test
/// in `crates/types/tests/wire_freeze.rs` (Task 11).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GossipMessage {
    /// A canonical [`Event`] envelope broadcast on a topic.
    Event(Event),
    /// Periodic per-author tip summary used by late joiners to detect
    /// missing events and trigger backfill via [`HeadsRequest`].
    HeadsSummary(HeadsSummary),
    /// Explicit request for missing events identified from a
    /// [`HeadsSummary`] diff against local DAG state.
    HeadsRequest(HeadsRequest),
    /// Equivocation / fork evidence broadcast by any peer that detects
    /// a chain violation for an author.
    Drift(DriftMessage),
}

/// Errors returned by [`Network`] transport operations.
#[derive(Debug, Error)]
pub enum NetError {
    /// The transport has been shut down; no further subscribe / publish
    /// / unsubscribe calls will succeed.
    #[error("subscription closed")]
    SubscribeClosed,
    /// The underlying transport rejected a publish.
    #[error("publish failed: {0}")]
    PublishFailed(String),
    /// The transport recognizes the call but the impl is not yet
    /// landed. Carries the method name + the slice in which it is
    /// planned. Used by skeleton transports (B-4.0) before behavioral
    /// implementations land. Per B-4.0 spec §3.3.
    #[error("network transport does not yet implement {method} (planned in {planned_in})")]
    Unimplemented {
        /// The name of the unimplemented method (e.g. `"Network::subscribe"`).
        method: &'static str,
        /// The plan slice in which this method's impl is scheduled
        /// (e.g. `"B-4.1"`).
        planned_in: &'static str,
    },
}

/// Errors returned by [`Subscription::recv`].
#[derive(Debug, Error)]
pub enum SubError {
    /// The transport's bounded buffer dropped `n` messages before this
    /// subscriber could consume them. Non-fatal — the runtime should
    /// publish a [`HeadsSummary`] to recover via backfill and continue
    /// calling `recv`.
    #[error("subscription lagged: dropped {0} messages")]
    Lagged(u64),
}

/// Network transport abstraction. Implementations are responsible for
/// gossip pub/sub semantics; iroh (B-4) provides QUIC + DHT under this
/// trait, [`MemNetwork`] provides in-process broadcast.
#[async_trait::async_trait]
pub trait Network: Send + Sync + 'static {
    /// The receive-side handle returned by [`Network::subscribe`].
    type Subscription: Subscription + Send + 'static;

    /// Subscribe to a topic. Caller drives `recv` on the returned subscription.
    ///
    /// # Errors
    /// Returns [`NetError::SubscribeClosed`] if the transport is shut down.
    async fn subscribe(&self, topic: Topic) -> Result<Self::Subscription, NetError>;

    /// Publish a message to all subscribers on a topic.
    ///
    /// # Errors
    /// Returns [`NetError::PublishFailed`] if the underlying transport rejects the send.
    async fn publish(&self, topic: Topic, msg: GossipMessage) -> Result<(), NetError>;

    /// Drop the subscription for `topic` on this network handle.
    ///
    /// # Errors
    /// Returns [`NetError::SubscribeClosed`] only if the transport is shut down.
    async fn unsubscribe(&self, topic: Topic) -> Result<(), NetError>;
}
