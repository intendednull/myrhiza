**Date:** 2026-05-21
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-4.5 — Kernel runtime integration of direct-stream HeadsRequest backfill

# Plan B-4.5 design — Kernel-side switchover to direct-stream backfill

## 1. Goal

Wire the kernel `Runtime` to use the direct-stream backfill plumbing landed in B-4.4. This is the first slice in which the new ALPN-multiplexed `request_heads` path is actually exercised against live traffic.

Concretely:

1. **Inbound (responder side)**: install a `KernelRequestHandler` on the Network at runtime startup. The handler is a thin shim that forwards inbound direct-stream requests into the runtime task via a new `HeadsRequestCommand` mpsc. The runtime task processes each request synchronously while owning DAG state, streaming events back through the supplied `HeadsResponder`.
2. **Outbound (requester side)**: when `handle_heads_summary` decides to back-fill, switch the emission from the broadcast `publish(GossipMessage::HeadsRequest(...))` path to a direct-stream `request_heads(remote.signed_by_peer, DirectHeadsRequest { topic, requests })` targeted at the peer that signed the summary. A drainer task forwards response events into a new `internal_event_rx` mpsc; the runtime's select loop pulls events off that channel and calls `handle_event` exactly as it would for gossip-delivered events.

**Out of scope (explicit deferrals)**:

- `request_author_chain_gap` (the Pending + InvalidChain paths at `runtime.rs:973` / `runtime.rs:1037`) stays on the gossip-routed path. Those call sites have no peer attribution available — they fire on Event receipt where the runtime only sees `event.author` (the original author, not the forwarder); routing them through direct-streams requires either a peer-authority index (B-4.6+ scope) or a per-Event last-hop attribution (a Subscription-trait change).
- The gossip-routed inbound handler (`handle_heads_request` at `runtime.rs:1222`) stays active for two reasons: it services broadcasts from peers that emitted via `request_author_chain_gap` (still gossip-routed in B-4.5), and it provides a backwards-compatible response surface for peers running pre-B-4.5 code. Removal lands once the gossip-routed emission paths are all converted.
- The `GossipMessage::HeadsRequest` wire variant stays for wire-freeze stability. Removal regenerates the snapshot and lands once `handle_heads_request` is fully retired.
- Cross-process tests, discovery / pkarr integration, per-requester rate limiting, peer-authority index, last-hop attribution on Event — all later slices.

## 2. Scope decisions (locked during brainstorming + runtime survey, 2026-05-21)

