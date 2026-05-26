**Date:** 2026-05-22
**Status:** active
**Subject:** Glossary for the schema-evolution corpus. Terms that get crossed across files.

## Structural

A schema change that affects the *shape* of the data — field name, field type, field presence, field ordering. Examples: rename a field, change `int32` to `int64`, add a nullable field, remove a deprecated field. Structural changes are what traditional schema-evolution formats handle. See [`open-problems.md §Semantic vs structural`](open-problems.md).

## Semantic

A schema change that affects the *meaning* of the data without changing its shape. Example: the field `due_date` is still a timestamp, but in v1.1 it means "auto-archive time" instead of v1.0's "becomes-overdue time." Semantic changes are **not** handled by any schema-evolution tool surveyed in this folder.

## Lens

A pair of functions `(forward, backward)` that maps data between two schemas in both directions. Lineage: Foster et al. TOPLAS 2007 ("Combinators for Bidirectional Tree Transformations"); Hofmann/Pierce/Wagner POPL 2012 ("Edit Lenses"). Cambria builds on Edit Lenses by operating on *patches* (CRDT ops) rather than whole documents. See [`cambria.md`](cambria.md).

## Bidirectional transformation (BX)

The academic name for the field that studies lenses, view-update problems, and related two-way data-translation theory. Cambria is the local-first / CRDT-flavored sub-area.

## Writer schema / reader schema

Avro terminology. The **writer schema** is the schema used to encode the bytes; the **reader schema** is the schema the decoder expects. Resolution rules map writer-shape to reader-shape at decode time. See [`traditional.md §Avro`](traditional.md).

## Field number / tag / ordinal

The numeric identifier for a field in identity-by-number formats. Protobuf calls it a "tag." Cap'n Proto calls it an "ordinal." Both formats forbid reusing a number after deletion. See [`traditional.md`](traditional.md).

## Type ID

Cap'n Proto-specific. A 64-bit identifier for a struct or interface, declared explicitly or auto-generated from the struct's fully-qualified name. Stable across renames and scope moves.

## Aliases

Avro's mechanism for renames. Reader schema lists alternate names for a record/field; resolver uses them to map writer's-name to reader's-name during decode. See [`traditional.md §Avro`](traditional.md).

## Compatibility mode

Confluent Schema Registry policy classes — BACKWARD, FORWARD, FULL, NONE — plus their TRANSITIVE variants. Encode what kinds of schema changes the producer is committing to. See [`traditional.md §Confluent Schema Registry`](traditional.md).

## Re-replay from genesis

Migration strategy: discard cached snapshot, replay full event log through the new `state-apply` code. Cheapest implementation; bounded by log size. See [`migration-strategies.md §Strategy 1`](migration-strategies.md).

## Migration function

Migration strategy: ship an explicit `migrate_snapshot(old) -> new` function that converts in-place. Used by PostgreSQL `pg_upgrade`, most production databases. See [`migration-strategies.md §Strategy 2`](migration-strategies.md).

## Version-and-refuse

Migration strategy: snapshot is content-pinned to the component hash that produced it; mismatched hashes cause a refuse-to-load. Cryptocurrency wallets, sensitive-data systems. See [`migration-strategies.md §Strategy 3`](migration-strategies.md).

## Reserved (Protobuf)

The keyword used to mark a deleted field number as off-limits for reuse. *"Field numbers should never be reused. Never take a field number out of the reserved list."* See [`traditional.md §Protocol Buffers`](traditional.md).

## Self-describing format

A wire format that embeds enough metadata for a decoder to parse without an external schema. JSON, CBOR are self-describing. Protobuf, Cap'n Proto, Postcard are not. Avro is partially self-describing in the registry-fetched-schema sense.

## Schema-pinned snapshot

A snapshot whose loadability depends on the producing component's content hash. Either loads cleanly (hash matches), or triggers a migration / re-replay / refuse decision. See [`migration-strategies.md`](migration-strategies.md).

## `state-apply`

Myrhiza-specific. The pure-function component profile that materializes events into state. Determinism is mandatory. See `CLAUDE.md` and [`../willow/state-machine.md`](../willow/state-machine.md). Schema-evolution discipline applies primarily to `state-apply`'s input event schema and output snapshot format.

## ABI break

A change to a component's interface (WIT types, exported function signatures, host imports) that requires recompilation or re-linking of callers. In Myrhiza's terms: any change to a `state-apply` component's event-schema or capability surface that isn't on the explicit safe-change list. See [`lessons.md`](lessons.md).

## Sources

- This file is a glossary; defining sources live in the linked files.
- Cambria essay: https://www.inkandswitch.com/cambria/
- Foster et al. TOPLAS 2007: https://www.cis.upenn.edu/~bcpierce/papers/index.shtml
- Protobuf evolution rules: https://protobuf.dev/programming-guides/proto3/#updating
- Cap'n Proto evolution rules: https://capnproto.org/language.html#evolving-your-protocol
- Avro 1.11.1 specification: https://avro.apache.org/docs/1.11.1/specification/
- Confluent compatibility modes: https://docs.confluent.io/platform/current/schema-registry/fundamentals/schema-evolution.html
- Postcard wire format: https://postcard.jamesmunns.com/wire-format
