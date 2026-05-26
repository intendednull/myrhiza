**Date:** 2026-05-22
**Status:** active
**Subject:** Validates / avoid / borrow synthesis for Myrhiza's `state-apply` snapshot-portability problem. The decision file — read this before designing a versioning spec.

## Validates

These positions are confirmed by the survey. Myrhiza should encode them as default assumptions.

1. **Identity-by-number for event-schema fields is the right discipline.** Both Protobuf and Cap'n Proto independently converged on numbered fields + "any change not on the safe list is unsafe" as the operating rule. Twenty years of production validates the choice. Cross-link: [`traditional.md §Protobuf`](traditional.md), [`../capn-proto/lessons.md`](../capn-proto/lessons.md) §3.
2. **Schema evolution is a problem of *operating discipline*, not just *format design*.** Confluent's Schema Registry compatibility modes (BACKWARD / FORWARD / FULL / NONE + TRANSITIVE) are a product wrapped around Avro; the modes encode policy, not technology. Myrhiza needs the equivalent policy layer for `state-apply` ABI: explicit compatibility commitments, not just rules-on-paper. Cross-link: [`traditional.md §Confluent Schema Registry`](traditional.md).
3. **Re-replay from genesis is a credible default for event-log systems.** Spritely Goblins' transcript-and-replay works in production. Most local-first CRDT systems work this way implicitly. For Myrhiza's deterministic `state-apply`, this is the simplest correct strategy. Cross-link: [`migration-strategies.md §Strategy 1`](migration-strategies.md).
4. **Semantic evolution is a hard wall that no current tool crosses.** Cambria's stall + the 2023 "Live & Local Schema Change" challenge problems both acknowledge this. The right response is to *not pretend it's solved* — distinguish semantic from structural in the ABI spec, and make semantic changes an explicit protocol event with user-visible UX. Cross-link: [`open-problems.md §Semantic vs structural`](open-problems.md).
5. **Snapshots are caches; the event log is the source of truth.** This is the position that makes re-replay-from-genesis work. Any design that treats the snapshot as primary state is taking on a much harder migration problem. Cross-link: [`migration-strategies.md`](migration-strategies.md).

## Avoid

These approaches were tried in research or production and have known failure modes Myrhiza should not repeat.

