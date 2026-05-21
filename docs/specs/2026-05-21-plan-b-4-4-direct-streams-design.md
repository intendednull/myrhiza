**Date:** 2026-05-21
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-4.4 — HeadsRequest direct-streams (network-layer plumbing)

# Plan B-4.4 design — Direct-stream `request_heads` plumbing

## 1. Goal

Pay off the next deferral in the B-4 sequence: **point-to-point HeadsRequest delivery via a dedicated ALPN**, replacing the current "broadcast `HeadsRequest` over the gossip topic and let every peer see it" design with a direct-streamed exchange between requester and the specific peer that signed the relevant `HeadsSummary`.

The current backfill path (per `crates/kernel/src/runtime.rs:872-933` in `handle_heads_summary`, and `runtime.rs:1208-1234` in `handle_heads_request`):

1. Peer A publishes a signed `HeadsSummary` on the gossip topic.
2. Peer B detects a gap, builds a `HeadsRequest`, publishes it via `Network::publish(topic, GossipMessage::HeadsRequest(req))` — broadcast to **every** subscriber on the topic.
3. Peer A receives the broadcast `HeadsRequest`, services it by **broadcasting** the requested events back via `publish(topic, GossipMessage::Event(e))`.

This is wasteful and lacks attribution: the response storms every peer instead of going only to the requester, the requester cannot bound the response by sender identity, and the responder cannot rate-limit per-requester.

B-4.4 fixes the plumbing only:

1. **New ALPN `b"myrhiza/heads-request/1"`** registered against an `iroh::protocol::Router`. Peers exchange backfill traffic over a dedicated QUIC bidi stream, not gossip.
2. **New trait method `Network::request_heads(peer, request) -> HeadsStream`** on the requester side. Returns a typed stream of `Result<Event, ...>` items terminated when the responder closes the stream.
3. **New trait method `Network::install_request_handler(handler)`** for the accept side. The embedder installs a handler at construction time; both `IrohNetwork` and `MemNetwork` route inbound requests through the registered handler.
4. **New wire shape `DirectHeadsRequest`** = `{ topic, requests }`. No peer signature (mutual QUIC auth on iroh, in-process trust on MemNetwork). Length-prefixed frames on the wire.
5. **Two new acceptance-test modules** verifying end-to-end direct-stream delivery over both `MemNetwork` (in-process) and `IrohNetwork` (real iroh-gossip Router with the new ALPN registered).
6. **No runtime integration in B-4.4.** The kernel `Runtime` continues to use the existing gossip-routed `HeadsRequest` path. B-4.5 wires the new direct-stream path into `Runtime::handle_heads_summary` and removes the gossip-routed handler.

**Out of scope (deferred to B-4.5 or later)**:

- **Kernel runtime integration.** B-4.5 will:
  - Construct a `HeadsRequestHandler` impl with DAG access (likely via mpsc back to the runtime task).
  - Replace `publish(GossipMessage::HeadsRequest(req))` call sites with `request_heads(target_peer, req)` calls (one per backfill site at `runtime.rs:973` and `runtime.rs:1039`).
  - Remove or no-op `handle_heads_request` (the gossip-routed handler at `runtime.rs:1208`).
  - Validate end-to-end backfill via the new path in a multi-peer integration test.
- **Removal of `GossipMessage::HeadsRequest` variant.** B-4.4 leaves it in place (preserves wire-freeze stability). B-4.5 decides whether to deprecate or remove. Removal is a wire-freeze regen.
- **Cross-process tests.** Real-cross-process spawning needs a test harness (separate binary + tokio::process API) that's its own slice; in-process two-`IrohNetwork`-peers tests give the same protocol coverage at much lower complexity.
- **Backpressure on the response stream.** The responder writes events as fast as it can. The QUIC stream's flow control bounds throughput at the transport layer. Application-layer rate-limiting (token bucket on the responder side) is a future ergonomics concern.
- **Discovery / pkarr / DHT integration.** `request_heads(peer, req)` assumes the requester knows the target peer's `EndpointId`. The signed `HeadsSummary` already provides `signed_by_peer` — the natural source for the target. Discovery for arbitrary lookups is future work (B-4.6+ per B-4.2 §10).

## 2. Scope decisions (locked during brainstorming + iroh-gossip / iroh::protocol survey, 2026-05-21)

