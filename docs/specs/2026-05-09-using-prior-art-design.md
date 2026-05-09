---
date: 2026-05-09
status: draft
topic: using-prior-art
---

# Design — Using Prior Art (consumer-side flow)

## Context

Myrhiza maintains a researched prior-art corpus under `docs/prior-art/` (11 folders, ~22K lines as of 2026-05-09) and a curated reference index under `docs/references/`. The producer-side skill `researching-prior-art` is what built this corpus.

There is no consumer-side flow. Agents starting a spec, plan, implementation, or review have no skill that tells them: *the corpus exists, here is how to find what is relevant, here is how to extend it with online research, here is how to flag gaps back into the producer-side workflow.* This design fills that gap.

## Goals

1. Make the prior-art corpus a discoverable, usable resource for any agent doing non-trivial work — primarily spec authoring, but also plan-writing, implementation, and review.
2. Treat the corpus as a *launchpad* for further online research, not a final destination. The corpus is a snapshot; current state always needs verification.
3. Close the loop: when online research surfaces material genuinely worth promoting to the corpus, the agent flags it inline so the user can decide whether to spawn `researching-prior-art`.

## Non-Goals

- No mandatory consult. The skill is a resource, not a gate. Agents decide depth based on context.
- No persistent backlog of suggested-but-unresearched prior-art (no `BACKLOG.md`). Suggestions surface inline in conversation; user triages on the spot.
- No new format on `docs/README.md` catalog (no `topics:` / `consult-for:` frontmatter arrays). Existing one-paragraph entries — each ending with "Consult before any spec on X, Y, Z" — are sufficient discovery surface.
- No edits to vendored superpowers skills.
- No automatic staleness detection. Skill leans on agent to notice the `Date:` frontmatter and treat the corpus as a snapshot.

## Design

### Skill: `using-prior-art`

New skill at `.claude/skills/using-prior-art/SKILL.md`. Mirror of `researching-prior-art` (producer side); this is the consumer side.

**Frontmatter:**

```yaml
---
name: using-prior-art
description: Use when starting spec/plan/impl/review work that touches a researched area — surface relevant prior-art folders from `docs/prior-art/`, treat them as launchpads for online research, and flag worth-promoting findings back to the corpus.
---
```

**Body — three-step flow:**

1. **Discover.** Read `docs/README.md` Prior-art section. Most entries close with a "Consult before any spec on X, Y, Z" clause — that is the primary topic surface. Some entries describe applicability inline instead; read the whole entry when the closing clause is absent. For matching folders, read `<folder>/README.md` (decision summary), then `lessons.md` (validates / avoid / borrow), then `open-problems.md` (gaps). If unsure whether a folder is relevant, read its README — the cost is one Read tool call.

2. **Extend.** Prior-art is a launchpad, not a destination. After consulting, do targeted online research: WebFetch on URLs cited in the prior-art folder if facts may have changed; WebSearch for developments after the folder's `Date:` frontmatter. Look specifically for: new releases, deprecations, security advisories, governance changes (acquisitions, license flips, maintainer departures, project renames).

3. **Promote.** If online research surfaces material worth a *new* prior-art folder, or a major correction to an existing one, flag inline in the conversation:

   ```
   **Prior-art suggestion** — <one-line name>
   - Why it matters: <load-bearing for X / invalidates Y / fills gap Z>
   - Source(s) found so far: <URL1>, <URL2>
   - Suggested action: spawn `researching-prior-art` <now | after current work lands | low-priority>
   - Not blocking: current work proceeds
   ```

   The skill does not spawn `researching-prior-art` itself. The user decides.

**Output expectation when invoked during spec brainstorming:** the resulting spec should cite consulted folders by path and section (`prior-art/mls/lessons.md §3`, not `see prior-art/mls`); name the runner-up paradigm if a choice was made (e.g. "lockstep — see `prior-art/croquet/`; rejected because reflector dependency conflicts with P2P-native commitment"); flag remaining gaps the prior-art does not cover.

**Anti-patterns (in skill body):**
- Reading prior-art and not extending with online research — corpus is a snapshot.
- Citing prior-art without specifying file/section — uncheckable.
- Spawning `researching-prior-art` mid-spec without flagging first — derails the work; instead, note the gap, finish the spec, let the user triage.

**Trigger conditions for promotion (inside skill body):**
- New project / library / protocol that is a load-bearing dependency Myrhiza will lean on
- Existing folder has a major commercial change (acquisition, license flip, project rename) that invalidates conclusions
- Existing folder has a structural gap (current work needs coverage that does not exist)
- New academic paper that reframes a paradigm already covered

