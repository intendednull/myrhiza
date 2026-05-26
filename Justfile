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
# Tools required: rustup target wasm32-unknown-unknown, wasm-tools.
build-fixtures: \
    (_build-fixture "counter-state-apply" "counter_state_apply_fixture" "state-apply") \
    (_build-fixture "echo-state-apply" "echo_state_apply_fixture" "state-apply") \
    (_build-fixture "over-importer" "over_importer_fixture" "state-apply") \
    (_build-fixture "pre-check-rejector" "pre_check_rejector_fixture" "state-apply") \
    (_build-fixture "infinite-loop" "infinite_loop_fixture" "state-apply") \
    (_build-fixture "float-banned" "float_banned_fixture" "state-apply") \
    (_build-fixture "counter-state-propose" "counter_state_propose_fixture" "state-propose") \
    (_build-fixture "counter-interaction" "counter_interaction_fixture" "interaction") \
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

spec-coverage:
    ./scripts/spec-coverage.sh

spec-coverage-check: spec-coverage
    @if ! git diff --exit-code tests/spec-coverage.md; then \
        echo "tests/spec-coverage.md is stale. Run 'just spec-coverage' and commit."; \
        exit 1; \
    fi

ci: fmt-check lint test test-iroh spec-coverage-check
