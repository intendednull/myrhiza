---
name: organizing-docs
description: Conventions for adding, naming, and organizing specs, plans, and reports in docs/. Use when creating a new spec/plan/report, modifying the docs structure (adding a feature area, splitting into nested folder, superseding a doc), or reorganizing the catalog. Mirrors docs/README.md.
---

# Organizing docs

Project-local skill for the cemented conventions of the `docs/` tree.

**Source of truth:** [`docs/README.md`](../../../docs/README.md) (master index, self-documenting). If this skill ever drifts from that file, the README is right. Update this skill in the same commit when conventions change.

## When to use this skill

Invoke before:

- Adding a new spec, plan, or report.
- Modifying the docs structure (adding a feature area to the catalog, splitting a spec into a nested folder, superseding or deprecating a doc).
- Reorganizing the catalog.

If you are *reading* docs (not changing them), go to `docs/README.md` directly — that is the catalog.

## Document types

Four types, each with one job. If a doc does not fit one of these, the type list is wrong, not the doc.

- **Spec** (`docs/specs/`) — *what we are building toward.* Target shape of the code: types, traits, invariants, public API, architectural boundaries. May briefly note current state for contrast, but the bulk is the destination, not the journey. Long-lived, canonical.
- **Plan** (`docs/plans/`) — *how we get from current code to the target.* Migration steps, file-by-file changes, ordering, risks, test strategy, PR-level breakdown. Cites the spec it realizes. Goes stale once shipped.
- **Report** (`docs/reports/`) — *findings from a one-shot investigation of our codebase.* Audits, post-mortems, performance investigations. Dated, immutable.
- **Prior-art** (`docs/prior-art/<system>/README.md`) — *deep dive on an external system we want to learn from.* Living documents; updated as the external system evolves and as our framing of what's relevant shifts. Each system gets its own subfolder so it can grow supporting children (sub-deep-dives, captured diagrams, archived snapshots) without the top-level prior-art directory turning into noise. Category grouping (P2P runtimes, WASM platforms, sync protocols, etc.) lives in the catalog index only — the on-disk layout stays flat.

Implications:

- A spec can have multiple plans (large target, multiple PR-sized chunks).
- A spec without a plan is fine (target known, path deferred).
- A plan without a spec is suspicious — flag it during review.
- Prior-art docs are NOT specs. Do not encode myrhiza-design decisions in a prior-art doc. Capture lessons in the doc's "Lessons for Myrhiza" section, then promote real decisions into a spec.

## Naming

| Type | Pattern | Example |
|---|---|---|
| Spec | `docs/specs/YYYY-MM-DD-<kebab>-design.md` | `2026-05-08-component-abi-design.md` |
| Multi-file spec | `docs/specs/YYYY-MM-DD-<kebab>/README.md` + children | `2026-05-08-runtime-overview/README.md` |
| Plan | `docs/plans/YYYY-MM-DD-<kebab>.md` (no `-design`) | `2026-05-08-component-abi.md` |
| Report | `docs/reports/YYYY-MM-DD-<kebab>.md` | `2026-05-08-wasm-host-audit.md` |
| Prior-art | `docs/prior-art/<system>/README.md` (no date prefix) | `docs/prior-art/holochain/README.md` |
| Prior-art child | `docs/prior-art/<system>/<facet>.md` | `docs/prior-art/holochain/networking-deep.md` |

The date is **when the doc was written**, not the implementation target. The `-design.md` suffix on specs is what visually distinguishes specs from plans in `ls` output. Plans omit it. Prior-art files use the system name (no date prefix) because they're living docs — track revision history via git, not filename.

**Existing files predating these rules are not renamed.** The convention applies to new docs only; the master index labels older entries explicitly so the missing suffix does not affect discovery.

## Document headers

Every new spec, plan, and report opens with:

```
**Date:** YYYY-MM-DD
**Status:** draft | active | landed | superseded
**Spec:** docs/specs/...      (plans only — REQUIRED, points at the spec being realized)
**Supersedes:** docs/specs/... (if applicable)
```

Status semantics for specs/plans/reports:

- `draft` — being written, target not yet stable.
- `active` — current target / in-flight migration.
- `landed` — realized in code; canonical reference.
- `superseded` — replaced; header links to successor.

Prior-art docs use a different header:

