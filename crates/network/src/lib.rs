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

use myrhiza_distribution::{PublicationEvent, RevocationEvent};
use myrhiza_types::{DirectHeadsRequest, DriftMessage, Event, HeadsSummary, PeerPubkey, Topic};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod memory;
pub mod request;
pub mod subscription;

#[cfg(feature = "network-iroh")]
pub mod iroh_transport;

pub use memory::{MemBus, MemNetwork};
pub use request::{
    ArcRequestHandler, HEADS_REQUEST_ALPN, HeadsResponder, HeadsStream, HeadsStreamError,
    RequestHandler,
};
pub use subscription::{MemSubscription, Subscription};

#[cfg(feature = "network-iroh")]
pub use iroh_transport::IrohNetwork;

/// Gossip message envelope — the only thing that crosses the wire.
///
/// Variant tags are u32 fixint big-endian per the v1 canonical bincode
/// options chain (determinism.md §5.4). Wire-frozen by snapshot tests
/// in `crates/types/tests/wire_freeze.rs` (Task 11; B-11 adds the
/// `Revocation`=3 / `Publication`=4 pins).
///
/// **Wire-freeze (B-11 §3.1):** variants are append-only — new variants
/// go AFTER the last existing one so canonical-bincode u32-BE tags never
/// shift. `Event`=0 / `HeadsSummary`=1 / `Drift`=2 are frozen; the B-11
/// `Revocation`=3 / `Publication`=4 ride the existing
/// `Network::subscribe`/`publish` path on per-author derived topics
/// rather than widening the `Network` trait.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GossipMessage {
    /// A canonical [`Event`] envelope broadcast on a topic.
    Event(Event),
    /// Periodic per-author tip summary used by late joiners to detect
    /// missing events and trigger backfill via direct-stream requests.
    HeadsSummary(HeadsSummary),
    /// Equivocation / fork evidence broadcast by any peer that detects
    /// a chain violation for an author.
    Drift(DriftMessage),
    /// Signed revocation envelope broadcast on the per-author revocation
    /// topic (`derive_revocation_topic`). Appended in B-11 at discriminant
    /// 3 (append-only — see the wire-freeze note above). Per B-11 spec §3.1.
    Revocation(RevocationEvent),
    /// Signed publication (release-announcement) envelope broadcast on the
    /// per-author publication topic (`derive_publication_topic`). Appended
    /// in B-11 at discriminant 4. Per B-11 spec §3.1.
    Publication(PublicationEvent),
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
    /// Subscribe call failed for a reason other than transport
    /// shutdown — e.g. invalid bootstrap peer pubkey, gossip-layer
    /// API error during topic-join. Carries a human-readable
    /// diagnostic. Per B-4.1 spec §3.0.
    #[error("subscribe failed: {0}")]
    SubscribeFailed(String),
    /// Direct-stream request to a peer failed to establish (dial error,
    /// ALPN refusal, peer unreachable). Per B-4.4 spec §3.2.
    #[error("request to peer {peer:?} failed: {reason}")]
    RequestFailed {
        /// The target peer that the request was directed at.
        peer: PeerPubkey,
        /// Human-readable diagnostic.
        reason: String,
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
    /// A received wire message did not decode under the canonical
    /// bincode contract. Carries the last-hop iroh-gossip neighbor
    /// (NOT necessarily the original publisher; per-publisher
    /// attribution is Q-4 / B-4.2 work). The runtime treats this as
    /// log + discard — distinct from [`SubError::Lagged`], which
    /// triggers a backfill `HeadsSummary` publish. Routing decode
    /// failures through `Lagged` would let a single bad-bytes peer
    /// flood the network with backfill traffic.
    ///
    /// Per B-4.1 spec §3.0 + the runtime handler at
    /// `runtime.rs handle_event` (see `SubError` handling in the
    /// receive loop).
    #[error("decoded message failed bincode contract (from peer: {peer:?})")]
    DecodeFailed {
        /// The iroh-gossip `delivered_from` peer (last-hop neighbor
        /// under Plumtree forwarding, not necessarily the
        /// publisher). `None` for transports without per-message
        /// sender identity ([`MemNetwork`] never emits this variant).
        peer: Option<PeerPubkey>,
    },

    /// The transport layer (e.g. iroh-gossip's actor) reported an
    /// error mid-stream. Semantically distinct from
    /// [`SubError::Lagged`] (broadcast-channel overrun, recoverable
    /// via backfill) and [`SubError::DecodeFailed`] (wire-byte parse
    /// failure on a single message). May indicate the underlying
    /// transport has DIED; the runtime accumulates these and halts
    /// after `RuntimeCfg::transport_error_halt_threshold` consecutive
    /// occurrences. Per B-4.3 spec §3.0.
    #[error("transport error: {0}")]
    TransportError(String),
}

