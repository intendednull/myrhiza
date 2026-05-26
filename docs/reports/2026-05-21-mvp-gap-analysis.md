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
| 16. Poll app | ✅ | **Shipped in B-6 (2026-05-26)**: four-component poll bundle (state-apply + propose + interaction + manifest) at `tests/fixtures/poll-*/`; 11 state-tier tests + 3 kernel-tier tests + coexistence-with-counter test green. Not v1-blocking (criterion 4 already satisfied by counter + echo) — shipped as second MVP demo app per [mvp.md §15.2](../specs/2026-05-09-myrhiza-master-design/mvp.md). |
| 17. State-tier tests | ❌ | Per-app state-apply unit tests not present — current tests use the in-Rust `counter_handle()` test helper, not real-app state-apply. |
| 18. Kernel-tier tests (convergence, coexistence, capability gating) | 🟡 partial | Convergence tested via B-1 + B-4. Capability gating tested in `crates/kernel/tests/acceptance.rs`. **Coexistence test (two apps in same kernel) missing** — load-bearing v1 demo per mvp.md §15.3. |
| 19. E2E test suite | 🟡 partial | In-process iroh integration tests landed in E2E-1 (2026-05-22) — `crates/kernel/tests/iroh_convergence.rs` + `iroh_coexistence.rs` route real `IrohNetwork` through real `Runtime` through real WASM; `crates/myrhiza-cli/tests/cli_binary.rs` exercises the binary entrypoint via subprocess. Remaining gap: cross-OS-process iroh convergence (deferred to E2E-2). See [docs/specs/2026-05-22-e2e-test-coverage-design.md](../specs/2026-05-22-e2e-test-coverage-design.md). |
| 20. SDK ergonomics (macros + tooling) | ❌ | No `crates/sdk/`. App authors today would write raw wit-bindgen + manifest TOML. |
| 21. jco backend | ❌ | Not started. Deferable to v1.5 per mvp.md §15.5 reduced-scope fallback. |
| 22. Browser-tier tests | ❌ | Depends on item 21. |
| 23. v1.1 behavior profile + criterion #6 | ❌ | Deferable per mvp.md §15.5. |
| 24. Dependency-direction CI check | ❌ | No examples yet to enforce direction against. |

**Tally**: 13 items ✅, 4 items 🟡, 7 items ❌ (4 of which are deferable per the mvp.md §15.5 reduced-scope fallback). Update 2026-05-26: item 16 (Poll app) flipped from ❌ to ✅ with B-6 (4-component poll bundle landed).

## v1 acceptance criteria status

Cross-checking against [mvp.md §15.1](../specs/2026-05-09-myrhiza-master-design/mvp.md):

| Criterion | Status |
|---|---|
| 1. Kernel loads + instantiates WASM state component from a bundle (iroh-blobs not required for the in-process tier) | ✅ Plan A acceptance tests prove this against the `counter-state-apply.wasm` fixture. |
| 2. Multi-peer convergence on same component bytes (verified via state-digest) | ✅ **Corrected 2026-05-21 during B-5 brainstorming**: `helpers::counter_handle()` (used by every B-1 + B-4 convergence test) loads the real `counter-state-apply.wasm` via `WasmtimeBackend::instantiate_state_apply` — every existing convergence test already runs on real WASM bytes. |
| 3. UI app loads interaction component, projects a view, submits a command, observes state change | ✅ **Shipped in B-7 (2026-05-21)**: `crates/myrhiza-cli/` harness drives the counter bundle's three components (state-apply + state-propose + interaction) through the `view → dispatch → propose → pre-check → apply` loop. E2E test at `crates/myrhiza-cli/tests/e2e.rs` asserts final state == `8_i64.to_be_bytes()` and pre-check ≡ apply on every step. |
| 4. Two apps coexist (different state component, different topic, same peer; events don't cross) | ✅ **Shipped in B-5 (2026-05-21)**: `crates/kernel/tests/coexistence.rs::two_apps_coexist_no_event_crossing` proves criterion 4 with counter + echo bundles on one peer. |
| 5. Capability declarations gate access (component cannot import undeclared interfaces) | ✅ Plan A `crates/kernel/tests/acceptance.rs` tests this with the `over-importer.wasm` fixture. |

**v1 blockers (post-B-7)**: none. All five v1 acceptance criteria are met. 5/5 ✅.

## Proposed slice sequence to v1 acceptance

Each slice = one PR, sized at ~B-4-slice cadence (1–3 days of focused work). Sequenced by dependency, with reduced-scope fallbacks for sub-items that can defer.

### ~~B-5: Counter app full + multi-peer convergence test~~ → SHIPPED 2026-05-21 (re-scoped to "Two-app coexistence + echo fixture")

The original B-5 scope was based on the incorrect read of criterion 2's status (see the dated correction above). The corrected B-5 scope shipped: built an `echo-state-apply` WASM fixture as the second app + a same-peer two-runtime acceptance test (`coexistence.rs::two_apps_coexist_no_event_crossing`). Closes criterion 4.

### ~~B-6: Poll app (not v1-blocking)~~ → SHIPPED 2026-05-26

**Scope** (as shipped): four-component poll bundle (state-apply + state-propose + interaction + manifest with `EndPoll` admin gate) at `tests/fixtures/poll-*/`. Per [mvp.md §15.2](../specs/2026-05-09-myrhiza-master-design/mvp.md), poll is the second of two MVP demo apps. Criterion 4 ("two apps coexist") was already satisfied by counter + echo in B-5, so poll was not v1-blocking — it ships as a non-trivial demo app for the v1 release showcase. First fixture exercising non-empty `deps` (which surfaced a parent-dedup bug in `EventDag::topo_sort_subset` fixed in `c96ffa7`) and first fixture using non-empty `peer_state` (harness now populates with local `AuthorPubkey`).

**Closes**: implementation.md §20 item 16; mvp.md §15.2 (second app for the v1 release). See [spec B-6](../specs/2026-05-26-b-6-poll-app-design.md) + [plan B-6](../plans/2026-05-26-b-6-poll-app.md).

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

**Next immediate slice (post-B-5): B-7 (native interaction harness + counter-interaction E2E)**. With criteria 1, 2, 4, 5 all ✅ post-B-5, criterion 3 is the sole v1 blocker. B-7 wires the counter app's interaction component to a CLI harness that projects state, takes user input, submits a command, and shows the resulting state change.

After B-7, all v1 acceptance criteria are met. B-6 (poll app), B-8 (SDK ergonomics), B-9 (storage), B-10 (iroh-blobs distribution) can land in any order driven by demo-readiness needs.

---

**Correction (2026-05-21)**: criterion 2 was originally listed as 🟡 partial with the rationale "B-1 + B-4 convergence tests use the `counter_handle()` native Rust state-apply, NOT the WASM component." This was wrong. Audit during B-5 brainstorming revealed `helpers::counter_handle()` (`crates/kernel/tests/helpers/mod.rs:33`) calls through to `counter_component_instance()` → `WasmtimeBackend::instantiate_state_apply` → a real wasmtime instance of `counter-state-apply.wasm`. Every existing convergence test runs on real WASM bytes; criterion 2 was ✅ shipped before B-5 began. The B-5 scope was re-pivoted to close criterion 4 (two-app coexistence) instead, which IS a genuine gap. The roadmap above reflects the corrected status.
