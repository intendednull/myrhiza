---
name: researching-prior-art
description: Use when adding a new system to docs/prior-art/. Drives a 7-stage workflow — competitor scan, structured overview, deep-dive fan-out, review, polish, second review, framing disclosure — that produces ~10–20 file folder of reference material future readers can consult when making design decisions.
---

# Researching prior art

For when Myrhiza needs to learn from an external system before adopting or rejecting its patterns. Output is a `docs/prior-art/<system>/` folder of focused, cross-linked reference files. Process is parallel-agent-heavy because depth matters more than speed.

## Required skills

- **REQUIRED:** `organizing-docs` — prior-art doc conventions (folder per system, header format, archival flow).
- **REQUIRED:** `superpowers:dispatching-parallel-agents` — every stage fans out 3–6 subagents.
- **RECOMMENDED:** `superpowers:requesting-code-review` — the review passes are the same shape.

## When to use

- Adding a new external system to `docs/prior-art/`.
- Doing a periodic refresh of an existing prior-art doc whose subject system has shipped major releases since the last update.

## When NOT to use

- One-shot research questions ("how does X work?"). Use `docs/reports/` instead.
- Internal codebase audits. That's `general-audit`-shaped work.
- A system that hasn't been validated as relevant. Run the **competitor scan** stage first; if the system isn't a real neighbor, don't invest.

## Workflow

Seven stages. Each stage is a checkpoint — verify completion before moving on.

### Stage 1: Competitor scan (validates relevance)

Before committing to a deep dive, confirm the system is actually a closest-neighbor and not an adjacent-but-distant project. Dispatch 3 parallel `general-purpose` agents:

- **Closest neighbors** — direct competitors, what they actually are today.
- **Adjacent stacks** — sister projects, integration targets, library-not-runtime layers.
- **Platform layer** — common substrate (e.g. WASM Component Model, libp2p, ocap-CapTP).

Each agent: ~600 words, table format, cite source URLs inline. Synthesize into a one-paragraph verdict naming the closest competitor + the empty niche Myrhiza targets.

**Stop condition:** verdict identifies a single clear closest neighbor. If multiple systems tie, deep-dive the strongest first; defer the others.

### Stage 2: Structured overview (one big file)

Dispatch one `general-purpose` agent for a 2,000–3,500-word structured overview covering:

1. What it is (1-paragraph + key facts table)
2. Architecture
3. Capabilities / authority model
4. Determinism / consistency
5. Networking
6. Identity / crypto
7. Browser / client viability
8. Distribution / versioning
9. Apps shipping
10. Open problems
11. Lessons for Myrhiza (validates / avoid / borrow)
12. Recommended reading order
13. Glossary
14. Sources

Output is one markdown file. Don't worry about file structure yet.

### Stage 3: Split into per-system folder

Following [`organizing-docs`](../organizing-docs/SKILL.md) conventions, break the overview into focused files inside `docs/prior-art/<system>/`:

- `README.md` — overview + key facts + ToC + glossary stub + canonical reading order.
- One file per major subsystem (`architecture.md`, `capabilities.md`, `determinism.md`, `networking.md`, `identity.md`, `distribution.md`, `apps.md`).
- `open-problems.md` — what the system structurally doesn't solve.
- `lessons.md` — **the consult-this-when-designing decision file.** validates / avoid / borrow.
- `glossary.md` — system-specific terms.

Each file ~50–150 lines, independently skimmable, ends with `## Sources`. Cross-link between siblings.

Header for every file uses the prior-art frontmatter from `organizing-docs`:

```
**Date:** YYYY-MM-DD
**Status:** active | archived
**Subject:** <System name + one-line scope>
```

### Stage 4: Deep-dive fan-out

The structured overview is shallow per subsystem. Dispatch ~6 parallel `general-purpose` agents to enrich each focus area. Suggested partition:

- **A1:** runtime internals (architecture / capabilities / determinism additions).
- **A2:** networking + testing / dev infrastructure (often a new file).
- **A3:** identity + crypto + abandoned-features post-mortem (often a new `abandoned.md`).
- **A4:** distribution + tooling + ecosystem (often new `tooling.md` and `ecosystem.md`).
- **A5:** chronological history + governance / funding (new `history.md`, `governance.md`).
- **A6:** third-party critiques + comparisons (new `critiques.md`, `comparisons.md`).

Each agent reads the existing files first to avoid duplication. Returns labeled blocks (`=== ARCHITECTURE ADDITIONS ===`, etc.) for orchestrator to merge. Total enrichment: ~1500–2500 words per agent.

After merge: ~12–20 files, ~2000+ lines.

### Stage 5: First review pass

