**Date:** 2026-05-22
**Status:** active
**Subject:** What schema evolution structurally doesn't solve. The semantic-vs-structural distinction, cross-app schema dependencies, and the long tail of cases where no current tool helps.

## The biggest unsolved problem: semantic vs structural evolution

Every tool in this folder — Cambria, Protobuf, Cap'n Proto, Avro, Postcard — handles **structural** schema evolution: rename a field, change a type, add a nullable field. None of them handle **semantic** evolution: the field is still named `due_date` and is still a timestamp, but its meaning changed.

Concrete example, from the local-first calendar app domain:

- **v1.0.** `due_date: Timestamp` means "the moment the task becomes overdue and turns red in the UI."
- **v1.1.** `due_date: Timestamp` means "the moment the task is automatically archived and hidden from the default view."

The field type didn't change. The field name didn't change. The wire bytes round-trip identically. But the same byte sequence represents two different things, and any v1.0 peer that observes a v1.1-set due-date will misinterpret it (and vice versa).

This is the **hard wall**. The Cambria essay acknowledges it in passing; the Edwards/Petricek/van der Storm "Live & Local Schema Change" challenge problems make it the central problem. Nobody has solved it. The general shape of any solution requires either:

- **A coordinated protocol cutover** — every peer agrees to switch interpretation at a specified point in causal history. Requires consensus, which CRDT systems are usually trying to avoid.
- **Explicit semantic versioning of the field itself** — `due_date_v1` and `due_date_v2` are different fields, and apps maintain both. Verbose, awkward, but tractable.
- **Per-event semantic tagging** — each event carries metadata saying "I was created by an app that interpreted due_date as semantics-X." Decoders use this to interpret. Adds wire overhead and is fragile against forgery.

**Implication for Myrhiza.** When designing the `state-apply` ABI evolution rules, distinguish structural from semantic changes explicitly. Structural changes are routine; semantic changes are a *protocol-level event* that the app developer must handle deliberately (and probably with explicit user-visible upgrade UX). Don't pretend any tool handles this; it doesn't.

## Cross-app schema dependencies

If app A's `state-apply` produces events that app B's `state-apply` reads (e.g. a notification system reading calendar events), then upgrading app A's schema breaks app B until app B also upgrades. The single-app schema-evolution problem becomes a graph problem.

In practice this is handled by:

- **Strict cross-app interfaces.** App A's exported event schema is a stable contract; bumping its major version is a kernel-level coordination event.
- **Adapter components.** An adapter component subscribes to app A's events and emits a stable-shape projection that app B reads.
- **Version-pinned event subscriptions.** App B subscribes to a specific version of app A's event schema; if app A upgrades past that version, the subscription is severed.

All three are real production patterns. None is fully automated. Cross-link to the future folder on app-distribution / capability versioning when it lands.

## Schema evolution under Byzantine peers

