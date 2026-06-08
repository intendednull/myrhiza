# M1b — `host.subscribe` + multi-topic interaction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bind a kernel-mediated `host.subscribe(topic)` capability for the interaction profile so a sandboxed UI component observes multiple topics' converged state and aggregates them in-sandbox into one multi-channel view.

**Architecture:** A new `subscribe` import (sync-acquire returning a `subscription` resource) recorded as pending intent in `HostState`; the kernel drains it, ensures a read-only per-topic `Runtime`, and forwards that engine's `digest_watch` to the interaction component's new `on-subscription-update` export. Subscribed states live in the component's own memory; `view`/`dispatch` are unchanged. Gating binds the import for `Interaction` only; `state-apply`/`state-propose` reject it.

**Tech Stack:** Rust 2024 workspace, Wasmtime component model, WIT, iroh-gossip (`network-iroh` feature, default-off), `tokio::sync::watch`, MemNetwork test harness.

**Spec:** [2026-06-08-m1b-host-subscribe-multitopic-interaction-design.md](../specs/2026-06-08-m1b-host-subscribe-multitopic-interaction-design.md)

---

## Prerequisites (read before executing)

- **Base must include B-13, and this docs PR must be merged.** The prior-art corpus + this spec/plan live on `docs/m1b-subscription-prior-art`; merge it first so the `../prior-art/*` citations resolve. Implement on a base that includes B-13 (the receive half builds atop the produce half); acceptance tests drive authoring via `RuntimeHandle::propose_and_author` (B-13). If implementing before B-13 merges, substitute the existing test-utils authoring helper.
- **`Runtime::start` has NO `propose` parameter.** B-13 added `propose_and_author` as a `RuntimeHandle` *method*, not a constructor arg. The real signature is `Runtime::start(network, topic, app_bundle_hash, topic_name, handle: StateApplyHandle, peer_key, author_key: Option<_>, cfg, bootstrap)` (`runtime.rs:569`). A read-only engine uses `author_key: None`.
- **`HostState` resource-table field is `table: ResourceTable`** (`engine.rs:231`) — **not** `resource_table`. Use `.table` throughout.
- **Wasmtime is `36.0.9`** (`Cargo.lock`). Verify `component::Linker::resource(name, ty, dtor)` (incl. the **dtor** signature), `Resource<T>` push/delete, and the bindgen mangled name for `subscription.id` against docs.rs for that exact version in T0.
- **`RuntimeHandle.digest_watch: watch::Receiver<Vec<u8>>`** (`runtime.rs:537`) — confirmed.
- **Fixtures build only in the PRIMARY (non-nested) checkout.** This plan is authored in a worktree; the executor MUST run T11's fixture-build steps from the primary checkout (exit the worktree, build + commit fixture bytes, return), then continue. `wasm-tools` pinned `1.248.0`.

## Invariants (do not break)

- **Determinism gate stays green.** `manifest_with_apply_only_capability_declared_for_propose_rejects` (`gating.rs`) and all existing gating tests must pass unchanged. `ambient_set(StateApply)`/`ambient_set(StatePropose)` must contain **only** `DeterministicHelper`-class caps.
- **Subscription state never enters `state-digest()`.** It lives only in component memory + the kernel subscription manager.
- **Backend stays network-free.** `host.subscribe`'s `func_wrap` records pending intent only; the kernel services it. No network handle in `HostState`.
- **`view`/`dispatch` WIT signatures unchanged.** Multi-topic state is accumulated in-component via `on-subscription-update`.
- **Both feature sets compile after every task:** `cargo check -p myrhiza-kernel` AND `cargo check -p myrhiza-kernel --features network-iroh`.
- **Fixtures build only in the primary (non-nested) checkout.** Do the fixture-build steps (T11) outside `.claude/worktrees/`. Pin `wasm-tools` to the CI version (`1.248.0`).

## File map

| Path | Responsibility | Tasks |
|---|---|---|
| `wit/myrhiza-kernel/wit/host-non-deterministic.wit` | `subscription` resource + `subscribe` func | T1 |
| `wit/myrhiza-kernel/wit/world-interaction.wit` | `on-subscription-update` export | T1 |
| `crates/wasmtime-backend/src/gating.rs` | ambient/validate/bound rework | T2, T3, T4 |
| `crates/backend/src/lib.rs` | `InteractionInstance` trait method | T5 |
| `crates/wasmtime-backend/src/engine.rs` | `HostState` pending-subs, prewalk allowlist | T5, T7 |
| `crates/wasmtime-backend/src/gating.rs` (`wire_linker`) | bind `subscribe` + resource | T6 |
| `crates/wasmtime-backend/src/interaction_instance.rs` | `call_on_subscription_update` impl | T8 |
| `crates/kernel/src/subscription.rs` (new) | read-only engine + subscription manager | T9, T10 |
| `crates/kernel/src/runtime.rs` | hold interaction instance, drain pending, view watch | T11 |
| `crates/kernel/tests/fixtures/` (+ a multi-topic interaction fixture) | acceptance fixture | T11 |
| `crates/kernel/tests/subscribe_multitopic.rs` (new) | MemNetwork acceptance | T12 |
| `crates/kernel/tests/iroh_subscribe.rs` (new) | iroh smoke | T13 |
| docs + `tests/spec-coverage.md` | index, coverage, gap-analysis | T14 |

---

### Task 0: Spike — resolve resource-handle plumbing + confirm signatures

**Goal:** De-risk the two open questions (spec §14) before TDD. Not strict TDD — a timeboxed investigation that ends in a written decision.

**Files:**
- Read: `crates/wasmtime-backend/src/engine.rs` (HostState, `instantiate_interaction`, prewalk consts), `crates/wasmtime-backend/src/interaction_instance.rs`, `crates/backend/src/lib.rs:150-245`, `crates/kernel/src/runtime.rs` (`Runtime::start` signature, `digest_watch`), `crates/kernel/tests/iroh_coexistence.rs`.
- Append decision to: this plan's "T0 decision" note below.

