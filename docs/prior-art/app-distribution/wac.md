**Date:** 2026-05-22
**Status:** active
**Subject:** `wac` (WebAssembly Compositions) — build-time component composition. Where the multi-component bundle becomes a single artifact.

# `wac`

## What it is

`wac` ("**w**eb**a**ssembly **c**ompositions", pronounced "whack") is the Bytecode Alliance's declarative composition tool for WASM components. It takes one or more components and produces one component, wiring their exports and imports together according to a `.wac` recipe — a small declarative superset of WIT.

- **Repo:** [`github.com/bytecodealliance/wac`](https://github.com/bytecodealliance/wac)
- **License:** Apache-2.0
- **Current release:** v0.10.0 (2024-04-17). Library crate `wac-graph` 0.10.0 alongside CLI `wac-cli` 0.10.0.
- **Disambiguation:** there is an unrelated abandoned `wac` crate at 0.0.1 from 2020 in crates.io — not the BA project. Always check `wac-cli` or look for `bytecodealliance/wac` in the source path.

## What composition means here

A WASM component has a typed interface: imports (functions it expects from the host) and exports (functions it provides). Two components compose when one's exports satisfy another's imports.

Two compositional regimes:

| Regime | Where | How |
|---|---|---|
| **Build-time (static)** | `wac` | One `.wasm` produced from N inputs. The component model "fuses" them; the linker resolves imports inside the bundle. Result is a single deployable artifact. |
| **Run-time (dynamic)** | host runtime (Wasmtime `add_to_linker`, wasmCloud's wRPC, Spin's host SDK) | Host wires imports at instantiation. Result is N artifacts that live together at runtime. |

`wac` is the canonical build-time tool. Runtime composition is host-specific and not standardized.

For Myrhiza: the four-component-profile app shape (`state-apply` + `state-propose` + `interaction` + `behavior`) is naturally a build-time composition. `wac` exists and works; if Myrhiza picks the CM as foundation, we get this for free.

## `.wac` file shape

```wac
// glue.wac — compose `auth` and `app` components into a single `myapp` component

package my-org:myapp@0.1.0;

let auth = new my-org:auth { ... };
let app = new my-org:app {
  // wire the auth component's exports into the app's imports
  authenticator: auth.authenticator,
  ...
};

// re-export the app's wasi:http handler outward
export app.handler as handler;
```

Each `let` binding instantiates a component, wiring imports to either other components' exports or to passthrough host imports. The final `export` lines declare what the composed component exposes outward.

CLI:

```bash
wac compose glue.wac -o myapp.wasm   # declarative form: full .wac language
wac plug input.wasm --plug other.wasm -o composed.wasm  # quick form: "fill A's imports from B"
wac resolve glue.wac                  # parse + type-check without writing output
```

`wac plug` is the simple case — "this component's imports are satisfied by that component's exports, just do the obvious thing." `wac compose` is the full declarative form when there are multiple inputs or non-obvious wiring.

## Relationship to `wasm-tools compose`

Before `wac` matured, `wasm-tools compose` was the BA's composition tool — a less expressive predecessor that did the equivalent of `wac plug`. **`wasm-tools compose` is deprecated**; `wac` is the supported successor. The deprecation is mentioned in `wasm-tools` 1.0 release notes (BA 2024) and the `wasm-tools` README points at `wac`.

## Relationship to `wasm-tools link`

`wasm-tools link` does **module linking** (legacy `.wasm` modules, not components). Don't confuse it with composition. If you're composing components, you want `wac`.

## Composition vs. dependency resolution

Worth keeping crisp:

- **`wac compose`** says: "given these N components, produce one fused component." It needs the components themselves and the recipe.
- **`wkg`** says: "given a WIT world that names dependencies, fetch the components from a registry." It doesn't fuse anything; it just resolves names → bytes.

Together: `wkg` fetches the deps → `wac` fuses them → the result ships as an OCI artifact via `wkg oci push` or `oras push`.

For Myrhiza: this two-step pipeline (fetch → compose → publish) is the right pattern for app bundling. Spin already runs it inside `spin build` when an app declares cross-component dependencies; see [`spin/sdks-and-tooling.md:85-114`](../spin/sdks-and-tooling.md).

## What `wac` doesn't do

- **Versioning.** `wac` doesn't pin component versions; that's `wkg.toml`'s job (or your `Cargo.toml` / `package.json` equivalent).
- **Signing.** The composed `.wasm` is unsigned bytes; signing is a separate Cosign/Notation step against the resulting artifact, after publish.
- **Distribution.** `wac` outputs a `.wasm`; you then push it. `wac` itself doesn't touch any registry.
- **Runtime substitution.** A composed component is fused at build time; you can't swap one piece out without recomposing and republishing. (Compare wasmCloud's link definitions, which are runtime-mutable.)
- **Conditional composition.** No `#if WASM_FEATURE_X then component-a else component-b` story. Everything is declarative-static.

## Implications for Myrhiza

**Yes-borrow:** `wac` is the right composition primitive for multi-profile apps. A Myrhiza app's `state-apply` + `state-propose` + `interaction` + `behavior` components compose into one bundle at build time. We don't need to invent a composition language; `.wac` already does it.

**Caveat:** the determinism contract that the four-profile spec requires (`state-apply` is pure, etc.) is enforced at the *runtime* boundary by the kernel, not at composition time. `wac` will happily compose a non-deterministic `state-apply` with the rest of the app — it doesn't know about Myrhiza's profile distinctions. Validation that a `state-apply` component is determinism-safe has to happen elsewhere (a `myrhiza validate` step? a kernel-side WIT-world type check?). See [`open-problems.md`](./open-problems.md) §determinism-validation.

**Runtime composition is a separate decision.** If Myrhiza wants apps to dynamically wire to host capabilities at instantiation (the way Spin / wasmCloud do), that's a host-side `add_to_linker` story and `wac` is irrelevant. The two regimes can coexist: build-time `wac` for app-internal composition, runtime host-linking for kernel↔app capability binding. This is roughly Spin's pattern.

**Tooling bus factor:** `wac` is again BA-small-team. v0.10.0 was 2024-04; no 2025 release as of this writing. Less actively developed than `wkg`. Don't assume permanent maintenance; budget for forking if it becomes critical.

## Sources

- `wac`: <https://github.com/bytecodealliance/wac>
- crates.io `wac-cli`: <https://crates.io/crates/wac-cli>
- Lin Clark's "Components" talk explains the composition model (2024 WasmCon): see [`wasm-component-model/`](../wasm-component-model/) sources
- `wasm-tools compose` deprecation: <https://github.com/bytecodealliance/wasm-tools>
- Spin's `wac` integration: [`spin/sdks-and-tooling.md:85`](../spin/sdks-and-tooling.md)
- Bytecode Alliance "Composing components" blog (2023): <https://bytecodealliance.org/articles/wac-composition-tool>
