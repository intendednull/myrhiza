**Date:** 2026-05-10
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-1 — Event DAG + MemNetwork + cross-peer convergence

# Plan B-1 design — DAG + MemNetwork + cross-peer convergence

## 1. Goal

Land the load-bearing acceptance evidence named in the plan-A handoff
(`docs/reports/2026-05-09-myrhiza-foundation-handoff.md` §"Plan B
starting point" #4): two in-process kernels, shared bundle, concurrent
multi-author event authoring, both peers converge to identical
`state-digest()` output. Plus the master-spec normative pieces this
test exercises: the event DAG, topo-sort, HeadsSummary sync protocol,
PendingBuffer, drift detection, and the topic-ID derivation that makes
MVP acceptance criterion #4 (coexistence) demonstrable.

This slice lands **none** of: iroh transport (B-4), persistent
identity / bech32m (B-2), module-dep recursion (B-3), revocation topic
(B-5), host-call fuel wiring (B-6), persistent DAG (B-7).

## 2. Scope decisions (locked during brainstorming, 2026-05-10)

| Decision | Chosen | Runner-up | Why |
|---|---|---|---|
| Sync protocol | Full HeadsSummary + PendingBuffer | Live-gossip only | iroh impl in B-4 drops in without retrofitting sync protocol; spec §4.2 is normative |
| Drift detection | Include in B-1 | Defer to B-2 | Spec §4.7 commits "v1 ships drift detection"; mechanism orthogonal to peer-identity persistence (in-memory stub suffices) |
| Topic-ID | Real derivation, manual seed in tests | Hardcode | Covers MVP §15.1 #4 coexistence; `app_bundle_hash` already produced by plan-A manifest pipeline |
| State-propose | Deferred; test-utils signs | Wire state-propose now | YAGNI for convergence test; state-propose lands when interaction profile does. When it lands, §9.2's `host.random` seed-injection path becomes mandatory — test-utils manual-seed shortcut is B-1-only. |
| DAG storage | In-memory BTreeMap | sled / redb | Acceptance test in-process; persistence is B-7 |
| Peer keypair | In-memory stub | Persistent now | Drift needs peer-scoped signing; full persistent identity is B-2 |
| Acceptance | 2 peers + concurrent multi-author | Single originator | Stronger topo-sort tie-break coverage |
| Architecture | Async Network trait + kernel Runtime loop | Sync test-driven | async-fn-in-trait is executor-agnostic; native uses tokio, browser plugs in wasm-bindgen-futures impl without trait churn; B-4 iroh drops in without retrofit |

## 3. Crate + module layout

New crate **`crates/network`**:

```
crates/network/
├── Cargo.toml
└── src/
    ├── lib.rs              — Network trait, NetError, GossipMessage enum
    ├── memory.rs           — MemBus + MemNetwork impl
    └── subscription.rs     — Subscription trait + MemSubscription
```

The iroh implementation lands in this same crate behind a
`network-iroh` cargo feature in B-4. The trait shape established in
B-1 is the interface contract.

Kernel additions:

```
crates/kernel/src/
├── dag.rs                  — EventDag, AuthorChain, DagError
├── pending.rs              — PendingBuffer (TTL + capacity eviction)
├── runtime.rs              — Runtime, RuntimeError, AuthorCommand
├── drift.rs                — DriftEmitter, drift compare logic
├── peer_identity.rs        — PeerKeypair (in-memory stub; B-2 replaces)
└── lib.rs                  — re-exports
```

Types extensions (`crates/types/src/`):

```
├── dag.rs                  — HeadsSummary, AuthorHead, HeadsRequest,
│                            EventRequest, DriftAnchor, AuthorSeq,
│                            DriftMessage (canonical-bincode shapes
│                            matching spec §4.2 + §4.7 byte-for-byte)
├── peer.rs                 — PeerPubkey
└── topic.rs                — extends with Topic::derive(...)
```

Plan-A's `crates/types/src/event.rs::serde_signature` (a private module
that serializes `[u8; 64]` as `serde_bytes::Bytes`) is promoted to a
crate-public helper `crate::serde_signature_64` so `DriftMessage` and
future structs can reuse it without duplicating the serde glue. The
plan-A `Event::signature` field continues to use it; behavior
unchanged.

Test-utils extensions (`crates/test-utils/src/`):

```
├── event_builder.rs        — EventBuilder, AuthorKeypair
└── harness.rs              — InProcessHarness, PeerHandle
```

## 4. EventDag

### 4.1 Storage

```rust
pub struct EventDag {
    by_hash: BTreeMap<EventHash, Event>,
    by_author: BTreeMap<AuthorPubkey, AuthorChain>,
    parents_to_children: BTreeMap<EventHash, BTreeSet<EventHash>>,
    topic: Topic,
    app_bundle_hash: BundleHash,
}

pub struct AuthorChain {
    head_seq: u64,            // 0 if no events yet from this author (sentinel)
    head_hash: EventHash,     // EventHash::ZERO when head_seq == 0; not a real event hash
    seq_to_hash: BTreeMap<u64, EventHash>,
}
```

All collections sorted for canonical iteration. `by_hash` keyed on
`Event::wire_hash()` (BLAKE3 of full canonical envelope including
signature). `parents_to_children` is the inverted edge index used by
Kahn's algorithm — for each `e ∈ by_hash`, the union of `{e.prev}`
(unless `prev == EventHash::ZERO`) and `e.deps` are the parents.

Additionally, `EventDag` stores a per-event indegree counter populated at insert time:

```rust
indegree: BTreeMap<EventHash, usize>,   // # of unvisited parents at insert
```

`indegree` is the canonical source of truth for parent count, computed exactly once during `insert`. The topo-sort algorithm (§4.3) consumes a snapshot of this map and decrements via `parents_to_children`; it does NOT re-derive parents from event fields. This eliminates the dual-computation hazard (insert-time parent logic must equal topo-sort parent logic — keeping the index as the source of truth makes that invariant structural).

### 4.2 Insert contract

