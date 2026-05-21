//! Iroh transport implementation of the [`Network`] trait.
//!
//! B-4.2 STATE: `subscribe`, `publish`, and `unsubscribe` are real
//! iroh-gossip 0.99.0-backed implementations. `unsubscribe` is a
//! semantic no-op at the `IrohNetwork` boundary — callers MUST drop
//! the [`IrohSubscription`] returned by `subscribe()` to actually
//! leave the topic. Q-4 sender attribution + real cross-process tests
//! are B-4.2 / B-4.3 scope.
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
use bytes::Bytes;
use futures_lite::StreamExt;
use iroh_gossip::api::{Event, GossipTopic};
use myrhiza_types::canonical_bincode;
use std::sync::{Arc, Mutex};

use crate::request::{
    ArcRequestHandler, HEADS_REQUEST_ALPN, HEADS_STREAM_CHANNEL_CAPACITY, HeadsStream,
    build_length_prefixed_frame,
};
use crate::{GossipMessage, NetError, Network, SubError, Subscription};
use myrhiza_types::{DirectHeadsRequest, PeerPubkey, Topic};

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
    /// Installed direct-stream request handler. Set via
    /// [`Network::install_request_handler`]; consumed by
    /// `HeadsRequestProtocol::accept` (Task 6) on inbound requests.
    /// `Arc<Mutex<Option<_>>>` so that protocol-handler clones returned
    /// from `protocol_handler()` share state with this instance — and
    /// because the trait contract allows re-installation (last call
    /// wins), so `OnceLock` is unsuitable.
    /// Per B-4.4 spec §3.4.3.
    request_handler: Arc<Mutex<Option<ArcRequestHandler>>>,
}

