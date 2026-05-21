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
    ArcRequestHandler, HEADS_REQUEST_ALPN, HEADS_STREAM_CHANNEL_CAPACITY, HeadsResponder,
    HeadsStream, HeadsStreamError, build_length_prefixed_frame,
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

        // **Impl-time verification (spec §9 gap #2)**: iroh-1.0.0-rc.0
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

/// Read a single length-prefixed frame from an iroh [`RecvStream`].
///
/// Returns `Ok(Some(payload))` on a complete frame, `Ok(None)` on a
/// clean EOF before any bytes of the frame were read (i.e. at a
/// frame boundary), or `Err(HeadsStreamError::Transport(...))` on a
/// truncated read, oversized frame, or transport failure.
///
/// **Impl-time verification (spec §9 gap #3)**: iroh-1.0.0-rc.0
/// `RecvStream` exposes a native `read_exact(&mut [u8]) -> Result<(),
/// ReadExactError>` (`recv_stream.rs:89`) where
/// `ReadExactError::FinishedEarly(n)` signals partial EOF — `n == 0`
/// at the 4-byte length-prefix position means clean stream end;
/// `n > 0` means a truncated frame. The plan's code used
/// `tokio::io::AsyncReadExt::read_exact` which produces
/// `std::io::ErrorKind::UnexpectedEof`; the native API is used
/// instead to get the correct EOF variant directly and avoid the
/// `AsyncRead` trait-impl layering.
///
/// This helper is shared by both the outbound reader task
/// ([`read_event_frames`]) and the accept-side handler
/// ([`HeadsRequestProtocol::accept`]), eliminating the duplication
/// flagged in the Task-5 spec review.
///
/// Per B-4.4 spec §3.4.1 + §3.4.2.
pub(crate) async fn read_length_prefixed_frame_from_iroh_recv(
    stream: &mut iroh::endpoint::RecvStream,
) -> Result<Option<Vec<u8>>, HeadsStreamError> {
    use crate::request::MAX_FRAME_BYTES;
    use iroh::endpoint::ReadExactError;

    let mut len_buf = [0u8; 4];
    match stream.read_exact(&mut len_buf).await {
        Ok(()) => {}
        // FinishedEarly(0) at the length prefix = clean EOF at a frame
        // boundary. FinishedEarly(n>0) = truncated prefix = error.
        Err(ReadExactError::FinishedEarly(0)) => return Ok(None),
        Err(ReadExactError::FinishedEarly(n)) => {
            return Err(HeadsStreamError::Transport(format!(
                "truncated length prefix: got {n} of 4 bytes"
            )));
        }
        Err(e) => {
            return Err(HeadsStreamError::Transport(format!("read length: {e}")));
        }
    }

    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(HeadsStreamError::Transport(format!(
            "frame too large: {len} bytes (max {MAX_FRAME_BYTES})"
        )));
    }

    let mut payload = vec![0u8; len];
    match stream.read_exact(&mut payload).await {
        Ok(()) => {}
        Err(ReadExactError::FinishedEarly(n)) => {
            // Truncated payload — a clean EOF can only occur at a
            // length-prefix boundary, never mid-payload.
            return Err(HeadsStreamError::Transport(format!(
                "truncated payload: got {n} of {len} bytes"
            )));
        }
        Err(e) => {
            return Err(HeadsStreamError::Transport(format!("read payload: {e}")));
        }
    }

    Ok(Some(payload))
}

