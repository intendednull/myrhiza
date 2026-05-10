# Plan A foundation: handoff to plan B

**Status:** `[landed]` — plan A foundation complete on `feat/foundation-plan-a`.
**Date:** 2026-05-09.
**Audience:** the agent or human picking up plan B.

## What plan B inherits

Plan A delivered the foundation slice of the Myrhiza runtime: a typed
manifest pipeline, a Wasmtime-based state-apply backend with capability
gating + float-ban + fuel + memcap, a kernel install flow, a state-apply
handle abstraction, a digest emitter stub, and acceptance tests against
real wasm fixtures. The branch lands six kernel-tier acceptance tests
that exercise the full load-and-apply loop end to end.

### Crates

| Crate | Role | Plan A coverage |
|---|---|---|
| `myrhiza-types` | Canonical encoding, hashes, HLC, identity scopes, event envelope, limit constants | Complete; types frozen for v1 |
| `myrhiza-manifest` | TOML/bincode manifest schema + canonicalization, Ed25519 RFC 8032 verify, capability vocabulary | Complete; signing target commits to component content hash |
| `myrhiza-backend` | `Backend` + `ComponentInstance` traits, `BackendError`, `Verdict` | Complete; trait surface is the seam plan C's jco backend will implement |
| `myrhiza-wasmtime-backend` | Wasmtime engine, host-deterministic linker wiring, byte-level float-ban lint, capability gating, `StateApplyInstance` | Complete; deterministic helpers + host.log only |
| `myrhiza-kernel` | `InstallFlow` (load + verify), `StateApplyHandle` (apply + pre_check), `DigestEmitter` stub | Complete for plan-A scope; recursive module-dep resolution and revocation are explicit plan-B carry-over |
| `myrhiza-test-utils` | Manifest + bundle builders for tests | Complete |

### WIT package

`wit/myrhiza-kernel/wit/` carries the v1 ABI: `host-deterministic`,
`host-non-deterministic`, `host-async`, `host-ui-surfaces`, `types`,
plus the four world files (`state-apply`, `state-propose`, `interaction`,
`behavior`). The state-apply world is frozen — a snapshot test in
`crates/wasmtime-backend/tests/wit_freeze.rs` asserts the bindgen
output against `tests/snapshots/state-apply-world.bindgen.txt`.

### Gates

`just ci` runs `fmt-check`, `lint` (`clippy --all-targets -- -D warnings`),
`test` (`--workspace --all-targets`), and `spec-coverage-check` (the
`/// Covers:` matrix at `tests/spec-coverage.md`). 69 tests pass with
zero warnings and zero ignored.

### Acceptance tests

The kernel-tier acceptance tests at `crates/kernel/tests/acceptance.rs`
are the load-bearing evidence for mvp.md §15.1's foundation criteria:

| Test | Plan-A criterion | Specs covered |
|---|---|---|
| `kernel_loads_signed_bundle` | #1 (load) | mvp §15.1, verification §22.1 |
| `kernel_instantiates_and_applies_increment` | #1 (full loop) | mvp §15.1, convergence §4.4 |
| `capability_gating_rejects_non_deterministic_import` | #5 (gating) | mvp §15.1, capabilities §7.2 |
| `pre_check_returns_reject_and_does_not_commit` | (pre-check) | mvp §15.1, convergence §4.4 |
| `fuel_exhaustion_traps_apply` | (fuel) | mvp §15.1, determinism §5.3 |
| `float_banned_fixture_rejected_at_install` | (float-ban) | mvp §15.1, determinism §5.2 |

Each acceptance test wires real artifacts: a wasm component built by
`just build-fixtures`, a real Ed25519 keypair, a real Wasmtime
instantiation, real fuel + memcap accounting.

### Fixtures

Five wasm component fixtures under `tests/fixtures/`:

- `counter-state-apply` — apply mode happy path; hand-rolled big-endian i64 wire format.
- `over-importer` — imports `host-non-deterministic.random`; instantiation must fail.
- `pre-check-rejector` — apply always returns Reject; exercises fail-closed.
- `infinite-loop` — `apply` spins forever; exercises fuel exhaustion.
- `float-banned` — apply contains `f32.add`; exercises the float-ban lint.

