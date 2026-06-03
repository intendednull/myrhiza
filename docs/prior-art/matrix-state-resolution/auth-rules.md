**Date:** 2026-05-29
**Status:** active
**Subject:** Matrix authorization rules and power levels — the per-event verdict function state resolution calls

# Authorization rules and power levels

State resolution does not invent authority; it **calls the auth rules** (the
"iterative auth checks" steps in [state-resolution-v2.md](state-resolution-v2.md))
to decide, for one event against one candidate state, *accept or reject*. The auth
rules are Matrix's analog of Myrhiza's `state-apply` **authority verdict**
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4 — the `Accept` return). Critically, the auth rules are a **kernel-fixed
function in Matrix** (the same for all rooms of a given version), whereas
Myrhiza's verdict is **app-supplied WASM**. That difference is the whole reason
Myrhiza needs this folder: Matrix can bake the power-ordering guard into the
protocol; Myrhiza pushes the equivalent responsibility onto app authors.

## The auth rules (room version 11, abbreviated)

Checks are applied in order; first failure rejects. The full list is in each room
version's spec; the load-bearing ones:

1. `m.room.create` must be the first event, have no `prev_events`-derived
   parent, and (≤ v11) a `room_id` whose domain matches the sender. (v12 changes
   this — see [project-hydra-v2.1.md](project-hydra-v2.1.md).)
2. The event's `auth_events` must be the *correct, complete* set for its type
   (e.g. a membership change must cite create + power-levels + the relevant
   membership + join-rules) and must not contain duplicates or rejected events.
3. The sender must be `join`ed to the room (with type-specific exceptions for
   invites, knocks, and the first join).
4. **Power-level gate**: the sender's power level must be ≥ the level required for
   the action. The required level comes from the current `m.room.power_levels`
   event.

## `m.room.power_levels`

A single state event (`type=m.room.power_levels`, empty `state_key`) holding an
integer power table:

- `users: { "@alice:x": 100, … }` — explicit per-user levels.
- `users_default` — level for users not listed (default 0).
- `events: { "m.room.name": 50, … }` — level required to *send* each state type.
- `events_default`, `state_default` — fallbacks for unlisted message/state types.
- `ban`, `kick`, `redact`, `invite` — levels required for those actions.

Power is a **total order of integers**, conventionally 0 (default) / 50
(moderator) / 100 (admin). The well-known footgun: there is no protocol notion of
an *owner* who cannot be demoted — a co-admin at 100 can demote another admin at
100. This is precisely what MSC4289 fixes by giving room creators an **infinite
power level** that `m.room.power_levels` *cannot* override
([project-hydra-v2.1.md](project-hydra-v2.1.md)).

### Power-level change rules

Changing `m.room.power_levels` has extra rules to stop privilege escalation: you
cannot grant anyone (including yourself) a level higher than your own, and you
cannot change a level you don't already out-rank. These rules are what make the
*ordering* of concurrent power-level events matter — if two admins concurrently
edit the table, which one "wins" determines who can subsequently do what, and a
mis-ordering is a state reset.

## Why auth rules + ordering are inseparable

The auth rules are evaluated **against a specific state** (the running
`partial_state`). The *same event* can pass against one state and fail against
another. State resolution's power ordering exists to feed the auth rules the
*right* state in the *right* order, so that authority-removing events are applied
before the events they would invalidate. Auth rules alone (Myrhiza's
`state-apply` verdict alone) are insufficient without an ordering discipline that
front-loads authority changes — the central lesson for the §4.5 RBAC module.

## Implications for Myrhiza

- Myrhiza's `state-apply` *is* the auth-rule function, but it is **per-app and
  unconstrained**. Matrix's experience says the *rules* are the easy part; the
  *ordering the rules are evaluated in* is where state resets hide. An RBAC
  module (§4.5 `myrhiza-permission-rbac`) that re-implements power levels
  inside `state-apply` inherits the ordering hazard *unless the kernel also
  power-orders the authority-changing events* — which §4.1's flat tie-break does
  not. See [lessons.md](lessons.md) Avoid §1, Borrow §2.
- The **no-undemotable-owner** footgun is a direct argument for a Myrhiza
  founder/creator privilege (Myrhiza already has `founder_pubkey` in the genesis
  event, §4.6). MSC4289's "infinite power level" is the validated shape: a
  creator authority that the in-band power table structurally cannot revoke. See
  [lessons.md](lessons.md) Borrow §3.

## Sources

- <https://spec.matrix.org/latest/rooms/v11/#authorization-rules>
- <https://spec.matrix.org/latest/client-server-api/#mroompower_levels>
- <https://matrix.org/docs/older/stateres-v2/>
- <https://github.com/matrix-org/matrix-spec-proposals/pull/4289>
