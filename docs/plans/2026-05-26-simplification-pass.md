**Date:** 2026-05-26
**Status:** active
**Subject:** Post-v1 simplification pass — three PRs in parallel, sequential merge

# Simplification pass — coordinator plan

After the v1 acceptance bar landed (PRs #20–#24), the test suite locks behavior in. This plan executes the "after we get it working, let's think about a path to simplify" follow-on: trim duplication, indirection, dead parameters, and premature abstraction **without** removing features, changing the wire format, or weakening determinism guarantees.

Scoped by a fresh simplification-audit agent on 2026-05-26 against `main` at commit `bab9edd`. Three PRs land in sequence, prioritized by clarity-gain-to-risk ratio.

## Anti-targets (do NOT touch)

| What | Why |
|---|---|
| `tests/fixtures/built/*.wasm`, WIT files | Cross-language ABI; wire-frozen. |
| `myrhiza_types::canonical_bincode` contract | Cross-peer convergence depends on byte-identical encodings. |
| `BTreeMap` orderings in state-apply paths | Determinism load-bearing per CLAUDE.md. |
| `float_ban.rs` whitelist | Audit-load-bearing; documented per-row. |
| Capability-gating allow-lists | Criterion 5; security boundary. |
| `IrohPeerStack` `gossip` + `router` fields | Retained for drop ordering — dropping them risks "endpoint died mid-test." |
| `manifest!` macro `*_hash: None` boilerplate | Macro is the user-facing schema surface; hiding fields hurts self-documentation. |
| `runtime.rs` drift-detection scaffolding | Fully wired and load-bearing per B-4.6. |

## PR A — Drop dead parameters sweep (LOW risk)

Net delta: ~−75 LOC across 3 files + ~20 test call-sites. One PR, one branch.

**Target 3:** `EventBuilder::genesis` drops two unused leading-underscore params (`_app_bundle_hash`, `_topic_name`). Body uses only `seed` + `app_payload`. New signature: `genesis(seed: [u8; 32], app_payload: Vec<u8>) -> Event`. Update ~17 callers.

**Target 5:** `InstallFlow` is an empty unit struct with zero state and a single `&self` method `load`. Promote `load` to a free function `myrhiza_kernel::install::load(addr)`. Drop the struct. Update ~17 callers. Re-export shrinks by one identifier in `crates/kernel/src/lib.rs`.

**Target 6:** `DriftRateLimit::new` accepts a `now: Instant` parameter, immediately discards with `let _ = now;`. Doc explicitly says "accepted for symmetry" — cargo cult. Drop the param. Update 3 callers (runtime + 2 in drift.rs tests).

**Risk:** Low. All three are mechanical, cargo-check verifies completeness, no behavior change.

## PR B — Collapse per-profile triplication in wasmtime-backend (MED risk)

Net delta: ~−160 LOC in `crates/wasmtime-backend/src/{gating.rs, engine.rs}`. One PR.

**Target 1:** Three parallel surfaces in v1 (`validate_state_apply_manifest` / `validate_state_propose_manifest` / `validate_interaction_manifest`, plus `*_bound_imports`, `*_ambient_set`, `wire_*_linker`, `prewalk_*_imports`, and three near-identical `instantiate_*` methods) where the propose + interaction variants are pure delegations to state-apply.

Introduce one `Profile` enum (`StateApply`, `StatePropose`, `Interaction`) carrying the fuel budget, the float-ban flag, the prewalk's `allow_ui_surfaces`, and the bindings-instantiate callback. Replace per-profile delegations with `validate_manifest(m, profile)`, `bound_imports(m)`, `instantiate(profile, ...)`. Keep the public `Backend` trait shape unchanged.

**Risk:** Medium. Touches capability gating + fuel-budget wiring. Acceptance suite covers each profile end-to-end; refactor is mechanical equivalence.

**Constraints:** Do NOT change the float-ban whitelist itself. Do NOT touch the prewalk's `is_types_only` audit rule. Float-ban remains gated only on `StateApply`.

## PR C — Extract shared `signed_envelope` for publication + revocation (HIGH risk)

Net delta: ~−150 LOC across `crates/distribution/src/{publication.rs, revocation.rs, dispatch.rs}`. One PR.

**Target 2:** `PublicationEvent` and `RevocationEvent` carry near-identical structure: byte-identical `serde_bytes_64` modules, parallel `*SignedFields` structs, parallel `signing_target()` (modulo domain constant), parallel `apply` validation sequences (length check → seq-monotonic → seq-jump cap → pubkey-decode → verify_strict), and `dispatch::verify_revocation` ≡ `verify_publication` modulo `MAX_*_LEN`.

Extract one `signed_envelope.rs` exposing shared `serde_bytes_64`, a `SignedEnvelope` trait (`domain_sep()` + `signing_target()` + `signature() -> &[u8; 64]`), and a free `verify(envelope, author) -> Result<(), DispatchReject>` performing field-length + pubkey + signature gates once. Both event types implement the trait; both `apply` methods start with a single `verify(event, author)?` line. Dispatch helpers collapse to one generic function.

**Risk:** High by default — touches signature verification. Mitigations: wire format does NOT change (both envelopes already canonical-bincode through identically; trait only abstracts in-memory access). Domain-sep constants stay distinct. Existing tests cover signature-mismatch, malformed-pubkey, seq-monotonic, seq-jump-cap, and field-length paths.

**Two-commit shape inside the PR:**
1. Introduce `signed_envelope.rs`; migrate one log (publication).
2. Migrate the other (revocation); collapse dispatch helpers.

## Coordinator workflow

- **Worktrees**: `.claude/worktrees/simplification-{a,b,c}` — three branches off `main` at `bab9edd`.
- **Parallel implementers**: dispatch all three at once. Each gets the relevant section of this plan as its brief.
- **Two-stage review per PR**: spec compliance reviewer first ("does the change preserve all behavior, touch only the in-scope files, respect anti-targets?"), then code quality reviewer.
- **Sequential merge**: A first (lowest risk, smallest blast radius). After A lands, rebase B + C and re-trigger CI. Then merge B. Then rebase C and merge.
- **Verification gates per PR**: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `just ci`. Zero warnings.
- **Per-PR commit shape**: one or two commits inside the PR, conventional-commits prefixed `refactor:`.
