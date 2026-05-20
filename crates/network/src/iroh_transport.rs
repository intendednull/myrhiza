//! Iroh transport implementation of the [`Network`] trait.
//!
//! B-4.0 SKELETON: this module compiles against iroh 1.0.0-rc.0 +
//! iroh-gossip 0.99.0 and exposes the type surface, but every
//! `Network` method returns [`crate::NetError::Unimplemented`].
//! B-4.1 will wire `subscribe` + `publish` to real iroh-gossip
//! semantics; B-4.2 will thread per-connection sender identity (Q-4);
//! B-4.3 adds real cross-process acceptance tests.
//!
//! ## Why "skeleton"
//!
//! `prior-art/iroh/lessons.md` §Avoid row 1: "Every minor is
//! breaking" — pre-1.0 iroh API churn means landing the compile
//! shell first (pin-against-rc-0, prove the type surface aligns)
//! reduces the blast radius of a future re-pin. Behavioral work
//! lands in B-4.1+.
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
    #[allow(
        dead_code,
        reason = "behavior lands in B-4.1; skeleton only holds the handle"
    )]
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

    async fn subscribe(&self, _topic: Topic) -> Result<Self::Subscription, NetError> {
        Err(NetError::Unimplemented {
            method: "Network::subscribe",
            planned_in: "B-4.1",
        })
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

/// Skeleton `Subscription` impl. Instances cannot be constructed
/// outside this module in B-4.0 (the `subscribe` method that would
/// return one always returns `Err`).
pub struct IrohSubscription {
    _private: (),
}

#[async_trait::async_trait]
impl Subscription for IrohSubscription {
    async fn recv(&mut self) -> Result<Option<GossipMessage>, SubError> {
        // Unreachable because no IrohSubscription is ever constructed
        // (subscribe always returns Err in B-4.0).
        #[allow(clippy::unreachable)]
        {
            unreachable!(
                "IrohSubscription cannot be constructed in B-4.0 — \
                 Network::subscribe always returns Err(NetError::Unimplemented). \
                 Reaching this code path indicates a future B-4.1+ refactor \
                 constructed a subscription without implementing recv."
            )
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