- [ ] **Step 1:** Confirm the Wasmtime `Linker::resource` / `ResourceTable` API for registering a host-owned `subscription` resource in a component `Linker` (read the wasmtime version in `Cargo.lock`; check `component::Linker::resource` + `component::Resource<T>`). Decide: WIT `resource` (preferred) vs opaque `u64` handle (fallback). Write the decision + the chosen `subscribe` host-signature.
- [ ] **Step 2:** Confirm exact current signatures: `InteractionInstance` trait methods + `WasmtimeInteractionInstance` fields; `engine.rs` prewalk allowlist constants (`HOST_DETERMINISTIC_INSTANCE`, `HOST_UI_SURFACES_INSTANCE`, `SHARED_TYPES_INSTANCE`) and add `HOST_NON_DETERMINISTIC_INSTANCE = "myrhiza:kernel/host-non-deterministic@1.0.0"`; `Runtime::start` full parameter list — it has **no** `propose` parameter (B-13 added `propose_and_author` as a `RuntimeHandle` method); a read-only Runtime is constructible with `author_key: None`.
- [ ] **Step 3:** Confirm `Runtime`/`RuntimeHandle` expose a `digest_watch` receiver and how a non-authoring peer subscribes to a topic (read-only). Note the exact accessor.
- [ ] **Step 4:** Record the T0 decision (resource vs opaque; exact signatures) in the note below and commit it.

```bash
git add docs/plans/2026-06-08-m1b-host-subscribe-multitopic-interaction.md
git commit -m "docs(m1b): T0 spike decision — subscription handle plumbing"
```

> **T0 decision (known facts + remaining spike items):** _Runtime::start_ = no `propose` param; read-only via `author_key: None` (`runtime.rs:569`). _digest_watch_ = `RuntimeHandle.digest_watch: watch::Receiver<Vec<u8>>` (`runtime.rs:537`). _HostState resource table_ = `table: ResourceTable` (`engine.rs:231`). _wasmtime_ = `36.0.9`. _fuel_ = `SUBSCRIBE_FUEL_COST = 100` (nominal). _revocation_ = via resource **destructor** (T5/T6). **SPIKE to confirm:** the `Linker::resource(name, ty, dtor)` dtor signature + `Resource<u64>` push/`table.delete` in 36.0.9; the bindgen mangled method name for `subscription.id` (run `cargo check -p myrhiza-wasmtime-backend` after T1, grep generated bindings for `subscription`); WIT-`resource` vs opaque-`u64` fallback (prefer resource).

---

### Task 1: WIT — `subscribe` + `subscription` resource + `on-subscription-update`

**Files:**
- Modify: `wit/myrhiza-kernel/wit/host-non-deterministic.wit`
- Modify: `wit/myrhiza-kernel/wit/world-interaction.wit`
- Test: the existing WIT-in-sync test (find via `grep -rn "wit" crates/*/tests crates/sdk` — B-8 added an SDK WIT-sync test).

- [ ] **Step 1: Add the resource + func to `host-non-deterministic.wit`.** Inside the interface, add:

```wit
/// A live subscription to a foreign topic's converged-state feed.
/// Peer-local and non-deterministic. Dropping the handle unsubscribes.
resource subscription {
    /// Stable correlation id, echoed by the interaction world's
    /// `on-subscription-update` delivery export.
    id: func() -> u64;
}

/// Subscribe to a foreign topic's converged-state feed (spec §4.1).
/// `topic` is a 32-byte content-addressed topic id. Returns a handle,
/// or an error string (capability denied, unknown/unreachable topic,
/// outstanding-subscription cap hit).
subscribe: func(topic: list<u8>) -> result<subscription, string>;
```

- [ ] **Step 2: Add the export to `world-interaction.wit`** (after the existing completion handlers):

```wit
/// Delivery of a subscribed topic's converged state (spec §4.1).
/// Called on initial sync and on every converged-state change.
/// `sub-id` correlates to `subscription.id()`. Peer-local; MUST NOT
/// enter any state-digest.
export on-subscription-update: func(sub-id: u64, topic: list<u8>, state: list<u8>);
```

- [ ] **Step 3: Sync the SDK's WIT copy** if `crates/sdk/wit/` mirrors these files (B-8 keeps a copy). Copy the two edits there.
- [ ] **Step 4: Run the WIT-sync test + workspace build.**

Run: `cargo test -p myrhiza-sdk wit 2>&1 | tail -20 && cargo check --workspace`
Expected: WIT-sync test PASS; workspace compiles (bindgen regenerates the interaction world with the new export — `WasmtimeInteractionInstance` will fail to build until T8 adds the call site, so if `cargo check -p myrhiza-wasmtime-backend` errors on a missing `on-subscription-update` binding, that is expected and resolved in T8; gate this step on `cargo check -p myrhiza-manifest -p myrhiza-backend` instead).

- [ ] **Step 5: Commit.**

```bash
git add wit/ crates/sdk/wit/
git commit -m "feat(m1b): WIT — subscribe + subscription resource + on-subscription-update"
```

---

### Task 2: `ambient_set` — surface `host.subscribe` for Interaction

**Files:**
- Modify: `crates/wasmtime-backend/src/gating.rs:45-57`
- Test: `crates/wasmtime-backend/src/gating.rs` (tests mod)

- [ ] **Step 1: Write the failing test.** Add to the `tests` mod:

