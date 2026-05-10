**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Verification


## 22. Verification

The convergence + capability + determinism commitments are
load-bearing. None of them can be hand-waved into existence; each
needs a corresponding test mechanic that catches regressions before
they ship. This section specifies the verification pipeline at the
master-spec level so plans A, B, and C inherit one discipline rather
than each reinventing it.

The verification surface has eight pieces:

1. Tier layout (where tests live).
2. Spec-coverage matrix (which spec section a test proves).
3. WIT/ABI freeze tests (catch convergence-breaking ABI drift).
4. Resource-cap regression (catch silent normative-constant changes).
5. Determinism property tests (state-apply purity).
6. Reproducible fixture builds (binary equivalence over time).
7. Cross-platform CI matrix (binary equivalence across OSes).
8. Test infrastructure crates (shared fixture builders + doubles).

### 22.1 Tier layout

Lifted from [mvp.md](mvp.md) §15.3 + Willow's `prior-art/willow/state-machine.md`.
Four tiers; each runs without depending on the slower tiers above
it.

```
tests/
├── state/                   tier 1: state-apply only, no kernel, no I/O
│   ├── counter.rs
│   ├── poll.rs
│   └── proptest_apply.rs    determinism property tests (§22.5)
├── kernel/                  tier 2: kernel + MemNetwork + apps in-process
│   ├── load_and_apply.rs
│   ├── capability_gating.rs
│   ├── pre_check_fail_closed.rs
│   ├── fuel_exhaustion.rs
│   ├── float_ban.rs
│   └── coexistence.rs       ⭐ load-bearing acceptance test
├── e2e/                     tier 3: real iroh transport, multi-process
│   ├── multi_peer_convergence.rs
│   ├── revocation_topic.rs
│   └── equivocation_first_seen.rs
├── browser/                 tier 4: jco-shimmed kernel, headless Firefox
│   ├── multi_tab_convergence.rs
│   └── nested_cm_memory_pressure.rs
├── snapshots/               §22.3 WIT-bindgen freeze artifacts
├── fixtures/                §22.6 reproducible-build fixtures
│   ├── counter-state-apply/ source
│   ├── poll-state-apply/    source
│   ├── float-banned/        source
│   ├── over-importer/       source
│   ├── infinite-loop/       source
│   ├── pre-check-rejector/  source
│   └── built/               committed wasm artifacts; nightly job
│                            rebuilds and asserts byte-equivalence
└── spec-coverage.md         §22.2 generated matrix
```

**Plan ownership**:

