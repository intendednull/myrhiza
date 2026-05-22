**Date:** 2026-05-22
**Status:** active
**Subject:** Production wire-format schema-evolution disciplines — Protocol Buffers, Cap'n Proto, Apache Avro, Postcard. Four formats, four different positions on the same trade-off space.

## The trade-off space

All four formats answer the same question — *given schema v1 was used to encode bytes, what are the safe changes to make in v2 such that old + new code can still read each other's data?* — but they make different trade-offs along three axes:

1. **Self-describing vs schema-required.** Self-describing formats embed enough metadata that a decoder without the schema can still parse. Schema-required formats produce smaller bytes but the decoder must have the matching (or compatible) schema.
2. **Identity by name vs identity by number.** Field changes are tracked by stable numeric tags (Protobuf, Cap'n Proto) or by name with optional aliases (Avro).
3. **Resolution time.** Compatibility is enforced at *schema-compile time* (Cap'n Proto), at *encode time* (Protobuf via writer hand-off), at *decode time via writer-schema lookup* (Avro), or *not at all* (Postcard).

Cross-link: [`migration-strategies.md`](migration-strategies.md) for how these wire-format choices interact with Myrhiza's broader migration strategy.

## Protocol Buffers

Google's wire format, used at Google scale since approximately 2001. Open-sourced 2008. Currently on proto3.

**Identity model.** Every field has a numeric **tag** declared in the `.proto` file. The tag is the wire-level identifier; the field name is source-level only.

**Safe changes** (per [protobuf.dev/programming-guides/proto3/#updating](https://protobuf.dev/programming-guides/proto3/#updating)):

- *"Adding new fields is safe."* Old code ignores unknown tags; new code uses default values when the field is absent.
- *"Removing fields is safe"* — provided the deleted number is added to a `reserved` list.
- *"Adding additional values to an enum is safe."*
- *"Changing a single explicit presence field or extension into a member of a new `oneof` is safe."* Same in reverse for single-field oneofs.
- Type conversions like `int32 → int64` are wire-compatible but lossy on the narrowing direction. Use carefully.

**Unsafe changes:**

- *"Changing field numbers for any existing field is not safe."*
- *"Moving fields into an existing `oneof` is not safe."*
- Renumbering anything. Ever.

**Key discipline:** *"Field numbers **should never be reused**. Never take a field number out of the reserved list for reuse with a new field definition."* This is the load-bearing rule. Violating it produces wire-level mis-parses where v2 writes `tag=7 → string` while v1 still expects `tag=7 → int32`. Bytes are interpreted as raw garbage.

**Versioning model.** No explicit version field. The schema *is* the version, and the rules-as-a-set are the version-compatibility guarantee. A `.proto` file is a contract; the rules above are the operating-discipline.

## Cap'n Proto

Kenton Varda's "infinitely faster" alternative to Protobuf. Same author. Started inside Google as Varda's escape hatch from Protobuf; moved to Sandstorm (2014); now stewarded primarily inside Cloudflare. Cross-link: [`../capn-proto/capnp.md`](../capn-proto/capnp.md) for the broader Cap'n Proto story.

**Identity model.** Every struct has a 64-bit **type ID** (auto-generated or explicit). Every field has an **ordinal number** within its struct. Wire format references both.

**Safe changes** (per [capnproto.org/language.html#evolving-your-protocol](https://capnproto.org/language.html#evolving-your-protocol)):

- *"New types, constants, and aliases can be added anywhere."*
- *"New fields, enumerants, and methods may be added to structs, enums, and interfaces, respectively, as long as each new member's number is larger than all previous members."*
- *"New parameters may be added to a method. The new parameters must be added to the end of the parameter list and must have default values."*
- *"Members can be re-arranged in the source code, so long as their numbers stay the same."*
- *"Any symbolic name can be changed, as long as the type ID / ordinal numbers stay the same."* (i.e. rename is safe.)
- *"Type definitions can be moved to different scopes, as long as the type ID is declared explicitly."*
- *"A field can be moved into a group or a union, as long as the group/union and all other fields within it are new."*
- Generics: non-generic types may become generic; new type parameters may be added.

**Unsafe changes:**

- *"You cannot change a field, method, or enumerant's number."*
- *"You cannot change a field or method parameter's type or default value."*
- *"You cannot change a type's ID."*
- *"You cannot change the name of a type that doesn't have an explicit ID."*
- *"You cannot move a type to a different scope or file unless it has an explicit ID."*
- *"You cannot move an existing field into or out of an existing union, nor can you form a new union containing more than one existing field."*

**Key discipline.** Like Protobuf, identity-by-number plus an explicit safe-change list. Cap'n Proto's list is slightly more permissive (renaming is explicitly fine; "any change not on the safe list is unsafe") and explicitly tracks **type IDs** as well as field ordinals, which makes cross-file refactoring tractable.

**Versioning model.** Same as Protobuf — schema *is* the version. Type IDs give an additional invariant: a `.capnp` struct keeps its identity across files and refactors.

## Apache Avro

Apache project, originating in the Hadoop ecosystem circa 2009. Heavily used by Confluent's Kafka Schema Registry. Distinctive choice: **the writer's schema travels with the data** (or via a side-channel schema registry).

**Identity model.** Fields identified by **name**, with optional **aliases**. Types identified by fully-qualified name.

**Resolution rules** (per [avro.apache.org/docs/1.11.1/specification/](https://avro.apache.org/docs/1.11.1/specification/)):

- *"Both schemas are records with the same (unqualified) name."*
- *"Fields can appear in different orders; they're matched by name rather than position."*
- *"Values for fields in the writer's record not present in the reader's schema are simply ignored."* — i.e. reader can drop fields it doesn't know.
- *"When the reader's schema contains a field absent in the writer's schema, the reader uses that field's default value if one exists."*
- *"If the reader requires a field with no default and the writer lacks it, an error occurs."*
- *"The writer's schema may be promoted to the reader's as follows: int is promotable to long, float, or double."*
- *"If the writer's symbol is not present in the reader's enum and the reader has a default value, then that value is used."*

**Aliases.** Avro's escape hatch for renames. Reader schema can list aliases for record names and field names; the resolver uses these to map writer's-name → reader's-name during decode.

**Confluent Schema Registry compatibility modes** (per [docs.confluent.io/.../schema-evolution.html](https://docs.confluent.io/platform/current/schema-registry/fundamentals/schema-evolution.html)):

| Mode | Meaning |
|---|---|
| **NONE** | No compatibility checks. Migrate all clients simultaneously. |
| **BACKWARD** (default) | New consumers can read data from immediately previous schema. Add optional fields or remove fields. |
| **FORWARD** | Old consumers can read data from new schema. Remove optional fields or add fields. |
| **FULL** | Both backward and forward. Only add or remove optional fields. |
| **BACKWARD_TRANSITIVE** | Backward compatible with *all* previous versions, not just one. |
| **FORWARD_TRANSITIVE** | Forward compatible with all previous versions. |
| **FULL_TRANSITIVE** | Both transitive. |

This is the *operating-discipline product* on top of Avro's *technical rules*. Avro itself defines what resolution does; Schema Registry layers compatibility-mode policy on top so producers and consumers can be deployed with predictable safety properties.

**Versioning model.** Each schema in the registry has a version number. Producer attaches a small schema-ID to each record on the wire; consumer fetches that schema from the registry to perform resolution. Resolution is decode-time, not encode-time.

## Postcard

James Munns's `#![no_std]` Serde serializer for Rust. Apache-2.0 + MIT. Latest 1.1.3 (2025-07-24). Used in embedded Rust + Willow's encoder. Cross-link: Willow uses postcard with sorted-collection helpers; see [`../willow/`](../willow/) for the Willow-side story.

**Identity model.** None. Postcard uses **Serde**'s data model: structs and enums in Rust source code define the schema, and postcard encodes them position-positionally on the wire. Field names are not on the wire. Enum discriminants are.

**Wire format properties** (per [postcard.jamesmunns.com/wire-format](https://postcard.jamesmunns.com/wire-format)):

- Varint encoding for integers.
- Length-prefixed sequences and strings (length is itself a varint).
- Raw little-endian for floats (4 bytes f32 / 8 bytes f64).
- Tagged unions: varint discriminant followed by associated data.
- **Not self-describing.** *"Postcard is **NOT** considered a 'Self Describing Format', meaning that users…are expected to have a mutual understanding of the encoded data."*

**Schema evolution position.** *"Backwards/forwards compatibility between revisions of a postcard schema are considered outside of the scope of the postcard wire format, and must be considered by the end users."*

That is the entire statement. There is no field-tagging discipline analogous to Protobuf's or Cap'n Proto's, no aliases analogous to Avro's. The wire format is **stable as a format** (since v1.0.0, any postcard 1.x decoder can read any postcard 1.x encoder's bytes for compatible Rust types) but compatibility *between Rust type versions* is the user's problem.

**Pragmatic options for evolving postcard-encoded data:**

1. **Outer envelope versioning** — wrap every payload in a `(version: u32, payload: ...)` struct; switch on the version byte.
2. **Newtype around the Rust type** — bump the major version of the crate defining the type, and require all peers to upgrade in lockstep.
3. **`#[serde(default)]`** — mark new fields with Serde's default attribute so old encoders' bytes deserialize with defaults for missing fields. *This is fragile* because postcard is position-sensitive: adding a field at the end is the only safe shape, and you need to be careful about enum discriminant numbering.
4. **Re-encode on upgrade** — write a migration that reads v1 bytes with the v1 type definition and re-encodes as v2. Simple but requires both type definitions in the migration binary.

**Versioning model.** Library-version-stable wire format; user-side type-version-stable nothing. Postcard is closer to "raw structural encoding" than to "schema-evolution-aware format."

## Side-by-side

| Feature | Protobuf | Cap'n Proto | Avro | Postcard |
|---|---|---|---|---|
| Identity model | Field tag | Type ID + ordinal | Field name + aliases | Position |
| Self-describing | No | No | No (writer schema travels) | No |
| Rename safe? | Yes (source-only) | Yes (source-only) | Yes (via alias) | N/A |
| Remove field safe? | Yes (reserved keyword) | Effectively yes | Yes (reader ignores) | Unsafe |
| Add field safe? | Yes | Yes (number > prior) | Yes (with default) | Position-end + `serde(default)` |
| Change type safe? | Limited (int widening) | No | Limited (int/float promotion) | No |
| Reuse tag/ordinal | Forbidden | Forbidden | N/A | N/A |
| Default values | Implicit zero | Explicit per field | Explicit per field | `serde(default)` |
| Versioning UI | None (schema = version) | None (schema = version) | Registry + version numbers | User-defined envelope |
| Resolution time | Encode + decode | Encode + decode | Decode (writer schema lookup) | None |
| Used in production by | Google, gRPC, billions of services | Cloudflare, Sandstorm | Hadoop, Kafka, Confluent | Embedded Rust, Willow |

## Pattern observations

- **All four formats refuse to handle semantic evolution.** A field renamed because its *meaning* changed is not a rename any of these formats can express. The "rename" knob is for re-labeling the *same* field. See [`open-problems.md §Semantic vs structural`](open-problems.md).
- **Identity-by-number (Protobuf, Cap'n Proto) gives the cleanest evolution story** at the cost of source-level fragility (forgetting to reserve a deleted tag is a foot-cannon).
- **Identity-by-name (Avro) is more flexible** but pays in: (a) writer-schema-must-be-fetchable (which is a runtime dependency), (b) larger wire bytes when schemas don't deduplicate, (c) aliases-as-evolution-mechanism only works if you remember to add the alias before the rename ships to production.
- **Postcard is the outlier.** It explicitly punts on schema evolution. That's defensible for its use case (embedded systems where the producer and consumer are usually the same binary across an OTA update) but means anyone using postcard for *durable* storage must layer their own versioning on top.

## Implications for Myrhiza

We will likely use postcard or a similar Serde-backed format inside `state-apply` snapshots (Willow already does). The lesson here is that **postcard's "your problem" stance is honest, but it is our problem to solve**. We cannot pick postcard and call our snapshot-evolution story done.

Three concrete recommendations:

1. **For event-log encoding**, prefer identity-by-number (Protobuf-style or Cap'n Proto-style) over position-based encoding. Events are durable and read by future versions; the cost of breaking the wire format is high.
2. **For snapshot encoding**, postcard is fine for the bytes-on-disk format *if* we own a wrapper layer that handles versioning. See [`migration-strategies.md`](migration-strategies.md).
3. **Borrow Cap'n Proto's safe/unsafe explicit list** for our own ABI documentation. *"Any change not on the safe list is unsafe"* is a sound default and a discipline we should adopt for `state-apply` ABI evolution. See [`../capn-proto/lessons.md`](../capn-proto/lessons.md) lesson 3.

## Sources

- Protobuf proto3 updating guide: https://protobuf.dev/programming-guides/proto3/#updating
- Cap'n Proto evolution rules: https://capnproto.org/language.html#evolving-your-protocol
- Apache Avro 1.11.1 specification: https://avro.apache.org/docs/1.11.1/specification/
- Confluent Schema Registry compatibility modes: https://docs.confluent.io/platform/current/schema-registry/fundamentals/schema-evolution.html
- Postcard repository: https://github.com/jamesmunns/postcard
- Postcard wire-format specification: https://postcard.jamesmunns.com/wire-format
- Cross-link: [`docs/prior-art/capn-proto/capnp.md`](../capn-proto/capnp.md)
- Cross-link: [`docs/prior-art/capn-proto/lessons.md`](../capn-proto/lessons.md) §3
