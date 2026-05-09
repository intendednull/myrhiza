**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — determinism: how convergence is achieved today and what PR #636 commits

`willow-state` is deterministic by construction: pure Rust, zero I/O,
zero networking. Cross-peer convergence is "DAG topology + lex hash
tiebreak + pure apply." PR #636 commits to extending the same property
across the WASM boundary by binding `state-apply` only to a
deterministic helper set.

See also: [state-machine.md](state-machine.md),
[authority.md](authority.md), [glossary.md](glossary.md),
[README.md](README.md).

## Today: how convergence is achieved

Three independent legs:

### 1. The DAG is content-addressed and structurally validated

Every event's identity is the SHA-256 of its signable content
(`crates/state/src/event.rs:511-546`). Tampering with any field
invalidates the hash. `EventDag::insert` enforces strict per-author
chain integrity (`dag.rs:130-218`) — `seq` must be `latest_seq + 1`,
`prev` must equal current head, signature must verify. Identical event
sets across peers therefore produce identical `EventDag` contents.

### 2. Topological sort is deterministic

`EventDag::topological_sort` (`dag.rs:362-418`) uses Kahn's algorithm
with a `BTreeSet<&EventHash>` for the ready set. Because `EventHash`
implements `Ord` as lex byte comparison (`hash.rs`,
`per-author-merkle-dag-state-design.md` §"EventHash"), concurrent
events are always tie-broken in the same order on every peer.

**Note: tie-breaking is by `EventHash`, *not* by HLC.**
`timestamp_hint_ms` is signed (it is part of the signable content) but
plays no role in ordering or merge logic. The 2026-04-01 design spec
calls this out explicitly: "Used ONLY for display purposes (e.g.,
showing '2 hours ago' in the UI). Never used for ordering, merge, or
any state logic" (`event.rs:496-498`, design spec lines 174-178). The
contrast with the previous chain-based design's `timestamp_ms`-driven
merge — which "depended on wall-clock time" and was a contradiction
the new design exists to fix — is also explicit (design spec §1).

### 3. `apply_event` is pure Rust with no non-deterministic inputs

The crate-level docstring is unambiguous: "All state is derived from a
per-author Merkle-DAG of signed events via the `materialize` function.
This crate has zero I/O, zero networking — just DAG operations and
deterministic state projection." (`crates/state/src/lib.rs:3-6`).

`apply_event` (`materialize.rs:161`) calls into:
- `check_permission` (pure read-only over `ServerState`).
- `apply_mutation` (pure mutation by match on `EventKind`).
- `check_and_apply_proposal` (pure threshold logic).

None of these consult a clock, a random source, the network, or the
filesystem. The only "external" inputs are the event itself and the
prior `ServerState`.

### Idempotency

`apply_incremental` deduplicates via `state.applied_events`
(`materialize.rs:92-102`). Replaying the same event twice yields
`AlreadyApplied`. The full materialize path also inserts each event
into `applied_events` before applying (`materialize.rs:76`), so
incremental and full materialization produce equivalent state.

## Today: HLC's actual role

Despite the spec's framing, the HLC (`crates/messaging/src/hlc.rs`,
570 lines) is **not** part of the state-crate's determinism story
today. It is used at the messaging layer for ordering display-relative
operations and for the `last_activity_hlc` field on channels. In
`willow-state`, HLC values arrive only as `event.timestamp_hint_ms`
and are stored verbatim into derived fields (`materialize.rs:513,
526, 587, 604, 880`) — never compared, never used for ordering.

The HLC's monotonicity guarantee (`hlc.rs:55-67`,
`MAX_FORWARD_DRIFT_MS` clamp at line ~80) protects per-peer monotone
ordering for the messaging crate; the state crate's correctness does
not depend on it.

## Today: idempotency hazards (load-bearing details)

A few details are easy to miss but load-bearing for determinism:

- **`ServerState` uses `BTreeMap` / `BTreeSet` for everything that gets
  serialized** (`server.rs:33-103`). `HashMap` appears only on
  `message_index`, which is `#[serde(skip)]` and never iterated in apply
  paths — it is purely an O(1) lookup index for `EditMessage` /
  `DeleteMessage` / `Reaction`. Mutation order is therefore stable
  across peers.
- **`EventDag` uses `HashMap` internally** for `events`, `chains`,
  `heads` (`dag.rs:103-112`). This is safe because `topological_sort`
  re-orders into a `BTreeSet` before iteration; `HashMap`'s
  non-deterministic iteration order never reaches `apply_event`.
- **`Snapshot::new` sorts heads by author bytes** before computing the
  verification hash (`sync.rs:88-95`). Cross-process snapshot hash
  comparison therefore agrees.
- **`bincode` is the canonical serialization format.** Event hashes are
  `EventHash::from_bytes(&bincode::serialize(&signable_content))`
  (`event.rs:524-534`). Deterministic for owned `Vec` / `String` /
  integers / `BTreeMap`.

