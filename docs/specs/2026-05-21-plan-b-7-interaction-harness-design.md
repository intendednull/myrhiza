**Date:** 2026-05-21
**Status:** draft
**Parent:** [mvp.md §15.1](2026-05-09-myrhiza-master-design/mvp.md)
**Subject:** Plan B-7 — native interaction harness + counter-interaction E2E (acceptance criterion 3)

# Plan B-7 — Native interaction harness + counter-interaction E2E

## 1. Goal

Satisfy [mvp.md §15.1](2026-05-09-myrhiza-master-design/mvp.md) v1 acceptance criterion 3:

> A UI app loads an interaction component for that state, projects a view, submits a command, observes the resulting state change.

B-7 ships the minimum surface to drive this end-to-end on a single peer:

1. Extends the `Backend` trait with `instantiate_state_propose` and `instantiate_interaction` constructors.
2. Wires Wasmtime's component-model `bindgen!` for the two additional WIT worlds.
3. Builds two new WASM fixtures: `counter-state-propose` and `counter-interaction`.
4. Ships a `counter-cli` native harness that loops `view → stdin → dispatch → propose → state-apply → loop`.
5. Adds an integration test that drives the harness with scripted input and asserts the final state matches the expected digest.

After B-7 lands, all five v1 acceptance criteria are met. B-6 (poll app), B-8 (SDK ergonomics), B-9 (storage), B-10 (iroh-blobs distribution) become demo polish.

## 2. Design choices (summary)