```rust
#[test]
fn interaction_ambient_includes_host_subscribe_only_for_interaction() {
    assert!(
        ambient_set(Profile::Interaction).contains("host.subscribe"),
        "interaction ambient must include host.subscribe"
    );
    assert!(
        !ambient_set(Profile::StateApply).contains("host.subscribe"),
        "state-apply ambient must NOT include host.subscribe"
    );
    assert!(
        !ambient_set(Profile::StatePropose).contains("host.subscribe"),
        "state-propose ambient must NOT include host.subscribe"
    );
}

/// Locks the determinism invariant: only interaction's ambient may grow
/// non-deterministic; state-apply and state-propose stay det-helpers-only
/// and identical to each other, even as future non-det imports land.
#[test]
fn state_apply_and_propose_ambients_stay_deterministic_helpers_only() {
    let apply = ambient_set(Profile::StateApply);
    let propose = ambient_set(Profile::StatePropose);
    let interaction = ambient_set(Profile::Interaction);
    assert_eq!(apply, propose, "apply and propose ambients must stay identical");
    assert!(apply.is_subset(&interaction), "interaction must be a superset");
    for cap in &apply {
        assert_eq!(
            myrhiza_manifest::vocabulary::classify(cap),
            Some(CapabilityClass::DeterministicHelper),
            "{cap} in apply/propose ambient must be a deterministic helper"
        );
    }
    for non_det in ["host.subscribe", "host.broadcast", "host.author-event"] {
        assert!(
            !apply.contains(non_det) && !propose.contains(non_det),
            "{non_det} must never appear in apply/propose ambient"
        );
    }
}
```

- [ ] **Step 2: Run it to verify it fails.**

Run: `cargo test -p myrhiza-wasmtime-backend interaction_ambient_includes -- --nocapture`
Expected: FAIL (`host.subscribe` not in the set).

- [ ] **Step 3: Branch `ambient_set` on profile.** Replace the body (gating.rs:45-57):

```rust
#[must_use]
pub fn ambient_set(profile: Profile) -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    s.insert("host.hash".into());
    s.insert("host.verify-signature".into());
    s.insert("host.now-hlc-from-event".into());
    s.insert("host.log".into());
    // Interaction is per-peer non-deterministic; it may observe foreign
    // topics. host.subscribe is the only non-deterministic import bound
    // for interaction in v1 (spec §4, §8). state-apply / state-propose
    // ambients stay deterministic-helpers-only (invariant, tested).
    if matches!(profile, Profile::Interaction) {
        s.insert("host.subscribe".into());
    }
    s
}
```

Also update the doc comment above `ambient_set` (gating.rs:27-43) to note interaction's divergence.

- [ ] **Step 4: Run the new test + the existing ambient-invariant tests.**

Run: `cargo test -p myrhiza-wasmtime-backend ambient`
Expected: PASS, including `state_apply_ambient_is_only_deterministic_helpers` and `state_propose_ambient_set_contains_deterministic_helpers_only` (host.subscribe is not added to those profiles, so they stay green).

- [ ] **Step 5: Commit.**

```bash
git add crates/wasmtime-backend/src/gating.rs
git commit -m "feat(m1b): ambient_set surfaces host.subscribe for Interaction"
```

---

### Task 3: `validate_manifest` — ambient-membership gate (behavior-preserving)

**Files:**
- Modify: `crates/wasmtime-backend/src/gating.rs:84-100` (the `host_imports` loop)
- Test: `crates/wasmtime-backend/src/gating.rs` (tests mod)

- [ ] **Step 1: Write the failing tests.**

```rust
#[test]
fn host_subscribe_accepted_for_interaction_rejected_for_apply_and_propose() {
    let mut m = sample_state_apply_manifest();
    m.capabilities.host_imports.insert("host.subscribe".into(), true);

    validate_manifest(&m, Profile::Interaction)
        .expect("host.subscribe must validate for interaction");

    assert!(
        matches!(validate_manifest(&m, Profile::StateApply),
                 Err(BackendError::UnauthorizedImport(c)) if c == "host.subscribe"),
        "host.subscribe must be unauthorized for state-apply"
    );
    assert!(
        matches!(validate_manifest(&m, Profile::StatePropose),
                 Err(BackendError::UnauthorizedImport(c)) if c == "host.subscribe"),
        "host.subscribe must be unauthorized for state-propose"
    );
}
```

- [ ] **Step 2: Run to verify it fails.**

Run: `cargo test -p myrhiza-wasmtime-backend host_subscribe_accepted_for_interaction`
Expected: FAIL — `validate_manifest(_, Interaction)` returns `UnauthorizedImport` today (class check rejects `HostImport`).

- [ ] **Step 3: Replace the class gate with an ambient-membership gate.** Replace the `host_imports` loop body (gating.rs:86-100):

```rust
for (cap, &enabled) in &m.capabilities.host_imports {
    if !enabled {
        continue;
    }
    if DEFERRED_TO_PLAN_B.contains(&cap.as_str()) {
        return Err(BackendError::DeferredToPlanB(cap.clone()));
    }
    // Unknown strings are rejected regardless of profile.
    if classify(cap).is_none() {
        return Err(BackendError::UnknownImport(cap.clone()));
    }
    // A declared host-import is authorized iff it is in this profile's
    // ambient set. Behavior-preserving for existing cases: every
    // profile's ambient is deterministic-helpers-only except
    // interaction, which adds host.subscribe (spec §8 step 3).
    if !ambient.contains(cap) {
        return Err(BackendError::UnauthorizedImport(cap.clone()));
    }
}
```

(`ambient` is already bound at gating.rs:85: `let ambient = ambient_set(profile);`.)

- [ ] **Step 4: Run the new test + the FULL gating suite.**

Run: `cargo test -p myrhiza-wasmtime-backend gating::tests`
Expected: PASS — new test green; `manifest_with_apply_only_capability_declared_for_propose_rejects`, `validate_state_apply_manifest_rejects_non_deterministic_imports`, both `*_deferred_to_plan_b`, and the accept tests all still green.

