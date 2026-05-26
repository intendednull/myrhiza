**Date:** 2026-05-22
**Status:** active
**Subject:** The three migration-strategy families Myrhiza must pick between for `state-apply` upgrades — re-replay from genesis, explicit migration function, version-and-refuse. Each has a different cost profile; the right answer probably differs per app.

## The decision

When a `state-apply` component is upgraded from v1.0 → v1.1, the existing on-disk snapshots and the existing in-flight event logs must be reconciled with the new code. Three families of answers:

1. **Re-replay from genesis** — discard the snapshot, replay the full event log through v1.1's `state-apply`.
2. **Explicit migration function** — call a v1.0 → v1.1 conversion routine on the snapshot, then continue replaying from where the snapshot left off.
3. **Version-and-refuse** — pin snapshot+code to a content-addressed version pair; refuse to load a v1.0 snapshot in a v1.1 binary at all.

Each one is a real production strategy used by real systems. None is universally right.

## Strategy 1: Re-replay from genesis

**Mechanism.** When an app upgrades from v1.0 to v1.1, the kernel discards every cached snapshot for that app's state, then re-replays the full event log through v1.1's `state-apply`. The output is the v1.1 view of the data.

**Cost model.** O(log size) per upgrade, on every replica. Storage cost: zero extra (no migration code, no version-pinned snapshots). Code cost: zero extra (no per-version migration function).

**When it's right.** Event-log-as-source-of-truth systems with bounded log sizes. If the log is "all events in the last 6 months" and the upgrade takes 30 seconds to re-replay, this is the simplest correct answer. The genesis-replay model is what makes the strategy *clean*: there is no other state on disk, just the log + a cache.

**When it's wrong.**

- **Unbounded logs.** If the log is "all events ever," re-replay is unbounded. Eventually it doesn't finish in a reasonable upgrade window.
- **Non-deterministic side effects.** If `state-apply` made any external calls during replay (sent a network message, woke up a user notification), re-replaying replays those too. Re-replay only works if state-apply is *purely* a function of the log.
- **Semantic changes that re-replay can't fix.** If v1.1 reinterprets the meaning of some past event differently from v1.0, re-replay gives you the v1.1 interpretation — which may or may not be what users want. If the v1.0 interpretation produced visible artifacts (calendar events users have already seen), retroactively reinterpreting them through v1.1 is its own bug.

**Production examples.** Event-sourced systems generally. Spritely Goblins' transcript-and-replay model (see [`../spritely-ocapn/`](../spritely-ocapn/)). Some Datomic deployments. Most local-first systems where the CRDT op log is the canonical state.

**Implication for Myrhiza.** Re-replay is the **default answer** for `state-apply` upgrades. It's cheap to implement, cheap to verify (you just need to confirm replay is deterministic), and aligns with the rest of the runtime's event-log-as-source-of-truth stance. Cross-link: [`../willow/state-machine.md`](../willow/state-machine.md) for the existing replay model.

## Strategy 2: Explicit migration function

**Mechanism.** The app ships v1.1 with an additional export: `migrate_snapshot_v1_0_to_v1_1(old_bytes) -> new_bytes`. When the kernel loads a v1.0 snapshot under v1.1 code, it calls the migration function before resuming. The migration may be a passthrough (no schema change) or a real conversion (rename field, default new field, drop removed field).

**Cost model.** O(snapshot size) per upgrade, **per replica**. Migration is incremental, not full re-replay. Storage cost: the migrated snapshot replaces the old one in place; or both are kept if the kernel preserves the old snapshot for rollback. Code cost: one migration function per version-pair the app supports.

**When it's right.** Long-lived state where re-replay is too expensive. Most production databases live here: PostgreSQL `pg_upgrade`, MySQL `mysql_upgrade`, SQLite's `PRAGMA user_version` + manual ALTER statements. Cap'n Proto's evolution rules effectively make this strategy implicit — same-shape struct, just decode the v2 fields you know about.

**When it's wrong.**

- **Migrations can't span versions easily.** A v1.0 → v1.3 upgrade must either chain three pairwise migrations (`1.0→1.1→1.2→1.3`) or maintain a quadratic table of direct migrations. Both have maintenance costs.
- **Migrations can't fix bugs.** If v1.0 had a bug that put bad data in the snapshot, the v1.0→v1.1 migration must somehow detect and repair the bad data. This is brittle. Re-replay can fix bugs by reinterpreting the log; migration cannot.
- **Migration must be tested in both directions if rollback matters.** Forward-only migrations make rollback fragile.
- **For local-first systems, the migration must run on every peer.** No central authority can run a migration "once for everyone." This means migrations must be deterministic and side-effect-free — which is a smaller class of operations than central-database migrations.

**Production examples.** PostgreSQL major-version upgrades. SQLite ALTER TABLE patterns. Agoric/Endo vat upgrades use a related but distinct mechanism — `baggage` is a structured key-value handle that survives an upgrade, with the new vat-incarnation responsible for reading legacy state from baggage. Cross-link: [`../agoric-endo/persistence.md`](../agoric-endo/persistence.md).

**Implication for Myrhiza.** Explicit migration is the **right answer for state-apply apps whose log is genuinely unbounded** (chat apps with years of history, document stores). The cost is making the per-app migration story a first-class concern: the kernel needs to know which migrations exist, gate snapshot loads on their availability, and provide deterministic-helper imports the migration can use. This is non-trivial. It's a v2+ feature.

## Strategy 3: Version-and-refuse

