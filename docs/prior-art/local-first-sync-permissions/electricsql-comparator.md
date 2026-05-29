**Date:** 2026-05-29
**Status:** active
**Subject:** ElectricSQL Shapes as comparator — auth is an *external* proxy concern; the sync engine has no permission model of its own

# ElectricSQL (Shapes) — comparator

Electric is a Postgres sync engine (founded 2021 as ElectricSQL; the
`electric-sql/electric` repo was created 2022-06-01; the project pivoted with the
"Electric Next" clean rebuild **announced 2024-07-17** after the
from-scratch-database approach proved hard to adopt; now branded simply
**Electric**, primary docs at electric-sql.com, newer site electric.ax). Electric
1.x is shipping; v1.1 (Aug 2025) added a new storage engine. It is included here as a **comparator**, not a
permission model, because Electric makes an instructive choice: **the sync engine
deliberately has no built-in permission system.**

## Shapes

Electric's one primitive is the **Shape**: a partial replica of a Postgres table
defined by a `WHERE` clause (plus column and include-tree filtering). A client
subscribes to a Shape and Electric streams the matching rows and keeps them live.
Shapes are pure **partial-replication subsets** — the read-side analogue of
PowerSync buckets ([powersync.md](powersync.md)) and Zero queries
([zero-rocicorp.md](zero-rocicorp.md)).

## Authorization is outside the engine

Electric does **not** evaluate identity or roles. Instead it documents two
**external** auth patterns:

- **Proxy auth**: route the shape request through your own HTTP proxy/middleware,
  which authorizes (or rejects) before the request reaches Electric.
- **Gatekeeper auth**: the client presents a token whose claim **contains the
  exact shape definition**; an authorizing proxy compares the signed shape claim
  to the requested shape and only forwards exact matches.

So Electric's "authority" lives entirely in a **trusted proxy in front of the
engine**, and writes go through your own API (Electric is read-path sync only).
This is the trusted-middlebox paradigm taken to its logical end: the middlebox is
literally a separate box you write.

## Why it's a useful comparator for Myrhiza

1. **Gatekeeper = signed shape claim** is a capability-token shape: a token that
   names the exact resource it authorizes. This rhymes with Myrhiza's
   capability-by-resource-handle ([capabilities.md](../../specs/2026-05-09-myrhiza-master-design/capabilities.md)
   §7.4) and with the broader ocap / capability-token corpus
   ([../capability-tokens/](../capability-tokens/)). The difference: Electric's
   gatekeeper is a *trusted online checker*; Myrhiza capabilities are *offline-
   verifiable*.
2. **"No permission model" is a defensible design** when authorization is genuinely
   someone else's job. Myrhiza makes the opposite call — authority is intrinsic to
   `state-apply` — but Electric shows the cost: every Electric deployment must
   build and operate a correct auth proxy, and an auth bug there is a full bypass.
3. **Read-only-engine** clarifies that "sync" and "authority" are separable
   concerns. Myrhiza fuses them deliberately (the verdict *is* part of apply);
   Electric separates them. Naming the runner-up makes the fusion a conscious
   choice.

## State (verified)

Founded 2021; `electric-sql/electric` repo created **2022-06-01** (GitHub API);
"Electric Next" rebuild **announced 2024-07-17** (the rebuild postdates the
repo — do not conflate the announcement with repo creation); v1.x shipping, v1.1
Aug 2025 with a new storage engine. Open-source (Apache-2.0 core). Funding not
verified to a primary source here.

## Sources

- Shapes guide: <https://electric-sql.com/docs/guides/shapes>
- Auth guide (proxy + gatekeeper): <https://electric-sql.com/docs/guides/auth>
- Gatekeeper demo: <https://electric-sql.com/demos/gatekeeper-auth>
- "A new approach to building Electric" (the pivot): <https://electric-sql.com/blog/2024/07/17/electric-next>
- v1.1 storage engine: <https://electric.ax/blog/2025/08/13/electricsql-v1.1-released>
- Repo creation date (`created_at` 2022-06-01): <https://api.github.com/repos/electric-sql/electric>
