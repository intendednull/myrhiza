**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — state machine: per-author Merkle DAG, materialize, EventKind

`willow-state` is a pure, no-I/O Rust crate that derives a `ServerState`
from a per-author Merkle DAG of signed events. This file documents what
ships today vs. what PR #636 envisions, and which pieces Myrhiza inherits
unchanged vs. lifts into the per-app `state-apply` component.

See also: [authority.md](authority.md), [determinism.md](determinism.md),
[glossary.md](glossary.md), [README.md](README.md).

## Shipped today: the DAG

Each event (`crates/state/src/event.rs:478-498`) carries:

- `author` — Ed25519 public key.
- `seq` — per-author monotonic counter starting at 1.
- `prev` — hash of this author's previous event (`EventHash::ZERO` for `seq=1`).
- `deps: Vec<EventHash>` — cross-author causal heads "this event has seen,"
  capped at `MAX_EVENT_DEPS = 64` (`event.rs:22`).
- `kind: EventKind` — the payload (chat-specific today; see below).
- `sig` — Ed25519 signature over `(author, seq, prev, deps, kind, timestamp_hint_ms)`.
- `timestamp_hint_ms` — wall-clock hint. **Not used for DAG ordering, merge,
  or topo-sort tie-break** — those are content-causal-plus-hash-lex. But it
  IS materialized into derived state (e.g. `Channel.last_activity_hlc` in
  `materialize.rs:521`, `ephemeral.rs` activity tracking) for activity
  timestamps and ephemeral-channel idle thresholds. Phrase it as
  "not used for DAG ordering or merge, but materialized into activity
  timestamps." (`event.rs:496-498`, design spec lines 174-178).

`EventDag` (`crates/state/src/dag.rs:103-112`) indexes events three ways:
`events: HashMap<EventHash, Event>`, `chains: HashMap<EndpointId, Vec<EventHash>>`,
`heads: HashMap<EndpointId, EventHash>`. The first event must be
`EventKind::CreateServer`; that hash becomes the `server_id`
(`per-author-merkle-dag-state-design.md` §"Server Identity").

`EventDag::insert` enforces structurally:

1. Anti-DoS vector caps (`MAX_EVENT_DEPS`, `MAX_ENCRYPTED_KEY_BYTES`)
   *before* signature verification, so DoS-shaped events are dropped
   without paying the ~50 µs Ed25519 cost (`dag.rs:130-160`).
2. Signature verifies (`dag.rs:163`).
3. Genesis-uniqueness (`NotGenesis` / `DuplicateGenesis`).
4. Per-author chain integrity: `seq == latest_seq + 1` and
   `prev == current_head` — combined, this makes per-author equivocation
   structurally impossible (`dag.rs:193-218`, comment).
5. `Vote` events must include their `proposal` hash in `deps` or `prev`,
   so topo-sort always places the proposal first (`dag.rs:223-230`).

## Shipped today: materialization

`materialize::materialize(dag) -> ServerState` (`materialize.rs:64-80`)
is the canonical projection. It topo-sorts the full DAG and replays each
event through `apply_event`. `apply_incremental(state, event)` is the
incremental form, deduplicated via `state.applied_events` so applying the
same event twice is a no-op (`materialize.rs:92-102`). These are the
*only* public mutation entry points — every consumer (client, worker,
relay) routes through them; nothing else may mutate `ServerState`
(see [authority.md](authority.md), `state-authority-and-mutations.md`
§"Single source of truth").

Topological sort is Kahn's algorithm with a `BTreeSet<&EventHash>` for
the ready set (`dag.rs:362-418`). Concurrent events are tie-broken by
**`EventHash` lexicographic byte comparison**, *not* by HLC. PR #636
lists HLC as a deterministic helper for state-`apply`, but in current
Willow ordering is DAG-causal-plus-hash-tiebreak; HLC is consumed only
on the *materialization* side (channel activity timestamps, ephemeral
idle thresholds — see `crates/state/src/ephemeral.rs`,
`crates/state/src/materialize.rs:521`). Worth flagging when reading PR
#636: Willow's *shipped* convergence proof is "DAG topology + lex hash";
HLC participates in derived state, not in DAG ordering. Whether Myrhiza
should generalize this (continue lex-hash tie-break, payload-agnostic) or
adopt PR #636's HLC-aware-helper framing is a state-apply ABI design
decision — see [determinism.md](determinism.md).

## Shipped today: `EventKind` (chat-specific)

`EventKind` is a 22-variant enum (`event.rs:280-468`) covering:

- Server lifecycle (`CreateServer`).
- Governance (`Propose`, `Vote`).
- Permissions (`GrantPermission`, `RevokePermission`).
- Server structure (`CreateChannel`, `DeleteChannel`, `RenameChannel`,
  `ChannelRevive`, `CreateRole`, `DeleteRole`, `SetPermission`,
  `AssignRole`).
- Chat (`Message`, `FileMessage`, `EditMessage`, `DeleteMessage`,
  `Reaction`).
- Identity (`SetProfile`, `UpdateProfile`).
- Encryption (`RotateChannelKey`).
- Pinning (`PinMessage`, `UnpinMessage`).
- Server metadata (`RenameServer`, `SetServerDescription`).
- Per-identity preferences (`MuteChannel`, `MuteGrove`).

`apply_event` checks `check_permission`, special-cases governance
(`Propose` / `Vote` insert into `pending_proposals`, then
`check_and_apply_proposal` runs threshold logic;
`materialize.rs:161-202`), then delegates to `apply_mutation` for the
rest.

## Shipped today: out-of-order delivery

