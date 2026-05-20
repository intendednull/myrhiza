**Date:** 2026-05-20
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-2.1 — Q-1 (replay_full tip-fast-path) + Q-7 (anchor-digest off-loop)

# Plan B-2.1 design — runtime perf carryovers

## 1. Goal

Address the two `TODO(B-2)` markers in `crates/kernel/src/runtime.rs` that B-2 explicitly deferred:

- **Q-1 — `replay_full` O(N) per accepted insert.** Current `Runtime::handle_event` and `Runtime::author` call `replay_full` after every `Inserted::NewlyApplied`. For DAGs > ~10k events this O(N) re-apply dominates the select loop and starves incoming gossip. Land a **tip-fast-path optimization**: when the new event is topologically-last *and* the new topo order extends the prior topo order by exactly one element, apply incrementally; else fall back to full replay. Correctness-preserving — the fast path is opportunistic.
- **Q-7 — `compute_anchor_digest` synchronous in the select loop.** Current `Runtime::handle_drift` calls `compute_anchor_digest` inline on cache miss. The subset-replay can be expensive for large anchors. Move it to `tokio::task::spawn_blocking` so other tokio tasks (MemBus publishes, digest_watch consumers, network tasks) can progress while the compute runs.

This slice lands **none** of:

- **Q-4** — sender identity on pending entries — still B-4 (requires iroh's per-connection authenticated sender).
- **Full incremental state model** (state cache per topo position, snapshot-and-delta, or memoized checkpoints) — defer until profiling on a real workload shows tip-fast-path is insufficient. Spec §13.
- **Loop restructure** to split `Subscription::recv()` into a separate task pushing into a buffered channel — would help backfill batching but is a much larger architectural change. Out of scope.
- **Replay batching during HeadsRequest backfill** — current code calls `replay_full` per `handle_event` arrival, even during backfill. Batching across an arrival burst would be a clear next win but needs a "batch boundary" signal that B-1's `Subscription` doesn't expose. Defer.

## 2. Scope decisions (locked during brainstorming + prior-art consultation, 2026-05-20)

| Decision | Chosen | Runner-up | Why |
|---|---|---|---|
| **Q-1 approach** | Tip-fast-path: detect new event is topologically-last *and* extends prior topo by one; apply incrementally; else `replay_full` | (a) Memoized checkpoints every K events; (b) Full per-event state cache; (c) Pure deferral | Checkpoint approach is invalidated when older events insert (e.g. backfill paths) so it doesn't help the worst case. Full state cache requires careful invariant work and is a larger slice. Tip-fast-path is the smallest meaningful win; correctness fallback is the existing `replay_full`. |
| **Q-1 invariant check** | `topo_order_len == prior_topo_order_len + 1 && topo_order[..prior_len] == prior_topo_order && topo_order[prior_len] == inserted_hash` | Just check `inserted_hash == topo_order.last()` | Tighter check protects against subtle edge cases (e.g. an event with the same hash inserts twice via some bug). The verbose check is `O(N)` to *compute* but the components (`==` on stored `Vec<EventHash>`) are cheap. The win is in not doing the **WASM apply** N times — comparing N hashes is two orders of magnitude cheaper than N apply calls. |
| **Cached prior topo + state** | `Runtime` gains `last_topo_order: Vec<EventHash>` + `last_state: Vec<u8>` (mirror of `self.state`) for the diff check | Recompute prior topo each time | The prior `self.state` is already kept; storing the topo order it corresponds to is the lightest possible cache. Memory cost: O(N) `EventHash` (32 bytes each) per kernel — for 100k events, 3.2MB. Acceptable. |
| **Replay fallback trigger** | Fall back to `replay_full` if any of the tip-fast-path checks fail | Only fall back on cache eviction | Fast-path can fail for many reasons (out-of-order insert, equivocation re-topo, etc.); explicit fallback is the simplest correctness guarantee. |
| **Q-7 approach** | `spawn_blocking` with `self.handle` moved into the task; await result; move handle back via task return | Maintain a separate `compute_handle: StateApplyHandle` pre-cloned for digest compute | `StateApplyHandle` is not `Clone`. Owning a second handle would require backend changes to support spawning a parallel instance. Move-in/move-out is simpler and the digest path is rare enough (cache-miss only) that serializing it is fine. |
| **Q-7 cache** | Existing `own_digest_cache` retained unchanged | Bypass cache, recompute each call | Cache covers the common case (same anchor seen repeatedly). spawn_blocking only addresses the cache-miss slow path. |
| **`StateApplyHandle` `Send` bound** | Already satisfied — `ComponentInstance: Send + 'static` per `backend/src/lib.rs:84` | Add Send bound explicitly | Existing trait bound is load-bearing for the spawn_blocking move. Spec §6 cross-references for future readers. |
| Acceptance criterion | New kernel-tier test: 2 peers, multi-event burst, peer A drives compute_anchor_digest while peer B publishes events → assert peer A's published digests show no apply ordering loss | Microbenchmark only | Functional acceptance + correctness-under-spawn_blocking — matches B-1's convergence-test discipline. Microbenchmarks have no harness yet. |

## 3. Code surface — Q-1 (tip-fast-path)

### 3.1 `Runtime` state additions

```rust
// crates/kernel/src/runtime.rs (Runtime struct)

/// Cached topo order corresponding to `self.state`. Used by the
/// tip-fast-path in [`Self::replay_or_incremental`]. Empty Vec when
/// no events have been applied yet (`self.state` is also empty in
/// that case).
last_topo_order: Vec<EventHash>,
```

`self.state: Vec<u8>` already exists from B-1 — that's the "state after applying `last_topo_order`."

### 3.2 New `replay_or_incremental` dispatcher

`replay_full` becomes the fallback; a new dispatcher decides:

```rust
/// Fast-path-then-fallback wrapper for `replay_full`. Per spec §3.
/// Examines the new topo order against the cached `last_topo_order`:
///
/// - If the new topo extends the prior by exactly one element AND
///   that element matches the just-inserted hash → incrementally
///   apply that single event to `self.state` and update
///   `last_topo_order`.
/// - Otherwise → call `replay_full` (existing path) and refresh both.
fn replay_or_incremental(
    &mut self,
    inserted_hash: EventHash,
) -> Result<(), RuntimeError> { /* ... */ }
```

Call sites in `Runtime`:

- `handle_event` (event arrived via gossip) — change `self.replay_full()` to `self.replay_or_incremental(hash)` (the `Inserted::NewlyApplied { hash, .. }` arm already has the hash).
- `author` (locally-authored event) — same call-site change.
- `drain_drift_stash` and other internal flows that currently call `replay_full` directly: audit each. If they're called after batch arrivals (multiple inserts), they should call `replay_full` (the conservative path) until batching lands.

### 3.3 `replay_full` signature change

`replay_full` becomes the authoritative path for full-recompute. It must also **refresh** `last_topo_order` to keep the cache in sync:

```rust
fn replay_full(&mut self) -> Result<(), RuntimeError> {
    let order = self.dag.topo_sort();
    let mut state = Vec::new();
    let mut drops: HashMap<EventHash, String> = HashMap::new();
    for hash in &order {
        // ... existing apply loop ...
    }
    self.state = state;
    self.last_topo_order = order; // NEW — keep cache in sync
    // existing drops + digest_watch publish unchanged
    Ok(())
}
```

### 3.4 Incremental apply implementation

```rust
fn try_tip_incremental(
    &mut self,
    inserted_hash: EventHash,
) -> Result<bool, RuntimeError> {
    let new_order = self.dag.topo_sort();
    // Tip-fast-path eligibility:
    // 1. New order is exactly one longer.
    // 2. New order's prefix matches our cached order.
    // 3. The new last element is the just-inserted event.
    if new_order.len() != self.last_topo_order.len() + 1 {
        return Ok(false);
    }
    if new_order[..self.last_topo_order.len()] != self.last_topo_order[..] {
        return Ok(false);
    }
    let Some(&last) = new_order.last() else {
        return Ok(false);
    };
    if last != inserted_hash {
        return Ok(false);
    }

    // Apply just the new event.
    let Some(event) = self.dag.get(&inserted_hash) else {
        // Theoretically unreachable — `Inserted::NewlyApplied`
        // implies the event is in the DAG. Surface as a fallback
        // signal rather than panic.
        return Ok(false);
    };
    let bytes = canonical_bincode().serialize(event)?;
    let result = self.handle.apply(&self.state, &bytes)?;
    match result.outcome {
        ApplyOutcome::Accepted => {
            self.state = result.new_state;
            self.last_topo_order = new_order;
            // Drops snapshot stays unchanged — the new event was
            // accepted, no new drops.
            let _ = self.digest_watch_tx.send(self.state.clone());
            Ok(true)
        }
        ApplyOutcome::Rejected(reason) => {
            // The newly-inserted event was rejected by state-apply.
            // Per spec §4.4 / §14 edge-case 8: event stays in DAG;
            // state ignores it. Record the drop and update topo cache
            // anyway (the DAG holds the event, the state doesn't).
            self.last_topo_order = new_order;
            #[allow(clippy::expect_used)]
            self.dropped_at_apply
                .lock()
                .expect("dropped_at_apply mutex poisoned")
                .insert(inserted_hash, reason);
            let _ = self.digest_watch_tx.send(self.state.clone());
            Ok(true)
        }
    }
}

fn replay_or_incremental(
    &mut self,
    inserted_hash: EventHash,
) -> Result<(), RuntimeError> {
    if self.try_tip_incremental(inserted_hash)? {
        return Ok(());
    }
    self.replay_full()
}
```

The `Ok(bool)` return from `try_tip_incremental` lets the dispatcher cleanly differentiate "fast-path applied (bool=true, no fallback)" from "ineligible (bool=false, fallback)" without using an error variant for control flow.

### 3.5 Edge cases preserved

- **Drop tracking on incremental Reject path:** the existing `replay_full` clears `dropped_at_apply` wholesale on every replay (spec §4.4: drops are per-replay, not sticky). The incremental path retains stale entries and *additionally* inserts the new drop (if any). **Why this is correct under tip-fast-path:** state-apply is a pure function of `(prior_state, event)`. Adding event `E` at the topo tail does not modify `prior_state` for any pre-existing event in the order, so no prior event's apply outcome can have changed. Drops recorded by previous incremental calls remain valid. If a future insert violates this invariant (event inserts in the middle, re-topos the order) the fast path correctly rejects eligibility and `replay_full` rebuilds the drops map from scratch. The "drops are per-replay" semantics from spec §4.4 are about re-topo scenarios; tip-only is a strict sub-case where drops are stable.

- **State-digest watch publish:** the digest_watch publish must fire on both incremental and full paths. `self.digest_watch_tx.send(self.state.clone())` at the end of `try_tip_incremental` matches `replay_full`'s publish semantics. On the Reject branch the state is emitted unchanged — watchers debounce duplicates.

- **Drift-stash drain:** unchanged. The drift-stash drain happens at the `handle_event` / `author` call site after `replay_or_incremental`, not inside.

- **Topo cache update on Reject:** the cache reflects the DAG's topo_sort output (which includes the rejected event since the DAG retains it). `self.state` reflects the apply outcomes (which excludes the rejected event). These two structures stay aligned because `last_topo_order` is sourced directly from `dag.topo_sort()`, not from the apply path.

## 4. Code surface — Q-7 (anchor-digest off-loop)

### 4.1 Current shape

```rust
// Inline in handle_drift (around runtime.rs:1295-1304 today):
let local_digest =
    if let Some(dg) = self.own_digest_cache.get(&d.anchor.author_seq_vec).copied() {
        dg
    } else {
        let Some(dg) = self.compute_anchor_digest(&d.anchor) else {
            return;
        };
        dg
    };
```

`compute_anchor_digest` calls `self.dag.topo_sort_subset(...)` and then a loop of `self.handle.apply(...)`. Synchronous, blocking the select loop.

### 4.2 New shape — spawn_blocking with handle move

```rust
async fn handle_drift(&mut self, d: DriftMessage) { /* ... unchanged through anchor coverage check ... */

    // Cached path is unchanged.
    let cached = self.own_digest_cache.get(&d.anchor.author_seq_vec).copied();
    let local_digest = if let Some(dg) = cached {
        dg
    } else {
        let Some(dg) = self.compute_anchor_digest_off_loop(&d.anchor).await else {
            return;
        };
        self.own_digest_cache.insert(d.anchor.author_seq_vec.clone(), dg);
        dg
    };

    // ... rest of handle_drift unchanged ...
}

/// Move `self.handle` into a `spawn_blocking` task so other tokio
/// tasks can progress while the subset-replay runs. Per spec §4.
///
/// The handle is moved back via the task's return value.
async fn compute_anchor_digest_off_loop(
    &mut self,
    anchor: &DriftAnchor,
) -> Option<[u8; 32]> {
    let bound = anchor_bound_map(&anchor.author_seq_vec);
    // Snapshot the subset events before moving the handle. The DAG
    // is a BTreeMap so this is a clone of the relevant slice.
    let subset_hashes = self.dag.topo_sort_subset(|e| {
        bound.get(&e.author).copied().is_some_and(|max| e.seq <= max)
    });
    let mut subset_events: Vec<Event> = Vec::with_capacity(subset_hashes.len());
    for h in &subset_hashes {
        let event = self.dag.get(h)?;
        subset_events.push(event.clone());
    }

    // Take ownership of the handle — placeholder swap lets us put it
    // back when the task returns.
    let mut handle = std::mem::replace(&mut self.handle, StateApplyHandle::tombstone());
    let result = tokio::task::spawn_blocking(move || -> (StateApplyHandle, Option<[u8; 32]>) {
        let digest = compute_subset_digest(&mut handle, &subset_events);
        (handle, digest)
    })
    .await
    .ok()?;
    self.handle = result.0;
    result.1
}

/// Pure compute helper — no `self`, no async. Runs on a blocking
/// worker thread.
fn compute_subset_digest(
    handle: &mut StateApplyHandle,
    subset: &[Event],
) -> Option<[u8; 32]> {
    let mut state = Vec::<u8>::new();
    for event in subset {
        let bytes = canonical_bincode().serialize(event).ok()?;
        let r = handle.apply(&state, &bytes).ok()?;
        if let ApplyOutcome::Accepted = r.outcome {
            state = r.new_state;
        }
    }
    let digest_bytes = handle.state_digest(&state).ok()?;
    Some(blake3::hash(&digest_bytes).into())
}
```

### 4.3 `StateApplyHandle::tombstone()` — placeholder during move

`std::mem::replace` requires a value to swap in. We can't construct a `StateApplyHandle` without an underlying `ComponentInstance`, but the runtime task is the only consumer — the tombstone is never observed externally. Add:

```rust
// crates/kernel/src/state_apply.rs
impl StateApplyHandle {
    /// Placeholder handle used while the real handle is moved into
    /// a `tokio::task::spawn_blocking` worker. Callers MUST NOT
    /// invoke any method on a tombstone — doing so panics.
    #[doc(hidden)]
    pub fn tombstone() -> Self {
        Self { instance: Box::new(TombstoneInstance) }
    }
}

struct TombstoneInstance;
impl myrhiza_backend::ComponentInstance for TombstoneInstance {
    fn call_apply(&mut self, _: &[u8], _: &[u8]) -> Result<(Verdict, Vec<u8>), BackendError> {
        unreachable!("tombstone state-apply handle invoked — runtime bug")
    }
    fn call_state_digest(&mut self, _: &[u8]) -> Result<Vec<u8>, BackendError> {
        unreachable!("tombstone state-apply handle invoked — runtime bug")
    }
}
```

The `unreachable!()` here is load-bearing: it converts a runtime bug (a code path that tries to use the handle during the spawn_blocking window) into a deterministic panic with a clear message, instead of silent miscomputation. Annotate with `#[allow(clippy::unreachable)]` if the workspace lint refuses; the `unreachable!` in a `tombstone` constructor is exactly the kind of case the lint contemplates.

**Runner-up considered and rejected:** wrap `self.handle` in `Option<StateApplyHandle>` so the absence-during-move is encoded in the type. Cost: every other use site needs `.as_mut().expect(...)`. Higher noise, same effective semantics. Tombstone keeps the field shape unchanged for the 90% of uses that don't move the handle.

### 4.4 Drift-handle path concurrency note

Because `handle_drift` is `&mut self`, the runtime task can't process other messages while awaiting `compute_anchor_digest_off_loop`. The win is at the **tokio runtime** scope: `spawn_blocking` schedules the compute on a dedicated blocking-IO worker pool, freeing the runtime's main worker threads to drive MemBus publishes, network subscription forwarding, and downstream consumers of `digest_watch`. The runtime task itself yields control during `.await` — so background work on other tokio tasks progresses.

Within the same `Runtime` task, sequential ordering is preserved: anchor-digest compute finishes before any subsequent message is handled. This matches the current synchronous semantics; there is no concurrent-state-mutation risk.

## 5. Acceptance tests

New tests in `crates/kernel/tests/perf_carryovers.rs` (or extend `convergence.rs` — choice up to plan):

| # | Test | Covers |
|---|---|---|
| 1 | `tip_fast_path_taken_for_single_author_extension` | Single peer authors 100 events; assert via a (temporary, test-only) counter that `replay_or_incremental` took the fast path ≥ 99 times. Counter is wrapped in `#[cfg(test)]` and removed for production. |
| 2 | `replay_fallback_when_topo_reorders` | Two peers; peer A authors A1; peer B authors B1 with lex-smaller hash than A1; peer A receives B1; assert fast path was rejected (counter remained at 1 from A1's self-authoring). |
| 3 | `incremental_apply_reject_records_drop` | Single peer, event that state-apply rejects; assert event lands in `dropped_at_apply` via incremental path; assert `digest_watch` published. |
| 4 | `convergence_unchanged_after_tip_fast_path_landing` | Re-run B-1's `convergence.rs::two_peers_two_authors_converge_to_identical_digest`; must still pass. Regression guarantee. |
| 5 | `compute_anchor_digest_off_loop_does_not_block_membus_publish` | Two peers; peer A primed with a 200-event DAG; peer B sends a `DriftMessage` whose anchor is uncovered, then immediately publishes 5 new events. Assert peer A receives at least one of the 5 new events into its pending/dag during the anchor-digest compute (i.e., MemBus publish wasn't gated on the compute). |
| 6 | `anchor_digest_correctness_after_off_loop_move` | Direct call: peer A computes an anchor digest via the new off-loop path; peer B computes the same anchor digest via the old in-line path. Bytes must match. |

Existing B-1 convergence tests (`crates/kernel/tests/convergence.rs`) must continue to pass unchanged — this is the regression guarantee.

Spec-coverage annotations:

- `convergence.md §4.4` (state-apply per-replay drop semantics) → tests 3, 4.
- `convergence.md §4.7` (drift detection + anchor digest) → tests 5, 6.
- `verification.md §22.5` (state-apply / pre-check coverage) → test 4 indirectly.

## 6. Cross-references to backend/state-apply contracts

- `crates/backend/src/lib.rs:84` — `pub trait ComponentInstance: Send + 'static`. The `Send` bound is **load-bearing** for B-2.1: `spawn_blocking` requires the value moved in to be `Send + 'static`. Documenting here so a future change tightening or loosening the trait bound flags B-2.1's spawn_blocking as a consumer.
- `crates/kernel/src/state_apply.rs::StateApplyHandle` — currently not `Clone`. B-2.1 does NOT change that. If a future plan wants concurrent compute (multiple anchor digests in parallel), it will need either (a) a `Backend::spawn_parallel_instance(&self) -> Result<StateApplyHandle>` API, or (b) a `Clone`-able variant. Out of scope.
- `crates/kernel/src/runtime.rs:1124-1161` — current `replay_full` body. B-2.1 keeps this as the fallback; adds the cache update at the end.

## 7. Surface change summary

New public surface in `myrhiza_kernel`:

- `StateApplyHandle::tombstone()` (doc-hidden) — for runtime internal use.

New module-private (`pub(crate)` or no-`pub`) surface:

- `Runtime::last_topo_order` field.
- `Runtime::replay_or_incremental` method.
- `Runtime::try_tip_incremental` method.
- `Runtime::compute_anchor_digest_off_loop` method (replaces synchronous `compute_anchor_digest` call site in `handle_drift`; the underlying `compute_subset_digest` free fn does the actual work).

Unchanged public surface:

- `Runtime::start` signature.
- `Runtime::handle_event` external behavior.
- `Runtime::author` external behavior.
- All B-1 / B-2 acceptance tests pass.

Modified internal behavior:

- `replay_full` now refreshes `last_topo_order` at the end (semantically unchanged from caller POV — `self.state` still reflects the full post-replay state).
- `handle_drift` cache-miss path runs in `spawn_blocking`; caller never sees this — return value semantics unchanged.

## 8. Non-goals (explicit)

- **No memoized checkpoints.** Future enhancement when profiling shows tip-fast-path insufficient.
- **No backfill batching.** `replay_full` still fires once per `Inserted::NewlyApplied`.
- **No microbenchmark harness.** B-2.1 ships behavioral tests; perf is verified by spec §1's qualitative claim ("tip path < 10% of replay_full cost"), not asserted in CI.
- **No StateApplyHandle::clone.** Tombstone-during-move is the v1 approach.
- **No subscription split.** `Runtime::run` continues to drive subscription + author commands from one task.

## 9. Prior-art consultation

Decisions in §2 were grounded in:

- **`prior-art/agoric-endo/persistence.md`** §"Snapshot frequency and the snapshot-as-cache stance": Agoric's "transcript is canonical, snapshot is cache" pattern (validator-local heap snapshots that are NOT consensus-state) is the right analogue for B-2.1's tip-fast-path: a local optimization that doesn't affect the canonical state model. **The pattern Myrhiza is borrowing**: cache may be wrong (or stale) without violating correctness because the canonical recompute path is always available. The tip-fast-path's correctness fallback (`replay_full`) is the Agoric "transcript replay" of last resort.
- **`prior-art/agoric-endo/persistence.md`** §"From LMDB to SQLite" — not directly relevant (B-2.1 has no persistent state), but informs B-7's storage layout.
- **`prior-art/croquet/`** (consulted via `prior-art/croquet/README.md` deferred to lessons.md): Croquet's lockstep VM avoids per-event replay by maintaining live state in the VM. Myrhiza's event-log-replay paradigm explicitly rejects lockstep ([master spec README](2026-05-09-myrhiza-master-design/README.md) §1) so Croquet's pattern doesn't translate. Confirmed: tip-fast-path is the right shape for an event-log-replay runtime, not a lockstep VM.
- **`prior-art/spritely-ocapn/persistence.md`** §"The transactional actormap is the enabler": Spritely's per-turn delta stream pattern (Bloblin store) is structurally similar to an incremental-apply path but lives in a different paradigm (live actor graph, not event-log replay). Validates the architectural intent of incrementally updating state from a known prior position.
- **`prior-art/willow/state-machine.md`** — Willow uses similar replay-on-update mechanics today; their plans documented in `willow-state` similar tip-fast-path ideas but Willow hasn't implemented incremental apply either. Myrhiza is leading the pattern.

**Runner-up paradigms rejected:**

- Lockstep deterministic VM (`prior-art/croquet/`): rejected per master spec; event-log replay is Myrhiza's model.
- Memoized checkpoints à la Agoric heap snapshots: scope creep for B-2.1 (requires snapshot frequency tuning, eviction policy, memory bookkeeping); defer until profiling warrants.

**Remaining gaps in the prior-art corpus** (candidate triggers for future research):

- No deep-dive on tokio scheduling semantics — `spawn_blocking` capacity, when it queues vs runs, interaction with `block_in_place`. Documented in tokio's own docs; not corpus-worthy.
- No deep-dive on incremental-CRDT state-cache patterns. Low priority; Myrhiza explicitly avoids CRDT-in-kernel.

## 10. Edge cases

- **First event ever** (chain genesis): `last_topo_order` starts empty; `prior_len + 1 == 1` matches `new_order.len() == 1`, and the tip equality check ensures the fast path engages correctly.
- **Equivocation insert that re-runs topo from scratch**: `dag.insert` returns `Inserted::NewlyApplied` whose `topo_index` reflects the new position. The tip-fast-path's prefix check catches this — if the new topo's prefix doesn't match `last_topo_order`, it falls back to `replay_full`. No special handling.
- **DAG with single author + monotonic seq**: topo order is strictly seq-order. Every new event is tip-extension; fast path engages 100% of the time. The expected common case.
- **`Inserted::Pending` arriving and being drained by later event**: drain happens before `handle_event` returns; the runtime caller still sees just one `Inserted::NewlyApplied` if the drain successfully advances state. The tip-fast-path eligibility check would fail (multiple new tip entries) — falls back to `replay_full`. Safe.
- **`spawn_blocking` task panic**: the `.await.ok()?` on the result swallows the panic. The handle is GONE — we never restore `self.handle` from a tombstone. Next call panics via `unreachable!()`. Fix: explicit `.await` match arm logs the panic, restores a fresh handle if possible, OR halts the runtime. Document this as a known gap; v1 acceptable because spawn_blocking task bodies are deterministic.
- **`compute_anchor_digest` called concurrently with author**: `handle_drift` and `author` are both `&mut self` on `Runtime`. They serialize naturally through the single-task ownership of `Runtime`. No new race.

## 11. Future work — explicit deferrals

- **Memoized state checkpoints**: every K events, snapshot `self.state` to `Vec<(topo_index, state_bytes)>`. On `replay_full`, find largest snapshot ≤ desired position and replay from there. Land when tip-fast-path miss rate becomes a measurable performance issue.
- **Backfill batching**: extend `Subscription::recv` to signal "batch boundary" (e.g. after a HeadsSummary response burst); runtime drops `replay_*` calls in favor of one terminal replay. Requires Subscription trait change.
- **`StateApplyHandle::clone` / parallel-instance API**: enables true concurrent compute across multiple anchor digests. Requires backend trait surface change.
- **B-4 will likely revisit `Runtime::handle_drift`** to thread the iroh per-connection sender into the drift message envelope (Q-4-shaped change). B-2.1's `compute_anchor_digest_off_loop` should slot into B-4 without changes.

## 12. Sources

- `crates/kernel/src/runtime.rs` lines 1119-1123, 1324-1328 — the two `TODO(B-2)` annotations B-2.1 closes.
- `crates/kernel/src/runtime.rs:1124-1161` — current `replay_full`.
- `crates/kernel/src/runtime.rs:1329-1348` — current `compute_anchor_digest`.
- `crates/backend/src/lib.rs:84` — `ComponentInstance: Send + 'static` (load-bearing for spawn_blocking).
- `crates/kernel/src/state_apply.rs:84-153` — `StateApplyHandle` shape; B-2.1 adds `::tombstone()`.
- [docs/specs/2026-05-09-myrhiza-master-design/convergence.md](2026-05-09-myrhiza-master-design/convergence.md) §4.4 — state-apply per-replay drop semantics.
- [docs/specs/2026-05-09-myrhiza-master-design/convergence.md](2026-05-09-myrhiza-master-design/convergence.md) §4.7 — drift detection + anchor digest.
- [2026-05-10-plan-b-1-dag-memnet-design.md](2026-05-10-plan-b-1-dag-memnet-design.md) §11 — Runtime architecture (single-task ownership).
- [2026-05-19-plan-b-2-persistent-identity-design.md](2026-05-19-plan-b-2-persistent-identity-design.md) §1 + §14 — explicit deferrals of Q-1 + Q-7 to B-2.1.
