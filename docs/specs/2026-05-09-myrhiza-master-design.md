**Date:** 2026-05-09
**Status:** draft

# Myrhiza — master design

The runtime spec. What we are building, what its shape is, what we
have committed to, what we have explicitly deferred, and how the v1
acceptance proof works.

This document is the canonical reference for any spec, plan, report,
or implementation work that touches the runtime. Child specs refine
specific subsystems — they cite this file and conform to it. When
this file changes, child specs review for impact.

## 1. Vision and scope

Myrhiza is a **P2P app runtime**. A small kernel hosts typed,
capability-mediated, content-addressed apps. Apps are bundles of
WebAssembly components. The kernel owns identity, peer protocol,
event/DAG primitives, the component loader, and the capability
arbiter. Everything else — chat, wikis, kanban, polls, whatever
someone builds in two years — is an app.

**The novel commitment**: peers are infrastructure. Storage,
replication, sync, replay buffering, snapshot custody — these are
work performed by participants, not deployed services. As more peers
participate in an app, the app's maintenance capacity grows. No
infrastructure deploy required to scale.

**The security commitment**: capabilities are the only way components
reach beyond their own memory. Apps cannot touch private keys, the
network, persistent storage, or other apps' state directly — every
operation is mediated by a kernel-arbitrated capability. WASM
execution is non-negotiable on every backend (native, browser,
mobile); compiling apps to native code for performance is explicitly
rejected.

**What this is not**:

- Not a chat client. Chat is one app among many.
- Not a plugin framework for a host application. Apps are the
  product; the kernel is the substrate.
- Not a CRDT library. Apps may use CRDTs internally; the kernel
  stays generic.
- Not a service to deploy. Peers are the runtime.

**What's novel and what's borrowed**: the "peers as infrastructure"
framing has been claimed by Holochain and Pears; it is not novel on
its own. Myrhiza's distinct combination is: WCM-typed components +
capability-secure host surface + no-CRDT-in-kernel + author-bounded-
scale-at-v1 + event-log-replay convergence with TUTTI-shaped drift
detection. No prior project has shipped this combination. Honest
positioning: not a new pitch, a new combination.

**On "production-validated" claims**: when this spec cites Agoric
and Willow as precedents (§4, §16), note that Agoric is a Cosmos
blockchain (consensus-given event ordering, validator-class
hardware) and Willow is currently a hundreds-of-users-shape chat
product. Neither has stress-tested event-log replay as P2P
infrastructure for write-heavy public-read apps at scale. The
master spec borrows the substrate shape with awareness that scale
validation is a v2+ obligation. See §4.5 + §19 for explicit
scaling acknowledgment.

## 2. The three-tier architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │                       KERNEL                             │
   │  Identity. Peer protocol. Event/DAG primitives.          │
   │  Component loader. Capability arbiter. Crypto            │
   │  primitives. Narrow native imports.                      │
   └──────────────────────────────────────────────────────────┘
                              ▲
                              │ host imports (WIT-typed)
                              │
   ┌──────────────────────────────────────────────────────────┐
   │              MODULES  (myrhiza-* WASM components)        │
   │  Cross-cutting concerns reusable across apps:            │
   │  - Participation: social-graph, tit-for-tat, ...         │
   │  - Permission: rbac, governance, invite-chain, ...       │
   │  - Crypto: mls, channel-key, double-ratchet, ...         │
   │  - State helpers: snapshot-cache, log-prune,             │
   │    crdt-{automerge,yjs,loro}, ...                        │
   │  - Identity: multi-device, behavior, ...                 │
   │  - UI: components, theme-tokens, accessibility, ...      │
   └──────────────────────────────────────────────────────────┘
                              ▲
                              │ component imports / wac composition
                              │
   ┌──────────────────────────────────────────────────────────┐
   │                         APPS                             │
   │  counter, poll, chat, kanban, wiki, etc.                 │
   │  Compose modules + add app-specific state-apply +        │
   │  state-propose + interaction + behavior components.      │
   └──────────────────────────────────────────────────────────┘
