**Date:** 2026-05-20
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-4.1 — Real subscribe + publish via iroh-gossip

# Plan B-4.1 design — Iroh-gossip subscribe + publish

## 1. Goal

Replace B-4.0's `NetError::Unimplemented` returns in `IrohNetwork::subscribe` + `publish` with real iroh-gossip-backed implementations:

- **`subscribe(topic, bootstrap) → IrohSubscription`** wires `iroh_gossip::Gossip::subscribe(topic_id, bootstrap_peers)` and wraps the returned `GossipTopic` (a `futures::Stream<Item = Result<Event, ApiError>>`) in our `Subscription` trait shape.
- **`publish(topic, message) → Result<(), NetError>`** canonical-bincode-encodes the `GossipMessage` enum to `bytes::Bytes` and calls `GossipSender::broadcast`.
- **Receive filtering**: iroh-gossip emits exactly four `Event` variants (`gossip/src/api.rs:332-345`): `Received(Message)` (real payload), `NeighborUp(EndpointId)`/`NeighborDown(EndpointId)` (membership), and `Lagged` (overrun). B-4.1 surfaces only `Received` as `GossipMessage`s, maps `Lagged` to `SubError::Lagged(0)` (count fidelity lost — see §6), and silently consumes neighbor events. A `GossipReceiver::joined() -> Result<(), ApiError>` method (`api.rs:299-304`) provides an explicit "swarm has formed" signal but B-4.1 does not consume it; tests rely on `recv()` returning a `Received` event to indicate propagation.
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
| **Subscription shape** | New `IrohSubscription` struct holding `GossipTopic` directly (it's already a `Stream`); loop inside `recv()` filters events | Pass-through to `GossipTopic` directly without filtering | iroh-gossip yields four `Event` variants (Received / NeighborUp / NeighborDown / Lagged per `api.rs:332-345`) which don't map 1:1 to `Result<Option<GossipMessage>, SubError>`. The wrapper filters + maps inside an inner loop. No external buffering needed — `Stream::next().await` already buffers internally via the receiver. |
| **Topic ↔ TopicId conversion** | Free fn `iroh_topic_id_from_topic(topic) -> iroh_gossip::TopicId` (no orphan-rule issue since `TopicId` is a foreign type but `Topic` is local-ish? **NO** — `Topic` is in `myrhiza-types`, `TopicId` is in `iroh-gossip`. Both foreign to `myrhiza-network`. Free fn for consistency with B-4.0's `peer_pubkey_from_iroh`.) | `From` impl | Same orphan-rule constraint as B-4.0's `PeerPubkey`. Free function in `iroh_transport.rs`. |
| **GossipMessage encoding** | Canonical bincode (`crate::canonical_bincode()` per types crate convention) → `bytes::Bytes` | Custom wire format | `GossipMessage` is already bincode-serialized in B-1's `wire_freeze` test (`crates/types/tests/wire_freeze.rs`); reusing matches the canonical envelope determinism guarantee from determinism.md §5.4. `Bytes::from(Vec<u8>)` is the only adaptation. |
| **Event filtering** | `Received` → decode + surface as `Some(GossipMessage)`; `NeighborUp` / `NeighborDown` → silently consume + continue loop; `Lagged` → return `Err(SubError::Lagged(0))`; **decode failure** → return `Err(SubError::DecodeFailed { peer })` (new variant — see below) | Surface all events through the trait; OR collapse decode-failure into `Lagged` | Membership events would force every `Subscription` consumer to handle them; only `IrohNetwork` produces them so the runtime would spend cycles ignoring them. **Decode-failure-as-Lagged is wrong**: `Runtime` at `runtime.rs:494-504` triggers `publish_heads_summary` + `PeerWarning::BroadcastLagged` on every `SubError::Lagged` — a flood of malformed gossip from one bad peer would spam backfill publishes from every recipient. New `SubError::DecodeFailed` variant routes the runtime to a discard-and-warn path instead. |
| **`SubError::DecodeFailed` variant** | **In scope for B-4.1**: `SubError::DecodeFailed { peer: Option<PeerPubkey> }` — `peer` is the iroh-gossip `delivered_from` last-hop neighbor (NOT the original publisher; Q-4 attribution is B-4.2). The runtime treats this as a "log + discard" signal, no backfill trigger. | Defer the variant; collapse into `Lagged` | Decode failures will happen at the wire boundary in B-4.1; the runtime's `Lagged` handler is structurally wrong for that case. Adding the variant now is one line; pretending decode is rare and falling back to `Lagged` would cause a real production bug. |
| **Lag count fidelity** | Map `Event::Lagged` to `SubError::Lagged(0)` with inline TODO | Patch iroh-gossip to expose count | Iroh-gossip drops the count at `gossip/src/net.rs:940` (the `_` pattern); reclaiming it requires either upstream patching or wrapping the underlying `tokio::sync::broadcast::Receiver` manually. Sentinel-0 + documented gap matches B-2.1's "best-effort observability" stance. |
| **Stream-end behavior** | Stream end → `Ok(None)` from `recv()` | `Err(SubError::Closed)` | `recv()` returning `Ok(None)` is the trait's documented "subscription closed" signal (per B-1 §6); matches `MemNetwork`'s shape. |
| **`Router` wiring** | Caller-responsibility — `IrohNetwork::new` still takes `(endpoint, gossip)` only; caller must have already registered gossip's ALPN with a `Router` | `IrohNetwork::new_with_router(endpoint, gossip, router)` | Caller-owned matches `prior-art/iroh/lessons.md` §Borrow row 2 ("Router + ProtocolHandler for ALPN-namespaced multi-tenant dispatch — kernel allocates"); apps registering additional ALPN handlers benefit from caller-control. Document the Router-must-be-configured requirement in `IrohNetwork::new`'s rustdoc. |
| **`NetError::Unimplemented` lifecycle** | Keep the variant; `unsubscribe` still returns it (planned in B-4.2 or future) | Delete the variant | One method (`unsubscribe`) is still unimplemented after B-4.1. Variant has further use; do not delete. |
| **`unsubscribe` deferral** | Returns `NetError::Unimplemented { method: "Network::unsubscribe", planned_in: "B-4.2" }` | Wire it in B-4.1 | Iroh-gossip's `GossipTopic` self-cleans when all sender + receiver handles drop (rustdoc at `gossip/src/api.rs:207-210` — cleanup is implicit via dropped mpsc senders, no explicit `Drop` impl). The runtime's per-topic `IrohSubscription` lifetime ALREADY covers the practical "stop receiving" case. Explicit `unsubscribe` semantics (signal swarm departure) is a real concern but not blocking gossip ingestion — defer. |
| **Acceptance test shape** | 2 in-process `IrohNetwork` instances; both subscribe (peer B bootstraps from A's `EndpointId`); A publishes; B's `recv()` returns the bincode-decoded message | Mock the gossip layer | Real iroh-gossip end-to-end is the load-bearing acceptance evidence. Mocking just verifies the wire-encode/decode contract which is already covered by `wire_freeze.rs`. |
| **Test runtime** | `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` for all gossip-driven tests | Single-threaded current-thread | iroh-gossip spawns internal tasks (`gossip/src/net.rs:207`); single-threaded would deadlock waiting for those tasks if the test holds the runtime. Multi-thread is the safe choice. |
| **Test timeouts** | `tokio::time::timeout(Duration::from_secs(5), ...)` wrap on `recv()` calls that expect a specific message | No timeout | iroh-gossip swarm formation is best-effort; tests can hang indefinitely if NeighborUp never arrives. Bounded timeout makes failure modes diagnosable. |

## 3. Code surface

### 3.0 `NetError` + `SubError` variant additions

In `crates/network/src/lib.rs`:

```rust
#[derive(Debug, Error)]
pub enum NetError {
    // ... existing SubscribeClosed, PublishFailed, Unimplemented unchanged ...

    /// Subscribe call failed for a reason other than transport
    /// shutdown (e.g. invalid bootstrap peer pubkey, gossip-layer
    /// API error). Carries a human-readable diagnostic.
    #[error("subscribe failed: {0}")]
    SubscribeFailed(String),
}

#[derive(Debug, Error)]
pub enum SubError {
    // ... existing Lagged unchanged ...

    /// A received wire message did not decode under the canonical
    /// bincode contract. Carries the iroh-gossip last-hop neighbor
    /// (NOT the original publisher; that's Q-4 / B-4.2 work). The
    /// runtime should log + discard, NOT trigger backfill — distinct
    /// from `SubError::Lagged` which IS a backfill trigger per
    /// `runtime.rs` HeadsSummary publish path.
    #[error("decoded message failed bincode contract (from peer: {peer:?})")]
    DecodeFailed {
        /// The iroh-gossip `delivered_from` peer (last-hop, not
        /// necessarily the publisher). `None` for transports without
        /// per-message sender identity ([`MemNetwork`] never emits
        /// this variant).
        peer: Option<PeerPubkey>,
    },
}
```

The kernel runtime gains a `SubError::DecodeFailed` branch in its receive loop (`runtime.rs` `handle_event` / equivalent) that logs the failure and continues, without triggering `publish_heads_summary`.

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

**Import note**: `futures::StreamExt` is NOT a workspace dependency. iroh-gossip's example uses `futures_lite::StreamExt`; iroh-gossip's `GossipTopic` is a `n0_future::Stream` (`gossip/src/api.rs:15`). The cleanest path is adding `futures-lite = { workspace = true, optional = true }` to `myrhiza-network`'s deps under the `network-iroh` feature gate, and importing `use futures_lite::StreamExt;` in `iroh_transport.rs`. Spec §3.1 (B-4.0) Cargo wiring is extended accordingly:

```toml
# crates/network/Cargo.toml (B-4.1 additions)
[dependencies]
futures-lite = { workspace = true, optional = true }
bytes = { workspace = true, optional = true }    # explicit; avoid transitive reliance

[features]
network-iroh = ["dep:iroh", "dep:iroh-gossip", "dep:futures-lite", "dep:bytes"]
```

Workspace `Cargo.toml`:
```toml
futures-lite = "2"
bytes = "1"
```

Then the module:

```rust
// crates/network/src/iroh_transport.rs

use bytes::Bytes;
use futures_lite::StreamExt;
use iroh_gossip::{api::Event, GossipTopic, TopicId};
use myrhiza_types::canonical_bincode;
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
        // Convert each PeerPubkey → EndpointId via the B-4.0 free fn.
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

    async fn publish(
        &self,
        topic: Topic,
        message: GossipMessage,
    ) -> Result<(), NetError> {
        let topic_id = iroh_topic_id_from_topic(topic);
        let bytes = canonical_bincode()
            .serialize(&message)
            .map_err(|e| NetError::PublishFailed(format!("encode: {e}")))?;
        // TRADE-OFF: each publish re-subscribes (and `split()`s)
        // the GossipTopic. Per iroh-gossip's actor architecture
        // (`gossip/src/net.rs:600-643`), every `subscribe` call
        // spawns a fresh `topic_subscriber_loop` task; per-publish
        // this is task-spawn churn, not just allocation. The
        // GossipTopic departs the swarm when its sender + receiver
        // drop, so the cost is bounded per call.
        //
        // B-4.2/B-4.3 may cache a per-topic `GossipSender` on
        // `IrohNetwork` to avoid the churn — spec §11 deferred work.
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
                    // rather than a fatal close. TRADE-OFF: if the
                    // underlying gossip task has died, we'll spin.
                    // Mitigate in B-4.3 with explicit halt detection.
                    return Err(SubError::Lagged(0));
                }
                Some(Ok(Event::Received(msg))) => {
                    // Capture the iroh-gossip last-hop neighbor for
                    // attribution. NOT the original publisher (see
                    // spec §1 + Q-4 deferral).
                    let last_hop_peer: Option<PeerPubkey> =
                        Some(peer_pubkey_from_iroh(msg.delivered_from));
                    // Decode the bincode-encoded GossipMessage.
                    match canonical_bincode().deserialize::<GossipMessage>(&msg.content) {
                        Ok(decoded) => return Ok(Some(decoded)),
                        Err(_decode_err) => {
                            // Decode failure is distinct from Lagged —
                            // see spec §2 SubError::DecodeFailed row.
                            // Runtime treats this as log+discard, NOT a
                            // backfill trigger.
                            return Err(SubError::DecodeFailed {
                                peer: last_hop_peer,
                            });
                        }
                    }
                }
                Some(Ok(Event::Lagged)) => {
                    // Iroh-gossip drops the count; sentinel 0.
                    return Err(SubError::Lagged(0));
                }
                Some(Ok(Event::NeighborUp(_))) | Some(Ok(Event::NeighborDown(_))) => {
                    // Membership events — silently consume, continue.
                    continue;
                }
            }
        }
    }
}
```

**API-name verification at impl time** (per `prior-art/iroh/lessons.md` §Avoid row 1):

- `iroh_gossip::api::Event` has **exactly four** variants per `iroh-gossip-0.99.0/src/api.rs:332-345`: `Received(Message)`, `NeighborUp(EndpointId)`, `NeighborDown(EndpointId)`, `Lagged`. There is NO `Joined` variant (the audit prep correctly flagged this — earlier drafts of this spec mistakenly added one). The `GossipReceiver::joined() -> Result<(), ApiError>` method (`api.rs:299-304`) is a separate API for the "swarm has formed" signal; B-4.1 does NOT use it.
- `iroh_gossip::TopicId::from_bytes([u8; 32])` — verified `gossip/src/proto/state.rs:20-22`.
- `iroh_gossip::GossipTopic::split() -> (GossipSender, GossipReceiver)` per `gossip/src/api.rs:226-229`.
- `iroh_gossip::Gossip::subscribe(topic, Vec<EndpointId>) -> Result<GossipTopic, ApiError>` per `gossip/src/api.rs:157-167`.
- `iroh_gossip::api::Message` has fields `content: Bytes`, `scope: DeliveryScope`, `delivered_from: EndpointId` per `api.rs:362-372`.
- `GossipSender::broadcast(Bytes) async -> Result<(), ApiError>` per `api.rs:179-183`. Also `GossipTopic::broadcast` at `api.rs:232-234` for the unsplit shape.

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

### 3.4 Trait-change call-site impact — seven sites

Confirmed via grep — every site that calls `Network::subscribe` (or `Subscription::recv` from a subscribe result) needs updating. Spec audit enumerated **seven** sites:

```
crates/kernel/src/runtime.rs:400        — Runtime::start, the primary consumer
crates/kernel/src/runtime.rs:555        — NetworkErased trait-object wrapper
crates/kernel/tests/convergence.rs:584  — net_tap.subscribe(topic)
crates/network/tests/memory_basic.rs:29  — MemNetwork basic test
crates/network/tests/memory_basic.rs:48
crates/network/tests/memory_basic.rs:66
crates/network/tests/memory_basic.rs:104
```

Plus `crates/network/tests/iroh_skeleton.rs` (B-4.0 file): the `iroh_network_subscribe_returns_unimplemented` test (test #2 of that file) becomes invalid because B-4.1's subscribe succeeds. **DELETE that test entirely** — its invariant no longer holds. Keep test #1 (`iroh_network_constructs_and_exposes_endpoint_id_as_peer_pubkey`) unchanged.

Update pattern (all seven sites): change `subscribe(topic)` → `subscribe(topic, vec![])` (or the appropriate bootstrap vector at test sites that want real iroh swarm formation).

## 4. Acceptance tests

`crates/network/tests/iroh_gossip.rs` (new file; feature-gated):

| # | Test name | Flavor | Pattern |
|---|---|---|---|
| 1 | `two_peers_subscribe_and_exchange_a_single_event` | `multi_thread, worker_threads = 2` | Spin up endpoint+gossip+router for peer A and peer B in-process. B subscribes with bootstrap = `[a_id]`. A subscribes with empty bootstrap. Wait until B's swarm has formed (poll `recv()` consuming neighbor events until first non-error). A publishes a `GossipMessage::Event(...)` (use B-1's `EventBuilder` to build a valid event). B's next `recv()` returns the same `GossipMessage`. Use `tokio::time::timeout(Duration::from_secs(5), ...)`. |
| 2 | `subscribe_publishes_propagate_via_gossip_to_three_peers` | `multi_thread, worker_threads = 2` | Same shape, 3 peers in a chain (A↔B↔C). C's bootstrap is `[b_id]`; B's is `[a_id]`; A's is empty. A publishes; C must receive. Exercises Plumtree forwarding across one hop. |
| 3 | `decode_failure_surfaces_as_subscribe_decode_failed` | `multi_thread, worker_threads = 2` | Two peers as in #1. Peer A publishes garbage bytes via the gossip backdoor: `let (sender, _receiver) = gossip_a.subscribe(topic_id, vec![]).await?.split(); sender.broadcast(Bytes::from_static(b"garbage-not-bincode")).await?;`. B's `recv()` returns `Err(SubError::DecodeFailed { peer })` where `peer == Some(a_peer_pubkey)` (the iroh-gossip last-hop neighbor, which is A in this two-peer setup). |
| 4 | `unsubscribe_returns_unimplemented` | default | Construct an IrohNetwork, call `unsubscribe(topic).await`, assert `NetError::Unimplemented { method: "Network::unsubscribe", planned_in: "B-4.2" }`. |
| 5 | `topic_id_from_topic_roundtrips` | default | Pure unit test (no async): construct a `Topic::from_bytes([0xAA; 32])`, convert via `iroh_topic_id_from_topic`, assert `topic_id.as_bytes() == [0xAA; 32]`. |

The `Event::Lagged` mapping is **not** acceptance-tested in B-4.1 — reliably triggering the underlying `tokio::sync::broadcast::Receiver` overrun requires controlling iroh-gossip's internal `JoinOptions::subscription_capacity` from the test, which iroh-gossip 0.99.0 does not expose at the API boundary. Move this test to B-4.3's scope (real cross-process tests with broader harness).

**Test setup helper** — extract `spawn_iroh_peer(label: &str) -> (Endpoint, Gossip, Router, IrohNetwork)` into a `helpers` module in the test file to keep the per-test boilerplate down.

Spec-coverage annotations: tests 1, 2 → `networking.md §11.1`; tests 3, 4 → `convergence.md §4.4` (wire-decode failure); test 5 → `networking.md §11.1`; test 6 → `convergence.md §4.6` (topic-ID).

## 5. Justfile changes

The existing `just test-iroh` recipe (from B-4.0) already covers the new test file (it runs `cargo test -p myrhiza-network --features network-iroh --tests`). No Justfile changes needed.

## 6. Edge cases

- **Empty bootstrap**: legal. `Gossip::subscribe(topic, vec![])` accepts an empty bootstrap set per `gossip/examples/chat.rs:83` (open-mode). The topic exists locally; the swarm forms via inbound joins.
- **Bootstrap peer not reachable**: iroh-gossip's join is async + best-effort. The subscribe call succeeds even if no peer in `bootstrap` is reachable; `recv()` simply doesn't yield until *some* peer joins. Tests bound with `tokio::time::timeout`.
- **Stale `IrohSubscription` after `Drop`**: iroh-gossip's `GossipTopic` rustdoc at `gossip/src/api.rs:207-210` documents that topic-leave happens when all sender + receiver handles drop (no explicit `Drop` impl; cleanup is implicit via the dropped mpsc senders inside the actor). `recv()` returns `Ok(None)` thereafter; matches the trait's stream-end semantics.
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
- `iroh-gossip-0.99.0/src/api.rs:207-210` — `GossipTopic` rustdoc: topic departs when senders + receivers all drop (implicit cleanup; no explicit `Drop` impl).
- `iroh-gossip-0.99.0/src/api.rs:299-304` — `GossipReceiver::joined()` method (not used in B-4.1; reserved for future "wait for swarm formation" semantics).
- `iroh-gossip-0.99.0/src/net.rs:940` — `_` discards Lagged count.
- `iroh-gossip-0.99.0/examples/chat.rs` — canonical end-to-end pattern.
- `iroh-gossip-0.99.0/src/proto/state.rs:20-22` — `TopicId([u8; 32])`.
- [`prior-art/iroh/gossip.md`](../prior-art/iroh/gossip.md) — Plumtree + HyParView background.
- [`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md) §Borrow — kernel-owned Endpoint + Router patterns.
- [`docs/specs/2026-05-10-plan-b-1-dag-memnet-design.md`](2026-05-10-plan-b-1-dag-memnet-design.md) §6 — `Network` trait shape.
- [`docs/specs/2026-05-20-plan-b-4-0-iroh-skeleton-design.md`](2026-05-20-plan-b-4-0-iroh-skeleton-design.md) §1 §3.2 — B-4.0 skeleton baseline + deferral list pointing here.
