**Date:** 2026-05-29
**Status:** active
**Subject:** State Resolution v2 — the algorithm: conflicted/unconflicted state, auth difference, power ordering, mainline ordering, iterative auth checks

# State Resolution v2 (the algorithm)

*The load-bearing file.* This is what Myrhiza's
[convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.1 tie-break is the simplified, flat cousin of. StateRes v2 (MSC1442 "State
Resolution: Reloaded", Erik Johnston, authored 2018-07-20; shipped with **room
version 2** via MSC1759 around Matrix 1.0, 2019-06-10)
resolves *N* conflicting state maps into one. It is a **pure, deterministic
function** of its inputs — same property Myrhiza requires of `state-apply`.

The plain-language reference is the matrix.org guide "State Resolution v2 for the
Hopelessly Unmathematical"; the normative version is in each room version's spec.

## Inputs and the conflict split

Input: a list of state maps (one per fork being merged), each a
`(type, state_key) → event` mapping.

- **Unconflicted state map**: tuples where *all* forks agree on the value (same
  event, or the value predates the fork). These pass through untouched.
- **Conflicted state map**: every other tuple — either forks disagree on the
  value, or a value exists in one fork but not another.

Resolving only the *conflicted* set, then layering it over the unconflicted set,
is the core efficiency win: you don't re-derive the whole room.

## The auth difference and the full conflicted set

The naive conflict split is **not enough** — this is the crux that defeats a flat
tie-break:

- Compute each fork's **full auth chain** (union of the auth chains of all its
  events).
- The **auth difference** = every event that does *not* appear in *every* fork's
  full auth chain — i.e. the authority decisions present in some forks but not
  others.
- The **full conflicted set** = conflicted state map ∪ auth difference.

Pulling in the auth difference is what guarantees the algorithm has *the
intermediate power-grant events it needs* to authorize a later event when forks
reconverge. The canonical example: Alice grants Bob power (**A**), Bob grants
Charlie power (**B**), Charlie changes the ban level (**C**). A fork that has
**C** but not **B** would fail to authorize **C** — unless **B** is dragged in
via the auth difference.

## Step 1 — power events, reverse-topological power ordering

A **control event** (a.k.a. **power event**) is a state event with the potential
to *remove someone's ability to do something*. The spec enumerates them:

- `m.room.power_levels` (the power-level table itself),
- `m.room.join_rules`,
- `m.room.member` with `membership: "leave"` where `sender != state_key` (a
  **kick**),
- `m.room.member` with `membership: "ban"` where `sender != state_key` (a
  **ban**).

Take all power events in the full conflicted set, plus any events in *their* auth
chains that also fall in the full conflicted set, and sort them by **reverse
topological power ordering** — a topological sort (Kahn's algorithm, like
Myrhiza's §4.1) where ties are broken by, **in order**:

1. **higher effective power level comes first** (the power-topological key),
2. then **lower `origin_server_ts`** (earlier wins),
3. then **lexicographically smaller event ID**.

This is the decisive difference from Myrhiza. Myrhiza's §4.1 uses **only key 3**
(`EventHash` lexicographic) for *every* event. Matrix puts **power level first**
specifically so that an authority-*increasing* or authority-*removing* event
cannot be reordered behind a stale event that would undo it. The
content-hash tie-break is the *last* resort, not the only rule.

## Step 2 — iterative auth checks over the sorted power events

Starting from the **unconflicted state map** as the running `partial_state`
(this baseline is exactly what v2.1 changes — see
[project-hydra-v2.1.md](project-hydra-v2.1.md)), walk the sorted power events in
order. For each event, run the [auth rules](auth-rules.md) using `partial_state`
as the allowed auth set. If it passes, it updates `partial_state`; if it fails,
it is **dropped from the resolution** (not added). Authority is thus re-derived
forward, in power order, from a trusted base.

## Step 3 — mainline ordering for the remaining (non-power) events

The remaining conflicted events (ordinary state: name, topic, non-kick
memberships, …) are ordered by **mainline ordering**, keyed off the
**power-levels mainline** — the chain you get by following each
`m.room.power_levels` event back through its auth chain to the create event. Each
event maps to "the closest power-levels event on the mainline." Sort by, in
order:

1. **closest-mainline depth** (events anchored to a later power-levels event come
   later),
2. then **lower `origin_server_ts`**,
3. then **lexicographically smaller event ID**.

Mainline ordering ties ordinary state changes to *the power structure in force
when they happened*, so a name change made under an old power-levels event can't
leapfrog the resolved power table.

## Step 4 — final iterative auth checks

Append the mainline-ordered events to the (already power-resolved)
`partial_state` and run iterative auth checks again across the whole sequence.
The survivors, layered over the original unconflicted state map, are the
**resolved state**.

## Determinism

Every step is a deterministic function of the input event set: topological sort
+ total tie-break order (power → ts → event ID) leaves no ambiguity, so all
honest servers compute the identical result. This is the same convergence
guarantee Myrhiza gets from §4.1 — but Matrix's *correctness* (not just
*agreement*) leans on the power-first key. Two peers can deterministically
*agree* on a wrong answer; that is the state reset
([state-reset-hazard.md](state-reset-hazard.md)).

## Implications for Myrhiza

- Myrhiza §4.1's flat `EventHash` tie-break is **StateRes v2 with steps 1–3
  collapsed to step's-3-tie-break-only**. It buys determinism but discards the
  power-topological guard. For data-plane events (chat lines, counter
  increments) this is fine. For **authority-changing** events — role grants,
  membership/ban changes, capability revocations — it courts the exact hazard
  Matrix spent years fixing. See [lessons.md](lessons.md) Avoid §1.
- Matrix re-derives authority by **replay in power order** rather than trusting
  pinned `auth_events`. Myrhiza already re-derives everything by replay through
  `state-apply` — but with no notion of "replay the authority subgraph first."
  Borrowing the *ordering*, not the dual-edge representation, is the realistic
  path (§4.5 RBAC). See [lessons.md](lessons.md) Borrow §2.

## Sources

- <https://matrix.org/docs/older/stateres-v2/>
- <https://github.com/matrix-org/matrix-spec-proposals/blob/main/proposals/1442-state-resolution.md>
- <https://spec.matrix.org/latest/rooms/v11/#state-resolution>
- <https://matrix.uhoreg.ca/stateres/reloaded.html>