```rust
pub enum Inserted {
    NewlyApplied { topo_index: u64, hash: EventHash },
    AlreadyKnown,
    Pending(BTreeSet<EventHash>),  // missing deps; caller stashes
}

impl EventDag {
    pub fn insert(&mut self, event: Event) -> Result<Inserted, DagError>;
    pub fn topo_sort(&self) -> Vec<EventHash>;
    pub fn get(&self, hash: &EventHash) -> Option<&Event>;
    pub fn known_hashes(&self) -> BTreeSet<EventHash>;     // for PendingBuffer drain

    /// Borrow an author's chain. Returns `None` if the author has no
    /// events yet — callers treat that as the sentinel state
    /// `(head_seq = 0, head_hash = EventHash::ZERO)` (the same shape
    /// as `AuthorChain::default()`).
    pub fn author_chain(&self, author: &AuthorPubkey) -> Option<&AuthorChain>;

    /// Build the `author_seq_vec` used in DriftAnchor + HeadsSummary diff.
    /// Iterates `by_author` in pubkey-byte-lex order; emits `AuthorSeq { author, max_seq: chain.head_seq }`
    /// for each author with `head_seq > 0`. Returns sorted by author pubkey
    /// bytes (canonical ordering).
    pub fn author_seq_vec(&self) -> Vec<AuthorSeq>;

    /// Build the `Vec<AuthorHead>` used in HeadsSummary. Same ordering as
    /// `author_seq_vec` but with the head event hash attached.
    pub fn author_heads(&self) -> Vec<AuthorHead>;

    /// Topo-sort a SUBSET of events selected by `filter`. Used for
    /// anchor-bounded drift replay (§8.4 step 3) where only events
    /// within the anchor's `author_seq_vec` participate.
    ///
    /// **Algorithm**: builds a fresh local `sub_indegree: BTreeMap<EventHash, usize>`
    /// counting only parents WITHIN the subset (events outside the
    /// subset are treated as absent — they do NOT contribute to the
    /// indegree of subset members). Runs Kahn's algorithm on that
    /// local indegree, never touching the full-DAG `self.indegree`.
    /// This handles the case where a subset-included event has deps
    /// pointing outside the subset (which would deadlock if we used
    /// the full-DAG indegree).
    ///
    /// **Child-decrement step (normative)**: when popping a node from
    /// the ready set and decrementing its children, iterate
    /// `self.parents_to_children[&next]` and decrement ONLY those
    /// children present in `sub_indegree` (i.e. `sub_indegree.get_mut(child)`
    /// returns `Some`); silently skip children not in `sub_indegree`
    /// (they are outside the subset). A naive copy of `topo_sort`'s
    /// `.expect("child has indegree entry")` would panic on non-subset
    /// children — `topo_sort_subset` MUST use `if let Some(deg) =
    /// sub_indegree.get_mut(child) { ... }` instead.
    ///
    /// **Empty subset**: if `filter` returns false for every event,
    /// `sub_indegree` is empty, the ready set is empty, the loop does
    /// not execute, and the function returns an empty `Vec`.
    ///
    /// Returns subset events in canonical topo-order (lex byte-tie-break
    /// on `EventHash`, same as `topo_sort`).
    pub fn topo_sort_subset<F: Fn(&Event) -> bool>(&self, filter: F) -> Vec<EventHash>;
}
```

Validation order (fail-fast):

1. **Signature**: compute `body_hash = event.hash_signed_body()` (BLAKE3 over canonical-encoded `SignedBody` per plan-A `crates/types/src/event.rs`); then `myrhiza_manifest::verify_signature(&event.author.as_bytes(), body_hash.as_bytes(), &event.signature)` (RFC 8032 strict via `VerifyingKey::verify_strict`). Fail → `DagError::InvalidSignature`.

   **Normative**: the Ed25519 signature covers the raw 32 bytes of `body_hash`, NOT the canonical-encoded `SignedBody` pre-image. Signing path MUST be `sk.sign(body_hash.as_bytes())`; verifying path MUST pass the same 32 bytes as `message` to `verify_strict`. The author and verifier must agree on this exact byte string or every event fails verification.
2. **Duplicate**: `event.wire_hash() ∈ by_hash` → `Ok(AlreadyKnown)`.
3. **Genesis case** (`event.seq == 1`):
   - `event.prev == EventHash::ZERO` else `DagError::InvalidGenesis("prev != ZERO")`.
   - `event.deps.is_empty()` else `DagError::InvalidGenesis("deps != ∅")`.
   - Decode `event.payload` as the **B-1 Genesis envelope** (versioned wrapper, exact bincode shape — no trailing-bytes-ignored decode):
     ```rust
     #[derive(Serialize, Deserialize)]
     struct GenesisV1 {
         seed: [u8; 32],
         founder_pubkey: AuthorPubkey,
         app_payload: Vec<u8>,    // opaque to kernel; handed to state-apply
     }
     ```
     Decode strictly via `decode_canonical::<GenesisV1>(&event.payload)` (plan-A `myrhiza_types::decode_canonical` — re-encode-and-byte-compare strict decoder). Mismatch → `DagError::InvalidGenesis("payload not a canonical GenesisV1")`.
   - `Topic::derive(app_bundle_hash, &seed, topic_name)` must equal `self.topic`. Else `DagError::InvalidTopic { expected: self.topic, derived }`.
   - `event.author == founder_pubkey` (the author of the Genesis is the founder).

   The kernel hands the FULL canonical `Event` envelope (with payload = canonical-encoded `GenesisV1`) to state-apply. State-apply decodes `GenesisV1`, reads `app_payload`, and initializes its state from those bytes. App authors writing custom genesis logic embed their app-specific payload INSIDE `app_payload` — never as a sibling field, never as trailing bytes. Future kernel majors may introduce `GenesisV2` with a tag-discriminator byte for migration; v1 commits to `GenesisV1` only.
4. **Chain integrity** (genesis and non-genesis):
   - Look up `chain = by_author.entry(event.author).or_default()`.
   - **Direct-receive equivocation check**: if `chain.seq_to_hash.get(&event.seq).is_some()` (we already have a different event from this author at this seq — the duplicate-wire-hash check at step 2 would have caught an identical event, so a hash exists at this seq AND it's not equal to `event.wire_hash()`), this is equivocation. Push `EquivocationFlag { author: event.author, seq: event.seq, local_hash: chain.seq_to_hash[event.seq], remote_hash: event.wire_hash(), peer: None }` to the caller's `equivocation_log` via the returned `DagError::Equivocation { ... }` variant. **This catches direct-receive equivocation at any seq, including the seq=1 (genesis) case** where step 3's validation has already passed but a different genesis with the same `seq=1` was already inserted. Without this check, second-genesis would silently fail step 4's `seq == head_seq + 1` (expected 2, got 1) as `InvalidChain` and never log as equivocation.
   - **Chain advance**: `event.seq == chain.head_seq + 1` else `DagError::InvalidChain { ... }`.
   - `event.prev == chain.head_hash` else `DagError::InvalidChain { ... }`.

   For genesis (seq=1): step 3 validates payload + topic; step 4's equivocation check catches a second genesis from the same author against an existing genesis; the chain-advance condition (`seq == 1 == head_seq + 1` requires `head_seq == 0`) implicitly enforces "no prior event from this author."
5. **Deps presence**: for each `d ∈ event.deps`, `d ∈ by_hash`. Any missing → `Ok(Inserted::Pending(missing_set))`. Caller stashes into `PendingBuffer`.
6. **Commit**: insert into all three maps; advance `AuthorChain`; return `NewlyApplied { topo_index, hash }`. `topo_index` is the monotonic count of events inserted to this DAG instance (used by drift-emit modulo trigger).

**First-seen-wins** falls out of step 4: any second event with same `(author, seq)` against the same `prev` arrives when `chain.head_hash` is already advanced past it → fails chain integrity → rejected.

### 4.3 Topo-sort

Used by drift-emit (to recompute digest at anchor) and by replay-from-genesis. Algorithm:

```
let mut indegree = self.indegree.clone();          // snapshot the stored index
let mut ready: BTreeSet<EventHash> = indegree.iter()
    .filter(|(_, deg)| **deg == 0)
    .map(|(h, _)| *h)
    .collect();

