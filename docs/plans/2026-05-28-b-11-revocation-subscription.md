**Date:** 2026-05-28
**Status:** landed
**Spec:** [docs/specs/2026-05-28-b-11-revocation-subscription-design.md](../specs/2026-05-28-b-11-revocation-subscription-design.md)
**Subject:** Plan B-11 — Revocation/publication subscription wiring implementation

# Plan B-11 implementation — Revocation/publication subscription wiring

> **For agentic workers:** REQUIRED SUB-SKILL: use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement task-by-task, and `superpowers:test-driven-development` within each task (every task lists its failing test first). The [spec](../specs/2026-05-28-b-11-revocation-subscription-design.md) is the design contract; this plan is the execution order. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Connect the `crates/distribution` pure tier (shipped inert in B-10) to the kernel `Runtime` so revocation/publication gossip is auto-subscribed on install, dispatched through `dispatch::verify_*` → `RevocationLog`/`PublicationLog::apply`, and surfaced via the `RuntimeHandle` poll-log pattern. Closes B-10 spec §6.4 and the [gap-analysis](../reports/2026-05-21-mvp-gap-analysis.md) item-14 footnote.

**Architecture:** 7 tasks T1–T7. Each task is one commit producing a buildable, test-green tree. Wire format change + network dep first (T1), then kernel dep + types (T2), then `Runtime` plumbing (T3) and dispatch handlers (T4), then mechanical call-site propagation + test accessors (T5), then the load-bearing iroh acceptance test (T6), then the final lint + dual-feature-matrix pass (T7). The drainer→`mpsc`→select-arm multiplexing (spec §3.2) mirrors the existing `internal_event_tx` pattern; the `Arc<Mutex<Vec<…>>>` surface (spec §3.5) mirrors `drift_log`/`peer_warnings`.

**Tech stack:** Rust 2024. No new external deps — `myrhiza-distribution` already exists; B-11 only changes its *visibility* (optional → unconditional in `crates/kernel`, new unconditional dep in `crates/network`). Existing `tokio` (sync/rt/macros/time), `ed25519-dalek`, `blake3`, `tracing`.

**Branch:** `worktree-b-10-revocation-wiring` (current worktree). Hygiene quick-wins are already staged on this branch; B-11 commits stack on top. (If splitting into two PRs is preferred, the hygiene commit and the B-11 commits are independent — see Self-review.)

---

## Pre-flight

- Worktree: `/mnt/storage/projects/myrhiza/.claude/worktrees/b-10-revocation-wiring/`. Run all `cargo`/`just` from here.
- **Worktree fixture caveat (spec §ops):** `just build-fixtures` / `just build-fixtures-check` CANNOT run from a worktree under `.claude/worktrees/` (the excluded standalone fixture crates bind to the *main* repo workspace — see the new `Justfile` note). B-11 touches only `crates/{network,kernel,types,test-utils}` + tests, none of which require rebuilt `.wasm`. The committed `tests/fixtures/built/*.wasm` (present in the worktree) satisfy all kernel-tier tests. Do **not** run `build-fixtures` here.
- Gate per task: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, the task's `cargo test` lines. Never `--no-verify`; root-cause every failure.
- Confirm `git status` clean (modulo the staged hygiene files) before each task.

---

## Task T1 — `GossipMessage::{Revocation, Publication}` variants + network→distribution dep + wire-freeze tests

**Spec:** §3.1, §6.1, §7.

**Files touched:**
- `crates/network/Cargo.toml` — add `myrhiza-distribution = { path = "../distribution" }` to `[dependencies]` (unconditional; pure tier has no iroh).
- `crates/network/src/lib.rs` — `use myrhiza_distribution::{RevocationEvent, PublicationEvent};`; append `Revocation(RevocationEvent)` then `Publication(PublicationEvent)` to `GossipMessage` **after** `Drift`. Do not reorder.
- `crates/types/tests/wire_freeze.rs` — two new discriminant-pinning tests.

**Test first:**
- [ ] Add `gossip_message_revocation_variant_tag_is_three_u32_be`: build `GossipMessage::Revocation(ev)` with a zero-signature `RevocationEvent`, `canonical_bincode(...)`, assert `bytes[..4] == [0,0,0,3]`. (Won't compile until the variant exists — that's the RED.)
- [ ] Add `gossip_message_publication_variant_tag_is_four_u32_be`: assert `[0,0,0,4]`.

**Implementation notes:**
- Confirm the existing `Event=0`/`HeadsSummary=1`/`Drift=2` wire-freeze tests stay green (no reorder).
- Verify dep direction: `cargo tree -p myrhiza-network -e features` shows distribution's pure tier pulled WITHOUT iroh (no `iroh`/`iroh-blobs` under `myrhiza-network` unless `network-iroh` is active).