### CLAUDE.md edit

Add a new top-level section, sibling to "## Component Profiles":

```markdown
## Prior Art

The corpus under `docs/prior-art/` is researched, dated, and load-bearing for spec
authoring. When brainstorming a spec, writing a plan, or reviewing code that touches
a researched area, invoke `using-prior-art` to surface relevant folders, extend with
online research, and flag worth-promoting findings. Prior-art is a launchpad, not a
destination — the corpus is a snapshot; current state needs verification.

When a spec consults prior-art, cite folder + section (`prior-art/mls/lessons.md §3`),
name the runner-up paradigm if a choice was made, and flag remaining gaps.

Producer-side workflow: `researching-prior-art` skill (builds new folders).
Consumer-side workflow: `using-prior-art` skill (this section's subject).
```

Promoting to its own section (rather than burying in the existing "## Skills" paragraph) signals that the corpus is substantive enough to warrant top-level mention. ~22K researched lines is substantive.

### researching-prior-art back-reference

Append one paragraph to `.claude/skills/researching-prior-art/SKILL.md` (under the existing lessons section):

> **Sibling skill.** `using-prior-art` is the consume-side flow. When this skill spawns to fill a gap, the gap was likely surfaced by `using-prior-art` during a spec, plan, or review. Both skills exist; consult the other when working on the corpus.

Closes the loop: producer skill knows about consumer skill.

### What is intentionally not changing

- `docs/README.md` catalog format — entries stay as one-paragraph prose with the existing "Consult before any spec on X, Y, Z" closing clause as topic surface.
- Per-folder `README.md` frontmatter — no new `topics:` / `consult-for:` arrays.
- Vendored superpowers skills (`brainstorming`, `writing-plans`, etc.) — no edits.
- No new files outside the skill itself, the CLAUDE.md edit, and the back-reference paragraph.

## Tradeoffs surfaced

| Decision | Runner-up | Why rejected |
|---|---|---|
| Hybrid (skill + light README polish) | New skill only / tooling only / spread-across-existing-skills | User chose; balances structure with low surface area |
| README catalog as-is | Per-folder frontmatter + auto-built INDEX.md | User chose; existing one-liner format is sufficient discovery surface |
| Resource, not gate | Mandatory consult at every phase | User explicit: "should not mandate a specific amount of focus on the prior art" |
| Inline-only push (no backlog file) | `BACKLOG.md` for cumulative suggestions | User chose; deferred suggestions may evaporate but adds long-memory only if pattern recurs |
| CLAUDE.md mention + brainstorming cross-ref | Slash command / vendored-skill edit / mention-only | User chose; respects vendored-skill boundary, leverages `using-superpowers` 1%-rule heuristic |
| Own CLAUDE.md section | Buried in existing "## Skills" paragraph | Corpus is ~22K lines — substantive enough to warrant top-level mention |
| Prose skill body with three numbered steps | Graphviz flowchart | Prose is shorter for v1; convert if it grows unwieldy |

## Open questions / accepted risks

- **Inline-only push has memory cost.** If user defers a suggestion mid-conversation, it vanishes when conversation ends. Accepted; if pattern emerges of repeatedly re-discovering the same gap across specs, that is the signal to add `BACKLOG.md`.
- **No automatic staleness detection.** Agent must notice `Date:` frontmatter and treat the corpus as a snapshot. Accepted; staleness detection tooling is out of scope.
- **README catalog discovery relies on prose convention.** "Consult before any spec on X, Y, Z" is a convention, not a structured field. If entries drift from the convention, discovery degrades. Mitigation: skill body cites the convention; future spec author notices if a new prior-art folder skips it.

## Implementation outline (handed off to writing-plans)

1. Create `.claude/skills/using-prior-art/SKILL.md` with frontmatter + three-step flow body.
2. Add `## Prior Art` section to `CLAUDE.md`.
3. Append sibling-skill paragraph to `.claude/skills/researching-prior-art/SKILL.md`.
4. Commit as a single `docs:` commit.

No code changes. No tests. No build artifacts.

## Sources

- User direction during 2026-05-09 brainstorm session.
- `docs/README.md` — current catalog format.
- `.claude/skills/researching-prior-art/SKILL.md` — producer-side skill being mirrored.
- `CLAUDE.md` — existing repository conventions for skills + spec discipline.