- [ ] **Step 5: Commit.**

```bash
git add crates/wasmtime-backend/src/gating.rs
git commit -m "feat(m1b): validate_manifest gates on ambient membership (host.subscribe for interaction)"
```

---

### Task 4: `bound_imports` — confirm `host.subscribe` is bound for interaction

**Files:**
- Test: `crates/wasmtime-backend/src/gating.rs` (tests mod). `bound_imports` already intersects declared ∩ ambient, so this is a lock test, not a code change.

- [ ] **Step 1: Write the test.**

```rust
#[test]
fn bound_imports_includes_host_subscribe_for_interaction() {
    let mut m = sample_state_apply_manifest();
    m.capabilities.host_imports.insert("host.subscribe".into(), true);
    let bound = bound_imports(&m, Profile::Interaction);
    assert!(bound.contains("host.subscribe"), "bound: {bound:?}");
    // Not bound for state-apply even if (wrongly) declared.
    let bound_apply = bound_imports(&m, Profile::StateApply);
    assert!(!bound_apply.contains("host.subscribe"));
}
```

- [ ] **Step 2: Run.** Run: `cargo test -p myrhiza-wasmtime-backend bound_imports_includes_host_subscribe`
Expected: PASS (no code change needed — `ambient.contains` already gates it). If it fails, `bound_imports` needs the same ambient lookup as `validate_manifest`; fix to match.

- [ ] **Step 3: Commit.**

```bash
git add crates/wasmtime-backend/src/gating.rs
git commit -m "test(m1b): lock host.subscribe binding for interaction in bound_imports"
```

---

### Task 4.5: Defense-in-depth — state-apply rejects host.subscribe at instantiation

**Files:**
- Test: `crates/wasmtime-backend/tests/` (or the existing instantiation-test module). Exercises the full path manifest → instantiate, not just the `validate_manifest` unit layer.

- [ ] **Step 1: Write the failing test.** Using an existing state-apply fixture + a manifest that (wrongly) declares `host.subscribe`, assert instantiation as `state-apply` is rejected:

```rust
#[test]
fn state_apply_manifest_declaring_host_subscribe_is_rejected_at_install() {
    let mut m = sample_state_apply_manifest();
    m.capabilities.host_imports.insert("host.subscribe".into(), true);
    // instantiate_state_apply runs validate_manifest(_, StateApply) internally.
    let res = backend.instantiate_state_apply(&counter_state_apply_bytes(), &m);
    assert!(
        matches!(res, Err(BackendError::UnauthorizedImport(c)) if c == "host.subscribe"),
        "state-apply must reject host.subscribe end-to-end: {res:?}"
    );
}
```

- [ ] **Step 2: Run to verify it fails / passes.** Run: `cargo test -p myrhiza-wasmtime-backend state_apply_manifest_declaring_host_subscribe`
Expected: PASS once T2–T3 land (the ambient-membership gate rejects it). If the backend has no public `instantiate_state_apply` taking a manifest, place this assertion at the `validate_manifest` boundary the instantiation path calls.

- [ ] **Step 3: Commit.**

```bash
git add crates/wasmtime-backend/
git commit -m "test(m1b): state-apply rejects host.subscribe end-to-end (defense in depth)"
```

---

### Task 5: Backend trait + `HostState` — pending-subscription plumbing

**Files:**
- Modify: `crates/backend/src/lib.rs` (the `InteractionInstance` trait, ~:157)
- Modify: `crates/wasmtime-backend/src/engine.rs` (`HostState`)
- Test: compile-only this task.

- [ ] **Step 1: Add the trait method (signature only).** In `InteractionInstance`:

```rust
/// Deliver a subscribed topic's converged state (spec §4.1).
/// `sub_id` correlates to `subscription.id()`. Calls the component's
/// `on-subscription-update` export. Peer-local; never hashed into state.
fn call_on_subscription_update(
    &mut self,
    sub_id: u64,
    topic: &[u8],
    state: &[u8],
) -> Result<(), BackendError>;
```

- [ ] **Step 2: Add pending-subscription state to `HostState`.** Add fields:

```rust
/// Subscriptions requested by `subscribe` since the last drain.
/// Drained by the kernel after each interaction call (spec §4.2).
pub pending_subscriptions: Vec<(u64, Vec<u8>)>,
/// Subscription ids whose handle was dropped (recorded by the resource
/// destructor) since the last drain. Drained by the kernel to revoke.
pub pending_unsubscriptions: Vec<u64>,
/// Monotonic source of subscription correlation ids.
pub next_sub_id: u64,
```

Initialize all three in `HostState`'s constructor(s) (`pending_subscriptions: Vec::new()`, `pending_unsubscriptions: Vec::new()`, `next_sub_id: 0`). Add helpers:

```rust
impl HostState {
    /// Allocate a fresh subscription id and record a pending request.
    pub fn record_subscription(&mut self, topic: Vec<u8>) -> u64 {
        let id = self.next_sub_id;
        self.next_sub_id += 1;
        self.pending_subscriptions.push((id, topic));
        id
    }
    /// Drain pending subscription requests for the kernel to service.
    pub fn take_pending_subscriptions(&mut self) -> Vec<(u64, Vec<u8>)> {
        std::mem::take(&mut self.pending_subscriptions)
    }
    /// Record a dropped subscription id (called from the resource
    /// destructor — fires on guest drop AND on instance teardown).
    pub fn record_unsubscription(&mut self, sub_id: u64) {
        self.pending_unsubscriptions.push(sub_id);
    }
    /// Drain dropped subscription ids for the kernel to revoke.
    pub fn take_pending_unsubscriptions(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.pending_unsubscriptions)
    }
}
```

