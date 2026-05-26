# `dep-direction-check`

Bespoke CI check that asserts no workspace member under `examples/*`
depends — directly or transitively — on a kernel-internal crate.

Per [`docs/specs/2026-05-26-b-8-sdk-design.md`](../../docs/specs/2026-05-26-b-8-sdk-design.md)
§2.4: example apps must reach the kernel only through the published
`myrhiza-sdk` surface. The SDK transitively pulls in `myrhiza-types`
and `myrhiza-manifest` (which it re-exports); that is permitted. The
kernel-internal crates the SDK consumers must NEVER name are:

- `myrhiza-kernel`
- `myrhiza-backend`
- `myrhiza-wasmtime-backend`
- `myrhiza-network`
- `myrhiza-test-utils`
- `myrhiza-cli`
- `myrhiza-distribution` (future-proof — listed ahead of B-10)

## Running

```bash
cargo run -p dep-direction-check
# or, equivalently
just dep-direction          # wired in B-8 T8
```

Exit codes:

- `0` — graph is clean. Prints `dep-direction OK`.
- `1` — at least one violation. Prints one diagnostic per line in the
  shape `{example_name}: transitive dep on forbidden crate {name}`.
- `2` — `cargo metadata` itself failed.

## Why a bespoke check, not `cargo-deny`?

`cargo-deny`'s `bans.deny` list is workspace-global and cannot express
"crate X may not depend on Y, but the rest of the workspace may." We
need conditional bans keyed on the dependent crate's path. See the
[rejected alternatives in §2.4](../../docs/specs/2026-05-26-b-8-sdk-design.md)
for the full comparison.

## Adding a forbidden crate

Edit `FORBIDDEN` in `src/lib.rs`. Add the corresponding unit-test
fixture under `tests/synthetic.rs` if the new crate exercises a path
not already covered (the existing tests assert the contract — direct
edges, transitive edges, and the clean graph — so adding a name to
the list inherits the same coverage shape).

## Dev-dep filtering

The check walks only **normal** and **build** edges of the resolve
graph. A dev-dep on a *transitive* dependency (e.g.
`myrhiza-types` → `myrhiza-network` as a dev-dep) is not pulled into
a consumer's link graph and is therefore not a violation. See
`src/lib.rs`'s module docs for the rationale.

## Unit tests

`tests/synthetic.rs` hand-rolls `cargo metadata`'s JSON output and
deserializes it into `cargo_metadata::Metadata`. This lets the check
function be exercised without mutating real `Cargo.toml` files —
testing-anti-patterns-safe per
[`docs/specs/2026-05-26-b-8-sdk-design.md`](../../docs/specs/2026-05-26-b-8-sdk-design.md)
§3.4.

Three cases are covered:

1. `clean_graph_returns_no_violations` — `examples/counter` →
   `myrhiza-sdk` → `myrhiza-types`. No `FORBIDDEN` edges. Returns
   `[]`.
2. `forbidden_edge_returns_violation` — `examples/counter` directly
   depends on `myrhiza-kernel`. Returns one diagnostic.
3. `transitive_forbidden_edge_returns_violation` — `examples/counter`
   → `myrhiza-sdk` → `myrhiza-kernel`. Returns one diagnostic.
