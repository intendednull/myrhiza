**Date:** 2026-06-03
**Spec:** [B-13 design](../specs/2026-06-03-b-13-kernel-mediated-authoring-design.md)
**Subject:** Implementation plan — kernel-mediated authoring (propose → Runtime::author)

# B-13 implementation plan

TDD throughout. **Every task must leave the tree compiling under BOTH feature
sets** (`cargo check -p myrhiza-kernel` AND `cargo check -p myrhiza-kernel
--features network-iroh`) and all pre-existing tests green. No new host import,
no WIT/wire change — purely additive `crates/kernel` Rust.

## Invariants (do not break)

- `gating.rs` test `manifest_with_apply_only_capability_declared_for_propose_rejects`
  STAYS GREEN (`host.author-event` still rejected for `StatePropose`). If it
  breaks, the change is wrong.
- `wire_freeze.rs` untouched (no `GossipMessage` change).
- `Runtime::author` body (`runtime.rs:2513`) reused verbatim — do not duplicate
  sign/pre-check/broadcast logic.

## Tasks

### T1 — Error + command types (compile-only)
- Add `RuntimeError::NoProposeComponent` and `RuntimeError::ProposeRejected(String)`.
- Add `AuthorCommand::ProposeAndAuthor { intent: Vec<u8>, reply:
  oneshot::Sender<Result<EventHash, RuntimeError>> }`.
- Map `ProposeError::Rejected(msg)` → `RuntimeError::ProposeRejected(msg)` (`From`
  or inline).
- Gate: both feature sets `cargo check`; `cargo test -p myrhiza-kernel` unchanged.

### T2 — Thread the propose handle into Runtime (compile-only, None everywhere)
- `Runtime { …, propose: Option<StateProposeHandle> }`.
- `Runtime::start(…, propose: Option<StateProposeHandle>, …)` — add the param next
  to `handle`/`author_key`; store it.
- Update EVERY existing `Runtime::start` call site to pass `None`
  (helpers/mod.rs, attribution.rs, convergence.rs, halt_detection.rs,
  revocation.rs, stale_backfill.rs, and the `network-iroh` iroh_*.rs sites).
- `#[cfg(test)] test_runtime()` constructor (runtime.rs ~3354): `propose: None`.
- Gate: both feature sets compile; FULL existing suite green (this is the
  ripple-churn task — verify nothing regressed before adding behaviour).

### T3 — propose_and_author handler + handle API (RED→GREEN)
- RED: a unit/acceptance test asserting `RuntimeHandle::propose_and_author`
  exists and round-trips (will fail to compile / fail assert).
- `Runtime::propose_and_author(&mut self, intent: Vec<u8>) -> Result<EventHash,
  RuntimeError>`:
  - if `author_key.is_none()` → `Err(ReadOnly)` (short-circuit before propose).
  - `propose = self.propose.as_mut().ok_or(RuntimeError::NoProposeComponent)?`.
  - `payload = propose.propose(&self.state, &intent).map_err(…ProposeRejected)?`.
  - `deps = <current applied frontier>` — use the existing DAG heads/frontier
    accessor if present; if none exists, use `BTreeSet::new()` and leave a
    `// B-13: cross-author deps optimization — frontier accessor TODO` note
    (per-author prev/seq chain in `author()` already orders correctly).
  - `self.author(payload, deps).await`.
- Select-loop arm: `AuthorCommand::ProposeAndAuthor { intent, reply }` →
  `let r = self.propose_and_author(intent).await; let _ = reply.send(r);`
  (mirror the existing `AuthorCommand::Author` arm at runtime.rs ~1081).
- `RuntimeHandle::propose_and_author(&self, intent: Vec<u8>)` — mpsc send +
  oneshot await, identical shape to the existing author path.
- Gate: both feature sets compile; RED test now GREEN.

### T4 — MemNetwork acceptance suite (`crates/kernel/tests/propose_author.rs`)
- Helper(s) in `helpers/mod.rs`: `poll_propose_handle()` /
  `counter_propose_handle()` mirroring `counter_handle()` but via
  `instantiate_state_propose`. (Pick whichever fixture has a clean
  intent→payload path; poll has an admin gate, counter is simplest — prefer
  counter.)
- Tests (spec §7): (1) intent→propose→author→applied→state-changed→broadcast;
  (2) propose-rejected → `ProposeRejected`, no event/broadcast; (3) propose output
  fails pre-check → `PreCheckRejected` (use the `pre-check-rejector` fixture as
  the state-apply side, or a propose that emits a payload apply rejects); (4) no
  propose component → `NoProposeComponent`; (5) read-only → `ReadOnly`.
- Use the existing condition-wait helper pattern (`poll_until`) for any async
  settle; no fixed sleeps.
- Gate: `cargo test -p myrhiza-kernel` green.

### T5 — iroh smoke (`crates/kernel/tests/iroh_propose_author.rs`, `network-iroh`)
- Two real `Runtime`s over the iroh harness; peer A `propose_and_author` →
  gossip → peer B applies → assert converged state-digest. One test.
- Gate: `cargo test -p myrhiza-kernel --features network-iroh --test
  iroh_propose_author`.

### T6 — Docs + coverage
- `just spec-coverage` regen → commit `tests/spec-coverage.md`.
- `docs/README.md`: add B-13 spec+plan entries (Runtime core + cross-list under
  whatever section fits); mark M1 author-path landed.
- `docs/reports/2026-05-21-mvp-gap-analysis.md`: add a B-13 update paragraph
  (kernel-mediated authoring lands the produce-events half of M1; subscribe +
  push bridge = M1b still open).

## Final gate (whole-tree)

`cargo fmt --all --check` · `cargo clippy --all-targets -- -D warnings` ·
`cargo clippy --all-targets --features myrhiza-network/network-iroh -- -D warnings`
· `cargo test -p myrhiza-kernel` · `cargo test -p myrhiza-kernel --features
network-iroh --tests` · `just spec-coverage-check` · `cargo run -p
dep-direction-check`. (`build-fixtures-check` unaffected — no fixture changes.)
Then `just ci` end-to-end if available locally.