/// Network transport abstraction. Implementations are responsible for
/// gossip pub/sub semantics; iroh (B-4) provides QUIC + DHT under this
/// trait, [`MemNetwork`] provides in-process broadcast.
#[async_trait::async_trait]
pub trait Network: Send + Sync + 'static {
    /// The receive-side handle returned by [`Network::subscribe`].
    type Subscription: Subscription + Send + 'static;

    /// Subscribe to a topic, with optional bootstrap peer hints.
    ///
    /// **SemVer-breaking from B-4.0:** this trait method gained the
    /// `bootstrap` parameter in B-4.1. Both in-tree impls
    /// ([`MemNetwork`], [`IrohNetwork`]) and all 7 call sites were
    /// updated atomically; out-of-tree implementors must add the
    /// parameter. See spec §3.1 for rationale.
    ///
    /// For transports that maintain a peer-discovery overlay
    /// ([`IrohNetwork`]), `bootstrap` is a list of `PeerPubkey`s to
    /// dial when forming the topic's swarm. An empty `bootstrap` is
    /// legal — the topic exists locally and waits for inbound joins.
    ///
    /// For transports without peer-discovery semantics
    /// ([`MemNetwork`]), `bootstrap` is ignored (in-process broadcast
    /// routes by topic only).
    ///
    /// Per B-4.1 spec §3.1.
    ///
    /// # Errors
    /// Returns [`NetError::SubscribeClosed`] if the transport has been
    /// shut down, or [`NetError::SubscribeFailed`] if the gossip-layer
    /// subscribe call fails (e.g. invalid bootstrap pubkey).
    async fn subscribe(
        &self,
        topic: Topic,
        bootstrap: Vec<PeerPubkey>,
    ) -> Result<Self::Subscription, NetError>;

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

    /// Issue a direct-stream `HeadsRequest` to a specific peer.
    ///
    /// Returns a [`HeadsStream`] that yields response events as they
    /// arrive. Stream terminates with `None` when the responder closes
    /// cleanly, or with an `Err` for transport / decode / handler
    /// failures.
    ///
    /// For transports without point-to-point semantics, this is the
    /// only correct backfill primitive. The gossip-routed
    /// `HeadsRequest` variant has been retired (B-4.7); this direct-stream
    /// method is the sole backfill primitive going forward.
    ///
    /// **SemVer-breaking** — added in B-4.4. Out-of-tree implementors
    /// must add it; both in-tree impls ([`MemNetwork`], [`IrohNetwork`])
    /// are updated in this PR.
    ///
    /// Per B-4.4 spec §3.2.
    ///
    /// # Errors
    /// Returns [`NetError::RequestFailed`] if the transport cannot
    /// dial the peer or establish a request stream.
    async fn request_heads(
        &self,
        peer: PeerPubkey,
        request: DirectHeadsRequest,
    ) -> Result<HeadsStream, NetError>;

    /// Install a [`RequestHandler`] for the accept side of
    /// direct-stream requests. Idempotent — last call wins.
    ///
    /// Embedders construct the handler with whatever state it needs
    /// (DAG access, topic filter, rate limiter) and install it on the
    /// network at startup. Without an installed handler, inbound
    /// direct-stream requests are silently rejected (clean EOF
    /// to the requester).
    ///
    /// **SemVer-breaking** — added in B-4.4. Out-of-tree implementors
    /// must add it.
    ///
    /// Per B-4.4 spec §3.2.
    fn install_request_handler(&self, handler: ArcRequestHandler);
}
