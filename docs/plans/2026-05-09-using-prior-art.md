# Using Prior Art Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the consumer-side `using-prior-art` skill, the CLAUDE.md surfacing edit, and the producer-side back-reference, all in a single `docs:` commit.

**Architecture:** Three small, self-contained file changes. No code, no tests in the unit-test sense — verification is structural (file exists, anchor strings present, line counts plausible, no broken cross-refs). Spec at `docs/specs/2026-05-09-using-prior-art-design.md`.

**Tech Stack:** Markdown files only. Editing tools: Write (new file), Edit (existing files), Bash + grep (verification).

---

## File Structure

| Path | Action | Responsibility |
|---|---|---|
| `.claude/skills/using-prior-art/SKILL.md` | Create | Consumer-side skill: discover-extend-promote three-step flow |
| `CLAUDE.md` | Modify | Add new top-level `## Prior Art` section between `## Component Profiles` and `## Build & Test` |
| `.claude/skills/researching-prior-art/SKILL.md` | Modify | Append sibling-skill back-reference paragraph |

No tests files. No code files. Verification is grep-based.

---

### Task 1: Create the `using-prior-art` skill

**Files:**
- Create: `.claude/skills/using-prior-art/SKILL.md`

- [ ] **Step 1: Verify skill directory does not yet exist**

Run: `ls .claude/skills/using-prior-art 2>&1`
Expected: `ls: cannot access '.claude/skills/using-prior-art': No such file or directory`

If the directory already exists, stop and ask the user — do not overwrite.

- [ ] **Step 2: Write the skill file**

Use the Write tool to create `.claude/skills/using-prior-art/SKILL.md` with this exact content:

````markdown
---
name: using-prior-art
description: Use when starting spec/plan/impl/review work that touches a researched area — surface relevant prior-art folders from `docs/prior-art/`, treat them as launchpads for online research, and flag worth-promoting findings back to the corpus.
---

# Using Prior Art

Myrhiza maintains a researched prior-art corpus under `docs/prior-art/` and a curated reference index under `docs/references/`. This skill is the consumer-side flow. The producer-side flow is `researching-prior-art` (which is what built the corpus).

## When to invoke

- **Brainstorming a spec** — primary case. Before drafting the design, surface relevant prior-art and extend with online research.
- **Writing a plan** — verify migration steps don't contradict consulted prior-art.
- **Implementing** — when a design references a borrowed pattern, re-read the source folder.
- **Reviewing** — when a PR touches an area with researched prior-art, check that consulted folders are cited and remain accurate.

Skip when the work is local or mechanical — rename, format, single-bug fix unrelated to runtime semantics.

## Stance: resource, not gate

This skill is not a mandatory checklist. It is an enabling resource. Agents decide depth based on context. The corpus is ~22K researched lines as of 2026-05-09 and is load-bearing for spec authoring; ignoring it when relevant wastes that work, but mechanical consultation when not relevant wastes time.

## Three-step flow

### 1. Discover

Read `docs/README.md` Prior-art section. Most entries close with a "Consult before any spec on X, Y, Z" clause — that is the primary topic surface. Some entries describe applicability inline instead; read the whole entry when the closing clause is absent.

For matching folders, read in this order:
1. `<folder>/README.md` — decision summary, key facts at a glance, why-this-folder-exists, honest assessment.
2. `<folder>/lessons.md` — validates / avoid / borrow.
3. `<folder>/open-problems.md` — gaps (especially relevant when judging whether the corpus covers your current work).

If unsure whether a folder is relevant, read its README — the cost is one Read tool call.

### 2. Extend

Prior-art is a launchpad, not a destination. After consulting, do targeted online research:

- **WebFetch** on URLs cited in the prior-art folder if facts may have changed since the folder's `Date:` frontmatter.
- **WebSearch** for developments after that date.

Look specifically for:
- New releases and version bumps
- Deprecations and end-of-life announcements
- Security advisories
- Governance changes — acquisitions, license flips, maintainer departures, project renames

The corpus is a snapshot. Current state always needs verification.

