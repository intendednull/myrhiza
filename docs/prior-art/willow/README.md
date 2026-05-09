**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — internal architectural ancestor of Myrhiza

Entry point for the Willow corpus. Willow is **not** an external system
this folder catalogues at arm's length. Willow is the codebase Myrhiza
generalizes from — the chat-specific peer that PR #636 reframed into a
runtime-shaped product, where the runtime is Myrhiza.

## What Willow is

Willow is a P2P Discord replacement: Rust + Leptos web UI + iroh
transport + Ed25519 identity + ChaCha20-Poly1305 + X25519 key exchange,
event-sourced over a per-author Merkle DAG synced via gossip. It ships
as 15 production crates (`willow-{actor, agent, client, common, crypto,
identity, messaging, network, relay, replay, state, storage, transport,
web, worker}`), ~30 specs and ~38 plans in `docs/`, and a working
multi-peer chat product with channels, roles, governance, voice notes,
file uploads, ephemeral channels, pinning, reactions, search, mobile
support. The most recent merge to `main` (commit `1c053bc`) is the UI
phase 3b voice-notes work, dated 2026-05-09.

## Why this folder exists

Myrhiza is Willow generalized. The reframe happened explicitly inside
Willow as PR #636 ("docs(runtime): master spec for Willow App
Runtime") — a 674-line draft master spec that argues Willow's chat
should be one app among many on top of a small kernel that owns
identity, peer protocol, event/DAG primitives, the component loader,
and the capability arbiter. PR #636 is **draft, not merged**; it is
the proto-spec for Myrhiza.

Several decisions from PR #636 already live verbatim in
`/mnt/storage/projects/myrhiza/CLAUDE.md`:

- The four-component-profile table (`state-apply` strict /
  `state-propose` loose / `interaction` non-deterministic /
  `behavior` non-deterministic).
- "Pre-check is mechanically the same WASM function as `state-apply`,
  called by the kernel in dry-run mode."
- "Capabilities are the only host surface."
- "Determinism is a load-bearing property" framing.

This folder is the curated reading on what worked, what didn't, what
PR #636 explicitly rejected, and what Willow named without resolving.

## Key facts

