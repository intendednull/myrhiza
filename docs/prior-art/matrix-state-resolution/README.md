**Date:** 2026-05-29
**Status:** active
**Subject:** Matrix state resolution — per-room event DAG + State Resolution v2 / v2.1; the deployed-at-scale reference on the state-reset failure mode of concurrent authority-changing events

# Matrix state resolution prior art

Reference folder for **Matrix room state resolution** — the algorithm a Matrix
homeserver runs to merge divergent views of a room's *state* (membership, power
levels, name, topic, …) after federated peers gossip events out of order over a
per-room event DAG. Scope is deliberately narrow: this folder covers the
**DAG-authority-resolution mechanics**, not the Matrix product, not E2EE
(that's [`../mls/`](../mls/)), not the federation/identity layer (that's
[`../at-protocol/`](../at-protocol/)).

10 content files + this index (11 markdown files total), ~1,190 lines.

## Why this folder exists — and why it is the runner-up, not the primary

Myrhiza's convergence design ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.1) tie-breaks concurrent events by **uniform lexicographic `EventHash`
comparison** — every event, including authority-changing ones, gets the same
flat ordering. Matrix learned the hard way that this is *exactly the recipe for
a state reset*: when two peers concurrently change who-can-do-what, a naive
ordering can resurrect a stale authority state and silently roll back a
membership ban, a power-level grant, or a room rename. Matrix's answer is
**power-topological ordering** — control events and their auth chains are sorted
by effective power level *first*, ordinary tie-breaks (timestamp, then event ID)
only *after*. That is the load-bearing lesson for Myrhiza.

The master spec's **named** prior art for convergence is Holochain warrants
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.4.1, with the broader Holochain primitive set in the §4.5 scaling future
direction) and Croquet's TUTTI ([`../croquet/`](../croquet/)). Matrix is the
**runner-up**. It earns a folder for one reason: it is the only system that has
deployed a per-author/per-room event-DAG authority-resolution algorithm **at
internet scale, kept hitting state resets in production for ~8 years, and
publicly documented every iteration of the fix** — culminating in the August
2025 "Project Hydra" disclosure and State Resolution v2.1. No other reference in
the corpus offers a comparably honest, churning track record on this specific
hazard.

## Key facts at a glance

| Field | Value | Source |
|---|---|---|
| Steward | **The Matrix.org Foundation** (CIC, UK; formed 2018, Matrix 1.0 in 2019). Spec curated by the **Spec Core Team** subcommittee. **Element** (formerly New Vector) is the primary commercial sponsor + Synapse author | matrix.org/foundation |
| Algorithm in use | **State Resolution v2** (room versions 2–11); **State Resolution v2.1** (room version 12) | spec.matrix.org |
| StateRes v2 origin | MSC1442 "State Resolution: Reloaded", Erik Johnston, **authored 2018-07-20**; **room version 2** (via MSC1759) shipped ~Matrix 1.0, **2019-06-10** | github MSC1442/1759 |
| Current spec | **Matrix 1.16**, released **2025-09-17**; makes **room version 12 the default** (servers SHOULD keep using v11 "for a little while") | matrix.org v1.16 blog |
| Project Hydra disclosure | Pre-disclosure **2025-07-16**, patches **2025-08-11**, full details + blog **2025-08-14** | Hydra blog |
| StateRes v2.1 MSCs | **MSC4297** (StateRes v2.1), **MSC4289** (privilege room creators / infinite power level), **MSC4291** (room IDs as create-event hash), **MSC4304** (room version 12 default) | Hydra blog |
| CVEs fixed | **CVE-2025-49090** (deficient state resolution → state resets, CVSS v3 **7.1 High**) and **CVE-2025-54315** (room create-event uniqueness not cryptographically enforced) | Tenable / Hydra blog |
| Reference impls | **Synapse** (Python, Element), **Dendrite** (Go), **Conduit/conduwuit** (Rust), **`ruma-state-res`** (Rust crate, MIT) | This Week in Matrix |

## Canonical reading order

1. **[event-dag.md](event-dag.md)** — the per-room event DAG: events, `prev_events`,
   `auth_events`, the auth chain, what "state at an event" means.
2. **[state-resolution-v2.md](state-resolution-v2.md)** — *the load-bearing file*.
   Conflicted vs unconflicted state, the auth difference, the full conflicted
   set, control/power events, **reverse-topological power ordering**, mainline
   ordering, iterative auth checks.
