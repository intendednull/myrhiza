**Date:** 2026-05-29
**Status:** active
**Subject:** Lessons from Matrix state resolution for Myrhiza — validates / avoid / borrow, tied to convergence.md §4.1, §4.4, §4.4.1, §4.5

# Lessons for Myrhiza

The decision-relevant synthesis. Other files are evidence; this is the takeaway.

**Framing first (honest):** Matrix is the **runner-up**, not the primary, prior
art for Myrhiza convergence. The master spec names Holochain warrants
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.4.1; the broader Holochain primitive set is in the §4.5 scaling future
direction) and Croquet TUTTI ([`../croquet/`](../croquet/)) as its references.
Matrix earns its place for *one* thing: it is the only system to deploy a
per-author/per-room event-DAG authority-resolution algorithm at internet scale,
hit the **state-reset failure mode** repeatedly for ~8 years, and document every
fix. Its single load-bearing message: **authority-changing events may need
power-topological ordering, not Myrhiza's uniform lexicographic `EventHash`
tie-break — or you risk state resets.**

## Validates

Matrix's experience *confirms* these Myrhiza bets:

- **Content-addressed genesis root is correct (§4.6).** MSC4291 (room ID =
  `m.room.create` hash) is Matrix arriving, after CVE-2025-54315, at exactly
  Myrhiza's §4.6 design (`topic_id` = BLAKE3 over bundle hash + genesis seed;
  genesis-event invariant). Myrhiza had this from day one; create-event
  forgeability is a non-issue by construction. See
  [project-hydra-v2.1.md](project-hydra-v2.1.md).
- **Excluding wall-clock from ordering is correct (§4.1).** Matrix's
  `origin_server_ts` tie-break is attacker-controllable and a recurring weak spot
  ([open-problems.md](open-problems.md) §4). Myrhiza signs HLC into events but
  *refuses to order by it*. That is a *stronger* position than Matrix's. Don't
  regress.
- **Building drift detection in is correct (§4.7).** Matrix found state resets via
  user reports and GitHub issues — it has **no** automated cross-peer drift check.
  Myrhiza's §4.7 TUTTI-shaped digest voting is ahead of the most-deployed
  comparable system. See [open-problems.md](open-problems.md) §7.
- **CRDT-in-state-apply, not CRDT-in-substrate, is correct (§4.9).** State
  resolution does only structural conflict resolution (one winner per slot, the
  loser silently lost) — no semantic merge. Apps wanting merge must bring their
  own, exactly Myrhiza's §4.9 stance. Cross-ref [`../crdts/`](../crdts/).

## Avoid

Specific pitfalls + the Myrhiza mitigation:

- **§1 (THE lesson) — don't trust a uniform lexicographic `EventHash` tie-break
  for authority-changing events.** Myrhiza §4.1 orders *all* concurrent events,
  including authority changes, by content hash alone. Matrix proved this is the
  state-reset recipe: a stale authority event can sort ahead of the event that
  revoked it, and the auth check then re-validates an illegitimate action against
  stale authority ([state-reset-hazard.md](state-reset-hazard.md)). **Mitigation:**
  for v1, *keep authority out of DAG-ordered state* (the genesis founder + caps
  are the only authority; no in-band role/power changes). When the §4.5 RBAC
  module (or the §4.4.1 warrant/equivocation module) lands and authority *does*
  change in-band, give authority-changing events a **power-topological** ordering
  key *ahead of* the hash — i.e. adopt StateRes v2's power-first sort, not
  Myrhiza's flat sort, for that event class.
- **§2 — don't frame authority divergence as "normal eventual consistency"
  (§4.4).** Myrhiza §4.4 calls cross-peer rejection from differing prior-states
  "normal." That is fine for data events, but if `state-apply` encodes authority,
  the *same* divergence *is* a state reset — Matrix made it a CVSS-7.1 CVE, not a
  shrug ([state-reset-hazard.md](state-reset-hazard.md)). **Mitigation:** the §4.4
  deps-monotonicity invariant must be *strengthened* for authority: an
  authority-bearing `state-apply` must be valid against any state containing the
  deps closure *and* must not let ordering of authority events change the verdict.
  Document this as a stricter invariant for RBAC modules.
- **§3 — don't import Matrix's trusted-aggregator trust model
  ([open-problems.md](open-problems.md) §3).** Matrix runs resolution on
  homeservers and trusts a homeserver for its own users; its threat actor is "a
  malicious *homeserver*." Myrhiza has no aggregator — every device is a potential
  adversary. Matrix's simplifications (soft-fail visibility, server-set timestamps)
  assume a server tier Myrhiza lacks. **Mitigation:** treat every Matrix "the
  server handles X" as "Myrhiza must handle X with no trusted intermediary."