impl IrohNetwork {
    /// Construct an `IrohNetwork` from a pre-built [`iroh::Endpoint`]
    /// and [`iroh_gossip::Gossip`].
    ///
    /// # Lifecycle precondition
    ///
    /// The caller MUST have already registered `iroh_gossip::ALPN`
    /// against `gossip` via an [`iroh::protocol::Router`] (constructed
    /// once at kernel boot per `prior-art/iroh/lessons.md` §Borrow
    /// row 2). Without that Router wiring, inbound iroh-gossip streams
    /// never reach the gossip handler — `subscribe` will appear to
    /// succeed while `recv` never yields a `Received` event. The
    /// Router must outlive this `IrohNetwork` instance; dropping it
    /// first causes subsequent `subscribe` calls to fail with
    /// `ApiError` (per B-4.1 spec §10 "Drop order with Router").
    #[must_use]
    pub fn new(endpoint: iroh::Endpoint, gossip: iroh_gossip::Gossip) -> Self {
        let endpoint_id = endpoint.id();
        let peer_pubkey = peer_pubkey_from_iroh(endpoint_id);
        Self {
            endpoint,
            gossip,
            peer_pubkey,
            request_handler: Arc::new(Mutex::new(None)),
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

    async fn publish(&self, topic: Topic, msg: GossipMessage) -> Result<(), NetError> {
        let topic_id = iroh_topic_id_from_topic(topic);
        let bytes = canonical_bincode()
            .serialize(&msg)
            .map_err(|e| NetError::PublishFailed(format!("bincode encode: {e}")))?;
        // TRADE-OFF (per spec §3.2): each publish re-subscribes + splits
        // the GossipTopic. Iroh-gossip's actor architecture spawns a
        // fresh topic_subscriber_loop task per subscribe call
        // (`gossip/src/net.rs:600-643`); per-publish this is task-spawn
        // churn. The GossipTopic departs the swarm when its sender +
        // receiver drop, so cost is bounded per call. B-4.2/B-4.3 may
        // cache a per-topic GossipSender — flagged in spec §11.
        let gossip_topic = self
            .gossip
            .subscribe(topic_id, vec![])
            .await
            .map_err(|e| NetError::PublishFailed(format!("iroh-gossip subscribe: {e}")))?;
        let (sender, _receiver) = gossip_topic.split();
        sender
            .broadcast(Bytes::from(bytes))
            .await
            .map_err(|e| NetError::PublishFailed(format!("iroh-gossip broadcast: {e}")))?;
        Ok(())
    }

    async fn unsubscribe(&self, _topic: Topic) -> Result<(), NetError> {
        // iroh-gossip 0.99.0 exposes no explicit "leave swarm" API in
        // its public surface; drop IS the v1 implementation. If iroh
        // adds an explicit leave API, this method becomes the natural
        // wrapper site.
        //
        // GossipTopic self-cleans when all senders + receivers drop
        // (iroh-gossip-0.99.0 gossip/src/api.rs:207 — implicit cleanup
        // via the dropped mpsc sender inside the actor; no explicit
        // Drop impl).
        //
        // This method is semantically a no-op at the IrohNetwork
        // boundary: `IrohNetwork` does not hold any subscriptions to
        // drop. Cleanup happens through caller-side subscription drop.
        // Callers MUST drop the IrohSubscription returned by
        // `subscribe()` to actually leave the topic — `unsubscribe()`
        // alone is insufficient.
        //
        // Per B-4.2 spec §3.3.
        Ok(())
    }

    async fn request_heads(
        &self,
        peer: PeerPubkey,
        request: DirectHeadsRequest,
    ) -> Result<HeadsStream, NetError> {
        let target_id =
            iroh_endpoint_id_from_peer_pubkey(peer).map_err(|e| NetError::RequestFailed {
                peer,
                reason: format!("invalid target pubkey: {e}"),
            })?;

        // **Impl-time verification (spec §9 gap #4)**: iroh 1.0.0-rc.0
        // `Endpoint::connect` takes `impl Into<EndpointAddr>` + `&[u8]`.
        // `EndpointId` implements `From<EndpointId> for EndpointAddr`
        // (iroh-base-1.0.0-rc.0 endpoint_addr.rs:155), so passing
        // `target_id` directly is correct — no `EndpointAddr::new()`
        // wrapper needed.
        let connection = self
            .endpoint
            .connect(target_id, HEADS_REQUEST_ALPN)
            .await
            .map_err(|e| NetError::RequestFailed {
                peer,
                reason: format!("connect: {e}"),
            })?;

        // **Impl-time verification (spec §9 gap #5)**: `open_bi()`
        // returns `(SendStream, RecvStream)` per prior-art/iroh/architecture.md.
        let (mut send_stream, recv_stream) =
            connection
                .open_bi()
                .await
                .map_err(|e| NetError::RequestFailed {
                    peer,
                    reason: format!("open_bi: {e}"),
                })?;

        // Encode + write the request.
        let req_bytes =
            canonical_bincode()
                .serialize(&request)
                .map_err(|e| NetError::RequestFailed {
                    peer,
                    reason: format!("encode request: {e}"),
                })?;
        let frame = build_length_prefixed_frame(&req_bytes);
        send_stream
            .write_all(&frame)
            .await
            .map_err(|e| NetError::RequestFailed {
                peer,
                reason: format!("write request: {e}"),
            })?;

        // **Impl-time verification (spec §9 gap #2)**: noq-1.0.0-rc.0
        // `SendStream::finish()` is SYNC-fallible — returns
        // `Result<(), ClosedStream>` (not async). The plan's comment
        // said "if async, change to .await". No `.await` needed here.
        send_stream.finish().map_err(|e| NetError::RequestFailed {
            peer,
            reason: format!("finish send: {e}"),
        })?;

        // Spawn reader task that decodes incoming frames and pushes to channel.
        let (tx, rx) = tokio::sync::mpsc::channel(HEADS_STREAM_CHANNEL_CAPACITY);
        tokio::spawn(read_event_frames(recv_stream, tx));
        Ok(HeadsStream::new(rx))
    }

    fn install_request_handler(&self, handler: ArcRequestHandler) {
        let mut slot = self
            .request_handler
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *slot = Some(handler);
    }
}

/// Iroh-gossip-backed subscription.
///
/// Wraps a [`iroh_gossip::GossipTopic`] (a stream of
/// `Result<Event, ApiError>`), filters events to only surface
/// [`Event::Received`] payloads (decoded via canonical bincode),
/// maps [`Event::Lagged`] to [`SubError::Lagged(0)`] (count fidelity
/// lost — see spec §6), maps stream-level `ApiError` to
/// [`SubError::TransportError`] (potentially terminal — runtime counts
/// consecutive occurrences and halts; see B-4.3 spec §3.1),
/// maps bincode-decode failures to [`SubError::DecodeFailed`], and
/// silently consumes membership events ([`Event::NeighborUp`],
/// [`Event::NeighborDown`]).
///
/// Per B-4.1 spec §3.2 + B-4.3 spec §3.1.
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
                Some(Err(api_err)) => {
                    // Stream-level error from iroh-gossip mid-flight. Surface as
                    // TransportError (distinct from Lagged); the runtime counts
                    // consecutive TransportErrors and halts after a configurable
                    // threshold. Per B-4.3 spec §3.0 + §3.1.
                    //
                    // NOTE: iroh-gossip's `Event::Lagged` (the API event variant)
                    // is structurally different from this `ApiError` path — Lagged
                    // means "broadcast channel overrun, missed N messages"
                    // (recoverable via backfill); ApiError means "the gossip actor
                    // reported an error" (may be terminal). The Event::Lagged arm
                    // below stays mapped to SubError::Lagged(0).
                    return Err(SubError::TransportError(format!(
                        "iroh-gossip api error: {api_err}"
                    )));
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

/// Reader task spawned by [`IrohNetwork::request_heads`]. Decodes
/// length-prefixed canonical-bincode [`myrhiza_types::Event`] frames
/// from the recv side of a bidi stream until EOF or error.
///
/// Per B-4.4 spec §3.4.1.
///
/// **Impl-time verification (spec §9 gap #3)**: noq-1.0.0-rc.0
/// `RecvStream` exposes a native `read_exact(&mut [u8]) -> Result<(),
/// ReadExactError>` (`recv_stream.rs:89`) where `ReadExactError::FinishedEarly`
/// signals clean EOF (stream finished before all bytes — used here
/// when `buf.len()` == 4 and 0 bytes were read). The plan's code used
/// `tokio::io::AsyncReadExt::read_exact` which produces
/// `std::io::ErrorKind::UnexpectedEof`; the native API is used
/// instead to avoid the trait-impl layering and get the correct EOF
/// variant directly. `ReadExactError` lives in `iroh::endpoint`.
async fn read_event_frames(
    mut recv_stream: iroh::endpoint::RecvStream,
    tx: tokio::sync::mpsc::Sender<Result<myrhiza_types::Event, crate::request::HeadsStreamError>>,
) {
    use crate::request::{HeadsStreamError, MAX_FRAME_BYTES};
    use iroh::endpoint::ReadExactError;

    loop {
        let mut len_buf = [0u8; 4];
        match recv_stream.read_exact(&mut len_buf).await {
            Ok(()) => {}
            // FinishedEarly with 0 bytes read = clean EOF (responder
            // sent FIN after last event frame). FinishedEarly with
            // >0 bytes = truncated frame = transport error.
            Err(ReadExactError::FinishedEarly(0)) => return,
            Err(ReadExactError::FinishedEarly(n)) => {
                let _ = tx
                    .send(Err(HeadsStreamError::Transport(format!(
                        "truncated length prefix: got {n} of 4 bytes"
                    ))))
                    .await;
                return;
            }
            Err(e) => {
                let _ = tx
                    .send(Err(HeadsStreamError::Transport(format!(
                        "read length: {e}"
                    ))))
                    .await;
                return;
            }
        }

        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_FRAME_BYTES {
            let _ = tx
                .send(Err(HeadsStreamError::Transport(format!(
                    "frame too large: {len} bytes (max {MAX_FRAME_BYTES})"
                ))))
                .await;
            return;
        }

        let mut payload = vec![0u8; len];
        if let Err(e) = recv_stream.read_exact(&mut payload).await {
            let _ = tx
                .send(Err(HeadsStreamError::Transport(format!(
                    "read payload: {e}"
                ))))
                .await;
            return;
        }

        let event: myrhiza_types::Event = match canonical_bincode().deserialize(&payload) {
            Ok(e) => e,
            Err(e) => {
                let _ = tx
                    .send(Err(HeadsStreamError::Decode(format!("event decode: {e}"))))
                    .await;
                return;
            }
        };

        if tx.send(Ok(event)).await.is_err() {
            // Requester dropped the stream — stop reading.
            return;
        }
    }
}