3. **[auth-rules.md](auth-rules.md)** — the per-event authorization rules and
   power levels; what makes an event "rejected" vs "soft-failed".
4. **[state-reset-hazard.md](state-reset-hazard.md)** — *the reason this folder
   exists*. How a forked DAG resurrects stale authority; the 8-year incident
   trail; why flat ordering is insufficient.
5. **[room-versions.md](room-versions.md)** — room versions as the spec's
   forward-compatibility lever; v1→v2→v11→v12 milestones.
6. **[project-hydra-v2.1.md](project-hydra-v2.1.md)** — the Aug-2025 fix:
   empty-baseline replay, the conflicted-state subgraph, creator infinite power,
   room-IDs-as-create-hash; CVE-2025-49090 / -54315.
7. **[implementations.md](implementations.md)** — Synapse / Dendrite / Conduit /
   ruma; the chain-cover index; performance traps.
8. **[open-problems.md](open-problems.md)** — what state resolution structurally
   does NOT solve.
9. **[lessons.md](lessons.md)** — *the decision file*: validates / avoid /
   borrow, tied to the Myrhiza decision surfaces.
10. **[glossary.md](glossary.md)** — Matrix-specific terms.

If you only have time for two files: **[lessons.md](lessons.md)** +
**[state-reset-hazard.md](state-reset-hazard.md)**.

## Relationship to sibling corpus folders

- **[`../croquet/`](../croquet/)** + **[`../holochain/`](../holochain/)** +
  [`../crdts/`](../crdts/) + [`../agoric-endo/`](../agoric-endo/) — the four
  *named* convergence paradigms. Matrix is a fifth data point on the same axis
  (deterministic re-derivation of state from an ordered DAG), specialised to the
  **authority-resolution** sub-problem the four don't dwell on.
- **[`../at-protocol/`](../at-protocol/)** — federation + identity. Distinct
  axis: atproto is the cautionary federation tale; Matrix here is *not* about
  federation topology, only the in-room ordering algorithm. Cross-linked, not
  overlapping.
- **[`../mls/`](../mls/)** — group keys / E2EE. Matrix uses MLS-adjacent crypto
  (Olm/Megolm) for message confidentiality; that is *out of scope* here. State
  resolution operates on cleartext state events.

## Framing disclosure

These docs are written from the **Myrhiza-as-deterministic-event-DAG-runtime**
stance — capability-mediated, P2P-only, Component-Model-on-Wasmtime,
event-log-replay `state-apply`. The "Implications for Myrhiza" notes read
Matrix's choices through the
[convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.1 tie-break decision and the deferred warrant/equivocation module (§4.4.1
`myrhiza-permission-warrants`) and RBAC/participation module (§4.5
`myrhiza-permission-rbac`). This is **not a neutral catalog**: the lessons read
Matrix through Myrhiza's current design lens, and the framing presumes Myrhiza's
event-DAG + capability model is the target. Matrix is a *server-federated*
system, not P2P — homeservers, not end devices, run state resolution, and a
homeserver is a trusted aggregator for its local users. Myrhiza has no such
aggregator. The ordering *algorithm* translates cleanly; the trust model does
not. Weigh [open-problems.md](open-problems.md) and the per-file "Implications"
notes with that gap in mind.

**Load-bearing caveat — read against the grain.** State resolution is a
*runner-up* prior art whose central hazard (state reset on concurrent authority
events) is one Myrhiza would **inherit** if authority ever lives in
DAG-ordered state. That gives this corpus an incentive to *soft-pedal* the
hazard — to frame Matrix's eight-year struggle as "they fixed it in v2.1" and
imply Myrhiza is clear by construction. It is not: Myrhiza's flat §4.1 tie-break
is *weaker* than StateRes v2 on exactly this axis, and v1's escape hatch
("keep authority static") is a scope limit, not a solution. When these files say
"Myrhiza is ahead," check whether that is true *today* or only true *if a
deferred module ships the right ordering discipline*.

## Sources

- <https://spec.matrix.org/latest/>
- <https://matrix.org/docs/older/stateres-v2/> (State Resolution v2 for the Hopelessly Unmathematical)
- <https://matrix.org/blog/2025/08/project-hydra-improving-state-res/>
- <https://matrix.org/blog/2025/09/17/matrix-v1.16-release/>
- <https://matrix.org/foundation/about/>
- <https://www.tenable.com/cve/CVE-2025-49090>
- <https://github.com/matrix-org/matrix-spec-proposals/blob/main/proposals/1442-state-resolution.md>
