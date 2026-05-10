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

build-fixtures:
    @echo "build-fixtures wired in Task 39"

ci: fmt-check lint test