```

**Kernel** is the privileged layer. Owns secrets, brokers all I/O,
arbitrates every cross-component call. Compiles to native (Wasmtime
host) or browser (jco-shimmed JS+wasm host).

**Modules** are reusable WASM components encapsulating cross-cutting
concerns. They look like apps to the kernel — same WASM Component
Model, same manifest format, same distribution channel — but are
designed to be pulled in by other apps as dependencies. App authors
declare module deps in their `manifest.toml`; the kernel intersects
capability declarations and links at instantiation time.

**Apps** are user-facing bundles. They compose zero or more modules
and add their app-specific component code (state-apply, state-propose,
interaction, behavior).

The tier separation is conceptual, not enforced — modules and apps
are mechanically the same shape (WASM components with manifest +
signature). The distinction is intent: modules are designed for
reuse; apps are designed for end users.

### 2.1 Why three tiers

**Why modules and not just apps**: cross-cutting concerns
(participation enforcement, RBAC, MLS, snapshot management) recur
across many apps. Without modules, every app reinvents these
patterns. With modules, each pattern is authored once, audited
once, distributed once.

**Why modules and not kernel features**: cross-cutting concerns
evolve faster than kernel ABI. Pinning MLS in the kernel locks
Myrhiza to one MLS implementation; pinning RBAC in the kernel
locks one permission model. Modules let the ecosystem evolve
without breaking kernel ABI.

**Why kernel and not just modules**: identity custody, capability
arbitration, deterministic state-apply replay, network plumbing,
content addressing — these need privileged access to native
resources (private keys, sockets, filesystem). They cannot be
modules without breaking the sandbox model.

The three tiers correspond to three trust boundaries: kernel is
trusted absolutely; modules are sandboxed but typically authored
or audited by the project; apps are sandboxed and may come from
anywhere.

## 3. Component profiles

Components within an app or module declare a runtime profile.
Profiles differ in determinism requirements and which host imports
they may bind.

| Profile | Purpose | Determinism | Where it runs |
|---|---|---|---|
| `state-apply` | Materialize event into state; authority verdict | **Strict** — pure function of `(prior state, event)` plus deterministic helper set | Every peer materializing the topic |
| `state-propose` | Build candidate event from intent | Loose — kernel re-checks via `state-apply` in dry-run | The peer originating the event |
| `interaction` | UI / user-facing surface | Non-deterministic OK; per-peer | Any peer with a UI / agent host |
| `behavior` | Bots, bridges, automations | Non-deterministic OK; per-(peer, instance) identity | Designated peer(s) |

A fifth role, **maintenance** (PR #636's 4th profile in earlier
framing), is not a separate profile. Peers performing maintenance
work do so by instantiating maintenance-shaped components — these
are usually `state-apply` (for replay buffering, snapshot
provision) or `behavior` (for archival, sync) profile components.
"Maintenance" is a deployment posture, not a runtime profile.

### 3.1 state-apply (strict purity)

The most constrained profile. A `state-apply` component is a pure
function over `(prior state, event)` returning a new state. The
kernel calls it during normal event ingestion (apply mode) and
during pre-check (dry-run mode against a hypothetical post-state).

**Permitted host imports**: only the deterministic helper set (see
§7). All return values are pure functions of inputs given the event
payload alone. No clock, no randomness, no network, no filesystem,
no environment, no threads.

**Floats**: banned at v1. App authors use scaled integers. Future
relaxation possible via manifest declaration `state-apply.allow-floats
= true` in a future child spec.

**Fuel**: instruction-count-based budget. Running out terminates
uniformly across peers. Wall-clock timeouts are not used because
they would diverge across peer hardware.

**Why strict**: cross-peer convergence is the load-bearing property.
If two peers run the same `state-apply` against the same event log
and get different state hashes, the system has failed. Strict
purity is how we prove convergence by construction.

### 3.2 state-propose (loose)

Builds a candidate event from user intent. Runs once on the
originating peer; the kernel re-runs `state-apply` (dry-run) to
verify the candidate before signing and broadcasting.

**Permitted host imports**: `host.hlc` (current hybrid logical
clock), `host.random`, `host.seal` (capability-gated, for sealing
content under app-declared key handles), `host.log`, plus the
deterministic helper set.

**Why loose**: intent generation legitimately needs entropy and
clock. The kernel re-checks via `state-apply` so non-determinism
in propose cannot leak into agreed state.

### 3.3 interaction (non-deterministic, per-peer)

User-facing UI surface. Per-peer state (cursor position, scroll
state, draft text) lives here.

**Permitted host imports**: `host.broadcast`, `host.subscribe`,
`host.kv` (per-peer key-value store), `host.user-prompt`, the UI
app's `ui:*` interfaces (panel, list, message, form, menu, etc.),
`host.open` (decryption for display).

**Determinism**: not required. Interaction state is local to each
peer; there is no convergence guarantee.

### 3.4 behavior (non-deterministic, per-(peer, instance))

Bots, bridges, automations. Long-running processes that observe
events and emit new ones.

**Permitted host imports**: superset of interaction's plus
`host.http`, `host.timer`, `host.author-event` (with behavior-
scoped IdentityScope; see §6).

**Identity**: per-(peer, instance). When a peer enables a behavior,
the kernel allocates a fresh IdentityScope under the peer's identity
with `instance: { peer, kind: behavior, name: <app-chosen> }`. Events
authored by the behavior are signed under this scope. The runtime
does not migrate behavior keypairs between peers; cross-peer behavior
continuity is an app-level concern (apps that need stable bot
identity across peers register an in-band mapping event).

### 3.5 Normative host import surface

The canonical reference for permitted host imports per profile.
Subsequent sections (§5 deterministic helper set, §9 crypto primitives)
expand on individual imports but do not contradict this table. When
this table changes, the master spec changes — host imports are an ABI
commitment.

| Host import | state-apply | state-propose | interaction | behavior |
|---|---|---|---|---|
| `host.log(level, msg)` | permitted (output-only) | permitted | permitted | permitted |
| `host.hash(bytes)` (BLAKE3) | permitted | permitted | permitted | permitted |
| `host.verify-signature(pubkey, msg, sig)` (Ed25519) | permitted | permitted | permitted | permitted |
| `host.verify-payload-mac(envelope, key-handle)` | permitted | permitted | permitted | permitted |
| `host.install-key(handle, sealed-distribution-blob) -> ()` | permitted | permitted | permitted | permitted |
| `host.now-hlc-from-event(event-bytes)` | permitted | permitted | permitted | permitted |
| `host.author-event(scope, event-payload)` | denied | denied (kernel signs after pre-check) | denied | permitted (with behavior scope) |
| `host.hlc()` (peer-local HLC) | denied | permitted | permitted | permitted |
| `host.random(bytes)` | denied | permitted | permitted | permitted |
| `host.broadcast(topic, payload)` | denied | denied (kernel handles) | permitted | permitted |
| `host.subscribe(topic) -> handle` | denied | denied | permitted | permitted |
| `host.kv.get(handle, key)` | denied | denied | permitted | permitted |
| `host.kv.put(handle, key, val)` | denied | denied | permitted | permitted |
| `host.kv.delete(handle, key)` | denied | denied | permitted | permitted |
| `host.kv.list-prefix(handle, prefix)` | denied | denied | permitted | permitted |
| `host.user-prompt(prompt) -> response` | denied | denied | permitted | denied |
| `host.seal(handle, plaintext)` | denied | capability-gated | denied | capability-gated |
| `host.open(handle, ciphertext)` | denied | denied | capability-gated | capability-gated |
| `host.can-open(handle) -> bool` | denied | denied | permitted | denied |
| `host.x25519-ecdh(scope, peer-pubkey)` | denied | denied | denied | capability-gated |
| `host.hkdf-derive(input, info, length)` | denied | denied | denied | capability-gated |
| `host.aead-seal(key, nonce-handle, plaintext, ad)` | denied | denied | per-call gated | per-call gated |
| `host.aead-open(key, nonce, ciphertext, ad)` | denied | denied | per-call gated | per-call gated |
| `host.timer.{schedule,cancel}` | denied | denied | denied | permitted |
| `host.http.request(req) -> token` | denied | denied | denied | per-call gated |
| `host.clipboard.write(text)` | denied | denied | per-call gated | denied |
| `host.file-picker.show()` | denied | denied | per-call gated | denied |
| `host.navigation.top-level(url)` | denied | denied | per-call gated | denied |
| `host.push.register(...)` | denied | denied | per-call gated | denied |
| `host.clipboard.read()` | denied | denied | **denied at v1** | denied |
| `host.geolocation.read()` | denied | denied | **denied at v1** | denied |
| `host.microphone.record(...)` | denied | denied | **denied at v1** | denied |
| `host.camera.capture(...)` | denied | denied | **denied at v1** | denied |
| `host.screen-capture.record(...)` | denied | denied | **denied at v1** | denied |
| `host.sensor.{accelerometer,orientation,battery,...}` | denied | denied | **denied at v1** | denied |
| `ui:*` interfaces (panel, list, message, form, menu, etc.) | denied | denied | permitted | denied |

Cells:

- **permitted** — bound automatically when the profile loads.
- **capability-gated** — bound only if the calling component's
  manifest declares it (§7.1).
- **per-call gated** — bound but each call rechecks the calling
  component's manifest (§7.3).
- **denied** — never bound; importing it makes the component invalid
  for that profile (component-install lint rejects).

This is the v1 normative surface. Adding an import is an ABI change
(new minor version of the kernel WIT package). Removing or changing
semantics of an import is a breaking ABI change.

**Why state-propose does not have `host.author-event`**: propose
returns an unsigned candidate event payload to the kernel. The kernel
runs `state-apply` in dry-run mode against a hypothetical post-state
(§4.4 pre-check), and only if pre-check returns Accept does the kernel
sign the event under the user's IdentityScope and broadcast it. Propose
never sees a private key and never produces a signature. This makes
the propose-vs-apply gap structurally smaller — propose cannot bypass
pre-check by signing directly.

**Denied capabilities at v1** (clipboard read, geolocation, microphone,
camera, screen capture, sensors): these capabilities exist as host-
imports to make their absence explicit. Their *absence* in the
kernel WIT package means components attempting to import them fail
at component-install lint. If a future kernel minor adds any of
these, they MUST be `per-call gated` (never `capability-gated` or
`permitted`) because credential-exfiltration via clipboard read and
device-fingerprinting via sensors are well-known attack classes.

**Why behavior gets `host.author-event` rather than `host.sign-via-scope`**:
the kernel enforces that the signed payload is a structurally-valid
event under the app's WIT contract (envelope shape, deps array, payload
type). A compromised behavior cannot use the kernel's signing capability
to sign arbitrary non-event bytes (e.g. a fake bundle manifest, a fake
identity claim) under the user's behavior scope.

**Nonce handling for AEAD**: the kernel manages nonces. `host.aead-seal`
takes a `nonce-handle` (kernel-allocated, monotonically-derived) rather
than raw nonce bytes. `host.aead-open` takes raw nonce bytes (since the
ciphertext author is responsible for transmitting them). This eliminates
nonce-reuse-by-mistake on the seal path.

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

`HeadsSummary` is also used on the **revocation topic** (§10.7) for
backfill of missed revocations on peer start. Same protocol shape;
revocation-event-shaped payloads instead of app events.

Out-of-order delivery is buffered in a `PendingBuffer` with two
eviction policies (independent): age-based (default 1 hour TTL) and
capacity-based (default 10,000 entries with per-author sub-cap of
`max_entries / 50` to thwart Sybil-shaped flooding).

**Snapshots are out of v1 scope** (per §19 Project-shape v2). v1
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
§5.4). Future kernel majors may add format opt-ins (e.g. postcard
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
  drops dramatically. Requires participation-enforcement (§12.5).
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
- §3 Warrants (bad-author signaling — see §4.4.1 future direction)
- §4 Countersigning (multi-author atomic events — relevant for
  governance modules; deferred)
- §6 Membrane proofs (capability-bound app entry — relevant for
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
  bundle (§10). Different versions of the same app have different
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

**Per-app namespace property** (acceptance criterion #4 in §15.1):
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

Strict state-apply purity (§3.1) gives convergence-by-construction,
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
modulo N (default N=1024 events; tunable per-app via manifest §10.2).
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

## 5. Determinism

The convergence proof rests on three legs: content-addressed events
(per §4), deterministic topo-sort (per §4.1), and pure `state-apply`
(this section).

### 5.1 Deterministic helper set

The exact set of host imports `state-apply` may bind. Each is a
pure function of its inputs given the event payload alone. No
peer-local return values; no information about who-can-decrypt;
no clock; no randomness.

The set is **normative** at v1 (any addition is a kernel minor
version bump; any removal or semantic change is breaking):

```wit
host.verify-signature(pubkey: list<u8>, msg: list<u8>, sig: list<u8>) -> bool
host.verify-payload-mac(envelope: list<u8>, key-handle: key-handle) -> bool
host.hash(bytes: list<u8>) -> list<u8>
host.install-key(handle: key-handle, sealed-distribution-blob: list<u8>) -> ()
host.now-hlc-from-event(event-bytes: list<u8>) -> hlc
host.log(level: log-level, msg: string) -> ()
```

**Algorithm pins** (master-spec normative; do not defer to crypto
child spec):

- `host.verify-signature` — Ed25519 only. RFC 8032 strict (rejects
  non-canonical s-values, malleable signatures). This is non-
  negotiable due to Cremers ETK 2025 (§6.2 + `prior-art/mls/critiques.md`).
  ECDSA is forbidden anywhere in the kernel surface.
- `host.hash` — BLAKE3, canonical 32-byte output. Pinning the algorithm
  is required because `state-digest()` (§4.3) gossips the hash for
  convergence verification; algorithm divergence breaks convergence.

Notes on each helper:

- **`host.install-key` returns `()` deliberately.** A boolean indicating
  "this peer can decrypt" would peer-locally branch state-apply,
  breaking determinism. Whether this peer can use the key is queried
  separately from interaction profile via `host.can-open(handle)`.
  The kernel-side bookkeeping the call updates IS part of the
  deterministic state surface — kernel implementations record the
  (handle, sealed-distribution-blob) pair on every peer in the same
  way; only the per-peer X25519 keystore (which determines actual
  decryptability) is peer-local and not visible from state-apply.
- **`host.verify-payload-mac` proves key possession, not author identity.**
  Author identity comes from the outer Ed25519 signature on the event
  itself. Verifying a MAC tells you "some holder of the key bound to
  this handle sealed this," nothing more. The handle binding is itself
  a deterministic function of the event log (via `install-key`).
- **`host.now-hlc-from-event`** is a pure decoder over event bytes.
  HLC is signed into the event envelope by the originator and extracted
  here by every peer. The kernel never consults the wall clock when
  serving this helper.
- **`host.log`** is output-only and does not affect state-digest. Log
  content is **not part of the cross-peer convergence surface** —
  implementations write log lines to a peer-local sink; cross-peer
  log content is not required to match.

**Side-channel resistance**: kernel implementations of all six helpers
MUST be constant-time with respect to secret inputs. State-apply must
not be able to infer per-peer secret state via timing differences in
helper return.

**Side-channel scope clarification**: the constant-time obligation
covers helper-internal computation over secret inputs (private keys,
key handles backed by symmetric secrets). It does NOT cover:
- Cache-timing leaks in Wasmtime's own execution of the WASM bytecode
  that consumes/branches on secret-derived values (mitigation via
  WASM cache-conscious crypto patterns; v1 audit obligation).
- Speculative-execution side-channels between components in the same
  Wasmtime instance (Wasmtime upstream issue; v1 accepts the residual
  risk; documented in §19).
- Capability-gate dispatch timing (whether a specific origin is
  allowlisted for `host.http.request` may leak via timing). v1
  mitigation: kernel implements capability checks via constant-time
  set-membership lookups for the high-value-op list.

**`host.install-key` kernel-side bookkeeping**: the kernel maintains
a deterministic-state map `installed-keys: BTreeMap<KeyHandle,
SealedDistributionBlob>` per app instance. The map is updated by
`host.install-key` calls during state-apply replay; every peer
applies the same events in the same canonical order, so the map is
identical across peers. The map IS part of the deterministic state
surface but is NOT directly visible to state-apply via
`state-digest()`. Apps that want to expose key-handle state via
their digest must materialize the relevant subset into their own
app state via state-apply. Helpers `host.verify-payload-mac` and
(interaction-side) `host.can-open` consult this map but do not
expose its contents to state-apply.

### 5.2 Denied imports for state-apply

- No wall clock. No randomness. No network. No filesystem. No
  environment. No threads.
- No floats at v1 (per §3.1). State-apply WASM modules importing or
  using float ops are rejected at component install time.
- No SIMD-float ops even if floats are eventually allowed; cross-platform
  divergence vectors.
- No nondeterministic instructions (e.g. `now-from-host-clock`).

### 5.3 Fuel and resource limits

Instruction-count fuel budget per state-apply invocation. Running
out terminates uniformly across peers.

**v1 normative defaults** (must be pinned at master-spec level
because cross-peer fuel determinism depends on every peer running the
same fuel-cost-table AND the same per-invocation budget):

- **state-apply per-event fuel budget**: 10,000,000 (10M) Wasmtime
  fuel units per `apply()` call. Sufficient for ~10^6 typical
  instructions on Wasmtime LTS reference fuel-cost-table.
- **state-propose per-event fuel budget**: 50,000,000 (50M) units
  (5x apply; loose-determinism profile may use complex logic).
- **Memory cap per component instance**: 64 MB.
- **Maximum event payload size**: 1 MB.
- **Maximum DAG deps array size**: 64.

**Pre-check shares apply's per-event fuel budget**. Pre-check fuel
exhaustion = pre-check fail-closed (event not signed). The shared
budget intentionally penalizes apps with expensive validation logic
— pre-check + apply combined cannot exceed 10M units, so apps
designing expensive checks see the cost on the originating peer
first.

**Why these defaults at master-spec level**: deferring to a child
spec means two kernel implementations could pick different defaults,
and convergence diverges at fuel exhaustion (peer A applies, peer B
traps; peer A advances state, peer B doesn't). The defaults MUST be
the same across all v1 implementations. Future kernel majors may
revise defaults; doing so is a kernel-major version bump.

**Per-host-call fuel costs**:
- `host.hash(bytes)` — `n * 5` units where n is byte-length (BLAKE3
  reference cost).
- `host.verify-signature(...)` — 5,000 units (Ed25519 verify cost).
- `host.verify-payload-mac(...)` — 1,000 units (MAC verify).
- `host.install-key(...)` — 100 units.
- `host.now-hlc-from-event(...)` — 50 units.
- `host.log(level, msg)` — `100 + n` units.

These are calibrated for the Wasmtime LTS reference fuel-cost-table.
Bumping Wasmtime LTS may require recalibration as a kernel major
bump (per §14.2).

### 5.4 Encoding for state-digest

Apps export `state-digest()` returning canonical bytes for
cross-peer convergence verification. The encoding must be
deterministic.

**v1 commitment**: `bincode 1.3.x` with the explicit `Options` chain
`bincode::DefaultOptions::new().with_fixint_encoding().with_big_endian()`
(or equivalent precise pin), backed by `serde 1.0.x` (any 1.0 minor),
over `BTreeMap` / `BTreeSet` collections.

The `Options` chain MUST be pinned exactly because bincode 1.3 has
multiple `serialize`/`deserialize` entry points with different defaults
(function-level `bincode::serialize` vs `Options::with_*` builder).
Two correct implementations following different idiomatic patterns
can produce different bytes — the convergence-divergence the spec is
designed to prevent. This is a **firm pin**, not a
default — changing it is an ABI break that requires a new kernel
major version. Apps must canonically encode their state via this
combination; `HashMap`, `HashSet`, and other unordered collections
are forbidden in any field that contributes to `state-digest()`.

**Why pin instead of defer**: `state-digest` is the convergence
verification primitive (§4.3). Two kernel implementations picking
different formats produce different digest bytes for identical
state, causing convergence false-positives and breaking the cross-
peer agreement check at acceptance criterion #2 (§15.1). Format
must be specified at master-spec level.

**Why bincode 1.3.x specifically**: it is what Willow ships
(`prior-art/willow/state-machine.md`); it is byte-deterministic over
`BTreeMap`/`BTreeSet`; it is mature and audited. Bincode 2.x has a
different default config; pinning to 1.3.x avoids that drift.
`postcard` was considered but the format choice does not justify
the migration cost without a forcing reason — bincode is sufficient
for v1.

**The load-bearing discipline is sorted collections.** `BTreeMap`
or `BTreeSet` everywhere in any field reachable from `state-digest()`.
`#[serde(skip)]` for any unordered indices. Future kernel majors
may relax format choice (e.g. allow apps to opt into postcard via
manifest declaration) without changing the discipline.

**Event envelope encoding**: events themselves (the wire bytes
hashed to produce `EventHash`) use the same bincode 1.3.x +
explicit Options chain. `host.now-hlc-from-event(event-bytes)`
operates on these canonical bytes — two peers receiving the same
event hash see identical envelope bytes and decode identical HLCs.
The kernel rejects events that fail strict-canonical-decode (any
byte string that doesn't round-trip is invalid).

## 6. Identity primitive

A single kernel primitive covers user identity, multi-device
identity, behavior identity, and MLS LeafNode identity.

### 6.1 IdentityScope

```wit
// identity-handle is an opaque WIT resource — components hold it but
// cannot inspect or forge its contents. Resource lifecycle is kernel-
// managed.
resource identity-handle {
    // No methods exposed to components. Handles are passed by value to
    // host imports that consume them.
}

// peer-handle is also opaque; one per peer the kernel currently knows.
resource peer-handle {}

record identity-scope {
    long-term: borrow<identity-handle>,
    instance: option<instance-binding>,
}

record instance-binding {
    peer: borrow<peer-handle>,
    kind: instance-kind,
    name: string,
}

variant instance-kind {
    device,
    behavior,
    mls-leaf,
    custom(string),
}
```

`long-term` is the durable user/author/member identity. `instance`,
when present, is the short-lived per-(peer, instance) signing scope
nested under that long-term. `instance: none` means the operation is
performed by the long-term identity directly.

The kernel custodies all private keys. Components see only opaque
`identity-handle` resources (non-forgeable, non-inspectable per WCM
resource semantics) and scope records that borrow handles. To sign,
components call:

```wit
host.author-event(scope: identity-scope, event-payload: list<u8>) -> sig
```

The kernel verifies the calling component is authorized to use the
scope (per §7), validates that `event-payload` is a structurally-valid
event under the app's WIT contract (envelope shape, deps array,
payload type), looks up the appropriate private key, signs the
canonical encoding, and returns the signature. Private keys never
enter component memory.

**Why structural validation matters**: a compromised behavior
component cannot use the kernel's signing capability to sign arbitrary
non-event bytes (e.g. a fake bundle manifest, a fake identity claim)
under the user's behavior scope. The kernel rejects malformed payloads
before signing.

**Per-(profile, payload-variant) authorization**: structural validity
checks "well-formed under the WIT type"; it does NOT check "the
calling profile/component is authorized to author this specific
variant." Apps that want fine-grained variant-level control declare
**permitted-author-set** in their manifest:

```toml
[author-policy]
# Map from profile to set of payload variants that profile may author.
# Variants not listed are forbidden (deny-by-default).
state-propose = ["UserAction", "Comment"]
behavior = ["AutoArchive", "RemindEveryone"]
```

