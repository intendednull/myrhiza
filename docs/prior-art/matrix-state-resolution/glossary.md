**Date:** 2026-05-29
**Status:** active
**Subject:** Matrix state-resolution glossary — system-specific terms

# Glossary

Matrix-specific terms used across this folder. Where a term maps onto a Myrhiza
concept, the mapping is noted.

- **Auth chain** — the transitive closure of an event's `auth_events`: every
  authority decision the event's validity depends on, back to `m.room.create`. A
  DAG. ([event-dag.md](event-dag.md))
- **Auth difference** — events present in *some* but not *all* forks' full auth
  chains. Dragged into the resolution so intermediate authority grants aren't
  lost. ([state-resolution-v2.md](state-resolution-v2.md))
- **`auth_events`** — the small, *separate* edge set naming the events that
  authorize an event (create, power-levels, membership, join-rules). Distinct from
  `prev_events`. No Myrhiza analog — Myrhiza folds authority into `deps` + the
  `state-apply` verdict. ([event-dag.md](event-dag.md))
- **Conflicted state map** — `(type, state_key)` tuples where forks disagree, or a
  value exists in one fork but not another. ([state-resolution-v2.md](state-resolution-v2.md))
- **Conflicted-state subgraph** — v2.1 addition: events bounded by two conflicted
  auth events (reachable from *and* reaching back to conflicted events); expands
  replay scope. ([project-hydra-v2.1.md](project-hydra-v2.1.md))
- **Control event / power event** — a state event that can *remove* an ability:
  `m.room.power_levels`, `m.room.join_rules`, kicks and bans. Power-ordered first
  in resolution. ([state-resolution-v2.md](state-resolution-v2.md))
- **Full conflicted set** — conflicted state map ∪ auth difference. The events
  state resolution actually re-orders and re-checks.
- **Homeserver** — a server hosting Matrix users and running state resolution.
  Trusted by its own users. **No Myrhiza analog** — Myrhiza is P2P, every device
  runs resolution. ([open-problems.md](open-problems.md))
- **Iterative auth checks** — re-deriving authority forward over the ordered
  events, each checked against the running `partial_state`; failures dropped.
  Matrix's analog of replaying through Myrhiza `state-apply`.
- **Mainline ordering** — ordering of *non-power* conflicted events by their
  closest event on the power-levels mainline, then ts, then event ID.
  ([state-resolution-v2.md](state-resolution-v2.md))
- **Power-levels mainline** — the chain of `m.room.power_levels` events from the
  current one back through its auth chain to the create event.
- **Power level** — integer authority. 0 default / 50 mod / 100 admin by
  convention. v12 adds **infinite** for room creators. ([auth-rules.md](auth-rules.md))
- **`prev_events`** — the causal DAG edges (what happened before). Myrhiza's
  `deps`. ([event-dag.md](event-dag.md))
- **Project Hydra** — the Aug-2025 coordinated security effort delivering StateRes
  v2.1 + room version 12. ([project-hydra-v2.1.md](project-hydra-v2.1.md))
- **Reject** — an event that fails the auth rules against its *own* `auth_events`;
  dropped, never enters the accepted DAG. ([event-dag.md](event-dag.md))
- **Reverse-topological power ordering** — the sort applied to power events:
  topo-sort with tie-break **higher power level → lower `origin_server_ts` →
  lexicographically smaller event ID**. The thing Myrhiza §4.1's flat hash
  tie-break lacks. ([state-resolution-v2.md](state-resolution-v2.md))
- **Room version** — immutable string in `m.room.create` pinning a room's ruleset
  (auth rules, hashing, state-res algorithm). Per-room. v2 = StateRes v2; v12 =
  StateRes v2.1. ([room-versions.md](room-versions.md))
- **Soft-fail** — an event valid against its `auth_events` but invalid against
  current resolved state; stored and used in resolution, but hidden from users and
  not a forward extremity. Myrhiza has no equivalent tier. ([event-dag.md](event-dag.md))
- **State at an event** — the resolved `(type, state_key) → event` map over an
  event's `prev_events`. ([event-dag.md](event-dag.md))
- **State reset** — resolved state reverting to an earlier/incorrect value with no
  event validly producing it; the failure mode this folder is about.
  ([state-reset-hazard.md](state-reset-hazard.md))
- **State Resolution v2 (StateRes v2)** — MSC1442 "Reloaded" (authored 2018; room
  version 2 shipped ~2019, Matrix 1.0); room versions 2–11.
  ([state-resolution-v2.md](state-resolution-v2.md))
- **State Resolution v2.1** — MSC4297 (2025); room version 12; empty baseline +
  conflicted-state subgraph. ([project-hydra-v2.1.md](project-hydra-v2.1.md))
- **Unconflicted state map** — tuples all forks agree on; passed through untouched
  in v2. v2.1 stops using it as the auth-check baseline.

## Sources

- <https://matrix.org/docs/older/stateres-v2/>
- <https://spec.matrix.org/latest/rooms/v11/>
- <https://matrix.org/blog/2025/08/project-hydra-improving-state-res/>
