//! Iroh transport implementation of the [`Network`] trait.
//!
//! B-4.1 STATE (this commit, Task 4 of plan B-4.1): `subscribe` +
//! [`IrohSubscription::recv`] are real iroh-gossip 0.99.0-backed
//! implementations. `publish` and `unsubscribe` still return
//! [`crate::NetError::Unimplemented`]:
//! - `publish` is landed in the next commit (Task 5 of this plan).
//! - `unsubscribe` is deferred to B-4.2 — drop semantics on
//!   [`IrohSubscription`] cover practical "stop receiving".
//!
//! Q-4 sender attribution + real cross-process tests are B-4.2 /
//! B-4.3 scope.
//!
//! ## Why phased
//!
//! `prior-art/iroh/lessons.md` §Avoid row 1: "Every minor is
//! breaking" — pre-1.0 iroh API churn means landing the compile
//! shell first (B-4.0, pin-against-rc-0) reduced the blast radius of
//! a future re-pin. B-4.1 fills in behavior; B-4.2 will harden
//! attribution; B-4.3 adds real cross-process acceptance tests.
//!
//! ## API adaptations from plan B-4.0 §3.2
//!
//! The plan's hypothetical iroh names (dated 2026-05-08 prior-art
//! snapshot) differ from iroh 1.0.0-rc.0 ship-state on three points:
//!
//! 1. `iroh::Endpoint::node_id()` → `iroh::Endpoint::id()` (rename).
//! 2. `iroh::endpoint_id::ParseError` → `iroh::KeyParsingError`
//!    (the error lives at the crate root via `iroh-base` re-export).
//! 3. `iroh::EndpointId` is a **type alias** for `iroh::PublicKey`
//!    (`pub type EndpointId = PublicKey;` in `iroh-base/src/key.rs`),
//!    not a distinct nominal newtype. This blocks the plan's
//!    `From<iroh::EndpointId> for PeerPubkey` / `TryFrom<PeerPubkey>
//!    for iroh::EndpointId` trait impls under Rust's orphan rule:
//!    neither `From`/`TryFrom`, nor `iroh::EndpointId`
//!    (= `iroh::PublicKey`, foreign), nor `PeerPubkey` (defined in
//!    `myrhiza-types`, foreign to this crate) is local to
//!    `myrhiza-network`, so the impls cannot be written here.
//!    Adapted to free conversion functions
//!    ([`peer_pubkey_from_iroh`] + [`iroh_endpoint_id_from_peer_pubkey`])
//!    which preserve the spec's intent (distinct nominal types, no
//!    leakage of iroh's API into `myrhiza-types`' public surface) and
//!    can be promoted to trait impls in a future plan if/when the
//!    conversion moves into `myrhiza-types` behind a feature gate.

use bincode::Options;
use futures_lite::StreamExt;
use iroh_gossip::api::{Event, GossipTopic};
use myrhiza_types::canonical_bincode;

use crate::{GossipMessage, NetError, Network, SubError, Subscription};
use myrhiza_types::{PeerPubkey, Topic};

/// Iroh-backed [`Network`] implementation.
///
/// Holds owned (Arc-backed, cheaply cloneable) handles to a
/// host-level [`iroh::Endpoint`] + an [`iroh_gossip::Gossip`]
/// instance. Per `prior-art/iroh/lessons.md` §Borrow row 1, the
/// kernel embedder constructs these once and may hand one clone
/// here while retaining another for router-level work.
pub struct IrohNetwork {
    endpoint: iroh::Endpoint,
    gossip: iroh_gossip::Gossip,
    /// Cached `PeerPubkey` derived from `endpoint.id()` at
    /// construction time. Avoids per-call conversion.
    peer_pubkey: PeerPubkey,
}