**Choice A — Kernel-side signing stays kernel-side.** The `state-propose` component returns event *payload bytes*; the harness wraps the bytes in a canonical envelope, signs with an in-process author keypair, and feeds the result into state-apply. `host.author-event` (WIT-declared on `host-non-deterministic`) is **not bound** in v1. Binding it would require: (a) putting a signing keypair into `HostState` (the linker closure must be `'static` so the key is cloned/Arc'd in at instantiation), and (b) wiring a `ResourceTable` to resolve the `identity-scope` record's inner `borrow<identity-handle>` to a real identity (`types.wit` declares `record identity-scope { long-term: borrow<identity-handle>, ... }`). Today's `HostState` is signing-key-free and its `ResourceTable` is unused. Avoiding `author-event` keeps B-7 within one PR-sized increment per slice and lets identity-handle resource plumbing land cleanly in a later identity-aware slice.

Runner-up: have `state-propose` call `host.author-event` so the WASM guest authors signed events directly. Rejected: requires `ResourceTable` wiring, signing-key access in `HostState`, and per-profile resource-management invariants. None of it is required for criterion 3.

**Choice B — host-import surface for v1 is host-deterministic only.** Both propose and interaction worlds *declare* imports against `host-non-deterministic`, `host-async`, and `host-ui-surfaces` in WIT. The counter fixtures actually need only `host-deterministic`. The wasmtime backend's per-profile prewalk + linker permits any subset of the ambient set; the manifest declares which subset is bound. Counter's manifest declares zero non-deterministic helpers for both new profiles → counter components instantiate without any non-det binding.

Runner-up: bind stub host-non-deterministic / host-async functions that trap on call. Rejected: introduces dead host imports we'd have to design once and rip out once, and creates a "looks-callable-but-isn't" trap surface that's hard to audit. The chosen approach matches the existing state-apply pattern: ambient set is the allowlist; manifest is the actual binding set; unauthorized imports surface as typed `BackendError::UnauthorizedImport` at prewalk.

**Choice C — `view(state, peer-state) -> bytes` returns UTF-8 text.** The interaction WIT declares `view`'s return as `list<u8>`; the kernel custodies the bytes opaquely. For v1 the contract between counter's interaction component and the CLI harness is: **bytes are UTF-8 text**, the harness writes them to `stdout`. This sidesteps the deferred `ui:*` rendering contract (see [`prior-art/willow/ui.md`](../prior-art/willow/ui.md) §"`ui:*` contract design") without precluding structured rendering later.

Runner-up: structured `ui-element` variant from `host-ui-surfaces`. Rejected: pulls the full `panel`/`button`/`form` rendering contract into v1, with no MVP demo that needs it.

**Choice D — `peer-state` is opaque app-defined bytes, custodied by the harness in-memory, always empty for counter v1.** Per Croquet's `viewId` precedent (`prior-art/croquet/programming-model.md` §4), per-peer view state diverges by design. Counter's interaction component reads `peer-state` but ignores it for v1; the harness initializes it to `[]` and passes it back round-trip. Persistence is deferred to B-9 (storage layer).

**Choice E — Pre-check via state-apply dry-run is exercised end-to-end in the harness.** Per [`prior-art/willow/authority.md`](../prior-art/willow/authority.md) §"pre-check = apply mechanic", the originator runs `state-apply` in dry-run before committing. The harness mechanically calls `pre_check` after `propose` and asserts verdict equality with the apply call. This validates Myrhiza's load-bearing "pre-check ≡ apply" invariant in real WASM, on every dispatch.

**Choice F — Slice sequence: 8 PRs at B-4-slice cadence.** Each slice is independently testable and lands one PR. The slice graph: bundle-signing extension → trait extensions → wasmtime backend (propose + interaction) → fixtures → harness binary → integration test. See §4 for the per-slice breakdown.

**Choice G — Multi-component bundles must be signature-covered.** Today's `signing_target_bytes(manifest, content_hash)` ([`crates/manifest/src/canonical.rs:98`](../../crates/manifest/src/canonical.rs)) commits to **one** content hash — the state-apply component's. Bundles with `state_propose.wasm` and `interaction.wasm` are content-integrity-protected for state-apply alone; the new components could be swapped after signing. B-7.0 generalizes `content_hash` into a **bundle-content-hash** computed by BLAKE3 over the canonical concatenation `BLAKE3(state_apply || state_propose || interaction || behavior)`, with absent components contributing `[0; 32]`. Wire-frame shape unchanged (still five length-prefixed fields per `distribution.md §10.2`); only the binding of field #3 is generalized. The kernel install path (`InstallFlow::load`) loads all declared component bytes and verifies the composite hash against the signing target before instantiating any of them.

Runner-up: ship multi-component bundles with content integrity only on state-apply, document the gap as a deferred security issue. Rejected: any production deployment of B-7's harness exposes a tamper hole; the fix is small (one canonical-bytes helper + the install verifier change) and lands cleanly as the first B-7 slice.

## 3. Architecture

### 3.1 Profile boundaries

| Profile | Export | Imports bound v1 | Imports declared but unbound v1 |
|---|---|---|---|
| `state-apply` (B-4 era) | `apply`, `state-digest` | host-deterministic (manifest-gated subset + always-on `host.log`) | `host.install-key`, `host.verify-payload-mac` (deferred to plan B) |
| `state-propose` (B-7) | `propose` | host-deterministic (manifest-gated subset + always-on `host.log`) | `host-non-deterministic.*` and `host-async.*` declared in WIT but not bound; component cannot import without manifest declaration → prewalk reject |
| `interaction` (B-7) | `view`, `dispatch`, `on-broadcast-completion`, `on-blob-fetch-completion` | host-deterministic (manifest-gated subset + always-on `host.log`); host-ui-surfaces (types-only, no functions to bind) | `host-non-deterministic.*` and `host-async.*` same as propose |

The two completion handlers (`on-broadcast-completion`, `on-blob-fetch-completion`) are exports the kernel calls when an async submit-and-poll request resolves. The v1 harness does not exercise them (no broadcast, no blob fetch); counter's interaction fixture emits them as no-op stubs and `ComponentInstance::call_on_*` methods are kernel-callable but unused by the harness.

**Prewalk allowlists** (per profile):

| Profile | Allowed top-level instances |
|---|---|
| state-apply | `myrhiza:kernel/host-deterministic@1.0.0`, `myrhiza:kernel/types@1.0.0` |
| state-propose | `myrhiza:kernel/host-deterministic@1.0.0`, `myrhiza:kernel/types@1.0.0` |
| interaction | `myrhiza:kernel/host-deterministic@1.0.0`, `myrhiza:kernel/types@1.0.0`, `myrhiza:kernel/host-ui-surfaces@1.0.0` |

`host-ui-surfaces` is types-only (no callable functions); its prewalk rule mirrors `types@1.0.0` — `Type` / `Resource` items permitted, `ComponentFunc` items rejected. A future WIT bump that adds methods to one of `host-ui-surfaces`'s records would surface as a `ComponentFunc` item and fail closed, matching the audit posture already established for `types@1.0.0`. `host-non-deterministic` and `host-async` instances appearing on a v1 component → typed `UnauthorizedImport` (since their function set is not in the manifest-gated binding subset for v1).

### 3.2 Backend trait extensions

```rust
// crates/backend/src/lib.rs (B-7 additions)

pub trait Backend: Send + Sync + 'static {
    // existing
    fn instantiate_state_apply(
        &self, component_bytes: &[u8], manifest: &Manifest,
    ) -> Result<Box<dyn ComponentInstance>, BackendError>;

    // B-7
    fn instantiate_state_propose(
        &self, component_bytes: &[u8], manifest: &Manifest,
    ) -> Result<Box<dyn ProposeInstance>, BackendError>;

    fn instantiate_interaction(
        &self, component_bytes: &[u8], manifest: &Manifest,
    ) -> Result<Box<dyn InteractionInstance>, BackendError>;
}

pub trait ProposeInstance: Send + 'static {
    /// Invoke `propose(prior_state, intent) -> result<list<u8>, string>`.
    /// On Ok, the returned bytes are the candidate event payload; the
    /// kernel runs state-apply pre-check against `prior_state` + a fresh
    /// envelope wrapping these bytes before signing.
    fn call_propose(
        &mut self, prior_state: &[u8], intent: &[u8],
    ) -> Result<Result<Vec<u8>, String>, BackendError>;
}

pub trait InteractionInstance: Send + 'static {
    /// Invoke `view(state, peer-state) -> list<u8>`. Returned bytes are
    /// the per-peer projected view; the v1 contract is UTF-8 text.
    fn call_view(&mut self, state: &[u8], peer_state: &[u8])
        -> Result<Vec<u8>, BackendError>;

    /// Invoke `dispatch(action) -> result<list<u8>, string>`. Returned
    /// Ok-bytes are the intent for `state-propose.propose`.
    fn call_dispatch(&mut self, action: &str)
        -> Result<Result<Vec<u8>, String>, BackendError>;

    // Completion handlers — bound but unused by the v1 harness.
    fn call_on_broadcast_completion(
        &mut self, token: &[u8], ok: bool, err: &str,
    ) -> Result<(), BackendError>;
    fn call_on_blob_fetch_completion(
        &mut self, token: &[u8], ok: bool, payload: &[u8], err: &str,
    ) -> Result<(), BackendError>;
}
```

`ComponentInstance` (state-apply) is left untouched. The trait split mirrors the WIT world split — each profile has its own instance trait with its profile-specific method set.

### 3.3 Wasmtime backend extensions

For each new world:

1. **`bindgen!` invocation** generates a distinct bindings type (`StatePropose`, `Interaction`) and a profile-shaped host-state-trait skeleton.
2. **`*_ambient_set()` + `*_bound_imports(manifest)`** functions enumerate what's allowed per profile (mirroring `state_apply_ambient_set`).
3. **`prewalk_*_imports(...)` function** walks the component's top-level imports against `bound_imports`, returning `UnauthorizedImport` for the first mismatch. Each profile has its own prewalk to avoid bleeding state-apply's restrictions onto the looser profiles.
4. **`wire_*_linker(linker, bound_imports)` function** binds the allowed subset.
5. **`*Instance` struct** wraps `Store<HostState>` + bindings, exposes the profile-specific call methods, maps wasmtime traps via the existing `map_wasmtime_error` helper.

The `HostState` struct itself is shared across all three profiles. Plan A's state-apply uses only the `LogSink` + `ResourceTable` fields; B-7's propose and interaction add no new fields (no `author-event` resource plumbing, no async-token registry). The `StoreLimits` memory cap is reused. Fuel budgets are profile-specific:

- `STATE_APPLY_FUEL_BUDGET_V1` = 10M (cross-peer-determinism critical; [`determinism.md §5.3`](2026-05-09-myrhiza-master-design/determinism.md))
- `STATE_PROPOSE_FUEL_BUDGET_V1` = 50M per [`determinism.md §5.3`](2026-05-09-myrhiza-master-design/determinism.md) (host-imposed resource cap; cross-peer determinism does not apply to propose since the kernel re-checks via state-apply)
- `INTERACTION_FUEL_BUDGET_V1` = 50M (heuristic; per-peer non-deterministic profile)

Stack cap and `max_wasm_stack` stay pinned at `MAX_WASM_STACK_V1` (512 KiB) per [`determinism.md §5.3`](2026-05-09-myrhiza-master-design/determinism.md) for both new profiles.

Float-ban: state-apply rejects banned floats. Propose and interaction are **non-deterministic profiles** so the float-ban lint is **not applied**. This mirrors the architecture table in [`architecture.md §3`](2026-05-09-myrhiza-master-design/architecture.md).

### 3.4 Manifest schema + bundle-signing extension

[`crates/manifest/src/lib.rs`](../../crates/manifest/src/lib.rs)'s `ComponentsSection` already has slots for `state_propose: Option<String>` and `interaction: Option<String>`. The **typed struct** needs no change.

The signing target **does** need extension (Choice G). [`signing_target_bytes(m, &content_hash)`](../../crates/manifest/src/canonical.rs) takes a single `EventHash`. B-7.0 introduces `bundle_content_hash(loaded_components: &LoadedComponents) -> EventHash`:

```
BLAKE3(
    BLAKE3(state_apply_bytes)
    || BLAKE3(state_propose_bytes_or_zeroes)
    || BLAKE3(interaction_bytes_or_zeroes)
    || BLAKE3(behavior_bytes_or_zeroes)
)
```

Absent components contribute `[0; 32]` (BLAKE3 of nothing in absentia is **not** valid; we use a sentinel so the byte-position of each slot is fixed). Order is **canonical** (state-apply, state-propose, interaction, behavior) — alphabetical-by-WIT-world-name would also work but the explicit order matches `ComponentsSection`'s struct field order which is already canonical per `manifest/src/schema.rs::canonicalize`.

The `signing_target_bytes` function signature stays `fn signing_target_bytes(m: &Manifest, content_hash: &EventHash) -> Vec<u8>` — only the callers (`InstallFlow::load`) change to pass the bundle-content-hash instead of the state-apply-only hash. Backward-compatible for single-component bundles: when only state-apply is present, the formula reduces to `BLAKE3(BLAKE3(state_apply) || [0;32] || [0;32] || [0;32])` which is **not** equal to `BLAKE3(state_apply)` — so existing fixture bundles need re-signing. The B-7.0 slice migrates the four existing acceptance/coexistence test fixtures + adds the multi-component bundle path; this is a one-time wire-frame change to bundle signing (not to event signing — events are unaffected).

The `LoadedBundle` struct in [`crates/kernel/src/install.rs`](../../crates/kernel/src/install.rs) gains `state_propose_bytes: Option<Vec<u8>>` and `interaction_bytes: Option<Vec<u8>>` (and `behavior_bytes: Option<Vec<u8>>` for v1.1 forward-compat); `InstallFlow::load` reads each from the manifest's declared path when present, computes the composite hash, and verifies against `signing_target_bytes`.

`wire_freeze.rs` regression test pins the new bundle-content-hash shape (the test currently pins Drift, HeadsSummary, and Event envelopes — bundle signing is **separate** from event/gossip wire-freeze and lands as its own pinned test).

### 3.5 Counter app components

Three components total, signed by one author keypair, packaged as a single bundle:

```
counter-bundle/
├── manifest.bincode
├── components/
│   ├── state-apply.wasm        # B-1 era, unchanged
│   ├── state-propose.wasm      # B-7
│   └── interaction.wasm        # B-7
```

**`counter-state-propose`** (~50 LOC): exports `propose(prior_state, intent) -> result<list<u8>, string>`. The `intent` argument is the bytes produced by `interaction.dispatch`.

The intent format is an **app-internal contract** between counter's interaction and propose components — it is **not** WIT-typed and **not** kernel-visible. The kernel treats intent bytes as opaque blobs. Counter's v1 intent vocabulary lives in code, documented inline in both fixtures' `lib.rs`:

```
intent[0]     = 0x00  // Increment
intent[1..9]  = i64 BE delta
```

Propose decodes the intent, validates the delta (e.g., `delta != 0`), and returns the 8-byte BE delta as the event payload bytes (matching counter-state-apply's expected non-genesis payload shape).

**`counter-interaction`** (~80 LOC): exports `view` and `dispatch`.

- `view(state, peer-state)`: state is 8-byte BE i64 counter; ignore `peer-state`; format as `"counter: <n>\n"` UTF-8.
- `dispatch(action)`: parse `action` as `"inc"`, `"inc N"`, `"dec"`, `"dec N"`; emit `intent[0]=0x00, intent[1..9]=delta BE`. Reject unrecognized actions.
- `on-broadcast-completion` / `on-blob-fetch-completion`: no-op stubs.

Both components are `#![no_std]` cdylib mirroring the existing counter-state-apply fixture. They get their own subdirectories under `tests/fixtures/counter-state-propose/` and `tests/fixtures/counter-interaction/`, each with its own `Cargo.toml`, `wit/` local copy, `src/lib.rs`, and `.gitignore`.

### 3.6 Native CLI harness

A new binary crate `crates/myrhiza-cli/` (bundle-agnostic, not counter-specific — per Q1 resolution: v1 ships one canonical CLI, B-6's poll app uses the same binary with `--bundle <poll-bundle-path>`):

```
main:
  - load bundle from --bundle path (or env)
  - instantiate state_apply, state_propose, interaction from bundle bytes
  - state = genesis app_payload (8 bytes BE i64 = 0)
  - peer_state = []  // v1: read-only, never mutated by the loop
  - loop:
      - view_bytes = interaction.call_view(state, peer_state)
      - write view_bytes to stdout
      - read action_line from stdin (or scripted)
      - if action == "quit", break
      - intent = interaction.call_dispatch(action)?
      - payload = propose.call_propose(state, intent)?
      - envelope = build_envelope(state, payload, author_key)  // see below
      - pre_check_result = state_apply.pre_check(state, envelope)
      - if Reject: print error, continue
      - apply_result = state_apply.apply(state, envelope)
      - assert pre_check_result.outcome == apply_result.outcome  // pre-check ≡ apply
      - if apply rejected: print error, continue
      - state = apply_result.new_state
```

**Envelope construction**: the harness uses [`myrhiza_test_utils::EventBuilder`](../../crates/test-utils/src/event_builder.rs) (already public, synchronous, signs body-hash via `AuthorKeypair`) to construct the signed `Event`, then serializes via `canonical_bincode()` to get the byte envelope state-apply expects. This is the established sync-signing path used by every B-1/B-4/B-5 test; it does not require Tokio. The async `runtime.rs::author` path is the *kernel's* signing route, which lives inside the multi-task runtime and is not reachable from a binary that doesn't spawn a `Runtime`.

**Why not pull the kernel runtime**: the harness's loop is single-tasked and synchronous; spinning up the full `Runtime` (with bus/network/drift/heads-summary handling) is unnecessary for criterion 3 and would entangle the harness with multi-peer code paths irrelevant to a single-peer demo. The harness reuses three smaller kernel pieces: `InstallFlow::load` (bundle loading), `state_apply::StateApplyHandle` (already public), and the new `state_propose::StateProposeHandle` + `interaction::InteractionHandle` (added in B-7.2/7.3 alongside the backend changes).

**Renaming caveat**: `myrhiza_test_utils` is currently named for its test role. The harness is **not** a test binary — it's a release binary. Moving `EventBuilder` into a public location is a B-7.0 sub-task: either re-home it under `myrhiza_kernel::event_builder` (preferred — single canonical home, harness imports from kernel) or expose `test-utils` as a non-test crate. **Recommendation**: re-home into `crates/kernel/src/event_builder.rs` as a public module; `test-utils` re-exports for backward-compat with existing tests.

### 3.7 Test infrastructure

Integration test at `crates/myrhiza-cli/tests/e2e.rs`:

- Builds a counter bundle in-test (signed with a deterministic keypair, all three components).
- Runs the harness with scripted input: `"inc 5\ninc 3\nquit\n"`.
- Asserts final state == `8_i64.to_be_bytes()` (== `[0, 0, 0, 0, 0, 0, 0, 8]`).
- Asserts `state_apply.call_state_digest(&final_state)` returns `[0, 0, 0, 0, 0, 0, 0, 8]` (counter's `state-digest` is identity per `counter-state-apply/src/lib.rs:239`).
- Asserts pre-check verdict equals apply verdict at every step (no divergence).

A separate unit test in `crates/wasmtime-backend/tests/profile_instantiation.rs` validates that each profile instantiation:
- Accepts the counter fixture bytes for its matching profile.
- **Wrong-profile instantiation surfaces from wasmtime's bindgen as a generic `Instantiation` error**, not `UnauthorizedImport`. The propose component does not export `apply` / `state-digest`; calling `instantiate_state_apply` on it returns `BackendError::Instantiation(...)` from the `StateApply::instantiate` typed-binding lookup. The test asserts the **variant** is `Instantiation` (not the inner string — string-matching is brittle).
- Rejects manifests that declare unauthorized imports → `UnauthorizedImport` (this is the manifest-validation error, separate from wrong-profile detection).

### 3.8 Justfile + fixture build

The `_build-fixture` recipe gets a third parameter `world_name` (or two recipe variants). New rules:

```
(_build-fixture "counter-state-propose" "counter_state_propose_fixture" "state-propose")
(_build-fixture "counter-interaction" "counter_interaction_fixture" "interaction")
```

`tests/fixtures/built/counter-state-propose.wasm` and `tests/fixtures/built/counter-interaction.wasm` land alongside the existing fixtures. CI's existing `build-fixtures` job picks them up.

## 4. Slice sequence

Each slice is one PR, sized at B-4-cadence (1-3 days focused work). All eight slices land before B-7 acceptance.

### B-7.0 — Multi-component bundle signing + EventBuilder re-home

Two coupled changes that gate every later slice:

1. **Bundle-content-hash**: add `bundle_content_hash(state_apply, state_propose, interaction, behavior) -> EventHash` to `crates/manifest/src/canonical.rs`. Update `crates/kernel/src/install.rs::InstallFlow::load` to: (a) read all four declared component paths from the manifest, (b) compute the composite hash, (c) verify against `signing_target_bytes(manifest, &bundle_hash)`. Existing fixture-write helpers in `install.rs::tests` and `crates/test-utils/src/bundle.rs` regenerate their signing target with the composite hash; single-component bundles still work because the formula admits absent components via `[0; 32]` sentinels. New `LoadedBundle` fields: `state_propose_bytes`, `interaction_bytes`, `behavior_bytes` (all `Option<Vec<u8>>`).
2. **EventBuilder re-home**: move `EventBuilder` from `crates/test-utils/src/event_builder.rs` to `crates/kernel/src/event_builder.rs` as `pub mod event_builder`. `test-utils` becomes a thin re-export. The harness in B-7.7 imports `myrhiza_kernel::event_builder::EventBuilder` — no test-only dep in a release binary.

Unit tests: composite hash for single-component bundle != raw state-apply hash (verifies the migration); composite hash matches across reorderings of `ComponentsSection` field iteration; install rejects tampered propose bytes with `InstallError::Signature`. Wire-freeze regression test pins the composite hash bytes for a known fixture.

**Why first**: every later slice writes or reads a multi-component bundle. Landing the security fix first means no slice ships unprotected.

### B-7.1 — Backend trait extensions

Add `ProposeInstance` + `InteractionInstance` traits, extend `Backend` with `instantiate_state_propose` + `instantiate_interaction`. `WasmtimeBackend` implements both as `BackendError::Instantiation("not yet wired")` placeholders so callers compile. No fixture build, no bindgen change.

**Why second**: locks the trait surface so B-7.2 / B-7.3 / B-7.7 have stable targets. Pure additive — no impact on B-5 acceptance tests.

### B-7.2 — Per-profile gating + prewalk infrastructure

Add to `crates/wasmtime-backend/src/gating.rs`: `state_propose_ambient_set`, `state_propose_bound_imports`, `validate_state_propose_manifest`, `interaction_ambient_set`, `interaction_bound_imports`, `validate_interaction_manifest`. Add to `crates/wasmtime-backend/src/engine.rs`: `prewalk_state_propose_imports` (allows `host-deterministic@1.0.0` + `types@1.0.0`), `prewalk_interaction_imports` (allows those plus `host-ui-surfaces@1.0.0`, with the types-only rule mirroring `types@1.0.0`). Add `wire_state_propose_linker` and `wire_interaction_linker` mirroring `wire_state_apply_linker`.

Unit tests: ambient sets enumerate the right capabilities per profile; manifest with state-apply-only capability declared for propose rejects with `UnauthorizedImport`; `host.log` is unconditionally bound; the new interaction prewalk accepts `host-ui-surfaces@1.0.0` types-only and rejects callable items inside it.

**Why third**: B-7.3 and B-7.4 are both "wire the linker, bind to instantiate_*" — landing the gating once before split avoids two parallel re-derivations.

### B-7.3 — `WasmtimeBackend::instantiate_state_propose`

Add a second `bindgen!` for `world: "state-propose"` in `engine.rs`. Add `ProposeInstance` impl in a new `crates/wasmtime-backend/src/propose_instance.rs` mirroring `instance.rs`. Wire `WasmtimeBackend::instantiate_state_propose` to call validate → prewalk → linker → store → bindings.

Unit test: instantiate counter-state-propose fixture bytes (built in B-7.5) and call `propose` with a synthetic increment intent; assert it returns 8 bytes of the BE-encoded delta. If B-7.5 hasn't shipped yet, the slice uses a hand-crafted minimal propose-world fixture inline (≤20 LOC test-only) — but per the sequence below, B-7.5 lands before B-7.3, so the real fixture is available.

### B-7.4 — `WasmtimeBackend::instantiate_interaction`

Same as B-7.3 but for interaction. Adds `bindgen!` for `world: "interaction"`, `InteractionInstance` impl in `crates/wasmtime-backend/src/interaction_instance.rs`, wires `instantiate_interaction`.

Unit test: instantiate counter-interaction fixture, call `view(empty_state, empty_peer_state)` and assert the byte output is recognizable UTF-8; call `dispatch("inc 5")` and assert it returns an intent matching counter's intent vocabulary.

### B-7.5 — Counter state-propose fixture

Add `tests/fixtures/counter-state-propose/` (Cargo.toml, wit/world.wit, src/lib.rs, .gitignore mirroring counter-state-apply). Justfile rule: parameterize `_build-fixture` with a third `world_name` argument; existing fixture calls keep `state-apply` literal. Acceptance: `just build-fixtures` produces `tests/fixtures/built/counter-state-propose.wasm`.

**Why before B-7.3 even though B-7.3 instantiates it**: the wasmtime-backend unit test in B-7.3 needs a real fixture to exercise. Landing the fixture first means B-7.3 ships with a passing test that uses real WASM (matching the established pattern from B-1 / B-5).

### B-7.6 — Counter interaction fixture

Add `tests/fixtures/counter-interaction/` mirroring B-7.5. Builds via the parameterized recipe from B-7.5. Acceptance: `just build-fixtures` produces `counter-interaction.wasm`.

### B-7.7 — myrhiza-cli harness + E2E integration test

Add `crates/myrhiza-cli/` workspace member (binary crate, depends on `myrhiza-kernel`). Add `crates/kernel/src/state_propose.rs` + `crates/kernel/src/interaction.rs` (handles mirroring the state-apply handle pattern). Wire the harness loop: load bundle → instantiate three profiles → loop view → stdin → dispatch → propose → envelope-build → pre-check → apply → assert verdict equality → repeat.

Tests in `crates/myrhiza-cli/tests/e2e.rs`: build counter bundle (all three components signed under one keypair), drive harness with scripted stdin `"inc 5\ninc 3\nquit\n"`, assert final state == `8_i64.to_be_bytes()`, assert pre-check ≡ apply on every step. Closes acceptance criterion 3.

**Ordering rationale**: B-7.5 + B-7.6 before B-7.3 + B-7.4 because real fixtures de-risk the backend slices. The actual implementation order is therefore: **B-7.0 → B-7.1 → B-7.2 → B-7.5 → B-7.6 → B-7.3 → B-7.4 → B-7.7**. The slice numbering is logical-grouping; the dependency-order is what the plan executes.

## 5. Deferred items (B-7 carryovers)

| Item | Trigger to revisit |
|---|---|
| `host.author-event` binding (WASM-side signing) | When propose components need to author multi-event sequences or sign with non-default identities. Requires `ResourceTable` + `identity-handle` resource plumbing. |
| `host-async.broadcast-submit` + completion handlers | When interaction components need to drive network broadcasts (e.g., chat send). Today the kernel's runtime owns broadcast; harness doesn't network. |
| `host-async.blob-fetch-submit` | When interaction components need to fetch blobs (e.g., images). Defer to B-10 (iroh-blobs distribution). |
| `host-async.http-request-submit` | Non-MVP capability. |
| `host-non-deterministic.now-hlc` / `random` / `user-prompt` binding | First fixture that needs them. Counter doesn't. |
| Structured `ui-element` rendering | When second-app demo (poll) ships and exercises the visual surface beyond text. B-6 candidate. |
| Persistent `peer-state` | B-9 (storage layer). |
| Browser-tier harness (jco) | v1.5+ per [mvp.md §15.5](2026-05-09-myrhiza-master-design/mvp.md). |
| Multi-peer interaction (P2P round-trip with two harness instances) | B-10 (iroh-blobs distribution + cross-process tests). |

## 6. Resolved decisions and open questions

**Resolved (no longer open):**

- **Binary name**: `myrhiza-cli` (bundle-agnostic). Counter is the first bundle it runs; poll (B-6) reuses the same binary with `--bundle <poll-path>`. All references in this spec use `myrhiza-cli`.
- **peer-state mutation in v1**: read-only. The harness initializes `peer_state = []` and threads it unchanged through every `view` call. The WIT signature `view(state, peer-state) -> bytes` does not surface an output peer-state, so the v1 contract is: `peer_state` is harness-owned read-only state; mutation requires a future WIT addition (`set-peer-state` export or `view` return-type change). Flagged as a deferred WIT item.
- **Pre-check vs apply divergence handling**: every step asserts equality; divergence is a fatal correctness bug, halts the harness with non-zero exit code, logs both verdicts. Tests assert no divergence on the golden path.

**Still open (require decision before B-7.7):**

1. **`AuthorKeypair` source for the harness**: deterministic seed (e.g., `AuthorKeypair::deterministic(0)`) baked in for the test, or read from a flag (`--author-key <path>`)? **Recommendation**: both. Default is deterministic-seed-0 for the integration test; a `--author-key <path>` flag overrides for actual deployments (reads bech32m-encoded keypair from disk per B-2's [`FilesystemIdentityStore`](../../crates/kernel/src/identity/fs.rs)).

2. **Genesis seed handling**: counter's state-apply expects the harness to construct a Genesis event with `seed = [0; 32]` and `app_payload = 0_i64.to_be_bytes()`. Should the harness compute and apply Genesis explicitly, or should the bundle ship with a pre-computed genesis stub the harness loads? **Recommendation**: harness constructs Genesis on first run. Subsequent runs (post-B-9 storage) read from disk.

3. **Stdin parsing UX**: counter's `dispatch("inc 5")` expects whitespace-separated. Should the harness echo each typed action back, support readline-style editing, or run dumb-pipe? **Recommendation**: dumb-pipe for v1 (scripts pipe stdin, no interactive shell yet); readline is B-8 polish.

## 7. Prior-art citations

- [`prior-art/willow/runtime-vision.md`](../prior-art/willow/runtime-vision.md) — four-profile table, MVP shape with criterion 3 quoted, two-entry-point `apply` + `propose` shape.
- [`prior-art/willow/state-machine.md`](../prior-art/willow/state-machine.md) — state-apply purity contract that grounds the propose→apply re-check.
- [`prior-art/willow/authority.md`](../prior-art/willow/authority.md) — pre-check ≡ apply mechanic; B-7.6's harness assertion enforces this in WASM.
- [`prior-art/willow/ui.md`](../prior-art/willow/ui.md) §"`ui:*` contract design" — UI-as-app reframe; v1 defers the structured `ui-element` rendering contract per Choice C.
- [`prior-art/willow/apps.md`](../prior-art/willow/apps.md) — counter MVP demo language ("~50 LOC state, ~100 LOC interaction") sets the fixture size budget.
- [`prior-art/wasm-component-model/wasmtime.md`](../prior-art/wasm-component-model/wasmtime.md) — embedder API: `Engine` + `Linker<T>` + `Store<T>` + per-profile fuel/epoch. The B-7 backend re-uses this shape exactly.
- [`prior-art/wasm-component-model/lessons.md`](../prior-art/wasm-component-model/lessons.md) — typed `Linker` resolves imports at link time; "world as unit of capability declaration." Validates the per-profile `bindgen!` approach.
- [`prior-art/croquet/programming-model.md`](../prior-art/croquet/programming-model.md) §1, §2, §10 — Model/View split = `state-apply`/`interaction`; `publish(scope, event, data)` is the conceptual ancestor of `dispatch → propose`. Runner-up paradigm; lockstep VM rejected, mechanics borrowed.
- [`prior-art/croquet/lessons.md`](../prior-art/croquet/lessons.md) — `viewId` per-peer view state precedent for the `peer-state` argument.
- [`prior-art/spritely-ocapn/architecture.md`](../prior-art/spritely-ocapn/architecture.md) — vat as unit of synchrony; near vs far refs informing the propose-vs-apply re-check boundary.

Gaps: no off-the-shelf precedent for a CLI bundle-launcher (closest is wasmtime CLI for single-component `wasi:cli/command`; Spin's trigger model is stateless event-driven, not loop-driven). B-7 invents this shape; explore promotion candidates in §8.

## 8. Promotion candidates (for `researching-prior-art`)

After B-7 lands, the following prior-art folders would inform follow-up work and are worth queueing:

- **`prior-art/elm-architecture/`** — TEA + Redux + Vue store all share the `dispatch(action) → update(msg, model) → (model, cmd)` shape, which is exactly what Myrhiza's `dispatch → propose → apply` codifies. Currently a gap.
- **`prior-art/wasmtime-cli/`** — wasmtime's `run` and `serve` host shapes, as a reference for any future Myrhiza CLI grow-up (daemon mode, HTTP surface). Not currently covered.
- **`prior-art/lamdera/`** or **`prior-art/replicache/`** — typed offline-first sync runtimes with mechanical view-of-state + intent-submission separation. Would inform how dispatch-rejection surfaces back through the interaction layer.

## 9. Acceptance criteria

B-7 ships when:

- [ ] `crates/manifest/src/canonical.rs` exposes a `bundle_content_hash` helper; `InstallFlow::load` verifies the composite signing target across all declared components.
- [ ] `EventBuilder` lives at `crates/kernel/src/event_builder.rs` and is publicly re-exported (existing `test-utils` re-export stays for backward-compat).
- [ ] `crates/backend/src/lib.rs` declares `ProposeInstance` + `InteractionInstance` traits and extends `Backend` with `instantiate_state_propose` + `instantiate_interaction`.
- [ ] `WasmtimeBackend` implements both new instantiation methods with per-profile gating, prewalk (interaction prewalk allows `host-ui-surfaces@1.0.0` types-only), and linker wiring.
- [ ] `tests/fixtures/counter-state-propose/` and `tests/fixtures/counter-interaction/` build clean WASM components via `just build-fixtures` (recipe parameterized with `world_name`).
- [ ] `crates/myrhiza-cli/` builds a single binary that loads a signed three-component counter bundle and runs the view → dispatch → propose → pre-check → apply loop with pre-check ≡ apply assertion every step.
- [ ] `crates/myrhiza-cli/tests/e2e.rs` drives the harness with scripted input `"inc 5\ninc 3\nquit\n"`, asserts final state == `[0, 0, 0, 0, 0, 0, 0, 8]`, asserts `state_digest(final_state) == [0, 0, 0, 0, 0, 0, 0, 8]` (counter's state-digest is identity).
- [ ] Wrong-profile detection test in `crates/wasmtime-backend/tests/profile_instantiation.rs` asserts `BackendError::Instantiation` (variant only, not string contents).
- [ ] `just ci` passes (fmt + lint + test + test-iroh + spec-coverage-check).
- [ ] [`docs/reports/2026-05-21-mvp-gap-analysis.md`](../reports/2026-05-21-mvp-gap-analysis.md) updates criterion 3 status from ❌ to ✅, marking 5/5 v1 criteria met.
