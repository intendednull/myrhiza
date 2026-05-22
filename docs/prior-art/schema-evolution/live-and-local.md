**Date:** 2026-05-22
**Status:** active
**Subject:** "Live & Local Schema Change: Challenge Problems" — Edwards, Petricek, van der Storm (LIVE@SPLASH 2023). The post-Cambria framing paper that recasts schema evolution as a challenge-problem set rather than a solved problem.

## What it is

A position paper at the LIVE Programming Workshop, co-located with SPLASH 2023 in Cascais, Portugal (2023-10-24). arXiv 2309.11406, submitted 2023-09-20. Authors:

- **Jonathan Edwards** (independent researcher, long association with Subtext and live-programming work)
- **Tomas Petricek** (Charles University, F# / Try F# work, "Foundations of a Live Data Exploration Environment")
- **Tijs van der Storm** (CWI / University of Groningen, Rascal language workbench)

Notable for who is **not** on the paper: Geoffrey Litt, Peter van Hardenberg, Orion Henry — i.e. the three Cambria authors. The "follow-up to Cambria" intellectual move was made by a different set of researchers in a different community (live-programming, not local-first). That's worth noting because it tells you the Cambria authors did not pick up the thread themselves.

## What it argues

Two claims, both deliberately framed as challenges rather than answers:

1. **Schema change is unsolved.** Verbatim from the abstract: *"Schema change is an unsolved problem in both live programming and local-first software."* The paper does not propose a solution. It enumerates problems and invites the community to compare alternative solutions against them.

2. **Live programming and local-first software have the same core problem.** Both have running code with materialized data structures; both can have the code change while the data persists; both can have the data shape change while the running code persists. The two communities had been working on this independently. The paper argues they're solving the same problem under different names.

The methodology: list **concrete scenarios** that any candidate solution should handle. These are the "challenge problems" of the title. They include scenarios like:

- Renaming a field while a peer is offline; the offline peer comes back online still using the old name.
- Splitting one field into two (e.g. `address` → `street + city`); old data has only the combined field.
- Merging two fields into one (the inverse — usually lossy backwards).
- Changing the meaning of an enum value without changing the value itself (semantic shift).
- Adding a constraint (e.g. "field must be non-null") that some existing data violates.
- Concurrent schema changes by different peers (two replicas independently rename the same field to different things).

Each scenario is meant to be a test: *"Here is the situation. What does your tool do?"* A tool that handles them all is, by definition, a solution. A tool that handles some is partial progress.

## Why it matters

Three reasons the paper is important even though it doesn't itself solve anything:

1. **It legitimizes "unsolved" as the honest current answer.** Cambria's posture in 2020-2021 was "we have a research-grade approach that should work." The 2023 paper's posture is "this is unsolved, here are the scenarios, please publish your attempts." The shift in tone reflects the field's increased honesty — 3+ years after Cambria, nobody has shipped lensing in production, and the academic community now acknowledges this openly.

2. **It is cross-community.** Petricek and van der Storm come from live-programming and language-workbench traditions, not from CRDT / local-first. That cross-pollination is unusual and is the paper's structural contribution. If the eventual solution emerges from live-programming tooling rather than CRDT-aware lenses, the 2023 paper will be the bridge document.

3. **It provides a test suite.** Any Myrhiza-side schema-evolution design can be evaluated against the paper's scenarios. Even our chosen position (re-replay-from-genesis as default, version-and-refuse as fallback) is in the paper's test set as "the trivial answer." The paper makes the failure modes of the trivial answer explicit — e.g. "what if the user has years of data?" The trivial answer fails that one.

## How it relates to Cambria

Cambria offered a specific answer (bidirectional lenses); Edwards/Petricek/van der Storm 2023 re-poses the question as a set of scenarios. Reading both:

- Cambria's lenses handle most of the structural scenarios (renames, additions, simple type changes) with non-trivial author effort.
- Cambria's lenses do NOT handle: semantic shifts (per [`open-problems.md`](open-problems.md)), constraint additions where existing data violates the new constraint, concurrent schema changes.
- The 2023 paper does not propose better tools; it formalizes "what a better tool would need to handle."

So the trajectory is: Cambria proposed an answer (2020-2021) → field tried to use it → the answer didn't reach production → the field stepped back and re-framed the question (2023). That's a healthy intellectual progression even if it doesn't produce a Myrhiza-ready library.

## Implications for Myrhiza

1. **Cite the paper in any state-apply versioning spec** as the canonical statement of the problem. *"This spec addresses the structural-evolution subset of the Edwards/Petricek/van der Storm 2023 challenge set; semantic-evolution and concurrent-schema-change are explicitly out of scope for v1."* That framing is honest, defensible, and makes the open problems explicit.
2. **Use the paper's scenarios as a test set** for any schema-evolution design discussion. When proposing a new mechanism, walk through 3-5 scenarios from the paper and document what the mechanism does in each. This is cheap and uncovers most of the failure modes early.
3. **Don't try to solve the full challenge set.** It's unsolved for a reason; solving it isn't a v1 problem and arguably isn't a Myrhiza-org problem at all. Pick a small structural subset, ship that, ship the version-and-refuse fallback for the rest, and contribute back to the academic conversation as Myrhiza encounters concrete cases.

## Implementation status

As of 2026-05-22: there is no implementation associated with the paper. Petricek's work on **Denicek** (https://tomasp.net/denicek/) is a related effort — a document-oriented end-user programming substrate that handles some schema-change scenarios — but it is positioned as a research probe, not a library Myrhiza can adopt. Van der Storm's **Rascal** language workbench (https://www.rascal-mpl.org/) provides meta-programming primitives that are useful for hand-written migration code but are not a schema-evolution solution per se.

## Sources

- "Live & Local Schema Change: Challenge Problems" (arXiv 2309.11406): https://arxiv.org/abs/2309.11406
- LIVE Programming Workshop @ SPLASH 2023: https://liveprog.org/live-2023/
- Tomas Petricek's Denicek project: https://tomasp.net/denicek/
- Tijs van der Storm's Rascal language workbench: https://www.rascal-mpl.org/
- Jonathan Edwards (Subtext author, independent researcher)
- Cross-link: [`cambria.md`](cambria.md)
- Cross-link: [`open-problems.md`](open-problems.md)
