**Date:** 2026-05-20
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-4.1 — Real subscribe + publish via iroh-gossip

# Plan B-4.1 design — Iroh-gossip subscribe + publish

## 1. Goal

Replace B-4.0's `NetError::Unimplemented` returns in `IrohNetwork::subscribe` + `publish` with real iroh-gossip-backed implementations:

- **`subscribe(topic, bootstrap) → IrohSubscription`** wires `iroh_gossip::Gossip::subscribe(topic_id, bootstrap_peers)` and wraps the returned `GossipTopic` (a `futures::Stream<Item = Result<Event, ApiError>>`) in our `Subscription` trait shape.
- **`publish(topic, message) → Result<(), NetError>`** canonical-bincode-encodes the `GossipMessage` enum to `bytes::Bytes` and calls `GossipSender::broadcast`.
- **Receive filtering**: iroh-gossip emits `Event::Received` (real payload), `Event::NeighborUp`/`NeighborDown` (membership), `Event::Joined` (initial), and `Event::Lagged` (overrun). B-4.1 surfaces only `Received` as `GossipMessage`s, maps `Lagged` to `SubError::Lagged(0)` (count fidelity lost — see §6), and silently consumes neighbor/joined events.
- **`Network` trait change**: add `bootstrap: Vec<PeerPubkey>` parameter to `subscribe`. iroh-gossip requires this for HyParView swarm formation; `MemNetwork` accepts and ignores it (in-process broadcast has no peer discovery).
- **`Router` wiring**: caller-responsibility, documented. The kernel embedder must construct `iroh::protocol::Router::builder(endpoint).accept(iroh_gossip::ALPN, gossip.clone()).spawn()` before handing the endpoint+gossip to `IrohNetwork::new`. Without Router wiring, inbound iroh-gossip streams won't dispatch.

This slice lands **none** of:

- **Q-4 — sender attribution at the protocol layer.** iroh-gossip's `delivered_from: EndpointId` field on `Event::Received` is **the last-hop neighbor under Plumtree forwarding, not the original publisher**. True sender identity for `HeadsSummary` + `HeadsRequest` requires either direct peer-to-peer streams (not gossip) or signed-envelope wrapping. Deferred to **B-4.2**, which owns the design choice.
- **Real cross-process tests.** B-4.1's acceptance tests are intra-process — two `IrohNetwork` instances over real iroh in the same tokio runtime. Cross-process / cross-machine validation lives in B-4.3.
- **Discovery / pkarr / DHT integration.** Bootstrap is caller-provided `Vec<PeerPubkey>`; how peers learn each other's pubkeys is app-layer in B-4.1. `prior-art/iroh/lessons.md` §Avoid row 7 names discovery as a deliberate Myrhiza design decision deferred until a discovery primitive spec.
- **Lag-count fidelity.** iroh-gossip drops the lagged count internally (`Event::Lagged` is a unit variant per `gossip/src/api.rs:344`). B-4.1 surfaces `SubError::Lagged(0)` with a TODO; reclaiming count fidelity requires patching iroh-gossip or wrapping the broadcast channel manually.
- **`NeighborUp` / `NeighborDown` observability.** Membership events are silently consumed in B-4.1. A future plan may extend `RuntimeHandle` with a `peer_membership_log` if apps need it.

## 2. Scope decisions (locked during brainstorming + iroh-gossip API survey, 2026-05-20)

