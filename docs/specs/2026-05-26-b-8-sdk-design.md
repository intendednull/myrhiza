**Date:** 2026-05-26
**Status:** draft
**Parent:** [implementation.md §20](2026-05-09-myrhiza-master-design/implementation.md)
**Subject:** Plan B-8 — SDK ergonomics + examples wiring + dependency-direction CI

# Plan B-8 — SDK ergonomics + examples wiring + dep-direction CI

## 1. Goal

Close [implementation.md §20](2026-05-09-myrhiza-master-design/implementation.md) items **20** (SDK ergonomics: macros + tooling) and **24** (dependency-direction CI check), and introduce the [`mvp.md §15.4`](2026-05-09-myrhiza-master-design/mvp.md) workspace shape under `examples/` so [B-6](../reports/2026-05-21-mvp-gap-analysis.md#b-6-poll-app-deferred-slice-not-v1-blocking) (poll app) has somewhere to land.

After B-8 lands:

- App authors have a single dependency (`myrhiza-sdk`) that bundles re-exported kernel types, the WIT directory consumed by `wit_bindgen::generate!`, a `manifest!` declarative macro, and a documented "this is how you build a Myrhiza app" boilerplate.
- The canonical counter demo lives at `examples/counter/` as a workspace member depending on `myrhiza-sdk`, replacing the three `tests/fixtures/counter-*` boilerplate directories.
- CI fails closed if `examples/` ever takes a transitive dependency on a kernel-side crate (kernel, backend, wasmtime-backend, network, manifest-internal, types-internal), enforcing the [`mvp.md §15.4`](2026-05-09-myrhiza-master-design/mvp.md) load-bearing direction `examples/ → crates/sdk` only.

B-8 is foundational, not feature-bearing: every v1 acceptance criterion is already satisfied per the [post-B-7 gap analysis](../reports/2026-05-21-mvp-gap-analysis.md). The MVP gains an authoring story, not a new runtime capability.

## 2. Design choices

### 2.1 SDK scope: types + WIT + `manifest!` only (no proc-macros at v1)

The counter fixtures' real friction is **not** wit-bindgen — that's already one macro invocation per crate. The friction is:

1. **Per-fixture boilerplate**: a bump allocator (`HEAP_SIZE = 64 * 1024` plus `BumpAlloc` impl), a `#[panic_handler]`, `#![no_std]`/`#![no_main]`, the `unsafe_op_in_unsafe_fn` allow for wit-bindgen 0.30, the `panic = "abort"` + `opt-level = "z"` + `lto = true` release profile. **Duplicated across every counter-* fixture verbatim.** ~80 LOC each that has nothing to do with the app.
2. **WIT file copies**: each fixture has its own `wit/world.wit` re-declaring a structurally-compatible subset of the kernel ABI. Drift between the fixture WIT and the canonical WIT in `wit/myrhiza-kernel/wit/` is a documented hazard (see [`counter-state-apply/wit/world.wit:1-12`](../../tests/fixtures/counter-state-apply/wit/world.wit)).
3. **Manifest hand-rolling**: today the manifest TOML is hand-written and signed by an external step (`crates/test-utils/src/manifest.rs::sign_manifest`). New app authors have no documented path from "I wrote a state-apply impl" to "I have a signed bundle the kernel will load."

**Choice (a) v1 scope — type re-exports + WIT directory + `manifest!` declarative macro.** The SDK ships:

- `myrhiza_sdk::types` — re-exports of `Verdict`, `Hlc`, `LogLevel`, `IdentityScope`, plus the manifest-side structs (`Manifest`, `AbiSection`, `CapabilitiesSection`, etc.) so app authors don't depend on `myrhiza-manifest` directly.
- `myrhiza_sdk::prelude` — `use myrhiza_sdk::prelude::*;` brings the common SDK names into scope (`Verdict`, `LogLevel`, the `manifest!` macro, the `myrhiza_app!` runtime-init macro).
- A `wit/` subdirectory shipped with the SDK crate, containing the canonical kernel WIT package at the version this SDK release targets. Apps point `wit_bindgen::generate!` at `myrhiza_sdk::local_wit_dir!()` (a `macro_rules!` returning the absolute path constant at compile time — note: the macro lives in the SDK but emits the **consumer**'s `wit/` directory path, hence `local_*`) instead of vendoring their own copy.
- A `manifest!` declarative macro (`macro_rules!`) that takes a structured Rust DSL and emits a `Manifest` struct literal + a function returning the canonical-bincode bytes ready for signing. Used at build time by the example crates and by app authors at publish time.
- A `myrhiza_app!` runtime-init macro: takes the world name and the `Component` type; expands to `#![no_std]`, `#![no_main]`, the bump allocator, the `#[panic_handler]`, the `wit_bindgen::generate!` invocation, the `export!(Component);` line, and the `#![allow(unsafe_op_in_unsafe_fn)]` workaround for wit-bindgen 0.30. Single macro per fixture replaces ~80 LOC of boilerplate.

**Runner-up — choice (b) full proc-macros (`state_apply!`, `state_propose!`, `interaction!`) wrapping wit-bindgen + manifest emission.** Rejected for v1 because:

- Proc-macros require a separate `crates/sdk-macros/` crate, doubling the v1 surface area for marginal authoring benefit.
- The wit-bindgen macro already exists and is well-understood; wrapping it adds a debugging layer between the app author's source and the generated `impl Guest for Component`. Diagnostic-quality regressions are real (see [`prior-art/wasm-component-model/lessons.md`](../prior-art/wasm-component-model/lessons.md) Avoid row 8 — per-language wit-bindgen ergonomics still rough).
- We do not yet know the right macro shape — the fixtures shipped over B-1 / B-5 / B-7 have used three different patterns. Locking in a proc-macro surface before the patterns stabilize is premature.
- B-8 is budgeted at 2-3 days per the gap analysis; a clean proc-macro crate with diagnostics is a multi-week effort.

A `state_apply!` / `state_propose!` / `interaction!` proc-macro layer remains on the roadmap; it can ship in a later B-8.1-style follow-up once the example apps stabilize. **Choice (a)'s `myrhiza_app!` declarative macro is the bridge** — app authors invoke a single macro that handles the boilerplate; the macro is structurally pure macro_rules, expanded inline, and contains no diagnostics-sensitive logic. If a later proc-macro layer ships, it can re-use the same arg shape.

**Runner-up — choice (c) full HDK-style cookbook (lifecycle callbacks, capability helpers, error types, prelude).** Rejected for v1 because we don't yet have enough app diversity to know what helpers are universal. Holochain's HDK accumulated significant per-app coupling that broke every minor release ([`prior-art/holochain/lessons.md`](../prior-art/holochain/lessons.md) Avoid row 1 — "custom WASM ABI that can't survive a host upgrade"). The SDK is a stable surface; we add helpers only when we see a pattern in 2+ apps.

### 2.2 SDK crate structure: single `crates/sdk/` (no macro-crate split needed)

Because choice 2.1 (a) defers proc-macros to a later slice, the SDK is **one crate**:

```
crates/sdk/
├── Cargo.toml
├── src/
│   ├── lib.rs                — re-exports, module declarations
│   ├── prelude.rs            — `pub use` of common surface
│   ├── types.rs              — re-exports of kernel-facing types
│   ├── manifest.rs           — re-exports of Manifest schema + the manifest! macro
│   ├── boilerplate.rs        — the bump allocator + panic handler the myrhiza_app! macro emits
│   └── macros.rs             — manifest!, myrhiza_app!, local_wit_dir! declarative macros
└── wit/                      — copy of wit/myrhiza-kernel/wit/*.wit pinned to the SDK release
```

Proc-macros would require a separate `crates/sdk-macros/` because the proc-macro-2 / syn / quote dependencies cannot be wasm32 targets and procedural-macro crates must declare `proc-macro = true` in their lib block. Since v1 ships only `macro_rules!` macros (which expand inline and have no separate compile target), the split is unnecessary. If/when we add proc-macros in a follow-up slice, the split happens then.

**Trade-off**: `macro_rules!` cannot do everything a proc-macro can — in particular, the `manifest!` macro can't validate capability names against the kernel's vocabulary at compile time (the macro doesn't have access to the `myrhiza_manifest::vocabulary` registry; it would have to be a string-match list embedded in the macro itself). The runtime path (manifest parse + validation) catches mistakes; the macro just makes them harder to make. A future proc-macro could surface validation errors at the source location instead of at install. **Accepted trade-off for v1.**

### 2.3 Examples migration: hybrid — migrate counter, leave echo + negative-test fixtures

Today's `tests/fixtures/` layout has three categories:

| Category | Members | Role |
|---|---|---|
| App demos | `counter-state-apply`, `counter-state-propose`, `counter-interaction` | Canonical first-app demonstration; cited by acceptance tests as "the counter app." |
| Coexistence fixtures | `echo-state-apply` | Deliberately-minimal second-app to prove [criterion 4](2026-05-09-myrhiza-master-design/mvp.md) (two-app coexistence on one peer). Not a demo app; not authored against the SDK. |
| Negative-test fixtures | `over-importer`, `pre-check-rejector`, `infinite-loop`, `float-banned` | Manifest-arm tests: components that should fail to instantiate. Each constructs an invalid component shape on purpose. |

**Choice (c) hybrid migration:**

- **Migrate**: `tests/fixtures/counter-state-apply/`, `tests/fixtures/counter-state-propose/`, `tests/fixtures/counter-interaction/` → consolidate as `examples/counter/`, a single crate with three component slots:
  ```
  examples/counter/
  ├── Cargo.toml             # workspace member, lib + 3 [[bin]] entries
  ├── manifest.rs            # uses manifest! macro, emits canonical bincode
  ├── wit/                   # copied from crates/sdk/wit/ via `just sync-wit` (§2.5)
  └── src/
      ├── state.rs           # myrhiza_app!(state_apply, Component); + impl Guest
      ├── propose.rs         # myrhiza_app!(state_propose, Component); + impl Guest
      ├── interaction.rs     # myrhiza_app!(interaction, Component); + impl Guest
      └── lib.rs             # re-exports (so cargo test --doc can build)
  ```
  Each component is its own cdylib `[[bin]]` (Cargo limitation: one crate, one wasm artifact — so `examples/counter/` is technically three artifact-producing builds wired through one Cargo.toml with `[[example]]` or separate `[[bin]]` targets per profile). The `just build-fixtures` recipe is updated to drive the new path.

- **Leave**: `tests/fixtures/echo-state-apply/`. Echo's intent is "a second WASM blob that proves two-app coexistence works in the kernel"; it does **not** need to look like a real app. Migrating it to `examples/echo/` would dilute the message of `examples/` ("look at how an app is authored").

- **Leave**: `tests/fixtures/over-importer/`, `tests/fixtures/pre-check-rejector/`, `tests/fixtures/infinite-loop/`, `tests/fixtures/float-banned/`. These are **not** apps — they are kernel-test inputs that construct deliberately-broken component shapes (e.g., over-importer declares a host import that's not in any manifest; float-banned emits a banned f64 instruction). They have no SDK story and shouldn't try to.

**Why not migrate everything**: bright-line separation between "examples/ = how to write an app" and "tests/fixtures/ = kernel test inputs." Migrating echo + the negative fixtures into examples/ would make examples/ a mixed bag that newcomers can't read as authoritative app patterns.

**Why not leave counter alone**: the counter fixture is the canonical first demo that every new contributor reads. Today they read three separate ~120-LOC crates each containing ~80 LOC of identical boilerplate plus a duplicated WIT subset. That experience is the #1 deterrent to "I could write a Myrhiza app." `examples/counter/` with `myrhiza_app!` macros + a single `manifest!` invocation is the right pedagogical surface.

**Impact estimate**: 5 file deletions (3 `tests/fixtures/counter-*` directories + 3 `Cargo.lock` files) + ~6 file creations (`examples/counter/{Cargo.toml, manifest.rs, src/state.rs, src/propose.rs, src/interaction.rs, src/lib.rs}`). Changes to:

- Root `Cargo.toml` workspace `members` + `exclude` arrays (add `examples/counter`, drop the three counter-* fixture excludes).
- `Justfile`'s `build-fixtures` recipe (change `(_build-fixture "counter-state-apply" "counter_state_apply_fixture" "state-apply")` to point at the new path; reuse for state-propose and interaction).
- `crates/test-utils/src/bundle.rs` paths: `counter_fixture_path()`, `counter_state_propose_fixture_path()`, `counter_interaction_fixture_path()` (3 helpers ~lines 85–208). Per §3.5 step 6 we keep the **output** location at `tests/fixtures/built/counter-*.wasm`, so these helpers' returned paths **do not change**.

Since the output fixture paths don't move, every other consumer of `build_signed_counter_bundle` / `build_signed_counter_bundle_three_components` and the path strings continues to work without code changes. However, several files contain **B-7-era narrative copy** in doc-comments that becomes stale once the source-of-truth moves from `tests/fixtures/counter-*/` to `examples/counter/`. Touch-list:

| File | What changes |
|---|---|
| `crates/test-utils/src/bundle.rs` (3 path helpers, lines ~85–208) | Doc-comments referring to "fixture wasm at `tests/fixtures/built/counter-state-apply.wasm`" stay accurate (output path is unchanged); narrative phrasing "reproducibly-built fixture at …" may want a "(produced by `just build-fixtures` from `examples/counter/src/state.rs`)" hint. |
| `crates/test-utils/src/manifest.rs` (line ~67) | Doc-comment on `helpers_only_three_component_manifest` references `build_signed_counter_bundle_three_components` — call site reference stays correct; no edit strictly required, but flagged for review during slice. |
| `crates/kernel/tests/acceptance.rs` (line ~46) | Direct path string `"tests/fixtures/built/counter-state-apply.wasm"` — unchanged (output path stable). |
| `crates/wasmtime-backend/tests/profile_instantiation.rs` (lines 5–23 module-doc, lines 135, 168, 199, 239) | 4 direct path reads — paths unchanged. Module-doc narrative ("the counter-state-propose WASM fixture (built by B-7.5)") references B-7-era build origin; this narrative is stale once the source moves but the fixture-output-name and behaviour are unchanged. |
| `crates/kernel/src/install.rs` (comment line ~241) | Comment mentions `tests/fixtures/built/counter-state-apply.wasm` as the "real component bytes" — path unchanged, no edit needed; flagged for confirmation only. |
| `crates/myrhiza-cli/tests/e2e.rs` (line ~16) | Imports `build_signed_counter_bundle_three_components` from `myrhiza_test_utils::bundle` — call-site is stable. |

The negative-test fixtures' `Cargo.toml` files stay in `tests/fixtures/` and continue to be excluded from the workspace. Their `wit/` directories continue to be local subsets because they intentionally diverge from the canonical WIT.

### 2.4 Dependency-direction CI check: cargo-metadata + small Rust script

**Rule (normative)**: A crate matched by `path matches examples/*` MAY depend on `myrhiza-sdk` (and on third-party crates not in the kernel set). It MUST NOT depend, transitively or directly, on any of:

- `myrhiza-kernel`
- `myrhiza-backend`
- `myrhiza-wasmtime-backend`
- `myrhiza-network`
- `myrhiza-test-utils`
- `myrhiza-cli`

The forbidden set is **kernel-internal crates**. The SDK is permitted; the SDK transitively depends on `myrhiza-types` and `myrhiza-manifest` (because it re-exports their types), which means example crates pick those up via SDK — but they cannot name them directly in their own `Cargo.toml`.

**Mechanism**: a small Rust binary at `xtask/dep-direction/src/main.rs` (the Cargo-community-blessed [xtask](https://github.com/matklad/cargo-xtask) pattern) calls `cargo metadata --format-version 1` and walks the package graph:

```rust
// xtask/dep-direction/src/main.rs (sketch — actual code lands in B-8.4)
use cargo_metadata::MetadataCommand;

const FORBIDDEN: &[&str] = &[
    "myrhiza-kernel",
    "myrhiza-backend",
    "myrhiza-wasmtime-backend",
    "myrhiza-network",
    "myrhiza-test-utils",
    "myrhiza-cli",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let meta = MetadataCommand::new().exec()?;
    let mut violations = Vec::new();

    for package in &meta.packages {
        let in_examples = package
            .manifest_path
            .components()
            .any(|c| c.as_str() == "examples");
        if !in_examples {
            continue;
        }
        // Walk transitive deps of this example package via resolve graph.
        let node = meta
            .resolve
            .as_ref()
            .and_then(|r| r.nodes.iter().find(|n| n.id == package.id))
            .ok_or("no resolve node")?;
        for dep_id in transitive_deps(&meta, &node.id) {
            let dep_name = meta.packages.iter()
                .find(|p| p.id == dep_id)
                .map(|p| p.name.as_str())
                .unwrap_or("");
            if FORBIDDEN.contains(&dep_name) {
                violations.push(format!(
                    "{}: transitive dep on forbidden crate {}",
                    package.name, dep_name
                ));
            }
        }
    }

    if violations.is_empty() {
        eprintln!("dep-direction OK");
        Ok(())
    } else {
        for v in &violations {
            eprintln!("VIOLATION: {v}");
        }
        std::process::exit(1)
    }
}
```

(The actual transitive-walk in B-8.4 traverses `cargo_metadata::Resolve::nodes` via BFS; this sketch is illustrative.)

**Why a bespoke script and not cargo-deny**:

- `cargo-deny` has a [`bans`](https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html) table but its `deny = [...]` list is workspace-global; it cannot express "crate X may not depend on Y, but the rest of the workspace may." We need conditional bans keyed on the dependent crate's path, which cargo-deny does not support natively.
- cargo-deny's `bans.deny` entries do support a `wrappers` field, which restricts which crates may reach a banned crate (reverse mapping — "only these wrappers may pull in banned crate X"). This is the inverse of what we need: we want "crates in `examples/*` may NOT reach kernel-internal crate Y," not "only these wrappers may reach Y workspace-wide." The graph cargo-deny operates on is direct deps + transitive bans, not arbitrary cross-cutting path-keyed rules.
- A 40-LOC Rust script in `xtask/dep-direction/` is auditable, has no third-party-config-language to learn, gives a unit-testable rule surface (synthetic `Metadata` fixtures vs. cargo-deny config-file disputes), and runs in CI as `cargo run -p dep-direction-check`.
- The script's unit tests can assert "given this synthetic Cargo.toml, the check fails / passes" — a tighter feedback loop than cargo-deny config testing.

**Why a Rust binary and not a shell + `cargo tree | grep`**:

- `cargo tree` output format is not stable; a parser is brittle. `cargo metadata --format-version 1` is stable and well-typed via the `cargo_metadata` crate.
- Shell-piped string-matching cannot distinguish transitive vs direct deps reliably.

**CI wiring**: `Justfile` gains `just dep-direction`:

```just
dep-direction:
    cargo run -p dep-direction-check --quiet

ci: fmt-check lint test test-iroh spec-coverage-check dep-direction
```

The `dep-direction-check` package is a workspace member with `publish = false` and `description = "CI check: examples/ may not depend on kernel-internal crates."`.

**Failure mode**: on violation, the script prints `VIOLATION: <example_crate>: transitive dep on forbidden crate <crate>` to stderr and exits 1. CI fails closed. Self-test: B-8.4 ships a regression test that adds `myrhiza-kernel = { path = "../../crates/kernel" }` to `examples/counter/Cargo.toml`, runs the check, asserts the script exits 1 with the expected diagnostic, then reverts. **Note**: the test needs to be a separate gated test (`#[ignore]` by default or a sibling cargo workspace) because the modified `Cargo.toml` is a worktree-poisoning change. **Recommendation**: implement the regression as a unit test against a synthetic `Metadata` struct built in-memory, not by mutating real Cargo.toml files. The cargo_metadata crate's types are `Deserialize` and can be constructed from a JSON fixture.

### 2.5 wit-bindgen integration: SDK ships WIT directory; apps invoke `wit_bindgen::generate!` themselves

The SDK does **not** re-export pre-generated bindings. wit-bindgen generates a `Guest` trait per-world that the consuming crate must `impl` — the trait must be visible in the crate that exports the component, so the macro must run in that crate. If the SDK ran the macro and re-exported `Guest`, examples would have to write `impl myrhiza_sdk::state_apply::Guest for Component`, which works mechanically but adds an indirection layer for no benefit.

**Instead**: SDK ships the `wit/` directory; the `myrhiza_app!` macro emits `wit_bindgen::generate!({ world: $world, path: <SDK_WIT_PATH> });` so the bindings are generated in the example's crate, against the same WIT bytes the kernel binds against. The kernel's WIT under `wit/myrhiza-kernel/wit/` and the SDK's `wit/` directory are **bit-identical copies** kept in sync by a B-8.0 test:

```rust
// crates/sdk/tests/wit_in_sync.rs
#[test]
fn sdk_wit_matches_kernel_wit() {
    let sdk_wit_dir = std::path::Path::new("wit");
    let kernel_wit_dir = std::path::Path::new("../../wit/myrhiza-kernel/wit");
    for entry in std::fs::read_dir(kernel_wit_dir).unwrap() {
        let entry = entry.unwrap();
        let kernel_bytes = std::fs::read(entry.path()).unwrap();
        let sdk_bytes = std::fs::read(sdk_wit_dir.join(entry.file_name())).unwrap();
        assert_eq!(
            kernel_bytes, sdk_bytes,
            "WIT drift: {} differs between kernel and SDK",
            entry.file_name().to_string_lossy()
        );
    }
}
```

**Why bit-identical**: the kernel binds against the WIT at `wit/myrhiza-kernel/wit/`. Apps bind against `wit/` shipped in the SDK. If they diverge, the canonical-ABI byte layout differs, the kernel's linker rejects the component at instantiate, and the error message is opaque. Bit-equality + a CI-tested invariant prevents this entire class of error at the source.

**Build-time mechanism for the SDK to ship its `wit/` directory**: the SDK crate's `wit/` directory is part of the crate's source tree (committed to git). `cargo package` would include it (the SDK is `publish = false` for v1, but the directory layout matches what a publishable crate would need). The path is exposed via a `local_wit_dir!()` macro that resolves to `concat!(env!("CARGO_MANIFEST_DIR"), "/../sdk/wit")` from the **example's** crate manifest dir — that's hacky.

**Alternative — a build script**: SDK ships a build.rs that copies the SDK's `wit/` directory to `OUT_DIR/wit`, and a `pub const WIT_PATH: &str = env!("MYRHIZA_SDK_WIT_DIR");` that's set by SDK's build.rs and re-exposed via... no, this also requires the example's build to know about the SDK's OUT_DIR.

**Resolution**: copy the WIT files into the example's source tree at build time via a workspace-level `xtask sync-wit` recipe, **and** assert sync via the in-sync test above. The `examples/counter/wit/` directory is committed to git (just like fixtures today), but its contents are mechanically copied from `wit/myrhiza-kernel/wit/` by `just sync-wit`. A pre-commit / CI test asserts no drift.

**Why not symlinks**: Windows + git checkout semantics. Symlinks in committed repos are a portability hazard.

This is mechanically the same pattern fixtures use today; B-8 adds the sync recipe + the regression test. **Trade-off**: there are now three copies of the kernel WIT — `wit/myrhiza-kernel/wit/` (source of truth), `crates/sdk/wit/` (SDK-distributed copy), `examples/counter/wit/` (per-app generated copy). Drift is prevented by the test, but the duplication is conceptually unfortunate. A future improvement could vendor the WIT into a registry-published crate that exposes its embedded WIT via `include_bytes!`. Out of scope for B-8.

### 2.6 Manifest authoring: declarative `manifest!` macro

App authors today have no documented path to a signed bundle. `crates/test-utils/src/manifest.rs` has helpers but they're test-only. The `manifest.toml` TOML wire format documented in [`distribution.md §10.2`](2026-05-09-myrhiza-master-design/distribution.md) is real but app authors writing TOML by hand have no schema validation until install time.

**Choice (b) declarative `manifest!` macro.** Sketch:

```rust
// examples/counter/manifest.rs
use myrhiza_sdk::prelude::*;

pub fn build() -> Manifest {
    manifest! {
        app {
            name: "counter",
            version: "0.1.0",
            description: "Shared counter MVP demo app",
            author_class: third_party,
        }
        abi {
            kernel_major: 1,
            kernel_minor_min: 0,
            state_digest_format: bincode13,
        }
        capabilities {
            deterministic_helpers: ["host.hash", "host.log"],
            // host_imports, ui_surfaces, high_value_ops: defaults to empty
        }
        components {
            state_apply: "components/state-apply.wasm",
            state_propose: "components/state-propose.wasm",
            interaction: "components/interaction.wasm",
        }
        // determinism, modules, author_policy: spec defaults
    }
}
```

The macro expands to a `Manifest` struct literal with the canonical fields populated from the DSL. Defaults: `determinism = { allow_floats: false, drift_detection: { interval_events: 1024 } }`, `modules.dep = []`, `author_policy = AuthorPolicy::Deny`. Missing required fields are compile errors (the macro pattern-matches on the DSL structure).

**On the helper-macro layer (`__author_class!`, `__sdf!`, etc.)**: the `manifest!` macro delegates ident → enum-variant translation to internal helper sub-macros (e.g., `__author_class!(third_party)` → `AuthorIdentityClass::ThirdParty`). The helper-macro layer adds an indirection step that obscures error messages slightly when a contributor mis-spells an ident; an alternative is to use `$($variant:tt)*` and let the call site spell the enum variant verbatim (`author_class: AuthorIdentityClass::ThirdParty`). We picked tt-based recognition for compile-time enum-mismatch failures with author-friendly snake-case spellings; fall back to verbatim variant spelling if the helper-macro maintenance burden grows.

**Why a declarative macro and not TOML + parser**:

- App authors write code, not TOML. The macro lives next to the app's source; the `cargo build` step type-checks the manifest against the `Manifest` struct schema.
- Schema breaking changes surface as macro expansion errors at the author's compile time, not at the install-time consumer's runtime.
- Capability names are still strings (the macro doesn't validate them against the kernel's vocabulary) — this is the same trade-off as TOML, just expressed in Rust syntax.

**Why a declarative macro and not a build script**:

- A build script writing a TOML file at build time means the manifest is generated at compile time but parsed at install time — two formats, two paths. The macro emits the typed struct directly; the canonical-bincode encoding happens at signing time (still a separate step but no TOML parse round-trip).
- Build scripts can't fail compilation as cleanly as macros.

**Why a declarative macro and not nothing (TOML-only)**:

- TOML has no compile-time schema validation. Typos in capability names, missing required sections, mistakes in semver strings — all surface at install time.
- The manifest is the **publish-side wire format**; we still need a TOML serializer (which `myrhiza_manifest` already has via `toml_edit`). The macro is the **authoring-side ergonomic surface**. They coexist: the example crates use the macro; an external publisher could still hand-write TOML.

**`bundle!` macro — out of scope**: a stretch design would have a `bundle!` macro that takes the manifest + the three component bytes and emits a signed canonical-bincode bundle. The signing step needs the author's keypair, which only the publisher has at publish time — the macro can't hold the secret. So bundle-build remains a runtime tool (a `myrhiza-cli` subcommand: `myrhiza-cli build-bundle --manifest <path>` could land in a B-9-or-later slice). **Out of scope for B-8.**

## 3. Architecture

### 3.1 SDK crate layout

```
crates/sdk/
├── Cargo.toml
├── src/
│   ├── lib.rs            — module declarations + crate-level doc
│   ├── prelude.rs        — re-export Verdict, LogLevel, Manifest, manifest!, myrhiza_app!
│   ├── types.rs          — re-exports from myrhiza-types (BundleHash, EventHash, Hlc)
│   ├── manifest.rs       — re-exports from myrhiza-manifest::schema
│   ├── macros.rs         — `manifest!`, `myrhiza_app!`, `local_wit_dir!` declarative macros
│   └── boilerplate.rs    — bump allocator + panic handler emitted by myrhiza_app!
├── wit/                  — sibling-synced copy of wit/myrhiza-kernel/wit/*.wit
└── tests/
    └── wit_in_sync.rs    — asserts SDK wit/ == kernel wit/
```

**Cargo.toml**:

```toml
[package]
name = "myrhiza-sdk"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Myrhiza application author surface — WIT, manifest macros, runtime-init boilerplate."

[lints]
workspace = true

[dependencies]
myrhiza-types = { path = "../types" }
myrhiza-manifest = { path = "../manifest" }
wit-bindgen = "0.30"

[features]
default = []
```

The SDK does **not** depend on `myrhiza-kernel`, `myrhiza-backend`, `myrhiza-wasmtime-backend`, or `myrhiza-network`. These are forbidden by §2.4. The SDK depends on `myrhiza-types` (low-level types like `BundleHash`) and `myrhiza-manifest` (manifest schema for the `manifest!` macro target) only.

**Re-export shape** (`src/lib.rs` sketch):

```rust
//! Myrhiza SDK — application author surface.
//!
//! See `myrhiza_sdk::prelude::*` for the common imports. The `myrhiza_app!`
//! macro is the entry point for component crates; the `manifest!` macro is the
//! entry point for manifest authoring.

#![cfg_attr(target_arch = "wasm32", no_std)]

#[cfg(target_arch = "wasm32")]
extern crate alloc;

pub mod prelude;
pub mod types;
pub mod manifest;
pub mod macros;

// boilerplate is only relevant on wasm32 targets — re-exposed via the
// myrhiza_app! macro, not directly consumed.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub mod __boilerplate;
```

**`#[cfg(target_arch = "wasm32")]` is load-bearing**: the SDK is consumed in two roles:

1. By the **example's component crates** (built for `wasm32-unknown-unknown`) — uses `#![no_std]`, the bump allocator, the panic handler.
2. By the **example's manifest builder** (built for the host architecture as a regular Rust binary or build helper) — uses `std` and `myrhiza_manifest`'s TOML/bincode encoders.

The boilerplate module is gated to wasm32 only; the manifest module is always available. The `myrhiza_app!` macro expands to host or wasm depending on context.

### 3.2 Public API sketch

```rust
// crates/sdk/src/prelude.rs
pub use crate::types::{Verdict, LogLevel, Hlc, BundleHash, EventHash};
pub use crate::manifest::{
    Manifest, AppSection, AbiSection, CapabilitiesSection, ComponentsSection,
    AuthorIdentityClass, AuthorPolicy, HighValueOps,
    StateDigestFormat, SignatureAlgorithm,
};
pub use crate::manifest;        // the manifest! macro
pub use crate::myrhiza_app;     // the myrhiza_app! macro
```

**cfg-gate note**: `manifest.rs` and the `manifest!` macro expansion below use `std::collections::BTreeMap`. The `boilerplate` module is cfg-gated to `target_arch = "wasm32"` (no `std`), but `manifest.rs` is **only ever compiled host-side** — never for `wasm32-unknown-unknown` — because manifest authoring is a publishing-side activity. Accordingly, the `manifest!` macro emits `std::*` paths unconditionally; this is safe given the host-side-only consumption context.

```rust
// crates/sdk/src/macros.rs (excerpt — the manifest! macro)
#[macro_export]
macro_rules! manifest {
    (
        app {
            name: $name:literal,
            version: $version:literal,
            description: $description:literal,
            author_class: $class:ident,
        }
        abi {
            kernel_major: $kmaj:literal,
            kernel_minor_min: $kmin:literal,
            state_digest_format: $sdf:ident,
        }
        capabilities {
            $( deterministic_helpers: [ $($helper:literal),* $(,)? ] , )?
            $( host_imports: [ $($hi:literal),* $(,)? ] , )?
            // ... more optional sections
        }
        components {
            $( state_apply: $sa:literal , )?
            $( state_propose: $sp:literal , )?
            $( interaction: $ix:literal , )?
            $( behavior: $bh:literal , )?
        }
    ) => {{
        use $crate::manifest::*;
        let mut m = Manifest {
            app: AppSection {
                name: $name.into(),
                version: $version.into(),
                description: $description.into(),
                author_pubkey: String::new(), // filled at signing time
                author_identity_class: $crate::macros::__author_class!($class),
            },
            abi: AbiSection {
                kernel_major: $kmaj,
                kernel_minor_min: $kmin,
                state_digest_format: $crate::macros::__sdf!($sdf),
            },
            capabilities: CapabilitiesSection {
                deterministic_helpers: {
                    #[allow(unused_mut)]
                    let mut h = std::collections::BTreeMap::new();
                    $( $( h.insert($helper.into(), true); )* )?
                    h
                },
                host_imports: {
                    #[allow(unused_mut)]
                    let mut h = std::collections::BTreeMap::new();
                    $( $( h.insert($hi.into(), true); )* )?
                    h
                },
                ui_surfaces: std::collections::BTreeMap::new(),
                high_value_ops: HighValueOps::default(),
            },
            determinism: DeterminismSection {
                allow_floats: false,
                drift_detection: DriftDetectionSection { interval_events: 1024 },
            },
            modules: ModulesSection { dep: vec![] },
            components: ComponentsSection {
                state_apply: None $(.or(Some($sa.into())))? ,
                state_propose: None $(.or(Some($sp.into())))? ,
                interaction: None $(.or(Some($ix.into())))? ,
                behavior: None $(.or(Some($bh.into())))? ,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: None,
        };
        m.canonicalize();
        m
    }};
}
```

```rust
// crates/sdk/src/macros.rs (excerpt — the myrhiza_app! macro)
#[macro_export]
macro_rules! myrhiza_app {
    (state_apply, $component:ident) => {
        #![no_std]
        #![no_main]
        #![allow(unsafe_op_in_unsafe_fn)]
        extern crate alloc;
        use $crate::__boilerplate::*;
        wit_bindgen::generate!({
            world: "state-apply",
            path: $crate::local_wit_dir!(),
        });
        struct $component;
        export!($component);
    };
    (state_propose, $component:ident) => { /* ... mirroring state_apply ... */ };
    (interaction, $component:ident) => { /* ... mirroring state_apply ... */ };
    (behavior, $component:ident) => { /* ... v1.1 stretch ... */ };
}

/// Resolves to the **consumer** crate's local `wit/` directory.
///
/// Lives in the SDK but expands to `concat!(env!("CARGO_MANIFEST_DIR"), "/wit")`,
/// where `CARGO_MANIFEST_DIR` is set by Cargo to the caller's manifest dir at
/// compile time. The `local_` prefix is a reminder that the bytes consumed are
/// the consumer crate's local copy (kept in sync with `crates/sdk/wit/` via
/// `just sync-wit` — see §2.5).
#[macro_export]
macro_rules! local_wit_dir {
    () => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/wit")
    };
}
```

**Note on `local_wit_dir!`**: this resolves to the **example crate's** `wit/` directory (not the SDK's), because `CARGO_MANIFEST_DIR` is the consumer crate's manifest. The `wit/` directory must be present in the consumer crate. The `local_` prefix encodes this "lives-in-SDK, emits-consumer-path" semantic asymmetry that would otherwise surprise readers expecting `myrhiza_sdk::wit_dir!()` to return the SDK's own dir. This is the same pattern fixtures use today; the `just sync-wit` recipe (§2.5) copies from `crates/sdk/wit/` into `examples/*/wit/` and the in-sync test asserts equality.

**Caveat with concat! and env!**: `wit_bindgen::generate!` takes a `path` that's evaluated at macro-expansion time. Using `concat!(env!("CARGO_MANIFEST_DIR"), "/wit")` works because `env!` is also expanded at macro time, but the path is interpreted relative to wit-bindgen's expectation (which is an absolute path or a path relative to the workspace root). This pattern works in the existing fixtures (`wit_bindgen::generate!({ world: "state-apply" })` defaults to `./wit` relative to `CARGO_MANIFEST_DIR`); the macro is just making the path explicit. Verify at slice time.

### 3.3 Examples directory layout

```
examples/
└── counter/
    ├── Cargo.toml
    ├── README.md           # "How to write a Myrhiza app — counter walkthrough"
    ├── manifest.rs         # uses manifest! macro, builds Manifest struct
    ├── wit/                # synced from crates/sdk/wit/
    │   ├── host-async.wit
    │   ├── host-deterministic.wit
    │   ├── host-non-deterministic.wit
    │   ├── host-ui-surfaces.wit
    │   ├── types.wit
    │   ├── world-behavior.wit
    │   ├── world-interaction.wit
    │   ├── world-state-apply.wit
    │   └── world-state-propose.wit
    └── src/
        ├── lib.rs          # pub use for cargo test --doc; per-bin re-exports
        ├── state.rs        # one component slot — myrhiza_app!(state_apply, Component);
        ├── propose.rs
        └── interaction.rs
```

File names track [`mvp.md §15.4`](2026-05-09-myrhiza-master-design/mvp.md): `src/{state, propose, interaction, behavior}.rs`. `behavior.rs` is absent at v1; ready for v1.1.

**Cargo.toml** for `examples/counter/`:

```toml
[package]
name = "counter-example"
version = "0.1.0"
edition = "2024"
publish = false
description = "Counter app — canonical Myrhiza first-app demo."

[lib]
crate-type = ["cdylib", "rlib"]
path = "src/lib.rs"

[[bin]]
name = "counter-state-apply"
path = "src/state.rs"
required-features = ["state-apply"]

[[bin]]
name = "counter-state-propose"
path = "src/propose.rs"
required-features = ["state-propose"]

[[bin]]
name = "counter-interaction"
path = "src/interaction.rs"
required-features = ["interaction"]

[features]
state-apply = []
state-propose = []
interaction = []

[dependencies]
myrhiza-sdk = { path = "../../crates/sdk" }

[profile.release]
panic = "abort"
lto = true
opt-level = "z"
codegen-units = 1
strip = "symbols"
```

**Caveat — multiple components per crate**: Rust's component-build story prefers **one cdylib per crate** because each `[lib]` block has one `crate-type` set and global allocator. The `[[bin]] + required-features` shape is the workaround: each component is a separate `[[bin]]` artifact, gated by a feature so only one component compiles per `cargo build --features ...` invocation. The Justfile recipe runs `cargo build --target wasm32-unknown-unknown --features state-apply --bin counter-state-apply` three times (once per component).

**Verify at slice 0**: existing fixtures use `crate-type = ["cdylib"]` on `[lib]` only — no `[[bin]]` precedent in tree today. Before B-8.3 ports any code, B-8.0 must validate that `[[bin]] + required-features + crate-type` inheritance (or explicit cdylib via per-target config) actually behaves on `wasm32-unknown-unknown` by building a minimal empty `[[bin]]` stub. If it does not, fall back to the runner-up below — and acknowledge in the slice-0 commit message that the fallback re-introduces the per-crate boilerplate the SDK is meant to consolidate.

**Runner-up — three separate crates `examples/counter-state-apply/`, `examples/counter-state-propose/`, `examples/counter-interaction/`**: simpler from a Cargo-mechanics standpoint, but loses the "one app, three components, one manifest" narrative. The `[[bin]] + required-features` pattern preserves the single-crate-per-app shape that's pedagogically right and matches the mvp.md layout exactly. If the `[[bin]] + required-features + wasm32-unknown-unknown` interaction proves brittle at slice time (see §6 risk row "Cargo's `[[bin]] + required-features`"), fall back to three crates and update the spec. **Trade-off worth surfacing**: that fallback effectively re-creates the per-fixture boilerplate problem the SDK is solving — three crates each repeating the `myrhiza-sdk` dep, manifest stub, and `wit/` sync. The `myrhiza_app!` macro still helps, but the "one app = one Cargo.toml" pedagogical line is lost.

### 3.4 Dep-direction CI mechanism (full spec)

See §2.4 for rule + script sketch. Concrete deliverables:

- New crate `xtask/dep-direction/` (workspace member, `publish = false`, package name `dep-direction-check` so `cargo run -p dep-direction-check` works).
- Dependency on `cargo_metadata = "0.18"` (the standard library for walking Cargo's resolved package graph).
- `src/main.rs` implementing the check (sketch in §2.4).
- `tests/` directory with unit tests that construct synthetic `Metadata` instances and assert violations are detected.
- `Justfile` integration: `just dep-direction` runs the binary; `just ci` includes it.
- README in the crate explaining the rule and the failure diagnostic format.

**Failure-loud test**: a test in `xtask/dep-direction/tests/violations.rs` constructs a synthetic `Metadata` where an `examples/foo` package depends on `myrhiza-kernel`; the test calls the check function and asserts a `Vec<String>` of violations contains the expected diagnostic. **This is the testing-anti-patterns-safe version**: we test the check function's contract, not by mutating real Cargo.toml files.

### 3.5 Migration plan for counter fixtures

Slice-by-slice (full sequence in §4):

1. Create `examples/counter/Cargo.toml` + `examples/counter/src/lib.rs` (empty stub) + `examples/counter/wit/` (synced from kernel).
2. Add `examples/counter` to workspace `members`. The dep-direction check identifies example crates by path match (`examples/*` segment in the manifest path) — no explicit allowlist or `[workspace.metadata]` table is needed.
3. Port `tests/fixtures/counter-state-apply/src/lib.rs` to `examples/counter/src/state.rs`, replacing the bump-allocator + panic-handler + wit-bindgen boilerplate with `myrhiza_app!(state_apply, Component);` and updating `wit_bindgen::generate!`'s path to `myrhiza_sdk::local_wit_dir!()`.
4. Port `tests/fixtures/counter-state-propose/src/lib.rs` → `examples/counter/src/propose.rs`.
5. Port `tests/fixtures/counter-interaction/src/lib.rs` → `examples/counter/src/interaction.rs`.
6. Update `Justfile`'s `build-fixtures` recipe: replace the three counter-* fixture build invocations with three example-targeted builds; output stays at `tests/fixtures/built/counter-{state-apply,state-propose,interaction}.wasm` so test-utils doesn't move.
7. Delete `tests/fixtures/counter-state-apply/`, `tests/fixtures/counter-state-propose/`, `tests/fixtures/counter-interaction/` from disk; remove from workspace `exclude`.
8. Run full CI (`just ci`); assert every test still passes; assert byte-identical state-apply / state-propose / interaction WASM output (the canonical-ABI byte layout is fixed by WIT, and `myrhiza_app!`'s expansion is structurally identical to today's fixture source modulo whitespace — so the WASM should be byte-identical, modulo compiler version drift, which we accept).

**Risk register for the migration**:

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `myrhiza_app!` macro expansion differs from hand-rolled boilerplate in subtle ways (e.g., bump allocator alignment, panic handler signature) | Medium | Migrated counter fixture produces different WASM bytes; wasmtime instantiation succeeds but behavioural divergence possible | Bit-for-bit copy of the fixture's `BumpAlloc` + `#[panic_handler]` into `crates/sdk/src/__boilerplate.rs`; the macro's output is structurally identical to the fixture today. (Counter WASM byte-drift is not pinned by any test today — see §5.2 note.) |
| B-7's E2E CLI test (`crates/myrhiza-cli/tests/e2e.rs`) breaks because the fixture path moves | High (if path moves) | B-7's load-bearing acceptance test (criterion 3) breaks | Don't move the **output path** — only the **source path** moves. `tests/fixtures/built/counter-*.wasm` stays canonical; the Justfile recipe just rebuilds them from a different source dir. |
| `myrhiza-sdk`'s `wit/` directory drifts from `wit/myrhiza-kernel/wit/` | Medium | Apps generate bindings against stale WIT; kernel rejects on instantiate; opaque error | `crates/sdk/tests/wit_in_sync.rs` asserts byte-equality; `just sync-wit` recipe copies kernel → SDK + SDK → examples; CI runs the test |
| Cargo's `[[bin]] + required-features` doesn't compose with `wasm32-unknown-unknown` target (existing fixtures only use `[lib] crate-type = ["cdylib"]` — no `[[bin]]` precedent in tree today) | Low | Migration plan needs restructuring | **Verify at slice 0** by building a single-component empty stub against `wasm32-unknown-unknown` with `[[bin]] + required-features`; if it fails, fall back to three separate `examples/counter-*` crates per §3.3 runner-up — which effectively re-introduces the per-crate boilerplate the SDK is meant to consolidate, a real trade-off to surface in slice-0 commit. |

### 3.6 Root Cargo.toml diff sketch

```diff
 [workspace]
 resolver = "2"
 members = [
     "crates/types",
     "crates/manifest",
     "crates/backend",
     "crates/wasmtime-backend",
     "crates/kernel",
     "crates/network",
+    "crates/sdk",
     "crates/test-utils",
     "crates/myrhiza-cli",
+    "examples/counter",
+    "xtask/dep-direction",
 ]
 exclude = [
     # Wasm-only fixtures — built for wasm32-unknown-unknown by
     # `just build-fixtures` and wrapped into components via wasm-tools.
-    "tests/fixtures/counter-state-apply",
     "tests/fixtures/echo-state-apply",
     "tests/fixtures/over-importer",
     "tests/fixtures/pre-check-rejector",
     "tests/fixtures/infinite-loop",
     "tests/fixtures/float-banned",
-    "tests/fixtures/counter-state-propose",
-    "tests/fixtures/counter-interaction",
 ]
```

The `examples/counter` member **must** build for the workspace's default target (the host's native target — usually `x86_64-unknown-linux-gnu`). With the `[[bin]] + required-features` shape, building `examples/counter` with no features compiles only the `[lib]` (which is just re-exports) and is harmless. The Justfile recipe explicitly passes `--target wasm32-unknown-unknown --features state-apply --bin counter-state-apply` to build the actual component.

**Caveat — workspace compile-check pollution**: `cargo check --workspace` will compile `examples/counter`'s `[lib]` against the host target. Since the lib is empty (just re-exports), this is harmless. If we later add host-side code to `examples/counter` (e.g., a build-helper for manifest emission), the host target must support it.

`xtask/dep-direction` is a host-only binary; it compiles cleanly under workspace builds.

## 4. Slice sequence

Five PRs at B-4-cadence (1-2 days each). Total: 2-3 days focused work matching the gap-analysis estimate.

### B-8.0 — SDK crate scaffold + WIT directory + in-sync test

Create `crates/sdk/{Cargo.toml, src/lib.rs, src/prelude.rs, src/types.rs, src/manifest.rs}` as a thin re-export crate (no macros yet). Copy `wit/myrhiza-kernel/wit/*.wit` into `crates/sdk/wit/`. Add `crates/sdk` to workspace members. Add `crates/sdk/tests/wit_in_sync.rs` asserting byte-equality between SDK wit/ and kernel wit/. Add `just sync-wit` recipe (copies kernel → SDK).

Unit tests: type re-exports compile; SDK depends only on `types` + `manifest`; wit_in_sync test passes.

**Why first**: every other slice imports `myrhiza-sdk`. Landing the crate as a no-op re-export decouples the SDK landing from the macro and migration work.

### B-8.1 — `myrhiza_app!` boilerplate macro + `local_wit_dir!` helper

Add `crates/sdk/src/__boilerplate.rs` (bump allocator + panic handler, gated to wasm32). Add `crates/sdk/src/macros.rs` with `myrhiza_app!` and `local_wit_dir!` macros. Add `pub use` lines to prelude.

Unit tests: build a minimal in-tree wasm test crate that uses `myrhiza_app!` and assert it produces a valid component via wasm-tools (test infrastructure already exists in `tests/fixtures/built/`).

### B-8.2 — `manifest!` declarative macro

Add the `manifest!` macro body to `crates/sdk/src/macros.rs`. Helper sub-macros `__author_class!`, `__sdf!` to translate ident → enum variant.

Unit tests: `manifest! { … }` produces a `Manifest` struct equivalent to today's `helpers_only_three_component_manifest`; canonicalization is idempotent; missing required sections fail to compile.

### B-8.3 — Migrate counter fixtures to `examples/counter/`

Per §3.5 step-by-step. Touches `examples/counter/**`, root `Cargo.toml`, `Justfile`'s build-fixtures recipe, optionally `crates/test-utils/src/bundle.rs` if paths change (they shouldn't per §3.5 step 6). Deletes the three `tests/fixtures/counter-*` directories.

CI check: B-7's `crates/myrhiza-cli/tests/e2e.rs` still passes (same counter behaviour, byte-equivalent WASM modulo compiler drift); all `tests/fixtures/built/` byte-pinned regression tests pass.

### B-8.4 — `xtask/dep-direction/` CI check

Create the workspace member per §3.4. Wire into `Justfile`'s `ci` target. Unit tests asserting violation detection. Self-test: run the check after migration → passes; assert that adding `myrhiza-kernel` to `examples/counter/Cargo.toml` would fail (unit-tested via synthetic Metadata, not by actually mutating the file).

**Why last**: examples/ + the migration must exist before there's anything to check. Landing the CI check first against an empty examples/ tree would be a no-op.

**Order rationale**: B-8.0 → B-8.1 → B-8.2 unblock the SDK surface incrementally; B-8.3 consumes the surface; B-8.4 protects it. Each slice is independently testable and reversible.

## 5. Test plan

### 5.1 SDK unit tests (slices B-8.0–B-8.2)

- `crates/sdk/tests/wit_in_sync.rs` — byte-equality between `crates/sdk/wit/` and `wit/myrhiza-kernel/wit/`. Runs on every CI invocation.
- `crates/sdk/tests/manifest_macro.rs` — `manifest!` produces canonical `Manifest` struct equivalent to hand-built struct; canonicalize is idempotent; missing fields → compile error (negative test via `trybuild` if we want strict compile-fail coverage, otherwise documented in macro docs).
- `crates/sdk/tests/myrhiza_app_macro.rs` — only meaningful when targeting wasm32; gated `#[cfg(target_arch = "wasm32")]`. CI workflow includes a `cargo build --target wasm32-unknown-unknown -p myrhiza-sdk --tests` step. Not strictly required for v1 if the integration via `examples/counter/` covers the path (which it does).

### 5.2 End-to-end (slice B-8.3)

- The full B-7 acceptance test (`crates/myrhiza-cli/tests/e2e.rs`) re-runs against the new `examples/counter`-sourced WASM artifacts. The test does not change; only the source location changes. Pre-check ≡ apply assertion at every step.
- The kernel acceptance suite (`crates/kernel/tests/acceptance.rs` + `crates/kernel/tests/coexistence.rs` + the iroh convergence/coexistence tests) all re-run unchanged.

**Acceptance: same number of passing tests; no regression in coverage; byte-pinned regressions (e.g., `wire_freeze.rs`'s `bundle_content_hash_three_component_fixture_is_frozen` test from B-7.0) still pass.**

Note on counter byte-drift coverage: `crates/types/tests/wire_freeze.rs` only pins `bundle_content_hash(Some(b"a"), Some(b"b"), Some(b"c"), None)` — synthetic literals, not counter WASM bytes. No test catches silent byte-drift on the counter fixture today; this PR does not change that. Fixture hashes are not a stability commitment — only event/gossip wire format is. The migration thus has nothing to "re-pin"; it neither regresses nor improves this coverage.

### 5.3 CI check fails-loud (slice B-8.4)

- `xtask/dep-direction/tests/violations.rs` — synthetic `Metadata` with a forbidden edge → check returns non-empty violations; assert diagnostic format.
- `xtask/dep-direction/tests/clean.rs` — synthetic `Metadata` with only allowed edges (examples → SDK → types/manifest only) → check returns empty violations.
- CI runs `just dep-direction` on every PR; fails the gate on violation.

## 6. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `[[bin]] + required-features` doesn't compose cleanly with `cargo build --target wasm32-unknown-unknown` (existing fixtures use `[lib] crate-type = ["cdylib"]` only — no `[[bin]]` precedent) | Low | §3.3 runner-up: split into three crates | Verify at slice B-8.0 by building an empty stub; fall back to three crates if needed. **Slice-0 verification gate:** if `[[bin]] + required-features` does not work on `wasm32-unknown-unknown`, the runner-up (three crates per app) effectively re-creates the boilerplate problem the SDK is solving — a real trade-off worth surfacing |
| `myrhiza_app!` macro semantics differ subtly from hand-rolled boilerplate (e.g., bump allocator alignment edge cases) | Low | Subtle determinism bug; per-peer state divergence | Boilerplate is bit-copied from the existing fixture; macro is structural-only; CI golden-byte test on counter pinned to a hash post-migration |
| Macro proliferation tempts contributors to add more macros to the SDK; surface grows uncontrolled | Medium | SDK becomes a maintenance burden over time | This spec explicitly defers proc-macros and helper macros to a later slice; macro additions require a spec amendment |
| WIT drift between `crates/sdk/wit/` and `wit/myrhiza-kernel/wit/` despite the in-sync test (e.g., contributor adds a file to kernel-wit and forgets the sync) | Medium | App bindings drift; kernel rejects apps at install | `just sync-wit` runs `cp -r wit/myrhiza-kernel/wit/* crates/sdk/wit/` + `cp -r crates/sdk/wit/* examples/counter/wit/`. Pre-commit hook recommended (out of scope for B-8 spec; flagged as carryover). CI test fails closed. |
| Cargo workspace `members` ordering matters for some tools; adding `examples/counter` between `myrhiza-cli` and excluded fixtures could surface ordering bugs | Low | Tool-specific surprise | Tested at slice B-8.3 via `just ci`; if any tool complains, reorder. |
| Contributors unfamiliar with the `xtask/` convention look for scripts under `scripts/` or `.github/scripts/` and don't find the check | Low | Bikeshedding | Resolved in §2.4 — using `xtask/`, the Cargo-community-blessed convention (see [matklad's xtask blog post](https://github.com/matklad/cargo-xtask)); the `Justfile` aliases `just dep-direction` so contributors don't need to know the directory layout |

## 7. Deferred items (B-8 carryovers)

| Item | Trigger to revisit |
|---|---|
| Proc-macro layer (`state_apply!`, `state_propose!`, `interaction!` proc-macros wrapping `wit_bindgen::generate!`) | When 3+ apps exist and we see common patterns in their `impl Guest` blocks that could be lifted out. Earliest: after B-6 (poll app) lands. |
| `bundle!` macro / `myrhiza-cli build-bundle` subcommand | When app authors actually publish bundles outside the test harness. Earliest: B-10 (iroh-blobs distribution). |
| Per-language SDKs (JS via componentize-js, Python via componentize-py) | v1.5+ — see [`prior-art/spin/sdks-and-tooling.md`](../prior-art/spin/sdks-and-tooling.md) for the production reference pattern. Bundles balloon by 5–35 MB; payoff is reach. |
| Capability-name compile-time validation (proc-macro that checks declared helpers against `myrhiza_manifest::vocabulary`) | When capability vocabulary stabilizes and the misspelling surface becomes a documented friction point |
| Pre-commit hook for `just sync-wit` | When `wit/` drift becomes a recurring CI failure. The hook is straightforward; out of scope for B-8 to keep the slice small. |
| Spec-author macro: `manifest!` is for app authors; spec authors may want a `spec_test_manifest!` that builds a synthetic manifest for tests | Likely re-usable from `crates/test-utils/src/manifest.rs`'s `helpers_only_*` patterns. Out of scope for B-8. |
| `examples/echo/` (or migration of echo to examples) | Echo's role is "second WASM blob for coexistence test." If we ever want it to be a real example, B-6 or later. |
| Migration of negative-test fixtures (`over-importer`, `pre-check-rejector`, etc.) into a `tests/fixtures/negative/` subfolder | Cosmetic cleanup; no functional benefit |
| The `local_wit_dir!()` macro is hacky (relies on `CARGO_MANIFEST_DIR/wit` convention); a future improvement would have the SDK ship its WIT via `include_bytes!` + write it to `OUT_DIR/wit` at SDK consumer build time | When publishing the SDK to crates.io becomes a goal. Out of scope for v1. |

## 8. Prior-art citations

Per [CLAUDE.md](../../CLAUDE.md): cite folder + section, name runner-up paradigms, flag gaps.

### 8.1 Holochain's HDK — runner-up SDK paradigm (rejected for v1)

[`prior-art/holochain/lessons.md`](../prior-art/holochain/lessons.md) Avoid row 1: "Custom WASM ABI that can't survive a host upgrade. HDK breakage every minor release is the symptom." Holochain's HDK is a full cookbook SDK — lifecycle callbacks, capability convenience helpers, error types, prelude — and every minor release broke it. **Lesson borrowed**: keep the SDK surface small; let app authors talk to WIT directly via `wit-bindgen`; the SDK is a *thin* compatibility layer, not a framework. This argues choice (a) over choice (c) explicitly.

[`prior-art/holochain/apps.md`](../prior-art/holochain/apps.md) — "builder-tools-for-builders trap": Holochain has framework users (hREA, Neighbourhoods) but no flagship consumer app. **Lesson borrowed**: Myrhiza's SDK targets end-app authors, not framework authors. The `examples/counter/` is a complete app, not a starter framework. The SDK does NOT provide an "app framework" abstraction layer; apps are WASM components, full stop.

### 8.2 wasmCloud — no separate guest SDK (validates §2.5)

[`prior-art/wasmcloud/tooling.md`](../prior-art/wasmcloud/tooling.md) line 132: "There is no separate 'wasmcloud-rs' SDK at the guest-component layer in v2 — the guest writes against WASI interfaces and Wasmtime hosts them." Validates the choice to **not** generate bindings inside the SDK and re-export them. The SDK ships WIT; the guest invokes `wit_bindgen::generate!` itself; the binding lives in the guest's crate. Same pattern as wasmCloud v2 (where guest = WASI-imports + WIT, no wasmCloud-specific SDK).

[`prior-art/wasmcloud/lessons.md`](../prior-art/wasmcloud/lessons.md) Avoid row 6: "Tooling as parallel CLIs (`wash`, `wadm`, `cosmo`, `washboard`). Onboarding cost; 'which command do I run?' friction." Myrhiza's CLI surface is one binary (`myrhiza-cli`); the SDK is one crate (`myrhiza-sdk`). Don't fragment.

### 8.3 Spin — borrowing CLI shape + manifest-static capability declaration

[`prior-art/spin/sdks-and-tooling.md`](../prior-art/spin/sdks-and-tooling.md): Spin's CLI shape (`spin new`/`build`/`up`/`watch`/`registry push`) and `#[http_component]` macro. **Lesson borrowed for §2.1's `myrhiza_app!`**: a single attribute-shaped macro can replace ~80 LOC of boilerplate per component. Spin's `#[http_component]` is a proc-macro because it parses Rust attribute syntax; Myrhiza's `myrhiza_app!` is a `macro_rules!` because it expands to a complete crate-attribute block (`#![no_std]`, `#![no_main]`, `extern crate alloc`, etc.) that can't be in an attribute macro. Same idea, different mechanism.

[`prior-art/spin/lessons.md`](../prior-art/spin/lessons.md) Borrow row 3: "Manifest-static capability declaration (`spin.toml`). A component's permissions are declared in the manifest, *not* requested at runtime." Validates the `manifest!` macro's role — it's not a runtime-cap-request mechanism; it's a static publishing artifact.

[`prior-art/spin/lessons.md`](../prior-art/spin/lessons.md) Borrow row 1: "Factor architecture (SIP-021)" — the per-capability host module pattern. **Not borrowed for B-8** (the SDK is on the guest side; factors are kernel-side); flagged as relevant for a later kernel-internal refactoring spec.

### 8.4 Component Model — WIT-first authoring + canonical ABI

[`prior-art/wasm-component-model/lessons.md`](../prior-art/wasm-component-model/lessons.md) Validates row 1: "Imports as the only host surface. A component sees nothing it didn't `import`. No ambient FS, no implicit network, no syscall." The SDK doesn't introduce ambient imports; it ships WIT that declares them explicitly, then the kernel binds (or refuses to bind) per the manifest.

[`prior-art/wasm-component-model/lessons.md`](../prior-art/wasm-component-model/lessons.md) Borrow §"The 4-pass authoring model" + §"The world as the unit of capability declaration": WIT → bindings → core wasm → component. The SDK lives at pass 1 (provides WIT) and pass 2 (the `myrhiza_app!` macro emits the `wit-bindgen` macro invocation). Passes 3 and 4 are the build-tool's job (Justfile's `_build-fixture` recipe).

[`prior-art/wasm-component-model/tooling.md`](../prior-art/wasm-component-model/tooling.md) §"`cargo-component` — Rust toolchain integration": the current state of cargo-component is "maintained but slow-moving"; the recommended path today is direct `wit-bindgen` + `wasm-tools component new`. **Lesson borrowed**: don't depend on cargo-component; the SDK + Justfile recipe replicate the same flow without the third-party dep.

### 8.5 Willow — proto-spec patterns + four-profile shape

[`prior-art/willow/`](../prior-art/willow/) — Willow's `state-apply` + `state-propose` + `interaction` + `behavior` profile shape is lifted into Myrhiza ([`prior-art/willow/runtime-vision.md`](../prior-art/willow/runtime-vision.md) §"four-component-profile table"). The SDK's `myrhiza_app!` macro variants (`state_apply` / `state_propose` / `interaction` / `behavior`) mirror this directly. **No surprise — Myrhiza is a generalization of Willow.** The migration of `tests/fixtures/counter-*` to `examples/counter/{state.rs, propose.rs, interaction.rs}` aligns the on-disk layout with the conceptual shape Willow established.

### 8.6 Gaps in prior-art

- **No off-the-shelf "WASM-Component-Model SDK" precedent for a P2P deterministic-state-apply runtime.** Spin/wasmCloud are server-side request-driven; Holochain is its own ABI; Croquet is JS-on-V8. The closest single-crate SDK for component authors is `spin-sdk` (Rust crate, ~50 deps) for the request-handler pattern. B-8 takes the *shape* (single crate, prelude, manifest authoring helpers) without the *runtime model* (Myrhiza is `(prior, event) → next`, not request-handler).
- **No production reference for the `manifest!` declarative macro in Rust.** `spin-sdk` uses a `#[http_component]` proc-macro for the component-export side but the manifest (`spin.toml`) is hand-written TOML. The macro form is a Myrhiza-specific choice.
- **No prior-art for cross-WIT-version SDK compatibility.** What happens when the kernel WIT bumps and the SDK is on an older version? Component Model has the adapter-module pattern (`prior-art/wasm-component-model/lessons.md` §"Adapter components for ABI migration"). For B-8's v1 timeframe, the SDK is pinned 1:1 to the kernel WIT version; multi-version support is a v1.5+ concern.

**Promotion candidates** (per `using-prior-art` §3):

- **`prior-art/cargo-component/`** — depth on the Bytecode-Alliance Rust component build tooling, its bus factor (maintained but slow-moving per [`prior-art/wasm-component-model/tooling.md`](../prior-art/wasm-component-model/tooling.md) §"cargo-component"), and the trade-off vs direct `wit-bindgen` + `wasm-tools` (which is what Myrhiza uses today). Currently not covered as its own folder; embedded inside `wasm-component-model/`. Low-priority; covers a single dependency that's not load-bearing.
- **`prior-art/elm-architecture/`** — TEA + Redux + Vue store all share the `dispatch(action) → update(msg, model) → (model, cmd)` shape, mirroring Myrhiza's `dispatch → propose → apply`. Useful for B-6 (poll app) interaction-layer design but not blocking for B-8. Was already flagged in [B-7 spec §8](2026-05-21-plan-b-7-interaction-harness-design.md) as a promotion candidate.

## 9. Estimate

**2-3 days inline / 3-5 days under full subagent-driven review cadence**, framing the [post-B-7 gap analysis](../reports/2026-05-21-mvp-gap-analysis.md#b-8-sdk-ergonomics--examples-wiring) figure as the inline-execution case. The subagent-driven figure accounts for review-round overhead — observed ~20 minutes per round across this session's data points, adding roughly one day spread across the 10-task sequence.

Breakdown (inline focused work):

- B-8.0 (SDK scaffold + WIT in-sync test): 0.5 day
- B-8.1 (`myrhiza_app!` + `local_wit_dir!` macros): 0.5 day
- B-8.2 (`manifest!` macro): 0.5 day
- B-8.3 (migrate counter fixtures): 0.5–1 day (the long pole — Justfile + test-utils + wire-freeze re-pin all touch)
- B-8.4 (dep-direction CI check): 0.5 day

Risk pad: 0.5 day for the `[[bin]] + required-features + wasm32-unknown-unknown` interaction (§3.3 runner-up if it doesn't work).

## 10. Acceptance criteria

B-8 ships when:

- [ ] `crates/sdk/` exists as a workspace member with `lib.rs` re-exports, `wit/` directory (bit-identical to `wit/myrhiza-kernel/wit/`), and a passing `wit_in_sync` test.
- [ ] `myrhiza_sdk::prelude` brings `Verdict`, `Hlc`, `LogLevel`, `Manifest`, and the `manifest!` / `myrhiza_app!` macros into scope.
- [ ] `manifest!` macro expands to a canonical `Manifest` struct equivalent to today's hand-rolled `helpers_only_three_component_manifest`. (Note: the macro invokes `.canonicalize()` on the emitted struct as its last step, which sorts maps deterministically — the resulting `Manifest` is "as-if signed" minus the signature, so downstream code can hash / serialize it without re-canonicalizing.)
- [ ] `myrhiza_app!` macro emits the bump allocator + panic handler + `wit_bindgen::generate!` + `export!` boilerplate for `state_apply`, `state_propose`, `interaction`, and `behavior` profile variants.
- [ ] `examples/counter/` exists as a workspace member with one Cargo.toml, three component slots (`src/state.rs`, `src/propose.rs`, `src/interaction.rs`), one manifest builder (`manifest.rs` using `manifest!`), and a synced `wit/` directory.
- [ ] `tests/fixtures/counter-state-apply/`, `tests/fixtures/counter-state-propose/`, and `tests/fixtures/counter-interaction/` are deleted; their workspace-exclude entries are removed.
- [ ] `tests/fixtures/built/counter-state-apply.wasm`, `tests/fixtures/built/counter-state-propose.wasm`, and `tests/fixtures/built/counter-interaction.wasm` are produced by `just build-fixtures` from the new `examples/counter/` source.
- [ ] Every existing test (`just ci`) passes; pre-check ≡ apply assertion in `crates/myrhiza-cli/tests/e2e.rs` still holds; any wire-freeze hash re-pin is documented in the slice's commit message.
- [ ] `xtask/dep-direction/` workspace member implements the dep-direction check; `just dep-direction` runs it; `just ci` includes it.
- [ ] Unit tests in `xtask/dep-direction/tests/` cover both clean-graph and violation cases via synthetic `Metadata`.
- [ ] [`docs/reports/2026-05-21-mvp-gap-analysis.md`](../reports/2026-05-21-mvp-gap-analysis.md) updates items 20 + 24 from ❌ to ✅.

## 11. Resolved decisions (none currently open)

The six critical ambiguities are resolved inline in §2:

| # | Question | Decision |
|---|---|---|
| 1 | SDK scope for v1 | Type re-exports + WIT + `manifest!` + `myrhiza_app!` declarative macros (§2.1 choice a) |
| 2 | SDK crate structure | Single `crates/sdk/` (no proc-macro split needed) (§2.2) |
| 3 | Examples migration scope | Hybrid: migrate counter only; leave echo + negative fixtures (§2.3 choice c) |
| 4 | Dep-direction CI mechanism | Bespoke `xtask/dep-direction/` Rust binary using `cargo_metadata` (§2.4) |
| 5 | wit-bindgen integration | SDK ships WIT directory; macro emits `wit_bindgen::generate!` in consumer crate (§2.5) |
| 6 | Manifest authoring | Declarative `manifest!` macro (§2.6 choice b) |

No items remaining for the plan-writer phase to resolve. Implementation details (exact `manifest!` DSL syntax tweaks, specific `cargo_metadata` version pin) are matters of taste resolved at slice time.