`PendingBuffer` (`crates/state/src/sync.rs:178-201`) buffers events that
arrive before their per-author predecessor. **Per-author chain gaps
(missing `prev`) are hard gaps — the event is buffered. Cross-author dep
gaps are soft — the event is accepted, the dep is recorded for
background fetching.** Two independent eviction policies bound the
buffer: age-based (`DEFAULT_PENDING_MAX_AGE_MS = 1h`) and capacity-based
(`DEFAULT_PENDING_MAX_ENTRIES = 10_000`, with a per-author sub-cap of
`max_entries / 50` to thwart SEC-V-08).

Sync uses `HeadsSummary { heads: BTreeMap<EndpointId, AuthorHead> }`
(`sync.rs:21-33`) as the compact peer-state advertisement, and
`Snapshot` (`sync.rs:61-71`) as the frozen-state bootstrap for
far-behind peers. `compare_chains` returns `Ahead` / `Behind` / `Synced`
/ `Forked` — the latter detects same-seq-different-hash equivocation.

## Convergence property

Two peers that have seen the same set of events arrive at the same
`ServerState`. The proof has three legs:

1. The DAG is content-addressed; identical event sets ⇒ identical
   `EventDag` contents.
2. Topo-sort is deterministic (Kahn's algorithm + `BTreeSet` lex order).
3. `apply_event` is pure Rust — no clock, no random, no I/O within the
   `willow-state` crate (`lib.rs:3-6`, "zero I/O, zero networking").

See [determinism.md](determinism.md) for the deeper analysis of what
makes this proof hold and where it could break.

## Aspirational (PR #636): generalization

PR #636 (`docs/specs/2026-04-27-willow-runtime/README.md` §"What changes
about Willow") commits to splitting `willow-state` into:

- A **payload-agnostic kernel half**: `Event` envelope, DAG topology,
  `PendingBuffer`, sync primitives, HLC. `EventDag<P>` becomes generic
  over an opaque payload type.
- A **`chat-server` app**: `EventKind`, `ServerState`, `apply_event`,
  `required_permission` — moved into a WASM `state-apply` component.

This split has not happened — today everything is in one crate, and
`EventDag` is concrete over `EventKind`.

## What Myrhiza inherits

**Lifts directly** (payload-agnostic, load-bearing):

- The `Event` envelope shape (`author`, `seq`, `prev`, `deps`, `kind`,
  `sig`, `timestamp_hint_ms`). Generalize `kind` to opaque payload bytes.
- Per-author hash-chain integrity rules (`seq == latest_seq + 1`,
  `prev == current_head`), which structurally prevent equivocation.
- Cross-author `deps` array as advisory causal heads (soft-accept on
  unknown deps).
- `EventDag` indexed by `EventHash` with per-author chain index +
  per-author head map.
- `HeadsSummary` / `AuthorRequest` / sync protocol shape.
- `PendingBuffer` two-policy eviction (age + capacity, with per-author
  sub-cap for Sybil resistance).
- Topo-sort via Kahn's + `BTreeSet` for deterministic ordering.
- `apply_incremental` + `applied_events` dedup contract.
- Genesis-event-defines-server-id pattern.

**Lifts conceptually** (already in Myrhiza CLAUDE.md):

- `state-apply` is a pure function of `(prior state, event)` plus the
  deterministic helper set.
- `state-apply` and pre-check are mechanically the same function.

**Moves into the per-app `state-apply` component** (chat-specific, must
not leak into the kernel):

- `EventKind` enum and its 22 variants.
- `Permission` enum (`SyncProvider`, `ManageChannels`, `ManageRoles`,
  `SendMessages`, `CreateInvite`).
- `ProposedAction` + `VoteThreshold` governance model.
- `ServerState` shape (channels, roles, members, messages, profiles…).
- `required_permission()` table.
- `check_and_apply_proposal` threshold logic.
- The owner-override carve-out for governance
  (`materialize.rs:213-218`, "genesis author can push governance through
  unilaterally") — this is one valid app-level pattern, not a kernel
  built-in.

**Re-evaluates** (Willow's choice may not be the right Myrhiza default):

- `timestamp_hint_ms` with split semantics (signed but not used for DAG ordering, materialized into derived state). PR #636
  proposes "HLC encoded in the event" as a deterministic input to
  `apply`; today's Willow does not use it for ordering. Myrhiza should
  pick a side and stick with it.
- `MAX_EVENT_DEPS = 64` and `MAX_ENCRYPTED_KEY_BYTES = 128` are
  chat-tuned. Myrhiza should re-derive caps once the payload set is
  known.
- The single global `EventDag` per server. Multi-topic peers in Myrhiza
  may want a different ownership story (kernel actor per topic, à la
  PR #636 §"Runtime and actors").

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)
- Files cited below are paths relative to repo root.

## Sources

- `crates/state/src/lib.rs:3-6` — pure-no-I/O contract.
- `crates/state/src/event.rs:22, 280-468, 478-498` — caps, `EventKind`,
  `Event` struct.
- `crates/state/src/dag.rs:103-112, 130-230, 362-418` — DAG indices,
  insert validation, topo-sort.
- `crates/state/src/materialize.rs:64-102, 161-202, 297-346` —
  materialize, apply_incremental, apply_event, required_permission.
- `crates/state/src/server.rs:33-104` — `ServerState` fields.
- `crates/state/src/sync.rs:21-201` — `HeadsSummary`, `Snapshot`,
  `PendingBuffer`.
- `docs/specs/2026-04-01-per-author-merkle-dag-state-design.md`
  §"Section 1", §"Server Identity" — design rationale.
- `docs/specs/2026-04-12-state-authority-and-mutations.md` §"Single
  source of truth", §"Local mutation flow", §"Remote event flow".
- PR #636 `docs/specs/2026-04-27-willow-runtime/README.md` §"Core idea",
  §"Apps as bundles of components", §"What changes about Willow",
  §"What stays the same about Willow".