1. **Do NOT ship bidirectional lenses as a v1 feature.** Cambria stalled — well-funded research lab, careful theoretical foundation, three accomplished authors — because the *authoring* cost is high and the *maintenance* cost is high. The theory works; the engineering doesn't. Myrhiza has no chance of succeeding where Cambria didn't. Cross-link: [`cambria.md §Why it stalled`](cambria.md).
2. **Do NOT assume postcard handles schema evolution.** Postcard's documentation is explicit: *"Backwards/forwards compatibility between revisions of a postcard schema are considered outside of the scope of the postcard wire format, and must be considered by the end users."* If we use postcard for snapshot bytes (Willow already does), we own the versioning layer entirely. Cross-link: [`traditional.md §Postcard`](traditional.md).
3. **Do NOT use position-based encoding for the event log.** Postcard is position-based: adding a field anywhere except the end breaks compatibility. Events live for years; position-based encoding turns every additive change into a flag-day. Use identity-by-number for events. Snapshots are different (they're rebuildable from the log; their format can be more fragile).
4. **Do NOT design schema-evolution policy in app-developer-land.** Confluent's Schema Registry exists because organizational discipline-alone is not enough. The kernel should enforce compatibility modes per app, not trust app authors to remember the rules.
5. **Do NOT conflate the "library upgrade" problem with the "schema evolution" problem.** Automerge's 1.x → 2.0 → 3.0 wire-format rewrites are a library-upgrade problem (cross-link: [`../crdts/open-problems.md §5`](../crdts/open-problems.md)); per-app schema changes are a different problem. Myrhiza needs both, but they need separate solutions.
6. **Do NOT ignore the cross-app schema dependency problem just because we don't have a solution yet.** It will appear the moment two apps reference each other's events. Document it as a v2+ open problem, refuse cross-major reads in v1, don't paper over it. Cross-link: [`open-problems.md §Cross-app schema dependencies`](open-problems.md).
7. **Do NOT design the ABI assuming "rename" is always safe.** Most formats permit source-level rename if the wire identity (tag, ordinal, position) is preserved. But a rename that changes the *meaning* of the field is a semantic change masquerading as a structural one. Per #4 in Validates: distinguish, don't conflate.

## Borrow

These specific mechanisms and disciplines from prior systems map cleanly onto Myrhiza's needs.

1. **Borrow Cap'n Proto's explicit safe/unsafe rule list.** From [`traditional.md §Cap'n Proto`](traditional.md). Document the equivalent for `state-apply` event-schema evolution: every allowed change-class enumerated, *"any change not on the list is unsafe"* as the default. Lift the rules-list shape (numbered, terse, explicit) directly.
2. **Borrow Avro's writer-schema-travels-with-data idea for snapshot-format evolution.** Each snapshot carries the content-addressed hash of the `state-apply` component that produced it. The loading code can either decode-in-place (if hashes match) or refuse-and-replay (if they don't). Cross-link: [`migration-strategies.md §Strategy 3 Version-and-refuse`](migration-strategies.md). This is Myrhiza-shaped Avro — schema-identity-as-content-hash instead of schema-ID-as-registry-key.
3. **Borrow Cambria's lens-combinator vocabulary** if and when Myrhiza ever builds a per-event migration UI. `addProperty / removeProperty / renameProperty / hoistProperty / plungeProperty / convertValue` is a tested vocabulary. The *mechanism* underneath can be one-directional (cheaper) rather than bidirectional (Cambria's overcommitment). Cross-link: [`cambria.md §What lens operations exist`](cambria.md).
4. **Borrow Confluent's compatibility-mode taxonomy** — BACKWARD / FORWARD / FULL / NONE + TRANSITIVE — for `state-apply` ABI declarations. Each app declares its ABI's compatibility commitment per major version. Cross-link: [`traditional.md §Confluent Schema Registry`](traditional.md). This is the operating-discipline layer that makes structural evolution predictable.
5. **Borrow the Agoric `baggage` shape if explicit migration becomes necessary.** A persistent, content-addressed key-value handle that survives `state-apply`-instance replacement. The migration function reads legacy data from baggage, writes the new shape, and the kernel atomically swaps the snapshot. Cross-link: [`../agoric-endo/persistence.md`](../agoric-endo/persistence.md), [`../agoric-endo/lessons.md`](../agoric-endo/lessons.md) §2 ("What is our baggage analog?").
6. **Borrow Postcard's `serde(default)` discipline** for the *easiest* case: snapshots with strictly additive structural evolution. If the only change is "added one new field at the end with a serde default," postcard reads old snapshots with the new type definition correctly. Use this for the structural-additive sweet spot; everything else needs a different strategy. Cross-link: [`traditional.md §Postcard`](traditional.md).
7. **Borrow the re-replay-from-genesis stance** as the canonical default. Spritely Goblins ships transcript-and-replay; CRDT systems do the equivalent. For Myrhiza's deterministic `state-apply`, this is the simplest correct strategy and aligns with the existing event-log-as-source-of-truth architecture. Cross-link: [`migration-strategies.md §Strategy 1`](migration-strategies.md).

## What this folder won't help with

A short list of decisions outside scope:

- **Specific WIT-level signature evolution rules** — the WASM Component Model side of the question. Cross-link: [`../wasm-component-model/`](../wasm-component-model/).
- **Capability revocation on type change** — see [`open-problems.md §Capability boundaries`](open-problems.md). Likely needs its own folder or spec when it becomes a v1.5+ concern.
- **App-distribution / installation-side upgrade orchestration** — when an app ships v1.1, how does the runtime decide whether to apply it to a given snapshot? That's app-distribution-folder territory.
- **CRDT-library-version-bump migrations** — see [`../crdts/open-problems.md §5`](../crdts/open-problems.md). Same family, different sub-problem.

## The single most important takeaway

If you read only one sentence from this folder, read this:

> **Cambria asked the maximalist question and stalled; the production answer is to make schema changes painfully explicit, identity-by-number, with re-replay as the default and one-shot migration as the escape hatch.**

That's the design center. Everything else in this folder is evidence for that conclusion.

## Sources

- All evidence files in this folder.
- Cross-link: [`docs/prior-art/crdts/open-problems.md`](../crdts/open-problems.md)
- Cross-link: [`docs/prior-art/willow/open-problems.md §Snapshot portability`](../willow/open-problems.md)
- Cross-link: [`docs/prior-art/agoric-endo/persistence.md`](../agoric-endo/persistence.md)
- Cross-link: [`docs/prior-art/agoric-endo/lessons.md`](../agoric-endo/lessons.md)
- Cross-link: [`docs/prior-art/capn-proto/capnp.md`](../capn-proto/capnp.md)
- Cross-link: [`docs/prior-art/capn-proto/lessons.md`](../capn-proto/lessons.md) §3