- [ ] **Step 3: Add a stub impl** in `crates/wasmtime-backend/src/interaction_instance.rs` so the trait compiles (real body in T8):

```rust
fn call_on_subscription_update(
    &mut self,
    _sub_id: u64,
    _topic: &[u8],
    _state: &[u8],
) -> Result<(), BackendError> {
    Err(BackendError::Instantiation(
        "on-subscription-update not yet wired".into(),
    ))
}
```

- [ ] **Step 4: Compile both feature sets.**

Run: `cargo check -p myrhiza-backend && cargo check -p myrhiza-wasmtime-backend`
Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add crates/backend/src/lib.rs crates/wasmtime-backend/src/engine.rs crates/wasmtime-backend/src/interaction_instance.rs
git commit -m "feat(m1b): InteractionInstance::call_on_subscription_update + HostState pending-subs"
```

---

### Task 6: `wire_linker` — bind `subscribe` + the `subscription` resource for interaction

**Files:**
- Modify: `crates/wasmtime-backend/src/gating.rs:173` (`wire_linker`)
- Test: `crates/wasmtime-backend/src/gating.rs` (`tests_wire` mod) + a fixture-backed link test if available.

> Uses the T0 decision for the exact `Linker::resource` / `Resource<T>` API. The code below assumes a WIT `resource subscription`; if T0 chose the opaque-id fallback, bind `subscribe` to return a `u64` and skip the resource registration.

- [ ] **Step 1: Write the failing test** (linker accepts the non-det instance + subscribe for interaction):

```rust
#[test]
fn wire_interaction_binds_host_subscribe() {
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);
    config.wasm_component_model(true);
    let engine = wasmtime::Engine::new(&config).expect("engine");
    let mut linker: wasmtime::component::Linker<HostState> =
        wasmtime::component::Linker::new(&engine);
    let mut bound = BTreeSet::new();
    bound.insert("host.subscribe".into());
    bound.insert("host.log".into());
    wire_linker(&mut linker, &bound, Profile::Interaction)
        .expect("wire interaction with host.subscribe OK");
}
```

- [ ] **Step 2: Run to verify it fails.**

Run: `cargo test -p myrhiza-wasmtime-backend wire_interaction_binds_host_subscribe`
Expected: FAIL (linker has no `host-non-deterministic` instance / `subscribe` registration).

- [ ] **Step 3: Add the non-det instance wiring in `wire_linker`.** After the deterministic-helper block, before `Ok(())`:

```rust
// Interaction-only: bind host.subscribe on the non-deterministic
// instance (spec §8 step 5). subscribe records pending intent in
// HostState and returns a handle; the kernel services it. No network
// I/O here — the backend stays network-free.
if matches!(profile, Profile::Interaction) && bound_imports.contains("host.subscribe") {
    let mut nd = linker
        .instance("myrhiza:kernel/host-non-deterministic@1.0.0")
        .map_err(|e| BackendError::Instantiation(format!("linker instance nd: {e}")))?;

    // Register the `subscription` resource (host-owned, rep = the stored
    // sub_id). The destructor fires on guest drop AND on instance
    // teardown — it deletes the table entry and records a pending
    // unsubscription the kernel drains (spec §4.2 revoke). This is the
    // ONLY revocation path; both routes unify here, so the kernel never
    // needs an explicit `unsubscribe` import.
    nd.resource(
        "subscription",
        wasmtime::component::ResourceType::host::<u64>(),
        |mut store: wasmtime::StoreContextMut<'_, HostState>, rep: u32| -> wasmtime::Result<()> {
            let sub: wasmtime::component::Resource<u64> =
                wasmtime::component::Resource::new_own(rep);
            let sub_id = store.data_mut().table.delete(sub)?;
            store.data_mut().record_unsubscription(sub_id);
            Ok(())
        },
    )
    .map_err(|e| BackendError::Instantiation(format!("resource subscription: {e}")))?;

    nd.func_wrap(
        "subscribe",
        |mut caller: wasmtime::StoreContextMut<'_, HostState>, (topic,): (Vec<u8>,)|
         -> wasmtime::Result<(Result<wasmtime::component::Resource<u64>, String>,)> {
            if topic.len() != 32 {
                return Ok((Err("topic must be 32 bytes".into()),));
            }
            let sub_id = caller.data_mut().record_subscription(topic);
            let res = caller
                .data_mut()
                .table
                .push(sub_id)
                .map_err(|e| wasmtime::Error::msg(format!("push subscription: {e}")))?;
            Ok((Ok(res),))
        },
    )
    .map_err(|e| BackendError::Instantiation(format!("wire host.subscribe: {e}")))?;

    // `subscription.id()` returns the correlation id.
    nd.func_wrap(
        "[method]subscription.id",
        |caller: wasmtime::StoreContextMut<'_, HostState>, (this,): (wasmtime::component::Resource<u64>,)|
         -> wasmtime::Result<(u64,)> {
            let id = *caller.data().table.get(&this)
                .map_err(|e| wasmtime::Error::msg(format!("get subscription: {e}")))?;
            Ok((id,))
        },
    )
    .map_err(|e| BackendError::Instantiation(format!("wire subscription.id: {e}")))?;
}
```

> `HostState` already carries `pub table: ResourceTable` (`engine.rs:231`) — use `.table`. Confirm the exact resource-method name string (`[method]subscription.id`) and the `Linker::resource` dtor signature against the bindgen output + wasmtime 36.0.9 in T0 (run `cargo check -p myrhiza-wasmtime-backend` after T1 and grep the generated bindings for `subscription`).

- [ ] **Step 4: Run the test + the existing wire tests.**

Run: `cargo test -p myrhiza-wasmtime-backend tests_wire`
Expected: PASS — new test green; `wire_state_propose_linker_accepts_deterministic_set`, `wire_interaction_linker_accepts_deterministic_set` still green (they pass a bound set without host.subscribe, so the new block is skipped).

- [ ] **Step 5: Commit.**

```bash
git add crates/wasmtime-backend/src/gating.rs crates/wasmtime-backend/src/engine.rs
git commit -m "feat(m1b): wire_linker binds subscribe + subscription resource for interaction"
```

---

### Task 7: `prewalk_imports` — permit the non-det instance for interaction

**Files:**
- Modify: `crates/wasmtime-backend/src/engine.rs:124-182` (prewalk allowlist)
- Test: backend fixture-link test (or unit if prewalk is unit-testable).

- [ ] **Step 1: Write the failing test.** Add a const for the instance name and a unit asserting the interaction allowlist includes it, state-apply does not:

```rust
#[test]
fn prewalk_allows_non_det_instance_for_interaction_only() {
    assert!(instance_allowed("myrhiza:kernel/host-non-deterministic@1.0.0", Profile::Interaction));
    assert!(!instance_allowed("myrhiza:kernel/host-non-deterministic@1.0.0", Profile::StateApply));
}
```

(If prewalk's allowlist is inline rather than a helper, extract a small `instance_allowed(name, profile) -> bool` helper as part of this task so it is unit-testable.)

- [ ] **Step 2: Run to verify it fails.**

Run: `cargo test -p myrhiza-wasmtime-backend prewalk_allows_non_det_instance`
Expected: FAIL.

- [ ] **Step 3: Add the non-det instance to the interaction allowlist.** In the prewalk allowlist, gate `HOST_NON_DETERMINISTIC_INSTANCE = "myrhiza:kernel/host-non-deterministic@1.0.0"` to `Profile::Interaction`, and audit its function children are each in `bound_imports` (so an interaction component importing `subscribe` it didn't declare still fails). Keep the existing types-only audit for `host-ui-surfaces`.

- [ ] **Step 4: Run + full backend suite.**

Run: `cargo test -p myrhiza-wasmtime-backend`
Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add crates/wasmtime-backend/src/engine.rs
git commit -m "feat(m1b): prewalk permits host-non-deterministic instance for interaction"
```

