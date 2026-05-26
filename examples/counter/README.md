# counter — canonical first Myrhiza app

A shared-counter app demonstrating the three component slots
(`state-apply`, `state-propose`, `interaction`) wired through the
`myrhiza-sdk` author surface. The single crate produces three WASM
artifacts via the `[[bin]] + required-features` shape — one per
component slot — keeping the "one app = one Cargo.toml, one manifest"
narrative intact.

The component logic in `src/state.rs`, `src/propose.rs`, and
`src/interaction.rs` is application code; the `myrhiza_app!(<slot>,
Component);` macro at the top of each file emits the `#![no_std]` /
`#![no_main]` shell, the bump allocator, the panic handler, the
`wit_bindgen::generate!` call against the local `wit/` directory, and
`export!(Component)`. The bundle manifest is authored in `manifest.rs`
via the `manifest!` declarative macro (host-only, behind
`#[cfg(not(target_arch = "wasm32"))]`).

Build a single component manually:

```bash
cargo build --target wasm32-unknown-unknown --release \
    --features state-apply --bin counter-state-apply
```

The canonical build path uses `just build-fixtures` from the workspace
root, which produces wrapped components at
`tests/fixtures/built/counter-{state-apply,state-propose,interaction}.wasm`.

See `docs/specs/2026-05-26-b-8-sdk-design.md` for the SDK design and
the rationale behind the single-crate-per-app shape.