| Decision | Chosen | Runner-up | Why |
|---|---|---|---|
| **HeadsSummary-driven backfill only; Pending/InvalidChain stays on gossip** | `handle_heads_summary` is the ONLY emission site to switch in B-4.5. `request_author_chain_gap` (called from Pending + InvalidChain branches) keeps its `publish(GossipMessage::HeadsRequest(req))` body. | Switch all 3 emission sites in one slice. | The HeadsSummary path has a target peer in the signed payload (`remote.signed_by_peer` — added in B-4.2). The Pending/InvalidChain paths have only `event.author` (the original author, not necessarily a peer holding the events). Wiring those paths needs either a peer-authority index (substantial new state) or a Subscription-trait change to surface `delivered_from` per-event. Both are large; defer. |
| **Runtime owns DAG; handler shims via mpsc** | New `KernelRequestHandler { tx: mpsc::Sender<HeadsRequestCommand>, topic: Topic }` does only topic validation + forward. Runtime drains in its select loop and processes synchronously while owning DAG. | (a) Arc<RwLock<Dag>> shared between handler and runtime; (b) Cloneable DAG snapshot held by handler. | Option (a) breaks the single-owner invariant: every existing DAG read site (~30 in runtime.rs) would need to hold a read guard, and concurrent writes by the runtime would block readers. Option (b) requires the snapshot to track every author chain head + every Event byte — substantial cost per snapshot, and staleness during the snapshot lifetime. The mpsc Command pattern matches the existing `AuthorCommand` + `halt_watch_tx` patterns exactly; reviewers know it; the runtime task already has a biased select loop set up to drain commands. |
| **`HeadsRequestCommand` is processed inline in the select loop** | When the runtime drains `heads_req_rx.recv()`, it calls `self.serve_direct_heads_request(cmd).await` inside the select arm. The handler trait's `handle()` returns when the responder drops, which happens when this method returns. | Spawn a separate task per command. | The processing is just a DAG read loop (handle_heads_request's existing logic) with an mpsc send instead of a gossip publish. Running it on the runtime task preserves the single-owner invariant. Spawning would require sharing DAG read access, which we explicitly rejected above. Per-command runtime cost is bounded by the 256-events-per-EventRequest cap from B-4.2 plus mpsc backpressure on the responder channel. |
| **Response stream drainer = spawned task** | After `handle_heads_summary` calls `request_heads(target, req)`, it spawns `tokio::spawn(drain_heads_response(stream, internal_event_tx.clone()))`. The drainer reads frames, forwards Events into the runtime via `internal_event_tx`. The runtime's select loop picks up Events via a new arm and processes them via the existing `handle_event` path. | Drain inline. | Inline draining would block the runtime select loop until the response completes — could be hundreds of events. Spawned drainer + mpsc-back-to-runtime decouples response receipt from the runtime's other work. Cap mpsc capacity (32) provides natural backpressure on the drainer if the runtime falls behind. |
| **`internal_event_rx` events go through `handle_event` unchanged** | Events arriving via direct-stream response are processed identically to events arriving via gossip (`handle_event` already does sig verification, pre-check, DAG insert, replay). | New code path for direct-stream events. | Direct-stream events have no different trust properties than gossip events — both arrive on authenticated transports (mutual QUIC TLS or iroh-gossip's signed envelope). The signature verification in `handle_event` is load-bearing in both cases. Deduplication is implicit (DAG insert by hash is idempotent). |
| **Direct-stream failure = log + continue, no fallback** | If `request_heads` returns `Err(NetError::RequestFailed)` (peer unreachable, ALPN mismatch, no handler installed), the runtime pushes a `PeerWarning::DirectRequestFailed { peer, reason }` and continues. The next periodic HeadsSummary tick or fresh inbound HeadsSummary triggers a retry. | Fallback to broadcast `publish(GossipMessage::HeadsRequest)`. | Fallback would mean both paths active per request — confusing semantics and double-bandwidth. The periodic-retry semantic is sufficient because HeadsSummary is rebroadcast on a 1-second ticker (`cfg.heads_summary_tick`) and on `SubError::Lagged`. Worst case: one missed backfill cycle (~1s of staleness). |
| **Topic validation in handler shim** | `KernelRequestHandler::handle` checks `request.topic == self.topic` and returns immediately (drops `responder`, signaling clean EOF to requester) if not. Defense in depth — IrohNetwork bound the connection to this Network, but a misconfigured embedder could register the same handler on multiple topics. | No topic validation. | One-line guard. Cheap. Matches the "confused-deputy" warning in B-4.4 spec §3.1 (handler trait contract). |
| **Per-runtime install_request_handler is called once at start, with `Arc::new(handler)`** | `Runtime::start` constructs the channel, builds the `KernelRequestHandler`, calls `network.install_request_handler(Arc::new(handler))` BEFORE moving `network` into the runtime. | (a) Install lazily on first inbound request; (b) Install from a separate `RuntimeHandle::install_handler()` method. | Lazy install fights the design: handlers must be ready before any peer can connect. Separate method splits the install ritual across the embedder + handle — error-prone. Install-at-start matches the iroh-Router-builder convention (register before spawn). |
| **`Runtime` field order**: `heads_req_rx` + `internal_event_rx` + `internal_event_tx` | Three new fields on `Runtime`. `heads_req_rx` is the responder mailbox. `internal_event_rx` is the requester response mailbox. `internal_event_tx` is held by the runtime so it can clone into drainer tasks. | One bidirectional channel. | Bidirectional would conflate request semantics with response semantics. Two distinct channels read cleaner in the select loop. |
| **No changes to wire-freeze, gossip-path verifiers, drift, equivocation logic** | Receive-side gossip dispatch (`handle_message` at `runtime.rs:731`) keeps all four arms exactly as today. The HeadsRequest gossip arm continues to verify + call `handle_heads_request` (which still publishes events back via gossip). | Remove the HeadsRequest gossip arm. | The arm services peers running pre-B-4.5 code AND the Pending/InvalidChain paths in the LOCAL runtime that still emit broadcasts. Removing it would break both. Removal happens after `request_author_chain_gap` is converted (post-B-4.5). |
| **Acceptance tests use existing MemNetwork infrastructure** | New tests at `crates/kernel/tests/direct_backfill.rs` use the same MemNetwork + MemBus + Runtime::start pattern as `convergence.rs`. No new test harness. | Dedicated harness file. | The MemNetwork peer-keyed refactor from B-4.4 already plumbs distinct peer pubkeys; tests can verify direct-stream backfill end-to-end without iroh complexity. Iroh-tier integration tests deferred. |
| **`net_a`/`net_tap` carryover from Task #54** | Audit the attribution.rs synthetic-publisher MemNetworks during B-4.5. If any test now relies on Runtime::start invoking `install_request_handler`, fix the pubkey alignment then. Otherwise the synthetic-publisher pattern remains valid (no runtime attached → no handler-registry collision). | Force a sweep regardless. | The TASK #54 brief notes these MemNetworks have NO Runtime attached; B-4.5 introduces Runtime::start → install_request_handler, but that only affects MemNetworks that HAVE a Runtime. Synthetic publishers stay synthetic. Audit-and-verify, then move on. |

## 3. Code surface

### 3.1 `KernelRequestHandler` + `HeadsRequestCommand` — `crates/kernel/src/runtime.rs`

```rust
use myrhiza_network::request::{HeadsResponder, RequestHandler};

/// Command sent to the runtime task by [`KernelRequestHandler`] when
/// an inbound direct-stream HeadsRequest arrives.
///
/// The runtime drains these from `heads_req_rx` in its select loop and
/// processes them via [`Runtime::serve_direct_heads_request`].
///
/// Per B-4.5 spec §3.1.
pub(crate) struct HeadsRequestCommand {
    /// QUIC-TLS-confirmed pubkey of the peer that issued the request.
    pub(crate) requester: PeerPubkey,
    /// The decoded request payload (already topic-validated by the
    /// handler shim).
    pub(crate) request: DirectHeadsRequest,
    /// Sender half of the response stream; the runtime pushes events
    /// through `responder.send(event)`; dropping the responder signals
    /// clean EOF to the requester.
    pub(crate) responder: HeadsResponder,
}

/// `RequestHandler` impl installed by [`Runtime::start`] on the
/// underlying [`Network`]. Forwards inbound direct-stream requests to
/// the runtime task via mpsc; does topic validation (defense in depth)
/// to prevent the same handler being misregistered on a different
/// topic.
///
/// Per B-4.5 spec §3.1.
pub(crate) struct KernelRequestHandler {
    /// Sender half of the runtime's inbound-direct-request mailbox.
    tx: mpsc::Sender<HeadsRequestCommand>,
    /// The topic this handler services. Inbound requests for any other
    /// topic are silently dropped (clean EOF).
    topic: Topic,
}

#[async_trait::async_trait]
impl RequestHandler for KernelRequestHandler {
    async fn handle(
        &self,
        requester: PeerPubkey,
        request: DirectHeadsRequest,
        responder: HeadsResponder,
    ) {
        // Defense in depth: confirm the request targets the topic this
        // runtime services. The IrohNetwork already routes by
        // peer+ALPN; this check guards against an embedder that
        // registers the same handler against multiple per-topic
        // networks by mistake.
        if request.topic != self.topic {
            // Drop responder — requester sees clean EOF.
            return;
        }
        // Forward to runtime. If the runtime task has exited, the
        // send fails; we drop the responder which yields EOF to the
        // requester. No diagnostic surfaced — the runtime has already
        // shut down, there's nothing to log into.
        let _ = self
            .tx
            .send(HeadsRequestCommand {
                requester,
                request,
                responder,
            })
            .await;
    }
}
```

### 3.2 `PeerWarning::DirectRequestFailed` variant — `crates/kernel/src/runtime.rs`

Add to the existing `PeerWarning` enum (alongside `TransportError`, `BroadcastLagged`, etc.):

```rust
/// A direct-stream `request_heads` call to a peer failed before any
/// events were streamed back — typically because the peer is
/// unreachable, hasn't registered the heads-request ALPN, or has no
/// handler installed. The runtime continues; the next periodic
/// HeadsSummary tick or fresh inbound HeadsSummary will trigger a
/// retry. Per B-4.5 spec §3.4.
DirectRequestFailed {
    /// The target peer the request was directed at.
    peer: PeerPubkey,
    /// Human-readable diagnostic from `NetError::RequestFailed`.
    reason: String,
},
```

### 3.3 Runtime struct fields — `crates/kernel/src/runtime.rs`

Add three fields to `Runtime`:

```rust
struct Runtime {
    // ... existing fields ...

    /// Mailbox for inbound direct-stream HeadsRequest commands. The
    /// `KernelRequestHandler` installed on `network` at startup is the
    /// only sender. Drained by the select loop's
    /// `heads_req_rx.recv()` arm.
    heads_req_rx: mpsc::Receiver<HeadsRequestCommand>,

    /// Mailbox for events arriving on direct-stream backfill
    /// responses. The drainer task spawned by `handle_heads_summary`
    /// is the only sender (cloned from `internal_event_tx`).
    /// Drained by the select loop's `internal_event_rx.recv()` arm
    /// and processed via `handle_event`.
    internal_event_rx: mpsc::Receiver<Event>,

    /// Sender half of `internal_event_rx`, retained by the runtime so
    /// it can be cloned into drainer tasks. Cloning into the drainer
    /// (rather than passing the receiver) means multiple in-flight
    /// backfill responses can all feed events into the same channel.
    internal_event_tx: mpsc::Sender<Event>,
}
```

Channel capacities:
- `heads_req_rx`: 32 (matches `HEADS_STREAM_CHANNEL_CAPACITY` from B-4.4).
- `internal_event_rx`: 128 (a backfill response can be up to 256 events per request; 128 provides backpressure without starving inbound gossip).

### 3.4 `Runtime::start` wiring — `crates/kernel/src/runtime.rs`

Modify `Runtime::start` (currently `runtime.rs:444-524`):

The constructor erases the parameter `network: N` into `NetworkErased<N>` at `runtime.rs:454`, then subscribes via `erased.subscribe(...)` at line 458, then wraps in `Arc::new(erased)` at line 478 when populating the `Runtime` struct. **The install call must run on `erased` after line 454 and before line 478.** Calling on the raw `network: N` parameter after line 454 is a use-after-move; calling on `Arc::new(erased)` would require cloning the Arc twice for the handler reference, which is unnecessary since `Network::install_request_handler` takes `&self`.

Concrete wiring:

```rust
let erased = NetworkErased::new(network);
// B-4.* will plumb peer-discovery into Runtime::start; for now ...
let sub = erased.subscribe(topic, vec![]).await?;

let (author_tx, author_rx) = mpsc::channel(64);
// NEW (B-4.5): create the direct-stream channels BEFORE building the handler.
let (heads_req_tx, heads_req_rx) = mpsc::channel::<HeadsRequestCommand>(32);
let (internal_event_tx, internal_event_rx) = mpsc::channel::<Event>(128);

// NEW (B-4.5): install the handler on the erased network. The trait
// method takes &self; NetworkErased delegates the call to the inner N.
// Must run before `Arc::new(erased)` below — Arc::new consumes the erased
// value into the Runtime field.
let handler = KernelRequestHandler {
    tx: heads_req_tx,
    topic,
};
erased.install_request_handler(Arc::new(handler));

// ... existing log/digest/watch construction unchanged ...

let mut runtime = Runtime {
    network: Arc::new(erased),  // line 478 — unchanged
    // ... existing fields ...
    // NEW (B-4.5) fields:
    heads_req_rx,
    internal_event_rx,
    internal_event_tx,
    consecutive_transport_errors: 0,  // existing
};
```

### 3.5 `Runtime::serve_direct_heads_request` — `crates/kernel/src/runtime.rs`

New method, structurally similar to the existing `handle_heads_request` (currently `runtime.rs:1222`) but pushing to `cmd.responder` instead of publishing to gossip:

```rust
/// Service an inbound direct-stream HeadsRequest. Mirrors the
/// per-EventRequest loop from [`Self::handle_heads_request`] but
/// streams events to `cmd.responder` instead of broadcasting them on
/// the gossip topic.
///
/// Bound: a single `EventRequest` may cover at most 256 events
/// (`to_seq - from_seq <= 255`); over-sized requests are silently
/// dropped per the same rule as `handle_heads_request`.
///
/// If `cmd.responder.send(event).await` returns `Err`, the requester
/// dropped the stream — stop processing further events.
///
/// Per B-4.5 spec §3.5.
async fn serve_direct_heads_request(&mut self, cmd: HeadsRequestCommand) {
    // `requester` is captured for future per-peer rate-limit hooks
    // (B-4.6+); currently unused but documented intent.
    let _requester = cmd.requester;
    let responder = cmd.responder;

    for r in cmd.request.requests {
        if r.to_seq < r.from_seq {
            continue;
        }
        if r.to_seq.saturating_sub(r.from_seq) > 255 {
            continue;
        }
        let Some(chain) = self.dag.author_chain(&r.author) else {
            continue;
        };
        // Snapshot the (seq, hash) pairs before any await so we don't
        // hold an immutable borrow of `self.dag` across responder.send.
        let pairs: Vec<(u64, EventHash)> = (r.from_seq..=r.to_seq)
            .filter_map(|seq| chain.seq_to_hash.get(&seq).copied().map(|h| (seq, h)))
            .collect();
        for (_, hash) in pairs {
            if let Some(e) = self.dag.get(&hash).cloned() {
                if responder.send(e).await.is_err() {
                    // Requester dropped the stream — stop early.
                    return;
                }
            }
        }
    }
    // Responder drops at end of function -> requester sees clean EOF.
}
```

### 3.6 `handle_heads_summary` switchover — `crates/kernel/src/runtime.rs`

Currently at `runtime.rs:1049-1055`:

```rust
if !requests.is_empty() {
    let req = self.build_signed_heads_request(requests)?;
    let _ = self
        .network
        .publish(self.topic, GossipMessage::HeadsRequest(req))
        .await;
}
```

Replace with:

```rust
if !requests.is_empty() {
    self.issue_direct_backfill(remote.signed_by_peer, requests).await;
}
```

New method:

```rust
/// Issue a direct-stream backfill request to `target_peer` for the
/// given `requests`. Spawns a drainer task that forwards response
/// events into the runtime's `internal_event_rx` mailbox; the runtime
/// select loop picks them up and processes them via `handle_event`
/// exactly as if they had arrived through gossip.
///
/// On `NetError::RequestFailed`, pushes a
/// `PeerWarning::DirectRequestFailed` and returns. The next periodic
/// HeadsSummary tick or fresh inbound HeadsSummary triggers a retry.
///
/// Per B-4.5 spec §3.6.
async fn issue_direct_backfill(
    &mut self,
    target_peer: PeerPubkey,
    requests: Vec<myrhiza_types::EventRequest>,
) {
    let direct_req = DirectHeadsRequest {
        topic: self.topic,
        requests,
    };
    let stream = match self.network.request_heads(target_peer, direct_req).await {
        Ok(s) => s,
        Err(e) => {
            #[allow(clippy::expect_used)]
            self.peer_warnings
                .lock()
                .expect("peer_warnings mutex poisoned")
                .push(PeerWarning::DirectRequestFailed {
                    peer: target_peer,
                    reason: format!("{e}"),
                });
            return;
        }
    };
    // Spawn a drainer task that forwards each Event from the stream
    // into the runtime's internal_event_rx mailbox. The select loop
    // picks them up and calls handle_event.
    let tx = self.internal_event_tx.clone();
    tokio::spawn(drain_heads_response(stream, tx));
}
```

### 3.7 `drain_heads_response` helper — `crates/kernel/src/runtime.rs`

```rust
use myrhiza_network::request::HeadsStream;

/// Drainer task that consumes a [`HeadsStream`] from a direct-stream
/// backfill response and forwards Events into the runtime's
/// `internal_event_rx` mailbox. The runtime processes them via
/// `handle_event`.
///
/// Errors from the stream (`HeadsStreamError::Transport`,
/// `::Decode`, `::Handler`) terminate the drainer silently; the
/// missing events surface as gaps on the next HeadsSummary cycle.
///
/// Per B-4.5 spec §3.7.
async fn drain_heads_response(
    mut stream: HeadsStream,
    tx: mpsc::Sender<Event>,
) {
    while let Some(item) = stream.next().await {
        match item {
            Ok(event) => {
                if tx.send(event).await.is_err() {
                    // Runtime task gone; stop draining.
                    return;
                }
            }
            Err(_e) => {
                // Stream-level error — terminate. The next HeadsSummary
                // cycle will surface the same gap and retry.
                return;
            }
        }
    }
}
```

### 3.8 Select-loop new arms — `crates/kernel/src/runtime.rs`

The existing select loop is at `runtime.rs:538-609`. Add two arms after the `author_rx` arm, before `sub.recv()`:

```rust
loop {
    tokio::select! {
        biased;
        // ... existing author_rx arm ...

        // NEW: drain inbound direct-stream requests.
        Some(cmd) = self.heads_req_rx.recv() => {
            self.serve_direct_heads_request(cmd).await;
        }

        // NEW: drain events from direct-stream backfill responses.
        Some(event) = self.internal_event_rx.recv() => {
            // handle_event already does signature verification,
            // pre-check, DAG insert, replay — identical to gossip path.
            let _ = self.handle_event(event).await;
        }

        // ... existing sub.recv() arm unchanged ...

        // ... existing ticker arm unchanged ...
    }
}
```

**Biased ordering rationale**: keep `author_rx` first (locally-issued events have priority); insert the two new arms next so direct-stream requests + responses are processed ahead of gossip backlog; then `sub.recv()`; then the ticker. The direct-stream paths have explicit per-peer pacing (one request = one stream); inbound gossip can flood; keeping direct-stream ahead prevents backfill from being starved by gossip backlog.

### 3.9 `RuntimeHandle::peer_warnings` already exposes `DirectRequestFailed`

No change to `RuntimeHandle` — `peer_warnings` is already an `Arc<Mutex<Vec<PeerWarning>>>` exposed via `RuntimeHandle::peer_warnings`. Tests can read the warnings vec to assert that a failed direct-backfill was logged.

## 4. Acceptance tests

### 4.1 New file: `crates/kernel/tests/direct_backfill.rs`

**Test 1: `direct_backfill_two_peer_convergence_over_mem`**

- Peer A authors 5 events.
- Peer B starts behind (empty DAG).
- A's HeadsSummary arrives at B (via gossip ticker or explicit send).
- B detects gap, issues `request_heads(A, …)` via direct-stream.
- A's `serve_direct_heads_request` streams the 5 events back.
- The drainer pushes them into B's `internal_event_rx`.
- B's `handle_event` applies them.
- Final assertion: B's `digest_watch` matches A's `digest_watch`.

**Test 2: `direct_backfill_target_peer_unreachable_logs_warning`**

- Peer B set up alone (no peer A).
- Hand-forge a HeadsSummary signed by `pub_a` (where pub_a is a non-existent peer pubkey on the bus). Inject into B via `bus.send`.
- B's `handle_heads_summary` issues `request_heads(pub_a, …)`.
- MemNetwork: no handler for pub_a → `request_heads` returns `Err(NetError::RequestFailed)`.
- Assert: B's `peer_warnings` contains a `DirectRequestFailed { peer: pub_a, .. }` entry.
- Assert: B is still alive (next operation succeeds).

**Test 3: `direct_backfill_handler_topic_validation_drops_wrong_topic`**

- Peer A on topic_a, Peer B on topic_a, Peer C on topic_b. All three on the same MemBus.
- C issues `request_heads(A, DirectHeadsRequest { topic: topic_b, ... })` (mismatched topic in the request body — A's handler is bound to topic_a).
- Assert: A's handler invokes the topic-validation drop; C's stream `next().await` returns `None` immediately (clean EOF, zero events).

**Test 4: `direct_backfill_legacy_gossip_routed_request_still_serviced`**

- Peer A on topic_a. Peer B on topic_a.
- B sends a hand-forged `GossipMessage::HeadsRequest(req)` via `network.publish(topic, GossipMessage::HeadsRequest(req))`.
- B subscribes to the topic to capture A's response.
- Assert: A's existing `handle_heads_request` services it (events published back via gossip).

**Test 5: `direct_backfill_pending_path_still_uses_gossip_routed`**

- Two-peer setup. A authors events 1, 2, 3 but B only receives 3 (forge by buffering or selective publish).
- B sees event 3, depends on 1+2 it doesn't have → Pending → `request_author_chain_gap` fires.
- Assert: B published a `GossipMessage::HeadsRequest` (still gossip-routed for the Pending path) — captured via a tap subscription.
- Assert: A's `handle_heads_request` services it, events 1+2 arrive at B, B converges.

**Test 6: `direct_backfill_multiple_concurrent_backfills_do_not_collide`**

- Three peers: A, B, C all on shared bus. A has events {1..3}, B has events {4..6}, C is empty.
- Send signed HeadsSummary from A → C and from B → C in rapid succession.
- C issues TWO concurrent `request_heads` calls (one to A, one to B). Two drainer tasks. Both feed `internal_event_rx`.
- Assert: C ends up with events {1..6} from both peers.

### 4.2 Carryover audit (Task #54)

In `crates/kernel/tests/attribution.rs`, locate the `net_a` / `net_tap` synthetic-publisher MemNetworks in tests 7+8 (the ones using `[0xE1; 32]` / `[0xE2; 32]`). Verify under B-4.5:

- No `Runtime::start` is invoked with `net_a` / `net_tap` → they remain pure bus-injection MemNetworks with no handler registered.
- The synthetic pubkey choice has no effect on direct-stream routing because no peer-A direct-stream request is exercised in those tests.
- No fix required; document the pattern in a code comment so future readers know the synthetic-pubkey choice is intentional.

If the audit reveals any test where `net_a` / `net_tap` IS attached to a Runtime, fix by using the runtime's PeerKeypair's pubkey (matches the convention now established by all other Runtime-attached MemNetworks post-B-4.4).

## 5. Justfile changes

`just spec-coverage` regeneration — add B-4.5 spec to the cited-by map.

## 6. Edge cases

| Scenario | Behavior |
|---|---|
| Request handler fires but DAG is empty | The per-EventRequest `dag.author_chain(&r.author)` lookup returns `None`; the loop continues to the next request. Responder drops; requester sees clean EOF (zero events). |
| Requester sends `requests: vec![]` | Outer loop has zero iterations; responder drops immediately; clean EOF. |
| Direct-stream response yields a duplicate Event | `handle_event` calls `dag.insert(event)` which is idempotent on hash collision. Replay continues normally. |
| Direct-stream response yields a malformed Event | `handle_event` catches it via signature verification or pre-check; event is rejected; no DAG mutation. |
| `request_heads(self.peer_key.public, ...)` (loopback) | MemNetwork: handler installed locally; KernelRequestHandler topic-validates and runs `serve_direct_heads_request` on the SAME runtime task — deadlock risk because the runtime is awaiting `request_heads().await` while the handler waits to run on the same task. **Mitigation**: `handle_heads_summary` skips backfill when `target_peer == self.peer_key.public`. Same loopback filter the B-4.2 spec applied to verify-side handling. |
| Runtime shutdown while a drainer task is mid-stream | The drainer's `tx.send(event).await` returns Err once `internal_event_rx` drops. Drainer exits cleanly. |
| `heads_req_rx` channel full | Send from handler shim returns `Err`. Responder drops; requester sees clean EOF. The protocol's natural retry (next HeadsSummary tick on the requester) will re-issue. |
| Two concurrent `request_heads` to same target peer | IrohNetwork opens two independent QUIC bidi streams (multiplexed on one Connection). MemNetwork spawns two handler tasks. No collision — handler is `&self`. Drainers both feed into the same `internal_event_rx`. Deduplication is implicit in `handle_event`. |

### Loopback filter detail

Add to `handle_heads_summary`, just before the `issue_direct_backfill` call:

```rust
if !requests.is_empty() {
    // Loopback guard: never issue a direct-stream backfill to
    // ourselves. The handler runs on this very task; awaiting the
    // request would deadlock. The HeadsSummary signed by our own
    // pubkey arrives via MemNetwork's broadcast — we receive our own
    // emit. Drop the backfill attempt.
    //
    // The Runtime's local pubkey is `self.peer_key.public` —
    // `peer_key: PeerKeypair` is the Ed25519 keypair field (matches
    // the existing loopback filter in `verify_heads_summary` at
    // `runtime.rs:1585`).
    if remote.signed_by_peer == self.peer_key.public {
        return Ok(());
    }
    self.issue_direct_backfill(remote.signed_by_peer, requests).await;
}
```

The B-4.2 spec already documents that we receive our own HeadsSummary via the broadcast loopback in MemNetwork. B-4.2 added a verify-side filter for the same loopback. The same filter applies here for backfill.

## 7. Surface change summary

**New crate-public surface**:

- `myrhiza_kernel::runtime::PeerWarning::DirectRequestFailed { peer, reason }` — new variant.
- `myrhiza_kernel::runtime::HeadsRequestCommand` — `pub(crate)` (internal).
- `myrhiza_kernel::runtime::KernelRequestHandler` — `pub(crate)` (internal; embedders use Runtime::start as before).

**Internal surface**:

- `Runtime::heads_req_rx`, `Runtime::internal_event_rx`, `Runtime::internal_event_tx` — new fields.
- `Runtime::serve_direct_heads_request` — new method.
- `Runtime::issue_direct_backfill` — new method.
- `drain_heads_response` — new free function.

**Unchanged**:

- `Runtime::start` signature (no new parameter — channels are constructed internally; handler is installed internally).
- `RuntimeHandle` (peer_warnings already surfaces the new variant via the existing Vec).
- `handle_heads_request` (gossip-routed inbound) — STAYS active.
- `request_author_chain_gap` (Pending/InvalidChain emitter) — STAYS gossip-routed.
- `GossipMessage::HeadsRequest` wire variant — STAYS.

## 8. Non-goals (explicit)

- **Convert `request_author_chain_gap` to direct-streams.** Needs per-event last-hop attribution or a peer-authority index. B-4.6+.
- **Remove `GossipMessage::HeadsRequest` variant.** Wire-freeze regen. After `request_author_chain_gap` is converted.
- **Remove `handle_heads_request`.** After `GossipMessage::HeadsRequest` variant is removed.
- **Cross-process tests** for direct-stream backfill. In-process MemNetwork tests give protocol coverage.
- **Per-requester rate limiting at the responder.** A peer with a valid QUIC connection could spam `request_heads`. Bounded by the 256-events-per-EventRequest cap from B-4.2 + the 32-deep `heads_req_rx` channel + iroh's connection-level flow control. Real rate-limiting is B-4.6+.
- **Retry policy for failed direct-stream requests.** The periodic HeadsSummary tick + new-inbound-HeadsSummary semantics already provide implicit retry.
- **Iroh-tier acceptance tests** for the kernel-side runtime integration. The MemNetwork tests verify protocol shape; iroh integration is exercised by the B-4.4 IrohNetwork tests plus this slice's MemNetwork tests.
- **Backfill of `PeerWarning::SignatureInvalid` into `process_drift_message`** (B-4.2 §10 carryover).
- **Lagged-event mapping test on real iroh-gossip** (B-4.1 §4 carryover).
- **`HeadsStreamError::Handler` variant population** — currently reserved (B-4.4 final-review revision §3). B-4.5's runtime handler never errors (it processes synchronously); if a future handler needs to surface internal errors, it'd push via the variant. Out of scope for now.

## 9. Prior-art consultation

- [`prior-art/iroh/architecture.md`](../prior-art/iroh/architecture.md) §"ALPN-based protocol multiplexing" — confirms the install-handler-once-at-startup convention.
- [`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md) §Borrow row 2 — "Router-owned by embedder, not by the transport library." Same pattern applies to handler installation: embedder (Runtime::start) installs once at construction.
- [`docs/specs/2026-05-20-plan-b-4-2-attribution-design.md`](2026-05-20-plan-b-4-2-attribution-design.md) §3.0 — `signed_by_peer` field on HeadsSummary is THE source of target-peer attribution for direct-stream backfill. Without B-4.2's attribution, this slice would have no way to know who to ask.
- [`docs/specs/2026-05-21-plan-b-4-4-direct-streams-design.md`](2026-05-21-plan-b-4-4-direct-streams-design.md) §3.1 — `RequestHandler` trait contract (topic-validation MUST be done by handlers) + §3.4.2 (HeadsRequestProtocol::accept lifecycle).
- [`docs/specs/2026-05-09-myrhiza-master-design/convergence.md`](2026-05-09-myrhiza-master-design/convergence.md) §4.2 + §4.6 — backfill semantics: HeadsSummary → diff → request gap → events. The shape doesn't change; only the wire path.

**Gaps**: no new prior-art gaps surfaced. The B-4.4 impl-time verifications (`SendStream::finish`, `RecvStream::read_exact`, `ProtocolHandler` signature) are all resolved.

## 10. Future work — explicit deferrals

- **B-4.6** — Convert `request_author_chain_gap` to direct-streams via either:
  - A peer-authority index (track which peers signed HeadsSummary for which authors), OR
  - A Subscription-trait extension to surface `delivered_from` for Event variants.
- **B-4.7** — Remove `GossipMessage::HeadsRequest` variant + `handle_heads_request` (gossip-routed inbound). Wire-freeze regen.
- **Cross-process / cross-machine kernel-tier tests.**
- **Per-requester rate limiting** at the responder side.
- **Discovery / pkarr / DHT integration.**
- **`Lagged`-event mapping test on real iroh-gossip** (B-4.1 §4 carryover).
- **`PeerWarning::SignatureInvalid` backfill in `process_drift_message`** (B-4.2 §10 carryover).

## 11. Sources

- `crates/kernel/src/runtime.rs:261-275` — `AuthorCommand` enum (template for `HeadsRequestCommand`).
- `crates/kernel/src/runtime.rs:284-340` — `RuntimeHandle` shape (peer_warnings exposure).
- `crates/kernel/src/runtime.rs:402-411` — Runtime struct private fields.
- `crates/kernel/src/runtime.rs:436-510` — `Runtime::start` (modified in §3.4).
- `crates/kernel/src/runtime.rs:531-609` — select loop (modified in §3.8).
- `crates/kernel/src/runtime.rs:973-989` — `request_author_chain_gap` (STAYS gossip-routed).
- `crates/kernel/src/runtime.rs:998-1057` — `handle_heads_summary` (modified at line 1049 in §3.6).
- `crates/kernel/src/runtime.rs:1222-1248` — `handle_heads_request` (STAYS active; template for `serve_direct_heads_request`).
- `crates/network/src/lib.rs` — `Network` trait `request_heads` + `install_request_handler` (B-4.4).
- `crates/network/src/request.rs` — `RequestHandler`, `HeadsStream`, `HeadsResponder`, `HEADS_STREAM_CHANNEL_CAPACITY`.
- [`docs/specs/2026-05-20-plan-b-4-2-attribution-design.md`](2026-05-20-plan-b-4-2-attribution-design.md).
- [`docs/specs/2026-05-21-plan-b-4-4-direct-streams-design.md`](2026-05-21-plan-b-4-4-direct-streams-design.md).
- [`docs/specs/2026-05-09-myrhiza-master-design/convergence.md`](2026-05-09-myrhiza-master-design/convergence.md).
- [`prior-art/iroh/architecture.md`](../prior-art/iroh/architecture.md).
- [`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md).