---

### Task 8: `call_on_subscription_update` — invoke the guest export

**Files:**
- Modify: `crates/wasmtime-backend/src/interaction_instance.rs` (replace T5 stub)
- Test: `crates/wasmtime-backend/tests/` (a fixture importing host.subscribe + exporting on-subscription-update) OR an existing interaction-instance test harness.

- [ ] **Step 1: Write the failing test** that instantiates an interaction fixture and delivers an update (the fixture from T11 may be needed; if so, write this test against a minimal fixture first or defer the assertion until T11 and keep this task to the call-site wiring with a compile+smoke check). Minimal smoke:

```rust
#[test]
fn on_subscription_update_invokes_export() {
    // Instantiate the multi-topic interaction fixture (T11), deliver an
    // update, and assert a subsequent view() reflects the delivered
    // state. Uses the fixture bytes from the test-utils helper.
    let mut inst = instantiate_multitopic_fixture();
    inst.call_on_subscription_update(7, &[1u8; 32], b"hello")
        .expect("delivery ok");
    let view = inst.call_view(b"primary", b"").expect("view");
    assert!(String::from_utf8_lossy(&view).contains("hello"));
}
```

- [ ] **Step 2: Run to verify it fails.** Run: `cargo test -p myrhiza-wasmtime-backend on_subscription_update_invokes_export`
Expected: FAIL (stub returns error).