impl IrohNetwork {
    /// Construct an `IrohNetwork` from a pre-built [`iroh::Endpoint`]
    /// and [`iroh_gossip::Gossip`].
    #[must_use]
    pub fn new(endpoint: iroh::Endpoint, gossip: iroh_gossip::Gossip) -> Self {
        let endpoint_id = endpoint.id();
        let peer_pubkey = peer_pubkey_from_iroh(endpoint_id);
        Self {
            endpoint,
            gossip,
            peer_pubkey,
        }
    }

    /// Return the local peer's public key (32-byte Ed25519).
    #[must_use]
    pub fn peer_pubkey(&self) -> PeerPubkey {
        self.peer_pubkey
    }

    /// Borrow the underlying [`iroh::Endpoint`] (for embedder use —
    /// relay config, ALPN registration). Kept narrow so future
    /// refactors can hide endpoint internals behind a capability gate.
    #[must_use]
    pub fn endpoint(&self) -> &iroh::Endpoint {
        &self.endpoint
    }
}

#[async_trait::async_trait]
impl Network for IrohNetwork {
    type Subscription = IrohSubscription;

    async fn subscribe(
        &self,
        topic: Topic,
        bootstrap: Vec<PeerPubkey>,
    ) -> Result<Self::Subscription, NetError> {
        let topic_id = iroh_topic_id_from_topic(topic);
        let mut bootstrap_ids: Vec<iroh::EndpointId> = Vec::with_capacity(bootstrap.len());
        for pk in bootstrap {
            let id = iroh_endpoint_id_from_peer_pubkey(pk)
                .map_err(|e| NetError::SubscribeFailed(format!("invalid bootstrap pubkey: {e}")))?;
            bootstrap_ids.push(id);
        }
        let gossip_topic = self
            .gossip
            .subscribe(topic_id, bootstrap_ids)
            .await
            .map_err(|e| NetError::SubscribeFailed(format!("iroh-gossip subscribe: {e}")))?;
        Ok(IrohSubscription::new(gossip_topic))
    }

    async fn publish(&self, _topic: Topic, _msg: GossipMessage) -> Result<(), NetError> {
        Err(NetError::Unimplemented {
            method: "Network::publish",
            planned_in: "B-4.1",
        })
    }

    async fn unsubscribe(&self, _topic: Topic) -> Result<(), NetError> {
        Err(NetError::Unimplemented {
            method: "Network::unsubscribe",
            planned_in: "B-4.1",
        })
    }
}

/// Iroh-gossip-backed subscription.
///
/// Wraps a [`iroh_gossip::GossipTopic`] (a stream of
/// `Result<Event, ApiError>`), filters events to only surface
/// [`Event::Received`] payloads (decoded via canonical bincode),
/// maps [`Event::Lagged`] and stream-level `ApiError` to
/// [`SubError::Lagged(0)`] (count fidelity lost — see spec §6),
/// maps bincode-decode failures to [`SubError::DecodeFailed`], and
/// silently consumes membership events ([`Event::NeighborUp`],
/// [`Event::NeighborDown`]).
///
/// Per B-4.1 spec §3.2.
pub struct IrohSubscription {
    inner: GossipTopic,
}