| Decision | Chosen | Runner-up | Why |
|---|---|---|---|
| **Strict B-4.4 = plumbing only; runtime switchover = B-4.5** | Land the network-layer trait surface, IrohNetwork impl, MemNetwork impl, and network-tier acceptance tests in this slice. Defer runtime integration to a separate slice. | Single slice that lands plumbing + runtime switchover. | Plumbing alone is already substantial: new ALPN, new wire types, new trait methods, MemNetwork peer-keyed refactor, IrohNetwork connect + accept code paths. Bundling runtime integration would mix "infrastructure exists" with "infrastructure is used" — review surface doubles. Splitting matches the B-4.0/4.1/4.2/4.3 pattern (each slice landed one concern). |
| **New trait method on `Network`, not a sibling trait** | `Network` gains two methods: `request_heads(peer, req) -> HeadsStream` (outbound) and `install_request_handler(handler)` (accept-side registration). | (a) Separate `DirectRequest` trait; (b) Direct-stream surface on `IrohNetwork` impl-only, no trait. | Tests need to exercise both sides over `MemNetwork`. Putting the surface on the `Network` trait keeps both impls behind one abstraction; the kernel can swap impls in tests vs production without touching code that issues backfill requests. Separate trait would force kernel-side `dyn Network + dyn DirectRequest` dispatch — extra surface for no benefit. Impl-only on `IrohNetwork` would block kernel-tier acceptance tests. |
| **Handler shape: trait, registered post-construction via `install_request_handler`** | `RequestHandler` is an async trait. `Network::install_request_handler(Arc<dyn RequestHandler>)` installs it; idempotent — last call wins. Impls may panic on double-install if explicit. | (a) Constructor-time handler (`IrohNetwork::new(ep, gossip, handler)`); (b) Closure-based registration | Constructor-time forces a circular construction: the kernel runtime owns the network *and* provides the handler. Solving this in Rust needs `Arc::new_cyclic` or `OnceLock`; post-construction registration is straightforward. Closures lose async support without boxing per-call; an async trait is cleaner. Concrete construction order: (1) construct Network, (2) construct Runtime with the Network, (3) Runtime calls `network.install_request_handler(self.heads_handler.clone())`. B-4.5 lands step 3. |
| **Wire shape: `DirectHeadsRequest { topic, requests }` — no signature, no `signed_by_peer`** | Length-prefixed canonical bincode frame. Topic carried in the payload; the responder validates topic membership before serving. No peer signature: mutual QUIC auth replaces it on `IrohNetwork`; in-process trust replaces it on `MemNetwork`. | Reuse existing `HeadsRequest { signed_by_peer, signature, requests }`. | The B-4.2 signature exists because Plumtree forwarding hides the publisher — a *forwarded* gossip message has no inherent attribution. Direct-stream attribution comes from the QUIC TLS handshake: the responder reads the connection's `remote_id()` and knows exactly who's asking. Carrying a redundant signature is dead bytes; verifying it would be redundant work. The signed-only `topic` field on `HeadsRequestSignedPayload` exists for cross-topic replay defense; in direct-streams the topic is bound to the request payload, which is bound to one stream, on one ALPN — no cross-topic replay vector. |
| **Response shape: framed `Event` items, terminator = stream close** | Responder writes `(u32 BE length, canonical-bincode Event bytes)` frames until it has served the request, then closes the SendStream half. Requester reads frames until EOF. No explicit `Done` marker. | (a) Explicit `HeadsStreamItem { Event(Event), Done }` enum; (b) Newline-delimited JSON | QUIC streams have native EOF semantics (`finish()` on the SendStream signals "no more data"). Adding an explicit `Done` variant duplicates QUIC's signal at the application layer for no gain. Length-prefix framing is the canonical pattern for binary streams; reading until EOF terminates cleanly. The error path — responder hits an internal error mid-stream — is "close the stream early"; requester sees premature EOF and surfaces the partial data + a "stream ended early" warning. |
| **`HeadsStream` shape: typed struct wrapping `mpsc::Receiver<Result<Event, _>>`** | `pub struct HeadsStream { rx: mpsc::Receiver<Result<Event, HeadsStreamError>> }` with `async fn next(&mut self) -> Option<Result<Event, HeadsStreamError>>` mirroring `Subscription::recv` shape. Sender lives in a spawned reader task that decodes frames from the QUIC SendStream. | `impl Stream` from futures-util. | Concrete type avoids `impl Stream` constraints on the trait surface (which forces `dyn`-unsafe trait or a boxed future per call). Matches the existing `Subscription` pattern in this crate (recv-style polling). `futures-util` is already a workspace dep so the cost is neutral; the gain is consistency with sibling code. |
| **`MemNetwork` gains `peer_pubkey` field** | `MemNetwork::new(bus, peer_pubkey)` — peer identity required at construction. The bus gains a `request_handlers: Mutex<BTreeMap<PeerPubkey, Arc<dyn RequestHandler>>>` registry keyed by peer. `install_request_handler` registers `(self.peer_pubkey, handler)`. `request_heads(target, req)` looks up `target` in the bus. | (a) Default-zero pubkey + lazy registration; (b) Separate per-peer wire | Adding a peer-identity field reflects what MemNetwork models (one peer's view of the bus); the previous shape was sufficient only because the in-mem bus had no point-to-point semantics. Tests that constructed `MemNetwork::new(bus)` get one extra arg (about 30 call sites; mechanical sweep). Zero-pubkey default is a silent footgun (two peers both default to zero, request routing collides). Separate per-peer wire would split the bus into a peer-direct registry sibling to topic broadcasts — same end state, more code. |
| **`MemNetwork::request_heads` runs the handler in-process synchronously** | The MemNetwork direct-stream impl looks up the target's handler in the bus registry, spawns it on `tokio::spawn`, and connects the handler's output channel to the returned `HeadsStream`. No actual stream over the wire — the bus acts as the transport. | Run handler inline (no spawn) | Spawning preserves the "stream items arrive over time" semantics that `IrohNetwork` exhibits — the requester can call `next()` and yield to the executor between items. Inline execution would cause `request_heads` to block until the handler completes before returning the stream — the test surface diverges from production. The spawned task closes its sender when done, mirroring `IrohNetwork`'s SendStream::finish() signal. |
| **ALPN bytes: `b"myrhiza/heads-request/1"`** | Versioned per n0 convention (`prior-art/iroh/architecture.md` §"ALPN-based protocol multiplexing"). The `/1` suffix carries the protocol version; bumping it lets B-4.5 or beyond decide whether to accept both versions during a rollout. | (a) Unversioned `b"myrhiza/heads-request"`; (b) `myrhiza-heads-request/1.0` | n0's convention is `<name>/<version>`. Following it preserves the option to register multiple versions on the same Router during a rollout. The dash variant (`myrhiza-heads-request`) is the namespace-style convention; we use `/` to separate version cleanly. Single-digit version is sufficient — semver-style minor bumps don't add value when ALPN matching is exact-string. |
| **Iroh protocol-handler registration is embedder-driven, not network-driven** | `IrohNetwork::install_request_handler` stores the handler in an `Arc<OnceLock<Arc<dyn RequestHandler>>>` interior field. The embedder separately calls `IrohNetwork::protocol_handler()` to get a `iroh::protocol::ProtocolHandler` impl, then registers it on its `Router`. | Network owns Router registration internally. | The existing IrohNetwork pattern keeps the `Router` caller-owned (per `iroh_transport.rs:73-80` rustdoc); the embedder constructs the Router with all desired ALPNs in one place. Forcing IrohNetwork to register itself would invert ownership and break the existing convention. The `protocol_handler()` method returns a struct that reads from the SendStream/RecvStream pair, decodes the request, looks up the handler in the OnceLock, and invokes it. |
| **No retry, no timeout** on `request_heads` outbound | The method returns whatever the QUIC connection + handler produce. If the connection drops, the stream closes; if the handler errors, the stream surfaces it. The kernel runtime is responsible for retry policy (B-4.5 scope). | Built-in retry / timeout. | Retry / timeout policy lives in the consumer, not the transport. The kernel's drift-handler ratelimit pattern (`RateLimit::try_emit`) is the existing precedent for "policy in runtime, mechanism in transport." Adding retry here would force the runtime to express "no, don't retry" via a separate code path. |
| **Test runtime: `multi_thread`, `worker_threads = 2`** for iroh-driven tests | Same as B-4.1/4.2 tests — iroh spawns tasks; current-thread deadlocks. MemNetwork tests can use default flavor since the bus's tokio broadcast machinery is the only async surface. | Single-threaded for all. | Documented in B-4.1 spec; preserve consistency. |

## 3. Code surface

### 3.0 `DirectHeadsRequest` wire type — `crates/types/src/dag.rs`

Add adjacent to the existing `HeadsRequest`:

```rust
/// Direct-stream variant of [`HeadsRequest`] sent over a dedicated
/// ALPN-multiplexed QUIC bidi stream (per B-4.4 spec §1).
///
/// **Distinguishing from [`HeadsRequest`]:** the gossip-routed
/// `HeadsRequest` carries `signed_by_peer` + `signature` because
/// Plumtree forwarding hides the original publisher (B-4.2 §3.0).
/// Direct-stream has no such issue — mutual QUIC TLS authenticates the
/// requester; the topic is carried in the request payload and bound to
/// one stream on one ALPN, so cross-topic replay has no vector.
///
/// **Wire layout (canonical bincode v1, normative)**:
///   1. `topic`: `Topic` (serde_bytes_32_pub, 40 bytes)
///   2. `requests`: `Vec<EventRequest>` (length-prefixed sequence)
///
/// Field order is normative — emitter and verifier MUST encode fields
/// in declaration order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectHeadsRequest {
    /// Topic this request applies to. The responder MUST verify it
    /// services this topic before serving events.
    pub topic: Topic,
    /// Range requests included in this request. Same bounded-by-256
    /// semantics as [`HeadsRequest::requests`].
    pub requests: Vec<EventRequest>,
}
```

Round-trip test added next to the existing `tests_drift_heads` module.

Wire-freeze: add `DirectHeadsRequest` snapshot to `crates/types/tests/wire_freeze.rs` with a `[topic=0xAB; 32]` + single-`EventRequest` sample.

### 3.1 ALPN constant + handler trait — `crates/network/src/request.rs` (new file)

New module `request` under `crates/network/src/`:

```rust
//! Direct-stream request/response surface for HeadsRequest backfill.
//!
//! Per B-4.4 spec §3.1.

use myrhiza_types::{DirectHeadsRequest, Event, PeerPubkey};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;

/// ALPN registered against `iroh::protocol::Router` for direct-stream
/// HeadsRequest. Per B-4.4 spec §2.
pub const HEADS_REQUEST_ALPN: &[u8] = b"myrhiza/heads-request/1";

/// Errors surfaced through [`HeadsStream::next`].
#[derive(Debug, Error)]
pub enum HeadsStreamError {
    /// The underlying transport failed mid-stream (QUIC reset, peer
    /// dropped, etc.). Carries a human-readable diagnostic.
    #[error("transport error: {0}")]
    Transport(String),
    /// A frame on the stream did not decode under the canonical bincode
    /// contract. Stream is terminated.
    #[error("decode failed: {0}")]
    Decode(String),
    /// The handler reported an internal error before the stream
    /// completed.
    #[error("handler error: {0}")]
    Handler(String),
}

/// Receive-side stream of [`Event`] items returned by
/// [`Network::request_heads`]. Polls until `None` (responder closed
/// cleanly) or the next error.
///
/// Per B-4.4 spec §2 "HeadsStream shape".
pub struct HeadsStream {
    rx: mpsc::Receiver<Result<Event, HeadsStreamError>>,
}

impl HeadsStream {
    /// Construct from an mpsc receiver. Crate-private — callers reach
    /// this via [`crate::Network::request_heads`].
    pub(crate) fn new(rx: mpsc::Receiver<Result<Event, HeadsStreamError>>) -> Self {
        Self { rx }
    }

    /// Receive the next event in the stream.
    ///
    /// - `Some(Ok(event))` — next event in the response sequence.
    /// - `Some(Err(_))` — an error occurred; the stream is terminated
    ///   and subsequent calls will return `None`.
    /// - `None` — responder closed cleanly; no further events.
    pub async fn next(&mut self) -> Option<Result<Event, HeadsStreamError>> {
        self.rx.recv().await
    }
}

/// Sender half of a HeadsStream, used by [`RequestHandler`]
/// implementations to write the response.
///
/// Each call to `send` pushes one [`Event`] onto the response stream.
/// Drop the responder to signal "no more events" (clean EOF).
pub struct HeadsResponder {
    tx: mpsc::Sender<Result<Event, HeadsStreamError>>,
}

impl HeadsResponder {
    /// Construct from an mpsc sender. Crate-private — callers reach
    /// this via [`RequestHandler::handle`].
    pub(crate) fn new(tx: mpsc::Sender<Result<Event, HeadsStreamError>>) -> Self {
        Self { tx }
    }

    /// Push an event onto the response stream. Returns `Err` if the
    /// requester dropped the stream before consuming this event (in
    /// which case the handler should stop producing).
    ///
    /// # Errors
    /// Returns `()` on receiver-dropped (request canceled).
    pub async fn send(&self, event: Event) -> Result<(), ()> {
        self.tx.send(Ok(event)).await.map_err(|_| ())
    }
}

/// Handler invoked on the responder side when a direct-stream request
/// arrives. The handler validates the request, queries its DAG, and
/// pushes events through [`HeadsResponder::send`].
///
/// Per B-4.4 spec §2 "Handler shape".
#[async_trait::async_trait]
pub trait RequestHandler: Send + Sync + 'static {
    /// Service a direct-stream request from `requester`. The handler
    /// pushes response events through `responder`. Returning closes
    /// the stream cleanly; pushing through `responder` with an
    /// `Err(HeadsStreamError)` surfaces the error to the requester
    /// before close.
    ///
    /// **Topic validation**: the handler MUST verify it services
    /// `request.topic` before pushing events. The trait does not
    /// enforce this — handlers that ignore topic gate become a
    /// confused-deputy risk.
    async fn handle(
        &self,
        requester: PeerPubkey,
        request: DirectHeadsRequest,
        responder: HeadsResponder,
    );
}

/// Convenient `Arc`'d trait object for [`RequestHandler`].
pub type ArcRequestHandler = Arc<dyn RequestHandler>;
```

The mpsc capacity is fixed at a constant `HEADS_STREAM_CHANNEL_CAPACITY = 32` (a reasonable default for batched event transfer; not configurable in B-4.4).

### 3.2 `Network` trait extension — `crates/network/src/lib.rs`

Add to the existing trait:

```rust
#[async_trait::async_trait]
pub trait Network: Send + Sync + 'static {
    // ... existing subscribe / publish / unsubscribe unchanged ...

    /// Issue a direct-stream HeadsRequest to a specific peer.
    ///
    /// Returns a [`HeadsStream`] that yields response events as they
    /// arrive. Stream terminates with `None` when the responder closes
    /// cleanly, or with an `Err` for transport / decode / handler
    /// failures.
    ///
    /// For transports without point-to-point semantics, this is the
    /// only correct backfill primitive. The gossip-routed
    /// [`GossipMessage::HeadsRequest`] variant is retained for
    /// wire-freeze stability but is being deprecated in favor of this
    /// method (B-4.5 will switch the kernel runtime over).
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

    /// Install a [`RequestHandler`] for the accept side of direct-stream
    /// requests. Idempotent — last call wins.
    ///
    /// Embedders construct the handler with whatever state it needs
    /// (DAG access, topic filter, rate limiter) and install it on the
    /// network at startup. Without an installed handler, inbound
    /// direct-stream requests are silently rejected.
    ///
    /// Per B-4.4 spec §3.2.
    fn install_request_handler(&self, handler: ArcRequestHandler);
}
```

Add to `NetError`:

```rust
/// Direct-stream request to a peer failed to establish (dial error,
/// ALPN refusal, peer unreachable). Per B-4.4 spec §3.2.
#[error("request to peer {peer:?} failed: {reason}")]
RequestFailed {
    /// The target peer that the request was directed at.
    peer: PeerPubkey,
    /// Human-readable diagnostic.
    reason: String,
},
```

Re-export `request` module's public items at the crate root:

```rust
pub mod request;
pub use request::{
    HeadsRequestHandler, HeadsResponder, HeadsStream, HeadsStreamError,
    ArcRequestHandler, HEADS_REQUEST_ALPN,
};
```

### 3.3 `MemNetwork` peer-keyed refactor + direct-stream impl — `crates/network/src/memory.rs`

Add `peer_pubkey` field to `MemNetwork`:

```rust
#[derive(Clone)]
pub struct MemNetwork {
    bus: Arc<MemBus>,
    peer_pubkey: PeerPubkey,
}

impl MemNetwork {
    /// Construct a new handle on the given shared [`MemBus`] with the
    /// specified `peer_pubkey`. Per B-4.4 spec §3.3.
    #[must_use]
    pub fn new(bus: Arc<MemBus>, peer_pubkey: PeerPubkey) -> Self {
        Self { bus, peer_pubkey }
    }

    /// Return the peer pubkey this `MemNetwork` was constructed with.
    #[must_use]
    pub fn peer_pubkey(&self) -> PeerPubkey {
        self.peer_pubkey
    }
}
```

Extend `MemBus` with the direct-stream registry:

```rust
pub struct MemBus {
    topics: Mutex<BTreeMap<Topic, TopicState>>,
    request_handlers: Mutex<BTreeMap<PeerPubkey, ArcRequestHandler>>,
    capacity_per_topic: usize,
}
```

Add to `MemBus::new`:

```rust
pub fn new(capacity: usize) -> Arc<Self> {
    Arc::new(Self {
        topics: Mutex::new(BTreeMap::new()),
        request_handlers: Mutex::new(BTreeMap::new()),
        capacity_per_topic: capacity,
    })
}
```

Implement `request_heads` + `install_request_handler` on `MemNetwork`:

```rust
async fn request_heads(
    &self,
    peer: PeerPubkey,
    request: DirectHeadsRequest,
) -> Result<HeadsStream, NetError> {
    let handler = {
        let handlers = self.bus.request_handlers.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        handlers.get(&peer).cloned()
    };
    let Some(handler) = handler else {
        return Err(NetError::RequestFailed {
            peer,
            reason: "no handler registered for target peer".to_string(),
        });
    };
    let (tx, rx) = tokio::sync::mpsc::channel(crate::request::HEADS_STREAM_CHANNEL_CAPACITY);
    let responder = HeadsResponder::new(tx);
    let requester = self.peer_pubkey;
    tokio::spawn(async move {
        handler.handle(requester, request, responder).await;
        // Responder drops here — closes the channel.
    });
    Ok(HeadsStream::new(rx))
}

fn install_request_handler(&self, handler: ArcRequestHandler) {
    let mut handlers = self.bus.request_handlers.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    handlers.insert(self.peer_pubkey, handler);
}
```

**Call-site sweep**: every `MemNetwork::new(bus)` becomes `MemNetwork::new(bus, peer_pubkey)`. Search-and-replace candidates:

- `crates/kernel/tests/attribution.rs` (multiple call sites)
- `crates/kernel/tests/convergence.rs` (multiple)
- `crates/kernel/tests/halt_detection.rs`
- `crates/kernel/src/runtime.rs` (any internal construction)
- `crates/network/tests/memory.rs` (existing MemNetwork tests)

Tests that don't care about peer identity pass an arbitrary fixed key like `PeerPubkey::from_bytes([0xA1; 32])` or use a per-peer constant. The plan task that does this sweep enumerates each site.

### 3.4 `IrohNetwork` outbound + accept — `crates/network/src/iroh_transport.rs`

#### 3.4.1 Outbound: `IrohNetwork::request_heads`

```rust
async fn request_heads(
    &self,
    peer: PeerPubkey,
    request: DirectHeadsRequest,
) -> Result<HeadsStream, NetError> {
    let target_id = iroh_endpoint_id_from_peer_pubkey(peer)
        .map_err(|e| NetError::RequestFailed {
            peer,
            reason: format!("invalid target pubkey: {e}"),
        })?;
    let connection = self.endpoint
        .connect(target_id, HEADS_REQUEST_ALPN)
        .await
        .map_err(|e| NetError::RequestFailed {
            peer,
            reason: format!("connect: {e}"),
        })?;
    let (mut send_stream, recv_stream) = connection
        .open_bi()
        .await
        .map_err(|e| NetError::RequestFailed {
            peer,
            reason: format!("open_bi: {e}"),
        })?;
    // Encode + write the request.
    let req_bytes = canonical_bincode().serialize(&request)
        .map_err(|e| NetError::RequestFailed {
            peer,
            reason: format!("encode request: {e}"),
        })?;
    let frame = build_length_prefixed_frame(&req_bytes);
    send_stream.write_all(&frame).await
        .map_err(|e| NetError::RequestFailed {
            peer,
            reason: format!("write request: {e}"),
        })?;
    send_stream.finish()
        .map_err(|e| NetError::RequestFailed {
            peer,
            reason: format!("finish send: {e}"),
        })?;
    // Spawn a reader task that decodes incoming frames and pushes to the channel.
    let (tx, rx) = tokio::sync::mpsc::channel(crate::request::HEADS_STREAM_CHANNEL_CAPACITY);
    tokio::spawn(read_event_frames(recv_stream, tx));
    Ok(HeadsStream::new(rx))
}

fn install_request_handler(&self, handler: ArcRequestHandler) {
    if self.request_handler.set(handler).is_err() {
        // Already set — last call wins per trait contract. Replace
        // by clearing first. OnceLock can't be reset; use Mutex<Option<Arc>>.
        // (See §3.4.3 — interior field is actually `Mutex<Option<Arc<dyn RequestHandler>>>`.)
    }
}
```

Decision lock: `request_handler` field uses `Mutex<Option<ArcRequestHandler>>`, not `OnceLock`, because the trait contract states "idempotent — last call wins."

#### 3.4.2 Accept side: `IrohNetwork::protocol_handler`

```rust
impl IrohNetwork {
    /// Return an [`iroh::protocol::ProtocolHandler`] implementation for
    /// the direct-stream HeadsRequest ALPN. Embedders register this on
    /// their [`iroh::protocol::Router`] at startup.
    ///
    /// The returned handler shares state (the installed
    /// [`RequestHandler`]) with this `IrohNetwork` instance via an
    /// internal `Arc`.
    #[must_use]
    pub fn protocol_handler(&self) -> HeadsRequestProtocol {
        HeadsRequestProtocol {
            handler: self.request_handler.clone(),
            local_peer: self.peer_pubkey,
        }
    }
}

#[derive(Clone)]
pub struct HeadsRequestProtocol {
    handler: Arc<Mutex<Option<ArcRequestHandler>>>,
    local_peer: PeerPubkey,
}

impl iroh::protocol::ProtocolHandler for HeadsRequestProtocol {
    fn accept(&self, connection: iroh::endpoint::Connection)
        -> impl Future<Output = Result<(), iroh::protocol::AcceptError>> + Send
    {
        let handler = self.handler.clone();
        async move {
            let requester_id = connection.remote_id()
                .map_err(/* convert to AcceptError */ )?;
            let requester = peer_pubkey_from_iroh(requester_id);
            let (mut send_stream, mut recv_stream) = connection.accept_bi().await
                .map_err(/* ... */)?;
            // Read length-prefixed request frame.
            let req_bytes = read_length_prefixed_frame(&mut recv_stream).await?;
            let request: DirectHeadsRequest = canonical_bincode()
                .deserialize(&req_bytes)
                .map_err(/* convert to AcceptError */)?;
            // Dispatch to installed handler, or close stream if none.
            let installed = {
                let lock = handler.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                lock.clone()
            };
            let Some(handler) = installed else {
                // No handler — close stream early (clean EOF from responder).
                send_stream.finish().ok();
                return Ok(());
            };
            // Run handler with a responder bound to a channel; forward
            // each event onto the QUIC SendStream.
            let (tx, mut rx) = tokio::sync::mpsc::channel(
                crate::request::HEADS_STREAM_CHANNEL_CAPACITY
            );
            let responder = HeadsResponder::new(tx);
            let handler_task = tokio::spawn(async move {
                handler.handle(requester, request, responder).await;
            });
            // Forward channel items to the QUIC stream.
            while let Some(item) = rx.recv().await {
                match item {
                    Ok(event) => {
                        let bytes = canonical_bincode().serialize(&event)
                            .map_err(/* ... */)?;
                        let frame = build_length_prefixed_frame(&bytes);
                        if send_stream.write_all(&frame).await.is_err() {
                            // Requester dropped — abort handler.
                            handler_task.abort();
                            return Ok(());
                        }
                    }
                    Err(_) => {
                        // Handler reported error — close stream early.
                        break;
                    }
                }
            }
            // Wait for handler task to finish, then finish the send half.
            handler_task.await.ok();
            send_stream.finish().ok();
            Ok(())
        }
    }
}
```

Exact `iroh::protocol::ProtocolHandler` trait signatures will need to be confirmed against iroh 1.0.0-rc.0; the impl skeleton above reflects the documented shape in `prior-art/iroh/architecture.md` §"ALPN-based protocol multiplexing".

#### 3.4.3 New `IrohNetwork` field

```rust
pub struct IrohNetwork {
    endpoint: iroh::Endpoint,
    gossip: iroh_gossip::Gossip,
    peer_pubkey: PeerPubkey,
    /// Installed direct-stream request handler. Set via
    /// [`Network::install_request_handler`]; consumed by
    /// [`HeadsRequestProtocol::accept`] on inbound requests.
    /// Per B-4.4 spec §3.4.3.
    request_handler: Arc<Mutex<Option<ArcRequestHandler>>>,
}

impl IrohNetwork {
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
}
```

### 3.5 Length-prefix framing helpers — `crates/network/src/request.rs`

```rust
/// Maximum size of a single framed message (request or event). Bounds
/// memory pressure on the read side. 4 MiB is generous for events;
/// HeadsRequest payloads are tiny in practice.
const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

/// Build a length-prefixed frame: `(u32 BE length, payload bytes)`.
pub(crate) fn build_length_prefixed_frame(payload: &[u8]) -> Vec<u8> {
    let len = u32::try_from(payload.len()).expect("frame fits in u32");
    let mut out = Vec::with_capacity(4 + payload.len());
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Read a length-prefixed frame from an iroh RecvStream. Returns the
/// payload bytes (length prefix stripped). Returns `None` on clean EOF.
#[cfg(feature = "network-iroh")]
pub(crate) async fn read_length_prefixed_frame(
    stream: &mut iroh::endpoint::RecvStream,
) -> Result<Option<Vec<u8>>, HeadsStreamError> {
    use tokio::io::AsyncReadExt;
    let mut len_buf = [0u8; 4];
    match stream.read_exact(&mut len_buf).await {
        Ok(_) => {}
        // EOF before any length bytes = clean stream close.
        Err(e) if /* check for clean EOF */ => return Ok(None),
        Err(e) => return Err(HeadsStreamError::Transport(format!("read length: {e}"))),
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(HeadsStreamError::Transport(format!(
            "frame too large: {len} bytes (max {MAX_FRAME_BYTES})"
        )));
    }
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).await
        .map_err(|e| HeadsStreamError::Transport(format!("read payload: {e}")))?;
    Ok(Some(payload))
}
```

The exact iroh recv-side API (`RecvStream::read_exact` or stream-impl variant) will be confirmed against iroh 1.0.0-rc.0 during implementation. The framing scheme is fixed; the read-call shape may adapt.

### 3.6 Reader task — `crates/network/src/iroh_transport.rs`

```rust
async fn read_event_frames(
    mut recv_stream: iroh::endpoint::RecvStream,
    tx: tokio::sync::mpsc::Sender<Result<Event, HeadsStreamError>>,
) {
    loop {
        let frame = match crate::request::read_length_prefixed_frame(&mut recv_stream).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return, // clean EOF
            Err(e) => {
                let _ = tx.send(Err(e)).await;
                return;
            }
        };
        let event: Event = match canonical_bincode().deserialize(&frame) {
            Ok(e) => e,
            Err(e) => {
                let _ = tx.send(Err(HeadsStreamError::Decode(format!("event decode: {e}")))).await;
                return;
            }
        };
        if tx.send(Ok(event)).await.is_err() {
            // Requester dropped the stream. Stop reading.
            return;
        }
    }
}
```

## 4. Acceptance tests

### 4.1 `crates/network/tests/direct_streams_memory.rs` (new file)

Tests the MemNetwork direct-stream surface end-to-end without iroh.

**Test 1: `mem_request_heads_delivers_events`** — Two `MemNetwork` peers on a shared bus. Peer B installs a handler that returns a vec of 3 pre-built events. Peer A calls `request_heads(B, req)`, drains the stream, asserts the 3 events arrived in order.

**Test 2: `mem_request_heads_unknown_peer_fails`** — Peer A calls `request_heads(B, req)` without installing a handler on B. Asserts `Err(NetError::RequestFailed { peer: B, .. })`.

**Test 3: `mem_request_heads_handler_drops_responder_signals_eof`** — Handler returns without pushing any events. Stream's first `next()` returns `None`.

**Test 4: `mem_request_heads_multiple_concurrent`** — Peer A issues two `request_heads` calls in parallel to peer B. Both complete with their own event sets; no cross-talk.

**Test 5: `mem_request_handler_sees_requester_pubkey`** — Handler captures the `requester` argument into an `Arc<Mutex<Option<PeerPubkey>>>`. After the call completes, assert the captured value equals peer A's pubkey.

**Test 6: `mem_install_request_handler_last_call_wins`** — Peer B installs handler1, then handler2. Peer A's request invokes handler2 (the last-installed). Verified via a sentinel value the handler injects.

### 4.2 `crates/network/tests/direct_streams_iroh.rs` (new file)

Gated on `#[cfg(feature = "network-iroh")]`. Reuses the `spawn_iroh_peer` helper from `iroh_gossip.rs` (with adaptations for the new ALPN).

**Test 1: `iroh_request_heads_delivers_events`** — Two `IrohNetwork` peers spawned with shared `MemoryLookup`. Both peers' Routers register `HEADS_REQUEST_ALPN` against the `protocol_handler()` returned by their respective `IrohNetwork`. Peer B's `install_request_handler` installs a handler that returns 3 events. Peer A calls `request_heads(B, req)`, drains the stream, asserts the 3 events.

**Test 2: `iroh_request_heads_unknown_alpn_fails`** — Peer B's Router does NOT register the HeadsRequest ALPN. Peer A's `request_heads(B, req)` fails with `NetError::RequestFailed`.

**Test 3: `iroh_request_heads_no_handler_installed_clean_eof`** — Peer B registers the ALPN but doesn't call `install_request_handler`. Peer A's request connects, sends the request, reads zero events, sees clean EOF (`next()` returns `None`).

**Test 4: `iroh_request_heads_handler_topic_validation`** — Handler explicitly rejects requests for an unknown topic by returning early without pushing events. Verified by passing a wrong-topic request from peer A and seeing zero events arrive.

**Test 5: `iroh_protocol_handler_clone_does_not_break`** — `IrohNetwork::protocol_handler()` is called twice; both returned protocol handlers share state via the internal `Arc<Mutex<Option<...>>>`. Installing the handler after the first `protocol_handler()` clone is captured by both clones.

### 4.3 Round-trip + wire-freeze tests — `crates/types/tests/wire_freeze.rs`

Add a `direct_heads_request_wire_freeze` test that constructs a sample `DirectHeadsRequest` and asserts its canonical bincode bytes match a pinned 200-byte (or whatever) hex literal.

## 5. Justfile changes

`just spec-coverage` regeneration — add B-4.4 spec ID to the cited-by map. Existing `just test --workspace --features network-iroh` continues to cover both test files.

## 6. Edge cases

| Scenario | Behavior |
|---|---|
| Requester drops `HeadsStream` mid-response | Responder's `tx.send` returns Err on next push; handler observes via `HeadsResponder::send` returning `Err(())`. Handler stops producing. |
| Responder handler panics | Channel sender drops; requester sees clean EOF. Handler panic is logged at the runtime layer (B-4.5 concern). |
| Both peers behind NAT, no relay | iroh path-upgrade machinery falls back to relay. `Endpoint::connect` succeeds via relay path. No change at our layer. |
| Request payload > MAX_FRAME_BYTES | Responder reads first 4 bytes, sees length > cap, returns `Err(Transport)` before allocating the buffer. Stream terminates immediately. |
| ALPN mismatch | `Endpoint::connect` fails at TLS handshake. `NetError::RequestFailed` surfaced. |
| Peer offline | `Endpoint::connect` fails (no path reachable). `NetError::RequestFailed` after iroh's connect timeout. Caller's responsibility to retry / give up. |
| Handler never installed before inbound request | Accept loop sees no installed handler, closes stream cleanly. Requester reads zero events + EOF — distinguishable from "handler installed, returned empty" only by side-channel observability (B-4.5 may add a warning). |
| Same `MemNetwork` instance calls `request_heads(self.peer_pubkey, ...)` | Loopback: handler runs in the same process. Tests may exercise this; production code should not — handlers may need `requester != self` filtering. Not enforced at the network layer. |
| Frame decode failure mid-stream | Reader pushes `Err(HeadsStreamError::Decode)` and returns; stream is terminated. Requester sees the error, no further events. |

## 7. Surface change summary

**New crate-public surface** (this slice):

- `myrhiza_types::DirectHeadsRequest` — wire type.
- `myrhiza_network::request::{HEADS_REQUEST_ALPN, HeadsStream, HeadsStreamError, HeadsResponder, RequestHandler, ArcRequestHandler}`.
- `myrhiza_network::Network::request_heads` — new trait method.
- `myrhiza_network::Network::install_request_handler` — new trait method.
- `myrhiza_network::NetError::RequestFailed` — new variant.
- `myrhiza_network::MemNetwork::new(bus, peer_pubkey)` — gained a parameter (SemVer-breaking).
- `myrhiza_network::MemNetwork::peer_pubkey()` — new getter.
- `myrhiza_network::IrohNetwork::protocol_handler()` — new method returning the `iroh::protocol::ProtocolHandler` impl.
- `myrhiza_network::iroh_transport::HeadsRequestProtocol` — new public type (ProtocolHandler impl).

**Breaking SemVer changes**:

- `Network` trait: two new methods. All in-tree impls (`MemNetwork`, `IrohNetwork`) provide them; out-of-tree impls must add them.
- `MemNetwork::new`: gained `peer_pubkey` parameter. ~30 call sites swept in this PR.

**Unchanged**:

- `GossipMessage` enum — keeps `HeadsRequest` variant for wire-freeze stability. Runtime continues to publish/handle it via the gossip path until B-4.5.
- `IrohNetwork::new` — unchanged signature (the new handler-registration is post-construction).
- Existing `HeadsRequest` + `HeadsRequestSignedPayload` wire shapes — kept as-is.

## 8. Non-goals (explicit)

- **No runtime integration.** The kernel `Runtime` continues to use `publish(GossipMessage::HeadsRequest(_))` for backfill. B-4.5 switches it over.
- **No rate-limiting at the request layer.** Per-requester request bucketing is a B-4.5+ concern.
- **No backpressure other than QUIC's native flow control.** Application-layer backpressure (e.g. credit-based windowing) is future ergonomics work.
- **No removal of the gossip-routed `HeadsRequest`** in this slice — B-4.5 decides whether to deprecate or remove.
- **No `request_heads` retry / timeout policy** at the trait level — consumers (B-4.5 runtime) own retry policy.
- **No cross-process tests.** B-4.5 may add them; in-process two-peer tests give sufficient protocol coverage.
- **No discovery integration.** `request_heads(peer, req)` assumes the requester already has the peer's `EndpointId` (via a signed HeadsSummary or other side channel). pkarr / DHT lookup is B-4.6+.

## 9. Prior-art consultation

- [`prior-art/iroh/architecture.md`](../prior-art/iroh/architecture.md) §"ALPN-based protocol multiplexing" — the ALPN convention `<name>/<version>`, `Router::builder.accept(ALPN, handler).spawn()`, `connection.open_bi()` / `accept_bi()` for bidirectional streams. **Borrowed wholesale**: this is the iroh-prescribed shape for application protocols on top of `Endpoint`.
- [`prior-art/iroh/architecture.md`](../prior-art/iroh/architecture.md) §"Connection errors" — `ConnectError` vs `ConnectionError` distinction; our `NetError::RequestFailed` is a connect-time error; mid-stream errors surface via `HeadsStreamError::Transport`.
- [`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md) §Borrow row 2 — "Router-owned by embedder, not by the transport library." Reaffirmed: `IrohNetwork::protocol_handler()` returns a handler that the embedder registers; we do not register internally.
- [`docs/specs/2026-05-20-plan-b-4-2-attribution-design.md`](2026-05-20-plan-b-4-2-attribution-design.md) §3.0 — `HeadsRequest` signature rationale (Plumtree-forwarding-hides-publisher) does not apply to direct-streams; QUIC TLS supplies the attribution.
- [`docs/specs/2026-05-20-plan-b-4-1-iroh-gossip-design.md`](2026-05-20-plan-b-4-1-iroh-gossip-design.md) §3 — the existing `IrohNetwork::new(endpoint, gossip)` constructor convention; B-4.4 preserves it.

**Gaps**:

- iroh-gossip's wire format for `iroh_gossip::ALPN` is opaque to us; we register a sibling ALPN, not extend gossip's. No conflict.
- iroh 1.0.0-rc.0's `ProtocolHandler` trait signature is not yet pinned in our prior-art docs — must verify against the crate at impl time. Spec assumes the documented `accept(connection) -> Future<Result<(), AcceptError>>` shape.

## 10. Future work — explicit deferrals

- **B-4.5** — Kernel runtime switchover to `request_heads`. Construct a `RequestHandler` impl with DAG access (mpsc back to runtime task); replace `publish(GossipMessage::HeadsRequest)` call sites; remove or no-op `handle_heads_request`; full backfill cycle test via direct-streams.
- **B-4.6** — Discovery / pkarr / DHT integration. `request_heads(peer, req)` currently assumes the requester has the target's `EndpointAddr`; integrate pkarr publish/lookup.
- **Cross-process / cross-machine acceptance tests.** Separate test harness slice.
- **Per-requester rate-limiting** at the responder side.
- **Backpressure beyond QUIC's native flow control.** Token-bucket / credit-based window.
- **Removal of `GossipMessage::HeadsRequest` variant.** Wire-freeze regen; happens after B-4.5 confirms the gossip-routed path is dead.
- **`Lagged`-event mapping test on real iroh-gossip** (B-4.1 §4 deferral) — still open.
- **Backfilling `PeerWarning::SignatureInvalid` into `process_drift_message`** (B-4.2 §10 deferral) — still open.

## 11. Sources

- `crates/network/src/lib.rs:128-173` — `Network` trait surface (extended in §3.2).
- `crates/network/src/memory.rs:1-219` — `MemBus` + `MemNetwork` (refactored in §3.3).
- `crates/network/src/iroh_transport.rs:1-312` — `IrohNetwork` (extended in §3.4).
- `crates/network/src/subscription.rs` — `Subscription` trait + `MemSubscription` (unchanged).
- `crates/types/src/dag.rs:267-292` — existing `HeadsRequest` + `HeadsRequestSignedPayload` (sibling of new `DirectHeadsRequest`).
- `crates/kernel/src/runtime.rs:872-933` — current `handle_heads_summary` (will be modified in B-4.5).
- `crates/kernel/src/runtime.rs:1208-1234` — current `handle_heads_request` (gossip-routed handler; removed/no-op'd in B-4.5).
- `crates/network/tests/iroh_gossip.rs:50-75` — `spawn_iroh_peer` helper template.
- `crates/types/tests/wire_freeze.rs` — wire-freeze test infrastructure.
- [`prior-art/iroh/architecture.md`](../prior-art/iroh/architecture.md).
- [`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md).
- [`docs/specs/2026-05-20-plan-b-4-1-iroh-gossip-design.md`](2026-05-20-plan-b-4-1-iroh-gossip-design.md).
- [`docs/specs/2026-05-20-plan-b-4-2-attribution-design.md`](2026-05-20-plan-b-4-2-attribution-design.md).
- [`docs/specs/2026-05-20-plan-b-4-3-halt-detection-design.md`](2026-05-20-plan-b-4-3-halt-detection-design.md).
