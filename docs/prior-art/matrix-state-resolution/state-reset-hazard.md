**Date:** 2026-05-29
**Status:** active
**Subject:** The state-reset hazard — how a forked authority DAG resurrects stale state, the 8-year incident trail, and why flat ordering is insufficient

# The state-reset hazard

*The reason this folder exists.* A **state reset** is when a room's resolved
state silently reverts to an earlier or incorrect value **in the absence of any
event that would validly produce that value** — a ban gets lifted, a power-level
grant disappears, a kicked user reappears, a room name rolls back — not because
anyone authorized the change, but because state resolution *re-ordered* the DAG
and re-derived authority from the wrong base.

Matrix's own definition (Project Hydra blog, 2025-08-14):

> "the scenario of a room's state resetting to an earlier or incorrect state in
> the absence of revocation events that would validly result in that state."

And the lived symptom (same source):

> "Servers will update the room state while concurrently they lose their
> permission to set said room state. When this happens, the room name change gets
> rolled back to its previous value."

## The mechanism

State resolution is a deterministic function — so a state reset is not a *bug in
one server*; it is the *spec algorithm computing a wrong-but-agreed answer*. The
shape:

1. The DAG forks. On one branch an authority change happens (e.g. Alice's power
   is reduced, or a member is banned).
2. On the other branch, concurrently, someone takes an action that the *old*
   authority permitted but the *new* authority forbids.
3. When the forks merge, the ordering algorithm has to decide which authority
   state was "in force." If a stale authority event sorts *ahead* of the change
   that superseded it, the auth rules re-validate the now-illegitimate action
   against the stale authority — and the legitimate change loses.
4. Result: state reverts. No malicious event was *injected*; the algorithm
   *resurrected* a superseded one by mis-ordering.

This is why StateRes v2 sorts power events **by effective power level first**
([state-resolution-v2.md](state-resolution-v2.md) step 1): front-loading
authority-removing events is the *entire* defence. A purely content-hash /
timestamp tie-break — like Myrhiza's §4.1 — has no such front-loading, so a
stale authority event can win the tie.

## The incident trail (~8 years, honest record)

State resets are the longest-running open wound in Matrix's design history. The
corpus records this verbatim because it is the strongest available evidence that
this failure mode is *subtle, recurring, and survives multiple "fixes"*:

- **Room version 1** (2014–early 2019) had a state-resolution bug that could
  reset state; **room version 2** (StateRes v2; MSC1442 "Reloaded" authored
  2018-07-20, room version 2 shipped via MSC1759 around Matrix 1.0, 2019-06-10)
  was the headline fix — and the matrix.org guide is titled around explaining it.
- Synapse issue **#1935** "Rejected events reset the room state" — mishandling
  *rejected* events (vs *soft-failed*, see [event-dag.md](event-dag.md)) as
  state-affecting.
- Synapse issue **#6774** "I got state-reset out of Matrix HQ (a v5 room)" — a
  reset out of Matrix's own flagship room, *after* StateRes v2.
- Synapse issue **#8629** "state resets still happen in v2 rooms" — explicit
  acknowledgement the v2 algorithm did not eliminate them.
- Synapse issue **#15987** "Another case of state reset in State Resolution v2"
  (2023) — still recurring nearly five years after v2 shipped.
- **Dendrite v0.13.1** fixed a long-standing **off-by-one** that "could result in
  state resets" — a *second independent implementation* hitting the same class of
  bug.
- **CVE-2025-49090** (2025): the algorithm itself, *before v2.1*, "has deficient
  state resolution" — escalating the hazard from operational nuisance to a
  **High-severity (CVSS 7.1)** security vulnerability, because a malicious
  homeserver can *deliberately* craft a fork to force a reset (e.g. revert access
  control). The fix is StateRes v2.1 ([project-hydra-v2.1.md](project-hydra-v2.1.md)).

The takeaway is not "Matrix is buggy" — it is "this failure mode is hard enough
that the only system to deploy it at scale needed *four room versions and a
CVE* to corner it." Any system re-deriving authority from a forkable DAG
inherits the hazard.

## Why flat lexicographic ordering is insufficient

Myrhiza §4.1: concurrent events are tie-broken by `EventHash` lexicographic
comparison, *uniformly, for all event types*. For data events this is correct and
clean. But for **authority-changing** events it means the winner of a concurrent
authority conflict is decided by **content hash** — i.e. essentially at random,
and *not* by which event removed authority. An attacker who can grind an event
hash (or merely gets lucky on ordering) can have a stale-authority event sort
ahead of the event that revoked it → state reset. The hash is uniformly
distributed; authority is not interchangeable. The tie-break treats them as if it
were.

## Implications for Myrhiza

- **Serves convergence.md §4.1 directly.** The load-bearing recommendation: do
  **not** assume uniform lexicographic `EventHash` ordering is safe for
  authority-changing events. Either (a) keep authority *out* of the DAG-ordered
  state entirely, or (b) when the §4.5 RBAC module lands, give authority-changing events a
  **power-topological** ordering key ahead of the hash. See
  [lessons.md](lessons.md) Avoid §1.
- **Serves §4.4 deps-monotonicity.** Matrix's "valid against own auth_events but
  invalid against current state" (soft-fail) is the §4.4 cross-peer-rejection
  case. Matrix treats the resulting divergence as a *correctness* problem worth a
  CVE; Myrhiza frames it as "normal eventual consistency." If a Myrhiza app's
  `state-apply` encodes authority, that framing is too generous — the divergence
  *is* a state reset. See [lessons.md](lessons.md) Avoid §2.

## Sources

- <https://matrix.org/blog/2025/08/project-hydra-improving-state-res/>
- <https://www.tenable.com/cve/CVE-2025-49090>
- <https://github.com/matrix-org/synapse/issues/1935>
- <https://github.com/matrix-org/synapse/issues/6774>
- <https://github.com/matrix-org/synapse/issues/8629>
- <https://github.com/matrix-org/synapse/issues/15987>
- <https://matrix.org/docs/older/stateres-v2/>
