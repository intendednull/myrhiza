default: ci

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace --all-targets

# Feature-gated iroh transport tests. Default-off (see
# crates/network/Cargo.toml `[features]`) because iroh pulls in QUIC +
# TLS + relay deps the in-process MemNetwork double does not need.
# Wired into `ci` below; run standalone for fast iteration on the
# iroh transport during B-4.x work.
test-iroh:
    cargo test -p myrhiza-network --features network-iroh --tests
    cargo test -p myrhiza-kernel --features network-iroh --tests
    cargo test -p myrhiza-test-utils --features network-iroh --tests

check:
    cargo check --workspace --all-targets

# Build wasm component fixtures.
#
# We deviate from the plan's cargo-component recipe here. cargo-component
# (and the rustc wasm32-wasip2 target) link a WASI shim that pulls in
# wasi:io/poll, wasi:cli/exit, etc. as component-level imports — even for
# `#![no_std]` crates. The kernel state-apply linker provides ONLY the
# host-deterministic helper set per architecture.md §3.5; any extra
# import causes instantiation to fail.
#
# So instead we compile to a core wasm module (`wasm32-unknown-unknown`,
# no WASI) and then use `wasm-tools component embed` + `component new` to
# wrap it into a component with the fixture's WIT. Result: a component
# with only the imports declared in the fixture's WIT.
#
# Counter slots are sourced from `examples/counter/` per spec §3.5
# (B-8 T6 cutover); the five negative-test fixtures remain in
# `tests/fixtures/`. Output paths under `tests/fixtures/built/` are
# unchanged so kernel + CLI consumers don't move.
#
# Tools required: rustup target wasm32-unknown-unknown, wasm-tools.
build-fixtures: \
    (_build-example "counter-state-apply" "state-apply" "state-apply") \
    (_build-example "counter-state-propose" "state-propose" "state-propose") \
    (_build-example "counter-interaction" "interaction" "interaction") \
    (_build-fixture "echo-state-apply" "echo_state_apply_fixture" "state-apply") \
    (_build-fixture "over-importer" "over_importer_fixture" "state-apply") \
    (_build-fixture "pre-check-rejector" "pre_check_rejector_fixture" "state-apply") \
    (_build-fixture "infinite-loop" "infinite_loop_fixture" "state-apply") \
    (_build-fixture "float-banned" "float_banned_fixture" "state-apply") \
    (_build-fixture "poll-state-apply" "poll_state_apply_fixture" "state-apply") \
    (_build-fixture "poll-state-propose" "poll_state_propose_fixture" "state-propose") \
    (_build-fixture "poll-interaction" "poll_interaction_fixture" "interaction")
    @echo "Built 11 fixtures into tests/fixtures/built/"

# Compile a single fixture into a wasm component. `crate_name` is the
# Rust crate name with hyphens replaced by underscores (cargo's artifact
# filename rule). `dir` is the fixture directory under tests/fixtures/.
# `world` is the WIT world name passed to `wasm-tools component embed`.
_build-fixture dir crate_name world:
    @mkdir -p tests/fixtures/built
    cd tests/fixtures/{{dir}} && \
        cargo build --release --target wasm32-unknown-unknown --locked
    wasm-tools component embed \
        tests/fixtures/{{dir}}/wit \
        tests/fixtures/{{dir}}/target/wasm32-unknown-unknown/release/{{crate_name}}.wasm \
        --world {{world}} \
        -o tests/fixtures/built/{{dir}}.embed.wasm
    wasm-tools component new \
        tests/fixtures/built/{{dir}}.embed.wasm \
        -o tests/fixtures/built/{{dir}}.wasm
    rm tests/fixtures/built/{{dir}}.embed.wasm

