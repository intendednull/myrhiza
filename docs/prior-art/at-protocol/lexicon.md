**Date:** 2026-05-22
**Status:** active
**Subject:** Lexicon — AT Protocol's schema system, NSID namespacing, and strict additive evolution rules

# Lexicon — atproto's schema system

Lexicon is the schema language atproto uses to define every record, RPC endpoint, and event in the system. It's roughly *"JSON Schema with conventions for RPC, content addressing, and DNS-rooted naming."* It is also the most architecturally interesting piece of atproto's "Atmosphere" story, because it answers a question Myrhiza has not yet answered: **how do you let third parties define new typed records on a shared substrate without breaking interop?**

For Myrhiza, the relevant questions Lexicon addresses are:

- How does an app declare a snapshot schema?
- How do you version that schema without forking the data?
- How do you let a different app read a record that the first app wrote?

The answers are not all good — Lexicon's evolution rules are aggressive enough that they punt on the "two apps disagree" problem entirely — but they're a deployed reference point.

## NSID — Namespaced ID

Every Lexicon schema is identified by an **NSID** in reverse-DNS form:

```
app.bsky.feed.post
com.atproto.repo.createRecord
com.whtwnd.blog.entry
chat.bsky.message
```

The NSID has three slots:

- **Authority** — reverse-DNS form of a domain the publisher controls (`app.bsky` = `bsky.app`, `com.whtwnd` = `whtwnd.com`).
- **Subdomain segments** — application/feature naming (`feed`, `repo`, `blog`).
- **Type name** — `post`, `createRecord`, `entry` — camelCase by convention.

**Authority is rooted in DNS control.** A publisher proves they own `whtwnd.com` by serving a DNS TXT record at `_lexicon.whtwnd.com`. This is the same trick used for handle resolution. It avoids a global namespace registry — anyone with a domain can publish a Lexicon.

**Practical reality**: enforcement is light. Most consumers don't actually check the DNS proof. The model is "trust the NSID prefix, verify if you care." This is fine for the current Bluesky-dominant ecosystem and would fail badly if there were a competitive land grab on NSID prefixes.

## Schema types

Lexicon schemas are JSON files. Each declares:

- **Primary types** (the top-level definition of the schema):
  - `record` — a stored data type, written to a repository
  - `query` — a read-only RPC endpoint (HTTP GET)
  - `procedure` — a mutating RPC endpoint (HTTP POST)
  - `subscription` — a long-lived event stream (WebSocket)
  - `permission-set` — a named set of OAuth-style permissions

- **Concrete types** used inside primary types:
  - `boolean`, `integer`, `string`, `bytes`
  - `cid-link` — a CID pointing to another CBOR object
  - `blob` — opaque binary data with MIME type

- **Container types**: `array`, `object`

- **Meta types**: `ref` (reference to another Lexicon type), `union` (tagged union), `unknown` (escape hatch), `token` (enum-like marker)

- **Sub-types** for specific contexts: `params` (URL query params), `permission` (used only in `permission-set`)

A simple Lexicon record looks like:

```json
{
  "lexicon": 1,
  "id": "app.bsky.feed.post",
  "defs": {
    "main": {
      "type": "record",
      "key": "tid",
      "record": {
        "type": "object",
        "required": ["text", "createdAt"],
        "properties": {
          "text": { "type": "string", "maxLength": 3000 },
          "createdAt": { "type": "string", "format": "datetime" },
          "reply": { "type": "ref", "ref": "#replyRef" },
          "embed": { "type": "union", "refs": ["app.bsky.embed.images", "..."] }
        }
      }
    },
    "replyRef": { "type": "object", "...": "..." }
  }
}
```

## Versioning — the `lexicon: 1` constant

This is where Lexicon's evolution philosophy lives. The spec is explicit:

> *"version 1 of the Lexicon definition language"*

The `lexicon` field at the top of every schema is a fixed integer `1`. There is **no version 2** in production. Lexicon takes the position that **the language itself doesn't evolve incompatibly** — instead, individual schemas evolve under strict additive rules.

What's allowed when revising a schema:

- **Add an optional field.** Old data is valid (missing the field is OK because it's optional). New data is valid (old readers ignore unknown fields).
- **Loosen a constraint.** Extending `maxLength`, adding allowed values to a closed enum, broadening a `union` — these are backward-compatible.

What's **not allowed**:

- **Remove a non-optional field** — old data wouldn't validate against the new schema.
- **Add a required field** — old data would lack it and fail validation.
- **Change a type** — `string` → `integer` invalidates all prior data.
- **Rename a field** — same as remove + add; breaks all old data and code.
- **Tighten a constraint** — `maxLength: 3000` → `maxLength: 1000` invalidates posts that fit the old limit.

The spec frames this as:

> *"All old data must still be valid under the updated Lexicon, and new data must be valid under the old Lexicon."*

This is **strict bidirectional compatibility**. It's a much stronger constraint than typical schema evolution stories (Protobuf, Avro, JSON Schema-with-versions) and it's what lets atproto avoid a `lexicon: 2`.

## What this gets right

Three things worth borrowing:

1. **Reverse-DNS namespacing rooted in domain control.** No registry, no central authority, no token-sale. The publisher proves authority by serving a DNS or HTTPS record. This is *exactly* the shape Myrhiza wants for module signing (`distribution.md §10.2` already uses domain-rooted publisher identities via the `wpub-*` HRP).
2. **Strict additive evolution as the default.** No `lexicon: 2` is a feature, not a missing feature. It pushes the design pressure onto schema authors to think carefully about v1 rather than ship-and-version. Myrhiza's snapshot schema should default to the same discipline.
3. **NSID as the canonical key for a record's type.** A record's collection path *is* its NSID. There's no separate "type field"; the namespace is structural. This makes index-by-type queries trivial without losing namespacing.

## What this gets wrong (or punts on)

Several things to be wary of:

1. **No formal versioning means no breaking-change story.** When `app.bsky.feed.post` needs a fundamentally incompatible change (e.g., the social-graph-restructuring everyone is whispering about), the answer is: **publish a new NSID** (`app.bsky.feed.post2`). The existing data isn't migrated. Clients have to support both forever or pick one. This is the same problem Protobuf had pre-`oneof`-and-fields-numbers and is the same problem Myrhiza will have if it adopts this discipline.
2. **DNS-rooted authority is fragile.** If `bsky.app` lapses, every consumer of every `app.bsky.*` schema is in trouble. The fragility is mitigated by Bluesky controlling the schemas-that-matter, but inherent to the model.
3. **"Trust the NSID prefix"** is the deployed reality and an XSS-equivalent waiting to happen. Nothing stops an attacker from publishing a malicious schema at `app.bskyy.feed.post` (note the typo) and convincing a careless consumer to use it.
4. **`union` type tagging is by NSID, not by tag.** A `union` of `[app.bsky.embed.images, app.bsky.embed.external]` is dispatched by the `$type` field in the JSON object. This means **the JSON document carries its own type tag**, which is robust but verbose — every embedded object has a `$type` string.
5. **No structural subtyping.** Two records with identical shapes but different NSIDs are different types. There's no notion of "compatible with." Code-gen is per-NSID. This is fine for the current Atmosphere where one organization owns the schemas-that-matter; it would be painful in a more diverse ecosystem.

## Code generation

Lexicon schemas are usually consumed via code-gen. The reference Bluesky toolchain (`@atproto/lex-cli`) generates TypeScript types and client SDK methods from Lexicon files. Similar generators exist for Go (`bluesky-social/indigo/lex`), Python (`atproto-py`), Rust (atrium), and other languages.

The Lexicon spec itself is permissive about code-gen approaches:

> *"it should be possible to translate Lexicon schemas to JSON Schema or OpenAPI and use tools and libraries from those ecosystems."*

Practically, going Lexicon → JSON Schema is lossy (Lexicon's `cid-link` and `blob` types have no JSON-Schema equivalent), and going JSON Schema → Lexicon requires inventing those types. Most projects keep code-gen Lexicon-native.

## Implications for Myrhiza

Lexicon is the closest deployed prior art for Myrhiza's open question: **how does an app declare a snapshot/event schema that other apps can read?**

The relevant Myrhiza primitives that map:

- **NSID ↔ Myrhiza module identity.** Both are reverse-DNS rooted in domain control. Myrhiza's `wpub-*` HRP for publishers + module names could play the NSID role.
- **`record` primary type ↔ Myrhiza event type.** Both are "a typed JSON/CBOR object written to a content-addressed log."
- **`procedure` primary type ↔ Myrhiza state-propose intent.** Both are "a typed call that produces effects on the state."

What Myrhiza should borrow:

- The strict additive evolution discipline. *No `lexicon: 2`* is a feature.
- DNS-rooted authority. No registry, no token, no central namespace authority.
- NSID-as-collection-path. The type tag is structural.

What Myrhiza should consider differently:

- **A real versioning story for breaking changes** (Lexicon's "publish a new NSID and forget the old one" doesn't work for Myrhiza if a snapshot is supposed to outlive multiple module versions). The runner-up is a `migrate-from` declaration on the new schema with a deterministic migration function; this trades schema discipline for migration complexity.
- **Schema attestation** as a state-apply concern, not a DNS concern. A peer-symmetric setting can sign schemas with publisher keys (as Myrhiza already does for modules via `wpub-*`) and skip DNS entirely. Stronger guarantees, simpler trust model.

## Sources

- Lexicon spec: <https://atproto.com/specs/lexicon>
- NSID spec: <https://atproto.com/specs/nsid>
- atproto glossary: <https://atproto.com/guides/glossary>
- @atproto/lexicon package: <https://github.com/bluesky-social/atproto/tree/main/packages/lexicon>
- @atproto/lex-cli code generator: <https://github.com/bluesky-social/atproto/tree/main/packages/lex-cli>
- Atrium (Rust client): <https://github.com/sugyan/atrium>