- [ ] **Step 3: Implement the call** using the bindgen-generated export accessor (mirror `call_on_broadcast_completion`'s body in the same file):

```rust
fn call_on_subscription_update(
    &mut self,
    sub_id: u64,
    topic: &[u8],
    state: &[u8],
) -> Result<(), BackendError> {
    self.bindings
        .call_on_subscription_update(&mut self.store, sub_id, topic, state)
        .map_err(|e| BackendError::Instantiation(format!("on-subscription-update: {e}")))
}
```

(Exact accessor name comes from bindgen for the `world-interaction` export added in T1 — confirm against the generated `InteractionWorld` bindings.)

- [ ] **Step 4: Run.** Expected: PASS (after T11 fixture exists; if executing strictly in order, mark this test `#[ignore]` until T11 lands the fixture, then un-ignore — note this in the commit).

- [ ] **Step 5: Commit.**

```bash
git add crates/wasmtime-backend/src/interaction_instance.rs
git commit -m "feat(m1b): call_on_subscription_update invokes the guest export"
```

---

### Task 9: Read-only per-topic engine

**Files:**
- Create: `crates/kernel/src/subscription.rs`
- Modify: `crates/kernel/src/lib.rs` (`mod subscription;`)
- Test: `crates/kernel/src/subscription.rs` (unit, MemNetwork)

- [ ] **Step 1: Write the failing test** — a read-only engine for a topic materializes state from gossiped events:

```rust
#[tokio::test]
async fn readonly_engine_materializes_topic_state() {
    // Two handles on one MemNetwork: an authoring Runtime on topic T,
    // and a read-only engine subscribed to T. After the author emits an
    // event, the read-only engine's digest_watch reflects it.
    // (Use the existing test-utils harness for the authoring side.)
    let (net, topic) = mem_network_with_topic();
    let author = start_authoring_runtime(&net, topic).await;
    let ro = ReadOnlyEngine::start(&net, topic).await.expect("ro engine");
    author.propose_and_author(increment_intent(1)).await.expect("author");
    let state = poll_until(|| ro.latest_state(), |s| !s.is_empty()).await;
    assert!(!state.is_empty());
}
```

- [ ] **Step 2: Run to verify it fails.** Run: `cargo test -p myrhiza-kernel readonly_engine_materializes`
Expected: FAIL (type missing).

- [ ] **Step 3: Implement `ReadOnlyEngine`** as a thin wrapper that starts a `Runtime` with `author_key: None, propose: None` (read-only) and exposes its `digest_watch` receiver + a `latest_state()` snapshot. Reuse `Runtime::start` (per T0's confirmed signature). Keep it ~40 lines.

- [ ] **Step 4: Run.** Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add crates/kernel/src/subscription.rs crates/kernel/src/lib.rs
git commit -m "feat(m1b): read-only per-topic engine for subscriptions"
```

---

### Task 10: Subscription manager — drain, ensure engine, forward updates

**Files:**
- Modify: `crates/kernel/src/subscription.rs`
- Test: `crates/kernel/src/subscription.rs` (unit, MemNetwork)

- [ ] **Step 1: Write the failing test** — manager refcounts engines + forwards `digest_watch` to a sink:

```rust
#[tokio::test]
async fn manager_forwards_updates_and_refcounts() {
    let (net, t1) = mem_network_with_topic();
    let author = start_authoring_runtime(&net, t1).await;
    let sink = RecordingSink::new(); // records (sub_id, topic, state)
    let mut mgr = SubscriptionManager::new(net.clone(), sink.clone());
    let id = mgr.ensure(7, t1).await.expect("ensure");
    assert_eq!(id, 7);
    // second subscription to same topic shares the engine (refcount 2)
    mgr.ensure(8, t1).await.expect("ensure 2");
    author.propose_and_author(increment_intent(1)).await.expect("author");
    poll_until(|| sink.count_for(7), |c| *c >= 1).await;
    assert!(sink.count_for(8) >= 1);
    mgr.remove(7); mgr.remove(8);
    assert_eq!(mgr.engine_count(), 0, "engine torn down at refcount 0");
}
```

- [ ] **Step 2: Run to verify it fails.** Run: `cargo test -p myrhiza-kernel manager_forwards_updates_and_refcounts`
Expected: FAIL.

- [ ] **Step 3: Implement `SubscriptionManager`**: `BTreeMap<Topic, (ReadOnlyEngine, refcount)>` + `BTreeMap<sub_id, Topic>`; `ensure(sub_id, topic)` starts/refs an engine and spawns a forwarder task awaiting `digest_watch` → `sink.deliver(sub_id, topic, state)`; `remove(sub_id)` decrefs + tears down at zero. The `sink` is a trait so tests use a recorder and the kernel uses the interaction instance.

- [ ] **Step 4: Run.** Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add crates/kernel/src/subscription.rs
git commit -m "feat(m1b): subscription manager — refcounted engines + update forwarding"
```

---

### Task 11: Multi-topic interaction fixture + kernel wiring

**Files:**
- Create: a multi-topic interaction fixture (WIT importing `host.subscribe`, exporting `on-subscription-update` + `view`/`dispatch`) under the fixtures tree used by `crates/kernel/tests/helpers`.
- Modify: `crates/kernel/src/runtime.rs` (or a new per-instance interaction host) to hold the long-lived interaction instance, drain `take_pending_subscriptions()` after interaction calls, route the manager's deliveries into `call_on_subscription_update`, and expose an embedder view accessor + post-update `watch`.
- Test: covered by T12; this task builds the fixture + wiring.

> **Fixture build runs in the PRIMARY checkout** (non-nested), `wasm-tools` pinned to `1.248.0`.

- [ ] **Step 1: Author the fixture** (Rust → component): a `view` that renders its primary state plus any subscribed-topic states it has stored; an `on-subscription-update` that stores `(sub_id → (topic, state))` in a global/`RefCell` map (keyed by **`sub_id`, not topic** — re-subscribing a topic yields a new id, and a stale forwarder may still deliver to an old id which the fixture ignores); a `dispatch("subscribe:<hex topic>")` that calls `host::subscribe`. Keep app logic minimal (counter-style rendering of `"t<hex>=<state>"` lines). Document two invariants in the fixture source: (a) it never folds subscription metadata (sub ids, topic set, arrival order) into any bytes it would author or digest — subscription-derived state is peer-local accumulation only (spec §7.2); (b) it only subscribes to topic ids handed to it via `dispatch` (in-state enumeration / reachability), never ids constructed from arbitrary user strings (spec §9.1).
- [ ] **Step 2: Build + commit the fixture bytes** from the primary checkout:

```bash
# in the PRIMARY checkout (not a nested worktree):
just build-fixtures   # or the fixture crate's documented build
git add crates/kernel/tests/fixtures/<multitopic>.wasm <sources>
```

- [ ] **Step 3: Wire the kernel host.** Add the glue: hold the interaction instance; after each `call_view`/`call_dispatch`, drain `take_pending_subscriptions()` → `manager.ensure(sub_id, topic)` for each, AND drain `take_pending_unsubscriptions()` → `manager.remove(sub_id)` for each (the destructor-recorded drops — spec §4.2); implement the manager `sink` so a delivery calls `instance.call_on_subscription_update(...)` then re-publishes `view` bytes on a `watch::Sender<Vec<u8>>`; expose `RuntimeHandle`-style `view_watch()` + `render(peer_state)`. On instance teardown the resource destructor fires for every live handle, so the unsubscription drain also revokes everything at termination.
- [ ] **Step 4: Compile both feature sets.** Run: `cargo check -p myrhiza-kernel && cargo check -p myrhiza-kernel --features network-iroh`
Expected: PASS. Un-ignore the T8 test now that the fixture exists; run `cargo test -p myrhiza-wasmtime-backend on_subscription_update_invokes_export` → PASS.
- [ ] **Step 5: Commit.**

```bash
git add crates/kernel/ crates/wasmtime-backend/
git commit -m "feat(m1b): multi-topic interaction fixture + kernel subscription wiring"
```

---

### Task 12: Kernel acceptance suite (MemNetwork)

**Files:**
- Create: `crates/kernel/tests/subscribe_multitopic.rs`
- Modify: `crates/kernel/tests/helpers/mod.rs` (a `multitopic_interaction_handle()` helper + an aggregating-view assert)

- [ ] **Step 1: Write the acceptance tests.**

```rust
// 1) aggregate + re-render: subscribe to two sibling topics fed by two
//    authors; the interaction view contains BOTH topics' state, and
//    re-renders after an event applies on EITHER topic.
// 2) determinism guard (MANDATORY): two peers, identical event history
//    on all topics, different subscription sets -> identical per-topic
//    digests. (Locks subscription-state-never-in-digest, spec §7.2.)
// 3) drop unsubscribes: dropping the subscription handle fires the
//    destructor -> kernel drains take_pending_unsubscriptions ->
//    manager.remove; assert no further on-subscription-update arrives.
// 4) re-subscribe same topic: after drop, subscribing the same topic
//    yields a NEW sub_id; assert the new forwarder delivers under the
//    new id and the old id receives nothing (documents id-keyed, not
//    topic-keyed, tracking — spec §4.2).
// 5) reachability boundary: subscribing a fabricated/unenumerated 32-byte
//    topic returns a handle but delivers no updates (no peers) and never
//    affects any digest; assert no panic, no delivery (spec §9.1).
```

Write each as a `#[tokio::test]` using `poll_until` (no fixed sleeps), mirroring `propose_author.rs` structure.

- [ ] **Step 2: Run to verify they fail** (wiring incomplete edges). Run: `cargo test -p myrhiza-kernel --test subscribe_multitopic`
Expected: FAIL initially; iterate on T11 wiring until green.
- [ ] **Step 3: Make them pass** (fix wiring, not the tests).
- [ ] **Step 4: Run the whole kernel suite.** Run: `cargo test -p myrhiza-kernel`
Expected: PASS.
- [ ] **Step 5: Commit.**

```bash
git add crates/kernel/tests/subscribe_multitopic.rs crates/kernel/tests/helpers/mod.rs
git commit -m "test(m1b): multi-topic interaction acceptance suite (MemNetwork)"
```

---

### Task 13: iroh smoke test

**Files:**
- Create: `crates/kernel/tests/iroh_subscribe.rs` (under `#![cfg(feature = "network-iroh")]`)

- [ ] **Step 1: Write the test** — two real peers over the iroh harness: peer A's interaction component subscribes to peer B's topic; B authors; assert A's `view`/`view_watch` reflects B's state. One test. Mirror `iroh_propose_author.rs`.
- [ ] **Step 2: Run.** Run: `cargo test -p myrhiza-kernel --features network-iroh --test iroh_subscribe`
Expected: PASS (allow for gossip settle via `poll_until`).
- [ ] **Step 3: Commit.**

```bash
git add crates/kernel/tests/iroh_subscribe.rs
git commit -m "test(m1b): iroh smoke — cross-peer subscription delivery"
```

---

### Task 14: Docs + coverage + final gate

**Files:**
- Modify: spec status (already `[active]`), `docs/README.md` (add spec + plan entries under Runtime core), `docs/reports/2026-05-21-mvp-gap-analysis.md` (M1b receive-half landed), `tests/spec-coverage.md` (regen).

- [ ] **Step 1: Add README entries** for the spec + plan under Runtime core, cross-listed under the interaction/UI area; note M1b receive+aggregate half landed (B-13 was produce).
- [ ] **Step 2: Update the gap-analysis** with an M1b paragraph (multi-topic interaction via host.subscribe; windowing/DHT-resolver deferred).
- [ ] **Step 3: Regenerate coverage.** Run: `just spec-coverage` → commit `tests/spec-coverage.md`.
- [ ] **Step 4: Full gate.**

Run:
```
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features myrhiza-network/network-iroh -- -D warnings
cargo test -p myrhiza-kernel
cargo test -p myrhiza-kernel --features network-iroh --tests
just spec-coverage-check
cargo run -p dep-direction-check
```
Then `just ci` end-to-end from the primary checkout (it builds fixtures).
Expected: all green.

- [ ] **Step 5: Commit.**

```bash
git add docs/ tests/spec-coverage.md
git commit -m "docs(m1b): index + gap-analysis + coverage for host.subscribe multi-topic interaction"
```

---

## Self-review (author)

**Spec coverage:** §4 capability → T1/T2/T3/T6; §5 delivery → T8/T11; §6 kernel wiring → T9/T10/T11; §7 determinism → T2/T3/T12 (guard); §8 ABI checklist → T1–T7; §9 discovery → fixture `dispatch("subscribe:…")` in T11 (in-state enumeration exercised by the acceptance test feeding child topic ids); §10 windowing → manager `ensure/remove` is the window API (T10), policy=all; §12 scope → tasks stop at the seams; §13 testing → T12/T13 + determinism guard. All covered.

**Placeholder scan:** code given for every code step except the kernel `ReadOnlyEngine`/`SubscriptionManager`/fixture/wiring tasks (T9–T11), which specify exact types, fields, signatures, and ~line budgets but defer full bodies to execution because they depend on the T0-confirmed `Runtime::start` signature and bindgen names. T0 is the gating spike that removes that uncertainty before those tasks run.

**Type consistency:** `call_on_subscription_update(sub_id: u64, topic: &[u8], state: &[u8])`, `subscribe(topic) -> result<subscription,string>`, `on-subscription-update(sub-id: u64, topic, state)`, `record_subscription`/`take_pending_subscriptions`, `SubscriptionManager::{ensure,remove,engine_count}` — consistent across tasks.

**Known dependency edge:** the T8 delivery test needs the T11 fixture; T8 step 4 / T11 step 4 call out the `#[ignore]`→un-ignore handoff explicitly.