**Verification:**
- [ ] `cargo check -p myrhiza-network`
- [ ] `cargo test -p myrhiza-network`
- [ ] `cargo test -p myrhiza-types --test wire_freeze`
- [ ] `cargo clippy -p myrhiza-network -p myrhiza-types --all-targets -- -D warnings`

---

## Task T2 — `crates/kernel` unconditional distribution dep + `RevocationApplied`/`PublicationAnnounced` types

**Spec:** §3.6, §4.2.

**Files touched:**
- `crates/kernel/Cargo.toml` — move `myrhiza-distribution` from optional to unconditional `[dependencies]`; drop `dep:myrhiza-distribution` from the `network-iroh` feature line but KEEP `"myrhiza-distribution/network-iroh"`.
- `crates/kernel/src/runtime.rs` — imports (`RevocationLog, PublicationLog, RevocationEvent, PublicationEvent, RevocationError, PublicationError, derive_revocation_topic, derive_publication_topic, dispatch` from `myrhiza_distribution`; `BlobHash` from `myrhiza_types`); `pub struct RevocationApplied` + `pub struct PublicationAnnounced` (spec §4.2) beside `DriftDetected`/`EquivocationFlag`.

**Test first:**
- [ ] In the `runtime.rs` `#[cfg(test)]` module, `revocation_applied_fields_accessible()`: construct a `RevocationApplied` with known values, assert each field round-trips. (RED until the struct exists.) Same for `PublicationAnnounced`.

**Implementation notes:**
- Critical: `cargo check -p myrhiza-kernel` with **no features** must pass — the pure tier compiles iroh-free. If it pulls iroh, the `dep:`/feature split in `Cargo.toml` is wrong.

**Verification:**
- [ ] `cargo check -p myrhiza-kernel` (no features)
- [ ] `cargo check -p myrhiza-kernel --features network-iroh`
- [ ] `cargo test -p myrhiza-kernel --lib` (the new field tests run)

---

## Task T3 — `Runtime` fields + `Runtime::start(installed_authors)` + `drain_distribution_sub`

**Spec:** §3.2, §3.3, §4.3, §4.4.

**Files touched:**
- `crates/kernel/src/runtime.rs` — Runtime struct fields (`revocation_logs`, `publication_logs`, `distribution_rx`, `revocation_events`, `publication_events`); RuntimeHandle fields (`revocation_events`, `publication_events`); `Runtime::start` gains `installed_authors: Vec<AuthorPubkey>` after `bootstrap`; inside `start` build the `mpsc::channel(256)`, subscribe both derived topics per author, spawn a `drain_distribution_sub` per subscription, construct the two `Arc<Mutex<Vec<…>>>` and wire into both the struct literal and `RuntimeHandle`; add the private `drain_distribution_sub` async fn (spec §4.4).

**Test first:**
- [ ] MemNetwork unit test `distribution_rx_receives_forwarded_message` in `runtime.rs` tests: subscribe a `MemNetwork` to `Topic::from_bytes(derive_revocation_topic(A))`, spawn `drain_distribution_sub(A, sub, tx)`, `MemNetwork::publish` a `GossipMessage::Revocation(ev)`, `rx.recv()` within a timeout, assert `(A, Revocation(ev))`. (RED until the drainer + channel exist.)

**Implementation notes:**
- Resolve spec §12.1: confirm the erased subscription type (`Box<dyn Subscription + Send>` vs generic) `erased.subscribe(...)` returns; make `drain_distribution_sub` bound match. The existing app-topic subscribe call in `start` is the template.
- `installed_authors` empty ⇒ zero extra subscriptions, zero behavior change (the safety net for all existing callers in T5).
- Verify `MemNetwork` routes by exact topic bytes (spec §12.3) so the distribution topic is isolated from the app topic.

**Verification:**
- [ ] `cargo check -p myrhiza-kernel --features network-iroh`
- [ ] `cargo test -p myrhiza-kernel --lib -- distribution_rx_receives_forwarded_message`

---

## Task T4 — Sixth select arm + `handle_distribution_message` + `handle_revocation` + `handle_publication`

**Spec:** §3.4, §3.5, §4.1.

**Files touched:**
- `crates/kernel/src/runtime.rs` — sixth `tokio::select!` arm in `run()` after the existing arms: `Some((author, msg)) = self.distribution_rx.recv() => self.handle_distribution_message(author, msg)`; `fn handle_distribution_message` (match `Revocation`/`Publication`/`_ → PeerWarning::DecodeFailed`); `fn handle_revocation` and `fn handle_publication` per spec §4.1 (verify-edge-first; clone prior log; insert-new + push event on Ok; `PeerWarning::SignatureInvalid` on verify Err; `PeerWarning::DecodeFailed` + re-insert prior on apply Err). All three are non-async.