The kernel checks `(calling-profile, payload-variant)` against this
manifest at every `host.author-event` call. Variant identification
uses WIT variant tag names. Apps that omit `[author-policy]` default
to "any profile may author any variant" (current behavior; useful for
simple apps but loses defense-in-depth).

Apps using behaviors for limited tasks (e.g. auto-moderation) should
declare a tight `behavior` variant set so a compromised behavior
cannot author admin-class events.

**v1 default for `[author-policy]`**: deny-by-default. Apps that
omit `[author-policy]` may NOT use `host.author-event` at all under
non-state-propose profiles (i.e. `behavior` profile cannot author
events without explicit policy). Apps that explicitly set
`policy = "permissive"` opt out (any profile may author any variant)
— useful for simple apps where the cost of variant enumeration
outweighs the security benefit.

This makes defense-in-depth the default and forces app authors to
*think* about which variants behaviors should be allowed to author,
rather than getting authorization-bypass-by-omission.

**State-apply re-validation**: every peer's state-apply re-checks
`(calling-profile, payload-variant)` against the manifest's author-
policy at apply time, not just at originator-side propose. Since
the manifest is content-hash-pinned via `app_bundle_hash`, every
peer materializing the topic shares the same author-policy. A
compromised originator that bypassed local pre-check still fails
remote apply, and the event is rejected from convergence.

**Cremers ETK 2025 enforcement is structural**: the kernel does not
expose any signing API that takes an algorithm parameter.
`host.author-event` always uses Ed25519 (RFC 8032 strict). Manifest
fields cannot declare alternative algorithms. ECDSA is unreachable
through the kernel surface.

### 6.2 Use cases under one primitive

| Use case | long-term | instance |
|---|---|---|
| User signing (single device) | User Ed25519 | none |
| User multi-device | User Ed25519 | `kind: device, name: "laptop"` |
| Behavior bot | Owner Ed25519 | `kind: behavior, name: "discord-bridge-1"` |
| MLS member, current epoch | Member Ed25519 | `kind: mls-leaf, name: "epoch-42"` |
| App author signing a release | Author Ed25519 | none |

**Cremers ETK 2025 constraint**: `long-term` MUST use Ed25519 (which
is SUF-CMA secure). ECDSA is EUF-CMA only and fails MLS FCGKA security.
This applies even to non-MLS scopes for forward compatibility. (For
context: Cremers et al. 2025 "End-to-end Tree-based Key agreement"
showed MLS implementations using EUF-CMA-only signatures break
Forward-Compromise Group Key Agreement security; SUF-CMA is the
stricter property Ed25519 provides. See `prior-art/mls/critiques.md`.)

### 6.3 Direction for deferred items

These items have a committed direction in the master spec; detailed
mechanics land in child specs as concrete needs emerge.

- **Device-add and device-revoke flow** — direction: an app-level
  `myrhiza-identity-multi-device` module implements device addition
  via in-band signed events from existing devices. The module wraps
  the IdentityScope primitive; the kernel does not bake device
  semantics. Device revocation is broadcast as a signed retirement
  event under the long-term identity. v2 child spec details.
- **MLS LeafNode lifecycle integration** — direction: the
  `myrhiza-crypto-mls` module composes IdentityScope with
  `instance-kind: mls-leaf` for epoch-bound signing keys. Per-epoch
  key rotation is module-internal; kernel exposes only primitive
  crypto (§9.2). v2+ child spec details.
- **Recovery semantics when long-term key is lost** — direction:
  social recovery (M-of-N trusted peers attest to a recovery event
  re-binding the long-term identity to a new keypair) OR
  out-of-band recovery via a stored recovery seed. Both deferred to
  multi-device child spec; v1 documents this gap honestly — losing
  a single-device IdentityScope without recovery is permanent identity
  loss.
- **Cross-peer behavior continuity** — direction: apps that want
  stable bot identity across peers register an in-band mapping event
  mapping peer-side behavior keypair to an app-level role; enforced
  by the app's own pre-check. SDK macros default to making this
  binding explicit so app authors don't accidentally ship behaviors
  that lose identity on restart.
- **Behavior identity revocation** — direction: a behavior keypair
  may be revoked by the user via a kernel-side `BehaviorRevoke` event
  authored under the user's long-term IdentityScope, naming the
  (peer, kind, name) tuple. After revocation, future events signed
  under that scope are flagged in derived state as "post-revocation"
  (apps choose whether to treat them as invalid). This handles
  compromised behavior keys without requiring app-level cooperation.
  v2 child spec details mechanics; v1 documents the gap (no
  revocation path for behavior keys).
- **Quantum-safe signature migration** — direction: kernel ABI bump
  with new `instance-kind` variant for PQC scope; existing scopes
  remain Ed25519. App authors opt-in to PQC scopes when modules support
  them. Out of scope until post-quantum schemes (e.g. ML-DSA) reach
  production maturity.

## 7. Capability model

Three layers of gating, each at a different boundary, plus
typed resource handles for non-forgeable inter-component refs.

### 7.1 App boundary — manifest declares ambient set

Every app's `manifest.toml` declares its ambient capability set:
which host imports it may call, which UI surfaces it may bind, which
modules it depends on. The manifest is signed (per §10) so the
declared set cannot be modified after publication.

At install time, the kernel renders a capability summary to the user
(bech32m-encoded author identity, version, declared capabilities).
The user confirms or rejects.

### 7.2 Module boundary — manifest intersection at link time

When an app declares a module dep, the module brings its own
capability declarations (what host imports it requires to function).
The kernel **intersects** the app's ambient set with the module's
required set at component instantiation:

```
M_effective = A_ambient ∩ M_required
```

A module can never exceed the calling app's grants. An app cannot
grant a module more than the module declared needing. This catches
both directions: malicious modules declaring excessive imports, and
apps trying to over-grant.

If the intersection is empty for a required capability — i.e. the
module needs something the app didn't declare — installation fails
with a precise error.

### 7.3 Per-call gating on high-value ops

Specific privileged operations are re-checked at every call against
the **calling component's** manifest:

- Clipboard write
- File picker invocation
- Top-level navigation
- Push notification registration
- AEAD seal/open with sensitive keys
- Network egress to specific origins (when interaction calls out to
  third-party services)

The list is curated; what counts as "high-value" is a child-spec
concern. The mechanism is uniform: the WIT contract for each such op
includes a per-call gate annotation; the kernel reads the calling
component's manifest at invocation time and rejects calls that
exceed its declared scope.

This catches social-engineering attacks: an interaction component
asking a UI module to write to clipboard cannot escalate beyond the
*interaction component's* clipboard grant, even if the UI module
itself has clipboard access.

### 7.4 Resource handles for non-forgeable refs

WASM Component Model resource handles (free from §8's full-CM ABI
choice) are the unit of explicit capability transfer. Apps pass
scoped handles to modules to grant fine-grained access:

```wit
// (Illustrative pseudocode. The actual API for creating private
// channels is defined in the kernel WIT package; the example below
// shows the pattern, not the exact host import name.)
let channel-handle = host.create-private-channel();  // illustrative
my-module.process(channel-handle);
```

The module cannot forge equivalent handles. It can only use what
was passed.

This pattern complements §7.1–7.3: ambient grants set the bounds,
intersection scopes the module, per-call gates protect high-value
ops, and resource handles enable explicit fine-grained transfer.

### 7.5 Defense in depth

The four layers catch different attack classes:

| Attack | Caught by |
|---|---|
| Malicious app over-declares capabilities | User reviews at install (§7.1) |
| Malicious module declares more than it needs | Manifest intersection (§7.2) |
| Compromised module attempts privilege escalation | Manifest intersection + per-call gate (§7.2 + 7.3) |
| Module forges capability ref | Resource handle non-forgeability (§7.4) |
| Social engineering across components | Per-call gate (§7.3) |
| Compromised behavior signing fake non-event payloads | Structural validation in `host.author-event` (§6.1) |
| Silent capability widening on update | Per-update install flow re-runs capability summary (§10.5) |
| Typosquatting on module names | Content-hash binding (§10.6); name is informative only |

The cost of these layers is **moderate, not free**. Manifest
intersection requires a capability vocabulary registry + intersection
check at every component instantiation. Resource handles come with
WCM full CM (Q2-A) but require disciplined SDK design. Per-call gating
needs WIT annotation infrastructure and a manifest lookup at every
high-value-op invocation (microseconds-class per call, but real). The
benefit justifies the cost: comprehensive containment of modules,
which is essential because modules are pulled in by apps and may come
from third parties.

**What this defense does NOT cover**:
- A user who explicitly grants a malicious app full capabilities at
  install (§7.1 user review can be ignored)
- A network adversary that controls the relay infrastructure (relays
  are dumb topic bridges; metadata correlation is a separate threat
  class — see §11.4)
- A malicious admitted member of a topic (group encryption protects
  against outsiders; insiders see what they were invited to see)
- An author whose private key is compromised post-install (revocation
  flow per §10.7 mitigates but cannot fully defend)

## 8. ABI and composition

### 8.1 Decision

**Full WebAssembly Component Model from day one**. WIT-bindgen for
the SDK (Rust). Wasmtime native runtime. jco-transpiled glue + core
wasm in browser.

### 8.2 Why full CM and not Extism

PR #636 leaned Extism v1 → CM v2 for ship-faster reasoning. Myrhiza
rejected this:

- Extism cannot express WCM resource handles, borrows, world
  composition, or futures/streams. Migration is a real refactor for
  app authors — not a regenerate-bindings event.
- Every Myrhiza app and module written before migration would be
  rewritten. Including Willow when it eventually refactors. Double-
  rewrite cost is unacceptable.
- We do not have a chat-product-keep-alive deadline that justified
  PR #636's ship-faster framing.
- Submit-and-poll (§8.5) gives us sync-ABI ergonomics regardless;
  full CM does not lose anything to Extism on that axis.

### 8.3 Cross-component composition

Components compose via typed WIT resource handles. A module exports
a WIT interface; an app (or another module) imports it. Resource
handles are non-forgeable refs (§7.4). The kernel arbitrates every
cross-component call.

```
component A imports: my-app:counter
                          ↓
component B exports: my-app:counter  (the counter app)
```

`wac` (WCM composition tool) is supported for build-time composition.
Runtime composition is also supported through the kernel's component
instantiation pathway — apps may load module components dynamically
based on user choice.

### 8.4 No cross-component shared memory

No direct memory sharing between components. Every interaction is
typed, bounded, and refusable. The kernel is the call broker.

### 8.5 Submit-and-poll for inherently async surfaces

Browser jco preview2 does not support async at the WIT boundary.
state-apply is sync by definition. Kernel calls that wrap async
surfaces (gossip broadcast, blob fetch, HTTP, persistent KV, timers)
follow a submit-and-poll pattern:

```wit
// Async surfaces use a -submit / on-completion pair:
host.broadcast-submit(topic: topic-id, msg: list<u8>) -> request-token
host.blob-fetch-submit(hash: blob-hash) -> request-token
host.http-request-submit(req: http-request) -> request-token

// Each profile that uses async surfaces exports a corresponding handler:
on-broadcast-completion(token: request-token, result: result<unit, broadcast-error>) -> ()
on-blob-fetch-completion(token: request-token, result: result<list<u8>, fetch-error>) -> ()
on-http-completion(token: request-token, result: result<http-response, http-error>) -> ()
```

The component returns immediately; the kernel re-enters via the
exported handler when the operation finishes. Back-pressure is
preserved (a slow operation does not stall the component's actor
mailbox).

**Token lifecycle**: tokens are kernel-issued opaque HMAC-tagged
values. Components cannot forge tokens. Each token is single-use —
the kernel rejects repeated `on-completion` calls with the same
token (replay protection per §19). Tokens issued to a component
expire when that component instance terminates.

**Outstanding-token bound**: the kernel caps per-component
outstanding tokens (default 256; configurable at master-spec
implementation time, not via app manifest). When the cap is hit,
new submit calls fail with `would-block-error` and the component
must wait for outstanding completions to drain.

When jco preview3 stabilizes async at the WIT boundary, the
kernel-side adapter migrates without API churn for app authors.

### 8.6 Coarse-grained interfaces

Interaction components return view models in per-surface units (one
channel timeline, one member list, one composer state). Returns are
version-tagged so the host can skip recomposition on no-op state
changes; large lists are paged. Behavior components observe and emit
in batches.

No tight inner-loop callbacks across component boundaries. Cross-
component calls have measurable cost (component instantiation, ABI
translation, capability gate check); coarse granularity amortizes
the cost.

## 9. Crypto primitives and key custody

### 9.1 Kernel custody

All secret material lives in the kernel:

- Private signing keys (per IdentityScope, §6).
- Symmetric channel/group keys.
- Ratchet state.
- MLS group state when adopted.

Components hold opaque handles to keys; the kernel custodies bytes.
Secrets do not enter component memory in their raw form.

### 9.2 Primitive crypto host imports

Provisional WIT contract (refined in crypto-and-key-custody child
spec):

```wit
// signature primitives
host.author-event(scope: identity-scope, event-payload: list<u8>) -> sig
host.verify-signature(pubkey: list<u8>, msg: list<u8>, sig: list<u8>) -> bool

// key agreement
host.x25519-ecdh(scope: identity-scope, peer-pubkey: list<u8>) -> secret-handle

// key derivation
host.hkdf-derive(input: secret-handle, info: list<u8>, length: u32) -> secret-handle

// authenticated encryption
host.aead-seal(key: secret-handle, nonce-handle: nonce-handle, plaintext: list<u8>, ad: list<u8>) -> list<u8>
host.aead-open(key: secret-handle, nonce: list<u8>, ciphertext: list<u8>, ad: list<u8>) -> result<list<u8>, error>
// nonce-handle is kernel-allocated, monotonically-derived per (scope, key);
// app components do not pick raw nonces on the seal path. Open path takes
// raw nonce because the ciphertext author transmits it on the wire.

// hashing (also a deterministic helper for state-apply per §5.1)
host.hash(bytes: list<u8>) -> list<u8>
```