```
**Date:** YYYY-MM-DD       (last meaningful update — bump on revision)
**Status:** active | archived
**Subject:** <System name + one-line scope>
```

Prior-art `active` = the system is still relevant to our framing. `archived` = we've concluded the system isn't worth tracking further; doc kept for historical context.

The status tag is a discovery aid, not a project-management tool. Stale tags are tolerable; missing entries in the master index are not.

## Nested folders

Use a folder (`docs/specs/YYYY-MM-DD-<topic>/`) only when one logical document is too large for a single file *and* its children are tightly coupled — they lose meaning without the parent.

Rules:

- The parent `README.md` is **required**. It states the folder's purpose and links every child.
- Children use kebab-case topic names with **no date prefix** — they inherit the parent's date.
- Children are facets of one design, not phase numbers. Phases imply ordering; children do not.
- Maximum one level deep. If a child needs its own children, promote it to a top-level spec.

Multiple independent documents that share a topic are flat siblings, not children — each ships independently.

## Adding a new spec, plan, or report

1. **Pick the right type.** Spec = target. Plan = migration. Report = audit.
2. **Name it.** `YYYY-MM-DD-<kebab>-design.md` (spec) or `YYYY-MM-DD-<kebab>.md` (plan/report). Date is today.
3. **Write the header.** All four fields where applicable. `Spec:` is required for plans.
4. **Add a catalog entry.** One line under the right area in `docs/README.md`:
    ```markdown
    - [Title](specs/YYYY-MM-DD-name-design.md) — 5–15 word summary. `[draft]`
    ```
5. **Pick the area.** Use the catalog's existing `### ` headers in `docs/README.md`. If a doc spans areas, file it under its primary area. If no existing area fits, see "Modifying the structure" below.
6. **Commit the doc and the README entry together.** The catalog must not lag the file.

## Adding a new prior-art doc

1. **Confirm it deserves its own folder.** A prior-art doc is for systems we expect to consult repeatedly. One-shot research notes belong in `docs/reports/` instead.
2. **Create the folder.** `docs/prior-art/<system>/`. Lowercase, kebab-case system name. No date in the path.
3. **Write the main doc** at `docs/prior-art/<system>/README.md`.
4. **Add the header** (see ## Document headers, prior-art variant).
5. **Cover the required sections** (see ## Prior-art structure below).
6. **Add a catalog entry** under the right category sub-section of `## Prior art` in `docs/README.md`. Categories are organizational groupings in the index only — they do NOT exist as on-disk subfolders.
7. **Commit the doc and the README entry together.**

Children (focused sub-deep-dives, captured diagrams, archived screenshots, raw research notes worth preserving) go in the same folder as siblings to `README.md` — e.g. `docs/prior-art/<system>/networking.md`. The README links them. No date prefix on children.

## Prior-art structure

Each prior-art doc covers, at minimum:

- **What it is** — one-paragraph summary + key facts table.
- **Architecture** — components, data model, control flow. Diagrams welcome.
- **Strengths** — what the system gets right that we should learn from.
- **Weaknesses / open problems** — what's broken, what's unsolved, what cost time.
- **Lessons for Myrhiza** — split into `Validates` (our existing choices), `Avoid` (specific pitfalls + our mitigation), `Borrow` (primitives worth studying).
- **Sources** — URLs cited inline + collected at the end.

Optional but encouraged: glossary of system-specific terms, recommended reading order.

## Modifying the structure

- **Adding a feature area:** rare. Adds an `### ` header to the catalog. Update this skill in the same commit if the wording in step 5 above needs to change.
- **Adding a prior-art category:** add a new sub-section under `## Prior art` in `docs/README.md`. Categories are catalog-only; do NOT create category subfolders on disk.
- **Promoting a spec to a nested folder:** rename `<topic>-design.md` → `<topic>/README.md`. Children are added later as kebab-case files (no date). Update the catalog entry to point at the folder's `README.md`.
- **Superseding a doc:** add `**Supersedes:**` to the new doc's header and `[superseded]` plus a link to the successor in the old doc's catalog entry. Do NOT delete the old doc.
- **Archiving a prior-art doc:** flip its `Status:` from `active` to `archived` and add a one-line "Why archived" note under the header. Move catalog entry under an `### Archived` sub-section of `## Prior art`. The folder stays where it is.
