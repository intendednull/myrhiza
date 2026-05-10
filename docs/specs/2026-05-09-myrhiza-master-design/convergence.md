**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Convergence and state-apply


## 4. Convergence and state-apply

**Decision**: event-log replay is the convergence paradigm.

Per-author signed Merkle DAG. Each event has author, sequence number
(monotonic per-author starting at 1), `prev` (hash of this author's
previous event), `deps` (array of cross-author causal heads), opaque
payload bytes, and Ed25519 signature.

Peers gossip events via iroh-gossip. Each peer's local DAG indexes
events by hash, by author chain, and by topic membership. Materialization =
deterministic topo-sort over the DAG, replaying each event through the
app's `state-apply` component.

### 4.1 Topological ordering

Topo-sort uses Kahn's algorithm with a `BTreeSet<&EventHash>` ready
set. Concurrent events (no causal order between them) are
tie-broken by **EventHash lexicographic byte comparison**, not by
HLC timestamp. This is content-causal-plus-lex-hash ordering.

HLC timestamps are signed into events but **not used for DAG
ordering, merge, or topo-sort tie-break**. They are materialized
into derived state where useful (channel-activity timestamps,
ephemeral idle thresholds, display ordering hints).

### 4.2 Sync protocol

`HeadsSummary`-style delta exchange. The protocol is normative at
v1:

```wit
record heads-summary {
    // Compact per-author DAG-tip vector. Sorted by author pubkey
    // bytes for canonical encoding.
    authors: list<author-head>,
    // Kernel-version skew detection (§19): peer's Wasmtime fuel-table
    // major version. Mismatch surfaces "kernel out of date" warning.
    kernel-fuel-table-version: u32,
}

record author-head {
    author-pubkey: list<u8>,
    seq: u64,
    hash: list<u8>,                 // EventHash of head event
}

// Wire protocol:
// 1. Peer A sends HeadsSummary to peer B.
// 2. B compares author-by-author:
//    - For each author A_i in B's local but not in A's summary:
//      B is ahead; send events.
//    - For each author A_i in A's summary with seq > B's local:
//      A is ahead; B requests missing events via heads-request.
//    - For each author A_i with same seq but different hash:
//      Equivocation detected (§4.4.1) — flag and continue.
// 3. Repeat exchange until both sides converge on identical
//    HeadsSummary.
record heads-request {
    requested-events: list<event-request>,
}

record event-request {
    author-pubkey: list<u8>,
    from-seq: u64,                  // inclusive
    to-seq: u64,                    // inclusive
}
```

`HeadsSummary` is also used on the **revocation topic** ([distribution.md](distribution.md) §10.7) for
backfill of missed revocations on peer start. Same protocol shape;
revocation-event-shaped payloads instead of app events.

Out-of-order delivery is buffered in a `PendingBuffer` with two
eviction policies (independent): age-based (default 1 hour TTL) and
capacity-based (default 10,000 entries with per-author sub-cap of
`max_entries / 50` to thwart Sybil-shaped flooding).

**Snapshots are out of v1 scope** (per [risks.md](risks.md) §19 Project-shape v2). v1
bootstrap is full-event-log replay from genesis. A peer joining a
topic for the first time fetches all events via HeadsSummary
delta exchange and replays through state-apply. Snapshot support
ships in v2 as the `myrhiza-state-snapshot-cache` module.

### 4.3 Cross-peer convergence proof

Convergence is verified by hashing each app's exported `state-digest()`
function output. The digest is canonical bytes under a deterministic
encoding using sorted collections (`BTreeMap` / `BTreeSet`). The
kernel hashes the digest output and gossips the hash; mismatches
surface as bugs.

**Why not hash WASM linear memory**: allocator behavior, struct field
padding, `HashMap` iteration order would diverge trivially across
peers. App-canonical digest is the load-bearing piece; format is
**pinned at v1 to bincode 1.3.x** with explicit Options chain (see
[determinism.md](determinism.md) §5.4). Future kernel majors may add format opt-ins (e.g. postcard
via manifest declaration); v1 commits one format.

### 4.4 Pre-check unification

`state-apply` runs in two modes:

- **Apply mode**: peer ingests an incoming event; runs `state-apply`
  against current state; commits new state if return is `Accept`.
- **Pre-check mode (dry-run)**: originating peer runs `state-apply`
  against a hypothetical post-state before signing the event. If
  the return is anything other than `Accept`, the kernel **rejects
  the user action and does not sign**. Pre-check fails closed.

Pre-check is **mechanically the same WASM function as `state-apply`**,
called by the kernel in dry-run mode. Not a convention. The same
deterministic helper set, the same fuel posture, the same denied
non-deterministic imports apply.

