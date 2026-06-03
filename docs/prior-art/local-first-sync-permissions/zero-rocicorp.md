**Date:** 2026-05-29
**Status:** active
**Subject:** Zero (Rocicorp) — server-authoritative sync with query-filter reads and server-mutator writes (the trusted-middlebox shape Myrhiza rejects)

# Zero (Rocicorp)

Zero is a general-purpose sync engine from **Rocicorp**, the team behind
Replicache and Reflect. Its tagline is "zero milliseconds" — instant optimistic
UI with a server reconciling the canonical state. It reached **1.0 "First Stable
Release"**; npm `@rocicorp/zero` latest is **1.5.0**, **Apache-2.0**. It is the
cleanest example of the paradigm Myrhiza rejects: **the server is the authority.**

## Architecture

- **zero-cache**: a server process that replicates the upstream Postgres
  (logical replication) and serves clients filtered, query-scoped data.
- Clients hold a local SQLite-like store, run **ZQL** queries, and apply
  optimistic mutations.
- The **server is authoritative**: the server mutator always takes precedence
  over the client mutator. The canonical database lives behind the server; the
  client's view is a cache.

## Permissions — two generations, both server-trusting

**Generation 1 (now deprecated): declarative RLS-style rules.** Permissions were
ZQL expressions compiled at build time into a JSON blob stored in a
`{app}.permissions` table and replicated to zero-cache. Read rules were row
filters (`zql.post.where('authorID', ctx.id)`); write rules were allow/deny
predicates. Zero's docs now label these **"RLS Permissions (Deprecated)."**

**Generation 2 (current): custom mutators.** The recommended model puts your
server on the write path: a **custom mutator** is an arbitrary TypeScript
function running server-side that validates and applies the write. Zero's docs:
*"Because custom mutators are just arbitrary TypeScript functions, there is no
need for a special permissions system … you won't use Zero's write permissions
when you use custom mutators."* Reads are still server-enforced filters via a
`Context` object carrying the server-verified `userID`.

Either way, **enforcement is on the server**. The docs are explicit: *"Zero does
not have (or need) a first-class permission system like RLS. Instead, you
implement permissions by authenticating the user in your queries and mutators
endpoints."* The trust boundary is the server, and *"the server is trusted."*

## How JWT auth feeds in

Clients send a JWT (cookie or `Authorization: Bearer`). Zero forwards the token
to your query/mutator endpoints; your server validates it, extracts identity, and
builds the `Context`. Zero itself doesn't parse claims — your trusted endpoint
does. Compromise the server (or its JWT signing key) and the entire authority
model collapses. Contrast cojson, where a compromised relay can only withhold or
reorder, never forge an accepted write ([jazz-cojson.md](jazz-cojson.md)).

## Why Myrhiza rejects this shape

Myrhiza has no trusted server: a Myrhiza relay is a dumb iroh topic bridge, and
every peer re-derives validity from the signed DAG
([../../specs/2026-05-09-myrhiza-master-design/networking.md](../../specs/2026-05-09-myrhiza-master-design/networking.md) §11.4,
[capabilities.md](../../specs/2026-05-09-myrhiza-master-design/capabilities.md) §7.5
"a network adversary that controls the relay infrastructure" is out of the
defense surface precisely because relays adjudicate nothing). Zero's
server-mutator model is the antithesis of `state-apply` running identically on
every peer. See [paradigm-contrast.md](paradigm-contrast.md).

## What is worth borrowing anyway

- **Optimistic-apply → server-reconcile → rebase** is a clean DX loop. Myrhiza's
  analogue is pre-check-then-sign ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.4):
  the *originating peer* runs the verdict before signing, so there is no separate
  trusted reconciler — the verdict is the same code on every peer.
- **Query-scoped sync** (clients receive only subscribed rows) is the
  partial-replication idea Myrhiza defers to v2 ([open-problems.md](open-problems.md)).
- **Custom-mutator candor**: Rocicorp's own pivot from declarative rules to
  "just write server code" is a data point that **declarative cross-object
  authorization is hard to express compactly** — relevant to the
  `myrhiza-permission-rbac` design.

## Funding / state (flagged)

Rocicorp is a small, long-running company (Replicache → Reflect → Zero). A
total raised figure (~$7M) appears on aggregator profiles but we **could not
verify it from a primary source** — treat as unconfirmed. What is verifiable:
Zero is 1.x stable, Apache-2.0, actively released.

## Sources

- Zero permissions / auth: <https://zero.rocicorp.dev/docs/permissions>
- Custom mutators: <https://zero.rocicorp.dev/docs/custom-mutators>
- Deprecated RLS permissions: <https://zero.rocicorp.dev/docs/deprecated/rls-permissions>
- Sync / server-authoritative: <https://zero.rocicorp.dev/docs/sync>, <https://zero.rocicorp.dev/docs/mutators>
- npm registry `@rocicorp/zero` (1.5.0, Apache-2.0): <https://registry.npmjs.org/@rocicorp/zero>
- Zero 1.0 release notes: <https://zero.rocicorp.dev/docs/release-notes/1.0>
