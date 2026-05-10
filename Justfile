default: ci

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace --all-targets

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
build-fixtures:
    @mkdir -p tests/fixtures/built
    cd tests/fixtures/counter-state-apply && \
        cargo build --release --target wasm32-unknown-unknown --locked --frozen
    wasm-tools component embed \
        tests/fixtures/counter-state-apply/wit \
        tests/fixtures/counter-state-apply/target/wasm32-unknown-unknown/release/counter_state_apply_fixture.wasm \
        --world state-apply \
        -o tests/fixtures/built/counter-state-apply.embed.wasm
    wasm-tools component new \
        tests/fixtures/built/counter-state-apply.embed.wasm \
        -o tests/fixtures/built/counter-state-apply.wasm
    rm tests/fixtures/built/counter-state-apply.embed.wasm

spec-coverage:
    ./scripts/spec-coverage.sh

spec-coverage-check: spec-coverage
    @if ! git diff --exit-code tests/spec-coverage.md; then \
        echo "tests/spec-coverage.md is stale. Run 'just spec-coverage' and commit."; \
        exit 1; \
    fi

ci: fmt-check lint test spec-coverage-check
