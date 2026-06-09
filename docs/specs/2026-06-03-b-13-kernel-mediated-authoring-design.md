**Date:** 2026-06-03
**Status:** landed-design (implementation pending)
**Subject:** B-13 — kernel-mediated authoring: drive an app's WASM `state-propose` through `Runtime::author` so apps emit events on the network

# B-13 — Kernel-mediated authoring

## 1. Goal

Let an installed app turn an app-internal **intent** into a **kernel-signed,
pre-checked, broadcast event** — through the live `Runtime`, using the app's own
WASM `state-propose` component. Today the runtime can author (raw-payload
`Runtime::author`) and can run propose (CLI harness only, single-peer, no
network), but no path connects an app's `propose` logic to the runtime's
sign+broadcast engine. B-13 wires that path. It is the first slice of the
gap-analysis "M1" milestone (app-facing I/O surface).

## 2. The §3.5 finding — why NOT `host.author-event`

The obvious framing ("bind `host.author-event` so a propose component signs its
own events") is **architecturally wrong** and was rejected:

- `architecture.md §3.5`: *"Propose never sees a private key and never produces
  a signature. This makes the propose-vs-apply gap structurally smaller —
  propose cannot bypass pre-check by signing directly."*
- `crates/wasmtime-backend/src/gating.rs` carries a test
  (`manifest_with_apply_only_capability_declared_for_propose_rejects`) that
  **asserts `host.author-event` is rejected for the `StatePropose` profile**.
  That invariant is correct and stays.
- The `host.author-event` WIT import (`host-non-deterministic.wit:18`) is real
  but belongs to the **behavior** profile (bots/automations sign directly with a
  behaviour-scoped identity), per `architecture.md §3.4`. The behavior profile
  has no backend variant yet (`Profile` enum = `StateApply | StatePropose |
  Interaction`); binding `author-event` is deferred to that future work.

So "apps emit events" in v1 is **not** a component-side signing call. It is the
kernel running `propose → pre-check → sign → broadcast` on the app's behalf. The
private key stays kernel-side; the gap between propose output and applied state
stays exactly one kernel-driven `state-apply` dry-run, unchanged.

## 3. Scope

**In:**
- `Runtime` holds the app's `StateProposeHandle` (optional), threaded in by the
  embedder the same way `StateApplyHandle` already is.
- New `AuthorCommand::ProposeAndAuthor { intent, reply }` + handler that runs the
  propose component against current state, then the existing `Runtime::author`
  engine (sign + pre-check + DAG insert + replay + broadcast).
- `RuntimeHandle::propose_and_author(intent) -> Result<EventHash, RuntimeError>`
  public API.
- Tests over the real `counter`/`poll` propose fixtures (MemNetwork; one iroh
  smoke test).

**Out (explicitly deferred, named so the boundary is clear):**
- `host.author-event` binding (behavior profile; future).
- `host.subscribe` + the kernel→interaction push bridge (M1b).
- Multi-device / `identity-handle` resource / `IdentityScope` selection — the
  single-author runtime signs with its one installed `author_key`.
- Driving propose from an interaction component's `dispatch` (UI loop; M1b/UI).
  B-13 stops at the runtime API; the intent producer is the embedder/test.
- Durable storage, behavior profile, browser target.

## 4. Design

### 4.1 Data flow

```
embedder / (future) interaction.dispatch
        │  intent: Vec<u8>
        ▼
RuntimeHandle::propose_and_author(intent)
        │  AuthorCommand::ProposeAndAuthor { intent, reply }  (mpsc)
        ▼
Runtime select-loop arm  ── self.propose_and_author(intent).await
        │
        ├─ propose = self.propose.as_mut().ok_or(RuntimeError::NoProposeComponent)?
        ├─ payload  = propose.propose(&self.state, &intent)?      // real WASM, ProposeError::Rejected → surfaced
        ├─ deps     = current applied frontier (DAG heads)
        └─ self.author(payload, deps).await                       // EXISTING: sign + pre-check + insert + replay + broadcast
        ▼
reply.send(Ok(event_hash))  →  caller gets EventHash
```

The only new logic is steps `propose →deps → author`; `Runtime::author`
(`runtime.rs:2513`) is reused verbatim. Signing, pre-check (state-apply dry-run),
DAG insert, drift, and gossip publish are all unchanged.

### 4.2 Runtime wiring

- `Runtime` gains `propose: Option<StateProposeHandle>` (field, after `handle`).
- `Runtime::start` gains a `propose: Option<StateProposeHandle>` parameter,
  positioned next to the existing `handle: StateApplyHandle` /
  `author_key: Option<AuthorKeypair>` params. All existing call sites pass
  `None`; the new acceptance test passes `Some(propose_handle)`. (Mirrors how the
  B-11 `RuntimeCfg` / B-12 params rippled through test literals — same mechanical
  churn, no behaviour change for `None` callers.)
- `RuntimeHandle` gains `propose_and_author(&self, intent: Vec<u8>)`, an
  `mpsc`+`oneshot` round-trip identical in shape to the existing author path.

### 4.3 deps selection

A propose-authored event causally depends on what the proposing peer has already
applied. Use the DAG's current applied frontier (the same head set the runtime
would attach to a locally-authored event). If no helper exists, add
`EventDag::frontier()`/reuse the existing heads accessor; v1 may default to the
empty set if the frontier accessor is not already present (the per-author
`prev`/`seq` chain — computed inside `author()` — already orders same-author
events; cross-author `deps` is an optimization for convergence speed, not
correctness). The plan resolves which accessor exists; do **not** invent a new
non-deterministic input.