All five are built by the same `_build-fixture` Justfile recipe:
`cargo build --release --target wasm32-unknown-unknown` for the core
module, then `wasm-tools component embed` + `component new` to wrap
into a component with the fixture's WIT. We deliberately avoid
cargo-component because the wasm32-wasip2 target links a WASI shim
that pulls in `wasi:io/poll`, `wasi:cli/exit`, etc. as component-level
imports — even for `#![no_std]` crates — which the deterministic-only
linker rejects.

## What plan B adds

Plan B is the networking and persistence layer. It builds on plan A's
seams but does not modify the v1 ABI:

- iroh-blobs integration for content-addressed bundle distribution
  (replaces plan A's local-directory bundle reads).
- Event DAG + gossip per network.md §11.
- Persistent identity store (replaces plan A's `0x<hex>` author pubkey
  with bech32m `wpub-author` HRP per distribution.md §10.2).
- Recursive module-dep resolution per modules.md §13.
- Revocation topic check during install per identity.md §6.7.
- Memory-network test double in `myrhiza-test-utils` for cross-peer
  convergence proofs.

## Known gaps left as TODO

These are explicit deferrals — none are bugs in plan-A code, but
plan B will need to address them before any production claim:

1. **Bindgen path.** `crates/wasmtime-backend/src/engine.rs` uses the
   `wasmtime::component::bindgen!` macro pointing at the production
   WIT, but the linker is wired by hand (`gating::wire_state_apply_linker`).
   This is intentional: per-import gating requires conditionally
   binding each helper based on the manifest. When plan B adds
   `host-non-deterministic` for state-propose / interaction / behavior,
   reuse the same hand-wired pattern; do not switch to the
   `add_to_linker` shortcut.
2. **bech32m identity strings.** Manifest `author_pubkey` is
   currently `0x<hex>`; distribution.md §10.2 calls for `wpub-author`
   bech32m. Plan B replaces `decode_author_pubkey_hex` in
   `crates/kernel/src/install.rs` with a bech32m decoder.
3. **Module-dep recursion.** Plan A's `InstallFlow::load` reads only
   the bundle's `state-apply` component. Multi-component bundles (UI +
   state-apply + behaviors) and recursive `[modules.dep]` resolution
   are plan B per modules.md §13.
4. **Revocation check.** Plan A does not consult a revocation topic
   during install. Plan B adds the gossip-DAG check per identity.md §6.7.
5. **Plan A's `host.install-key` and `host.verify-payload-mac` are
   declared in the WIT but not bound by `wire_state_apply_linker`.**
   Both take a `key-handle` resource whose infrastructure lands in
   plan B. A plan-A state-apply that imports them fails to link
   (which is the desired plan-A behavior; plan B replaces the failure
   with a real binding).
6. **Counter fixture wire format.** The fixture uses hand-rolled
   big-endian i64 instead of canonical bincode because deriving on a
   no-float struct with serde unconditionally pulls in
   `serde_core::de::Visitor::visit_f64` and float-Display format paths
   (which trip the byte-level float-ban lint at instantiation). For
   real apps, the canonical bincode discipline still applies; the
   fixture's hand-rolled encoding is a test-only measure.

## Acceptance evidence

- `git log feat/foundation-plan-a --oneline` — 25+ commits, all
  Conventional Commits, one concern per commit.
- `just ci` — green on the foundation branch (69 tests, zero warnings).
- `crates/kernel/tests/acceptance.rs` — six end-to-end acceptance
  tests pass.
- `tests/spec-coverage.md` — `/// Covers:` matrix maps tests back to
  spec sections.
- `tests/fixtures/built/*.wasm` — five wasm components reproducibly
  built via `just build-fixtures`.

## Plan B starting point

1. Create branch `feat/network-plan-b` off `main` (after plan A merges).
2. Read `docs/specs/2026-05-09-myrhiza-master-design/network.md` and
   `iroh.md` end to end before opening any iroh dependency.
3. The iroh integration belongs in a new crate
   `crates/myrhiza-network` behind a `Network` trait (mirror plan A's
   `Backend` trait — design the seam in from the start so the test
   double can satisfy the same trait without retrofitting).
4. The first acceptance test for plan B should be cross-peer
   convergence: two in-process kernels, one shared bundle, one
   originator and one receiver, observe that both peers commit the
   same state-digest after applying the same event sequence. The
   memory-network double makes this a unit test, not an integration
   test.