Dispatch a fresh `general-purpose` reviewer agent. Brief it as "you're seeing this for the first time; tell us if it's reference-grade." Have it evaluate:

1. Accuracy (named versions/dates/people/dollars — these get hallucinated).
2. Coverage gaps (what would a Myrhiza designer hit that's missing?).
3. Internal consistency (do files contradict each other?).
4. Redundancy (full-paragraph repeats are waste).
5. Tone (fluff, AI padding, soft-pedaling unflattering facts).
6. Practical usefulness (would `lessons.md` + a subsystem file actually help?).
7. Cross-link health.

Output is a verdict: **ship-as-is / fix-then-ship / major rework**, plus specific file:line citations.

**Expected verdict on first pass:** fix-then-ship. The next stage handles the fixes.

### Stage 6: Polish pass

Apply reviewer recommendations. Typically requires more parallel agents:

- **Verification agent for version/date facts** — query GitHub releases, official blog posts, dev pulses against suspect claims.
- **Verification agent for citations** — confirm academic paper IDs, verbatim quotes, ICO numbers, repo URLs all resolve as cited.
- **Worked-example agent** — pick a flagship shipping app, walk it end-to-end (e.g. Holochain → Relay). This is the single biggest reviewer-flagged gap on every first pass.
- **Coverage-gap agent** — fill the technical gaps reviewer identified (binary formats, IPC protocols, error models, formal-verification status, hot-reload mechanics).

Apply local text fixes inline:
- Strip "Saves you N years" / "groundbreaking" / "comprehensive" rhetoric.
- Drop superlatives ("most important cautionary tale").
- Move Myrhiza-pitch paragraphs out of evidence files.
- Fix cross-link targets (after restructuring, anchors drift).
- Fold tiny single-point files into adjacent broader files.

### Stage 7: Final review + framing disclosure

Dispatch a second fresh reviewer. Brief includes the polish-pass changelog. Should return **ship now**.

Add a one-paragraph **framing disclosure** to README.md's `## How to use` section. Every prior-art doc is written from a "Myrhiza-design-bet-as-foundation" stance and the lessons reflect that bias. State it explicitly so future readers auditing the bet itself know to weigh accordingly:

```markdown
**Framing disclosure.** These docs are written from a <Myrhiza-bet>-as-foundation
stance — most "Implications for Myrhiza" sub-sections frame <System>'s choices
through that lens. Future readers auditing whether <Myrhiza-bet> is itself the
right primitive should weigh the corpus accordingly: it's a learn-from-<System>-
into-<Myrhiza-bet> artifact, not a neutral catalog.
```

Commit. Move on.

## Hard rules

- **No on-disk category folders.** Each system gets its own folder under `docs/prior-art/<system>/`. Categories (`p2p-runtimes`, `wasm-platforms`, etc.) live as section headers in `docs/README.md` only.
- **Folder name = system name.** Lowercase, kebab-case. No date prefix (these are living docs).
- **Per-file `## Sources` section** with all URLs cited in that file. De-dupe across files is fine — sources can appear in multiple files.
- **Verify cited facts before stage 7.** Specific dates, version numbers, dollar amounts, named people, academic paper IDs, verbatim quotes are the high-risk claims. The polish pass MUST run a verification-agent over them.
- **Honest tone.** Surface unflattering facts. Don't soft-pedal. Quote third-party critiques verbatim. Marketing prose ("groundbreaking") is a smell.
- **Each file independently skimmable.** Some duplication across files is fine; full-paragraph repeats are waste. Use cross-links instead.
- **`lessons.md` is the decision file.** It's where the corpus's value lands. Other files are evidence; lessons.md is the synthesis. Format: validates / avoid / borrow.

## Scaling

The Holochain corpus shipped at 20 files / ~2,300 lines after the full 7-stage process. Smaller / less mature systems land at 10–15 files. Don't pad — if a subsystem only has 30 lines of substance, it's a 30-line file, not a 200-line file with filler.

If the system is research-grade (no shipping apps, small community), shrink stages 4 and 6:
- Skip the worked-example agent in stage 6 if there's no flagship app.
- Skip the testing-infra agent in stage 4 if the project's test harness is just `cargo test`.
- Abandoned-features file may not exist if the project is too young.

## Self-improvement

After each prior-art deep-dive, append a `## Lessons` section at the bottom of this file with what worked / what didn't. The user folds good lessons up over time.

## Lessons

<!-- Append `- YYYY-MM-DD — lesson` entries below. Keep each under ~3 lines. -->

- 2026-05-08 — First execution on Holochain. The 6-agent stage-4 fan-out + worked-example stage-6 agent (Relay deep-dive) was the single biggest quality-mover. Framing disclosure was added in stage 7 after a reviewer flagged the corpus as "not neutral." Don't skip it.