### 3. Promote

If online research surfaces material worth a *new* prior-art folder, or a major correction to an existing one, flag it inline in the conversation:

```
**Prior-art suggestion** — <one-line name>
- Why it matters: <load-bearing for X / invalidates Y / fills gap Z>
- Source(s) found so far: <URL1>, <URL2>
- Suggested action: spawn `researching-prior-art` <now | after current work lands | low-priority>
- Not blocking: current work proceeds
```

The skill does not spawn `researching-prior-art` itself. The user decides.

**Trigger conditions for promotion** (any one suffices):
- New project / library / protocol that is a load-bearing dependency Myrhiza will lean on
- Existing folder has a major commercial change (acquisition, license flip, project rename) that invalidates conclusions
- Existing folder has a structural gap — current work needs coverage that does not exist
- New academic paper that reframes a paradigm already covered

## Output expectation

When invoked during spec brainstorming, the resulting spec should:

- **Cite consulted folders by path AND section.** `prior-art/mls/lessons.md §3` is a citation. `see prior-art/mls` is not — uncheckable.
- **Name the runner-up paradigm** if a choice was made. Example: "lockstep deterministic VM — see `prior-art/croquet/`; rejected because reflector dependency conflicts with P2P-native commitment."
- **Flag remaining gaps** the prior-art does not cover. These become candidate trigger conditions for future `researching-prior-art` spawns.

## Anti-patterns

- **Reading prior-art and not extending with online research.** Corpus is dated; current state is not.
- **Citing without specifying file/section.** Uncheckable citation = no citation.
- **Spawning `researching-prior-art` mid-spec without flagging first.** Derails the work. Flag the gap, finish the current work, let the user triage.
- **Mandatory consult on local/mechanical work.** Skill is a resource; using it on a one-line rename wastes time.

## Sibling skill

`researching-prior-art` is the producer-side flow — it builds new folders. When this skill identifies a gap and the user decides to spawn research, that is what they spawn.
````

- [ ] **Step 3: Verify file written**

Run: `ls -la .claude/skills/using-prior-art/SKILL.md`
Expected: file exists, size > 3000 bytes, < 10000 bytes.

Run: `head -5 .claude/skills/using-prior-art/SKILL.md`
Expected: starts with `---`, contains `name: using-prior-art`.

- [ ] **Step 4: Verify anchor strings present**

Run: `grep -c "Prior-art suggestion" .claude/skills/using-prior-art/SKILL.md`
Expected: `1`

Run: `grep -c "researching-prior-art" .claude/skills/using-prior-art/SKILL.md`
Expected: at least `2` (mentioned in body + sibling section)

Run: `grep -c "docs/prior-art" .claude/skills/using-prior-art/SKILL.md`
Expected: at least `2`

- [ ] **Step 5: Do NOT commit yet**

Wait for Tasks 2 and 3 — single commit at end of Task 4.

---

### Task 2: Add `## Prior Art` section to CLAUDE.md

**Files:**
- Modify: `CLAUDE.md` — insert new section between `## Component Profiles` and `## Build & Test`

- [ ] **Step 1: Read the surrounding context**

Run: `grep -n "^## " CLAUDE.md`
Expected output includes lines for `## Project Overview`, `## Dev Guidelines`, `## Repository Structure`, `## Component Profiles`, `## Build & Test`, `## Skills`, `## Branch & PR Hygiene`.

Confirm the insertion point: directly after the `## Component Profiles` block (which ends with the line "Pre-check is mechanically the same WASM function as `state-apply`, called by the kernel in dry-run mode. Not a convention.") and before `## Build & Test`.

- [ ] **Step 2: Insert the new section**

Use the Edit tool with these exact strings.

`old_string`:
```
Pre-check is mechanically the same WASM function as `state-apply`, called by the kernel in dry-run mode. Not a convention.

## Build & Test
```