I checked for `HashMap` iteration in apply-path code — found none. The
state crate is clean on this axis. (PR #636 §"Determinism, in detail"
explicitly warns: "a hash of WASM linear memory … would diverge
trivially across peers due to allocator behavior, struct field
padding, or `HashMap` iteration order.")

## Aspirational (PR #636): determinism across the WASM boundary

PR #636 commits to a specific, bounded surface for state-`apply`:

> The rule is **`apply` may import only host functions whose output is
> a pure function of their inputs** — not "no imports."

The deterministic helper set bound to state-`apply`:

- `host.verify-signature(pubkey, msg, sig)` — Ed25519 verification.
- `host.verify-payload-mac(envelope, key-handle)` — authenticity check
  on a sealed payload, proving key-possession (not author identity).
- `host.hash(bytes)` — blake3 / sha256.
- `host.install-key(handle, sealed-distribution-blob) -> ()` — record
  that a key-distribution blob exists. Returns no value, so peer-local
  decryptability cannot leak into state-`apply`.
- `host.now-hlc-from-event(event)` — extract HLC bytes from the
  envelope (no wall clock).
- `host.log` — host-side logging only; no return value.

Explicitly denied to state-`apply`:

- Wall clock, randomness, network, filesystem, environment, threads.
- Non-deterministic floats (the WASM spec pins these, but PR #636
  recommends banning v1 anyway "to avoid review pain").
- A deterministic fuel budget terminates non-converging executions
  uniformly across peers.

**The determinism proof is therefore "by absence":** every host import
bound to `apply` is pure; there is no non-deterministic capability to
grant. State-`propose` is allowed `host.hlc`, `host.random`,
`host.seal` — but `propose` runs only on the originating peer, after
which all peers replay through `apply` (PR #636 §"Determinism, in
detail").

### state-digest, not memory-hash

PR #636 commits to apps exporting a canonical `state-digest()`
function whose bytes the kernel hashes for cross-peer convergence
checks:

> The kernel verifies cross-peer convergence by hashing a **canonical
> state digest** the app exports — *not* a hash of WASM linear memory,
> which would diverge trivially across peers …. Apps export a
> `state-digest()` function (or equivalent) that returns canonical
> bytes under a deterministic encoding (postcard with sorted
> collections is the existing-codebase precedent); the kernel hashes
> the result and gossips the hash.

**Caveat on PR #636's wording**: shipped Willow uses `bincode`, not
postcard (`event.rs:532`, `sync.rs:100-103`). The load-bearing piece
of the "existing-codebase precedent" is the **sorted-collection
discipline** (`BTreeMap` / `BTreeSet`), not the format. Postcard
substitution is straightforward — `postcard` is also size-deterministic
for the same primitive set — but treat PR #636's "postcard precedent"
phrasing as forward-looking, not a historical reference.

## What Myrhiza inherits

**Lifts directly**:

- **The deterministic helper set** as listed above. Bind nothing else
  to `state-apply`.
- **state-digest-not-memory-hash** for cross-peer convergence checks.
- **Postcard (or bincode) with `BTreeMap`/`BTreeSet`** as the canonical
  encoding precedent.
- **Topo-sort by Kahn's + lex `EventHash` tiebreak** as the deterministic
  ordering primitive at the kernel layer. Apps don't choose this.
- **Idempotent apply via `applied_events` dedup** as the contract apps
  must satisfy.
- **Pre-check runs under the same profile as apply** — same fuel,
  same denied imports — so the pre-check-equals-apply property holds
  across the WASM boundary.

**Verifies-and-confirms** (Willow is clean here, Myrhiza must stay clean):

- No `HashMap` iteration in apply paths. Willow's apply paths use
  `BTreeMap`/`BTreeSet` for serialized state and `Vec` for ordered
  collections; the only `HashMap` (`message_index`) is `#[serde(skip)]`
  and used only for keyed lookup. Myrhiza must enforce the same
  discipline at the WIT-bindings layer.

**Re-evaluates**:

- **HLC's role.** Today's Willow doesn't use HLC for ordering;
  PR #636 lists `host.now-hlc-from-event` as a deterministic helper
  that `apply` may consult. Myrhiza needs to decide: is HLC a kernel
  primitive every app sees, or one input among many that the kernel
  passes via the event payload? Picking the latter avoids a load-
  bearing helper that not every app needs.
- **Float ban.** PR #636 recommends but does not require banning
  floats in v1. Myrhiza should pick a side; "ban" is cheaper than
  "audit every component" later.

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)

## Sources

- `crates/state/src/lib.rs:3-6` — pure-no-I/O contract.
- `crates/state/src/event.rs:478-577` — content-hash + signature
  derivation.
- `crates/state/src/dag.rs:130-218, 362-418` — strict insert
  validation, deterministic topo-sort.
- `crates/state/src/materialize.rs:64-102, 161-202` — pure
  `materialize` / `apply_incremental`, idempotent dedup.
- `crates/state/src/sync.rs:82-105` — `Snapshot::new` canonical
  encoding (sorted heads + bincode).
- `crates/state/src/server.rs:33-103` — `ServerState` uses
  `BTreeMap`/`BTreeSet` for serialized fields; `message_index` is
  `#[serde(skip)]`-and-not-iterated.
- `crates/messaging/src/hlc.rs:1-80` — HLC algorithm and clamp;
  consumed by messaging, not state.
- `docs/specs/2026-04-01-per-author-merkle-dag-state-design.md`
  §"Design Goals" (no wall-clock dependence; zero I/O), §"EventHash"
  (lex `Ord` for tiebreaking).
- PR #636 `docs/specs/2026-04-27-willow-runtime/README.md`
  §"Determinism, in detail", §"Constraints we accept"
  (deterministic-by-construction).
