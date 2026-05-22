**Date:** 2026-05-22
**Status:** active
**Subject:** Project Cambria — Ink & Switch's bidirectional-lens approach to heterogeneous-schema CRDT convergence. Research-grade, stalled, but still the canonical reference for the maximalist version of the schema-evolution problem.

## What it is

Cambria is a TypeScript library + a research essay + a PaPoC 2021 paper. The proposal: define **bidirectional lenses** between schema versions, and you get a single artifact that translates documents *both ways* between v1 and v2. Old peers running v1 can interoperate with new peers running v2, with the lens converting their respective views at the boundary.

Two artifacts to keep distinct:

- **The essay** (October 2020): https://www.inkandswitch.com/cambria/ — *"Translate your data with lenses."* Authors: Geoffrey Litt, Peter van Hardenberg, Orion Henry. Long-form essay framing the problem and sketching the approach. Not peer-reviewed.
- **The PaPoC paper** (April 2021): "Cambria: Schema Evolution in Distributed Systems with Edit Lenses." Same three authors. PaPoC@EuroSys 2021. DOI 10.1145/3447865.3457963, pages 8:1–8:9. Workshop paper, not full conference.

And the implementation:

- **[`inkandswitch/cambria-project`](https://github.com/inkandswitch/cambria-project)** — TypeScript, MIT, 132 commits, 691 stars (as of 2026-05-22). README warns: *"Cambria is still immature software, and isn't yet ready for production use."* The repo is not archived, but it has not been a focus of active Ink & Switch work since approximately 2022. Geoffrey Litt — the lead author — is now at Notion.

## The core idea

Schema migration in traditional databases is **one-directional**: you write a migration from v1 to v2, run it, and the v1 data is gone. In a local-first system this doesn't work: peers come back online running old code, and the system has to keep working anyway.

Cambria's answer: write a **lens** instead of a migration. A lens is a pair of functions `(forward, backward)` that map v1 documents to v2 documents and back. Given a lens, the system can:

1. Apply forward at the read boundary when v2 code reads v1 data.
2. Apply backward at the write boundary when v1 code writes data that v2 code will read.
3. Compose lenses to span multiple versions (`v1 → v2 → v3`).

The intellectual lineage is **Foster et al.'s "Combinators for Bidirectional Tree Transformations"** (TOPLAS 29(3):17, May 2007) and **Hofmann/Pierce/Wagner's "Edit Lenses"** (POPL 2012). Cambria builds on Edit Lenses specifically: lenses operate on **patches** (CRDT operations) rather than whole documents. That's what makes the approach plausible-in-principle for CRDTs — you don't need to translate the whole state on every op, just translate the op itself.

## What lens operations exist

The `cambria-lens` library exposes a fixed combinator set. The major ones:

- **`addProperty`** — add a field at a path. Optionally with a default value for backward translation.
- **`removeProperty`** — remove a field. Backward translation must synthesize a default.
- **`renameProperty`** — rename a field. Bidirectional rename is the cleanest case.
- **`hoistProperty`** — promote a nested field to the parent scope.
- **`plungeProperty`** — the inverse: demote a field into a nested object.
- **`wrapProperty` / `headProperty`** — wrap a scalar into a singleton array (or unwrap).
- **`convertValue`** — type-changing conversion (string ↔ int, enum widening, etc.) with explicit forward + backward functions.
- **`renameType`** — rename the document type, mostly cosmetic.

A lens is a JSON-encoded list of these operations. The forward direction reads the list left-to-right; the backward direction reads it right-to-left applying the inverse of each combinator.

## What Cambria actually demonstrated

The PaPoC paper's main result: a TypeScript implementation that integrates with **Automerge** (as it existed circa 2020) and demonstrates patch-level translation between schema versions on a small example. The demo shape: two replicas running different versions of a Trello-clone schema; a lens between them; concurrent edits that both replicas can apply via the lens.

The Cambria essay extends the story with additional UX ambitions: schema explorers, lens-by-example construction, automatic backward-lens inference for simple cases. None of these UX layers were brought to production.

## Why it stalled

No single failure mode; a cluster of difficulties that accumulate. From a 2026 vantage:

1. **Bidirectional lenses are not easy to construct.** Foster et al.'s combinators are *well-typed* — the type system guarantees that if a lens type-checks, its `forward ∘ backward = identity on the v1 view` law holds. But constructing a lens for any realistic schema change is hard. Renames are easy. Type-changing conversions require the author to write *both directions* and verify that round-tripping doesn't lose information. Removals are worst — the backward direction has to invent the missing data from nothing.

2. **Lens authoring tooling never materialized.** The essay hinted at lens-by-example UI ("Show Cambria two documents, get a lens"); the implementation never delivered. Without tooling, lens authoring is hand-written TypeScript JSON. That's a higher bar than `git diff` between two `.proto` files.

3. **Semantic vs structural is a hard wall.** Renaming `due_date` to `deadline` is structural — a lens handles it. Changing the semantics of `due_date` from "the moment past which the task is overdue" to "the moment past which the task is hidden" is **semantic**. No lens can patch up semantics; the two interpretations require either a coordinated cut-over or an explicit human-mediated reconciliation. The Cambria essay acknowledges this; the implementation has no story for it.

4. **CRDT integration is fragile.** Cambria's CRDT-aware mode (lenses applied to Automerge patches rather than whole documents) is one Automerge version old. Automerge has since rewritten its wire format twice (2.0, 3.0). Keeping the CRDT-patch lens layer working across those rewrites was nontrivial, and nobody owned the work.

5. **The team moved on.** Geoffrey Litt left Ink & Switch for Notion. Peter van Hardenberg continued at Ink & Switch but pivoted to other projects (Patchwork, Trail Runner, automerge-repo). Orion Henry's affiliation is private. The repo gets occasional dependency-update PRs from external contributors; no architectural work since approximately 2022.

6. **The challenge problems followed up — without Litt as an author.** "Live & Local Schema Change: Challenge Problems" (Edwards, Petricek, van der Storm, LIVE@SPLASH 2023, arXiv 2309.11406) explicitly cites the unsolved-ness of the problem Cambria set out to solve. The authors are different — Cambria's lead, Geoffrey Litt, is not on the 2023 paper. That's worth noting: the academic conversation moved on, the original Cambria authors did not pick up the thread. See [`live-and-local.md`](live-and-local.md) for the dedicated treatment.

## What it taught us

Not nothing. Cambria's contributions stand even though the project stalled:

- **Patches-not-documents is the right abstraction for CRDT lensing.** Hofmann/Pierce/Wagner's Edit Lenses pointed in this direction; Cambria operationalized it. Any future CRDT-schema-evolution work that ignores this lesson is starting from scratch.
- **Bidirectionality is well-typed but hard.** The Foster et al. type system gives you safety; it doesn't give you ease. A future system aiming at lensing should expect the *authoring* problem (not the *runtime* problem) to be the bottleneck.
- **Lens combinators decompose nicely.** The `addProperty / removeProperty / renameProperty / hoist / plunge` set has held up — both as a vocabulary for talking about schema changes and as a UI for showing them. Any future tool in this space will likely use a similar vocabulary.
- **The challenge problems are real.** Even if Cambria's specific approach didn't ship, the problem it stated (heterogeneous-schema convergence in local-first systems) is still unsolved. That validates the framing, even as it un-validates the specific solution.

## Implications for Myrhiza

Cambria is the **canonical research reference** for the maximalist version of `state-apply` snapshot portability. We are not going to ship lenses in Myrhiza v1. We should:

1. **Cite Cambria in any state-apply versioning spec** as the prior art that asked the maximalist question. Frame our approach (likely "re-replay from genesis" or "explicit migration function") as the **pragmatic** answer to the question Cambria asked.
2. **Borrow Cambria's lens-combinator vocabulary** if we ever build per-event migration tooling — `addProperty / renameProperty / convertValue` is a good UX vocabulary even if the underlying mechanism is one-directional.
3. **Treat Cambria's stall as a data point**, not a deterrent. The project stalled because of authoring + tooling + maintenance gaps, not because the underlying theory failed. If Myrhiza needs lensing in v2+ for some specific subset of cases (e.g. enum widening), the theory still works.
4. **Do NOT ship a bidirectional-lens system as a v1 feature.** The Cambria lesson is exactly that the production cost is high enough to kill a well-funded research lab's project. We do not have the resources for that fight.

Cross-link: [`lessons.md`](lessons.md) for the validates/avoid/borrow synthesis. [`migration-strategies.md`](migration-strategies.md) for the three-way design choice we will make instead of lensing.

## Sources

- Cambria essay (Litt, van Hardenberg, Henry; October 2020): https://www.inkandswitch.com/cambria/
- Cambria PaPoC paper (DOI 10.1145/3447865.3457963; PaPoC@EuroSys 2021): https://dl.acm.org/doi/10.1145/3447865.3457963
- `inkandswitch/cambria-project` repository: https://github.com/inkandswitch/cambria-project
- Foster, Greenwald, Moore, Pierce, Schmitt. *Combinators for bidirectional tree transformations: A linguistic approach to the view-update problem.* TOPLAS 29(3):17, May 2007: https://www.cis.upenn.edu/~bcpierce/papers/index.shtml
- Hofmann, Pierce, Wagner. *Edit Lenses.* POPL 2012.
- Live & Local Schema Change: Challenge Problems (Edwards, Petricek, van der Storm; LIVE@SPLASH 2023): https://arxiv.org/abs/2309.11406
- Geoffrey Litt's site (current affiliation: Notion): https://www.geoffreylitt.com/
- Cross-link: [`docs/prior-art/crdts/open-problems.md §1`](../crdts/open-problems.md)