- **Plan A** delivers `tests/state/` + `tests/kernel/` (kernel-tier
  acceptance test for criteria #1 and #5 of [mvp.md](mvp.md) §15.1).
- **Plan B** adds `tests/e2e/` for multi-peer convergence, revocation
  topic backfill, equivocation first-seen.
- **Plan C** adds `tests/browser/` and populates `tests/state/` with
  counter+poll. Plan C also adds `coexistence.rs` to `tests/kernel/`.

**Dependency direction** (CI-enforced; per [mvp.md](mvp.md) §15.4):

- `tests/state/` depends only on the app crate under test (no kernel).
- `tests/kernel/` depends on `myrhiza-kernel` + `myrhiza-test-utils`.
- `tests/e2e/` depends on real `iroh` transport + `myrhiza-test-utils`.
- `tests/browser/` depends on `jco` toolchain + `myrhiza-test-utils`.
- `myrhiza-test-utils` depends on every workspace crate it doubles for
  but on no `tests/` directory. Tests never depend on tests.

### 22.2 Spec-coverage matrix

Every test in `tests/{state,kernel,e2e,browser}/` carries a doc
comment naming the spec sections it proves:

```rust
/// Covers: convergence.md §4.4 (pre-check fail-closed),
///         determinism.md §5.3 (fuel sharing).
#[test]
fn pre_check_reject_does_not_commit_state() { ... }
```

A `tests/spec-coverage.sh` script greps every `tests/` Rust file for
`/// Covers:` lines and produces `tests/spec-coverage.md` — a
section-by-section matrix:

```markdown
## convergence.md §4.4 — pre-check unification
- tests/kernel/pre_check_fail_closed.rs::reject_does_not_commit
- tests/kernel/pre_check_fail_closed.rs::accept_signs_event
- tests/state/proptest_apply.rs::pre_check_apply_agree
```

Sections with zero matching tests appear under `# UNCOVERED`. CI does
NOT block on uncovered sections — some sections are aspirational
direction (e.g. [identity.md](identity.md) §6.3 deferred items) — but PR review uses
the matrix to verify new behavior lands with a covering test.

**Convention**: when a test covers multiple sections, list each. When
a section is intentionally uncovered (deferred to a later plan), the
spec section itself names the gap (the existing pattern in
[risks.md](risks.md)). The matrix is generated, not hand-maintained.

### 22.3 WIT/ABI freeze tests

State-apply imports are normative per [architecture.md](architecture.md) §3.5; adding any
helper is a kernel-major bump per [distribution.md](distribution.md) §10.2. The kernel
WIT package is the contract; accidental drift is convergence-breaking.

**Mechanic**: snapshot the `wit-bindgen`-generated host trait skeleton
into `tests/snapshots/state-apply-world.bindgen.rs`. A test in
`crates/wasmtime-backend/tests/wit_freeze.rs` invokes the same bindgen
configuration used at runtime, captures the generated module text,
and diffs against the snapshot:

```rust
#[test]
fn state_apply_world_bindings_match_snapshot() {
    let generated = generate_state_apply_bindings();
    let snapshot = include_str!("../../tests/snapshots/state-apply-world.bindgen.rs");
    assert_eq!(generated, snapshot, "WIT/ABI drift detected; \
        update snapshot and bump kernel-major");
}
```

Drift produces a diff that names the changed signature. Acceptance
of drift requires:

1. Confirmation that the change is intentional.
2. Update `tests/snapshots/state-apply-world.bindgen.rs` (e.g. via
   `cargo insta accept`, or hand-edit).
3. Bump `kernel-major` in the kernel WIT package + manifest schema.
4. PR description names the spec section authorizing the change.

The freeze applies to ALL four worlds (state-apply, state-propose,
interaction, behavior) but state-apply is the convergence-load-
bearing one. The other three's snapshots are advisory until plans
B/C bind them.

### 22.4 Resource-cap regression

[determinism.md](determinism.md) §5.3 pins normative constants: 10M state-apply fuel,
50M state-propose fuel, 64 MB memory, 1 MB payload, 64 deps. These
must be the same across every v1 implementation per the convergence
argument.

The constants live in `myrhiza-types::limits`:

```rust
//! V1 normative resource caps per determinism.md §5.3.
//!
//! Bumping any constant requires:
//! 1. A kernel-major version bump (convergence-breaking).
//! 2. Updating the matching shadow value in
//!    crates/types/tests/limits_shadow.rs.
//! 3. A spec amendment naming the new value.

pub const STATE_APPLY_FUEL_BUDGET_V1: u64 = 10_000_000;
pub const STATE_PROPOSE_FUEL_BUDGET_V1: u64 = 50_000_000;
pub const COMPONENT_MEMORY_CAP_V1: usize = 64 * 1024 * 1024;
pub const EVENT_PAYLOAD_CAP_V1: usize = 1 * 1024 * 1024;
pub const DAG_DEPS_CAP_V1: usize = 64;

pub const HOST_HASH_FUEL_PER_BYTE: u64 = 5;
pub const HOST_VERIFY_SIGNATURE_FUEL: u64 = 5_000;
pub const HOST_VERIFY_PAYLOAD_MAC_FUEL: u64 = 1_000;
pub const HOST_INSTALL_KEY_FUEL: u64 = 100;
pub const HOST_NOW_HLC_FROM_EVENT_FUEL: u64 = 50;
pub const HOST_LOG_FUEL_BASE: u64 = 100;
```

`crates/types/tests/limits_shadow.rs` re-declares the same constants
as hard-coded literals and asserts equality. Drift surfaces in CI
via the lint test:

```rust
#[test]
fn fuel_budgets_match_spec_v1() {
    assert_eq!(myrhiza_types::limits::STATE_APPLY_FUEL_BUDGET_V1, 10_000_000);
    assert_eq!(myrhiza_types::limits::STATE_PROPOSE_FUEL_BUDGET_V1, 50_000_000);
    // ... one assertion per constant
}
```

This forces an author who wants to bump a constant to update both
the source and the shadow — a deliberate friction point. The shadow
file's git history is the audit trail of who bumped what when.

### 22.5 Determinism property tests

State-apply must be a pure function of `(prior state, event)`.
Property tests in `tests/state/proptest_apply.rs` exercise this
across arbitrary inputs:

```rust
proptest! {
    #[test]
    fn state_apply_is_pure(
        prior in arbitrary_state(),
        event in arbitrary_event(),
    ) {
        let mut h1 = fixture_handle();
        let mut h2 = fixture_handle();
        let r1 = h1.apply(&prior, &event)?;
        let r2 = h2.apply(&prior, &event)?;
        prop_assert_eq!(r1.outcome, r2.outcome);
        prop_assert_eq!(r1.new_state, r2.new_state);
    }

    #[test]
    fn state_digest_is_byte_stable(state in arbitrary_state()) {
        let mut h = fixture_handle();
        let d1 = h.state_digest(&state)?;
        let d2 = h.state_digest(&state)?;
        prop_assert_eq!(d1, d2);
    }

    #[test]
    fn pre_check_apply_agree(prior in arbitrary_state(), event in arbitrary_event()) {
        let mut a = fixture_handle();
        let mut b = fixture_handle();
        let pc = a.pre_check(&prior, &event)?;
        let ap = b.apply(&prior, &event)?;
        prop_assert_eq!(pc.outcome, ap.outcome);
        if pc.outcome == ApplyOutcome::Accepted {
            prop_assert_eq!(pc.candidate_state, ap.new_state);
        }
    }
}
```

`arbitrary_state()` and `arbitrary_event()` are app-specific
generators in `crates/test-utils`. Each app under test ships its
own generator pair. Plan A's counter app provides
`counter::arbitrary_state` returning a `BTreeMap<&str, i64>` with
seeded values + `counter::arbitrary_event` returning random
`Increment(by)` operations.

Property tests run as part of `cargo test --workspace`. Seed
discovery (proptest persists failure seeds to
`crates/<app>/proptest-regressions/`) is committed; rerunning a
seed must reproduce the failure deterministically.

**v1 status**: the `proptest!` harness above is the target shape.
Current plan-A coverage is smoke-only — single-input deterministic
tests in `crates/kernel/tests/acceptance.rs` exercise the
pre-check / apply agreement invariant (the third property above)
on a fixed `(prior, event)` pair, but do not yet generate over
arbitrary inputs. Building the `arbitrary_state` /
`arbitrary_event` generators and wiring proptest seed-persistence
is a plan-B deliverable; spec-coverage matrix entries pointing at
`verification.md §22.5` therefore reflect smoke coverage, not the
property-test surface this section ultimately specifies.

### 22.6 Reproducible fixture builds

Per [distribution.md](distribution.md) §10.10: kernel binary distribution leans on
reproducible builds for verifier-side trust. The same discipline
applies to test fixtures: `tests/fixtures/built/*.wasm` MUST be
byte-reproducible from `tests/fixtures/<name>/` source given the
pinned toolchain.

**Mechanic**: a `just rebuild-fixtures` recipe runs

```bash
cargo component build --release \
    --manifest-path tests/fixtures/<name>/Cargo.toml \
    --target wasm32-wasip2 \
    --locked --frozen
```

against each fixture and writes the output to
`tests/fixtures/built/<name>.wasm`. A nightly CI job
(`.github/workflows/repro-fixtures.yml`) re-runs the recipe and
asserts byte-equivalence with the committed `built/*.wasm` files.

**Failure handling**: nightly drift is **not** a merge blocker.
Toolchain reproducibility is brittle (rustc minor bumps, target
linker changes, system library drift). The job posts a known-issue
summary and the maintainer chooses whether to refresh fixtures.
Refreshing fixtures requires:

1. Run `just rebuild-fixtures` locally.
2. Commit updated `built/*.wasm` with rationale ("rustc 1.85.1 minor
   bump shifted .wasm bytes by 4; behavior unchanged").
3. PR title `chore(fixtures): rebuild fixtures for <reason>`.

Fixture rebuilds NEVER pair with code-behavior changes in the same
PR — keeps audit trail clean.

### 22.7 Cross-platform CI matrix

Convergence is the load-bearing property of Myrhiza. Two peers on
different OSes ingesting the same event log MUST produce
byte-equivalent `state-digest()` output. CI enforces this with a
matrix job over `ubuntu-latest`, `macos-latest`, `windows-latest`.

**Mechanic**: a `tests/kernel/cross_platform_digest.rs` integration
test loads a fixed-seed event sequence (committed under
`tests/fixtures/digest-replay/events.bincode`) into a fresh kernel +
fixture state-apply, runs apply for every event, and writes the
final digest bytes to `target/digest-replay-<platform>.json`:

```json
{
    "platform": "ubuntu-latest",
    "fixture": "counter-v1-replay-1",
    "rust_version": "1.85.0",
    "events_applied": 1024,
    "final_digest_hex": "af1349..."
}
```

Each platform uploads its artifact. A final job
(`cross-platform-digest-equivalence`) downloads all three artifacts
and asserts:

```rust
assert_eq!(linux.final_digest_hex, macos.final_digest_hex);
assert_eq!(linux.final_digest_hex, windows.final_digest_hex);
```

Mismatch is a merge blocker. The diff names the platform pair that
diverged + the digest values, so triage can isolate which event in
the replay sequence first introduced divergence (binary-search via
truncated replays).

The `events.bincode` fixture is regenerated ONLY when the counter
state-apply ABI changes; the regeneration script is committed at
`tests/fixtures/digest-replay/regenerate.sh` and produces canonical
output deterministically.

### 22.8 Test infrastructure crates

A workspace crate `crates/test-utils` consolidates shared fixture
builders, network doubles, and proptest generators. It is **not**
published; consumers are tests + dev-builds only.

```
crates/test-utils/
├── Cargo.toml          dev-only; no production dependents
└── src/
    ├── lib.rs
    ├── manifest.rs     test_manifest() builders for common shapes
    ├── bundle.rs       write_signed_bundle() helper for tempdir tests
    ├── mem_network.rs  MemNetwork double for kernel-tier tests (plan B)
    ├── proptest_gen.rs arbitrary_state / arbitrary_event registry
    └── digest_replay.rs digest-replay fixture readers
```

Plan A populates `manifest.rs` + `bundle.rs`. Plan B adds
`mem_network.rs`. Plan C adds the proptest generators that depend
on counter/poll app types.

**Dependency rule**: `test-utils` may depend on every workspace crate
under `crates/` but NEVER on anything under `tests/`. Tests depend
on `test-utils`; `test-utils` doesn't depend on tests.

### 22.9 Plan-by-plan handoff summary

| Verification piece | Plan A | Plan B | Plan C |
|---|---|---|---|
| Tier layout (§22.1) | scaffold state + kernel | add e2e | populate state, add browser |
| Spec-coverage matrix (§22.2) | scaffold script + matrix | extend | extend |
| WIT/ABI freeze (§22.3) | snapshot all 4 worlds | re-snapshot if WIT bumps | re-snapshot if WIT bumps |
| Resource-cap regression (§22.4) | constants + shadow | extend if §5.3 grows | extend if §5.3 grows |
| Determinism property tests (§22.5) | smoke pre-check/apply agreement (single input) | scaffold proptest harness in state-tier + hash-based event-stream proptest | counter+poll generators |
| Reproducible fixtures (§22.6) | initial fixtures committed | network fixtures (events.bincode) | counter+poll fixtures |
| Cross-platform CI (§22.7) | digest-replay job (state+kernel only) | extend with e2e topology fixture | extend with browser tier |
| test-utils crate (§22.8) | manifest+bundle helpers | mem-network double | proptest generators |

### 22.10 Out of scope at v1

- **Formal verification of state-apply purity** (e.g. coq-style
  proofs). v1 leans on byte-level + property-test discipline.
  Future direction: TLA+ or Lean spec for the convergence argument.
- **Mutation testing** of state-apply. High signal but high cost;
  defer to v2.
- **Continuous fuzzing** (e.g. cargo-fuzz target on the manifest
  parser, the wasm float-ban scanner). v1 ships unit + property
  tests; nightly fuzz is a future direction.
- **Performance regression CI**. v1 acceptance is correctness, not
  throughput. Performance benchmarks land alongside the v2 scaling
  work named in [convergence.md](convergence.md) §4.5.

### 22.11 Prior art

- Tier hierarchy: Willow's state-machine test layout
  (`prior-art/willow/state-machine.md`) is the source. Myrhiza
  inherits the tier names + dependency direction.
- WIT freeze test mechanic: standard `cargo-insta`-shape snapshot
  pattern used across the Rust ecosystem (e.g. `wit-bindgen` itself,
  `serde_json`).
- Reproducible-build pattern: aligns with `cargo --locked --frozen`
  + `rust-toolchain.toml` pinning recommended by the cargo team and
  used by debian-reproducible.
- Cross-platform digest-equivalence: lifted from the broader
  determinism-testing playbook (e.g. ICU collation tests, SQLite's
  cross-platform regression suite).

No new prior-art folder needed; existing `prior-art/willow/` is
referenced for tier hierarchy. If Myrhiza accepts byzantine-fault
testing as an explicit v2 obligation, a `prior-art/byzantine-test/`
folder lands alongside that work.