| | |
|---|---|
| **Repo** | [github.com/intendednull/willow](https://github.com/intendednull/willow) (local working tree: `/mnt/storage/projects/willow`) |
| **Maintainer** | Single-author project (Noah / @intendednull) |
| **License** | (Willow repo — check `LICENSE` file directly) |
| **Last commit on main** | `1c053bc`, 2026-05-09 (UI phase 3b voice notes merge) |
| **Crates** | 15 production crates under `crates/willow-*` |
| **Code size** | ~102k LOC of Rust across `crates/` |
| **Specs** | ~30 in `docs/specs/` |
| **Plans** | ~38 in `docs/plans/` |
| **Master runtime spec** | PR #636, branch `claude/wasm-plugin-system-WyY1p`, **draft, not merged**; diff at `/tmp/willow-pr-636.diff` |
| **Tech stack** | Rust + Leptos + iroh (gossip + blob fetch + relay) + Ed25519 + ChaCha20-Poly1305 + X25519 + HLC |
| **Test stack** | cargo test + wasm-pack (Firefox + geckodriver) + Playwright (Chrome + Firefox + mobile-chrome) |
| **Notable sibling specs** | `2026-04-01-per-author-merkle-dag-state-design.md`, `2026-04-12-state-authority-and-mutations.md`, `2026-04-21-e2e-test-architecture-design.md`, `2026-04-26-state-management-model-design.md`, `2026-04-27-event-based-waits-design.md`, `2026-04-27-willow-runtime/README.md` (PR #636) |

## How to use this folder

1. Read this README.
2. Read [`runtime-vision.md`](runtime-vision.md) — the curated synthesis
   of PR #636.
3. Read [`lessons.md`](lessons.md) — the validates / avoid / borrow
   decision file.
4. Dive into subsystem files when writing a Myrhiza spec that touches
   that area.

## Reading order

1. [`runtime-vision.md`](runtime-vision.md) — what PR #636 envisioned;
   the Myrhiza proto-spec.
2. [`lessons.md`](lessons.md) — validates / avoid / borrow synthesis.
   The load-bearing decision file.
3. [`state-machine.md`](state-machine.md),
   [`authority.md`](authority.md), [`determinism.md`](determinism.md) —
   load-bearing for Myrhiza state-apply work.
4. [`actors.md`](actors.md) — concurrency model (`willow-actor`,
   StateActor, mailbox semantics).
5. [`networking.md`](networking.md), [`identity.md`](identity.md),
   [`crypto.md`](crypto.md) — kernel capability plumbing.
6. [`workers.md`](workers.md), [`apps.md`](apps.md), [`ui.md`](ui.md) —
   the runtime's outward face.
7. [`open-problems.md`](open-problems.md) — what Willow doesn't solve
   and Myrhiza inherits.
8. [`testing.md`](testing.md) — test discipline (the most
   directly-liftable artifact in the corpus).
9. [`glossary.md`](glossary.md) — terms.

Files marked above as siblings are written by other agents in the same
fan-out and may land slightly after this README.

## Framing disclosure

This corpus is written from a **"Myrhiza-as-generalization-of-Willow"**
stance. It is not neutral. We wrote Willow, so the lessons are skewed
toward what Willow attempted; conversely, Willow's gaps are the gaps
Myrhiza is going to face. The "validates" entries in
[`lessons.md`](lessons.md) reflect "we shipped it, we use it, we'd ship
it again" — not "we surveyed alternatives and judged this best." The
"avoid" entries reflect "we shipped it and learned not to" or "PR #636
explicitly rejects it" — not "this is wrong in general."

Future readers auditing **whether Myrhiza-as-generalization-of-Willow
is itself the right primitive** should weigh the corpus accordingly.
This is a **learn-from-Willow-into-Myrhiza-runtime** artifact, not a
neutral catalog of P2P-app-runtime designs. For that wider survey,
consult sibling folders in `prior-art/` (`holochain/`, `mls/`,
`croquet/`, `agoric-endo/`, `spritely-ocapn/`, `pears/`, etc.).

The internal-ancestor framing means **double disclosure**: bias toward
Willow's choices is unavoidable because we made them, and bias against
Willow's gaps is unavoidable because we lived with them. Where this
corpus and another `prior-art/` folder disagree about a pattern, the
other folder is more likely to be the neutral source.

## Consult before any spec on

The decision domains where Willow + PR #636 carry direct, load-bearing
prior-art Myrhiza spec authors should not write without consulting:

- **`state-apply` ABI** — see [`runtime-vision.md`](runtime-vision.md)
  §"ABI commitments", [`determinism.md`](determinism.md).
- **Capability surface** — [`runtime-vision.md`](runtime-vision.md)
  §"Capability model", [`apps.md`](apps.md).
- **Key custody** — [`runtime-vision.md`](runtime-vision.md)
  §"Crypto and key custody", [`crypto.md`](crypto.md).
- **Sync protocol** — [`state-machine.md`](state-machine.md)
  §"`HeadsSummary`", [`networking.md`](networking.md).
- **Actor topology** — [`actors.md`](actors.md),
  [`runtime-vision.md`](runtime-vision.md) §"Runtime and actors".
- **Worker security model** — [`workers.md`](workers.md),
  [`runtime-vision.md`](runtime-vision.md) §"Worker trust shifts".
- **UI app contract** — [`ui.md`](ui.md),
  [`runtime-vision.md`](runtime-vision.md) §"UI is an app".
- **Distributed maintenance & participation** —
  [`open-problems.md`](open-problems.md) §"Distributed maintenance",
  [`runtime-vision.md`](runtime-vision.md).
- **MVP demo app shape** — [`runtime-vision.md`](runtime-vision.md)
  §"MVP shape".
- **Test architecture** — [`testing.md`](testing.md) (lift directly).
- **Authority + permission model** — [`authority.md`](authority.md),
  [`lessons.md`](lessons.md) §"Avoid: centralized
  `required_permission()` table".

## Sources

- **GitHub** — [github.com/intendednull/willow](https://github.com/intendednull/willow) (canonical remote; use this if local working tree is unavailable).
- **Local working tree** — `/mnt/storage/projects/willow/` (full git history available).
- **PR #636 master spec** — [PR #636](https://github.com/intendednull/willow/pull/636), branch `claude/wasm-plugin-system-WyY1p`. Files under `docs/specs/2026-04-27-willow-runtime/` (`README.md` 674 lines + `research-notes-distributed-maintenance.md` 157 lines). Local diff cache: `/tmp/willow-pr-636.diff`.
- **Willow dev guide** — `CLAUDE.md` at the repo root ([on GitHub](https://github.com/intendednull/willow/blob/main/CLAUDE.md)).
- **Myrhiza dev guide** — `/mnt/storage/projects/myrhiza/CLAUDE.md` (cross-checked for what is already lifted).
- **Cross-references** — sibling folders under `/mnt/storage/projects/myrhiza/docs/prior-art/`.
