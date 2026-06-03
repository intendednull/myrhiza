**Date:** 2026-05-29
**Status:** active
**Subject:** Project Hydra / State Resolution v2.1 — the Aug-2025 fix: empty-baseline replay, conflicted-state subgraph, creator infinite power, room-IDs-as-create-hash

# Project Hydra and State Resolution v2.1

**Project Hydra** is the Matrix.org Foundation's August 2025 coordinated security
fix for state resolution. Timeline (verified):

- **2025-07-16** — pre-disclosure of upcoming federation-protocol fixes.
- **2025-08-11** — patched homeserver releases shipped.
- **2025-08-14** — embargo lifted; remaining MSCs and the "Project Hydra"
  explainer blog published.
- **2025-09-17** — changes merged into the stable spec as **Matrix 1.16**;
  **room version 12** becomes default.

The name nods to the hydra: cut off one state-reset head and two grow back. v2.1
is described by its own MSC as "an incremental change over the current State
Resolution 2.0 algorithm, which protects against various classes of 'state
resets'."

## The four MSCs

| MSC | Title | What it does |
|---|---|---|
| **MSC4297** | State Resolution v2.1 | The algorithm change — empty baseline + conflicted-state subgraph (below). |
| **MSC4289** | Explicitly privilege room creators | Room creators get an **infinite power level** the power-table can't override; adds `additional_creators`. |
| **MSC4291** | Room IDs as hashes of the create event | Room ID *is* the `m.room.create` hash; `m.room.create` removed from `auth_events`. |
| **MSC4304** | Room version 12 | Bundles the above into a new room version, made the spec default. |

## CVEs fixed

- **CVE-2025-49090** — "The Matrix specification before 1.16 (i.e., with a room
  version before 12 and State Resolution before 2.1) has deficient state
  resolution." CVSS v3 base **7.1 (High)**, published **2025-10-02**. A malicious
  participating homeserver can craft a fork to force a state reset (e.g. revert
  access control / membership).
- **CVE-2025-54315** — Matrix rooms before v12 "do not strongly (cryptographically)
  enforce the uniqueness of a room's creation event." Fixed by MSC4291 making the
  room ID the create-event hash.

## Mechanical change 1 — empty baseline for iterative auth checks

StateRes v2 seeds the iterative auth checks ([state-resolution-v2.md](state-resolution-v2.md)
step 2) with the **unconflicted state map** as the starting `partial_state`.
v2.1 starts from the **empty state map** instead — verified phrasing from the
implementer's guide: "replace the unconflicted set with the empty set," passing
`{}` instead of `unconflicted_state` to `_iterative_auth_checks`. The room
version 12 spec confirms: "The iterative auth checks now start with an empty state
map instead of the unconflicted state map."

Why: trusting the unconflicted set as an unchecked baseline let a crafted fork
smuggle a stale authority value through as "unconflicted," which then anchored a
reset. Re-deriving *everything* from empty, in power order, removes that trusted
baseline.

## Mechanical change 2 — the conflicted-state subgraph

v2.1 adds an explicit **conflicted-state subgraph**: the set of events "bounded
by two conflicted auth events" — events that are both *reachable from* conflicted
events and *can reach back to* conflicted events. The replay scope expands to
include **all events between conflicted events**, not just the conflicted events
themselves. The implementer's guide calls this "a more invasive change and is the
bulk of this guide," and offers two computations: a naive depth-first traversal,
and an optimised strongly-connected-components approach (forward/backward
reachability intersection). This is the head that kept growing back: the earlier
algorithm under-scoped what needed re-checking, so authority changes in the
*gaps* between conflicted events got skipped.

## Mechanical change 3 — room-creator infinite power level (MSC4289)

The `sender` of `m.room.create`, plus any user IDs in an `additional_creators`
array, are **room creators**. Spec phrasing: "The power level of 'room creators'
is infinitely high"; creators "cannot be demoted to a lower power level, even
through `m.room.power_levels`." `m.room.power_levels` is *rejected* if its `users`
map tries to assign a level to a creator. This eliminates the
no-undemotable-owner footgun ([auth-rules.md](auth-rules.md)) and gives state
resolution an unconditional authority anchor that no fork can reorder away —
because the creator's power isn't a value in the orderable state at all.

## Mechanical change 4 — room IDs as create-event hash (MSC4291)

The room ID becomes the hash of `m.room.create`; the domain component is dropped.
Consequence relevant here: **`m.room.create` MUST NOT be selected for
`auth_events` on events** (room version 12 spec) — the create event is now
implied by the room ID itself, so it no longer needs to be a pinned auth edge,
and its uniqueness is cryptographically guaranteed (no two distinct create events
can share a room ID). This is structurally identical to Myrhiza's
[convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.6 **content-addressed topic ID** (`BLAKE3` over bundle hash + instance seed)
and the §4.6 genesis-event invariant — Matrix arrived at the same idea, eight
years and one CVE later.

## Implications for Myrhiza

- **§4.6 validation, retroactive.** Matrix's MSC4291 is independent confirmation
  that the genesis/create event *should* be the content-addressed root and *not*
  a forgeable, separately-pinned auth reference. Myrhiza already does this
  (`topic_id` = hash including genesis seed; CVE-2025-54315 is a non-issue for
  Myrhiza by construction). See [lessons.md](lessons.md) Validates.
- **§4.6 founder → creator-infinite-power.** Myrhiza has `founder_pubkey` in
  genesis but no spec'd notion that the founder's authority is *unrevocable by
  in-band state*. MSC4289 is the validated shape for the deferred RBAC module
  (§4.5 `myrhiza-permission-rbac`): a creator privilege that lives *outside* the
  orderable power state, so no fork can reset it. See [lessons.md](lessons.md)
  Borrow §3.
- **Empty-baseline / subgraph = "re-derive authority from nothing, in power
  order."** Even though Myrhiza re-derives all state by replay, it has no concept
  of scoping a re-check to "the conflicted authority subgraph." If the §4.5 RBAC
  module needs to resolve concurrent authority changes, v2.1's subgraph computation is
  the reference algorithm. See [lessons.md](lessons.md) Borrow §2.

## Sources

- <https://matrix.org/blog/2025/08/project-hydra-improving-state-res/>
- <https://matrix.org/blog/2025/09/17/matrix-v1.16-release/>
- <https://matrix.org/docs/spec-guides/state-res-2.1/>
- <https://spec.matrix.org/unstable/rooms/v12/>
- <https://github.com/matrix-org/matrix-spec-proposals/pull/4289>
- <https://www.tenable.com/cve/CVE-2025-49090>
