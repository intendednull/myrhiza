**Date:** 2026-05-29
**Status:** active
**Subject:** Matrix room event DAG — events, prev_events, auth_events, the auth chain, and "state at an event"

# The Matrix room event DAG

A Matrix room is a **directed acyclic graph of events**, replicated across the
homeservers whose users participate. State resolution
([state-resolution-v2.md](state-resolution-v2.md)) is the function that turns
this DAG into a single agreed-upon *current state* despite federated peers
seeing events in different orders.

## Event shape (the parts that matter here)

Every event carries, among other fields:

- `type` (e.g. `m.room.message`, `m.room.member`, `m.room.power_levels`).
- `state_key` — present iff the event is a **state event**. A state event
  occupies the slot identified by the tuple `(type, state_key)`. A message
  (`m.room.message`) has no `state_key` and never participates in state.
- `prev_events` — hashes of the event(s) this event was built on top of. This is
  the **causal DAG edge set** — directly analogous to Myrhiza's `deps`
  ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
  §4). Multiple `prev_events` = a merge of forks.
- `auth_events` — a *separate* small set of events that **authorize** this event:
  the create event, the current power-levels event, the sender's membership
  event, and (for membership changes) the target's membership and the join-rules
  event. This is the key structural difference from Myrhiza, which has no
  separate auth-edge set.
- `content`, `sender`, `origin_server_ts` (the sending server's wall clock —
  **advisory, attacker-controllable**, used only as a *secondary* tie-break).
- A reference hash / event ID. In room versions ≤ 11 the room ID embeds the
  creating server's domain; in **v12** the room ID *is* the hash of the
  `m.room.create` event ([project-hydra-v2.1.md](project-hydra-v2.1.md)).

## Two edge sets: `prev_events` vs `auth_events`

This dual-edge design is the single most important thing to absorb before
reading the algorithm:

- `prev_events` describes **what happened before** (causality / ordering).
- `auth_events` describes **what gave this event permission** (authority).

An event points at the specific power-levels and membership events that were
"current" (per its sender) when it was created. The transitive closure of
`auth_events` is the event's **auth chain**: every authority decision that
event's validity depends on, all the way back to `m.room.create`. The auth chain
is itself a DAG.

Myrhiza folds both concepts into one `deps` array plus the `state-apply` verdict
function: causality and authority share a single edge set, and authority is
*re-derived* by replay rather than *pinned* by reference. That is a deliberate
simplification — see [lessons.md](lessons.md) for why it both helps (no stale
auth pin to attack — cf. CVE-2025-54315) and hurts (no explicit auth-chain to
power-sort by).

## "State at an event"

To authorize an incoming event a server must know the **state of the room at
that point in the DAG** — the resolved `(type, state_key) → event` map computed
over the event's `prev_events`. Because forks exist, two `prev_events` can carry
*different* values for the same `(type, state_key)` slot. Resolving that
disagreement is precisely the job of state resolution.

The room's *current* state is "state at the forward extremities" — the set of
events with no children yet. As gossip delivers more events, extremities merge
and state resolution re-runs.

## Soft-fail vs reject

Two distinct failure modes — easy to conflate, load-bearing for the state-reset
story ([state-reset-hazard.md](state-reset-hazard.md)):

- **Rejected**: the event fails the [auth rules](auth-rules.md) *against its own
  declared `auth_events`*. It is dropped and never enters the room's accepted
  DAG. Historically, mishandling rejected events as if they reset state was a
  recurring Synapse bug.
- **Soft-failed**: the event is *valid* against its `auth_events` but *invalid*
  against the server's current resolved state (e.g. the sender was banned in the
  meantime, by an event the sender hadn't seen). The event is stored and used in
  state resolution, but **not shown** to local users and **not** treated as a
  forward extremity. Soft-fail is Matrix's defence against a peer reaching into
  the past to splice an authorized-looking-but-actually-superseded action.

Myrhiza's analog is the [convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.4 pre-check/apply split with deps-monotonicity: "valid against its deps" vs
"valid against my current state" is the same distinction, but Myrhiza has no
soft-fail tier — apply either `Accept`s or doesn't.

## Implications for Myrhiza

- Matrix's **separate `auth_events` edge set** is what makes power-topological
  ordering *possible*: you can extract "the authority subgraph" cheaply. Myrhiza
  has no such set; reconstructing an authority ordering from `deps` alone is
  harder. If the §4.5 RBAC module or the §4.4.1 warrant module ever need
  power-ordering, this absence is the first design wall. See
  [lessons.md](lessons.md) Borrow §1.
- The **soft-fail tier** is a concept Myrhiza lacks and may want: an event that
  is deps-valid but current-state-invalid is *exactly* the §4.4 cross-peer
  rejection case, and Matrix's choice to *store-but-hide* rather than *drop* is
  worth weighing against Myrhiza's "normal eventual consistency" framing. The
  closest Myrhiza analog to a soft-failed event — *valid against its own deps,
  invalid against current state because a concurrent authority change superseded
  it* — is the **author-equivocation / warrant** flow
  ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
  §4.4.1): an event that is well-formed against the author's chain but conflicts
  with what other peers accepted. Matrix's store-but-hide-and-still-resolve
  treatment is a candidate model for that §4.4.1 surface, where Myrhiza v1
  currently does first-seen-wins-per-peer with no shared tier.

## Sources

- <https://spec.matrix.org/latest/server-server-api/#room-state-resolution>
- <https://matrix.org/docs/older/stateres-v2/>
- <https://spec.matrix.org/latest/rooms/v11/> (auth_events selection, soft-fail)
- <https://matrix.org/blog/2025/08/project-hydra-improving-state-res/>
