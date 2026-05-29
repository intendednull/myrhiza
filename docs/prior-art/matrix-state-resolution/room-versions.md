**Date:** 2026-05-29
**Status:** active
**Subject:** Room versions — Matrix's forward-compatibility lever for evolving auth rules, ordering, and event format

# Room versions

Matrix cannot change the auth rules, the event hashing, or the state-resolution
algorithm in place — every server replaying a room's DAG must agree on the rules,
forever. So breaking changes are gated behind a **room version**: an immutable
string in `m.room.create` that pins *which ruleset* a room runs under. A room is
created at a version and (mostly) stays there; "upgrading" a room actually
creates a *new* room linked to the old via `m.room.tombstone` /
`m.room.create.predecessor`.

This is Matrix's analog of Myrhiza's **kernel-version / room-version-style
gating** — and a sharper version of it, because Myrhiza pins format at the
*kernel* level ([determinism.md](../../specs/2026-05-09-myrhiza-master-design/determinism.md)
§5.4, bincode 1.3.x; [convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.6 `"myrhiza/topic/v1"` domain separator) whereas Matrix pins it *per room*.
The per-room granularity is what let Matrix ship StateRes v2.1 to *new* rooms
without forcibly migrating every existing one — a property worth weighing.

## The milestones that matter for state resolution

| Version | What it introduced (state-res relevant) |
|---|---|
| **v1** | Original algorithm; event IDs are server-assigned and *separate* from content hash. Had the original state-reset bug. Used 2014–early 2019. |
| **v2** | **State Resolution v2** (MSC1442 "Reloaded", authored 2018-07-20; room version 2 shipped via MSC1759 around Matrix 1.0, 2019-06-10). Auth difference + power ordering + mainline ordering. The headline fix for the v1 reset bug. |
| **v3** | Event IDs become the **reference hash** of the event (content-addressed), removing the server-assigned ID. Hardens against ID forgery. |
| **v4** | URL-safe base64 for event IDs (cosmetic). |
| **v5** | Enforced signing-key validity periods. (Matrix HQ, where issue #6774's reset occurred, was a v5 room — resets survived all of v2–v5.) |
| **v6–v10** | Incremental auth-rule hardening (notarized redactions, restricted join rules, knocking, integer canonicalisation). |
| **v11** | Current pre-Hydra baseline; was the default before Matrix 1.16. Cleaned up redaction rules and `m.room.create` content handling. |
| **v12** | **State Resolution v2.1** + room-creator infinite power + room-ID-as-create-hash. The Project Hydra room version. Default as of Matrix 1.16. |

## Status as of this writing (verified 2026-05-29)

- **Matrix 1.16** (spec release **2025-09-17**) makes **room version 12 the
  default**, with the caveat that servers "SHOULD keep using room version 11 for
  a little while" (MSC4304) to let the ecosystem catch up.
- v12 is **stable spec**, not unstable — it merged into the 1.16 release
  alongside MSC4289 / MSC4291 / MSC4297. (An `unstable/rooms/v12/` spec page also
  exists; the change landed stable in 1.16.)
- Older rooms remain on their original version indefinitely; the v2.1 fix only
  protects rooms created at v12 (or migrated via room upgrade). This is the cost
  of per-room versioning: **the long tail of pre-v12 rooms keeps the old
  hazard**.

## Implications for Myrhiza

- Myrhiza's format/algorithm pinning is **kernel-global**
  ([determinism.md](../../specs/2026-05-09-myrhiza-master-design/determinism.md)
  §5.4 "v1 commits one format"; §4.6 `"myrhiza/topic/v1"` domain separator). That
  is *simpler* than Matrix's per-room versioning but *less migratable*: a future
  ordering fix (e.g. adding power-topological ordering for authority events) would
  be a **kernel major** affecting all apps at once, not an opt-in per-topic
  version. Matrix's pain — a permanent long tail of unfixable old rooms — is the
  cost Myrhiza avoids by pinning globally, *if* it is willing to make ordering
  changes kernel-breaking. See [lessons.md](lessons.md) Borrow §4.
- The **room-upgrade-as-new-room** pattern (tombstone + predecessor link) is a
  precedent for how Myrhiza might migrate a topic across an incompatible
  `state-apply`/ordering change without rewriting the old DAG: spawn a new topic,
  link it, snapshot-import. Relevant to the §4.5 snapshot-as-bootstrap direction.

## Sources

- <https://spec.matrix.org/latest/rooms/>
- <https://matrix.org/blog/2025/09/17/matrix-v1.16-release/>
- <https://spec.matrix.org/unstable/rooms/v12/>
- <https://github.com/Nheko-Reborn/nheko/issues/1931> (room version 12 / CVE context)