This makes pre-check / apply divergence **structurally impossible
given identical prior-state inputs** — there is one code path, one
verdict-function.

**Important caveat on cross-peer rejection**: pre-check runs on the
originating peer against *its* current state. Other peers ingesting
the resulting event run apply against *their* current states, which
may differ if they have seen events the originator hadn't. The
invariant is **deps-monotonicity**: state-apply must be valid against
*any* state that contains all the event's declared `deps`, not just
the originator's specific snapshot. Apps that violate deps-monotonicity
will see convergence diverge — pre-check accepts on prior-state P_origin,
apply rejects on prior-state P_other, both contain the same `deps`,
because state-apply consulted state outside the deps closure.

**v1 enforcement**: deps-monotonicity is an app-author invariant
checked by code review and integration tests; the kernel does not
mechanically enforce it (deferred to a determinism-enforcement child
spec that may add a static-analysis tool).

**Cross-peer rejection from differing prior-states is normal eventual
consistency**, not a bug. Two peers receiving events in different orders
may temporarily reject events the other accepts; under continued event
gossip and DAG replay, both peers converge to the same canonical
state (because content-causal ordering eventually places events in
the same canonical position).

### 4.4.1 Author equivocation

A malicious author may sign two events with the same `seq` against
the same `prev`. Different peers see one or the other first; whichever
they see first becomes the author's canonical chain head. Subsequent
events under either branch are validated against `prev` — the rejected
branch's events fail.

**v1 resolution**: **first-seen-wins per peer**. Each peer's
`EventDag::insert` enforces per-author chain integrity (`seq ==
latest_seq + 1` and `prev == current_head`). The first event with
seq=N from author A becomes that peer's chain head; subsequent
seq=N events from A are rejected as invalid.