/// Reader task spawned by [`IrohNetwork::request_heads`]. Decodes
/// length-prefixed canonical-bincode [`myrhiza_types::Event`] frames
/// from the recv side of a bidi stream until EOF or error.
///
/// Uses [`read_length_prefixed_frame_from_iroh_recv`] for all frame
/// reads — no duplicated length-prefix logic here.
///
/// Per B-4.4 spec §3.4.1.
async fn read_event_frames(
    mut recv_stream: iroh::endpoint::RecvStream,
    tx: tokio::sync::mpsc::Sender<Result<myrhiza_types::Event, HeadsStreamError>>,
) {
    loop {
        let payload = match read_length_prefixed_frame_from_iroh_recv(&mut recv_stream).await {
            Ok(Some(p)) => p,
            Ok(None) => return, // clean EOF
            Err(e) => {
                let _ = tx.send(Err(e)).await;
                return;
            }
        };

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

// ---- Accept-side: HeadsRequestProtocol ProtocolHandler ----

impl IrohNetwork {
    /// Return an [`iroh::protocol::ProtocolHandler`] impl for the
    /// direct-stream `HeadsRequest` ALPN. Embedders register this on
    /// their [`iroh::protocol::Router`] at startup:
    ///
    /// ```ignore
    /// let router = iroh::protocol::Router::builder(endpoint)
    ///     .accept(iroh_gossip::ALPN, gossip.clone())
    ///     .accept(myrhiza_network::HEADS_REQUEST_ALPN,
    ///             network.protocol_handler())
    ///     .spawn();
    /// ```
    ///
    /// The returned handler shares the installed [`crate::request::RequestHandler`]
    /// with this `IrohNetwork` instance via an internal
    /// `Arc<Mutex<Option<_>>>`. Re-installing a handler via
    /// [`crate::Network::install_request_handler`] is immediately
    /// visible to the returned protocol handler (last-call-wins).
    ///
    /// Per B-4.4 spec §3.4.2 + §3.4.3.
    #[must_use]
    pub fn protocol_handler(&self) -> HeadsRequestProtocol {
        HeadsRequestProtocol {
            handler_slot: Arc::clone(&self.request_handler),
        }
    }
}

/// [`iroh::protocol::ProtocolHandler`] impl for [`HEADS_REQUEST_ALPN`].
///
/// Accepts an inbound bidi stream, reads the length-prefixed
/// [`DirectHeadsRequest`] frame, looks up the installed
/// [`crate::request::RequestHandler`], and writes response
/// [`myrhiza_types::Event`] frames back over the send side.
///
/// If no handler is installed, the send stream is closed cleanly
/// (requester sees zero events + EOF).
///
/// **Impl-time verification (spec §9 gate #1)**:
/// `iroh::endpoint::Connection::remote_id()` returns `EndpointId`
/// **directly** (infallible, not `Result<EndpointId, _>`), confirmed
/// against `iroh-1.0.0-rc.0/src/endpoint/connection.rs:1101`. The
/// plan's draft used `.map_err(...)` on the call site; that is
/// incorrect and has been removed.
///
/// **Impl-time verification (spec §9 gate #4)**:
/// `iroh::protocol::ProtocolHandler` requires `std::fmt::Debug`
/// (trait bound: `Send + Sync + std::fmt::Debug + 'static`); `Debug`
/// is derived here. The `accept` signature is:
/// `fn accept(&self, connection: Connection) -> impl Future<Output =
/// Result<(), AcceptError>> + Send` — matches the plan exactly.
///
/// **`AcceptError::from_err` constructor**: takes
/// `T: std::error::Error + Send + Sync + 'static`. `String` does not
/// implement `std::error::Error`; error messages are wrapped via
/// `std::io::Error::other(msg)` (available since Rust 1.74) which
/// converts to `AcceptError` through the `From<std::io::Error>`
/// blanket impl on `AcceptError`.
///
/// Per B-4.4 spec §3.4.2.
/// **Debug note**: `ArcRequestHandler` is a `dyn RequestHandler` trait
/// object which does not require `Debug`. The `Debug` impl is written
/// manually to satisfy `iroh::protocol::ProtocolHandler`'s
/// `std::fmt::Debug` bound without adding `Debug` to the
/// `RequestHandler` trait (which would be an unnecessary constraint on
/// embedder implementations).
#[derive(Clone)]
pub struct HeadsRequestProtocol {
    handler_slot: Arc<Mutex<Option<ArcRequestHandler>>>,
}

impl std::fmt::Debug for HeadsRequestProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let has_handler = self
            .handler_slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some();
        f.debug_struct("HeadsRequestProtocol")
            .field("has_handler", &has_handler)
            .finish()
    }
}