The classical formats (Protobuf, Cap'n Proto, Avro) assume the peer producing the bytes is following the schema-evolution rules. A Byzantine peer can produce wire bytes that:

- Use a reserved tag number to inject data your code doesn't expect.
- Lie about the writer schema in Avro's "schema travels with data" model.
- Encode a string where the schema declares an int, exploiting decoder lenience.

CRDT-aware lensing (Cambria) inherits this problem: nothing in Cambria's lens definition verifies that the producing peer actually used the lens correctly. A malicious peer can produce v2 patches that don't follow any consistent forward direction.

For Myrhiza, this is a kernel-level concern. The `state-apply` component's input-event validation is the line of defense, not the wire format. Cross-link: [`../crdts/open-problems.md §2 Authority`](../crdts/open-problems.md).

## Long-running offline peers across multiple schema versions

A peer that has been offline for a year wakes up running v1.0 against a network that's at v1.7. Even with the best schema-evolution tooling:

- A six-step migration chain (v1.0→v1.1→…→v1.7) might be present locally, but executing it requires the v1.0 peer to first download all six migrations. Bootstrap problem.
- Re-replay from genesis works in theory but the log might be too large to fetch over a slow link.
- The v1.0 peer might emit events using v1.0 semantics that no other peer can process anymore.

Workable strategies are gateway-flavored: a more-up-to-date peer accepts the v1.0 peer's events, translates them through the migration chain to v1.7, and re-emits them. This makes the gateway a trust point. Avoiding the trust-point is hard.

Cross-link: [`../crdts/open-problems.md §6 Long-running collaboration with intermittent peers`](../crdts/open-problems.md).

## Schema evolution of indices and derived state

A `state-apply` component might materialize the event log into both authoritative state AND derived indices (full-text search, sort orders, aggregate counts). When the schema changes:

- The authoritative state needs migration (see [`migration-strategies.md`](migration-strategies.md)).
- **The indices also need migration**, and they're often larger than the authoritative state, and they're often computed lazily.
- A migration that updates the authoritative state but leaves the indices stale produces inconsistent reads.

The traditional database answer is "rebuild indices on schema change." That works if rebuild is fast and rebuild can run in the background. For an event-sourced system this is just "re-replay produces fresh indices for free," which is a point in re-replay's favor. But for systems with large derived state (e.g., a search index over a million documents), re-replay is also slow.

Open problem: how to schema-evolve derived state without either (a) re-replaying everything or (b) maintaining migration code for every index separately.

## Schema evolution of capability boundaries

If `state-apply` exports a capability (a function or resource handle) to another component, that capability has a *type*. Evolving the type evolves the capability's wire shape. Two cases:

- **Holders of the old-typed capability can still call it after the type evolves.** Requires backward-compat at the host-call layer; possible in Component Model with careful lifting/lowering.
- **Holders of the old-typed capability stop working.** Effectively a revocation; the capability is dropped on type evolution.

Today, WASM Component Model's answer is closer to the second: changing the WIT signature of an exported function changes the component's interface ID, which makes the new component a different interface from the perspective of the importer. Effectively, every interface change is a revocation. Cross-link: [`../wasm-component-model/`](../wasm-component-model/).

Open problem: how to evolve capability types without forcing all holders to re-import. Cap'n Proto's RPC layer has thought about this (interfaces have type IDs; new methods can be added at the end); the Component Model has not yet.

## What "evolution" doesn't mean

A few non-problems that are sometimes lumped under "schema evolution" but are actually different:

- **Schema lookup at runtime.** Avro's "writer schema travels with data" pattern is a *distribution* problem, not an evolution problem. The schema doesn't change; the decoder needs to find it. Solved by schema registries.
- **Library version bumps.** When Automerge ships 3.0 with a new wire format, that's a *library upgrade* problem, not a schema-evolution problem in the per-app sense. Cross-link: [`../crdts/open-problems.md §5`](../crdts/open-problems.md).
- **Code refactoring.** Renaming a field in source code without changing its wire identity (Protobuf field number stays the same, Cap'n Proto ordinal stays the same) is a no-op at the schema-evolution level. Important not to confuse this with rename-as-evolution.

## Implications for Myrhiza

1. **Document the structural-vs-semantic distinction explicitly in the `state-apply` ABI spec.** Structural changes follow rules; semantic changes are a protocol event with mandatory user-visible upgrade UX.
2. **Cross-app schema evolution is a v2+ problem.** Don't try to design it pre-shipping. v1 should require apps to declare a major version on every exported event schema and refuse cross-major reads.
3. **Capability-type evolution is not solved by the Component Model today.** If Myrhiza needs cap-passing across upgrade boundaries, design a kernel-level adapter layer; don't expect the substrate to give it to us.
4. **Don't over-engineer migrations for "what if a peer is two years stale."** Solve the 80% case (peers stale by hours or days). Pathological staleness is a re-genesis problem.

## Sources

- Live & Local Schema Change: Challenge Problems (Edwards/Petricek/van der Storm, LIVE@SPLASH 2023): https://arxiv.org/abs/2309.11406
- Cambria essay's "Limitations" section: https://www.inkandswitch.com/cambria/
- Cross-link: [`docs/prior-art/crdts/open-problems.md`](../crdts/open-problems.md)
- Cross-link: [`docs/prior-art/willow/open-problems.md`](../willow/open-problems.md)
- Cross-link: [`docs/prior-art/wasm-component-model/`](../wasm-component-model/)
- Cross-link: [`docs/prior-art/agoric-endo/persistence.md`](../agoric-endo/persistence.md)