**Convergence implication**: equivocating authors can permanently
fork their own chain across peers. Peer X holds A's branch B1; peer Y
holds A's branch B2. They cannot reconcile without explicit equivocation
resolution. **v1 does not provide automatic resolution.** Equivocation
is treated as an app-level concern (apps may surface it via derived
state, e.g. "this author is flagged as equivocating; their events
are partitioned").

**Future direction (Holochain warrant pattern)**: warrants are signed
attestations — "I observed equivocation by author A at seq=N" — broadcast
on-DAG. Apps that import a warrant module convert warrants into
derived state automatically. Deferred to a warrant-and-equivocation
child spec; v1 manifest may declare opt-in to a future
`myrhiza-permission-warrants` module.

### 4.5 Future direction: scaling

Event-log replay scales linearly: every materializing peer carries
the full log for the topic. **v1 ships an author-bounded scale
constraint**: chat, kanban, wiki, poll, counter shapes with bounded
author counts (~tens to ~hundreds) fit comfortably on event-log
replay alone. Apps targeting larger scale (Twitter-shape, large public
read access) are out of v1 product scope.

**Back-of-envelope for v1 scope** (informative, not normative):
- Counter app, 10 authors × 10 increments/day × 365 days =
  ~36K events/year, ~3.6 MB at ~100 B/event. Trivial.
- Chat app, 100 authors × 100 messages/day × 365 days =
  ~3.65M events/year, ~365 MB. Per-peer storage; large but bounded.
- Wiki app with 1000 contributors × 50 edits/day = ~18M events/year,
  ~1.8 GB. Approaching storage ceiling on consumer devices; v1 is
  not the right substrate for this shape.

**Master spec acknowledges this as the named-but-deferred scaling
problem.** When real apps approach the ceiling, evolution paths
include:

- **Snapshot-as-bootstrap with log-pruning**. Eg-walker-style log
  compaction is research-grade; log truncation past a snapshot is
  well-understood for some shapes. Reduces storage but not bandwidth
  or replay CPU.
- **Cooperative pinning** via maintenance modules. Persister modules
  store full history; consumers fetch snapshots; per-peer storage
  drops dramatically. Requires participation-enforcement ([maintenance.md](maintenance.md) §12.5).
- **Read-replica through a separate channel**. Read-heavy apps
  materialize from log on dedicated peers (operator-deployed or
  social-graph-elected) and gossip materialized state directly.
- **DHT-shape sharding** layered on top of event-log canonical
  ordering. Closest existing precedent: **Holochain's DHT op
  decomposition** (`prior-art/holochain/lessons.md` Borrow §2) —
  events decompose into typed ops (StoreEntry, RegisterAddLink,
  RegisterAgentActivity, etc.); each op is sharded by neighborhood
  pubkey distance. Layering Holochain's op decomposition over
  Myrhiza's event-log gives a path from "every peer holds everything"
  to "neighborhoods hold their share" without changing the event-log
  contract.

**Decision criteria for picking the v2+ answer**: when the first
Myrhiza app hits the scaling ceiling, measure where the actual
bottleneck is (storage cost / replay time / bandwidth / participation
enforcement) and pick the answer that addresses that bottleneck.
Don't speculatively ship sharding before the bottleneck is real.

**Holochain primitives Myrhiza inherits as future-direction options**
(per `prior-art/holochain/lessons.md` Borrow):
- §1 Source-chain semantics (already aligned: per-author Merkle DAG
  IS source-chain shape)
- §2 DHT op decomposition (informs v2 sharding direction)
- [architecture.md](architecture.md) §3 Warrants (bad-author signaling — see §4.4.1 future direction)
- §4 Countersigning (multi-author atomic events — relevant for
  governance modules; deferred)
- [identity.md](identity.md) §6 Membrane proofs (capability-bound app entry — relevant for
  participation primitive; informs `myrhiza-permission-rbac`)

### 4.6 Topic identity

A **topic** is a content-addressed identifier under which gossip
events flow. Membership in a topic = the peer is gossiping events
on that topic.

**Topic ID formula**:

```
topic_id = BLAKE3(
    "myrhiza/topic/v1" |
    app_bundle_hash |
    app_instance_seed |
    topic_name
)
```

Where:
- `app_bundle_hash` is the iroh-blobs content hash of the app's
  bundle ([distribution.md](distribution.md) §10). Different versions of the same app have different
  bundle hashes; topics under different versions do not collide.
- `app_instance_seed` is a 32-byte random value chosen at app-instance
  creation time. Two installations of the same app create different
  instances (e.g. two separate counter games); their topics do not
  collide. The seed is included in the genesis event so all peers
  joining the instance reach the same topic ID.
- `topic_name` is an app-chosen UTF-8 string. Within an instance, an
  app may have multiple topics distinguished by name (e.g. "main",
  "channel-foo", "thread-1234"). Empty string is the default
  "main" topic.
- The `"myrhiza/topic/v1"` domain separator prevents cross-protocol
  collision and lets future kernel versions evolve the formula.

**Per-app namespace property** (acceptance criterion #4 in [mvp.md](mvp.md) §15.1):
since `app_bundle_hash` and `app_instance_seed` differ between any
two coexisting app instances, their topic IDs cannot collide. Events
from app instance X cannot leak into app instance Y's gossip subscription.

**Genesis event semantics**: the first event in any topic MUST be
a `Genesis` event with payload variant `Genesis(app_instance_seed,
initial_state, founder_pubkey)`, signed by the founding peer's
IdentityScope. The genesis event's `seq=1`, `prev=EventHash::ZERO`,
`deps=[]`. The kernel verifies the genesis event's `seed` matches
the topic_id formula's `app_instance_seed`; mismatch = invalid topic.

**Seed injection flow**: when a state-propose component returns
a candidate `Genesis` payload, the kernel **injects** the 32-byte
seed from `host.random` BEFORE pre-check runs. State-propose does
not supply the seed (any propose-supplied seed value is overwritten
by the kernel); pre-check sees the candidate event with seed already
set; only after pre-check passes does the kernel sign and broadcast.
This prevents apps from deliberately picking colliding seeds across
instances.

**Topic-ID in invitations**: out-of-band invitations to join an
existing app instance MUST carry the full `topic_id` (and the
`app_bundle_hash` for kernel-version verification), not just the
`app_instance_seed`. Carrying only the seed leaves invitations
vulnerable to MITM substitution (an attacker substitutes a different
genesis with a different seed → different topic_id → joiner subscribes
to MITM-controlled topic). Invitation links should be canonical
bech32m-encoded `(topic_id, app_bundle_hash, founder_pubkey)`
tuples — child spec defines the format.

**`app_instance_seed` source**: the seed MUST be 32 bytes from
`host.random` on the originating peer (not app-controlled input).
This prevents apps from deliberately causing topic collisions across
instances. Enforced by the `host.author-event` payload validation
for `Genesis` variants.

**Topic creation**:

- A topic is implicitly created when its first event is gossiped.
- Apps subscribe to topics via `host.subscribe(topic_id) -> handle`
  (interaction or behavior profile).
- Topic discovery for new joiners is out-of-band at v1 (link, QR,
  in-app share). In-band catalog gossip (a parent "registry topic"
  for an app instance listing its child topics) is a v2 child-spec
  concern.

**Topic-ID rotation**: apps that need unlinkability across rotations
(e.g. epoch-key-rotation pattern) compute new `topic_id` from a new
`app_instance_seed` (rotated via in-band events on the existing topic
before rotation). The kernel is not in this loop. Detailed protocol
deferred to relay-and-rotation child spec.

### 4.7 Cross-peer drift detection (TUTTI-shaped)

Strict state-apply purity ([architecture.md](architecture.md) §3.1) gives convergence-by-construction,
but does not protect against bugs in the state-apply implementation
itself, in the kernel's helper-set implementation, or in serialization
edge cases. **v1 ships runtime drift detection** modeled on Croquet's
TUTTI snapshot-equality voting pattern (`prior-art/croquet/lessons.md`
Borrow §"Snapshot-equality voting").

**Anchor model**: each drift-detection emission is anchored to a
specific event in the canonical topo-sort. The anchor format:

```wit
record drift-anchor {
    event-hash: list<u8>,                  // the canonical "after this event" point
    author-seq-vec: list<author-seq>,      // {author_pubkey, max_seq} for each author
                                           // up to and including the anchor
}

record drift-message {
    anchor: drift-anchor,
    digest: list<u8>,                      // BLAKE3 of state-digest() output at anchor
    digest-format: string,                 // "bincode-1.3" at v1 (per §5.4)
    signed-by-peer: peer-handle,
    signature: list<u8>,                   // peer signs the (anchor, digest) tuple
}
```

Peers compare drift messages **only** when their own materialized
state's `author-seq-vec` matches the message's anchor. If anchors
differ, the receiving peer either has not yet materialized to that
anchor (sync-lag, ignore message), has already materialized past
(catch up via HeadsSummary), or is on a different equivocation
branch (§4.4.1; flag separately, do not treat as drift).

**Drift-message signing scope**: drift-messages are signed by the
emitting peer under its peer-scoped IdentityScope (the peer's
long-term identity, NOT a per-instance scope). This anchors digest
claims to verifiable peer identities; peers cannot anonymously
publish fake drift-messages. Drift-message rate is capped at 1
message per (peer, topic) per minute at the gossip layer, with a
1024-message-per-day per-peer cap to bound DoS surface.

**Trigger**: digest emission anchored to canonical topo-sort index
modulo N (default N=1024 events; tunable per-app via manifest [distribution.md](distribution.md) §10.2).
**Wall-clock-driven backstop disabled at v1** — using time-based
emission would inject peer-local non-determinism into gossip cadence
(observable as side-channel of peer-local clock state). Future
versions may add a backstop with documented determinism implications.

**Cost honest**: `state-digest()` walks the entire app state. For a
chat-shape app at ~100MB materialized, this is ~100MB encode + hash
per emission. With N=1024 events default, cost amortizes per-1024-
events. Apps with large state may tune N higher to reduce cost; apps
with small state may tune lower. The kernel does not artificially
bound digest computation — it is the app's `state-digest()`
responsibility to be efficient.

**Equivocation interaction**: §4.4.1 first-seen-wins means peers
holding different equivocation branches WILL produce different
digests at the same anchor. The drift-detection mechanism flags this
as a "convergence drift" event — but the surfaced UI marks it
specifically as "author X equivocated; peers diverged on which branch
was first-seen" rather than "your state is buggy." Drift surfaced
without equivocation context defaults to the "implementation bug"
framing.

**v1 enforcement**: drift detection is a built-in kernel feature, not
a module. Apps cannot opt out (only tune the N interval via manifest).
Recovery from drift is a child-spec concern; v1's contract is detection
+ surfacing, not auto-recovery.

**Integration with §4.3**: digest gossip rides existing gossip topics;
no new transport surface.

### 4.8 PendingBuffer eviction is local

The PendingBuffer (§4.2) eviction policies (age-based 1h TTL,
capacity-based 10K entries with per-author sub-cap) are **local to
each peer**. Two peers with different eviction outcomes for the same
out-of-order events still converge: they both end up requesting the
missing predecessors via `HeadsSummary` exchange and materialize the
same events in the same canonical topo-sort order.

**Convergence does not depend on eviction parameters.** Operators may
tune buffer size and TTL freely without affecting cross-peer agreement.

### 4.9 CRDT use cases

Apps that want commutative-merge semantics (collaborative text,
real-time docs) embed a CRDT inside their own `state-apply`. The
kernel stays generic — it does not bake any CRDT into the event
substrate.

This is a documented usage pattern, not an architectural feature.
The CRDT's internal state must be canonically encoded (sorted
collections) for state-digest determinism. Future modules
(`myrhiza-state-crdt-{automerge,yjs,loro}`) ship the CRDT-in-state-
apply pattern as a reusable component for apps that want it.