**Test first (MemNetwork acceptance, no iroh):**
- [ ] `revocation_applied_on_valid_event` — `InProcessHarness` two peers; receiver `installed_authors=[A]`; publish `GossipMessage::Revocation(valid_signed_ev)` on the revocation topic; bounded wait; assert `peer.revocation_events()` contains the `RevocationApplied`.
- [ ] `invalid_sig_revocation_becomes_peer_warning` — wrong key; assert `revocation_events` empty + `peer_warnings` gains `SignatureInvalid`.
- [ ] `seq_not_monotonic_second_event_dropped` — two seq=1 events; assert exactly one `RevocationApplied`.
- [ ] Publication analogue for the happy path.

(These need the T5 accessors to compile; write the bodies here, let them block on T5, or land the accessors as the first sub-step of T4. Keep them in this task conceptually — they prove T4's logic.)

**Implementation notes:**
- Edge order is load-bearing (spec §3.4): `dispatch::verify_revocation` BEFORE `apply`. Do not collapse to apply-only.
- `RevocationLog::apply` consumes `self` → clone prior before calling; re-insert prior on `Err` (spec §5 last row).
- Spec §12.4: keep `handle_revocation`/`handle_publication` as two explicit methods (clarity over a generic).

**Verification:**
- [ ] `cargo test -p myrhiza-kernel` (no features — MemNetwork tests run)
- [ ] `cargo clippy -p myrhiza-kernel --all-targets -- -D warnings`

---

## Task T5 — Call-site propagation + `PeerHandle` accessors

**Spec:** §7 (modified surface).

**Files touched:**
- `crates/test-utils/src/harness.rs` — `InProcessHarness::spawn_peer` gains `installed_authors: Vec<AuthorPubkey>`, forwards to `Runtime::start`; add `PeerHandle::revocation_events() -> Vec<RevocationApplied>` and `publication_events() -> Vec<PublicationAnnounced>` (lock the RuntimeHandle Arcs, clone out).
- `crates/test-utils/src/iroh_harness.rs` — `IrohHarness::spawn_peer` gains the same parameter; forward; update the in-file self-test caller to `vec![]`.
- All existing call sites pass `vec![]`: `crates/kernel/tests/{iroh_convergence,iroh_coexistence,iroh_bundle_distribution,convergence,acceptance,coexistence,direct_backfill,halt_detection,peer_authority_index,perf_carryovers,persistence,poll,poll_state_apply,attribution}.rs` (grep for `spawn_peer(` and `Runtime::start(` to find the exact set; the grep is authoritative over this list).

**Test first:** none new — this is the compile-fixing pass. The assertion is a green full suite.

**Verification:**
- [ ] `cargo test --workspace --all-targets` (no features)
- [ ] `cargo test --workspace --all-targets --features network-iroh` (iroh tests compile; `iroh_revocation.rs` lands in T6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`

---

## Task T6 — `crates/kernel/tests/iroh_revocation.rs` (closes B-10 spec §6.4)

**Spec:** §6.3; B-10 spec §6.4.

**Files touched:**
- `crates/kernel/tests/iroh_revocation.rs` (new) — `#![cfg(feature = "network-iroh")]`, `#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]`.
- Optional: `crates/kernel/tests/helpers/mod.rs` — `sign_revocation_event` / `sign_publication_event` helpers if not better kept local to the test file.

**Test first:** the file IS the test. Write four `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` stubs with `todo!()`, confirm they compile under the gate, then fill bodies:
- [ ] `revocation_gossip_applies_and_surfaces` — receiver via `IrohHarness::spawn_peer(installed_authors=[A])`; publisher via `spawn_iroh_peer` (raw `IrohPeerStack::network`); publisher subscribes the revocation topic with peer-A bootstrap; settle sleep; `network.publish(revocation_topic, GossipMessage::Revocation(signed_ev))`; poll `peer_a.revocation_events()` in a `tokio::time::timeout(5s)` loop until the `RevocationApplied{A, bundle_hash}` appears.
- [ ] `publication_gossip_applies_and_surfaces` — analogous.
- [ ] `invalid_signature_becomes_peer_warning` — wrong key; assert empty events + `SignatureInvalid` warning.
- [ ] `seq_not_monotonic_second_event_dropped` — two seq=1; assert one accepted.

**Implementation notes:**
- Sign with `deterministic_signing_key(7)` (matches `build_signed_counter_bundle` author so the helper is consistent).
- `AuthorPubkey::from_bytes(sk.verifying_key().to_bytes())`; `Topic::from_bytes(derive_revocation_topic(author))`.
- Resolve spec §12.2: tune settle/poll timing against observed swarm-formation latency; mirror `iroh_convergence.rs` (≈300ms settle). Record observed numbers in the PR body.
- Publisher MUST subscribe the topic before publishing or iroh-gossip has no route (spec §6.3 note).

**Verification:**
- [ ] `cargo test -p myrhiza-kernel --features network-iroh --test iroh_revocation` (all four pass, non-flaky across 3 runs)

---

## Task T7 — Final lint + dual-feature-matrix + docs pass

**Spec:** all.

**Files touched:**
- `docs/reports/2026-05-21-mvp-gap-analysis.md` — append a dated note: item-14 footnote resolved; revocation/publication subscription wiring shipped in B-11; iroh_revocation.rs closes B-10 §6.4.
- `docs/README.md` — flip the B-11 catalog entry (added under Runtime core / App distribution by the spec) from `[active]` to `[landed]`; the B-10 cross-refs already point at this spec.

**Test first:** none — final CI-equivalent verification of everything written T1–T6.

**Verification:**
- [ ] `cargo fmt --all -- --check` (zero diff)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` (zero)
- [ ] `cargo clippy --workspace --all-targets --features network-iroh -- -D warnings` (zero)
- [ ] `cargo test --workspace --all-targets` (green)
- [ ] `cargo test -p myrhiza-network -p myrhiza-kernel -p myrhiza-test-utils --features network-iroh --tests` (green; mirrors `just test-iroh`)
- [ ] `cargo run -p dep-direction-check --quiet` (still OK — no `examples/*` change)
- [ ] Note: `just ci` from this worktree skips fixture rebuild (committed `.wasm` used); a final `just ci` on the **primary checkout** before merge is the canonical gate.

---

## Spec-coverage table

| Spec § | Requirement | Task | Test |
|---|---|---|---|
| §2 / §3.1 | `GossipMessage::{Revocation,Publication}`, wire-freeze | T1 | wire_freeze 3/4 tags |
| §3.2 | drainer→mpsc→select-arm multiplexing | T3, T4 | `distribution_rx_receives_forwarded_message` |
| §3.3 | auto-subscribe via `installed_authors` | T3 | iroh_revocation setup |
| §3.4 | verify-edge-first, then apply | T4 | `invalid_sig_*` + `seq_not_monotonic_*` |
| §3.5 | `Arc<Mutex<Vec>>` poll-log surface | T2, T3, T4 | `revocation_applied_on_valid_event` |
| §3.6 / §4.2 | `RevocationApplied`/`PublicationAnnounced` in kernel | T2 | field-access tests |
| §4.1 | full data flow | T3–T4 | MemNetwork acceptance |
| §6.3 / B-10 §6.4 | iroh propagation test | T6 | `iroh_revocation.rs` ×4 |
| §7 | call-site + accessor surface | T5 | green full suite |

## Determinism callouts

- `RevocationApplied`/`PublicationAnnounced` are **observation events**, not part of any state-apply path — no determinism contract. Surfacing order is the gossip-arrival order on this peer (per-peer, non-converging by design, like `drift_log`).
- `RevocationLog::apply`/`PublicationLog::apply` are pure (B-10 spec §4.4) and consumed unchanged. B-11 adds no non-determinism to them.
- No new host import / no WASM ABI change — purely kernel-resident wiring. (Confirms CLAUDE.md "Capabilities are the only host surface" is untouched.)

## Risks per task

See spec §5. Highest: T5's ~13-call-site churn (mechanical, `vec![]`) and T6's swarm-timing flakiness (bounded poll loop + 3-run check). T1 wire-freeze is the correctness backstop against accidental discriminant reorder.

## Estimate

2–3 days (spec §11). T1 ~0.5d · T2–T4 ~1d · T5 ~0.5d · T6 ~0.5–1d · T7 ~0.25d.

## Self-review (per writing-plans skill)

- **Each task buildable + test-green?** Yes — T1–T4 each gated by `cargo check`/`test`; T5 restores the full suite after the `Runtime::start` signature change; T6 adds the iroh test; T7 is the dual-matrix lint pass.
- **TDD honored?** Every task lists its RED test first (T5/T7 are propagation/verification passes — green suite is the assertion).
- **Independent of hygiene commit?** Yes — B-11 touches `crates/*` + tests + the two B-11 docs; the hygiene commit touches `docs/README.md`/`.gitignore`/`Justfile`/`ci.yml`/`dag.rs`. They can be one PR or two. Recommend **two PRs** (hygiene; then B-11) for the one-concern-per-PR rule — B-11's `docs/README.md` catalog edit (T7) is the only overlap and is trivially rebasable.
- **Reversibility:** all additive. `GossipMessage` variants append (no wire break); `installed_authors=vec![]` is a no-op for existing callers; new surface is purely additive.