impl iroh::protocol::ProtocolHandler for HeadsRequestProtocol {
    fn accept(
        &self,
        connection: iroh::endpoint::Connection,
    ) -> impl std::future::Future<Output = Result<(), iroh::protocol::AcceptError>> + Send {
        let handler_slot = Arc::clone(&self.handler_slot);
        async move {
            // **Impl-time verification (spec §9 gate #1)**:
            // `Connection::remote_id()` is infallible in iroh-1.0.0-rc.0
            // (returns `EndpointId` directly, not `Result<_, _>`).
            // Confirmed at iroh-1.0.0-rc.0 endpoint/connection.rs:1101.
            let requester_id = connection.remote_id();
            let requester = peer_pubkey_from_iroh(requester_id);

            let (mut send_stream, mut recv_stream) = connection
                .accept_bi()
                .await
                .map_err(|e| std::io::Error::other(format!("accept_bi: {e}")))?;

            // Read the request frame using the shared helper so there
            // is no duplicated length-prefix logic.
            let req_bytes = match read_length_prefixed_frame_from_iroh_recv(&mut recv_stream).await
            {
                Ok(Some(b)) => b,
                Ok(None) => {
                    // Clean EOF before any request bytes — close
                    // our send side with FIN (not RESET_STREAM)
                    // for symmetry with the no-handler path.
                    let _ = send_stream.finish();
                    // Keep the connection alive until the requester closes
                    // from their side. Without this await, dropping the
                    // `connection` handle here would send CONNECTION_CLOSE
                    // before the FIN frame is processed by the requester,
                    // causing a "connection lost" error instead of clean EOF.
                    // Pattern from iroh's own Echo example (protocol.rs §tests).
                    connection.closed().await;
                    return Ok(());
                }
                Err(e) => {
                    return Err(std::io::Error::other(format!("read request: {e}")).into());
                }
            };

            let request: DirectHeadsRequest = canonical_bincode()
                .deserialize(&req_bytes)
                .map_err(|e| std::io::Error::other(format!("decode request: {e}")))?;

            // Snapshot the Arc so the Mutex is not held across the
            // handler invocation.
            let installed = {
                let lock = handler_slot
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                lock.clone()
            };

            let Some(handler) = installed else {
                // No handler installed — close stream cleanly.
                // Requester sees zero events + EOF.
                let _ = send_stream.finish();
                // Keep connection alive until requester closes (same
                // reasoning as the clean-EOF-before-request path above).
                connection.closed().await;
                return Ok(());
            };

            // Spawn the handler task with a fresh channel.  The
            // forward loop below reads from the channel and writes
            // each event as a length-prefixed frame to the QUIC send
            // stream.
            let (tx, mut rx) = tokio::sync::mpsc::channel::<
                Result<myrhiza_types::Event, HeadsStreamError>,
            >(HEADS_STREAM_CHANNEL_CAPACITY);
            let responder = HeadsResponder::new(tx);
            let handler_task = tokio::spawn(async move {
                handler.handle(requester, request, responder).await;
                // Responder drops here -> channel closes -> rx.recv()
                // returns None -> forward loop below exits cleanly.
            });

            while let Some(item) = rx.recv().await {
                match item {
                    Ok(event) => {
                        let bytes = match canonical_bincode().serialize(&event) {
                            Ok(b) => b,
                            Err(e) => {
                                handler_task.abort();
                                return Err(
                                    std::io::Error::other(format!("encode event: {e}")).into()
                                );
                            }
                        };
                        let frame = build_length_prefixed_frame(&bytes);
                        if send_stream.write_all(&frame).await.is_err() {
                            // Requester dropped mid-stream — abort the
                            // handler and return Ok (from our side the
                            // stream is done).
                            handler_task.abort();
                            return Ok(());
                        }
                    }
                    Err(_handler_err) => {
                        // Handler signalled an error via the channel.
                        // Abort the spawned task explicitly — a buggy
                        // handler that sends Err but keeps running
                        // would otherwise block the `handler_task.await`
                        // below indefinitely. Mirrors the abort sites
                        // on the encode-error and write-error paths.
                        handler_task.abort();
                        break;
                    }
                }
            }

            // Wait for the handler task to drain (or confirm it was aborted),
            // then close the send side cleanly and keep the connection alive
            // until the requester closes from their side.
            let _ = handler_task.await;
            let _ = send_stream.finish();
            // Keep the connection alive until the requester closes from their
            // side (same FIN-vs-CONNECTION_CLOSE race fix as above).
            connection.closed().await;
            Ok(())
        }
    }
}
