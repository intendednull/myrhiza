**Date:** 2026-05-29
**Status:** active
**Subject:** Jazz 2.0 alpha — the relational rewrite that reintroduces a trusted-server tier (don't overstate cojson's maturity)

# Maturity and the Jazz 2.0 pivot

The single most important honesty caveat in this folder. The peer-verified cojson
model ([jazz-cojson.md](jazz-cojson.md)) is **Classic Jazz** (npm `cojson` 0.20.x,
`jazz-tools` classic line). Garden Computing is mid-rewrite into **Jazz 2.0**, and
2.0 is **not** a straight continuation of the pure model.

## What 2.0 changes

Jazz 2.0 is described in the repo as a **"local-first relational database"** with
"an entirely new API" — a table-first storage engine (flat `row_format` rows with
reserved `_jazz_*` columns), reactive SQL-ish queries, RocksDB server storage, and
a Rust core (the repo is now ~64% Rust). The root README states plainly: *"this is
the Jazz 2.0 alpha with an entirely new API."* It is **alpha**, prerelease-tagged
`alpha` via Changesets.

## The trust model shifts

The 2.0 sync design introduces an explicit **trust asymmetry** that Classic
cojson did not have. From Jazz's own `specs/status-quo/sync_manager.md`:

> **Upward, toward trusted servers** — Jazz forwards row batch entries … so the
> server can build the same relational view and answer forwarded queries.
> **Downward, toward clients** — Jazz sends only the data that matches the
> client's active query subscriptions.

Client connections now carry a **role** (`User` / `Admin` / `Peer`), and "User
writes may require a session and can be queued for permission evaluation before
they are applied … On an enforcing runtime, missing explicit policies are
rejected." Permissions live in a `permissions.ts` schema file evaluated by the
`QueryManager`. This is much closer to the **Zero/Triplit server-authoritative**
shape ([zero-rocicorp.md](zero-rocicorp.md), [triplit.md](triplit.md)) than to
cojson's relay-is-dumb purity.

## Why the pivot matters for Myrhiza

This is the rare case where a product that **had** the Myrhiza-aligned model is
**moving away from it** under product pressure. Garden's own design notes
(`specs/concerns.md`) name the friction directly:

- *"e2ee is most beloved feature … e2ee makes automatic migration tricky"*
- *"permissions — DX around optimistic updates is crucial"*
- the SQL/relational interface and global transactions ("ecommerce? -> implies …
  global txs") push toward a server that can adjudicate.

The lesson for Myrhiza is **not** "abandon peer-verified authority." It is: the
costs that pushed Jazz toward a trusted tier are real and must be designed for up
front — schema migration over encrypted state, optimistic-update DX, and
cross-object invariants (the very gaps [../crdts/open-problems.md](../crdts/open-problems.md)
§3 names). Myrhiza's answers — pre-check unification
([../../specs/2026-05-09-myrhiza-master-design/convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.4),
schema-version ABI ([../schema-evolution/](../schema-evolution/)), CRDT-in-state-apply
(§4.9) — are exactly the surfaces under pressure here.

## Maturity scorecard (verified)

| Fact | Value | Source |
|---|---|---|
| Classic cojson latest | `0.20.18`, MIT | npm registry |
| Jazz 2.0 status | **alpha**, new API, Rust rewrite | repo README |
| External funding | none disclosed | Tracxn (Apr 2025) |
| Named at-scale production users | **none verified** | — |
| Hosted offering | Jazz Cloud (relay+storage) | jazz.tools/cloud |

Do not cite Jazz as "battle-tested." Cite it as **the clearest architectural
demonstration that per-transaction peer-verified authority is implementable** —
and as a cautionary tale about the forces pulling local-first products back toward
a trusted middlebox.

## Sources

- Jazz repo README + `specs/` (`README.md`, `status-quo/sync_manager.md`, `concerns.md`): <https://github.com/garden-co/jazz>
- npm registry `cojson`: <https://registry.npmjs.org/cojson>
- Tracxn: <https://tracxn.com/d/companies/jazz/__zNBGBa_i64EhT_bn7WnAcrSg2wmDYY47ReTJInp-zuY>
- Classic Jazz docs: <https://classic.jazz.tools/docs>