let mut out = Vec::with_capacity(self.by_hash.len());
while let Some(next) = ready.pop_first() {        // BTreeSet::pop_first = lex byte-order tie-break
    out.push(next);
    if let Some(children) = self.parents_to_children.get(&next) {
        for child in children {
            let deg = indegree.get_mut(child).expect("child has indegree entry");
            *deg -= 1;
            if *deg == 0 {
                ready.insert(*child);
            }
        }
    }
}
assert_eq!(out.len(), self.by_hash.len(), "DAG must be acyclic");
out
```

`indegree` and `parents_to_children` are both maintained at insert time; the topo-sort consumes them, never re-derives.

HLC ignored entirely. `BTreeSet::pop_first` is the lex byte-order
tie-break specified in §4.1. Property test (in `dag.rs`):
randomly-shuffled insertion order produces same topo-sort output
(N=100 shuffles).

### 4.4 Replay-from-genesis

```rust
impl Runtime {
    fn replay_full(&mut self) -> Result<(), RuntimeError> {
        let order = self.dag.topo_sort();
        let mut state = Vec::new();
        for hash in order {
            let event = self.dag.get(&hash).unwrap();
            let bytes = canonical_bincode().serialize(event)?;
            let r = self.handle.apply(&state, &bytes)?;
            match r.outcome {
                ApplyOutcome::Accepted => state = r.new_state,
                ApplyOutcome::Rejected(reason) => {
                    // Apply-time Reject after pre-check passed = bug
                    // (deps-monotonicity violation per §4.4) OR remote
                    // event whose pre-check was on different prior state.
                    // Spec §4.4: "cross-peer rejection from differing
                    // prior-states is normal eventual consistency".
                    // Drop the event from materialization; it remains
                    // in DAG for future sync but does not commit.
                    self.dropped_at_apply.insert(hash, reason);
                }
            }
        }
        self.state = state;
        Ok(())
    }
}
```

**Plan-B-1 simplification**: full replay on every insert. The handoff
explicitly defers snapshotting to v2 per §4.2. Acceptance tests run
at ~10-event scale; full-replay cost is microseconds.

## 5. PendingBuffer

```rust
pub struct PendingBuffer {
    by_hash: BTreeMap<EventHash, PendingEntry>,
    by_author_count: BTreeMap<AuthorPubkey, usize>,
    by_insert_time: BTreeMap<(Instant, EventHash), ()>,  // compound key — Instant
                                                          // alone collides on
                                                          // same-microsecond inserts
                                                          // (common in tests)
    max_total: usize,
    max_per_author: usize,    // = max_total / 50 per §4.2
    ttl: Duration,            // = 1h per §4.2
}

struct PendingEntry {
    event: Event,
    missing_deps: BTreeSet<EventHash>,
    inserted_at: Instant,
}
```

**Insert**: if `by_author_count[event.author] >= max_per_author` OR
`by_hash.len() >= max_total`, evict oldest by `by_insert_time` first.
TTL eviction runs lazily on every insert (`drain_filter` on
`by_insert_time` for entries past `ttl`).

**Drain**: `pub fn newly_satisfied(&mut self, known: &BTreeSet<EventHash>) -> Vec<Event>`. Iterates `by_hash`; for each entry whose `missing_deps` is a subset of `known` (i.e., all deps now present), removes the entry and returns the event. Caller (Runtime) re-inserts into DAG.

**Convergence preservation**: §4.8 — eviction is local; peers converge via subsequent HeadsSummary backfill.

**Configurable via `PendingCfg { max_total, max_per_author, ttl }`** with spec defaults. Tests use small caps to exercise eviction paths.

## 6. Network trait + MemNetwork

### 6.1 Trait

```rust
#[async_trait::async_trait]
pub trait Network: Send + Sync + 'static {
    type Subscription: Subscription + Send + 'static;
    async fn subscribe(&self, topic: Topic) -> Result<Self::Subscription, NetError>;
    async fn publish(&self, topic: Topic, msg: GossipMessage) -> Result<(), NetError>;
    async fn unsubscribe(&self, topic: Topic) -> Result<(), NetError>;
}

#[async_trait::async_trait]
pub trait Subscription: Send {
    /// Receive the next message, lag signal, or end-of-stream.
    ///
    /// Returns `Ok(Some(msg))` for a delivered message,
    /// `Err(SubError::Lagged(n))` when the underlying transport dropped
    /// `n` messages (broadcast channel overflow, packet loss buffer
    /// exhaustion, etc.) — the Runtime treats this as a signal to
    /// publish a HeadsSummary and recover via backfill, and continues
    /// calling `recv` afterward,
    /// `Ok(None)` when the subscription is closed.
    async fn recv(&mut self) -> Result<Option<GossipMessage>, SubError>;
}

pub enum NetError {
    SubscribeClosed,
    PublishFailed(String),
}

pub enum SubError {
    Lagged(u64),                      // dropped message count; not fatal
}
```

**Why `async_trait` macro** vs native async-fn-in-trait: native async-fn-in-trait (stable in 1.75) requires `Send` future bounds that the compiler currently can't express ergonomically across `dyn` (tracked at rust-lang/rust#100013 and the `async-fn-in-trait` lint family). `async_trait` adds a `Box<Future>` allocation per call; B-4 may revisit once `async fn in trait` `Send`-bound ergonomics stabilize. Decision documented in `crates/network/src/lib.rs` head comment.

**Sender identity not in trait surface**: messages carry their own `signed_by_peer` or `author` field where needed. B-4 iroh impl wraps QUIC connection identity into `GossipMessage` reception via a separate `from_peer` channel if peer-identity validation is needed at transport layer.

### 6.2 GossipMessage

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GossipMessage {
    Event(Event),
    HeadsSummary(HeadsSummary),
    HeadsRequest(HeadsRequest),
    Drift(DriftMessage),
}
```

Canonical-bincode encoding. Variant tags are u32 fixint big-endian per the v1 bincode options chain (determinism.md §5.4). This is **wire-frozen**: a snapshot test in `crates/types/tests/wire_freeze.rs` asserts the canonical encoding of each variant against a checked-in byte string.

### 6.3 MemBus + MemNetwork

```rust
pub struct MemBus {
    topics: Mutex<BTreeMap<Topic, broadcast::Sender<GossipMessage>>>,
    capacity_per_topic: usize,   // tokio broadcast channel capacity
}

impl MemBus {
    pub fn new(capacity: usize) -> Arc<Self>;
    pub fn inject_lag(&self, topic: Topic);  // cfg(test): force lag on next recv
}

pub struct MemNetwork {
    bus: Arc<MemBus>,
}

#[async_trait::async_trait]
impl Network for MemNetwork {
    type Subscription = MemSubscription;
    async fn subscribe(&self, topic: Topic) -> Result<MemSubscription, NetError> {
        let mut topics = self.bus.topics.lock().unwrap();
        let sender = topics.entry(topic)
            .or_insert_with(|| broadcast::channel(self.bus.capacity_per_topic).0)
            .clone();
        Ok(MemSubscription { rx: sender.subscribe(), topic })
    }
    async fn publish(&self, topic: Topic, msg: GossipMessage) -> Result<(), NetError> {
        let sender = self.bus.topics.lock().unwrap()
            .entry(topic).or_insert_with(|| broadcast::channel(self.bus.capacity_per_topic).0)
            .clone();
        sender.send(msg).map_err(|e| NetError::PublishFailed(format!("{e}")))?;
        Ok(())
    }
    async fn unsubscribe(&self, _topic: Topic) -> Result<(), NetError> {
        // Subscription's Drop releases the broadcast receiver.
        Ok(())
    }
}
```

**Lagged receiver**: tokio broadcast returns `RecvError::Lagged(n)`
when the receiver fell behind capacity. `MemSubscription::recv` maps
this to `Err(SubError::Lagged(n))` (per §6.1 `Subscription` trait
signature). The Runtime's run loop (§11.2) reacts by publishing a
HeadsSummary, recovering missing events via backfill. This models
real-world packet loss + bounded inbox.

## 7. HeadsSummary sync protocol