`new_string`:
```
Pre-check is mechanically the same WASM function as `state-apply`, called by the kernel in dry-run mode. Not a convention.

## Prior Art

The corpus under `docs/prior-art/` is researched, dated, and load-bearing for spec authoring. When brainstorming a spec, writing a plan, or reviewing code that touches a researched area, invoke `using-prior-art` to surface relevant folders, extend with online research, and flag worth-promoting findings. Prior-art is a launchpad, not a destination — the corpus is a snapshot; current state needs verification.

When a spec consults prior-art, cite folder + section (`prior-art/mls/lessons.md §3`), name the runner-up paradigm if a choice was made, and flag remaining gaps.

Producer-side workflow: `researching-prior-art` skill (builds new folders).
Consumer-side workflow: `using-prior-art` skill.

## Build & Test
```

- [ ] **Step 3: Verify the edit took**

Run: `grep -n "^## Prior Art$" CLAUDE.md`
Expected: exactly one match. Note the line number.

Run: `grep -c "using-prior-art" CLAUDE.md`
Expected: `2` (inline mention + consumer-side bullet)

Run: `grep -c "researching-prior-art" CLAUDE.md`
Expected: `1` (producer-side bullet)

- [ ] **Step 4: Verify section ordering**

Run: `grep -n "^## " CLAUDE.md`
Expected order: Project Overview → Dev Guidelines → Repository Structure → Component Profiles → **Prior Art** → Build & Test → Skills → Branch & PR Hygiene.

If `## Prior Art` is in the wrong position, revert and re-do Step 2 carefully.

- [ ] **Step 5: Do NOT commit yet**

Continue to Task 3.

---

### Task 3: Append sibling-skill back-reference to `researching-prior-art`

**Files:**
- Modify: `.claude/skills/researching-prior-art/SKILL.md` — append one paragraph

- [ ] **Step 1: Find the lessons section**

Run: `grep -n "^## " .claude/skills/researching-prior-art/SKILL.md | tail -20`
Expected: a list of section headers. Identify the last `## Lessons` (or similarly-named) section. The skill has been appended to with execution lessons 8-11 in prior commits — the file ends with those lessons.

If the skill has no `## Sibling skill` heading already, the back-reference goes at the end of the file as a new top-level section. If a heading already exists for sibling-skill content, stop and ask the user.

Run: `grep -c "^## Sibling" .claude/skills/researching-prior-art/SKILL.md`
Expected: `0`. If non-zero, stop and ask.

- [ ] **Step 2: Find the exact tail of the file**

Run: `tail -5 .claude/skills/researching-prior-art/SKILL.md`
Expected: shows the last few lines of the file. Note the last non-empty line — call it `<last-line>`.

- [ ] **Step 3: Append the sibling-skill section**

Use the Edit tool. Replace the literal `<last-line>` (with surrounding context to ensure uniqueness) with the same line followed by the new section.

If the file ends with a single trailing newline (typical), the Edit pattern is:

`old_string`: `<last-line>` (with enough preceding context — e.g. the previous 1-2 lines — to make it unique)

`new_string`: `<last-line>` followed by:

```
\n## Sibling skill\n\n`using-prior-art` is the consume-side flow. When this skill spawns to fill a gap, the gap was likely surfaced by `using-prior-art` during a spec, plan, or review. Both skills exist; consult the other when working on the corpus.\n
```

(In actual Edit tool input, use real newlines, not `\n` literals.)

- [ ] **Step 4: Verify**

Run: `grep -n "^## Sibling skill$" .claude/skills/researching-prior-art/SKILL.md`
Expected: exactly one match, near end of file.

Run: `grep -c "using-prior-art" .claude/skills/researching-prior-art/SKILL.md`
Expected: at least `2` (heading allusion + body).

Run: `tail -5 .claude/skills/researching-prior-art/SKILL.md`
Expected: shows the new sibling-skill paragraph.

- [ ] **Step 5: Do NOT commit yet**

Continue to Task 4 (verification + commit).

---

### Task 4: Final verification + single commit

**Files:** none modified. Read-only verification + git commit.

- [ ] **Step 1: Confirm all three artifacts present**

