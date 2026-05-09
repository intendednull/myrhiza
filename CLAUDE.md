# CLAUDE.md — Myrhiza Development Guide

## Project Overview

Myrhiza = P2P app runtime. Apps are bundles of WASM components, brokered by a kernel that owns I/O, keys, network, and storage. Apps cannot touch those directly; everything is mediated by capabilities.

The runtime is the host. Applications built on it are separate projects.

The master spec lives under `docs/specs/` (incoming — see PR handoff). Read that before any non-trivial change to runtime semantics.

## Dev Guidelines

Quality + longevity beat speed + convenience.

- **Choose right solution, not easy one.** Ask: which approach makes most sense long-term, causes least future confusion, lasts? Pick that.
- **No hacky workarounds, no shortcuts.** If obvious fix is band-aid, keep digging for real fix.
- **Root-cause every bug.** No patching symptoms. No disabling failing tests. No swallowing errors. Find why, fix why.
- **A failing test is a question, not a chore.** When a test starts failing, the bug is the default suspect — not the test. Diagnose what changed in the code under test before touching the test. Only update the test if (a) the spec/intent genuinely changed and you've updated the spec to match, or (b) the test was always wrong (asserting a false invariant) and you can articulate why. *Never* relax an assertion, swap a real interaction for a synthetic one, or rewrite a helper to dodge a regression — that hides the bug from the next person who runs the suite. If the change in behavior is intentional, fix the production code or update the spec, then update the test to match the new spec.
- **Scope creep OK when warranted, not speculative.** Doing it right means touching more files / refactoring abstraction — do it. Don't add features, abstractions, error handling task didn't ask for.
- **Answer not obvious? Stop, design.** Two+ reasonable approaches? Brief note in `docs/specs/YYYY-MM-DD-<name>-design.md` before coding. Plan in `docs/plans/YYYY-MM-DD-<name>.md`. Cheap up front, expensive later. **Specs in `docs/specs/`, plans in `docs/plans/`.** See `organizing-docs` skill.
- **Surface tradeoffs explicit.** Picking between approaches, name runner-up + why rejected. Commit body or PR description. Future-you needs reasoning, not just result.
- **Mechanical rigor: format + lint + test before commit.** Zero warnings.
- **Semantic rigor: verify before claiming done.** Run actual test, hit actual UI, read actual output. No "should work" assertions. See `superpowers:verification-before-completion`.
- **Process skills before implementation skills.** Brainstorming + debugging determine *how*. Don't skip to feel productive.
- **Determinism is a load-bearing property.** State-apply components must be pure functions of `(prior state, event)` plus the deterministic helper set. Cross-peer convergence depends on it. Treat any non-determinism in `state-apply` as a correctness bug, not a quirk.
- **Capabilities are the only host surface.** Apps reach the host through declared imports. Adding a new host import is an ABI change; design it deliberately, document it in a spec, and consider determinism / sandboxing implications.

## Repository Structure

```
docs/
├── README.md           — Master index of specs, plans, and reports (start here)
├── specs/              — Target state — what we are building toward (YYYY-MM-DD-<name>-design.md)
├── plans/              — Migration steps — how we get to the target (YYYY-MM-DD-<name>.md)
└── reports/            — One-shot audits and investigations
src/                    — Crate sources (workspace layout TBD as runtime lands)
```

The crate layout will fill in as the runtime spec is realized. Don't pre-create empty crates; add them when their first non-trivial code lands.

## Component Profiles

The runtime distinguishes four component profiles. Determinism rules differ by profile — get this right before writing code that crosses a profile boundary:

| Profile | Purpose | Determinism |
|---|---|---|
| `state-apply` | Materialize event into state; authority verdict | Strict — pure fn of `(prior state, event)` plus deterministic helper set |
| `state-propose` | Build candidate event from intent | Loose — kernel re-checks via `state-apply` in dry-run |
| `interaction` | UI / user-facing surface | Non-deterministic OK; runs per-peer |
| `behavior` | Bots, bridges, automations | Non-deterministic OK; identity per `(peer, instance)` |

Pre-check is mechanically the same WASM function as `state-apply`, called by the kernel in dry-run mode. Not a convention.

## Prior Art

The corpus under `docs/prior-art/` is researched, dated, and load-bearing for spec authoring. When brainstorming a spec, writing a plan, or reviewing code that touches a researched area, invoke `using-prior-art` to surface relevant folders, extend with online research, and flag worth-promoting findings. Prior-art is a launchpad, not a destination — the corpus is a snapshot; current state needs verification.

When a spec consults prior-art, cite folder + section (`prior-art/mls/lessons.md §3`), name the runner-up paradigm if a choice was made, and flag remaining gaps.

Producer-side workflow: `researching-prior-art` skill (builds new folders).
Consumer-side workflow: `using-prior-art` skill.

## Build & Test

```bash
cargo fmt --all                # format
cargo clippy --all-targets -- -D warnings   # lint, warnings as errors
cargo test                     # all tests
cargo check                    # quick type-check
```

A `justfile` will land alongside the workspace once there are multiple targets to coordinate.

## Skills

Skills live in `.claude/skills/`. Hooks in `.claude/hooks/` preload `using-superpowers` and `caveman` at session start. Disable both with `MYRHIZA_SKIP_VENDORED_SKILLS=1`.

Imported skill set covers: brainstorming, writing/executing plans, TDD, debugging, code review (give + receive), git worktrees, parallel agents, simplification, organizing docs. See `.claude/skills/` for the full list.

## Branch & PR Hygiene

- Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`).
- One concern per PR. If the PR description needs three sections, it's two PRs.
- Never `--no-verify`. If a hook fails, fix the root cause.
- Never force-push to `main`.