### 7.1 Types

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadsSummary {
    pub authors: Vec<AuthorHead>,            // sorted by author pubkey bytes
    pub kernel_fuel_table_version: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthorHead {
    pub author: AuthorPubkey,
    pub seq: u64,
    pub hash: EventHash,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadsRequest {
    pub requests: Vec<EventRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventRequest {
    pub author: AuthorPubkey,
    pub from_seq: u64,                       // inclusive
    pub to_seq: u64,                         // inclusive
}
```

### 7.2 Wire flow

The Runtime publishes its own HeadsSummary:
1. **On startup**, after subscribing.
2. **On RecvError::Lagged** (catch-up after broadcast lag).
3. **On `tick_interval`** (default 5s in production; 100ms in tests for fast convergence).

The Runtime, on receiving each message:

- **`Event(e)`**: validate signature → `dag.insert(e)`. Outcomes:
  - `Ok(Pending(missing))`: stash in `PendingBuffer` + publish a `HeadsRequest` for the missing events' authors.
  - `Ok(AlreadyKnown)`: no-op.
  - `Err(DagError::Equivocation { author, seq, local_hash, remote_hash })`: push `EquivocationFlag { author, seq, local_hash, remote_hash, peer: Some(sender_if_known) }` to `Runtime::equivocation_log`. Do NOT halt the Runtime — equivocation is byzantine input, not a runtime error. Continue.
  - `Err(other)`: log + drop (signature fail, invalid genesis, etc. — peer is misbehaving but no convergence impact).
  - `Ok(NewlyApplied)`:
    1. Drain `PendingBuffer.newly_satisfied(&self.dag.known_hashes())` in a loop: insert each returned event into DAG (which may further unblock subsequent stashed events), repeating until `newly_satisfied` returns empty. Equivocation errors during drain are logged-and-continued same as direct receive.
    2. Call `replay_full` ONCE after the drain loop settles — do NOT replay after each individual `dag.insert` in the drain.
    3. Call `drain_drift_stash()` (per §8.4) to process any stashed drift-messages whose anchors are now covered by the updated `author_seq_vec`. Independent of emit cadence.
    4. Check the highest `topo_index` produced by the batch against the drift trigger; emit at most one drift-message per batch even if multiple anchors were crossed (skipping intermediate anchors is acceptable — the rate cap §8.3 would suppress them anyway).
- **`HeadsSummary(remote)`**: compute diff against local `dag.author_heads()`. For each `AuthorHead { author, seq: remote_seq, hash: remote_hash }` in `remote`, compare to `(local_seq, local_hash)`:
  - Local has no entry for `author`, or `local_seq < remote_seq` → request the missing range via `EventRequest(author, local_seq+1, remote_seq)` (with `local_seq = 0` when absent).
  - `local_seq > remote_seq` → **first** check `chain.seq_to_hash[remote_seq] == remote_hash`; if mismatch, flag equivocation (`Runtime::equivocation_log` entry, `peer = Some(sender)`) and DO NOT push events for this author — the chains have diverged at `remote_seq` and pushing past that point would help propagate one branch over the other. If the hash check passes, publish events `(remote_seq+1 ..= local_seq)` for that author as individual `Event` messages.
  - `local_seq == remote_seq && local_hash != remote_hash` → equivocation flag (logged + recorded in `Runtime::equivocation_log`; no events requested).
  - `local_seq == remote_seq && local_hash == remote_hash` → in sync; no action.

  For each `author` in local but not in `remote`: publish all events for that author as `Event` messages. Aggregate backward-direction asks into a single `HeadsRequest`.

  **Equivocation detection invariant**: the ahead-by-≥1 hash check above ensures that an equivocating author is flagged regardless of how far ahead one branch has progressed. §4.4.1's "same seq, different hash" requirement is satisfied at any matching seq position, not just at the head.
- **`HeadsRequest(req)`**: for each `EventRequest { author, from_seq, to_seq }`, look up `chain.seq_to_hash[from_seq..=to_seq]`, fetch each event from `by_hash`, publish as `Event` messages. Bounds-check (anti-amplification): if `to_seq < from_seq` (malformed) OR `to_seq.saturating_sub(from_seq) > 255` (exceeds 256-event page), log + drop the request. The 255 bound corresponds to a 256-event maximum response: `from_seq + 255` inclusive through `from_seq` inclusive = 256 events. The requester is responsible for paginating.

  **Requester pagination (normative)**: when constructing a `HeadsRequest` from a `HeadsSummary` diff, the requester MUST split any `(author, from_seq, to_seq)` range with `to_seq - from_seq > 255` into consecutive pages of at most 256 events each: `(from_seq, from_seq+255), (from_seq+256, from_seq+511), ...`. Each page is sent as a separate `HeadsRequest` (one per `Runtime::publish_heads_summary` tick, or back-to-back if the requester batches). The responder accepts each page. Late-joining peers thus catch up in chunks of 256 events.

  **Bound rationale**: the responder gate `to_seq - from_seq > 255` and the requester pagination algorithm both target a 256-event maximum per page. The bounds are aligned exactly — neither side leaves a 257-event gap.
- **`Drift(d)`**: §8 drift logic below.

**Kernel-version skew**: HeadsSummary carries `kernel_fuel_table_version`. Mismatch → log "kernel out of date" warning to `Runtime::peer_warnings`; do not refuse sync. The version mismatch is *informative*; the topic-id-includes-kernel-major rule (per `browser-native.md §14.2`) is what structurally separates incompatible kernels.

## 8. Drift detection

### 8.1 Types

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriftAnchor {
    pub event_hash: EventHash,
    pub author_seq_vec: Vec<AuthorSeq>,     // sorted by author pubkey
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuthorSeq {
    pub author: AuthorPubkey,
    pub max_seq: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriftMessage {
    pub anchor: DriftAnchor,
    pub digest: [u8; 32],                   // BLAKE3 of state-digest() output;
                                            // fixed-length array (tighter than
                                            // master-spec WIT's `list<u8>`)
                                            // since BLAKE3 output is invariant
                                            // 32 bytes — the type encodes the
                                            // constraint and avoids a length
                                            // prefix on the wire
    pub digest_format: String,              // "bincode-1.3" at v1
    pub signed_by_peer: PeerPubkey,
    #[serde(with = "myrhiza_types::serde_signature_64")]
    pub signature: [u8; 64],
}

/// The exact byte target that `DriftMessage::signature` covers.
///
/// **Normative wire layout**: peer signing =
/// `peer_key.sign(canonical_bincode().serialize(&DriftSignedPayload{...}))`.
/// Verifying = same. Field order is pinned by the struct's declaration order
/// AND deliberately matches the first three fields of `DriftMessage`
/// (`anchor`, `digest`, `digest_format`) so a non-Rust reimplementation
/// can construct the signing bytes by simply serializing `DriftMessage`'s
/// first three fields in declaration order. Canonical bincode encodes
/// fields sequentially with no field-name metadata, so emitter and
/// verifier MUST produce identical bytes given the same field order.
/// The `signed_by_peer` and `signature` fields of `DriftMessage` are
/// EXCLUDED from the signed payload (the signature can't cover itself;
/// the public-key identity is supplied separately for verification).
#[derive(Serialize, Deserialize)]
pub struct DriftSignedPayload {
    pub anchor: DriftAnchor,
    pub digest: [u8; 32],
    pub digest_format: String,
}
```

### 8.2 Emit trigger

After each `dag.insert(...)` returning `NewlyApplied { topo_index, hash }`:
- If `topo_index % drift_interval == 0` where `drift_interval` is read from the loaded manifest's `[determinism.drift-detection]` `interval-events` field (plan-A `crates/manifest/src/schema.rs::DriftDetectionSection`). The kernel's `InstallFlow::load` extracts this value and injects it into `RuntimeCfg.drift_interval`. Test code overrides via `RuntimeCfg` directly (e.g. `drift_interval = 1` to exercise emit on every event). The manifest field is signed; honoring it is non-optional under spec §10.2's signing contract.
  - Compute `state_digest_bytes = self.handle.state_digest(&self.state)?`
  - `digest = blake3::hash(&state_digest_bytes).into()`
  - Build anchor: `event_hash = hash`, `author_seq_vec = dag.author_seq_vec()`.
  - Cache `(author_seq_vec → digest)` in `Runtime::own_digest_cache: BTreeMap<Vec<AuthorSeq>, [u8; 32]>` (keyed on `author_seq_vec` only; `event_hash` is metadata only — see §8.4).
  - Sign: build `DriftSignedPayload { anchor: anchor.clone(), digest, digest_format: "bincode-1.3".into() }` (field order matches §8.1 declaration: anchor, digest, digest_format), serialize via `canonical_bincode().serialize(&payload)`, then `signature = peer_key.sign(&serialized_bytes)`.
  - Publish `GossipMessage::Drift(DriftMessage { anchor, digest, digest_format: "bincode-1.3".into(), signed_by_peer: peer_key.public, signature })`.

### 8.3 Rate cap

```rust
pub struct DriftRateLimit {
    min_interval: Duration,        // 1 minute production; configurable in tests
    daily_cap: u32,                // 1024 per day
    last_emit: Option<Instant>,
    today_count: u32,
    today_started: Instant,
}
```

Both gates applied at emit. Over cap → silently drop emission. Logged
to `Runtime::peer_warnings` for visibility. Production caps per
§4.7; test config overrides both to zero to exercise the emit path
deterministically.

### 8.4 Receive logic

On `GossipMessage::Drift(d)`:

0. **Loopback filter (normative)**: if `d.signed_by_peer == self.peer_key.public`, this is the runtime's own emit returning via the broadcast channel. Drop silently — do NOT verify, do NOT compare, do NOT log. tokio broadcast (and any iroh-gossip equivalent in B-4) delivers messages to all subscribers including the publisher; without this filter, every emitted drift-message round-trips through verify + cache lookup + digest compare. Functionally a no-op (own digest matches own digest, silent success per step 4), but wasted CPU. Loopback filter is normative for predictable acceptance-test counting (e.g. tests that assert `peer_warnings` or `drift_log` entry counts).
1. Verify `d.signature` over `canonical_bincode().serialize(&DriftSignedPayload { anchor: d.anchor.clone(), digest: d.digest, digest_format: d.digest_format.clone() })` against `d.signed_by_peer` (using `myrhiza_manifest::verify_signature`). Field order matches §8.1 declaration (anchor, digest, digest_format) — MUST match the emit-side order in §8.2. Fail → drop.
2. Match against own state. **Anchor identity is `d.anchor.author_seq_vec` only — `event_hash` is informative metadata and MUST NOT be used for anchor equality**. Two peers reaching the same `(author, max_seq)` vector are at the same materialized state regardless of which event triggered the emit (different insertion orderings can pin different `event_hash` values for the same author-seq tuple). Match:
   - For each `AuthorSeq { author, max_seq }` in `d.anchor.author_seq_vec`:
     - Local `chain.head_seq < max_seq` → anchor not yet materialized locally → stash and exit.
     - Local `chain.seq_to_hash[max_seq]` exists and differs from any previously-observed hash at `(author, max_seq)` → equivocation context → log "branch divergence" and exit; do NOT compare digests.
   - All `(author, max_seq)` present locally → look up own digest cache by `author_seq_vec` key.
3. Local cache: `Runtime::own_digest_cache: BTreeMap<Vec<AuthorSeq>, [u8; 32]>` keyed on `author_seq_vec` (not full `DriftAnchor`). Look up using `d.anchor.author_seq_vec`. Cache miss → anchor was not an emit position locally; compute digest now by anchor-bounded replay:
   - **Build a lookup map** from the anchor's `Vec<AuthorSeq>`: `let bound: BTreeMap<AuthorPubkey, u64> = d.anchor.author_seq_vec.iter().map(|a| (a.author, a.max_seq)).collect();` — converts the canonical `Vec` into an O(log n) lookup for the filter.
   - **Replay subset**: call `dag.topo_sort_subset(|e| bound.get(&e.author).map_or(false, |max| e.seq <= *max))`. **Authors NOT listed in `author_seq_vec` are EXCLUDED entirely — `bound.get(&e.author) == None` → filter returns false → event omitted.** Events from listed authors with `seq > max_seq` are also excluded. `topo_sort_subset` handles the indegree correctly (per §4.2 — events with deps pointing outside the subset have those deps treated as absent, so they still reach indegree 0 within the subset).
   - **Materialize**: initialize `let mut state: Vec<u8> = Vec::new();` (identical to `replay_full` in §4.4 — the subset always includes the genesis event, which initializes state from the genesis payload). For each event hash in subset-topo-order: `state = handle.apply(&state, canonical_bincode().serialize(event)?)?.new_state`. After the loop, compute `state_digest_bytes = handle.state_digest(&state)?` and `digest = blake3::hash(&state_digest_bytes).into()`.
   - **Why excluded**: the emitter computed its digest from a state materialized at the moment its own `author_seq_vec` was the head set. Including events outside that frontier produces a digest the emitter never saw.
   - **Performance note** (carry-over to B-4): the cache-miss replay path is a blocking O(N) wasmtime call sequence inside an async task. For B-1's bounded test scale this is fine, but B-4 should evaluate offloading to `spawn_blocking` for production conditions where anchor-bounded replay may take seconds. Same pattern as §11.2's biased-select starvation note.
4. Compare cached/computed digest against `d.digest`.
   - Match → log success (no surface; success is silent).
   - Mismatch → push `DriftDetected { peer: d.signed_by_peer, anchor: d.anchor, local_digest: own, remote_digest: d.digest }` to `Runtime::drift_log`. Acceptance tests inspect this.

Pending stash for not-yet-materialized anchors: `Runtime::incoming_drift_pending: BTreeMap<Vec<AuthorSeq>, Vec<DriftMessage>>` capped at 256; oldest evicted.

**Drain trigger** (normative): after every `replay_full` completes (i.e., after every event-batch insertion advances `self.state`), the runtime invokes `drain_drift_stash()`. This function builds the current `current_seq_map: BTreeMap<AuthorPubkey, u64>` from `dag.author_seq_vec()`, then iterates `incoming_drift_pending`. For each stashed anchor whose `author_seq_vec` is **covered** by current state — defined as: for every `AuthorSeq { author, max_seq }` in the stashed anchor, `current_seq_map.get(&author).copied().unwrap_or(0) >= max_seq` — remove from stash and re-run §8.4 step 2 onward on each stashed `DriftMessage`.

**Implementation idiom (normative)**: Rust's borrow checker forbids iterating a `BTreeMap` while removing entries. Collect covered keys into a `Vec` first, then iterate the `Vec` to remove + process:

```rust
let covered_keys: Vec<Vec<AuthorSeq>> = self.incoming_drift_pending.keys()
    .filter(|asv| asv.iter().all(|a| {
        current_seq_map.get(&a.author).copied().unwrap_or(0) >= a.max_seq
    }))
    .cloned()
    .collect();
for key in covered_keys {
    if let Some(msgs) = self.incoming_drift_pending.remove(&key) {
        for m in msgs { self.process_drift_message(m).await; }
    }
}
```

`process_drift_message` is §8.4 steps 1-4 (or 2-4 if signature already verified at stash time — the spec leaves this an implementation choice; verifying once at stash time is the default to avoid recomputing the same Ed25519 verify per drain).

**Why decoupled from emit**: own emit only fires at `topo_index % drift_interval == 0`. Coupling stash drain to emit means anchors materialized between emits sit unprocessed for up to `drift_interval - 1` events. Production configs with `drift_interval = 1024` would delay stashed drift comparisons by ~1024 events. Decoupled drain runs at every state advancement, ensuring timely comparison regardless of emit cadence.

## 9. Topic-ID + genesis

### 9.1 Topic::derive

```rust
impl Topic {
    /// Derive a topic ID from the bundle hash + seed + a pre-normalized name.
    /// Caller MUST pass an NFC-normalized name. Use `Topic::derive_normalized`
    /// if the name comes from an unnormalized source.
    pub fn derive(app_bundle_hash: &BundleHash, seed: &[u8; 32], name: &str) -> Topic {
        let mut h = blake3::Hasher::new();
        h.update(b"myrhiza/topic/v1");
        h.update(app_bundle_hash.as_bytes());
        h.update(seed);
        h.update(name.as_bytes());
        Topic::from_bytes(h.finalize().into())
    }
}
```

**NFC normalization** lives in the layer above `crates/types` to keep
the types crate's dependency footprint minimal (plan-A's
`crates/manifest` already depends on `unicode-normalization`; the types
crate does not). A thin `myrhiza_manifest::derive_topic_normalized(...)`
helper applies `unicode_normalization::UnicodeNormalization::nfc` to
the name and forwards to `Topic::derive`. The kernel + test-utils call
this helper at the boundary; raw `Topic::derive` is reserved for paths
where the caller has already normalized (or where NFC is guaranteed
by upstream invariants — e.g. the `Genesis::topic_name` field is
NFC-canonicalized at install time).

**Why two functions**: `Topic::derive` stays in `crates/types`
(zero new deps; usable from anywhere). `derive_topic_normalized` is
in `crates/manifest` (already has the dep). Callers in the kernel use
the manifest-side helper.

### 9.2 Genesis payload contract for B-1

Test-utils `EventBuilder::genesis` produces `event.payload` as the
canonical-bincode encoding of `GenesisV1` (the struct defined in §4.2
step 3). Field names are normative: `seed`, `founder_pubkey`,
`app_payload`. No field is omitted; no bytes exist outside the
`GenesisV1` boundary — `decode_canonical::<GenesisV1>` (strict
re-encode-and-byte-compare) enforces exact round-trip.

Apps with app-specific initialization data embed those bytes inside
`app_payload: Vec<u8>`. There is no "prefix" convention; appending
bytes after `GenesisV1` fails the strict decode and the event is
rejected at insert.

The plan-A counter fixture's wire format remains big-endian i64 for
per-event increment payloads; B-1 introduces `GenesisV1` only for the
genesis event. The counter fixture's genesis `app_payload` is the
canonical 8-byte big-endian encoding of `0_i64` (the initial counter
value).

**State-propose seed-injection from `host.random`** is the spec
§4.6-mandated path for production. B-1 defers because state-propose
component isn't wired (plan-A is state-apply-only). The test-utils
shortcut is acceptable because B-1 tests are kernel-tier, not
state-propose-tier; the seed-injection contract is enforced by the
state-propose-binding code when that lands.

## 10. Peer identity stub

```rust
pub struct PeerKeypair {
    secret: ed25519_dalek::SigningKey,
    pub public: PeerPubkey,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PeerPubkey(#[serde(with = "crate::hash::serde_bytes_32_pub")] [u8; 32]);

impl PeerKeypair {
    pub fn generate<R: rand_core::CryptoRng + rand_core::RngCore>(rng: &mut R) -> Self;
    pub fn from_secret_bytes(b: [u8; 32]) -> Self;     // test-deterministic
    pub fn sign(&self, message: &[u8]) -> [u8; 64];
}
```

In-memory only. `Runtime::start` accepts a `PeerKeypair` by value. B-2
adds a `PeerIdentityStore` trait that produces these; B-1's tests
construct via `from_secret_bytes`.

**`PeerPubkey` vs `AuthorPubkey`**: distinct nominal types. Author
signs events. Peer signs drift-messages. Two scopes per spec §4.7.
Same underlying primitive (Ed25519 32-byte pubkey) but the type
boundary prevents accidental cross-use.

## 11. Kernel Runtime

### 11.1 Surface

```rust
pub struct Runtime {
    /* internal */
}

pub struct RuntimeCfg {
    pub drift_interval: u64,             // events between drift emits
    pub drift_min_interval: Duration,    // rate cap
    pub drift_daily_cap: u32,
    pub heads_summary_tick: Duration,
    pub pending_cfg: PendingCfg,
    pub broadcast_capacity: usize,       // for MemBus
}

pub struct RuntimeHandle {
    pub author_tx: mpsc::Sender<AuthorCommand>,
    pub drift_log: Arc<Mutex<Vec<DriftDetected>>>,
    pub equivocation_log: Arc<Mutex<Vec<EquivocationFlag>>>,
    pub peer_warnings: Arc<Mutex<Vec<PeerWarning>>>,
    pub digest_watch: watch::Receiver<Vec<u8>>,
    pub halt_watch: watch::Receiver<Option<RuntimeError>>,
}

pub enum AuthorCommand {
    Author { payload: Vec<u8>, deps: BTreeSet<EventHash>,
             reply: oneshot::Sender<Result<EventHash, RuntimeError>> },
    Shutdown,
}

impl Runtime {
    pub async fn start(
        network: Arc<dyn Network>,
        topic: Topic,
        app_bundle_hash: BundleHash,
        handle: StateApplyHandle,
        peer_key: PeerKeypair,
        author_key: Option<AuthorKeypair>,    // None = read-only peer
        cfg: RuntimeCfg,
    ) -> Result<RuntimeHandle, RuntimeError>;
}
```

`Runtime::start` spawns the loop task and returns `RuntimeHandle`.
Single-task ownership of `Runtime` internals; no `Mutex` on `EventDag`.
External observation via `Arc<Mutex<Vec<_>>>` log handles + `watch`
channels.

**`StateApplyHandle` purity guarantee (normative, load-bearing)**:
`StateApplyHandle::apply(prior_state, event_bytes) -> ApplyResult` and
`StateApplyHandle::pre_check(...)` MUST be pure functions of their
input bytes. Each invocation MUST:
- Initialize the WASM linear memory's view of state from the
  `prior_state` argument bytes (no carrying over from prior calls).
- Reset the wasmtime `Store`'s fuel counter to the per-event budget
  (`MAX_FUEL_V1 = 10_000_000` per determinism.md §5.3) before
  executing the guest call.
- Return `new_state` as fresh bytes derived from the guest function's
  output (no in-place mutation of `prior_state`).

The wasmtime `Store` MAY be reused across calls for `Engine` /
`InstancePre` caching (cold-instantiation cost dominance per
browser-native.md §14.5), but its **per-event observable state**
(linear memory of state, fuel) MUST be reset between calls. This is
the foundational guarantee on which B-1's drift-drain design rests:
`drain_drift_stash`'s cache-miss subset replay (§8.4 step 3) drives
the handle through an arbitrary event subset, then the main loop's
next `replay_full` drives it through the full canonical sequence.
Both paths share the same `StateApplyHandle`; correctness depends on
each call being independent.

Plan-A's `crates/kernel/src/state_apply.rs::StateApplyHandle::apply`
already satisfies this contract (its signature passes `prior_state`
by reference and returns `new_state` by value); B-1 explicitly states
the guarantee as a load-bearing invariant. Any implementer adding
cross-call Store mutation in B-1 or later violates this invariant and
breaks drift comparison correctness.

### 11.2 Loop

```rust
async fn run(&mut self, sub: impl Subscription, author_rx: mpsc::Receiver<AuthorCommand>) {
    let mut ticker = tokio::time::interval(self.cfg.heads_summary_tick);
    self.publish_heads_summary().await;
    loop {
        tokio::select! {
            biased;
            cmd = author_rx.recv() => match cmd {
                Some(AuthorCommand::Author { payload, deps, reply }) => {
                    let r = self.author(payload, deps).await;
                    let _ = reply.send(r);
                }
                Some(AuthorCommand::Shutdown) => break,
                None => break,
            },
            recv_result = sub.recv() => match recv_result {
                Ok(Some(m)) => self.handle_message(m).await,
                Ok(None) => break,
                Err(SubError::Lagged(n)) => {
                    self.peer_warnings.lock().unwrap()
                        .push(PeerWarning::BroadcastLagged { dropped: n });
                    self.publish_heads_summary().await;
                    // continue loop; subsequent recv resumes from new position
                }
            },
            _ = ticker.tick() => self.publish_heads_summary().await,
        }
    }
}
```

`biased` polling: author commands have higher priority than incoming
messages, so authoring is responsive even under message flood.

**Starvation risk (documented)**: under sustained author flood, the
`sub.recv()` and `ticker.tick()` arms will be starved. For B-1's
bounded test-driven authoring this is fine. B-4 (iroh) MUST evaluate
whether fair `select` or bounded author batching (e.g. drain at most
K author commands per loop iteration) is needed for production
network conditions. The decision is logged here so B-4 doesn't ship
biased select silently into the iroh path.

### 11.3 Author path

```rust
async fn author(&mut self, payload: Vec<u8>, deps: BTreeSet<EventHash>)
    -> Result<EventHash, RuntimeError>
{
    let author_key = self.author_key.as_ref().ok_or(RuntimeError::ReadOnly)?;
    // Drain any pending incoming messages so chain head is current.
    self.drain_subscription_nonblocking().await;
    // Compute next slot.
    let chain = self.dag.author_chain(&author_key.author);
    let (seq, prev) = if chain.head_seq == 0 {
        (1, EventHash::ZERO)
    } else {
        (chain.head_seq + 1, chain.head_hash)
    };
    let hlc = self.hlc.now();
    let body = Event {
        author: author_key.author,
        seq, prev, deps, hlc, payload,
        signature: [0; 64],     // placeholder, signed below
    };
    let body_hash = body.hash_signed_body();
    let signature = author_key.sign_body_hash(body_hash);
    let event = Event { signature, ..body };
    let envelope = canonical_bincode().serialize(&event)?;
    // Pre-check.
    let pre = self.handle.pre_check(&self.state, &envelope)?;
    if let ApplyOutcome::Rejected(reason) = pre.outcome {
        return Err(RuntimeError::PreCheckRejected(reason));
    }
    // Self-insert (proves signature + chain) then replay.
    let inserted = self.dag.insert(event.clone())?;
    if matches!(inserted, Inserted::NewlyApplied { .. }) {
        self.replay_full()?;
        self.drain_drift_stash().await;   // same trigger as §7.2 step 3
        self.maybe_emit_drift(event.wire_hash()).await;
    }
    // Broadcast.
    self.network.publish(self.topic, GossipMessage::Event(event.clone())).await?;
    Ok(event.wire_hash())
}
```

### 11.4 HLC source

`Runtime::hlc` is an internal `HlcClock` initialized from `SystemTime::now()` at startup. Advanced on each author via `Hlc::increment_local`. On receive of an event, `HlcClock::merge_remote(event.hlc)` (HLC monotonic merge per Kulkarni 2014). Not used for ordering — purely materialized into derived state.

## 12. Test infrastructure

### 12.1 Test-utils additions

```rust
// crates/test-utils/src/event_builder.rs
pub struct AuthorKeypair {
    secret: ed25519_dalek::SigningKey,
    pub author: AuthorPubkey,
}
impl AuthorKeypair {
    pub fn deterministic(seed: u64) -> Self;
    pub fn sign_body_hash(&self, body_hash: EventHash) -> [u8; 64];
}

pub struct EventBuilder<'k> {
    pub author_key: &'k AuthorKeypair,
}
impl<'k> EventBuilder<'k> {
    pub fn genesis(&self, app_bundle_hash: &BundleHash, seed: [u8; 32],
                   name: &str, payload: Vec<u8>) -> Event;
    pub fn next(&self, prev: &Event, deps: BTreeSet<EventHash>,
                payload: Vec<u8>) -> Event;
}

// crates/test-utils/src/harness.rs
pub struct InProcessHarness {
    bus: Arc<MemBus>,
    bundle: BuiltBundle,
    app_bundle_hash: BundleHash,
}
impl InProcessHarness {
    pub fn new(bus_capacity: usize) -> Self;     // builds counter-state-apply bundle
    pub async fn spawn_peer(&self, peer_seed: u64, author_seed: Option<u64>,
                            topic: Topic, cfg: RuntimeCfg) -> RuntimeHandle;
}
```

`BuiltBundle` reuses plan-A's bundle builder from `crates/test-utils`.

### 12.1.1 Counter fixture update (load-bearing)

Plan-A's `tests/fixtures/counter-state-apply/` consumes the event
argument as **hand-rolled big-endian i64 payload bytes** (per
`docs/reports/2026-05-09-myrhiza-foundation-handoff.md` §"Known gaps"
#6). Plan-A's `crates/kernel/src/state_apply.rs` module doc explicitly
flags this: plan-B's event-ingestion path MUST pass the FULL canonical
`Event` envelope (per convergence.md §4) to the wasm boundary, so the
counter fixture's `apply` export must be rewritten in B-1 to:

1. Decode the incoming event slice via canonical bincode 1.3.x as `Event`.
2. Detect Genesis: if `event.seq == 1 && event.prev == EventHash::ZERO`,
   decode `event.payload` via `decode_canonical::<GenesisV1>` (the
   strict decoder defined in §4.2 step 3); the fixture uses `app_payload`
   as the initial state bytes (canonical 8-byte big-endian `0_i64` for
   the counter). Return `Accept` with `new_state = app_payload`.
3. For non-Genesis: decode payload as big-endian i64 increment; add
   to current state's i64; return updated state.

Same float-ban discipline applies (no `serde_core::de::Visitor::visit_f64`
in dependency closure; the existing hand-rolled big-endian i64
encoding survives the lint and is reused for the i64 payload portion).

The `over-importer`, `pre-check-rejector`, `infinite-loop`, and
`float-banned` fixtures continue to consume hand-rolled bytes — they
do not produce or consume canonical Event envelopes (they are tested
through plan-A's apply path which passes hand-rolled bytes directly).
B-1 adds the canonical-envelope discipline only to the counter
fixture, since that is the one wired into convergence tests.

### 12.2 Acceptance tests

Located at `crates/kernel/tests/convergence.rs`. Each test annotated `/// Covers: ...` so the spec-coverage matrix picks them up.

| Test | Covers | Behavior |
|---|---|---|
| `single_originator_single_receiver_converges` | mvp §15.1 #1+#2, convergence §4 | A authors 5 events; B subscribes and converges. Both digests equal. |
| `concurrent_multi_author_converges` ⭐ | convergence §4.1, mvp §15.1 #2 | A + B both author concurrently with cross-deps; both peers' topo-sort produces equal sequence + digest. |
| `late_joiner_backfills_via_heads_summary` | convergence §4.2 | A authors 10 events alone; B subscribes; B's startup HeadsSummary triggers A's backfill; B converges. |
| `coexistence_two_topics_no_event_crossing` | mvp §15.1 #4, convergence §4.6 | Two Runtimes per peer on distinct topic_ids; cross-topic delivery does not happen. |
| `drift_detected_when_state_apply_corrupted` | convergence §4.7 | Peer B wraps StateApplyHandle in `CorruptingDecorator` that flips one byte at apply N. Both peers' drift_log records mismatch. |
| `equivocating_author_chain_first_seen_wins` | convergence §4.4.1 | Two events with same `(author, seq)` (both same-`prev` and different-`prev` variants); first inserted wins; second returns `DagError::Equivocation { author, seq, local_hash, remote_hash }`. Runtime's `equivocation_log` gains exactly one entry. |
| `pending_buffer_evicts_oldest_under_capacity` | convergence §4.2, §4.8 | PendingBuffer cap = 3; flood 10 out-of-order events; oldest 7 evicted; convergence still reached via HeadsSummary backfill. |
| `lagged_broadcast_recovers_via_heads_summary` | convergence §4.2 | MemBus capacity = 2; A floods 10 events while B is slow; B's `RecvError::Lagged` triggers HeadsSummary; B converges. |

### 12.3 Spec-coverage gate

The plan-A spec-coverage CI gate (`just spec-coverage-check`) ingests `/// Covers:` annotations and rejects unknown spec refs. B-1 adds new refs against `convergence.md §4.7` and others; `scripts/spec-coverage.sh` recomputes `valid_refs`.

## 13. Error model

```rust
// crates/kernel/src/dag.rs
pub enum DagError {
    InvalidSignature,
    InvalidGenesis(&'static str),
    InvalidTopic { expected: Topic, derived: Topic },
    InvalidChain { author: AuthorPubkey, expected_seq: u64, got_seq: u64,
                   expected_prev: EventHash, got_prev: EventHash },
    Equivocation { author: AuthorPubkey, seq: u64,
                   local_hash: EventHash, remote_hash: EventHash },
}

// crates/network/src/lib.rs
pub enum NetError {
    SubscribeClosed,
    PublishFailed(String),
}

// crates/kernel/src/runtime.rs
pub enum RuntimeError {
    Network(#[from] NetError),
    Apply(#[from] ApplyError),
    Dag(#[from] DagError),
    PreCheckRejected(String),
    Canonical(#[from] EncodingError),
    PendingFull,
    ReadOnly,                          // author called on peer without author_key
}
```

Each `Runtime` halts on first irrecoverable error; halt surfaced via `halt_watch` so test code can assert intentional halt vs unexpected halt. Recoverable errors (signature fail on inbound message, lag) logged + ignored.

**Observation log types** (not errors; structured records surfaced via the `Arc<Mutex<Vec<_>>>` handles on `RuntimeHandle`):

```rust
pub struct DriftDetected {
    pub peer: PeerPubkey,
    pub anchor: DriftAnchor,
    pub local_digest: [u8; 32],
    pub remote_digest: [u8; 32],
}

pub struct EquivocationFlag {
    pub author: AuthorPubkey,
    pub seq: u64,
    pub local_hash: EventHash,
    pub remote_hash: EventHash,    // hash seen in remote HeadsSummary at same seq
    pub peer: Option<PeerPubkey>,  // None if observed during own emit path
}

pub enum PeerWarning {
    KernelFuelTableMismatch { peer: Option<PeerPubkey>, remote_version: u32, local_version: u32 },
    DriftRateLimited { kind: RateLimitKind },
    BroadcastLagged { dropped: u64 },
}
```

## 14. Edge cases handled

1. **PendingBuffer overflow**: oldest evicted; convergence preserved via HeadsSummary backfill (§4.8).
2. **Concurrent deps topo-sort**: BTreeSet ready set; lex byte-order; property test with 100 shuffles.
3. **New author on HeadsSummary diff**: falls out of the algorithm — request `(1, remote_head_seq)`.
4. **Receive during own author**: drain subscription before computing `prev`.
5. **Drift-message before anchor materialized**: stashed in `incoming_drift_pending` (256-entry cap); processed on reaching anchor.
6. **MemBus subscriber lag**: `RecvError::Lagged` → publish HeadsSummary to recover.
7. **Signature verification fail on receive**: drop + log; do not insert.
8. **Apply-time Reject after pre-check pass**: per §4.4 cross-peer eventual consistency. Event recorded in `dropped_at_apply` map; not removed from DAG; replay on future state-change may re-accept. **Including on the originator**: after `Runtime::author` signs and self-inserts, the subsequent `replay_full` may produce a `state` that differs from `pre_check`'s `candidate_state` because previously-dropped events (in `dropped_at_apply`) may now be re-accepted under the new topo ordering. The authored event itself may even land in `dropped_at_apply` post-replay. This is expected behavior, not a bug: the pre-check guarantee is "would this event be Accepted against current `self.state`", not "this event will be Accepted in the post-broadcast canonical state." Cross-peer convergence is preserved because all peers run the same canonical topo-sort. Acceptance tests MUST NOT assume the authored event ends up in `state` — they assert `event.wire_hash() ∈ dag.by_hash` and assert digest equality across peers; they do NOT assert the event was applied locally.
9. **Topo-sort cycle**: structurally impossible (each event's `prev` strictly precedes it in seq; deps are content-hashes which by collision-resistance cannot back-reference). Asserted via `assert_eq!(out.len(), by_hash.len())`.

## 15. Explicit non-goals at B-1

- iroh transport (B-4)
- Persistent identity / bech32m (B-2)
- Module-dep recursion (B-3)
- Revocation topic check (B-5)
- Host-call fuel wiring (B-6 carry-over)
- Persistent DAG (B-7)
- State-propose component wiring
- Snapshot / checkpoint
- Equivocation auto-resolution (warrant pattern)
- Drift recovery
- Wall-clock drift backstop
- (Manifest `[determinism.drift-detection]` `interval-events` field is honored by reading it at install — see §8.2)
- iroh-blobs bundle fetch (B-4)

## 16. Acceptance evidence at B-1 ship

- `crates/kernel/tests/convergence.rs` — 8 tests pass
- `just ci` green; no warnings; spec-coverage matrix extended with §4.1, §4.2, §4.4.1, §4.6, §4.7, §4.8 refs
- Plan-A acceptance tests in `crates/kernel/tests/acceptance.rs` updated where the counter fixture's wire shape changes (§12.1.1): `kernel_instantiates_and_applies_increment` must be rewritten to construct a canonical `Event` envelope rather than passing hand-rolled increment-payload bytes (the over-importer, pre-check-rejector, infinite-loop, float-banned tests are unaffected because their fixtures do not consume canonical envelopes). All 6 acceptance tests remain green post-rewrite.
- Property test in `crates/kernel/src/dag.rs` exercises topo-sort determinism

## 17. Rough schedule

Per mvp §15.5 critical-path items 8 (event/DAG primitives + topo-sort + PendingBuffer) + 9 (network) + 10 (HeadsSummary + drift):

- Item 8: ~2 wk
- Item 9 (MemNet subset): ~1 wk
- Item 10: ~2 wk
- Test-utils + acceptance: ~1 wk

**Total: ~6 wk**, single engineer. Parallelizable: dag.rs ↔ network/memnet are independent until Runtime ties them together. Two engineers ≈ 3 wk.

## 18. Sources

- `docs/specs/2026-05-09-myrhiza-master-design/convergence.md` §4 (load-bearing)
- `docs/specs/2026-05-09-myrhiza-master-design/networking.md` §11
- `docs/specs/2026-05-09-myrhiza-master-design/mvp.md` §15.1, §15.4, §15.5
- `docs/specs/2026-05-09-myrhiza-master-design/determinism.md` §5.4
- `docs/specs/2026-05-09-myrhiza-master-design/browser-native.md` §14
- `docs/reports/2026-05-09-myrhiza-foundation-handoff.md`