### 4.4 Errors

- No propose component installed → `RuntimeError::NoProposeComponent` (new
  variant).
- Propose rejects the intent (`ProposeError::Rejected(msg)`) → surface as a
  distinct `RuntimeError::ProposeRejected(msg)`.
- Read-only runtime (no `author_key`) → existing `RuntimeError::ReadOnly` (from
  `author()`), reached only after a successful propose — acceptable (propose is
  pure/cheap); or short-circuit on `author_key.is_none()` before calling propose.
  Plan picks one; short-circuit preferred (don't run propose if we can't author).
- Propose output fails pre-check → existing `RuntimeError::PreCheckRejected`
  (from `author()`). This is the load-bearing safety property: a buggy/malicious
  propose cannot get an invalid event applied — the kernel still dry-runs
  `state-apply` before committing.

## 5. ABI / wire impact

**None.** No new host import, no WIT change, no `GossipMessage` variant, no
manifest capability. `host.author-event` stays unbound and the propose-rejects
test stays green. `wire_freeze` is untouched. This is purely additive Rust
surface inside `crates/kernel`.

## 6. Identity

The runtime signs with its single installed `author_key` (the `AuthorKeypair`
already held by `Runtime`, used by `author()`). No identity-handle, no scope
selection, no multi-device. Behaviour/device/MLS scopes are a separate future
capability (they are *why* `host.author-event` + the resource table exist) and
are out of scope here.

## 7. Testing

MemNetwork acceptance (`crates/kernel/tests/`, new file e.g.
`propose_author.rs`):

1. `intent_drives_propose_then_authors_event` — install counter (or poll)
   propose + state-apply; `propose_and_author(increment_intent)` → returns an
   `EventHash`; the event is applied; state reflects the increment; the event was
   published to the topic (assert via MemNetwork sent-log).
2. `propose_rejected_intent_surfaces_error` — propose returns `Err` (zero-delta /
   invalid intent) → `RuntimeError::ProposeRejected`, no event authored, no
   broadcast.
3. `propose_output_failing_precheck_is_rejected` — a propose fixture that emits a
   payload state-apply rejects → `RuntimeError::PreCheckRejected`, nothing
   committed (proves the kernel still gates).
4. `propose_and_author_without_propose_component_errs` → `NoProposeComponent`.
5. `read_only_runtime_propose_and_author_errs` → `ReadOnly` (or short-circuit).

iroh smoke (`crates/kernel/tests/iroh_propose_author.rs`, `network-iroh`
feature): two peers; peer A `propose_and_author` → event gossips → peer B applies
→ both converge on the same state-digest. One test, reusing existing iroh harness
topology.

Compile under BOTH feature sets at every step (default + `network-iroh`).
Regenerate `tests/spec-coverage.md`. No fixture changes (existing
counter/poll propose fixtures suffice), so `build-fixtures-check` is unaffected.

## 8. Touch-list

- `crates/kernel/src/runtime.rs` — `propose` field; `Runtime::start` param;
  `AuthorCommand::ProposeAndAuthor`; select-loop arm; `propose_and_author`
  method; `RuntimeHandle::propose_and_author`; `RuntimeError::{NoProposeComponent,
  ProposeRejected}`.
- `crates/kernel/src/state_propose.rs` — ensure `StateProposeHandle` is
  constructible/holdable by the runtime (it already wraps `Box<dyn
  ProposeInstance>`); re-export if needed.
- `crates/kernel/tests/propose_author.rs` (new), `iroh_propose_author.rs` (new).
- `crates/kernel/tests/helpers/mod.rs` — a `poll_propose_handle()` /
  `counter_propose_handle()` helper (mirror `counter_handle()`), and add the new
  `Runtime::start` `None` arg to existing literals.
- Other `Runtime::start` call sites (attribution.rs, convergence.rs,
  halt_detection.rs, revocation.rs, stale_backfill.rs, iroh_*.rs) — pass `None`.
- `tests/spec-coverage.md` — regenerate.
- `docs/README.md`, `docs/reports/2026-05-21-mvp-gap-analysis.md` — index B-13;
  note M1 partially open (author path landed; subscribe/push bridge still M1b).

## 9. Rejected alternatives

- **Bind `host.author-event` for `StatePropose`** — violates `architecture.md
  §3.5`; would require deleting a correct test and handing a signing key to
  propose. Rejected.
- **Bind `host.author-event` for `Interaction`** — interaction is per-peer UI;
  it should not author without an explicit user-confirmation/behaviour path.
  Rejected for v1.
- **Build the behavior profile now to host `author-event`** — large (no `Profile`
  variant, no instantiate path, no `behavior_instance.rs`, behaviour-identity
  unbuilt). Correct eventual home, wrong size for this slice. Deferred.
- **Have the embedder keep doing propose itself (status quo, B-7 harness)** —
  keeps app event-production single-peer and outside the networked runtime; does
  not unblock real apps. Rejected.
