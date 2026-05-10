# Foundation Review Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` to execute task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Address all 22 findings (6 critical, 8 important, 8 minor) from the four-reviewer swarm against PR #1 `feat/foundation-plan-a`, so the foundation merges with no known wire-format breaks, sandbox-escape vectors, or acceptance-bar overclaim.

**Architecture:** Spec amendments first (so code references authoritative spec), then code fixes ordered by isolation, then tests + cleanup. Lands as commits on existing `feat/foundation-plan-a` branch — no new branch, no rebase.

**Tech Stack:** unchanged (Rust 1.95, wasmtime 36.0.9 LTS, ed25519-dalek 2.1, bincode 1.3.3, wit-parser 0.236).

**Decisions made up front (so subagents don't re-litigate):**

- **Crit #2 (signing target framing):** amend spec to include `DOMAIN_SEP` as a fifth length-prefixed field. The code is already correct; framed domain-sep is the cryptographically defensible choice (no prefix/suffix collision risk).
- **Imp #9 (capability vocab name):** capability key is `"host.broadcast"` per `architecture.md §3.5`; WIT wire name remains `broadcast-submit` per `abi.md §8.5`. Vocabulary, fixture manifest, and spec example must all use `host.broadcast`.
- **Imp #11 (`borrow<key-handle>` semantics):** amend `determinism.md §5.1` to declare `borrow<key-handle>` for `verify-payload-mac` (resource-handle borrow, not move). Move semantics for `install-key` (it consumes the handle binding).
- **Imp #10 (`host.install-key` + `host.verify-payload-mac`):** drop from the v1 state-apply ambient set; explicitly defer to plan B. Vocabulary keeps the names registered (still authored capabilities) but `validate_state_apply_manifest` rejects them with a `DeferredToPlanB` variant.
- **Crit #5 + #6 acceptance + freeze:** all 4 worlds get freeze snapshots; named-render not Debug-printed `idx`. Manifest-arm acceptance test added against the counter fixture with an over-declared `host.broadcast`.

---

## Task 1: Spec amendments (single docs commit, no code)

**Files:**
- Modify: `docs/specs/2026-05-09-myrhiza-master-design/distribution.md` §10.2 (signing target framing)
- Modify: `docs/specs/2026-05-09-myrhiza-master-design/distribution.md` §10.2 (TOML example: `host.broadcast` capability key)
- Modify: `docs/specs/2026-05-09-myrhiza-master-design/architecture.md` §3.5 (capability table — confirm `host.broadcast`)
- Modify: `docs/specs/2026-05-09-myrhiza-master-design/determinism.md` §5.1 (`borrow<key-handle>` for verify-payload-mac, value semantics for install-key)
- Modify: `docs/specs/2026-05-09-myrhiza-master-design/determinism.md` §5.3 (kernel_major v1 reject at install)

- [ ] **Step 1.1: amend `distribution.md §10.2` signing target to 5 length-prefixed fields**

Replace the signing-target body with:
```
signing_target = length_prefix("myrhiza/manifest/v1") |
                 length_prefix(BLAKE3(manifest_body_without_signature)) |
                 length_prefix(content_hash) |
                 length_prefix(version) |
                 length_prefix(author_pubkey)
```
Add rationale paragraph: "The framed domain separator eliminates prefix/suffix collision risk; verifiers MUST reject signatures computed over a 4-field framing."

- [ ] **Step 1.2: amend §10.2 TOML example**

Find example `host_imports."host.broadcast-submit"` and rename to `host.broadcast`. Add note: "the capability key is `host.broadcast` (per architecture.md §3.5); the WIT wire name `broadcast-submit` is the kernel-side import binding."

- [ ] **Step 1.3: confirm `architecture.md §3.5` table — `host.broadcast` capability key**

Read §3.5 capability table; if any row uses `broadcast-submit` as the capability key, fix to `host.broadcast` and leave a footnote on WIT wire name.

- [ ] **Step 1.4: amend `determinism.md §5.1` `verify-payload-mac` signature**

Change to:
```
host.verify-payload-mac(envelope: list<u8>, key: borrow<key-handle>) -> bool
host.install-key(handle: key-handle, sealed: list<u8>) -> ()
```
Add: "v1 defers both functions to plan B; manifests for state-apply that declare them are rejected at install with `InstallError::DeferredToPlanB`."

- [ ] **Step 1.5: amend `determinism.md §5.3` (or `distribution.md §10.5`) — kernel_major v1 install gate**

Add: "Install MUST reject any manifest where `kernel_major != 1`. `kernel_major` is the kernel ABI major version; cross-major peers do not interop."

- [ ] **Step 1.6: commit**

```bash
git add docs/specs/2026-05-09-myrhiza-master-design/
git commit -m "docs(specs): amend §10.2 signing target framing + §5.1 mac borrow + §10.5 kernel_major v1 gate"
```

---

## Task 2: Event `prev` encoding — `Option<EventHash>` → `EventHash` with `EventHash::ZERO`

**Files:**
- Modify: `crates/types/src/hash.rs` (add `EventHash::ZERO` const)
- Modify: `crates/types/src/event.rs` (change `prev` field type, update `SignedBody`, hash_signed_body, wire_hash, all tests)

- [ ] **Step 2.1: write failing test**

In `crates/types/src/event.rs` test mod, add:
```rust
#[test]
fn genesis_event_prev_is_zero_sentinel() {
    let event = make_test_event(/*seq=*/ 1, /*prev=*/ EventHash::ZERO);
    let bytes = canonical_bincode().serialize(&event).unwrap();
    // genesis prev is 32 raw zero bytes, no Option discriminant
    let prev_offset = /* compute via field layout */ ;
    assert_eq!(&bytes[prev_offset..prev_offset + 32], &[0u8; 32]);
}
```

- [ ] **Step 2.2: run test, expect FAIL**

`cargo test -p myrhiza-types genesis_event_prev_is_zero_sentinel` → FAIL (Option discriminant present).

- [ ] **Step 2.3: implement**

In `hash.rs`:
```rust
impl EventHash {
    pub const ZERO: EventHash = EventHash([0u8; 32]);
}
```
In `event.rs`: `pub prev: EventHash` (drop `Option`), update `SignedBody`, all constructors, all tests. `hash_signed_body` and `wire_hash` continue calling `canonical_bincode().serialize(&body)`.

- [ ] **Step 2.4: run all types tests, expect PASS**

`cargo test -p myrhiza-types` — every test passes including the new one and the full envelope round-trip.

- [ ] **Step 2.5: commit**

```bash
git add crates/types/
git commit -m "fix(types): event prev is EventHash with ZERO sentinel for genesis (not Option)"
```

---

## Task 3: Wasmtime `Config` pin nondeterministic features off

**Files:**
- Modify: `crates/wasmtime-backend/src/engine.rs` (`WasmtimeBackend::new`)
- Modify: `crates/wasmtime-backend/tests/engine_config.rs` (new test file)

- [ ] **Step 3.1: write failing test**

`crates/wasmtime-backend/tests/engine_config.rs`:
```rust
#[test]
fn engine_config_pins_deterministic_features() {
    // Component using SIMD must fail to compile in our engine.
    let backend = WasmtimeBackend::new().unwrap();
    let simd_wat = r#"(component (core module (func (export "f") v128.const i32x4 0 0 0 0 drop)))"#;
    let bytes = wat::parse_str(simd_wat).unwrap();
    let result = backend.compile_for_test(&bytes);
    assert!(result.is_err(), "engine accepted SIMD component despite deterministic config");
}
```

- [ ] **Step 3.2: run, expect FAIL**

`cargo test -p myrhiza-wasmtime-backend engine_config_pins_deterministic_features` → FAIL or PASS-incorrectly. Either way confirm pre-fix state.

- [ ] **Step 3.3: implement Config pin**

In `WasmtimeBackend::new`:
```rust
let mut config = Config::new();
config
    .wasm_simd(false)
    .wasm_relaxed_simd(false)
    .wasm_threads(false)
    .wasm_memory64(false)
    .wasm_multi_memory(false)
    .wasm_reference_types(true)   // CM requires
    .wasm_bulk_memory(true)       // CM requires
    .wasm_multi_value(true)       // CM requires
    .cranelift_nan_canonicalization(true)
    .consume_fuel(true);
let engine = Engine::new(&config)?;
```
Add a `pub fn compile_for_test(&self, bytes: &[u8]) -> Result<Component, _>` (cfg-gated) so the test can probe.

- [ ] **Step 3.4: run, expect PASS**

`cargo test -p myrhiza-wasmtime-backend engine_config_pins_deterministic_features` → PASS.

- [ ] **Step 3.5: commit**

```bash
git add crates/wasmtime-backend/
git commit -m "fix(wasmtime-backend): pin Config — disable SIMD, threads, memory64, multi-memory; canonicalize NaN"
```

---

## Task 4: Float-ban — recurse into nested components + exhaustive SIMD-float opcodes

**Files:**
- Modify: `crates/wasmtime-backend/src/float_ban.rs`
- Modify: `tests/fixtures/float-banned/` (verify still triggers; possibly add a nested-component fixture)

- [ ] **Step 4.1: write failing test for nested component**

In `crates/wasmtime-backend/src/float_ban.rs` test mod:
```rust
#[test]
fn detects_float_in_nested_component() {
    let wat = r#"
    (component
      (component
        (core module
          (func (export "f") (result f32) f32.const 1.0))))
    "#;
    let bytes = wat::parse_str(wat).unwrap();
    let result = scan_component_for_floats(&bytes);
    assert!(result.is_err(), "nested-component float escaped scanner");
}
```

- [ ] **Step 4.2: run, expect FAIL** (scanner walks only top-level)

- [ ] **Step 4.3: implement recursion**

Refactor `scan_component_for_floats` to accept an offset stack. On `Payload::ComponentSection { unchecked_range, .. }`, push the inner range and continue scanning. Handle `Payload::ModuleSection { unchecked_range, .. }` by passing inner bytes to `scan_core_module_for_floats`. Use `Parser::new(0).parse_all(bytes)` per-frame, not a global walker.

- [ ] **Step 4.4: SIMD opcode exhaustive enumeration**

Replace `is_float_op` with `is_allowed_int_op` (whitelist) inverted: any operator NOT in the int/control/memory whitelist that touches f32/f64/v128 is rejected. Reference Wasmtime's `Operator` enum exhaustively. Specifically include the 40+ missing ops: `F32x4{Min,Max,Sqrt,Abs,Neg,Pmin,Pmax,Eq,Ne,Lt,Gt,Le,Ge,Ceil,Floor,Trunc,Nearest,ConvertI32x4S,ConvertI32x4U,DemoteF64x2Zero}`, `F64x2{Min,Max,Sqrt,Abs,Neg,Pmin,Pmax,Eq,Ne,Lt,Gt,Le,Ge,Ceil,Floor,Trunc,Nearest,PromoteLowF32x4}`, `I32x4TruncSatF32x4{S,U}`, `I32x4TruncSatF64x2{S,U}Zero`, all `RelaxedF32x4*`/`RelaxedF64x2*`. (Note: with Task 3's `wasm_simd(false)` pin, SIMD ops won't reach the lint, but the lint must still defend in depth — Wasmtime defaults move across LTS bumps.)

- [ ] **Step 4.5: run all float-ban tests, expect PASS**

`cargo test -p myrhiza-wasmtime-backend float_ban` → all pass.

- [ ] **Step 4.6: commit**

```bash
git add crates/wasmtime-backend/src/float_ban.rs
git commit -m "fix(wasmtime-backend): float-ban recurses into nested components + exhaustive SIMD-float opcodes"
```

---

## Task 5: Manifest signing target — confirm code matches amended spec (no code change expected)

**Files:**
- Modify: `crates/manifest/src/canonical.rs` (header doc)
- Modify: `crates/manifest/tests/` (strengthen `signing_target_layout` test)

Per spec amendment in Task 1.1, code at `canonical.rs:96-103` is now correct (5 length-prefixed fields). Strengthen tests + fix doc.

- [ ] **Step 5.1: replace doc comment at top of canonical.rs**

Drop the `BLAKE3("myrhiza/manifest/v1")` claim. Replace with concrete 5-field framing matching spec §10.2.

- [ ] **Step 5.2: replace weak `signing_target_layout` test**

Old test asserts `>= 16` bytes. New test:
```rust
#[test]
fn signing_target_is_five_length_prefixed_fields() {
    let bytes = signing_target_bytes(/* fixture */);
    let mut cursor = 0;
    for (idx, expected_len) in [(0, DOMAIN_SEP.len()),
                                 (1, 32),  // canonical_hash
                                 (2, 32),  // content_hash
                                 (3, /* version str */),
                                 (4, 32)]  // author_pubkey
    {
        let len = u32::from_le_bytes(bytes[cursor..cursor+4].try_into().unwrap()) as usize;
        assert_eq!(len, expected_len, "field {} length", idx);
        cursor += 4 + len;
    }
    assert_eq!(cursor, bytes.len(), "trailing garbage in signing target");
}
```

- [ ] **Step 5.3: run manifest tests, expect PASS**

- [ ] **Step 5.4: commit**

```bash
git add crates/manifest/
git commit -m "test(manifest): assert signing target is 5 length-prefixed fields per spec §10.2"
```

---

## Task 6: Capability vocabulary — `host.broadcast` rename + defer install-key/verify-payload-mac

**Files:**
- Modify: `crates/manifest/src/vocabulary.rs`
- Modify: `crates/wasmtime-backend/src/gating.rs`
- Modify: `crates/manifest/tests/fixtures/counter-manifest.toml`
- Modify: `crates/backend/src/lib.rs` (add `BackendError::DeferredToPlanB`)

- [ ] **Step 6.1: vocabulary rename**

In `vocabulary.rs`: rename entry `"host.broadcast-submit"` → `"host.broadcast"`. Keep classification as `HostImport` (non-deterministic). Update tests.

- [ ] **Step 6.2: defer install-key + verify-payload-mac**

In `vocabulary.rs`: keep both registered as `DeterministicHelper` (still authored capabilities), but add a deferred-marker field. In `wasmtime-backend/src/gating.rs::state_apply_ambient_set`, drop both. In `validate_state_apply_manifest`, when manifest declares either, return new error `BackendError::DeferredToPlanB(name)`. In `state_apply_bound_imports`, drop both.

- [ ] **Step 6.3: add `BackendError::DeferredToPlanB`**

In `crates/backend/src/lib.rs`:
```rust
#[error("capability {0:?} declared but deferred to plan B")]
DeferredToPlanB(String),
```

- [ ] **Step 6.4: rename fixture**

`crates/manifest/tests/fixtures/counter-manifest.toml`: `host.broadcast-submit` → `host.broadcast`.

- [ ] **Step 6.5: tests**

Add `gating::tests::install_key_in_manifest_returns_deferred_to_plan_b` and `verify_payload_mac_in_manifest_returns_deferred_to_plan_b`. Run full `cargo test`.

- [ ] **Step 6.6: commit**

```bash
git add crates/manifest/ crates/wasmtime-backend/ crates/backend/
git commit -m "fix(manifest,backend): rename host.broadcast capability + defer install-key/verify-payload-mac to plan B"
```

---

## Task 7: Verdict categorization — typed downcast instead of string match

**Files:**
- Modify: `crates/wasmtime-backend/src/instance.rs`
- Modify: `crates/wasmtime-backend/src/engine.rs` (pre-walk imports for `UnauthorizedImport`)
- Modify: `crates/kernel/tests/acceptance.rs` (assert `BackendError` variant, not error substring)

- [ ] **Step 7.1: pre-walk component imports against bound set in `instantiate_state_apply`**

Before `linker.instantiate_async` (or sync), enumerate `Component::imports()` (or `component_type().imports()`) and reject any import not present in `bound_imports`, returning `BackendError::UnauthorizedImport(name.into())`.

- [ ] **Step 7.2: typed trap downcast in `instance.rs`**

Replace string-match block with:
```rust
fn map_wasmtime_error(e: wasmtime::Error) -> BackendError {
    if let Some(trap) = e.downcast_ref::<wasmtime::Trap>() {
        return match trap {
            wasmtime::Trap::OutOfFuel => BackendError::FuelExhausted,
            wasmtime::Trap::MemoryOutOfBounds => BackendError::MemoryExhausted,
            other => BackendError::Trap(other.to_string()),
        };
    }
    BackendError::Instantiation(e.to_string())
}
```
Add `BackendError::FuelExhausted` and `BackendError::MemoryExhausted` variants if not present.

- [ ] **Step 7.3: acceptance tests assert variants**

`crates/kernel/tests/acceptance.rs`:
- `infinite-loop` test: `assert!(matches!(err, BackendError::FuelExhausted))`
- `over-importer` test: `assert!(matches!(err, BackendError::UnauthorizedImport(name) if name.contains("host.")))`
- `float-banned` test: `assert!(matches!(err, BackendError::BannedInstruction(_)))`

- [ ] **Step 7.4: run full ci**

`just ci` → all pass.

- [ ] **Step 7.5: commit**

```bash
git add crates/wasmtime-backend/ crates/kernel/ crates/backend/
git commit -m "fix(wasmtime-backend,kernel): typed verdict categorization — Trap downcast + import pre-walk"
```

---

## Task 8: `host.now-hlc-from-event` — strict canonical decode

**Files:**
- Modify: `crates/wasmtime-backend/src/helpers.rs`
- Modify: `crates/types/src/encoding.rs` (add `decode_canonical` helper)

- [ ] **Step 8.1: add `decode_canonical` helper to `types::encoding`**

```rust
pub fn decode_canonical<T>(bytes: &[u8]) -> Result<T, EncodingError>
where T: Serialize + DeserializeOwned + PartialEq
{
    let value: T = canonical_bincode().deserialize(bytes)?;
    let re_encoded = canonical_bincode().serialize(&value)?;
    if re_encoded != bytes {
        return Err(EncodingError::NonCanonical);
    }
    Ok(value)
}
```

- [ ] **Step 8.2: write failing test**

```rust
#[test]
fn host_now_hlc_rejects_non_canonical_event_bytes() {
    let event = make_test_event();
    let mut bytes = canonical_bincode().serialize(&event).unwrap();
    bytes.push(0); // trailing garbage
    let result = host_now_hlc_from_event_impl(&bytes);
    assert!(result.is_err());
}
```

- [ ] **Step 8.3: replace `.deserialize(...).ok()` with `decode_canonical(...)`**

In `helpers.rs::host_now_hlc_from_event_impl`. Drop the `.ok()` swallow.

- [ ] **Step 8.4: run, expect PASS**

- [ ] **Step 8.5: commit**

```bash
git add crates/wasmtime-backend/ crates/types/
git commit -m "fix(wasmtime-backend): host.now-hlc-from-event rejects non-canonical event bytes"
```

---

## Task 9: WIT freeze — 4 worlds + named-render not Debug `idx`

**Files:**
- Modify: `crates/wasmtime-backend/tests/wit_freeze.rs`
- Add: `tests/snapshots/state-propose-world.bindgen.txt`
- Add: `tests/snapshots/interaction-world.bindgen.txt`
- Add: `tests/snapshots/behavior-world.bindgen.txt`
- Modify: `tests/snapshots/state-apply-world.bindgen.txt` (regenerate without Debug `idx`)
- Modify: `crates/wasmtime-backend/src/world-state-apply.wit` if Task 1.4 changed types

- [ ] **Step 9.1: refactor render to name-resolved**

In `wit_freeze.rs`, replace the `{:?}` debug-print of `Id { idx: N }` with a recursive name resolver that walks `Resolve::types[id]` / `Resolve::interfaces[id]` and emits `interface@version::name(...) -> result-type-name`. Functions render as `name(param: type, ...) -> result-type` using fully-qualified resolved type names.

- [ ] **Step 9.2: iterate all 4 worlds**

Loop `["state-apply", "state-propose", "interaction", "behavior"]`. Each writes to `tests/snapshots/<world>-world.bindgen.txt`. Snapshot file format unchanged: text canonical dump.

- [ ] **Step 9.3: regenerate state-apply snapshot, write 3 new snapshots**

Run with `MYRHIZA_SNAPSHOT_UPDATE=1` (or equivalent env), commit the 4 snapshots.

- [ ] **Step 9.4: run, expect PASS for all 4**

- [ ] **Step 9.5: commit**

```bash
git add crates/wasmtime-backend/tests/wit_freeze.rs tests/snapshots/
git commit -m "test(verification): WIT freeze covers all 4 worlds + name-resolved render (no Debug idx)"
```

---

## Task 10: Acceptance — §15.1 #5 manifest-arm e2e test

**Files:**
- Modify: `crates/kernel/tests/acceptance.rs`

- [ ] **Step 10.1: add manifest-over-declares test**

```rust
/// Covers: capabilities.md §7.2 (manifest-arm of acceptance criterion #5);
///         distribution.md §10.5 (install reject for non-deterministic cap on state-apply)
#[test]
fn manifest_declaring_non_deterministic_cap_rejects_at_install() {
    let bundle = test_utils::build_counter_bundle_with_extra_cap("host.broadcast");
    let result = InstallFlow::load(&bundle);
    assert!(matches!(
        result.unwrap_err(),
        InstallError::Backend(BackendError::UnauthorizedImport(_)) |
        InstallError::Backend(BackendError::DeferredToPlanB(_))
    ));
}
```

- [ ] **Step 10.2: extend `test-utils` with `build_counter_bundle_with_extra_cap`**

Helper: build a bundle from the counter fixture + signed manifest with one extra `host.*` capability declared.

- [ ] **Step 10.3: run, expect PASS**

- [ ] **Step 10.4: commit**

```bash
git add crates/kernel/tests/acceptance.rs crates/test-utils/
git commit -m "test(kernel): acceptance — §15.1 #5 manifest-arm rejects non-deterministic cap on state-apply"
```

---

## Task 11: Cleanup — minors batched

**Files:** various

Each step independent; can commit together.

- [ ] **Step 11.1: kernel_major v1 install gate** (`crates/kernel/src/install.rs`): reject `manifest.kernel_major != 1` with `InstallError::IncompatibleKernelMajor(v)`. Add unit test.

- [ ] **Step 11.2: parse high-value-ops strict bool** (`crates/manifest/src/parse.rs:225-232`): replace `.as_bool().unwrap_or(false)` with `.as_bool().ok_or_else(|| ParseError::InvalidValue { ... })?`. Test: TOML with `clipboard.write = "yes"` → ParseError, not silent false.

- [ ] **Step 11.3: drop Box::leak in static_str + banned_instruction** (`parse.rs::static_str`, `engine.rs:153-156`): change `BackendError::BannedInstruction` to carry `String`; type-tighten `require(table: &'static str, ...)`; delete `static_str`.

- [ ] **Step 11.4: pre-check apply agreement test** (`crates/kernel/tests/acceptance.rs`): after the existing `pre_check` Reject assertion against `pre-check-rejector`, also assert `handle.apply(...)` returns same `Verdict::Reject`. Documents §22.5 invariant.

- [ ] **Step 11.5: rename `verify_rejects_non_strict_signature`** (`crates/manifest/src/signature.rs:120`): rename to `verify_strict_accepts_canonical_signature` since body asserts canonical accept. Note in body: "malleable s-value adversarial vector lands plan B".

- [ ] **Step 11.6: fix `canonical.rs` doc rot** (`crates/manifest/src/canonical.rs:6-7`): drop the `BLAKE3("myrhiza/manifest/v1")` doc claim. Use raw-string framing per amended §10.2.

- [ ] **Step 11.7: `LogSink::drain` recover from poison** (`crates/wasmtime-backend/src/helpers.rs:88`): `lock().unwrap_or_else(|e| e.into_inner())` instead of dropping log entries on poison.

- [ ] **Step 11.8: `bincode::Error` not in pub API** (`crates/kernel/src/install.rs:52`): replace `Bincode(#[from] bincode::Error)` with `Decode(String)`; convert via `.map_err(|e| InstallError::Decode(e.to_string()))?`.

- [ ] **Step 11.9: `host.log` debug_assert** (`crates/wasmtime-backend/src/gating.rs:160`): add `debug_assert!(bound_imports.contains("host.log"))` and unit test "manifest omitting host.log still binds it" to lock the always-on behavior.

- [ ] **Step 11.10: spec-coverage script validates refs** (`scripts/spec-coverage.sh`): build set of valid `<file>.md §X.Y` tuples by grepping `^### \d+\.\d+` from `docs/specs/2026-05-09-myrhiza-master-design/*.md`; exit non-zero on any `/// Covers:` ref not in the set.

- [ ] **Step 11.11: spec-coverage matrix add missing entries**: annotate tests covering `determinism.md §5.1`, `§5.4`, `distribution.md §10.5`, `capabilities.md §7.1` with `/// Covers:` doc comments.

- [ ] **Step 11.12: CI runs `just build-fixtures`**: in `.github/workflows/ci.yml`, add `wasm32-unknown-unknown` toolchain target and `wasm-tools` install steps; run `just build-fixtures` before `cargo test`. Alternative: `#[ignore]`-gate the fixture-dependent tests behind `MYRHIZA_FIXTURES_BUILT=1`.

- [ ] **Step 11.13: run full ci**

`just ci` → green.

- [ ] **Step 11.14: commit**

```bash
git add -A
git commit -m "chore: address review minors — kernel_major gate, log poison, bincode err, spec-coverage validator, CI fixtures"
```

---

## Task 12: Final spec-compliance + ci sanity sweep

- [ ] **Step 12.1: run `just ci`** — must be green end-to-end.

- [ ] **Step 12.2: verify `git log --oneline main..HEAD` shape** — readable, conventional-commit prefixes, one concern per commit.

- [ ] **Step 12.3: update PR #1 body** — append "## Review fixes" block summarizing 22 findings addressed; cite this plan.

- [ ] **Step 12.4: handoff doc update** — strike "host-call fuel costs declared but not deducted" (still gap), add note that all critical findings from review swarm are addressed.

---

## Self-review checklist

- [x] Spec coverage: each of 22 findings from review synthesis has a task or task-step that addresses it.
- [x] Decisions made up front: spec-vs-code conflicts resolved in plan header (no subagent re-litigation).
- [x] No placeholders: every step has concrete file path, code snippet, command, or assertion.
- [x] Type consistency: `BackendError::DeferredToPlanB`, `FuelExhausted`, `MemoryExhausted`, `InstallError::IncompatibleKernelMajor`, `InstallError::Decode`, `EncodingError::NonCanonical` all newly named — must be added in their introducing tasks (6, 7, 11.1, 11.8, 8.1).

---

## Mapping: 22 findings → tasks

| Finding | Task | Severity |
|---|---|---|
| Crit 1 (event prev) | Task 2 | C |
| Crit 2 (signing target framing) | Task 1.1 + Task 5 | C |
| Crit 3 (Wasmtime Config) | Task 3 | C |
| Crit 4 (float-ban nested + SIMD) | Task 4 | C |
| Crit 5 (acceptance §15.1 #5 manifest arm) | Task 10 | C |
| Crit 6 (WIT freeze 4 worlds + named) | Task 9 | C |
| Imp 7 (verdict categorization) | Task 7 | I |
| Imp 8 (now-hlc canonical decode) | Task 8 | I |
| Imp 9 (host.broadcast vocab rename) | Task 1.2 + Task 6.1 + Task 6.4 | I |
| Imp 10 (defer install-key/verify-payload-mac) | Task 1.4 + Task 6.2 + Task 6.3 | I |
| Imp 11 (borrow<key-handle> WIT) | Task 1.4 | I |
| Imp 12 (HVO strict bool parse) | Task 11.2 | I |
| Imp 13 (CI build-fixtures) | Task 11.12 | I |
| Imp 14 (spec-coverage missing entries) | Task 11.10 + Task 11.11 | I |
| Min 15 (Box::leak) | Task 11.3 | M |
| Min 16 (bincode err in pub API) | Task 11.8 | M |
| Min 17 (pre-check apply agree) | Task 11.4 | M |
| Min 18 (verify_rejects_non_strict name) | Task 11.5 | M |
| Min 19 (canonical.rs doc rot) | Task 11.6 + Task 5.1 | M |
| Min 20 (repro-fixtures workflow) | (deferred to plan B; not in this plan) | M |
| Min 21 (LogSink poison) | Task 11.7 | M |
| Min 22 (kernel_major gate) | Task 11.1 | M |

Min 20 deferred per scope discipline — `repro-fixtures.yml` nightly drift check is plan B's CI matrix work; flagged in handoff doc instead.
