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

spec-coverage:
    ./scripts/spec-coverage.sh

spec-coverage-check: spec-coverage
    @if ! git diff --exit-code tests/spec-coverage.md; then \
        echo "tests/spec-coverage.md is stale. Run 'just spec-coverage' and commit."; \
        exit 1; \
    fi

ci: fmt-check lint test spec-coverage-check