**Mechanism.** Every snapshot carries the content-addressed hash of the v1.0 `state-apply` component that produced it. When v1.1 boots and finds a v1.0 snapshot, it does not migrate — it *refuses to load* and requires the user (or the app) to explicitly re-genesis (Strategy 1) or invoke an out-of-band migration tool.

**Cost model.** Zero migration code. Zero implicit data loss risk. The cost is operational — users hit "the app is offline until you do X" messages on upgrade.

**When it's right.** Highly sensitive state where data loss is catastrophic and silent corruption is worse than downtime. Financial ledgers, key material, audit logs. Also the right answer during early development when the schema is changing rapidly and nobody has yet committed to backward-compat guarantees.

**When it's wrong.**

- **Bad UX for any consumer-facing app.** "The app refuses to open because you upgraded" is a hostile error.
- **Doesn't scale to long-lived state with many versions.** Eventually some peer will be on v1.0 and the rest on v1.7, and version-and-refuse leaves them stranded forever.
- **Forces app authors into "lockstep upgrade" mode.** Which doesn't exist for P2P systems where you can't ship upgrades simultaneously.

**Production examples.** Most cryptocurrency wallets refuse to open old wallet files until the user explicitly migrates. Some database engines refuse to open older-version files (e.g., PostgreSQL 17 will not read a 13-format data directory). Most VM hypervisors version-pin snapshots to the runtime that produced them.

**Implication for Myrhiza.** Version-and-refuse is the **right default for v0 / v1 pre-stability development** — until Myrhiza's `state-apply` ABI itself is stable, we should refuse to load mismatched snapshots and require explicit re-genesis. Past v1.0 ABI stability, version-and-refuse remains useful as a *fallback* (when no migration is available and re-replay is too expensive, refuse rather than guess).

## Hybrid strategies

In practice, real systems combine all three:

- **Genesis + checkpoints.** Re-replay from a checkpoint (not from absolute genesis), where checkpoints are produced by a coordinated protocol on a schedule. The checkpoint itself must survive version upgrades, which means the checkpoint format needs Strategy 2.
- **Migration + replay-tail.** Migrate the snapshot to v1.1's format, then re-replay only the events between the snapshot timestamp and the present. This is the standard database-upgrade-then-resume-replication pattern.
- **Refuse-then-tools.** Refuse to load the old snapshot, but ship an offline migration tool. The user (or the app's own update flow) runs the tool, then opens the migrated snapshot in the new version.

These hybrids are usually the right production answer. Pure strategies are most useful as a *mental model* for understanding the trade-offs.

## The decision tree

For Myrhiza, we can sketch the rough decision tree:

```
Is the event log bounded (months, not years)?
├── Yes → Strategy 1 (re-replay). Default.
└── No  → Is state-apply deterministic and side-effect-free?
         ├── No  → This is a bug; fix state-apply first.
         └── Yes → Is the schema change purely structural (rename, add nullable field)?
                   ├── Yes → Strategy 2 (migration function).
                   └── No  → Strategy 3 (refuse) + out-of-band tool.
```

Two of the three legs lead to "re-replay or refuse" — pure migration is reserved for the structural-evolution sweet spot. This matches the Cambria lesson: lensing is the maximalist answer, classical-format evolution is the structural sweet spot, and re-replay is the cheap default.

## Implications for Myrhiza

1. **Make re-replay-from-genesis the v1 default.** It's the only strategy that aligns with the runtime's already-existing event-log-as-source-of-truth stance, and it requires zero per-app code.
2. **Ship version-and-refuse as the explicit-opt-in fallback.** Provide a kernel API for app developers to say "this snapshot was produced by v1.0; if you're not v1.0, refuse to load it." Useful for sensitive state.
3. **Defer explicit migration functions to v2+.** The cost of a per-app migration API is non-trivial: kernel needs migration-helper imports, sandboxing of migration code, replay-tail semantics, rollback support. Don't ship this until at least one app *demands* it. Most apps will be fine with re-replay.
4. **Cross-app schema-evolution dependencies are not a v1 problem.** If app A's events reference app B's schema, both apps must upgrade in tandem; that's a v2+ feature interleaved with app-distribution upgrade orchestration. Cross-link: [`../app-distribution/`](../app-distribution/) once that folder is mature.
5. **Treat snapshots as caches.** A snapshot is invalidated by a `state-apply` upgrade; the source of truth is the event log. This is a Myrhiza-wide architectural principle, not a schema-evolution-folder-only one.

## Sources

- PostgreSQL `pg_upgrade` documentation: https://www.postgresql.org/docs/current/pgupgrade.html
- SQLite `PRAGMA user_version` + ALTER patterns: https://www.sqlite.org/lang_altertable.html
- Agoric vat-upgrade `baggage` mechanism: [`docs/prior-art/agoric-endo/persistence.md`](../agoric-endo/persistence.md)
- Spritely Goblins transcript-and-replay: [`docs/prior-art/spritely-ocapn/`](../spritely-ocapn/)
- Cap'n Proto evolution rules: https://capnproto.org/language.html#evolving-your-protocol
- Cross-link: [`docs/prior-art/willow/open-problems.md §Snapshot portability`](../willow/open-problems.md)
- Cross-link: [`docs/prior-art/willow/state-machine.md`](../willow/state-machine.md)
- Cross-link: [`docs/prior-art/crdts/open-problems.md §5 Schema migration of on-disk bytes`](../crdts/open-problems.md)