Algorithm choices: Ed25519 (signing), X25519 (ECDH), ChaCha20-Poly1305
(AEAD), HKDF-SHA256 (KDF), BLAKE3 (hashing). Provisional; final
selection is in the crypto child spec.

### 9.3 MLS as a module

The official `myrhiza-crypto-mls` module ships as Myrhiza's canonical
group encryption solution when the first MLS-needing app emerges.
The module implements RFC 9420 entirely in WASM, calling the kernel
crypto primitives for cryptographic operations and the kernel
IdentityScope primitive for member/leaf signing keys.

Kernel does not bake any specific MLS implementation. Module authors
may compete; users may choose alternatives. Post-quantum migration
is a module swap, not a kernel ABI change.

The kernel-baked MLS path (PR #636's `host.mls.*` host imports)
remains open as a future-additive ABI change if module-based MLS
proves insufficient. v1 commits the module path.

### 9.4 Other crypto modules

Common patterns ship as additional modules:

- `myrhiza-crypto-channel-key` — symmetric channel-key encryption
  (Willow-shape).
- `myrhiza-crypto-double-ratchet` — Signal-style DM ratchets when DM
  apps emerge.
- `myrhiza-crypto-sealed-content` — NIP-44/59-shape sealed payloads.

All compose the same primitive crypto host imports.

## 10. Apps, modules, and bundle distribution

### 10.1 Bundle shape

Apps and modules use the same shape:

```
bundle/
├── manifest.toml          author pubkey + version + capabilities
│                          + module deps + signature
├── components/
│   ├── state-apply.wasm
│   ├── state-propose.wasm
│   ├── interaction.wasm
│   └── behavior.wasm      (optional)
├── ui-assets/             (optional; static UI assets if present)
└── signature              Ed25519 over (manifest_hash + content_hash
                           + version + author_pubkey)
```

Modules use the same shape but may not include `state-propose` or
`behavior` profiles depending on what they implement.

### 10.2 Manifest schema (v1 normative)

The manifest schema is part of the v1 master spec, not a deferred
child spec, because §7.2's intersection mechanic cannot be specified
without it.

```toml
[app]
name = "counter"
version = "0.1.0"
description = "Simple shared counter"
# Author identity. bech32m-encoded Ed25519 pubkey with HRP discriminating
# author identity class:
#   wpub-author     — third-party app/module author
#   wpub-myrhiza    — official myrhiza-* module signing root
author-pubkey = "wpub-author1q9q...xy"
author-identity-class = "third-party"   # or "myrhiza-official"

[abi]
kernel-major = 1                   # target kernel major version
kernel-minor-min = 0                # minimum kernel minor for required imports
state-digest-format = "bincode-1.3"  # the only v1 value; future opt-in

[capabilities.host-imports]
# capability-gated host imports; kernel intersects with module deps
"host.author-event" = true
"host.broadcast" = true
"host.subscribe" = true
"host.kv.get" = true
"host.kv.put" = true
"host.kv.delete" = true
"host.kv.list-prefix" = true

[capabilities.ui-surfaces]
"ui:panel" = true
"ui:button" = true

[capabilities.high-value-ops]
# per-call gated; v1-mandatory list. Apps explicitly opt-in.
"host.clipboard.write" = false
"host.file-picker.show" = false
"host.navigation.top-level" = false
"host.push.register" = false
"host.aead-seal" = []              # list of key-handle namespaces app may seal under;
                                   # per-call gated to specific keys
"host.aead-open" = []               # same shape
"host.http.request" = []           # array of RFC 6454 exact origins (scheme + host + port);
                                   # empty = denied. v1 does NOT support glob/wildcard
                                   # patterns (subdomain-injection attack class). Future
                                   # kernel minor may add suffix-wildcard support behind
                                   # an explicit opt-in.

[capabilities.deterministic-helpers]
# state-apply may bind these; always permitted for that profile, listed
# for self-documentation
"host.verify-signature" = true
"host.verify-payload-mac" = true
"host.hash" = true
"host.install-key" = true
"host.now-hlc-from-event" = true

[determinism]
# state-apply discipline. v1 lints reject violations at install.
allow-floats = false               # v1: false; future opt-in via this field

[determinism.drift-detection]
# §4.7 TUTTI-shaped drift detection. Tunes how often each peer emits
# state-digest() output for cross-peer convergence verification.
interval-events = 1024             # emit digest every N events (canonical topo-sort index modulo N)
# Wall-clock backstop is disabled at v1 (would inject peer-local non-determinism)

[modules]
# Module deps. Each entry is content-hash-pinned, not name+version.
# Name is informative; the hash is the trust binding.
[[modules.dep]]
name = "myrhiza-permission-rbac"
content-hash = "blake3:abc123..."
expected-author = "wpub-myrhiza1xyz..."
required-capabilities = ["host.kv"]   # what this module imports from kernel

[[modules.dep]]
name = "myrhiza-state-snapshot-cache"
content-hash = "blake3:def456..."
expected-author = "wpub-myrhiza1xyz..."
required-capabilities = ["host.kv", "host.broadcast"]

[components]
# WASM component artifacts in this bundle, by profile.
state-apply = "components/state-apply.wasm"
state-propose = "components/state-propose.wasm"
interaction = "components/interaction.wasm"
behavior = "components/behavior.wasm"   # optional

[signature]
# Ed25519 signature over canonical encoding of:
#   length-prefixed("myrhiza/manifest/v1") |
#   length-prefixed(BLAKE3(manifest_body_without_signature_section)) |
#   length-prefixed(BLAKE3(components_directory_canonical)) |
#   length-prefixed(version_string) |
#   length-prefixed(author_pubkey_bytes)
# Canonical encoding: each field as 4-byte LE length followed by bytes.
algorithm = "ed25519"
value = "0x..."
```

**Capability vocabulary** is the table in §3.5 plus `ui:*` surfaces.
The v1 `ui:*` minimum vocabulary is enumerated in the kernel WIT
package at v1 ship: `ui:panel`, `ui:list`, `ui:message`, `ui:form`,
`ui:menu`, `ui:button`, `ui:input`, `ui:dialog`. Counter+poll MVP
exercises panel + button + input + form. Apps may declare any
of these; the kernel rejects unknown `ui:*` strings at install.

Apps cannot invent new capability strings outside the kernel-defined
vocabulary; the kernel rejects any unknown capability identifier at
install. Future kernel minor versions may extend the vocabulary; apps
declaring vocabulary requiring a higher kernel-minor are rejected by
older kernels (per `kernel-minor-min` field).

**ABI versioning semantics** are nuanced for state-apply imports:

- **Adding a non-deterministic import** (state-propose / interaction /
  behavior only) is a kernel **minor** version bump. State-apply
  cannot bind it; convergence is unaffected.
- **Adding a deterministic helper** that state-apply MAY bind is a
  kernel **major** version bump. Two peers running different majors
  applying the same event with the same state-apply WASM produce
  different state if the WASM imports a new helper from one major
  but not the other. This is convergence-breaking.
- **Removing or changing semantics of any import** is a kernel
  major version bump.

Apps declare `kernel-major` in manifest. Peers running incompatible
kernel-majors cannot interoperate on the same topic (§11.2 implicit:
topic IDs include `app_bundle_hash` which depends on the kernel-major
the app was built against; cross-major peers cannot subscribe to
the same topic).

**TOML canonicalization for signature**: the manifest signature
(below) is computed NOT over the TOML text itself but over a
**canonical bincode 1.3.x encoding** of the parsed manifest's
typed structure. This eliminates TOML-encoder-library drift entirely.

Canonical-encoding rules:
- Parse manifest with `toml_edit 0.22.x` (pinned at v1; bumping is
  a kernel minor version bump if-and-only-if the encoder is not
  involved in canonical signature computation; otherwise major).
- Convert to typed manifest struct (defined in `myrhiza-manifest`
  WIT package).
- Encode struct via the same bincode 1.3.x + Options chain pinned
  in §5.4.
- BLAKE3 the encoded bytes → `manifest_canonical_hash`.
- Author signs `manifest_canonical_hash + content_hash + version
  + author_pubkey`.

The TOML text is the human-readable representation; the canonical
encoding is the byte-stable signature target. This means apps may
freely re-format their TOML (whitespace, comments, key order) without
breaking the signature, as long as the parsed struct is unchanged.

`[[modules.dep]]` array in the parsed struct is sorted by
`content-hash` alphabetically before encoding (canonical order).
Strings are UTF-8 NFC-normalized at struct-construction time.
Numbers are canonical i64/u64 binary encoding via bincode.

The `[signature]` block is excluded from the body when computing
the signature (the signature signs the body, which by definition
does not contain itself).

Quoted dotted keys are required for capability identifiers containing
dots: `"host.author-event" = true` (unquoted `host.author-event = true`
is parsed as nested table `host.author-event` and conflicts with
sibling capability keys).

**Module dep content-hash discipline**: `content-hash` is the bundle's
iroh-blobs hash. Two modules with the same name but different content
hashes are different modules. Typosquatting is impossible because
the hash is the binding. The `name` field is informative for UI
display only.

**`expected-author` field**: the signing pubkey the kernel expects on
the module's bundle signature. If the module's actual signature is
under a different pubkey, install fails. This catches a compromised
hash-replacement attack.

**`author-identity-class`**: distinguishes third-party apps from
official myrhiza-* modules. The kernel maintains a small built-in
allowlist of `wpub-myrhiza` pubkeys (initially the project's signing
root); any module declaring `myrhiza-official` whose pubkey is not
on this list is rejected at install. This is a soft trust-root
signal — users may trust myrhiza-official modules differently than
third-party.

**Schema evolution**: adding a capability or module field is additive
(new kernel minor version). Removing or changing semantics of a field
is breaking (new kernel major version). The manifest schema version
is implicit in the kernel's `kernel-major` requirement.

### 10.3 Distribution

Bundles distributed via iroh-blobs by content hash. No central
registry. Discovery is out-of-band at v1: hashes shared via links,
QR codes, in-app share. Future-direction (deferred to child spec):
in-band catalog gossip for app/module discovery.

### 10.4 Signing

Author Ed25519 signs `(manifest_hash + content_hash + version +
author_pubkey)`. The signature is part of the bundle. The author
public key is embedded in the manifest.

Author identity reuses the IdentityScope primitive (§6). App
authors are users; user signing keys can sign app releases.
Production-grade authors typically use a separate IdentityScope
long-term identity for releases (separation of concerns).

### 10.5 Install flow

1. User receives bundle hash via out-of-band channel.
2. Kernel fetches bundle via iroh-blobs by hash.
3. Kernel verifies Ed25519 signature against author pubkey embedded
   in manifest. Cremers ETK 2025 enforcement: kernel structurally
   rejects any non-Ed25519 signature algorithm — there is no
   manifest field to declare alternative algorithms.
4. Kernel resolves module deps recursively. For each module dep,
   kernel fetches by content hash, verifies signature against
   `expected-author`, and recursively resolves transitive deps.
   Failures (hash mismatch, signature failure, capability excess)
   abort install with precise error.
5. Kernel intersects capability declarations across the dep tree:
   - Each module's required capabilities are intersected with the
     calling app's ambient set (§7.2).
   - Transitive module deps follow the same rule recursively. A
     module's required capabilities cannot exceed its calling
     module/app's grants.
6. Kernel renders capability summary to user:
   - bech32m-encoded author identity (with HRP class indicator —
     `wpub-myrhiza-...` highlighted as official)
   - version + bundle hash (truncated)
   - capability summary (host imports, high-value ops, ui surfaces)
   - module dep tree (each module's name + content hash + author)
   - high-value-op list separately highlighted
7. User confirms or rejects. **Kernel-controlled UI surface** (chrome
   the app cannot draw over) renders the prompt; high-value-op
   prompts must use the same surface (§7.3).
8. Kernel instantiates the app's components.

**Per-update consent**: when the app or any module dep updates,
step 7 re-runs. Users approve each update individually. Silent
in-place updates are forbidden — capability widening on update is
the attack class this defense closes.

**Per-module-update consent (separate from per-app-update consent)**:
when an app version bump changes ONLY a module dep (no app code
changes, no capability changes), the install flow surfaces the
module update specifically — "App X updated module M from hash Hold
to hash Hnew (capabilities unchanged)" — rather than rolling it
into the app-update prompt. Users may approve the module change
without approving an associated app capability change. This prevents
authors from hiding module substitutions inside larger app updates.

**`on-completion` UI rendering**: high-value-op approval prompts
(per §7.3) MUST render via the kernel-controlled UI surface defined
in §13.2.1 (kernel-rendered chrome that the UI app cannot draw
over). UI app cannot intercept or fake these prompts. Non-privileged
prompts (`host.user-prompt` for general intent) MAY render via the
UI app's own surface, with the understanding that the UI app is in
the TCB for those prompts.

**Capability summary fatigue mitigation** (skeptic finding):
- Default deny for capabilities not explicitly highlighted by the
  user as understood ("auto-approve trivial caps after N installs"
  is rejected).
- High-value-op prompts have a 2-second minimum render time before
  the Approve button enables (anti-clickthrough).
- Bech32m author identity rendered with visual hash icon (e.g.
  4×4 colored grid derived from pubkey) to ease author recognition
  across installs.
- New author identities highlighted as "first time installing from
  this author"; subsequent installs from same author show the same
  hash icon.

### 10.6 Versioning

Semver for human-readable version strings. Bundle hash (content-
addressed iroh-blobs hash) is the **trust binding** — semver is
informative only.

Module deps pin **content hashes**, not semver ranges. An app that
wants to allow semver-compatible upgrades publishes a new app version
referencing the new module hash; users approve the app update
(which surfaces the new module hash in the install flow's capability
summary at step 6).

This makes silent module updates impossible. An app cannot say "I
depend on `^1.0.0` of module X" and have the kernel auto-pull a
patched version; every module-version-bump is an app-version-bump
with explicit user consent.

### 10.7 Revocation

Author retracts a bad version by publishing a **revocation event**
signed under the same author IdentityScope. The revocation event
declares:

```toml
[revocation]
revoked-bundle-hash = "blake3:..."
reason = "string describing why"
revoked-at = "2026-05-09T12:34:56Z"
```

**Distribution mechanism (v1 commitment):**

- Revocations propagate via iroh-gossip on a **per-author
  revocation topic** computed as
  `topic_id = BLAKE3("myrhiza/revocations/v1" | author_pubkey)`.
- Every peer that has ever installed an app or module signed by
  this author auto-subscribes to the author's revocation topic on
  install.
- When a revocation arrives, the kernel surfaces it to the user
  for any installed bundle matching `revoked-bundle-hash`. User
  is prompted to uninstall (default action) or pin (explicit
  opt-in).
- Revocations are append-only and signed; previous revocations
  cannot be retracted.

**Threat model coverage:**

- **Author key compromise**: if the author's key is leaked, an
  attacker can forge revocations or new releases. The kernel cannot
  distinguish; users must out-of-band verify if a sudden revocation
  storm appears suspicious. Future direction: key transparency
  log + petname registry (deferred to identity-binding child spec).
- **Stale-network attack**: an adversary may withhold revocation
  events from a target peer. Mitigation: revocation topic is part
  of the auto-subscribed set; peers run a HeadsSummary-shape sync
  on the revocation topic at start to backfill missed revocations.
  Peers without a fresh sync within 24 hours surface a "potentially
  stale" warning before installing a new version.
- **Mass revocation by malicious author / flooded revocation topic**:
  revocation events MUST carry a monotonically-increasing
  `revocation-seq: u64` per author. The kernel tracks the highest
  observed `revocation-seq` per author and rejects revocations with
  lower or equal seq. Single-key compromise can therefore at-most
  publish one revocation per (author, seq); a flood of fake
  revocations under the same seq is structurally impossible.
  **Maximum seq jump**: the kernel rejects any revocation whose seq
  exceeds `last_observed_seq + MAX_REVOCATION_JUMP` (default 1024
  per author per 24-hour window). This prevents a compromised key
  from publishing seq=`u64::MAX` and bricking the author's
  revocation channel. If the legitimate author needs to revoke many
  bundles fast, they may publish at most 1024 revocations per
  24-hour window. Users may pin a specific bundle hash (decline
  revocation); the kernel surfaces pinning prominently when an
  author's revocation sequence jumps abnormally fast.

**Subscription enumeration risk**: a relay observing revocation
topic subscriptions can enumerate which peers ever installed software
from author A (subscription is sticky after install). This is part
of the §11.4 metadata-correlation surface; mitigation requires
relay rotation + topic-subscription cover (out of scope for v1;
named in §19).

**Out of scope at v1**: certificate-transparency-style log;
post-revocation re-keying; revocation forwarding via third-party
attestations.

### 10.8 No central registry

No Myrhiza-operated registry. No sigstore dependency. No reliance on
any centralized service for app distribution. P2P-native distribution
is non-negotiable; matches iroh-blobs commitment and the project's
no-deployed-infrastructure framing.

### 10.9 Myrhiza-official signing root

The kernel maintains a small built-in allowlist of bech32m-encoded
Ed25519 pubkeys with HRP `wpub-myrhiza` recognized as the official
project signing root. Modules signed by these pubkeys may declare
`author-identity-class = "myrhiza-official"` in their manifest.

The allowlist is hard-coded in the kernel binary (a `const` in
`crates/kernel/src/identity/official_root.rs`). Updating the allowlist
requires a kernel binary update — i.e. users must re-install the
kernel to trust new official pubkeys.

This provides a soft trust-root signal — modules signed by listed
pubkeys may be treated differently in install UX (e.g. less prominent
warnings) but the kernel does not block third-party modules.

**Initial allowlist members** (provisional; pinned at v1 ship time):
- The Myrhiza project's primary release-signing pubkey.
- Three backup pubkeys held offline by separate maintainers, used
  for **community-attested rotation** of the primary key. Rotation
  procedure: the new primary pubkey is announced via three separate
  channels (project website / community forums / signed posts under
  maintainer identities), and an emergency kernel binary update
  carries the new allowlist. The backups are not used as a
  cryptographic threshold signature in v1 — proper threshold-Ed25519
  schemes (e.g. FROST-Ed25519 IETF draft) are not yet RFC-stable
  and adding their verification logic to the kernel TCB at v1 is
  premature. Future kernel majors may adopt FROST-Ed25519 once it
  reaches RFC.

**v1 rotation is policy + emergency-update, not cryptographic
threshold.** This is honest about what the maintainer ceremony
actually provides. The threat model assumes that compromising the
primary key requires also compromising the kernel-update channel
(§10.10) for an attacker to land malicious modules — defense in
depth via separate trust roots, not via threshold cryptography.

### 10.10 Kernel binary distribution and authentication

The Myrhiza-official allowlist (§10.9) is the trust root for module
signing. **The kernel binary itself is the trust root for the
allowlist.** Distribution and authentication of the kernel binary
matter as much as any in-spec security mechanism.

**v1 kernel distribution channels** (operator chooses):

1. **OS package managers** (homebrew, apt, dnf, MSI). The package
   manager's signing infrastructure verifies the binary; users trust
   the package manager root. This is the recommended path for desktop.
2. **GitHub release artifacts** with reproducible builds. The Myrhiza
   project signs each kernel release with a separate offline-key
   "kernel signing root" (distinct from the module-signing allowlist).
   The kernel-signing-root pubkey is published in the project README,
   on the project website, and via the `wpub-myrhiza-kernel` HRP. Users
   verify by checking the kernel binary's signature against this root.
3. **Reproducible build verification**: kernel source is open; users
   may build from source and compare against published checksums.

**Kernel-signing-root rotation**: if the kernel-signing-root key is
compromised, the project announces rotation via:
- Out-of-band channels (project website / community forums / signed
  social posts under maintainer identities).
- A signed advisory pushed via the `myrhiza-revocation` topic
  (§10.7-shape) under the offline backup keypair.
- Distribution channels (OS package managers, GitHub) are updated to
  the new signing root.

**v1 acknowledged risk**: a sophisticated adversary controlling both
the project's release infrastructure AND the OS package manager could
distribute a compromised kernel. Mitigation is reproducible builds +
multi-channel announcements; v1 does not commit a transparency log
or third-party attestation. Future direction (v2+): kernel-binary
transparency log + community attestation.

**Out-of-band trust still required for first-install**: a user
downloading Myrhiza for the first time must trust the publication
channel. Users who care can verify the binary against the
kernel-signing-root pubkey published on the project website (HTTPS
+ DNSSEC) and via community-mirror posts. v1 does not hide this
gap; it is the standard "trust the OS package manager" model used
by every desktop runtime.

## 11. Networking, sync, and relays

### 11.1 Transport

iroh — gossip, content-addressed blob fetch, dial-by-pubkey QUIC,
DERP-style relay-bridged NAT traversal. The locked load-bearing
transport dependency.

**Version pin**: v1 commits to a specific iroh version pinned at
implementation start (likely `iroh 1.0.0` once stable, or the latest
RC at v1 ship). iroh's pre-1.0 API churn is real
(`prior-art/iroh/lessons.md`); v1 absorbs the pin and budgets for
upgrade pain.

**Network trait abstraction is preserved as a design seam.** Even
though iroh is committed for v1, the kernel-internal `Network` trait
(see §15.4 `crates/network/`) is shaped so a future kernel could
swap transports if iroh strategy shifts (Number 0 has redirected
before; iroh-ffi was mothballed). Trait shape: gossip publish/subscribe,
blob publish/fetch by content hash, dial-by-pubkey, NAT-traversal
hint. v1 ships only the iroh implementor; the seam exists for
optionality.

The kernel exposes a narrow networking surface to apps via
capability-gated host imports (broadcast, subscribe, blob fetch).
Apps do not see iroh directly. Transport-implementation changes are
not ABI changes for apps.

### 11.2 Topic membership

Apps subscribe to topics. A topic is a content-addressed identifier;
exact formula at §4.6. Membership in a topic = the peer is gossiping
events on that topic.

**Membership tracking** (v1): membership is implicit via subscription.
The kernel does not maintain a global membership roster — peers who
subscribe receive gossip; peers who unsubscribe stop receiving. Apps
that need explicit membership tracking (presence, online indicators)
implement it via state-apply events (e.g. `Join` / `Leave` events
materialized into `members` derived state).

**No anonymous-stranger gossip**: a peer cannot publish events to a
topic without first being granted topic-write permission via the
app's authority model (typically a permission module like
`myrhiza-permission-rbac`). Bandwidth cost of accepting gossip from
non-members is mitigated by the participation primitive (§12.5).

### 11.3 Sync protocol

`HeadsSummary` delta exchange, per §4.2. Future work:
`HistorySyncComplete` EOSE-style signal so peers know when backfill
finished (Willow precedent); negentropy-shape range reconciliation
for very large topics (deferred).

### 11.4 Relays

Relays are dumb topic bridges. They do not inspect payloads, do not
materialize state, do not run WASM. Their role is to bridge browser
peers (TCP/WebSocket) with iroh-native peers (QUIC), and to provide
NAT-traversal hole-punching assistance.

A peer that wants to act as a relay for an app must be granted
explicit permission (via the app's authority model — usually a
sync-provider-shape grant from a `myrhiza-permission-*` module).
Without permission, the peer functions as a regular peer, not a relay.

**Metadata correlation risk** (accepted): relays see traffic
patterns — which topic IDs subscribers join, message frequency,
participant count. The spec does NOT claim relays are trustless;
it claims relays do not see *payload contents* (encrypted with
group keys) and do not see *event semantics* (treated as opaque
gossip). For threat models requiring metadata privacy, relays must
be trusted operators or apps must implement traffic shaping (cover
traffic, padding) at the application layer. v1 does not budget
cover-traffic infrastructure.

**Censorship**: a malicious relay can selectively drop messages,
delaying convergence. Mitigation: peers route through multiple
relays when available; persistent message drops surface as
HeadsSummary divergence. Future-direction: relay-rotation policies.

### 11.5 Topic-ID rotation through dumb relays

Apps that rotate topic IDs (e.g. for unlinkability via Willow's
epoch-key-rotation pattern) face a coordination problem: how do
existing members tell the relay where the next topic lives without
publishing the next topic ID on a public channel.

The kernel is not in this loop. Apps coordinate rotation through
in-band events on the existing topic before rotation. The exact
protocol is deferred to the relay-and-rotation child spec.

### 11.6 Browser peer connectivity

Browser peers connect via iroh-relay-bridged QUIC. Pure-browser
WebTransport-as-iroh-transport is not a current path (it would defeat
dial-by-pubkey identity). WebRTC is a Holochain-tx5-shape detour we
do not pursue.

## 12. Maintenance and participation

### 12.1 No worker class

There is no "worker" as a peer-class. Every peer participating in an
app can perform maintenance work for that app — that is what
participation means. Some peers contribute more (operator-deployed
infrastructure, dedicated archival peers); some contribute less
(mobile clients on metered connections); most do the default amount
automatically.

### 12.2 Maintenance modules

Maintenance work is encapsulated in **maintenance-shaped modules**
(WASM components in the module ecosystem). Common shapes:

- Persister (durable storage of event log).
- Snapshot provider (cached materialization for fast bootstrap).
- Sync provider (serves events to peers behind on heads).
- Replay buffer (recent-events cache for fast catch-up).

Maintenance modules use the standard module-ecosystem distribution +
signing + capability gating mechanism (§10, §7).

### 12.3 Default client behavior

Peers participating in an app automatically instantiate cheap
maintenance modules for that app (sync provider, replay buffer
scoped small). Expensive modules (full archival persister,
dedicated relay) are gated by per-app user UI: "How much do you
want to contribute to this app?"

### 12.4 Operator-deployed infrastructure

An operator may run a peer configured with all maintenance modules
instantiated. This peer is not architecturally distinct from a user
peer — it is a peer that opted into more modules. It must be
invited into the social graph of any app it serves (see §12.5);
without invitation, the participation primitive may refuse to
route work to it.

This preserves the Willow pattern (deployed relay / replay /
storage workers) as one valid deployment shape, but not the only
one.

### 12.5 Sybil-resistant participation

The primary direction is **social-graph Sybil resistance**:
leverage apps' existing permission/invite trust graphs. A peer
contributing maintenance work to an app must be inside that app's
trust graph; fake identities not invited cannot inject themselves.

The participation primitive is itself a module:

- `myrhiza-participation-social-graph` (primary direction)
- `myrhiza-participation-tit-for-tat` (bandwidth-bound roles)
- Other variants as warranted

Apps choose modules based on threat model. Apps without a membership
model (anonymous bulletin boards) use alternate modules or accept
non-Sybil-resistant participation.

### 12.6 Anonymous participation

Excluded by social-graph approach. Apps that need anonymous
contributors use different modules (tit-for-tat for bandwidth
reciprocity; storage proofs for high-stakes durable data) or
accept the threat-model implications.

### 12.7 Future research direction

Master spec acknowledges these as named-but-deferred:

- What maintenance modules ship as official `myrhiza-*` modules first?
- Default-instantiation heuristic for cheap-vs-expensive triage.
- Capability advertisement (peer signals "willing/able to host module
  X") — operator-config at v1; in-band gossip future.
- Resource limit defaults (fuel + memory per maintenance module
  instance).
- Fair-share scheduling between topics on a single peer.
- Reputation aggregation as overlay on social-graph.
- Bridge between operator-deployed-infrastructure and social-graph
  invitation discipline.

v1 ships zero maintenance modules. The framework is named; the
implementation lands when the first scaling demand emerges.

## 13. UI: framework, not app substrate

PR #636 framed the UI surface as "another app." Master spec adopts
the framing with explicit honesty about its caveats.

### 13.1 UI as app

The default UI app is shipped in-tree (initially `myrhiza-ui-leptos`
for Leptos-based browser/native UI). It exports `ui:*` interfaces
(panel, list, message, form, menu, button, input, ...) that other
apps' interaction components import.

Other UI apps may be authored:

- `myrhiza-ui-tui` — terminal, ratatui rendering.
- `myrhiza-ui-mcp` — agent host, structured-data rendering for an
  LLM.
- `myrhiza-ui-mobile-native` — Compose/SwiftUI shell, future.
- `myrhiza-ui-dioxus` — when Dioxus Blitz matures.

### 13.2 UI app capability surface

A UI app must bind a broad capability surface (DOM, focus, IME,
clipboard, file pickers, navigation, viewport, push, IndexedDB,
service workers, drag-and-drop on web; equivalent on native). The
master spec acknowledges:

- The default UI app is privileged. It is in the TCB for its own
  chrome and DOM.
- The default UI app is **not** in the TCB for arbitrary callers'
  intents — but only because per-call gating (§7.3) protects against
  caller social engineering at the **kernel** boundary, NOT inside
  the UI app's render path.

The "UI is just another app" framing is honest only when the UI
app's privilege is bounded by the runtime AND specific privileged
operations bypass the UI app entirely.

### 13.2.1 Kernel-controlled UI surface

For high-value-op approval prompts (clipboard write, file picker,
top-level navigation, push registration, AEAD seal/open with
sensitive keys, HTTP egress with origin filter), the **kernel renders
the prompt directly**, not via the UI app. This is required because:

- The UI app cannot be trusted to faithfully render a prompt for
  privileged operations. A compromised UI app could fake an approval.
- Per-call gating's defense in depth requires that the user response
  is genuinely from the user, not synthesized by the UI app.

**v1 kernel-controlled surface implementations**:

- **Native**: kernel renders prompts via OS-native modal dialog
  primitives (Cocoa, GTK, WinUI). The UI app cannot draw over OS-
  native modals. Engineering effort: per-platform; budget for v1.
- **Browser**: the **UI app itself runs in a sandboxed iframe whose
  parent is the kernel-controlled origin** (NOT the other way around).
  The kernel renders chrome (toolbar, install prompts, high-value-op
  approvals) in the parent context; the UI app inhabits the child
  iframe with `sandbox="allow-scripts"` (no `allow-same-origin`,
  no `allow-top-navigation`, no `allow-popups`). The UI app cannot
  reach the parent's DOM, cannot postMessage into kernel-controlled
  surfaces unless the parent explicitly opens a postMessage channel,
  cannot manipulate z-index of parent's chrome.

  **Why parent = kernel, not child**: z-index alone is not a security
  boundary. A child iframe is an OS-enforced isolation: scripts in
  the child cannot reach the parent's window object, cannot fake
  approval clicks for parent-rendered controls, cannot adjust their
  own z-index above the parent. This is the standard pattern used
  by browser extensions for protected UI; we adopt it.

  Concrete: kernel ships as an HTTPS-served origin (e.g.
  `https://kernel.localhost`). The UI app loads as
  `<iframe sandbox="allow-scripts" src="https://app-{hash}.kernel.localhost">`.
  High-value-op approval prompts render in the parent context; the
  iframe cannot draw over them.

The kernel-controlled surface is **kernel TCB**, not part of any UI
app. App authors do not customize it; the kernel ships a fixed
prompt format with the visual hash icon (§10.5) for author identity.

**`host.user-prompt(prompt) -> response`** for non-privileged intent
prompts MAY use the UI app's surface. The UI app is in the TCB for
those prompts; the kernel doesn't enforce kernel-rendered chrome
for non-privileged prompts (the cost would be prohibitive for normal
UX flows like "Are you sure you want to send this message?").

### 13.3 Custom-pixel surfaces

Whiteboards, code editors, network-graph visualizers, 3D voice
rooms, custom physics — these need custom-pixel control beyond
what `ui:*` interfaces express. Solution:

- On web: sandboxed iframe with postMessage protocol (kernel-mediated
  capability).
- On native: platform-specific equivalent.
- On TUI / MCP: rendered as "unavailable on this surface."

The escape hatch is web-shaped on purpose. GPU-driven UI substrates
(e.g. Bevy as a surface plugin once its web tooling matures) compose
here, not as replacements for the default UI app.

### 13.4 ui:* WIT contract

The `ui:*` interfaces are an interaction contract for app-to-UI
integration, not a portable UI substrate. They define how an
interaction component declares the views it wants rendered, the
commands it accepts, and the contextual integration points it offers
— not how those views are painted.

Each UI app implements the contract in its own idiom. Reusing one
set of interaction components across UIs is the goal; UI apps that
do not export an interface (e.g. a TUI without `ui:rich-card`) cause
graceful degradation, not breakage.

The exact `ui:*` contract is a child-spec concern.

## 14. Browser and native: dual-stack at v1

### 14.1 Decision

Both Wasmtime native and jco browser ship at v1. Kernel internals
abstract over a stable internal trait that both backends satisfy.

### 14.2 Wasmtime native

Default backend on macOS / Linux / Windows. Mobile (iOS / Android)
uses Wasmtime's AOT path (cranelift + winch baseline compiler) since
iOS prohibits JIT.

WebView desktop wrappers (Tauri, Wails) embed the native kernel
binary, not the browser kernel. This preserves native iroh transport
in the desktop app.

**Wasmtime version pin**: v1 commits to **Wasmtime LTS** (the next
LTS release available at v1 ship time, expected to be v48 at end-of-
2026 per Wasmtime's 12-month LTS cadence). **Bumping Wasmtime LTS
is a kernel MAJOR version bump**, not minor — fuel-cost-table
shifts between Wasmtime majors are convergence-breaking per §10.2's
ABI versioning rule (deterministic-helper additions are major;
fuel-cost recalibration falls in the same convergence-breaking
class). LTS is mandatory because:

- **Cross-peer fuel determinism requires identical fuel-cost tables.**
  Cranelift's per-instruction fuel costs may shift between Wasmtime
  majors (`prior-art/wasm-component-model/wasmtime.md`). Two peers
  on different Wasmtime versions can produce different fuel exhaustion
  outcomes for the same event, causing convergence divergence at the
  fuel boundary.
- **LTS provides 12+ months of stability** before forced bump, matching
  Myrhiza's release cadence.
- **Bumping Wasmtime LTS major is a kernel minor version bump and an
  ABI advisory** to app authors. Apps may need to re-compile against
  the new fuel cost table.

Mid-cadence Wasmtime majors (non-LTS) MAY be supported by the kernel
build but are not the canonical fuel-determinism reference. Operators
running mixed Wasmtime versions accept the convergence-divergence
risk; the canonical reference is LTS.

**Apps cannot interoperate across kernel-major boundaries.** App
`manifest.toml` declares `kernel-major`; topic IDs include the
kernel-major in the `app_bundle_hash` derivation, so peers running
different kernel-majors cannot subscribe to the same topic.
Kernel-major-bump rollouts therefore split the network — apps must
re-publish with the new kernel-major, and users must update kernels
before re-joining.

### 14.3 jco browser

Browser path. jco preview2 is the v1 target; preview3 when stable
migrates in-place (no API churn for app authors).

Constraints:

- Sync ABI only at preview2. Submit-and-poll (§8.5) is the workaround.
- ~350KB JS shim floor accepted as the cost of browser parity.
- Browser peers use iroh-relay-bridged QUIC for connectivity.

### 14.4 Why dual-stack at v1

- Browser is the project's pitch surface. Native-only-v1 undersells.
- "v1.5 fast-follow" framing for browser risks indefinite slip.
- Architecture pressure on backend abstraction is healthy from day
  one (avoids painful retrofit).
- Willow refactor onto Myrhiza targets v1; Willow is browser-shipped.

### 14.5 Native ≠ trusted-Rust apps

Critical clarification: "native" means the kernel runs as a native
Rust binary. Inside that kernel, **apps still run as WASM components
via Wasmtime**. The sandbox model requires WASM execution on every
backend. Compiling apps to native code for performance is explicitly
rejected — the only way to guarantee "WASM code can never access
more than what it's granted" is to run everything through the WASM
execution environment.

**Performance trade accepted, honestly**:

- **Steady-state straight-line numeric code**: ~2–5% Wasmtime overhead
  vs native code. The headline figure.
- **Hot-path state-apply with frequent host-import crossings** (sig
  verify, hash, payload-MAC verify): ~5–15% overhead. Host-call ABI
  translation costs dominate over WASM execution costs.
- **Cold component instantiation**: ms-class on Wasmtime, higher on
  jco. Aggressive caching (`Engine::precompile_component` +
  `InstancePre` reuse) is required; without it, per-event instantiation
  cost dominates everything else.

Sandbox is non-negotiable; this is the cost of the security model.
v1 commits to measuring overhead during MVP development and
documenting actual figures (rather than relying on the headline ~2-5%).

## 15. MVP

### 15.1 Acceptance criteria (lifted from PR #636)

**v1 acceptance** must demonstrate criteria 1-5:

1. The kernel loads and instantiates a WASM state component from a
   bundle fetched via iroh-blobs.
2. The component applies events deterministically; multiple peers
   running the same component bytes converge to the same state hash
   (verified via `state-digest`). Convergence is guaranteed only
   among non-equivocating authors per §4.4.1.
3. A UI app loads an interaction component for that state, projects
   a view, submits a command, observes the resulting state change.
4. A second app instance (different state component, different
   topic) coexists on the same peer; events do not cross.
5. Capability declarations actually gate access — a component cannot
   import an interface its manifest does not declare.

**v1.1 acceptance** adds criterion 6:

6. A behavior component runs on a designated peer, observes events,
   and logs them.

The behavior profile + criterion #6 ship as v1.1 stretch goal — they
are not v1-blocking. Counter app's auto-reset-at-midnight behavior
component is the v1.1 demo target. v1 ships criteria 1-5 as the
acceptance bar.

### 15.2 MVP shape: counter + poll

Two minimal apps coexisting in the same kernel.

**Counter app**:
- State: `{ value: u64 }`
- Events: `Increment(by: i32)`, `Decrement(by: i32)`, `Reset`
- Permission gate on `Reset` (admin only).
- v1.1 behavior component: `auto-reset-at-midnight` running on a
  designated peer (acceptance criterion #6; v1.1 stretch per §15.1).

**Poll app**:
- State: `{ options: Vec<String>, votes: Map<peer, option_index>,
  ended: bool }`
- Events: `CreatePoll(options)`, `Vote(option_index)`,
  `EndPoll(creator-only)`
- Permission gate on `EndPoll` (only poll creator).

Both apps live in `examples/counter/` and `examples/poll/`. Each ~50–
150 LOC state-apply + ~100–200 LOC interaction. Total ~300–700 LOC
across both apps + manifests.

### 15.3 Test infrastructure

Multi-tier test hierarchy lifted from Willow:

- **State tier** (instant): unit-test each app's state-apply directly
  with crafted events. No kernel, no I/O.
- **Kernel tier** (fast): kernel + MemNetwork + apps in-process.
  Verifies per-app namespace, convergence, capability gating.
- **E2E tier** (slow): real iroh transport, two peer processes on
  loopback or two machines.
- **Browser tier** (slow): jco-shimmed kernel, headless Firefox,
  multi-tab convergence.

Test files live in `tests/{unit, integration, e2e}/`. The
`coexistence.rs` e2e test is the load-bearing acceptance test —
both apps in same kernel, no event-crossing, capability gating
verified.

### 15.4 Workspace shape

```
myrhiza/
├── Cargo.toml                 workspace root
├── crates/
│   ├── kernel/                runtime (host)
│   ├── sdk/                   app-author surface (state-apply / propose
│   │                          / interaction macros, manifest tools)
│   ├── network/               iroh wrappers, network trait
│   ├── storage/               event log, snapshot cache
│   ├── crypto/                primitive crypto host imports
│   └── ...
├── examples/
│   ├── counter/               wasm32 component, depends on sdk
│   │   ├── Cargo.toml
│   │   ├── manifest.toml
│   │   └── src/{state, propose, interaction, behavior}.rs
│   └── poll/                  same shape
└── tests/
    ├── unit/                  per-crate
    ├── integration/           kernel-with-MemNetwork, single peer
    └── e2e/
        ├── counter.rs
        ├── poll.rs
        ├── coexistence.rs     ⭐ load-bearing acceptance test (file extension is .rs not .cs)
        ├── multi_peer_convergence.rs
        └── capability_gating.rs
```

**Dependency direction** (load-bearing constraint): `examples/` →
`crates/sdk`. Kernel crates **never** depend on examples. Examples
never appear in `crates/`. Violation = bug.

### 15.5 Estimated v1 scope

**Honest range**: 24-32 weeks engineering effort for dual-stack v1
(both Wasmtime and jco backends, full capability gating, all v1-
mandatory list items, MVP apps, complete test tier). 16-20 weeks is
plausible only with 2-3 senior engineers full-time AND no major
surprises on the browser path.

**Critical-path items in dependency order**:

1. Workspace + core types — ~1 wk
2. State-digest format pin + WIT package authoring — ~2 wk
3. Manifest schema implementation + capability vocabulary — ~2-3 wk
4. Wasmtime backend with capability-gated linker (designed in from
   start, not retrofitted) — ~3-4 wk
5. Backend trait abstraction (Wasmtime impl satisfying it; jco impl
   designed in but not implemented yet) — ~1 wk
6. State-apply ABI + helper set + fuel + float-ban lint — ~3-4 wk
7. Per-call gating + manifest intersection — ~2 wk
8. Event/DAG primitives + topo-sort + PendingBuffer — ~2 wk
9. iroh integration + MemNetwork double — ~2 wk
10. HeadsSummary sync + drift detection — ~2 wk
11. Crypto primitives (host imports backed by Rust crypto crates) — ~1-2 wk
12. Bundle distribution + signing + revocation topic — ~2-3 wk
13. Counter + poll example apps + state-tier tests — ~1-2 wk
14. Kernel-tier tests (incl. coexistence) — ~2 wk
15. SDK macros + tooling — ~3-4 wk
16. **jco backend implementation** (against existing trait) — ~4-6 wk
17. Browser-tier tests — ~2-3 wk
18. v1.1: behavior profile + acceptance criterion #6 — ~1-2 wk

**Sum**: 33-46 weeks if all items run sequentially. With parallelism
(SDK macros, e2e tests, jco backend can overlap with later kernel
work), realistic: **24-32 weeks**.

(Note: the §20 implementation outline lists 24 numbered items at a
finer granularity than this 18-item critical-path list — §20 is the
detailed engineering plan; §15.5 is the schedule rollup. The numbers
are not contradictory; §20 splits some items here into multiple
engineering steps for sequencing clarity.)

**v1 reduced-scope fallback**: if 24-32 weeks proves untenable, the
following cuts preserve architectural commitments while shrinking
schedule:

- **Defer jco backend to v1.5** (~4-6 wk savings). Risk: v1.5 slip
  pushes browser support out indefinitely. Spec mitigation: lock v1.5
  jco backend to a calendar deadline (e.g. "ships within 8 weeks of
  v1") with explicit ownership.
- **Defer behavior profile + criterion #6 to v1.1** (~1-2 wk savings).
  Already named as v1.1 candidate.
- **Defer per-call gating to v1.1** (~2 wk savings). Manifest
  intersection (§7.2) and resource handles (§7.4) preserved; per-call
  gates added later. Risk: gap window during which clipboard, file
  picker, etc. operate at module boundary, not per-call.

**Decision criteria for cutting**: if at week 16 the critical path
has slipped >2 weeks AND the browser backend is not at integration-
test stage, defer to v1.5 path. Otherwise hold dual-stack at v1.

**Out of v1 by design**: maintenance modules (zero ship); MLS module;
multi-device flow; scaling solutions; topic-ID rotation through dumb
relays; cross-app authority composition; bundle revocation distribution
beyond the per-author topic mechanism in §10.7.

## 16. Migration: Willow → Myrhiza

Willow continues to develop independently. Eventually, Willow refactors
onto Myrhiza — chat becomes one app among many on the runtime, the
Leptos web client becomes a `myrhiza-ui-leptos` instance, and Willow's
worker binaries (replay, storage, relay) become maintenance modules.

The migration is not a fork. Willow's existing codebase ships chat to
users; Myrhiza is a separate runtime project. Decisions in Willow
inform Myrhiza (the prior-art folder `prior-art/willow/` captures the
mapping); Myrhiza decisions are made fresh, re-evaluating each Willow
choice rather than blindly inheriting.

When Willow refactors onto Myrhiza, the chat product becomes
`willow-chat` — a Myrhiza app. Its state-apply contains the chat
semantics that today live in `willow-state`. Its interaction component
consumes the `myrhiza-ui-leptos` UI app. Its identity, encryption, and
permission concerns use Myrhiza primitives + modules
(IdentityScope, `myrhiza-permission-governance`,
`myrhiza-crypto-channel-key` or future `myrhiza-crypto-mls`).

**Architectural pieces enabling mechanical migration**:

- **Event-log shape (§4)** matches Willow's per-author Merkle DAG
  almost 1:1. Willow's `EventDag`, `materialize`, `HeadsSummary`,
  `PendingBuffer` map directly to Myrhiza primitives. Willow's existing
  event log is replayable through a chat-shaped `state-apply` WASM
  component. The `EventKind` enum (Willow's hard-coded chat
  semantics) becomes the chat-app's `state-apply` payload variant —
  no kernel work required.
- **Identity (§6)**: Willow's Ed25519 user keys reuse as
  `IdentityScope.long-term`. Existing chat servers become app
  instances; existing channel topic IDs translate via §4.6 formula
  with the chat-app's bundle hash + an instance seed derived from
  the existing server identity.
- **Permission model (§7)**: Willow's permission tiers (Owner,
  Admin, SyncProvider, etc.) become a `myrhiza-permission-governance`
  module that the chat app declares as a dep. Authority logic stays
  in app territory; the kernel hosts.
- **Encryption (§9)**: Willow's `seal_content` channel-key encryption
  becomes a `myrhiza-crypto-channel-key` module. Future MLS adoption
  is a module swap.
- **Browser parity (§14)**: dual-stack at v1 means Willow's existing
  Leptos web UI translates directly. The `myrhiza-ui-leptos` UI app
  is the Leptos client adapted to host other apps' interaction
  components.
- **Worker pattern (§12)**: Willow's `replay`, `storage`, `relay`
  binaries become maintenance modules. The deployment shape (operator-
  run peers configured with all maintenance modules) is preserved
  (§12.4).

**Migration timing**: target v1 (browser available from v1 ship).
Migration is *mechanical given the architecture above* — Willow's
team writes a chat-app bundle that uses Myrhiza primitives + modules,
then runs both Willow chat and Myrhiza chat side-by-side during the
cutover, then deprecates Willow chat.

**Migration mechanics specific to Willow** (event-log translation tool,
identity migration UX, channel-history-import flow) are a Willow-side
project planned separately when Willow is ready.

## 17. Future-direction items (named-but-deferred)

The master spec commits the *direction* on these items so v1 design
does not paint corners. Implementation lands in child specs when
demand emerges.

### Scaling
- Event-log replay scales linearly. Likely v2+ evolution: DHT-shape
  sharding layered on top. Other paths preserved (cooperative
  pinning, log-pruning, derived-state replication). Decision criteria:
  measure the bottleneck before committing.

### Distributed maintenance
- Default-instantiation heuristic for cheap-vs-expensive maintenance
  modules.
- Capability advertisement (peer signals "willing/able to host module
  X") — operator-config at v1; in-band gossip future.
- Resource limit defaults.
- Fair-share scheduling between topics on a single peer.
- Bridge between operator-deployed-infrastructure and social-graph
  invitation discipline.

### Identity
- Multi-device device-add/revoke flow.
- Recovery semantics (lost device).
- Cross-peer behavior continuity.
- Quantum-safe signature migration.

### Crypto
- `myrhiza-crypto-mls` module (when first MLS-needing app emerges).
- Other crypto modules (channel-key, double-ratchet, sealed-content).
- Quantum-safe primitives.

### Capability model
- High-value-op list for per-call gating.
- Cross-app authority composition (out of scope at v1).
- Capability vocabulary in manifest schema.

### Distribution
- Bundle revocation (author retracts bad version).
- In-band catalog gossip for app/module discovery.
- Supply-chain hardening (dependency review tooling).

### Networking
- Topic-ID rotation through dumb relays (relay-and-rotation child
  spec).
- `HistorySyncComplete` EOSE-style signal for backfill completion.
- Negentropy-shape range reconciliation for very large topics.

### Determinism
- Float opt-in path (manifest `state-apply.allow-floats = true`).
- Snapshot portability across component-version upgrades.
- Additional state-digest formats opt-in (bincode is pinned at v1;
  future opt-ins via manifest declaration).
- Pre-check fuel budget independence from apply.

### Interaction
- `ui:*` WIT contract details.
- Custom-pixel surface escape hatch on non-web platforms.
- Hot-reload (deferred to v2).

### Module ecosystem
- Versioning + semver discipline child spec (already content-hash
  pinned per §10.6, but version-display + compatibility checks).
- Bus-factor on official `myrhiza-*` modules.
- Module audit / curation policy.

### Prior art borrowed but not yet implemented

Patterns from `prior-art/` that the master spec acknowledges and
commits as future-direction; implementation lands in child specs or
module ecosystem.

- **Holochain source-chain semantics** (`prior-art/holochain/lessons.md`
  Borrow §1) — already aligned: per-author Merkle DAG IS source-chain
  shape. No future work needed; called out for clarity.
- **Holochain DHT op decomposition** (`prior-art/holochain/lessons.md`
  Borrow §2) — informs v2+ scaling direction (§4.5). Events
  decomposed into typed ops, sharded by neighborhood. v2 scaling
  child spec.
- **Holochain warrants** (`prior-art/holochain/lessons.md` Borrow §3) —
  signed attestations of bad-author behavior (equivocation, etc.). v2
  warrant-and-equivocation child spec. Surfaced in §4.4.1 future
  direction.
- **Holochain countersigning** (`prior-art/holochain/lessons.md`
  Borrow §4) — multi-author atomic events. Relevant to governance
  modules; deferred. Possible v2 `myrhiza-permission-countersign`
  module.
- **Holochain membrane proofs** (`prior-art/holochain/lessons.md`
  Borrow §6) — capability-bound app entry. Relevant to participation
  primitive; informs `myrhiza-permission-rbac` / `myrhiza-participation-*`
  module designs.
- **Croquet TUTTI snapshot-equality voting** (`prior-art/croquet/lessons.md`
  Borrow §"Snapshot-equality voting") — ratified in §4.7 (cross-peer
  drift detection). Implementation lands at v1.
- **Agoric `baggage` upgrade convention** (`prior-art/agoric-endo/lessons.md`
  Borrow §"`baggage` upgrade convention") — durable component-state
  bridge across upgrades. Informs snapshot portability child spec.
- **Agoric `bringOutYourDead` distributed GC** (`prior-art/agoric-endo/lessons.md`
  Borrow §"`bringOutYourDead`") — long-lived peer-as-infra needs GC
  of stale state (event log, snapshot cache, per-component KV).
  Future-direction for distributed-maintenance child spec.
- **Willow `timestamp_hint_ms` split-semantics review-trap**
  (`prior-art/willow/lessons.md` Avoid) — Willow signs HLC into events
  but doesn't use it for ordering, only materialized-state. Myrhiza
  inherits this exactly (§4.1). Pick-a-side mitigation: master spec
  documents both uses explicitly (HLC IS extracted via
  `host.now-hlc-from-event` and IS materialized into derived state;
  HLC is NOT used for DAG topo-sort or merge). Reduces but does not
  eliminate the review-trap; add static-analysis tooling future
  direction.

## 18. Tradeoffs surfaced

| Decision | Runner-up | Why rejected |
|---|---|---|
| Event-log replay (paradigm) | Validating DHT (Holochain) | 6+ year unfinished sharding story; throws out Q3 progress |
| Full Component Model day-one | Extism v1 → CM v2 | Double-rewrite cost for app authors and Willow |
| Dual-stack v1 | Native-first + browser v1.5 | v1.5 slip risk; browser is project pitch |
| Module ecosystem (3-tier) | Kernel-baked features | Module-level evolution faster than kernel ABI; vendor lock-in avoided |
| Layered cap gating | Per-call only | Modules need containment; layered defense in depth |
| `IdentityScope` unified | Separate per-domain | Triple design + impl cost; PR #636 names structural similarity |
| MLS as module | Kernel-baked MLS | Vendor lock-in (one impl); kernel surface bloat; PQ migration kernel break |
| Ed25519 + iroh-blobs (P2P) | OCI + sigstore | Centralizes what we made P2P; Sigstore Public Good single point |
| Float ban in state-apply v1 | Spec-pinned floats | NaN canonicalization + SIMD divergence vectors; debugging painful |
| WASM on every backend | Native compilation for performance | Defeats sandbox model; capability discipline requires WASM execution |
| No worker class | Worker-as-peer-class | Architecturally honest; doesn't close paths; v1 ships zero |
| Counter + poll MVP | Single app or chat MVP | Coexistence (#4) requires two apps; chat recapitulates Willow |

## 19. Open questions / accepted risks

### Project-shape v2 (snapshot lifecycle, kernel skew, resource cleanup)

**Snapshot lifecycle at v1**: the kernel does NOT compute, store,
or distribute snapshots at v1. Bootstrap is full-event-log replay
from genesis. This is intentional simplification: v1 acceptance
criteria do not require snapshots, counter+poll have small enough
state that full replay is fast, and snapshot lifecycle (when create,
when evict, who provides, how to verify) is non-trivial. **Snapshot
support lands at v2** as a `myrhiza-state-snapshot-cache` module
(distinct from the kernel) that subscribes to events and provides
snapshots-on-request through the standard kernel host imports.
Module is opt-in per app.

**Wasmtime LTS kernel-version-skew**: when the kernel-signing-root
publishes a new kernel binary built against a newer Wasmtime LTS,
peers running the old kernel and peers running the new kernel may
disagree on fuel exhaustion outcomes. v1 mitigation: kernel announces
its Wasmtime fuel-table version in HeadsSummary; if peers detect
mismatch, the older peer surfaces a "kernel out of date; upgrade
recommended for convergence guarantee" warning. Active divergence
from kernel-version-skew is treated by drift detection (§4.7) as a
flagged event with a specific "kernel-version-skew" reason rather
than generic "convergence drift."

**Resource-handle cleanup discipline**: the kernel MUST revoke all
outstanding resource handles when a component instance terminates by
any path (normal exit, fuel exhaustion, trap, fatal error,
operator-initiated kill). Component instance restart yields a fresh
handle table; previously-issued handles are no longer valid. v1
implementation tests: handle-revocation-on-instance-termination
under representative termination scenarios. Failure to revoke is a
v1 audit blocker.

### Performance and correctness

- **MLS performance under WASM**: ~2-5x slower than native MLS for
  group operations. The 2-5x figure is steady-state, post-warmup.
  Cold instantiation overhead can be ms-class per call; aggressive
  instance caching is required. **Benchmark MLS-in-WASM at expected
  group sizes before committing the `myrhiza-crypto-mls` module**
  to canonical. Reopen the kernel-baked-MLS option if module path
  doesn't make budget.
- **Wasmtime overhead figure honesty**: §14.5 cites ~2-5% overhead
  for Wasmtime vs native code. This is the steady-state straight-
  line numeric figure. Hot-path state-apply with frequent host-import
  crossings (signature verify, hash, payload-MAC verify) sees higher
  overhead — host-call ABI translation costs dominate over WASM
  execution costs. Realistic figure: 2-15% depending on workload.
  Sandbox is non-negotiable; this is the cost.
- **Component instantiation overhead**: ~ms-class on Wasmtime; higher
  on jco. Aggressive caching of `Engine::precompile_component` +
  `InstancePre` reuse is required. v1 measurement on Safari iOS in
  particular is unverified; budget for surprises.
- **jco preview2 sync-only ABI**: submit-and-poll is the workaround;
  ergonomics are real. Preview3 async stabilization improves this
  but does not change v1. **Preview3 has been "almost ready" for
  ~3 years** per `prior-art/wasm-component-model/lessons.md`; treat
  the timing as uncertain.
- **Browser CM nested in browser-WASM (Leptos UI app loading nested
  CM components)**: not battle-tested at scale. Risk of weird bugs.
  Mitigation: counter + poll MVP exercises this path early; commit
  to early benchmarking on Safari iOS specifically.
- **Wasmtime version churn**: Cranelift fuel cost tables may shift
  between Wasmtime majors. Mitigated by Wasmtime LTS pin (§14.2).
  LTS bump is a kernel minor version and ABI advisory.

### Security

- **Author key compromise**: phishing-shape attack surface. Mitigated
  by user-visible bech32m author identity at install + revocation
  topic auto-subscription (§10.7) + visual hash icon (§10.5 step 6).
  Future direction: key transparency log + petname registry —
  deferred to identity-binding child spec.
- **Identity binding gap**: pubkey-as-identity is the v1 model. There
  is no notion of "this pubkey belongs to specific human X." Phishing-
  shape attacks rely on this gap; users must out-of-band verify
  unfamiliar author identities. Future direction: petnames, web-of-trust,
  key transparency. v1 documents the gap explicitly.
- **No sigstore transparency log**: trust comes from author identity +
  user judgment. Trade accepted; matches P2P framing. Pairs with
  identity binding gap as a known v1 limitation.
- **Side-channel resistance in deterministic helper set**: §5.1 mandates
  constant-time implementations with respect to secret inputs. v1
  audit obligation: kernel implementations of `host.verify-signature`,
  `host.verify-payload-mac`, `host.aead-{seal,open}`, `host.x25519-ecdh`
  use constant-time crypto crates (ed25519-dalek, x25519-dalek,
  chacha20poly1305 in Rust have constant-time implementations).
- **DoS in helper set**: `host.hash` and `host.verify-signature` consume
  host CPU disproportionately to WASM instruction cost. A malicious
  app could call them with large payloads to drain fuel asymmetrically.
  Mitigation (deferred to fuel-cost-table child spec): per-host-call
  fuel costs proportional to wall-clock cost, not WASM-instruction
  count.
- **Pre-check fuel exhaustion as soft DoS**: pre-check shares apply's
  per-event fuel budget. A malicious event with deep validation logic
  can consume budget that downstream peers also pay. Open question:
  separate per-event fuel for pre-check vs apply. Deferred but flagged.
- **jco shim in browser TCB**: jco's preview2 transpiler generates
  the JS that bridges WASM components to browser APIs. A jco
  resource-handle-lifecycle bug could leak handles across components.
  v1 commitment: pin a specific jco version per kernel release with
  deterministic build verification; jco upgrades are kernel ABI
  advisories.
- **Snapshot security at bootstrap**: snapshots fetched from peers
  at bootstrap are not authoritative — kernel re-validates by replaying
  the event log up to the snapshot's anchor hash on first install.
  Snapshots remain useful as bootstrap optimization (skip slow
  storage I/O) but are never trusted for state contents.
- **Operator-deployed infrastructure + invitation flow**: operators
  needing to host many apps face a tension with social-graph Sybil
  resistance — they cannot be invited to every customer's social
  graph. v1 acknowledged limitation; future direction: capability
  attestation patterns (operator publishes "I run maintenance for
  these app shapes" attestation; apps opt to accept attestation-
  based participation in lieu of invitation).
- **Manifest TOCTOU**: capability declaration parsed at install,
  intersected at instantiation. Bundle update flow MUST re-run install
  (per §10.5 step 7). Silent in-place bundle update is forbidden by
  spec.
- **Replay attack on submit-and-poll completion handlers**: kernel
  enforces that only kernel-issued tokens can re-enter components
  via `on-completion`. Tokens are unforgeable (kernel-side opaque
  HMAC). v1 implementation MUST verify token before dispatching
  completion.
- **Capability summary fatigue**: MetaMask Snaps lesson — users
  habituate to permission prompts. Mitigations in §10.5: 2-second
  minimum render time on high-value-op approval; visual hash icons;
  highlighted "first time installing from this author" markers.
  Insufficient long-term; future direction: trust-rating heuristics.

### Ecosystem

- **Anonymous participation excluded by social-graph Sybil resistance**:
  documented; apps that need anonymity use other modules (tit-for-tat,
  storage proofs).
- **Module ecosystem bus factor**: official `myrhiza-*` modules we
  author, we maintain. Mitigated: encourage third-party alternatives;
  module ecosystem stays open even when official modules ship.
- **Browser peer story is relay-bound**: WebTransport-as-iroh-transport
  not a current path; WebRTC not pursued. Browser peers depend on iroh
  relays. Mitigation: relay-bridged QUIC is the only shipped path; v1
  does not pretend NAT-traversal works in pure browser.
- **iroh pre-1.0 churn**: iroh is currently 1.0-rc; API has been
  volatile (`prior-art/iroh/lessons.md` flags constant pre-1.0 API
  churn). v1 pins iroh to a specific version (TBD at implementation
  start); upgrade pain is budgeted explicitly.
- **Cherry-picked precedents disclosure**: §1 cites Agoric and Willow
  as "production-validated" for event-log replay. Agoric is a
  blockchain (consensus-given ordering); Willow is at hundreds-of-
  users scale. Neither validates "event-log replay scales as P2P
  infrastructure for write-heavy public-read apps." See §4.5 scaling
  section for explicit ceiling acknowledgment.

### Determinism enforcement

- **Cremers ETK 2025 enforcement**: Ed25519 mandatory for IdentityScope
  long-term identity. Enforced **structurally** (not advisory) — the
  kernel does not expose any signing API that takes an algorithm
  parameter (`host.author-event` is always Ed25519). Manifest
  declaring non-Ed25519 is rejected at install.
- **Float-ban scope**: lint at WASM byte level; rejects modules
  containing `f32.*` / `f64.*` instructions in any function reachable
  from a state-apply export. Unreachable float ops in dead code are
  permitted (the linter follows reachability). Cargo-component build
  recipe for state-apply components includes `RUSTFLAGS="-Cno-float"`
  as a safeguard.
- **Resource-handle persistence**: WCM resource handles are per-
  instance and non-durable in v1. Component restart loses handles;
  apps must re-acquire from kernel state. Future direction: Agoric
  `baggage`-style upgrade convention for handle persistence across
  upgrades.
- **Behavior identity continuity across peer failures**: the runtime
  does not migrate behavior keypairs between peers. Apps that need
  stable bot identity across peer failures register an in-band mapping
  event; SDK macros default to making this binding explicit (so
  app authors don't accidentally ship behaviors that lose identity
  on restart).

### Project-shape

- **Schedule risk**: 24-32 weeks honest range; 16-20 was optimistic.
  v1 scope-reduction fallback (§15.5) cuts jco / behavior / per-call
  gating to v1.5 if mid-project measurement shows slip. Fallback is
  preserved as recoverable.
- **Single architectural ancestor (Willow) at small scale**: Myrhiza
  inherits architectural lessons but cannot rely on Willow as
  validation at scale. v1 acceptance test (counter+poll) is a smoke
  test, not scale validation.
- **iroh strategy shift risk**: Number 0 has redirected before
  (relay infra ownership, ticket changes, FFI mothballing). v1
  pinning to a specific iroh version is the immediate mitigation;
  long-term mitigation requires kernel network-trait abstraction
  preserved as a design seam (planned in §20).
- **"Novel angle" precision**: §1 frames "peers as infrastructure"
  as novel. Holochain and Pears have framed this similarly. The
  actually-novel piece is the **combination**: WCM + capability
  discipline + no-CRDT-in-kernel + author-bounded-scale-at-v1. No
  single prior project has shipped this combination. Honest
  positioning: not a new pitch, a new combination.

## 20. Implementation outline (handed off to writing-plans)

Implementation plan lives at
`docs/plans/2026-05-09-myrhiza-master-design.md`. Critical path
(reordered per implementation feasibility review — manifest schema
+ capability gating + backend trait abstraction must be designed in
from the start, not retrofitted):

1. **Workspace scaffold** + initial crate structure (`kernel/`,
   `sdk/`, `network/`, `storage/`, `crypto/`, `examples/counter/`,
   `examples/poll/`, `tests/`).
2. **Core types**: `IdentityScope`, `IdentityHandle`, `EventHash`,
   `Event`, `Topic`, `BundleHash`, manifest schema types.
3. **State-digest format pin** (decision step): commit `bincode 1.3.x`
   with default config; sorted-collection discipline doc.
4. **WIT package authoring**: state-apply, state-propose, interaction,
   behavior worlds; canonical kernel host import surface (per §3.5).
5. **Manifest schema implementation + capability vocabulary**:
   TOML parser; capability vocabulary registry; v1-mandatory
   high-value-op list; signature verification.
6. **Wasmtime backend with capability-gated linker** (designed in
   from start, not retrofitted): component instantiation; per-call
   gate dispatch; manifest intersection at instantiation.
7. **Backend trait abstraction**: stable internal trait both Wasmtime
   and jco backends will satisfy. Wasmtime impl satisfies it; jco
   impl deferred to step 17.
8. **State-apply ABI** + deterministic helper set + fuel budget +
   float-ban byte-level lint.
9. **Pre-check unification + drift detection scaffold**: dry-run
   path for state-apply; periodic state-digest gossip stub.
10. **Event/DAG primitives**: per-author Merkle DAG storage;
    topo-sort with EventHash lex tie-break; PendingBuffer (1h TTL,
    10K entries, per-author /50 sub-cap).
11. **iroh integration**: gossip wrapper, blob fetch wrapper, network
    trait abstraction (production iroh + MemNetwork test double).
12. **HeadsSummary sync** + drift-detection digest gossip integration.
13. **Crypto primitives**: host imports backed by Rust crypto crates
    (ed25519-dalek RFC 8032 strict, x25519-dalek, chacha20poly1305,
    hkdf-sha256, blake3).
14. **Bundle distribution + signing**: Ed25519 over canonical
    manifest+content+version+pubkey encoding; iroh-blobs publication
    and fetch; revocation topic auto-subscribe per §10.7.
15. **Counter app**: state-apply, propose, interaction, manifest.
16. **Poll app**: same shape.
17. **State-tier tests** for both apps.
18. **Kernel-tier tests** (kernel + MemNetwork): convergence,
    capability gating, coexistence.
19. **E2E test suite**: counter, poll, coexistence, multi-peer
    convergence, capability gating.
20. **SDK ergonomics**: macros and tooling for app authors;
    cargo-component integration; manifest helpers.
21. **jco backend implementation**: generate JS+wasm shim; iroh-relay
    bridge for browser transport. Implements the trait from step 7.
22. **Browser-tier tests**: headless Firefox, multi-tab convergence,
    nested-CM-in-browser-WASM viability under realistic memory
    pressure.
23. **v1.1 (or v1 stretch goal)**: counter app's auto-reset behavior
    component (acceptance criterion #6).
24. **Dependency-direction CI check**: enforce `examples/` →
    `crates/sdk` only; kernel crates never depend on examples.

**Order rationale**: steps 3-7 establish the runtime ABI and the
backend abstraction *before* deep kernel work. Step 5 (manifest)
comes before step 6 (Wasmtime) so the linker is built with capability
gating from the start, not retrofitted in step 9 of the original
ordering. Step 17 (jco backend) implements the trait designed in
step 7.

## 21. Sources

- All 12 prior-art folders under `docs/prior-art/`:
  agoric-endo, crdts, croquet, holochain, iroh, mls, pears, spin,
  spritely-ocapn, wasmcloud, wasm-component-model, willow.
- `docs/references/local-first.md` — anchor index of papers and
  talks.
- `docs/reports/2026-05-09-myrhiza-design-space/` — preparation
  phase exploration:
  - `README.md` (master question list)
  - `convergence-and-determinism.md`
  - `wasm-and-abi.md`
  - `identity-crypto-caps.md`
  - `networking-sync-maintenance.md`
  - `ui-distribution-mvp.md`
  - `brainstorming-decisions.md` — running decision log
- Willow PR #636 (master runtime spec, draft):
  [github.com/intendednull/willow/pull/636](https://github.com/intendednull/willow/pull/636)
- Willow repo: [github.com/intendednull/willow](https://github.com/intendednull/willow)
- `CLAUDE.md` — Myrhiza dev guide (locked decisions inventory).
