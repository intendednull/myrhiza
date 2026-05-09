**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — Bytecode Alliance authoring tools

# Component Model Tooling

This file surveys the tools that turn source code into WASM components and
move components between machines. The audience is Myrhiza spec authors deciding
what authoring surface to support and which tools we'd reuse vs. replace.

For language-specific toolchain details see [`languages.md`](./languages.md).
For the runtime / host side see [`wasmtime.md`](./wasmtime.md). For the WIT and
binary surface see [`spec.md`](./spec.md) and [`abi.md`](./abi.md). For preview
status see [`preview-status.md`](./preview-status.md). The wider ecosystem
context is in [`ecosystem.md`](./ecosystem.md). Compare with
[Agoric's bundle-source story](../agoric-endo/modules-and-bundling.md) for an
alternate take on what "bundle a component" means.

All version numbers below were verified against crates.io / npm / GitHub on
2026-05-09. Pre-1.0 packages move fast; treat the versions as a snapshot.

---

## `wasm-tools` — the swiss-army CLI

Repo: <https://github.com/bytecodealliance/wasm-tools>
Crate: `wasm-tools` 1.248.0 (2026-04-28). Library crates `wit-component`,
`wit-parser`, `wit-smith` etc. share the same release train at `0.248.x`.

`wasm-tools` is the single most-used CLI in the component-model world. It
covers everything from raw module work to component-specific subcommands.
Most other tools (`cargo-component`, `wac`, `wkg`) link against the same
underlying library crates.

Install:

```
cargo install wasm-tools
```

Useful subcommands, grouped by purpose:

- **Inspect / debug.** `wasm-tools dump`, `wasm-tools print` (binary → WAT),
  `wasm-tools parse` (WAT → binary), `wasm-tools objdump`, `wasm-tools
  validate`, `wasm-tools strip` (drop custom sections), `wasm-tools
  metadata show`.
- **Component lifecycle.** `wasm-tools component new` wraps a core module
  plus an embedded WIT (the component-type custom section) into a real
  component. `wasm-tools component embed` puts a `component-type` custom
  section into a core module so a core toolchain can produce
  component-ready output. `wasm-tools component wit` reverses the flow: it
  prints the WIT a component declares.
- **Build glue.** `wasm-tools compose` (legacy linker, superseded by `wac`),
  `wasm-tools mutate` (fuzzing), `wasm-tools shrink`, `wasm-tools demangle`.
- **Authoring.** `wasm-tools metadata add` to stamp producer info,
  `wasm-tools addr2line` for source-map style debug.

One-liner — convert a Rust core wasm into a component without
`cargo-component`:

```
rustc --target wasm32-wasip1 -O lib.rs -o core.wasm \
  && wasm-tools component embed wit/world.wit core.wasm -o embedded.wasm \
  && wasm-tools component new embedded.wasm -o component.wasm \
       --adapt wasi_snapshot_preview1=wasi_snapshot_preview1.reactor.wasm
```

For Myrhiza: `wasm-tools` is the lingua franca. Anything we ship that
manipulates components — bundling, signing, stripping debug sections,
verifying that an app declares no host imports outside our capability set —
will either call out to it or link `wit-component` / `wasmparser` directly.
We will almost certainly vendor parts of it.

---

## `wit-bindgen` — host and guest binding generators

Repo: <https://github.com/bytecodealliance/wit-bindgen>
Crate: `wit-bindgen` 0.57.1 (2026-04-17). CLI crate `wit-bindgen-cli` at the
same version.

`wit-bindgen` translates a WIT world into language-specific glue. It exists
in two surfaces:

- **The macro form** — `wit_bindgen::generate!{ world: "my-world", path:
  "wit" }` from a Rust crate. Most Rust component code uses this. The macro
  re-runs the generator at build time, so updating the WIT updates the
  generated module without a separate codegen step. Default ABI mode is
  the canonical CABI; preview2 streams / futures appear as opaque resource
  handles where supported.
- **The CLI form** — `wit-bindgen <lang> wit/ --out-dir bindings/`. Targets
  in-tree as of 0.57: `rust`, `c`, `markdown`, `moonbit`. The previous
  `tiny-go` / `teavm-java` / `csharp` generators were spun out (TinyGo's
  generator is now in TinyGo itself) or unmaintained — verify per language
  in [`languages.md`](./languages.md).

Architecture: each language generator is a Rust crate implementing a
shared trait that walks a resolved WIT package and emits source. This makes
out-of-tree generators feasible — Java's TeaVM bindings, the older C# work,
and `componentize-py` all consume the same `wit-parser` resolver.

One-liner — generate Rust bindings for a world:

```
wit-bindgen rust ./wit --world my-world --out-dir src/bindings
```

For Myrhiza: we want `wit-bindgen` for Rust, period. The other languages we
care about (see [`languages.md`](./languages.md)) bring their own
generators or are downstream of `wit-bindgen` indirectly. The macro form is
the right authoring path for app authors writing Rust components against a
Myrhiza WIT world.

---

## `cargo-component` — Rust toolchain integration

Repo: <https://github.com/bytecodealliance/cargo-component>
Crate: `cargo-component` 0.21.1 (2025-04-07).

The version date here is older than `wasm-tools` / `wit-bindgen`. As of
this writing the project has not had a release in roughly a year; Rust
component authoring has been migrating toward the `wit-bindgen` macro plus
`wasm-tools component new` flow, or toward `cargo build --target
wasm32-wasip2` once support stabilises. **Treat `cargo-component` as
maintained but slow-moving and verify the situation when you next reach for
it.**

What it does when it works:

- `cargo component new my-app` scaffolds a crate with `wit/world.wit` and a
  `lib.rs` implementing the world.
- `cargo component build` compiles to wasm, runs `wit-bindgen` codegen,
  then runs `wasm-tools component new` — output is a real component, not a
  core module.
- Resolves WIT package dependencies via a registry config (`Cargo.toml`
  `[package.metadata.component]` plus a project-level lockfile capturing
  resolved package versions).

Install:

```
cargo install cargo-component
```

For Myrhiza: useful as a reference path for app authors today. Whether we
recommend it or recommend the `wasm-tools` + `wit-bindgen` direct path
depends on the state of `wasm32-wasip2` rustc support. Either way our app
build template should produce the same artifact: a component binary plus
its declared WIT world.

---

## `wac` — WebAssembly Compositions

Repo: <https://github.com/bytecodealliance/wac>
Crate: `wac-cli` 0.10.0 (2026-04-17). Library crate `wac-graph` 0.10.0.
(Note: there is an unrelated abandoned `wac` crate at 0.0.1 from 2020 — not
this project.)

`wac` is a small declarative language for stitching components together at
the WIT level — think "make for components". A `.wac` file names component
files, declares which exports of one connect to which imports of another,
and produces a single composed component.

Status: pre-1.0. The CLI works and is in active use within wasmCloud /
Wasmtime examples. The composition language has had several breaking
revisions — pin a version in any reproducible pipeline.

One-liner:

```
wac plug input.wasm --plug other.wasm -o composed.wasm
```

(`wac plug` is the simple "fill in imports of A using exports of B"
mode. `wac compose deps.wac` is the full declarative form.)

For Myrhiza: composition is interesting because Myrhiza apps are bundles of
components with kernel-mediated wiring. Whether `wac`'s static composition
fits — or whether all wiring should happen at the kernel layer at runtime —
is a design question. See `app-bundle` design notes when they land. At
minimum `wac-graph` (the library) is a useful reference for "given these
components and these WIT worlds, can the imports be satisfied?".

---

## `wkg` — wasm package manager

Repo: <https://github.com/bytecodealliance/wasm-pkg-tools>
Crate: `wkg` 0.15.0 (2026-02-06).

`wkg` is the package fetch / publish tool for the component-model world.
It reads a `wkg.toml` declaring WIT package dependencies, resolves them
against configured registries, and downloads or publishes them.

Registry backends: OCI registries (Docker Hub, GHCR, etc.) and the
pre-standard "warg" registry. The OCI path is dominant — components and
WIT packages live as OCI artifacts under a custom media type. This piggy-
backs on the existing container-registry infrastructure rather than
building yet-another package server.

Common commands:

```
wkg get wasi:http@0.2.0          # fetch a WIT package
wkg publish my-component.wasm    # push a component to a registry
wkg wit fetch                    # populate ./wit/deps from wkg.toml
```

For Myrhiza: distribution is one of the open design questions. We will
need *some* answer for "where do app bundles live and how are they
addressed". `wkg` plus OCI-as-registry is one available answer — the
trade-off is centralised registry hosts vs. our broader P2P story. Compare
with [Agoric's hash-addressed bundles via Endo
sources](../agoric-endo/modules-and-bundling.md), which sidestep registries
entirely.

---

## `componentize-js` — JS guest builder

Repo: <https://github.com/bytecodealliance/jco/tree/main/packages/componentize-js>
npm: `@bytecodealliance/componentize-js` 0.20.0 (2026-04-14).

`componentize-js` takes a JavaScript source file plus a WIT world and emits
a component. Internally it bundles a SpiderMonkey engine (StarlingMonkey),
freezes the JS source into the component's data section, and exposes the
declared WIT exports as JS functions.

Tradeoffs:

- **Size.** A minimal JS component lands at roughly 5–10 MB because the JS
  engine is along for the ride. Treeshaking helps, and WIZER-based
  pre-initialisation removes parse-time startup cost, but the floor is the
  engine.
- **Speed.** Cold-start is fast (engine is pre-initialised); steady-state
  is JS-engine speed, not native.
- **Async.** Limited; preview2 stream/future support is partial.

Install (as part of jco):

```
npm install -g @bytecodealliance/jco @bytecodealliance/componentize-js
```

One-liner:

```
jco componentize app.js --wit ./wit --world-name my-world --out app.wasm
```

---

## `componentize-py` — Python guest builder

Repo: <https://github.com/bytecodealliance/componentize-py>
PyPI: `componentize-py` 0.23.0 (2026-04-15). The GitHub repo only tags a
`canary` release; the PyPI wheel is the canonical artifact.

Same shape as `componentize-js` but for CPython: bundles a CPython
interpreter into the component, freezes Python sources alongside, exposes
declared WIT exports as Python callables. Bundle size is similar (tens of
MB) and steady-state performance is CPython speed.

Install:

```
pip install componentize-py
```

One-liner:

```
componentize-py -d wit -w my-world componentize app -o app.wasm
```

For both `componentize-{js,py}`: the model is "ship the engine in every
component". Acceptable for cloud serverless, possibly fine for Myrhiza
desktop/server peers, painful for resource-constrained nodes. Whether we
want to support these for app authors is a policy choice not a technical
one — once we accept components, we accept anyone's components.

---

## `jco` — JS toolchain Swiss army knife

Repo: <https://github.com/bytecodealliance/jco>
npm: `@bytecodealliance/jco` 1.19.0 (2026-04-22).

`jco` is the JS-side analogue of `wasm-tools`. The browser-relevant pieces
(`jco transpile`, the runtime polyfills) are covered in
[`browser.md`](./browser.md). On the dev / authoring side, the pieces that
matter for tooling:

- `jco componentize` — wraps `componentize-js`. The "build a JS component"
  entry point.
- `jco transpile` — convert a component into ES module JS that can run on
  any JS engine (relevant for browser hosting, deferred to `browser.md`).
- `jco wit` — print the WIT a component declares (parallels
  `wasm-tools component wit`).
- `jco new` — scaffold a JS component project.
- `jco run` — run a component in Node.js using the preview2 shim
  (`@bytecodealliance/preview2-shim`, currently 0.17.x).

Install:

```
npm install -g @bytecodealliance/jco
```

For Myrhiza: `jco transpile` in particular matters for the hypothetical
"run an app component in a browser peer" path. That belongs in
[`browser.md`](./browser.md).

---

## OCI as a component registry

The de-facto distribution path for components is "publish them to an OCI
registry", which means Docker Hub, GHCR, ECR, Quay etc. all already work.
The convention:

- A component is pushed as an OCI artifact with a custom media type
  (`application/vnd.wasm.component.v1+wasm`) — not as a container image.
- WIT packages similarly get their own media type and live as artifacts.
- Wasmtime, wasmCloud, and `wkg` all understand the convention; tools like
  `oras` work too.
- Authentication, signing, and discovery come for free from the existing
  OCI ecosystem (cosign, Sigstore, etc.).

For Myrhiza: this is the most-trodden path. The relevant questions are
whether OCI's centralised-registry posture is acceptable and whether
component identity / addressing should be content-hash-based (Agoric-style,
Spritely-style) or registry-coordinate-based (OCI-style). The tooling in
this file all assumes the latter — adopting it pulls us toward registry
coordinates whether we want them or not.

---

## What Myrhiza reuses vs. replaces

Reuse outright:

- `wasm-tools` and its underlying library crates (`wasmparser`,
  `wit-parser`, `wit-component`) — these are how we parse / validate / load
  / print components. No replacement is sane.
- `wit-bindgen` for Rust component authors. The macro form is the
  recommended app-author path.
- A component build path: probably `cargo build --target wasm32-wasip2`
  once it stabilises, with `wasm-tools component new` as the fallback. The
  status of `cargo-component` directly is a watch-item — see version note
  above.

Reuse with care:

- `wac` / `wac-graph` for declarative composition — useful as a library,
  but Myrhiza's runtime wiring of capabilities is the authoritative
  composition step. We may compose at build time *and* at kernel load
  time.
- `componentize-{js,py}` — accept them as valid component producers, but do
  not endorse them as the recommended Myrhiza authoring path; the size /
  performance cost is too high for a default.

Replace or reimagine:

- **Distribution.** `wkg` + OCI is *a* working answer. Whether it's the
  Myrhiza answer is open — see [Agoric's bundle-source
  story](../agoric-endo/modules-and-bundling.md) for the alternative. Likely
  Myrhiza ships its own app-bundle format and addresses bundles by content
  hash, with OCI registries as one possible transport.
- **App bundling.** A Myrhiza app is "components plus a WIT world plus a
  capability manifest plus state-apply / propose / interaction / behavior
  profile assignments" — none of the existing tools know about any of that.
  We will write an `myrhiza-bundle` tool that uses `wasm-tools` + `wac` as
  libraries.

For language-side details see [`languages.md`](./languages.md). For
unresolved problems with the tooling chain see
[`open-problems.md`](./open-problems.md) and [`critiques.md`](./critiques.md).

---

## Sources

- <https://github.com/bytecodealliance/wasm-tools>
- <https://crates.io/crates/wasm-tools>
- <https://github.com/bytecodealliance/wit-bindgen>
- <https://crates.io/crates/wit-bindgen>
- <https://github.com/bytecodealliance/cargo-component>
- <https://crates.io/crates/cargo-component>
- <https://github.com/bytecodealliance/wac>
- <https://crates.io/crates/wac-cli>
- <https://github.com/bytecodealliance/wasm-pkg-tools>
- <https://crates.io/crates/wkg>
- <https://github.com/bytecodealliance/jco>
- <https://www.npmjs.com/package/@bytecodealliance/jco>
- <https://www.npmjs.com/package/@bytecodealliance/componentize-js>
- <https://github.com/bytecodealliance/componentize-py>
- <https://pypi.org/project/componentize-py/>
- <https://component-model.bytecodealliance.org/>
- <https://github.com/bytecodealliance/registry> (warg)
- <https://oras.land/> (OCI Registry As Storage)