impl IrohSubscription {
    /// Construct from a [`GossipTopic`] returned by
    /// `iroh_gossip::Gossip::subscribe`. Crate-private — callers
    /// reach this via [`IrohNetwork::subscribe`].
    pub(crate) fn new(inner: GossipTopic) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl Subscription for IrohSubscription {
    async fn recv(&mut self) -> Result<Option<GossipMessage>, SubError> {
        loop {
            match self.inner.next().await {
                None => return Ok(None),
                Some(Err(_api_err)) => {
                    // Stream-level error from iroh-gossip mid-flight.
                    // Surfacing as Lagged(0) is a pragmatic mapping —
                    // we lose error specifics but the runtime's
                    // Lagged path (HeadsSummary backfill) is the
                    // closest match for "I may have missed messages."
                    // TRADE-OFF: if the gossip task has actually died,
                    // we'll spin re-calling recv and getting ApiError
                    // forever. Halt detection is B-4.3's scope.
                    return Err(SubError::Lagged(0));
                }
                Some(Ok(Event::Received(msg))) => {
                    // Capture the last-hop neighbor for attribution
                    // on decode failure. NOT necessarily the original
                    // publisher (Plumtree forwarding hides that;
                    // Q-4 attribution is B-4.2 scope).
                    let last_hop_peer = Some(peer_pubkey_from_iroh(msg.delivered_from));
                    match canonical_bincode().deserialize::<GossipMessage>(&msg.content) {
                        Ok(decoded) => return Ok(Some(decoded)),
                        Err(_decode_err) => {
                            return Err(SubError::DecodeFailed {
                                peer: last_hop_peer,
                            });
                        }
                    }
                }
                Some(Ok(Event::Lagged)) => {
                    // Iroh-gossip drops the lagged count
                    // (`gossip/src/net.rs:940` discards it with `_`);
                    // sentinel 0 preserves the variant shape. Reclaiming
                    // count fidelity needs an upstream patch — out of
                    // scope for B-4.1.
                    return Err(SubError::Lagged(0));
                }
                Some(Ok(Event::NeighborUp(_) | Event::NeighborDown(_))) => {
                    // Membership events — silently consume + loop.
                    // Surfacing through the trait would force every
                    // Subscription consumer to handle them; only
                    // IrohNetwork produces them.
                }
            }
        }
    }
}

// ---- PeerPubkey <-> iroh::EndpointId conversions ----
//
// These are free functions, not trait impls, because the orphan
// rule blocks `impl From<iroh::EndpointId> for PeerPubkey` (and the
// reverse): every type involved is foreign to `myrhiza-network`
// (`iroh::EndpointId` is a re-export of `iroh::PublicKey`;
// `PeerPubkey` lives in `myrhiza-types`). See the module-level
// docstring §"API adaptations" for the full reasoning.

/// Convert an iroh endpoint identifier into a Myrhiza `PeerPubkey`.
///
/// Infallible: both types are raw 32-byte Ed25519 public keys per
/// `prior-art/iroh/identity.md` §"`NodeID` = Ed25519 public key", and
/// `PeerPubkey::from_bytes` is a transparent wrap.
#[must_use]
pub fn peer_pubkey_from_iroh(endpoint_id: iroh::EndpointId) -> PeerPubkey {
    PeerPubkey::from_bytes(*endpoint_id.as_bytes())
}

/// Convert a Myrhiza `PeerPubkey` into an iroh endpoint identifier.
///
/// Fallible: iroh validates the bytes form a valid Ed25519 curve
/// point. In practice the conversion only fails on `PeerPubkey`
/// values that were never produced from a verified key — Myrhiza's
/// internal construction paths all originate from verified Ed25519
/// signatures, so the failure path is unreachable in normal use.
/// `TryFrom` semantics are still correct.
///
/// # Errors
///
/// Returns [`iroh::KeyParsingError`] if the underlying 32 bytes do
/// not form a valid Ed25519 public key (e.g. not a curve point).
pub fn iroh_endpoint_id_from_peer_pubkey(
    peer: PeerPubkey,
) -> Result<iroh::EndpointId, iroh::KeyParsingError> {
    iroh::EndpointId::from_bytes(peer.as_bytes())
}

/// Convert a Myrhiza [`Topic`] into an `iroh_gossip::TopicId`.
///
/// Both types are transparent 32-byte newtypes. Free function (not a
/// `From`/`Into` impl) for the same orphan-rule reason as
/// [`peer_pubkey_from_iroh`]: `TopicId` lives in `iroh-gossip` and
/// `Topic` lives in `myrhiza-types`; neither is local to
/// `myrhiza-network`.
///
/// Per B-4.1 spec §2 (Topic ↔ `TopicId` conversion row).
#[must_use]
pub fn iroh_topic_id_from_topic(topic: Topic) -> iroh_gossip::TopicId {
    iroh_gossip::TopicId::from_bytes(*topic.as_bytes())
}
