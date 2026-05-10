**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — MVP


## 15. MVP

### 15.1 Acceptance criteria (lifted from PR #636)

**v1 acceptance** must demonstrate criteria 1-5:

1. The kernel loads and instantiates a WASM state component from a
   bundle fetched via iroh-blobs.
2. The component applies events deterministically; multiple peers
   running the same component bytes converge to the same state hash
   (verified via `state-digest`). Convergence is guaranteed only
   among non-equivocating authors per [convergence.md](convergence.md) §4.4.1.
3. A UI app loads an interaction component for that state, projects
   a view, submits a command, observes the resulting state change.
4. A second app instance (different state component, different
   topic) coexists on the same peer; events do not cross.
5. Capability declarations actually gate access — a component cannot
   import an interface its manifest does not declare.

**v1.1 acceptance** adds criterion 6:

6. A behavior component runs on a designated peer, observes events,
   and logs them.

The behavior profile + criterion #6 ship as v1.1 stretch goal — they
are not v1-blocking. Counter app's auto-reset-at-midnight behavior
component is the v1.1 demo target. v1 ships criteria 1-5 as the
acceptance bar.

### 15.2 MVP shape: counter + poll

Two minimal apps coexisting in the same kernel.

**Counter app**:
- State: `{ value: u64 }`
- Events: `Increment(by: i32)`, `Decrement(by: i32)`, `Reset`
- Permission gate on `Reset` (admin only).
- v1.1 behavior component: `auto-reset-at-midnight` running on a
  designated peer (acceptance criterion #6; v1.1 stretch per §15.1).

**Poll app**:
- State: `{ options: Vec<String>, votes: Map<peer, option_index>,
  ended: bool }`
- Events: `CreatePoll(options)`, `Vote(option_index)`,
  `EndPoll(creator-only)`
- Permission gate on `EndPoll` (only poll creator).

Both apps live in `examples/counter/` and `examples/poll/`. Each ~50–
150 LOC state-apply + ~100–200 LOC interaction. Total ~300–700 LOC
across both apps + manifests.

### 15.3 Test infrastructure

Multi-tier test hierarchy lifted from Willow:

- **State tier** (instant): unit-test each app's state-apply directly
  with crafted events. No kernel, no I/O.
- **Kernel tier** (fast): kernel + MemNetwork + apps in-process.
  Verifies per-app namespace, convergence, capability gating.
- **E2E tier** (slow): real iroh transport, two peer processes on
  loopback or two machines.
- **Browser tier** (slow): jco-shimmed kernel, headless Firefox,
  multi-tab convergence.

Test files live in `tests/{unit, integration, e2e}/`. The
`coexistence.rs` e2e test is the load-bearing acceptance test —
both apps in same kernel, no event-crossing, capability gating
verified.

### 15.4 Workspace shape

```
myrhiza/
├── Cargo.toml                 workspace root
├── crates/
│   ├── kernel/                runtime (host)
│   ├── sdk/                   app-author surface (state-apply / propose
│   │                          / interaction macros, manifest tools)
│   ├── network/               iroh wrappers, network trait
│   ├── storage/               event log, snapshot cache
│   ├── crypto/                primitive crypto host imports
│   └── ...
├── examples/
│   ├── counter/               wasm32 component, depends on sdk
│   │   ├── Cargo.toml
│   │   ├── manifest.toml
│   │   └── src/{state, propose, interaction, behavior}.rs
│   └── poll/                  same shape
└── tests/
    ├── unit/                  per-crate
    ├── integration/           kernel-with-MemNetwork, single peer
    └── e2e/
        ├── counter.rs
        ├── poll.rs
        ├── coexistence.rs     ⭐ load-bearing acceptance test (file extension is .rs not .cs)
        ├── multi_peer_convergence.rs
        └── capability_gating.rs
```

**Dependency direction** (load-bearing constraint): `examples/` →
`crates/sdk`. Kernel crates **never** depend on examples. Examples
never appear in `crates/`. Violation = bug.

### 15.5 Estimated v1 scope

**Honest range**: 24-32 weeks engineering effort for dual-stack v1
(both Wasmtime and jco backends, full capability gating, all v1-
mandatory list items, MVP apps, complete test tier). 16-20 weeks is
plausible only with 2-3 senior engineers full-time AND no major
surprises on the browser path.

**Critical-path items in dependency order**:

1. Workspace + core types — ~1 wk
2. State-digest format pin + WIT package authoring — ~2 wk
3. Manifest schema implementation + capability vocabulary — ~2-3 wk
4. Wasmtime backend with capability-gated linker (designed in from
   start, not retrofitted) — ~3-4 wk
5. Backend trait abstraction (Wasmtime impl satisfying it; jco impl
   designed in but not implemented yet) — ~1 wk
6. State-apply ABI + helper set + fuel + float-ban lint — ~3-4 wk
7. Per-call gating + manifest intersection — ~2 wk
8. Event/DAG primitives + topo-sort + PendingBuffer — ~2 wk
9. iroh integration + MemNetwork double — ~2 wk
10. HeadsSummary sync + drift detection — ~2 wk
11. Crypto primitives (host imports backed by Rust crypto crates) — ~1-2 wk
12. Bundle distribution + signing + revocation topic — ~2-3 wk
13. Counter + poll example apps + state-tier tests — ~1-2 wk
14. Kernel-tier tests (incl. coexistence) — ~2 wk
15. SDK macros + tooling — ~3-4 wk
16. **jco backend implementation** (against existing trait) — ~4-6 wk
17. Browser-tier tests — ~2-3 wk
18. v1.1: behavior profile + acceptance criterion #6 — ~1-2 wk

**Sum**: 33-46 weeks if all items run sequentially. With parallelism
(SDK macros, e2e tests, jco backend can overlap with later kernel
work), realistic: **24-32 weeks**.

(Note: the [implementation.md](implementation.md) §20 implementation outline lists 24 numbered items at a
finer granularity than this 18-item critical-path list — [implementation.md](implementation.md) §20 is the
detailed engineering plan; §15.5 is the schedule rollup. The numbers
are not contradictory; [implementation.md](implementation.md) §20 splits some items here into multiple
engineering steps for sequencing clarity.)

**v1 reduced-scope fallback**: if 24-32 weeks proves untenable, the
following cuts preserve architectural commitments while shrinking
schedule:

- **Defer jco backend to v1.5** (~4-6 wk savings). Risk: v1.5 slip
  pushes browser support out indefinitely. Spec mitigation: lock v1.5
  jco backend to a calendar deadline (e.g. "ships within 8 weeks of
  v1") with explicit ownership.
- **Defer behavior profile + criterion #6 to v1.1** (~1-2 wk savings).
  Already named as v1.1 candidate.
- **Defer per-call gating to v1.1** (~2 wk savings). Manifest
  intersection ([capabilities.md](capabilities.md) §7.2) and resource handles ([capabilities.md](capabilities.md) §7.4) preserved; per-call
  gates added later. Risk: gap window during which clipboard, file
  picker, etc. operate at module boundary, not per-call.

**Decision criteria for cutting**: if at week 16 the critical path
has slipped >2 weeks AND the browser backend is not at integration-
test stage, defer to v1.5 path. Otherwise hold dual-stack at v1.

**Out of v1 by design**: maintenance modules (zero ship); MLS module;
multi-device flow; scaling solutions; topic-ID rotation through dumb
relays; cross-app authority composition; bundle revocation distribution
beyond the per-author topic mechanism in [distribution.md](distribution.md) §10.7.


