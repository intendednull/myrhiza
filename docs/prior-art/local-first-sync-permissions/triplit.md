**Date:** 2026-05-29
**Status:** active
**Subject:** Triplit (Aspen Cloud) — schema-embedded, JWT-claim-driven access rules enforced on the server

# Triplit (Aspen Cloud)

Triplit is a full-stack syncing database originally from **Aspen Cloud** (Y
Combinator W21, San Francisco). It runs on both client and server, syncs over
WebSockets, stores via pluggable backends (IndexedDB, SQLite, Durable Objects),
and uses CRDTs internally for merge. npm `@triplit/client` latest is **1.0.50**,
license **AGPL-3.0-only**. Authority sits on the server — another instance of the
trusted-middlebox paradigm.

**Ownership note (2025-10-08):** Triplit was acquired by **Supabase**;
co-founder Matt Linkous joined Supabase to lead offline-first / third-party
integrations. Supabase states Triplit "remains open source." This is a material
maturity signal — the independent-startup framing below is superseded — though it
does not change the technical authority model documented here. See **State**.

## Permission model

Access control is **declared in the schema** alongside collection definitions
(the newer **permissions API** supersedes the older **rules API**, which remains
supported). Permissions are defined per collection, per operation:

- **read** — who can query
- **insert** — who can add
- **update** / **postUpdate** — who can modify (pre- and post-image checks)
- **delete** — who can remove

Permissions are keyed by **roles**, and roles are matched against **JWT claims**.
A permission is a filter expressed against the row and the token: e.g.
`['authorId', '=', '$token.sub']` restricts to rows owned by the authenticated
subject. Triplit's marketing example: "any authenticated user can read all blog
posts, but only insert/update/delete their own."

## Enforcement is server-side

From Triplit's release notes: *"Access control rules are checked on the server and
unauthorized updates are rejected and not propagated."* The server is the
gatekeeper. A client cannot get an unauthorized write accepted because the server
refuses to propagate it — but this means **the server must be trusted and
honest**, and it sees plaintext to evaluate filters. (Triplit added
access-code/"if you know you know" sharing — capability-URL-shaped grants — but
still server-evaluated.)

## Contrast with cojson

Triplit and cojson both attach authority to data and both use CRDTs for merge.
The difference is **where the verdict is computed**:

- **cojson**: every peer re-runs `determineValidTransactions` from signatures and
  the signed group state; the relay is dumb ([jazz-cojson.md](jazz-cojson.md)).
- **Triplit**: the server evaluates the schema rules against decrypted rows and
  JWT claims; clients trust that verdict.

For Myrhiza this is the same dividing line as Zero ([zero-rocicorp.md](zero-rocicorp.md)):
authority-on-the-server is incompatible with "no trusted middlebox." See
[paradigm-contrast.md](paradigm-contrast.md).

## Worth borrowing

- **Per-operation rule granularity** (read / insert / update / postUpdate /
  delete) and **postUpdate** (validate the *resulting* row, not just the
  request) map cleanly onto what a Myrhiza `state-apply` verdict must check:
  pre-check validates the hypothetical post-state ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.4),
  which is the postUpdate idea generalized.
- **Roles-from-claims** is a compact RBAC vocabulary worth studying for
  `myrhiza-permission-rbac` — but in Myrhiza the "claims" must be signed group
  state on the DAG, not a JWT minted by a trusted issuer.

## State (verified)

AGPL-3.0 client; YC W21; small team. Active 1.x releases through 2025–2026. No
specific funding amount verified beyond "YC-backed." **Acquired by Supabase on
2025-10-08** (verified via Supabase's own announcement); co-founder Matt Linkous
joined Supabase, project stated to remain open source. Treat Triplit as a
Supabase-owned codebase going forward, not an independent startup.

## Sources

- Triplit auth docs: <https://www.triplit.dev/docs/auth>
- Permissions release notes: <https://www.triplit.dev/blog/release-notes-2024-07-12>, <https://www.triplit.dev/blog/release-notes-2025-04-11>, <https://www.triplit.dev/blog/release-notes-2025-04-25>
- Schemas: <https://www.triplit.dev/docs/schemas>
- npm registry `@triplit/client` (1.0.50, AGPL-3.0-only): <https://registry.npmjs.org/@triplit/client>
- Repo: <https://github.com/aspen-cloud/triplit>; YC: <https://www.ycombinator.com/companies/aspen-cloud>
- Supabase acquisition (2025-10-08, "remains open source," Matt Linkous): <https://supabase.com/blog/triplit-joins-supabase>