# Compile one slot of `examples/counter/` into a wasm component. The
# example crate is a workspace member and exposes three `[[bin]]`
# artifacts (`counter-state-apply`, `counter-state-propose`,
# `counter-interaction`), each gated by `required-features` so a single
# `cargo build --features <feature> --bin <slot>` produces only that
# binary. `feature` matches the bin's `required-features` (one of
# `state-apply` / `state-propose` / `interaction`); `world` is the WIT
# world name passed to `wasm-tools component embed`.
#
# Per docs/specs/2026-05-26-b-8-sdk-design.md §3.3 + §3.5: the
# `[[bin]] + required-features` shape preserves the "one app, three
# components, one manifest" narrative. `myrhiza_sdk::myrhiza_app!`
# emits the `extern crate alloc;` + bump allocator + `#[panic_handler]`
# + `wit_bindgen::generate!` + `export!` boilerplate; the workspace
# root's `[profile.release]` block (panic=abort, lto, opt-level=z,
# strip) provides the float-ban-compliance opts.
_build-example slot feature world:
    @mkdir -p tests/fixtures/built
    cargo build --release --target wasm32-unknown-unknown --locked \
        -p counter-example --features {{feature}} --bin {{slot}}
    wasm-tools component embed \
        examples/counter/wit \
        target/wasm32-unknown-unknown/release/{{slot}}.wasm \
        --world {{world}} \
        -o tests/fixtures/built/{{slot}}.embed.wasm
    wasm-tools component new \
        tests/fixtures/built/{{slot}}.embed.wasm \
        -o tests/fixtures/built/{{slot}}.wasm
    rm tests/fixtures/built/{{slot}}.embed.wasm

# Rebuild all wasm fixtures and fail if the committed
# tests/fixtures/built/*.wasm differ from a fresh build. The committed
# artifacts are the source of truth for contributors without a wasm
# toolchain (`cargo test` loads them directly), so they must stay in
# lockstep with the examples/counter/ + tests/fixtures/* sources.
# Mirrors spec-coverage-check below; CI runs this instead of bare
# build-fixtures.
#
# NOTE: build-fixtures cannot run from a git worktree created under
# .claude/worktrees/ — the excluded standalone fixture crates walk past
# the worktree root to the main repo's workspace, which does not list
# the worktree-path copies. Run from the primary checkout.
build-fixtures-check: build-fixtures
    @if ! git diff --exit-code tests/fixtures/built/; then \
        echo "tests/fixtures/built/*.wasm is stale. Run 'just build-fixtures' and commit."; \
        exit 1; \
    fi

spec-coverage:
    ./scripts/spec-coverage.sh

spec-coverage-check: spec-coverage
    @if ! git diff --exit-code tests/spec-coverage.md; then \
        echo "tests/spec-coverage.md is stale. Run 'just spec-coverage' and commit."; \
        exit 1; \
    fi

# Dep-direction check: examples/* MUST NOT transitively depend on
# kernel-internal crates. Per docs/specs/2026-05-26-b-8-sdk-design.md §2.4.
dep-direction:
    cargo run -p dep-direction-check --quiet

ci: fmt-check lint test test-iroh spec-coverage-check dep-direction

# Sync `crates/sdk/wit/` and `examples/counter/wit/` from the
# canonical `wit/myrhiza-kernel/wit/`. Run when the kernel WIT
# changes; the in-sync test (`crates/sdk/tests/wit_in_sync.rs`)
# asserts SDK ↔ kernel bit-equality. `examples/counter/wit/` is
# a separate copy because `wit_bindgen::generate!` resolves its
# default `./wit` path against the consumer's `CARGO_MANIFEST_DIR`
# (per spec §3.3 / macros.rs::myrhiza_app docs).
sync-wit:
    cp -p wit/myrhiza-kernel/wit/*.wit crates/sdk/wit/
    cp -p crates/sdk/wit/*.wit examples/counter/wit/
    @echo "WIT files synced from wit/myrhiza-kernel/wit/ → crates/sdk/wit/ → examples/counter/wit/"