| Decision | Chosen | Runner-up | Why |
|---|---|---|---|
| **`Network::subscribe` signature change** | Add `bootstrap: Vec<PeerPubkey>` parameter | (a) Out-of-band `IrohNetwork::add_bootstrap` method; (b) `IrohNetwork::new` takes static bootstrap list | iroh-gossip's `Gossip::subscribe(topic, bootstrap_vec)` requires bootstrap at subscribe time (`iroh-gossip-0.99.0/src/api.rs:157-167`). Threading through the trait is the only shape that keeps `IrohNetwork` and `MemNetwork` behind the same surface. `MemNetwork` ignores the vec (B-1's MemBus is in-process broadcast — no peer discovery). **This is a SemVer-breaking trait change**; the only call site is `Runtime::start` (B-1 `runtime.rs:368`) which can pass `vec![]` for now (B-4.* doesn't yet plumb peer-discovery to the runtime). |
| **Subscription shape** | New `IrohSubscription` struct holding `Pin<Box<iroh_gossip::GossipReceiver>>` + a buffered `Option<GossipMessage>` for back-pressure | Pass-through to `GossipTopic` directly | iroh-gossip yields `Event` variants (Received / NeighborUp / NeighborDown / Joined / Lagged) which don't map 1:1 to `Result<Option<GossipMessage>, SubError>`. The wrapper filters + maps. Buffering one decoded message lets `recv()` resolve cleanly when multiple non-Received events arrive between calls. |
| **Topic ↔ TopicId conversion** | Free fn `iroh_topic_id_from_topic(topic) -> iroh_gossip::TopicId` (no orphan-rule issue since `TopicId` is a foreign type but `Topic` is local-ish? **NO** — `Topic` is in `myrhiza-types`, `TopicId` is in `iroh-gossip`. Both foreign to `myrhiza-network`. Free fn for consistency with B-4.0's `peer_pubkey_from_iroh`.) | `From` impl | Same orphan-rule constraint as B-4.0's `PeerPubkey`. Free function in `iroh_transport.rs`. |
| **GossipMessage encoding** | Canonical bincode (`crate::canonical_bincode()` per types crate convention) → `bytes::Bytes` | Custom wire format | `GossipMessage` is already bincode-serialized in B-1's `wire_freeze` test (`crates/types/tests/wire_freeze.rs`); reusing matches the canonical envelope determinism guarantee from determinism.md §5.4. `Bytes::from(Vec<u8>)` is the only adaptation. |
| **Event filtering** | `Received` → decode + surface as `Some(GossipMessage)`; `NeighborUp` / `NeighborDown` / `Joined` → silently consume + continue loop; `Lagged` → return `Err(SubError::Lagged(0))` | Surface all events through the trait | Membership events would force every `Subscription` consumer to handle them; only `IrohNetwork` produces them so the runtime would spend cycles ignoring them. Keep the trait surface clean. |
| **Lag count fidelity** | Map `Event::Lagged` to `SubError::Lagged(0)` with inline TODO | Patch iroh-gossip to expose count | Iroh-gossip drops the count at `gossip/src/net.rs:940` (the `_` pattern); reclaiming it requires either upstream patching or wrapping the underlying `tokio::sync::broadcast::Receiver` manually. Sentinel-0 + documented gap matches B-2.1's "best-effort observability" stance. |
| **Stream-end behavior** | Stream end → `Ok(None)` from `recv()` | `Err(SubError::Closed)` | `recv()` returning `Ok(None)` is the trait's documented "subscription closed" signal (per B-1 §6); matches `MemNetwork`'s shape. |
| **`Router` wiring** | Caller-responsibility — `IrohNetwork::new` still takes `(endpoint, gossip)` only; caller must have already registered gossip's ALPN with a `Router` | `IrohNetwork::new_with_router(endpoint, gossip, router)` | Caller-owned matches `prior-art/iroh/lessons.md` §Borrow row 2 ("Router + ProtocolHandler for ALPN-namespaced multi-tenant dispatch — kernel allocates"); apps registering additional ALPN handlers benefit from caller-control. Document the Router-must-be-configured requirement in `IrohNetwork::new`'s rustdoc. |
| **`NetError::Unimplemented` lifecycle** | Keep the variant; `unsubscribe` still returns it (planned in B-4.2 or future) | Delete the variant | One method (`unsubscribe`) is still unimplemented after B-4.1. Variant has further use; do not delete. |
| **`unsubscribe` deferral** | Returns `NetError::Unimplemented { method: "Network::unsubscribe", planned_in: "B-4.2" }` | Wire it in B-4.1 | Iroh-gossip's `GossipTopic` self-cleans on drop (`gossip/src/api.rs:298-303` — the `Drop` impl closes the broadcast channel). The runtime's per-topic `IrohSubscription` lifetime ALREADY covers the practical "stop receiving" case. Explicit `unsubscribe` semantics (signal swarm departure) is a real concern but not blocking gossip ingestion — defer. |
| **Acceptance test shape** | 2 in-process `IrohNetwork` instances; both subscribe (peer B bootstraps from A's `EndpointId`); A publishes; B's `recv()` returns the bincode-decoded message | Mock the gossip layer | Real iroh-gossip end-to-end is the load-bearing acceptance evidence. Mocking just verifies the wire-encode/decode contract which is already covered by `wire_freeze.rs`. |
| **Test runtime** | `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` for all gossip-driven tests | Single-threaded current-thread | iroh-gossip spawns internal tasks (`gossip/src/net.rs:207`); single-threaded would deadlock waiting for those tasks if the test holds the runtime. Multi-thread is the safe choice. |
| **Test timeouts** | `tokio::time::timeout(Duration::from_secs(5), ...)` wrap on `recv()` calls that expect a specific message | No timeout | iroh-gossip swarm formation is best-effort; tests can hang indefinitely if NeighborUp never arrives. Bounded timeout makes failure modes diagnosable. |

## 3. Code surface

### 3.1 `Network` trait change

In `crates/network/src/lib.rs`:

```rust
#[async_trait::async_trait]
pub trait Network: Send + Sync + 'static {
    type Subscription: Subscription + Send + 'static;

    /// Subscribe to a topic, with optional bootstrap peer hints.
    ///
    /// For transports that maintain a peer-discovery overlay
    /// ([`IrohNetwork`]), `bootstrap` is a list of `PeerPubkey`s to
    /// dial when forming the topic's swarm. An empty `bootstrap`
    /// is legal — the topic exists locally and waits for inbound
    /// joins.
    ///
    /// For transports without peer-discovery semantics
    /// ([`MemNetwork`]), `bootstrap` is ignored (in-process
    /// broadcast routes by topic only).
    ///
    /// # Errors
    /// Returns [`NetError::SubscribeClosed`] if the transport has
    /// been shut down.
    async fn subscribe(
        &self,
        topic: Topic,
        bootstrap: Vec<PeerPubkey>,    // NEW in B-4.1
    ) -> Result<Self::Subscription, NetError>;

    // ... publish, unsubscribe unchanged in signature ...
}
```

Existing `MemNetwork::subscribe` updates to accept-and-ignore the bootstrap parameter. `Runtime::start` updates its single call site to pass `vec![]` (B-4.* will plumb peer-discovery to the runtime in a later slice).

### 3.2 `iroh_transport.rs` additions

```rust
// crates/network/src/iroh_transport.rs

use bytes::Bytes;
use futures::StreamExt;
use iroh_gossip::{api::Event, GossipReceiver, GossipSender, GossipTopic, TopicId};
// ... existing imports ...

/// Topic → iroh_gossip::TopicId conversion. Free function (orphan
/// rule, same as B-4.0's PeerPubkey conversions).
pub fn iroh_topic_id_from_topic(topic: Topic) -> TopicId {
    TopicId::from_bytes(*topic.as_bytes())
}

impl Network for IrohNetwork {
    type Subscription = IrohSubscription;

    async fn subscribe(
        &self,
        topic: Topic,
        bootstrap: Vec<PeerPubkey>,
    ) -> Result<Self::Subscription, NetError> {
        let topic_id = iroh_topic_id_from_topic(topic);
        // Convert each PeerPubkey → EndpointId via the B-4.0
        // free function. Failures (invalid curve point) propagate.
        let mut bootstrap_ids: Vec<iroh::EndpointId> = Vec::with_capacity(bootstrap.len());
        for pk in bootstrap {
            let id = iroh_endpoint_id_from_peer_pubkey(pk).map_err(|e| {
                NetError::PublishFailed(format!("bootstrap peer pubkey invalid: {e}"))
            })?;
            bootstrap_ids.push(id);
        }
        let gossip_topic = self
            .gossip
            .subscribe(topic_id, bootstrap_ids)
            .await
            .map_err(|e| NetError::PublishFailed(format!("iroh-gossip subscribe: {e}")))?;
        Ok(IrohSubscription::new(gossip_topic))
    }

    async fn publish(
        &self,
        topic: Topic,
        message: GossipMessage,
    ) -> Result<(), NetError> {
        let topic_id = iroh_topic_id_from_topic(topic);
        let bytes = canonical_bincode()
            .serialize(&message)
            .map_err(|e| NetError::PublishFailed(format!("encode: {e}")))?;
        // Re-subscribe is a no-op if already subscribed (per
        // iroh-gossip's idempotent subscribe semantics); we
        // unconditionally re-subscribe-then-broadcast to keep
        // publish independent of subscribe call ordering.
        //
        // TRADE-OFF: this allocates a new GossipTopic per publish.
        // B-4.2/B-4.3 may cache a per-topic sender to avoid the
        // overhead. See spec §11 for the deferred optimization.
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
        // GossipTopic self-cleans on drop (iroh-gossip-0.99.0
        // gossip/src/api.rs:298-303). The IrohSubscription's
        // lifetime already covers practical "stop receiving"
        // semantics. Explicit swarm-departure signaling is a
        // future plan.
        Err(NetError::Unimplemented {
            method: "Network::unsubscribe",
            planned_in: "B-4.2",
        })
    }
}

/// Iroh-gossip-backed subscription.
///
/// Wraps a [`iroh_gossip::GossipTopic`] (a `futures::Stream<Item =
/// Result<Event, ApiError>>`), filters events to only surface
/// [`Event::Received`] payloads, maps [`Event::Lagged`] to
/// [`SubError::Lagged(0)`] (count fidelity lost — see spec §6), and
/// silently consumes membership events ([`Event::NeighborUp`],
/// [`Event::NeighborDown`], [`Event::Joined`]).
pub struct IrohSubscription {
    inner: GossipTopic,
}

impl IrohSubscription {
    fn new(inner: GossipTopic) -> Self {
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
                    // ApiError from iroh-gossip during streaming
                    // — treat as a transient lag-like signal
                    // rather than a fatal close. The next call
                    // will retry. TRADE-OFF: if the underlying
                    // gossip task has died, we'll spin. Mitigate
                    // in B-4.3 with explicit halt detection.
                    return Err(SubError::Lagged(0));
                }
                Some(Ok(Event::Received(msg))) => {
                    // Decode the bincode-encoded GossipMessage
                    // payload.
                    let decoded: GossipMessage = canonical_bincode()
                        .deserialize(&msg.content)
                        .map_err(|_decode_err| SubError::Lagged(0))?;
                    return Ok(Some(decoded));
                }
                Some(Ok(Event::Lagged)) => {
                    // Iroh-gossip drops the count; sentinel 0.
                    return Err(SubError::Lagged(0));
                }
                Some(Ok(Event::NeighborUp(_)))
                | Some(Ok(Event::NeighborDown(_)))
                | Some(Ok(Event::Joined(_))) => {
                    // Membership events — silently consume,
                    // continue the loop.
                    continue;
                }
            }
        }
    }
}
```

**API-name verification at impl time** (per `prior-art/iroh/lessons.md` §Avoid row 1):

- `iroh_gossip::api::Event` variant names (`Received` / `Lagged` / `NeighborUp` / `NeighborDown` / `Joined`) — verified against `iroh-gossip-0.99.0/src/api.rs:332-345` in the B-4.1 prep survey. The `NeighborUp` / `NeighborDown` variants may carry a `EndpointId` payload (per `gossip/src/api.rs` survey); test the matcher pattern at impl time.
- `iroh_gossip::TopicId::from_bytes([u8; 32])` — verified `gossip/src/proto/state.rs:20-22`.
- `iroh_gossip::GossipTopic::split()` returns `(GossipSender, GossipReceiver)` per `gossip/src/api.rs:227-229`.
- `iroh_gossip::Gossip::subscribe(topic, Vec<EndpointId>) -> Result<GossipTopic, ApiError>` per `gossip/src/api.rs:157-167`.

If a name has rotated, adapt at impl time and document in commit body.

### 3.3 `MemNetwork` update

`crates/network/src/memory.rs`'s `Network::subscribe` impl takes the new `bootstrap: Vec<PeerPubkey>` parameter and discards it. One-line change:

```rust
async fn subscribe(
    &self,
    topic: Topic,
    _bootstrap: Vec<PeerPubkey>,    // ignored — in-process broadcast has no peer discovery
) -> Result<Self::Subscription, NetError> { /* ... unchanged body ... */ }
```

### 3.4 `Runtime::start` update

`crates/kernel/src/runtime.rs:368-371` calls `erased.subscribe(topic).await?`. Update to:

```rust
let sub = erased.subscribe(topic, vec![]).await?;
```

This is the only consumer of `Network::subscribe` outside the kernel-internal tests.

B-1's convergence tests in `crates/kernel/tests/convergence.rs` may also call `subscribe` directly via the harness — those need the same `vec![]` update. The implementer should grep `\.subscribe(` across `crates/` to find every site.

## 4. Acceptance tests

`crates/network/tests/iroh_gossip.rs` (new file; feature-gated):

| # | Test name | Flavor | Pattern |
|---|---|---|---|
| 1 | `two_peers_subscribe_and_exchange_a_single_event` | `multi_thread, worker_threads = 2` | Spin up endpoint+gossip+router for peer A and peer B in-process. B subscribes with bootstrap = `[a_id]`. A subscribes with empty bootstrap. Wait until B's swarm has formed (poll `recv()` consuming neighbor events until first non-error). A publishes a `GossipMessage::Event(...)` (use B-1's `EventBuilder` to build a valid event). B's next `recv()` returns the same `GossipMessage`. Use `tokio::time::timeout(Duration::from_secs(5), ...)`. |
| 2 | `subscribe_publishes_propagate_via_gossip_to_three_peers` | `multi_thread, worker_threads = 2` | Same shape, 3 peers in a chain (A↔B↔C). C's bootstrap is `[b_id]`; B's is `[a_id]`; A's is empty. A publishes; C must receive. Exercises Plumtree forwarding across one hop. |
| 3 | `decode_failure_surfaces_as_subscribe_lagged` | `multi_thread, worker_threads = 2` | Two peers as in #1, but A publishes via a backdoor (`gossip_a.subscribe(topic, vec![]).await?.0.broadcast(Bytes::from_static(b"garbage-not-bincode"))`) bypassing IrohNetwork's encode path. B's `recv()` returns `Err(SubError::Lagged(0))` (decode failure path per spec §3.2). |
| 4 | `lagged_event_maps_to_sub_error_lagged` | `multi_thread, worker_threads = 2` | Hard to trigger deterministically without controlling iroh-gossip's internal broadcast buffer. **Defer test #4 with `#[ignore]` + a comment** — flag as B-4.3 acceptance work when we have a more invasive test harness. Or skip entirely and rely on the unit-level decode-failure test for SubError surfacing. |
| 5 | `unsubscribe_returns_unimplemented` | default | Construct an IrohNetwork, call `unsubscribe(topic).await`, assert `NetError::Unimplemented { method: "Network::unsubscribe", planned_in: "B-4.2" }`. |
| 6 | `topic_id_from_topic_roundtrips` | default | Pure unit test (no async): construct a `Topic::from_bytes([0xAA; 32])`, convert via `iroh_topic_id_from_topic`, assert `topic_id.as_bytes() == [0xAA; 32]`. |

**Test setup helper** — extract `spawn_iroh_peer(label: &str) -> (Endpoint, Gossip, Router, IrohNetwork)` into a `helpers` module in the test file to keep the per-test boilerplate down.

Spec-coverage annotations: tests 1, 2 → `networking.md §11.1`; tests 3, 4 → `convergence.md §4.4` (wire-decode failure); test 5 → `networking.md §11.1`; test 6 → `convergence.md §4.6` (topic-ID).

## 5. Justfile changes

The existing `just test-iroh` recipe (from B-4.0) already covers the new test file (it runs `cargo test -p myrhiza-network --features network-iroh --tests`). No Justfile changes needed.

## 6. Edge cases

- **Empty bootstrap**: legal. `Gossip::subscribe(topic, vec![])` accepts an empty bootstrap set per `gossip/examples/chat.rs:83` (open-mode). The topic exists locally; the swarm forms via inbound joins.
- **Bootstrap peer not reachable**: iroh-gossip's join is async + best-effort. The subscribe call succeeds even if no peer in `bootstrap` is reachable; `recv()` simply doesn't yield until *some* peer joins. Tests bound with `tokio::time::timeout`.
- **Stale `IrohSubscription` after `Drop`**: iroh-gossip's `GossipTopic::Drop` closes the broadcast channel (`gossip/src/api.rs:298-303`). `recv()` returns `Ok(None)` thereafter; matches the trait's stream-end semantics.
- **Bincode decode failure on receive**: surfaces as `SubError::Lagged(0)` per spec §3.2. **Imperfect mapping** — `Lagged` semantically means "messages dropped due to buffer overrun", but our `SubError` enum has no "decode failed" variant. Could introduce `SubError::DecodeFailed`; reserved for a follow-up if app code needs to differentiate. Acceptable for B-4.1 because the runtime treats both as transient.
- **Concurrent `subscribe` to the same topic**: iroh-gossip's `subscribe` is idempotent at the topic level — multiple subscribe calls produce independent `GossipTopic` handles, each with its own broadcast receiver. Our `publish` (§3.2) re-subscribes per call; this is acceptable for B-4.1 but allocates and is flagged in spec §11 as a deferred optimization.
- **`ApiError` mid-stream**: mapped to `SubError::Lagged(0)`. The underlying gossip task may have died; B-4.1 will spin if the next `recv()` keeps yielding `ApiError`. Mitigation deferred to B-4.3 (proper halt detection).
- **Sender identity**: every received `Message` carries `delivered_from: EndpointId` (last-hop neighbor under Plumtree). B-4.1 discards this; B-4.2 owns the sender-identity design.

## 7. Surface change summary

**Trait change (SemVer-breaking for `Network`)**:
- `Network::subscribe` adds `bootstrap: Vec<PeerPubkey>` parameter. Only kernel-internal consumer (`Runtime::start`); updated to pass `vec![]`. `MemNetwork::subscribe` accepts and ignores.

**New public surface in `myrhiza_network::iroh_transport`** (feature-gated):
- `iroh_topic_id_from_topic(topic) -> iroh_gossip::TopicId` free function.
- `IrohSubscription::recv` now does real work (not `unreachable!()` anymore).
- `IrohNetwork::subscribe` + `IrohNetwork::publish` now do real work.
- `IrohNetwork::unsubscribe` still returns `NetError::Unimplemented`.

**Modified existing files**:
- `crates/network/src/lib.rs` — trait shape update.
- `crates/network/src/memory.rs` — ignore-bootstrap update.
- `crates/network/src/iroh_transport.rs` — bodies fleshed out.
- `crates/kernel/src/runtime.rs` — call-site update (one line).
- `crates/network/tests/iroh_skeleton.rs` (B-4.0 file) — update test calls to pass `vec![]` for bootstrap, OR delete the test (the new B-4.1 tests cover construction).

## 8. Non-goals (explicit)

- **No Q-4 sender attribution.** Deferred to B-4.2 explicitly. `delivered_from` is available but unused.
- **No cross-process tests.** B-4.3.
- **No discovery.** Bootstrap is caller-provided; no DHT, no pkarr.
- **No lag-count fidelity.** Sentinel 0.
- **No `unsubscribe` implementation.** Drop semantics cover practical use.
- **No publish-side topic caching.** Re-subscribes per publish; deferred per §11.
- **No `NeighborUp` / `NeighborDown` observability** through the runtime.
- **No iroh `Router` integration on the kernel side.** Caller wires it.

## 9. Prior-art consultation

Consulted via the `using-prior-art` skill, 2026-05-20:

- **`prior-art/iroh/gossip.md`** — Plumtree + HyParView fundamentals, "topic-based pub/sub", "small messages with weak delivery guarantees", `iroh-gossip 0.99.0` release context. Confirms our `GossipMessage` envelope shape (canonical bincode opaque payload) is exactly how the iroh-gossip transport expects to be used (opaque user-defined bytes). The "don't fold gossip into deterministic state" warning aligns with Myrhiza's existing `state-apply` purity discipline.
- **`prior-art/iroh/architecture.md`** §"Router + ProtocolHandler" — confirms caller-owned Router pattern. B-4.1 documents the requirement; the kernel embedder constructs once.
- **`prior-art/iroh/lessons.md`** §Borrow rows 1+2 — kernel-owned `Endpoint` + ALPN-namespaced multi-tenant dispatch via `Router`. B-4.1 honors both.
- **`prior-art/iroh/identity.md`** §"NodeID = Ed25519 public key" — `delivered_from: EndpointId` is structurally a `PeerPubkey`; the future B-4.2 attribution work has the type pipeline ready.

**Runner-up paradigms rejected:**
- Direct peer-to-peer streams (bypassing gossip) for `HeadsSummary` / `HeadsRequest` — would solve sender-attribution but bypasses the gossip overlay's discovery + churn-resilience properties. iroh's `Endpoint::connect`-then-direct-stream pattern is more appropriate for blob distribution (B-5+) than for sync metadata.
- Embedding `signed_by_peer: PeerPubkey` directly in `HeadsSummary` / `HeadsRequest` envelopes — requires bumping the wire-freeze test in `crates/types/tests/wire_freeze.rs` and is scope-cross for B-4.1. Reserve for B-4.2.

**Remaining gaps in the prior-art corpus** (candidate triggers for future research):
- iroh-gossip's behavior under high churn (many NeighborUp/Down events per second) — not benchmarked in the prior-art.
- The exact semantics of `subscribe_and_join` vs `subscribe` for our use case — both are documented but the survey didn't dive into which is preferable for production. B-4.1 uses `subscribe` for explicit control over join-completion polling.

## 10. Edge cases (continued from §6)

See §6 above for the load-bearing edge cases. Additional caveats for the implementer:

- **`recv()` stack depth on heavy NeighborUp churn**: the loop calls `self.inner.next().await` inside `recv()` and continues on neighbor events. Under heavy churn this could iterate many times per logical `recv()`. Stack depth is bounded (each iteration releases the previous future), but latency may spike. Acceptable for v1.
- **Drop order with `Router`**: the caller owns the `Router`. If `Router` is dropped before `IrohNetwork`, the gossip ALPN handler stops accepting; subsequent `subscribe` calls will fail with `ApiError`. Document the lifecycle: Router outlives IrohNetwork.

## 11. Future work — explicit deferrals

- **B-4.2** — Q-4 sender attribution. Design choice between (a) direct peer-to-peer streams for HeadsSummary/Request; (b) embedded signed envelope on each gossip message. Probably (b) for HeadsSummary (broadcast semantics) + (a) for HeadsRequest (point-to-point semantics). Also: `unsubscribe` real impl.
- **B-4.3** — Real cross-process / multi-process acceptance tests. Test 4 (`lagged_event_maps_to_sub_error_lagged`) is candidate work since reliable lag triggering needs more harness control. Also: halt detection for `ApiError`-mid-stream.
- **Publish-side topic caching**: cache `(Topic → GossipSender)` map on `IrohNetwork` to avoid re-subscribing per publish. Worth profiling first; may not be a real bottleneck in the runtime's hot path.
- **`SubError::DecodeFailed` variant**: distinct from `Lagged`. Add when an app or the runtime needs to differentiate.
- **`NeighborUp` / `NeighborDown` observability**: extend `RuntimeHandle` with a `peer_membership_log: Arc<Mutex<Vec<PeerMembershipEvent>>>`.

## 12. Sources

- `iroh-gossip-0.99.0/src/api.rs:157-167` — `Gossip::subscribe` signature.
- `iroh-gossip-0.99.0/src/api.rs:336-345` — `Event` enum (Received / NeighborUp / NeighborDown / Joined / Lagged).
- `iroh-gossip-0.99.0/src/api.rs:362-372` — `Message { content: Bytes, scope, delivered_from }`.
- `iroh-gossip-0.99.0/src/api.rs:259-265, 312-330` — `GossipTopic` + `GossipReceiver` implement `Stream<Item = Result<Event, ApiError>>`.
- `iroh-gossip-0.99.0/src/api.rs:227-234` — `split()` + `broadcast(Bytes)`.
- `iroh-gossip-0.99.0/src/api.rs:298-303` — `GossipTopic::Drop` closes channel.
- `iroh-gossip-0.99.0/src/net.rs:940` — `_` discards Lagged count.
- `iroh-gossip-0.99.0/examples/chat.rs` — canonical end-to-end pattern.
- `iroh-gossip-0.99.0/src/proto/state.rs:20-22` — `TopicId([u8; 32])`.
- [`prior-art/iroh/gossip.md`](../prior-art/iroh/gossip.md) — Plumtree + HyParView background.
- [`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md) §Borrow — kernel-owned Endpoint + Router patterns.
- [`docs/specs/2026-05-10-plan-b-1-dag-memnet-design.md`](2026-05-10-plan-b-1-dag-memnet-design.md) §6 — `Network` trait shape.
- [`docs/specs/2026-05-20-plan-b-4-0-iroh-skeleton-design.md`](2026-05-20-plan-b-4-0-iroh-skeleton-design.md) §1 §3.2 — B-4.0 skeleton baseline + deferral list pointing here.