Run:
```bash
ls -la .claude/skills/using-prior-art/SKILL.md \
       .claude/skills/researching-prior-art/SKILL.md \
       CLAUDE.md
```
Expected: all three files exist, all readable.

- [ ] **Step 2: Cross-reference sanity check**

Run: `grep -n "using-prior-art" CLAUDE.md .claude/skills/researching-prior-art/SKILL.md .claude/skills/using-prior-art/SKILL.md`

Expected:
- `CLAUDE.md` — at least 2 mentions
- `.claude/skills/researching-prior-art/SKILL.md` — at least 2 mentions
- `.claude/skills/using-prior-art/SKILL.md` — at least 1 (in the title or filename context)

Run: `grep -n "researching-prior-art" CLAUDE.md .claude/skills/using-prior-art/SKILL.md`
Expected: cross-references in both directions are present.

- [ ] **Step 3: Confirm no path drift**

Run: `grep -n "docs/superpowers" CLAUDE.md .claude/skills/using-prior-art/SKILL.md`
Expected: zero matches. Myrhiza uses `docs/specs/` and `docs/plans/`, not the superpowers default `docs/superpowers/specs/`. If any match appears, fix it before committing.

- [ ] **Step 4: Stage and review**

Run:
```bash
git status
git diff --stat
```

Expected git status:
- New file: `.claude/skills/using-prior-art/SKILL.md`
- Modified: `CLAUDE.md`
- Modified: `.claude/skills/researching-prior-art/SKILL.md`

Three files. No others. If there are other modified or untracked files unrelated to this work, do NOT stage them.

Run:
```bash
git add .claude/skills/using-prior-art/SKILL.md \
        .claude/skills/researching-prior-art/SKILL.md \
        CLAUDE.md
```

Run: `git diff --cached --stat`
Expected: three files staged, all under `.claude/skills/...` or `CLAUDE.md`.

- [ ] **Step 5: Commit**

Run:
```bash
git commit -m "$(cat <<'EOF'
docs(skills): add using-prior-art consumer-side flow

Land the consumer-side skill mirroring `researching-prior-art` (producer
side). New skill at `.claude/skills/using-prior-art/SKILL.md`; surfacing
paragraph in CLAUDE.md as a top-level `## Prior Art` section; sibling-skill
back-reference appended to `researching-prior-art`.

Stance: resource, not gate. Three-step flow (discover, extend, promote).
Promotion suggestions surface inline; no backlog file. Catalog format
(`docs/README.md`) unchanged.

See `docs/specs/2026-05-09-using-prior-art-design.md` for design rationale
and tradeoffs.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Run: `git status`
Expected: `nothing to commit, working tree clean` (assuming no unrelated changes).

Run: `git log -1 --stat`
Expected: shows the new commit with the three files.

- [ ] **Step 6: Done**

Confirm to the user:
- Skill landed
- CLAUDE.md surfaces it
- Producer-side back-reference in place
- Single commit, conventional `docs(skills):` prefix
- No code, no tests, no other side effects

---

## Self-review notes

**Spec coverage check** (all spec sections accounted for):

- Skill at `.claude/skills/using-prior-art/SKILL.md` with frontmatter + three-step flow → Task 1.
- CLAUDE.md `## Prior Art` section sibling to `## Component Profiles` → Task 2.
- `researching-prior-art` sibling-skill back-reference → Task 3.
- "Commit as a single `docs:` commit" → Task 4 Step 5 (single commit, all three files).
- README catalog format unchanged → no task touches `docs/README.md` (negative requirement satisfied by absence).
- No vendored superpowers skills modified → only `.claude/skills/researching-prior-art/SKILL.md` (locally maintained, per existing in-repo presence) and the new local skill.

**Type/name consistency check:**
- Skill name `using-prior-art` used identically in: skill frontmatter, CLAUDE.md mentions, researching-prior-art back-reference, commit message.
- Sibling skill `researching-prior-art` — same convention.
- Section name `## Prior Art` (not `## Prior-art` or `## Prior art`) — consistent.

**Placeholder scan:** none. Every step has the actual content needed.
