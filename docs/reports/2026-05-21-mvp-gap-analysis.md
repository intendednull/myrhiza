**Date:** 2026-05-21
**Status:** roadmap
**Subject:** Post-B-4 MVP gap analysis — what's needed to satisfy mvp.md §15.1 v1 acceptance

# Post-B-4 MVP gap analysis

This document audits what's shipped vs. what's left for the v1 acceptance bar from [mvp.md §15.1](../specs/2026-05-09-myrhiza-master-design/mvp.md). The B-4 iroh-integration sequence completed on 2026-05-21 (PRs #6–#14 over 8 slices: B-4.0–4.7 + B-4.8 carryover). With networking done, the remaining work is concentrated in the application + SDK + distribution layers.

## What's shipped

Cross-referenced against [implementation.md §20](../specs/2026-05-09-myrhiza-master-design/implementation.md)'s 24-item plan:

| Item | Status | Where |
|---|---|---|
| 1. Workspace scaffold + crates | ✅ | `crates/{backend, kernel, manifest, network, test-utils, types, wasmtime-backend}/` |
| 2. Core types (`EventHash`, `Topic`, etc.) | ✅ | `crates/types/` |
| 3. State-digest format pin (canonical bincode 1.3.x) | ✅ | `myrhiza_types::canonical_bincode` + wire-freeze tests |
| 4. WIT package authoring (state-apply, propose, interaction, behavior worlds) | ✅ | `wit/myrhiza-kernel/wit/*.wit` (5 files: types + 2 host worlds + interaction + behavior) |
| 5. Manifest schema + capability vocabulary | ✅ | `crates/manifest/` (typed Manifest, TOML, Ed25519) |
| 6. Wasmtime backend with capability-gated linker | ✅ | `crates/wasmtime-backend/` (~2,266 LOC: engine + float_ban + gating + helpers + instance) |
| 7. Backend trait abstraction | ✅ | `crates/backend/src/lib.rs` |
| 8. State-apply ABI + helper set + fuel + float-ban | ✅ | wasmtime-backend `float_ban.rs` (698 LOC) + `gating.rs` (507 LOC) + `helpers.rs` (205 LOC) |
| 9. Pre-check unification + drift detection scaffold | ✅ | Kernel's pre-check via state-apply dry-run; drift in `runtime.rs` |
| 10. Event/DAG primitives | ✅ | Plan B-1 (`crates/kernel/src/dag.rs` etc., topo-sort, PendingBuffer) |
| 11. Iroh integration | ✅ | B-4.0–4.4 (real `IrohNetwork` with subscribe/publish/request_heads, MemNetwork sibling) |
| 12. HeadsSummary sync + drift-detection gossip | ✅ | B-1 (HeadsSummary) + B-4.5/4.6 (direct-stream backfill + peer-authority index) |
| 13. Crypto primitives (host imports) | 🟡 partial | `myrhiza_manifest::verify_signature` (Ed25519) exists, but no `crates/crypto/` and no full host-import surface for app components. WIT files declare crypto host imports but the binding crate is missing. |
| 14. Bundle distribution + signing | 🟡 partial | Bundle install/load works (acceptance tests at `crates/kernel/tests/acceptance.rs` use real signed bundles). Revocation topic / per-author publishing flow not implemented. |
| 15. Counter app (state-apply + propose + interaction + manifest) | ❌ | Only the `counter-state-apply.wasm` fixture exists (`tests/fixtures/counter-state-apply/`). No propose/interaction components, no `examples/counter/` workspace member. |
| 16. Poll app | ❌ | Not started. |
| 17. State-tier tests | ❌ | Per-app state-apply unit tests not present — current tests use the in-Rust `counter_handle()` test helper, not real-app state-apply. |
| 18. Kernel-tier tests (convergence, coexistence, capability gating) | 🟡 partial | Convergence tested via B-1 + B-4. Capability gating tested in `crates/kernel/tests/acceptance.rs`. **Coexistence test (two apps in same kernel) missing** — load-bearing v1 demo per mvp.md §15.3. |
| 19. E2E test suite | ❌ | No real iroh-cross-process tests; B-4.4 acceptance tests use in-process two-`IrohNetwork`-peers (sufficient for protocol shape but not a true E2E). |
| 20. SDK ergonomics (macros + tooling) | ❌ | No `crates/sdk/`. App authors today would write raw wit-bindgen + manifest TOML. |
| 21. jco backend | ❌ | Not started. Deferable to v1.5 per mvp.md §15.5 reduced-scope fallback. |
| 22. Browser-tier tests | ❌ | Depends on item 21. |
| 23. v1.1 behavior profile + criterion #6 | ❌ | Deferable per mvp.md §15.5. |
| 24. Dependency-direction CI check | ❌ | No examples yet to enforce direction against. |

**Tally**: 12 items ✅, 4 items 🟡, 8 items ❌ (4 of which are deferable per the mvp.md §15.5 reduced-scope fallback).

## v1 acceptance criteria status

Cross-checking against [mvp.md §15.1](../specs/2026-05-09-myrhiza-master-design/mvp.md):

| Criterion | Status |
|---|---|
| 1. Kernel loads + instantiates WASM state component from a bundle (iroh-blobs not required for the in-process tier) | ✅ Plan A acceptance tests prove this against the `counter-state-apply.wasm` fixture. |
| 2. Multi-peer convergence on same component bytes (verified via state-digest) | 🟡 The B-1 + B-4 convergence tests use the `counter_handle()` native Rust state-apply, NOT the WASM component. A test that runs the WASM `counter-state-apply` across two peers and asserts digest convergence is the gap. |
| 3. UI app loads interaction component, projects a view, submits a command, observes state change | ❌ Needs the counter app's interaction component + a host-side launcher. Native (CLI) suffices for v1; jco-browser is v1.5+. |
| 4. Two apps coexist (counter + poll), events don't cross | ❌ Needs poll app + `coexistence.rs` test. |
| 5. Capability declarations gate access (component cannot import undeclared interfaces) | ✅ Plan A `crates/kernel/tests/acceptance.rs` tests this with the `over-importer.wasm` fixture. |

**v1 blockers**: criteria 2, 3, 4. Criterion 2 needs a multi-peer test against the real WASM component. Criteria 3 + 4 need example apps (counter interaction + poll state-apply/interaction) and a coexistence harness.

## Proposed slice sequence to v1 acceptance

Each slice = one PR, sized at ~B-4-slice cadence (1–3 days of focused work). Sequenced by dependency, with reduced-scope fallbacks for sub-items that can defer.

### B-5: Counter app full + multi-peer convergence test

**Scope**: build the counter example as a real component bundle (state-apply already exists as a fixture; add propose + interaction; manifest with capability declarations). Acceptance test: two `Runtime` instances on a shared `MemBus`, both load the counter bundle, one authors `Increment` events, the other converges via direct-stream backfill, both produce the same state-digest.

**Closes**: criterion 2 (multi-peer convergence on real WASM bytes); items 15, 17 partial.

**Estimate**: 2-3 days.

### B-6: Poll app + coexistence test

**Scope**: build the poll example (state-apply + propose + interaction + manifest with `EndPoll` admin gate). Acceptance test: a single `Runtime` with both counter and poll bundles loaded on different topics; events on one app don't appear on the other.

**Closes**: criterion 4 (coexistence); items 16, 18 (coexistence portion).

**Estimate**: 2-3 days.

### B-7: Native interaction harness + counter-interaction E2E

**Scope**: build a small CLI harness that loads an interaction component, projects state, takes user input, submits a command via the propose path. Acceptance: `cargo run -p counter-cli` increments the counter, prints the resulting state.

**Closes**: criterion 3 (UI/interaction loop, native-only — jco/browser remains v1.5+).

**Estimate**: 3-5 days. Larger because interaction-component dispatch is novel surface.

### B-8: SDK ergonomics + examples wiring

**Scope**: extract common app-author patterns into `crates/sdk/` (re-exports of canonical types; convenience macros if useful but not load-bearing). Wire the `examples/` workspace members into the Cargo workspace with the dependency-direction CI check.

**Closes**: items 20, 24.

**Estimate**: 2-3 days.

### B-9 (optional): Storage layer for runtime restart

**Scope**: persist the event log + state snapshot to disk; replay-on-restart. Not strictly v1-acceptance-critical (mvp.md §15.1 doesn't require restart-survival explicitly) but is critical for any non-toy deployment.

**Estimate**: 5-7 days.

### B-10 (optional): Bundle distribution polish

**Scope**: revocation topic, per-author bundle publishing, blob-fetch path via iroh-blobs (currently bundles are loaded from disk).

**Estimate**: 5-7 days.

### v1.5+ (post-v1)

- jco backend (item 21) — 4-6 weeks per mvp.md §15.5.
- Browser-tier tests (item 22).
- v1.1 behavior profile (item 23).
- B-4 carryovers (per-requester rate limiting, eviction-on-failure, cross-process tests).
- Wire-version envelope (first post-launch wire change).

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Interaction component dispatch is novel surface; WIT bindings may have unanticipated gotchas | Medium | Slows B-7 by days | Reference iroh-docs and Spritely interaction-component patterns in prior-art |
| The fixture-counter doesn't fully exercise the propose path — additional wiring may be needed in the kernel | Low | Adds 1-2 days to B-5 | Read existing `crates/kernel/src/install.rs` to confirm propose-path support |
| jco backend ships behind schedule, browser support slips | High | v1.5 slips into 2027 | Accept the deferral; ship v1 native-only |
| Multi-peer convergence on real WASM reveals determinism bugs in counter-state-apply | Low | Could derail B-5 by days | Wasmtime float-ban is in place; bump-allocator is deterministic |

## Open questions

1. **State persistence**: Does mvp.md §15.1 require it for v1? Reading §15.4 workspace shape suggests `crates/storage/` is in scope but the acceptance criteria don't mandate restart-survival. Treat as optional for v1 acceptance.
2. **Iroh-blobs for bundle fetch**: §15.1 #1 says "fetched via iroh-blobs" — this is a real requirement. Today bundles load from disk in tests; we need a real iroh-blobs integration path before v1 ships.
3. **Real cross-process tests**: §15.3 lists E2E tier as "slow" and depends on item 19. Should B-7 or B-8 include cross-process tests, or defer entirely?

## Recommendation

**Next immediate slice: B-5 (counter app + multi-peer convergence on real WASM)**. This is the highest-impact single PR — it converts the existing counter-state-apply fixture into a full app + closes criterion 2, the only criterion currently 🟡. After B-5, B-6 (coexistence) follows naturally; together they close criteria 2 + 4 and three of the deferred items (15, 16, 18 coexistence).

Then evaluate whether to push toward criterion 3 via B-7 (native interaction) or sidetrack into B-9/B-10 (storage + iroh-blobs distribution) depending on which gap looks more demo-load-bearing.