- **§4 — don't assume one implementation is enough.** Synapse and Dendrite
  diverged on the same algorithm (off-by-one → state reset,
  [implementations.md](implementations.md)). **Mitigation:** keep authority
  ordering *in the kernel* (one implementation) rather than in per-app
  `state-apply` WASM where every app re-implements and re-bugs it.
- **§5 — don't underestimate the cost of the fix.** v2.1's conflicted-state
  subgraph is *more* expensive than v2; hardening against resets cost real CPU and
  needed an auth-reachability index ([implementations.md](implementations.md)).
  **Mitigation:** budget for it — power-ordering authority isn't free.

## Borrow

Primitives worth studying / lifting:

- **§1 — the separate auth-edge set as the enabler of power-ordering.** Matrix's
  `auth_events` (distinct from `prev_events`) is what makes "extract the authority
  subgraph and sort it by power" cheap ([event-dag.md](event-dag.md)). Myrhiza
  folds both into `deps`. *If* the §4.5 RBAC module needs power-ordering, the first
  design question is how to identify the authority subgraph without a dedicated
  edge set — e.g. typed events the kernel recognises as authority-bearing.
- **§2 — StateRes v2's power-topological ordering + iterative auth checks.** The
  algorithm itself ([state-resolution-v2.md](state-resolution-v2.md)): reverse-topo
  sort with the **power-level-first** tie-break (then ts, then ID), then re-derive
  authority forward via iterative auth checks. This is the concrete recipe to lift
  for authority-changing events. v2.1's **empty-baseline + conflicted-state
  subgraph** ([project-hydra-v2.1.md](project-hydra-v2.1.md)) is the hardened
  version. **`ruma-state-res`** (Rust, MIT) is the readable reference impl.
- **§3 — creator infinite power level outside the orderable state (MSC4289).** Make
  the founder's authority *unrevocable by in-band power state* — a privilege the
  power table structurally cannot reorder away. Myrhiza already has
  `founder_pubkey` in genesis (§4.6); MSC4289 is the validated shape for giving it
  unconditional, un-resettable authority in the RBAC module. Kills the
  no-undemotable-owner footgun ([auth-rules.md](auth-rules.md)).
- **§4 — per-version gating as the migration lever ([room-versions.md](room-versions.md)).**
  Matrix shipped v2.1 to new rooms without rewriting old DAGs via room versions.
  Myrhiza pins format kernel-globally (§5.4); the *cost* Matrix paid (permanent
  unfixable long tail) is the trade Myrhiza avoids by making ordering changes
  kernel-breaking — provided it's willing to. Record the trade explicitly.

## Open questions for the spec author

- **Does authority ever change in-band in v1?** If §4.1's flat tie-break stands,
  v1 must keep authority *static* (genesis founder + caps only). If any app needs
  in-band role/ban/power changes, §4.1 needs a power-ordering carve-out *before*
  that app ships. This is the gating decision.
- **What identifies an "authority-changing event" without an auth-edge set?** A
  kernel-recognised event type? A manifest declaration? An RBAC-module-owned
  namespace? Needed before power-ordering is implementable.
- **How does §4.4 deps-monotonicity tighten for authority?** Draft the stricter
  invariant (ordering-independence of authority verdicts) for the §4.5 RBAC child
  spec.
- **Is the founder privilege un-resettable (MSC4289-style)?** Decide whether
  `founder_pubkey` authority lives outside the orderable state.

## Cross-paradigm placement

Matrix is a **fifth** data point alongside the four named convergence paradigms
([`../croquet/`](../croquet/), [`../holochain/`](../holochain/),
[`../crdts/`](../crdts/), [`../agoric-endo/`](../agoric-endo/)): deterministic
re-derivation of state from an ordered DAG, *specialised to authority resolution*
— the sub-problem the other four don't dwell on. Distinct from
[`../at-protocol/`](../at-protocol/) (federation/identity) and [`../mls/`](../mls/)
(group keys); cross-linked, non-overlapping.

## Sources

- Synthesises the sibling files; primary sources cited per sibling.
- <https://matrix.org/docs/older/stateres-v2/>
- <https://matrix.org/blog/2025/08/project-hydra-improving-state-res/>
- <https://www.tenable.com/cve/CVE-2025-49090>
- Myrhiza spec: [convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.1, §4.4, §4.4.1, §4.5, §4.6, §4.7, §4.9
