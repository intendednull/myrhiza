**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — glossary of substrate-specific terms

# Glossary

System-specific terms used across this folder. Cross-references to the file where the term is treated in depth.

## Spec / type system

- **Component** — a unit of WASM code that imports and exports typed interfaces. Not the same as a core wasm module; a component bundles core modules + adapter modules + linkage. See [spec.md](spec.md), [abi.md](abi.md).
- **Core module** — a plain WebAssembly 1.0 / 2.0 module: linear memory, functions, no typed imports beyond i32/i64/f32/f64. The thing components wrap. See [abi.md](abi.md).
- **WIT (Wasm Interface Types)** — the IDL the Component Model uses to describe component boundaries. File extension `.wit`. See [spec.md](spec.md).
- **Interface** — a named collection of typed functions and types in WIT (e.g. `wasi:io/streams`). See [spec.md](spec.md).
- **World** — a collection of imports + exports a component fulfills. Defines the component's "shape" toward its host. See [spec.md](spec.md).
- **Package** — a versioned collection of WIT files (`namespace:name@semver`, e.g. `wasi:io@0.2.11`). See [spec.md](spec.md).
- **Resource** — a typed, opaque handle to host- or component-managed state. Two kinds: `own` (caller has unique ownership, calls drop) and `borrow` (caller has a temporary reference). See [abi.md](abi.md).
- **Future** — a single-shot async value in preview3 WIT. See [preview-status.md](preview-status.md).
- **Stream** — a multi-shot async value in preview3 WIT. See [preview-status.md](preview-status.md).
- **Error-context** — the preview3 mechanism for attaching diagnostic detail to a failed future/stream. See [preview-status.md](preview-status.md).
- **Variant / record / enum / option / result / tuple / list / flags** — the WIT primitive type set. See [spec.md](spec.md).

## ABI

- **Canonical ABI** — the rules that map WIT types onto core wasm i32/i64/f32/f64 + linear memory. Document at `design/mvp/CanonicalABI.md`. See [abi.md](abi.md).
- **Lift** — the operation that reads core wasm bytes and produces a WIT-typed value (host-side input). See [abi.md](abi.md).
- **Lower** — the inverse: serialize a WIT-typed value into core wasm bytes (guest-side input). See [abi.md](abi.md).
- **`realloc`** — guest-exported function the host calls to allocate space for arguments it's lowering into the guest's linear memory. See [abi.md](abi.md).
- **`post-return`** — guest-exported function the host calls after lifting return values, to let the guest free the memory holding those return values. See [abi.md](abi.md).
- **Adapter module** — a WASM module that the component model toolchain inserts to bridge between core wasm and the canonical ABI. See [abi.md](abi.md).
- **Canonical built-ins** — the small set of host-provided functions a component model runtime must implement (e.g. resource intrinsics, async machinery). See [abi.md](abi.md).
- **Shared-nothing linkage** — components do not share linear memory; each instance has its own. The default and only model in the current spec. See [abi.md](abi.md).
- **Shared-everything linkage** — historical alternative (rejected) where components shared memory. Not in the current spec. See [abi.md](abi.md).

## Wasmtime

- **`Engine`** — Wasmtime's compilation context, shared across instances. Holds JIT caches and configuration. See [wasmtime.md](wasmtime.md).
- **`Store`** — per-instance state container. Owns the linear memory, table, globals, host data. Not shared between instances. See [wasmtime.md](wasmtime.md).
- **`Component`** — a compiled component, ready to be instantiated. See [wasmtime.md](wasmtime.md).
- **`Instance`** — a running component. Has live memory, resource tables, etc. See [wasmtime.md](wasmtime.md).
- **`Linker`** — the type-checked import resolver. Wires host functions to component imports. See [wasmtime.md](wasmtime.md).
- **`Func`** — a typed handle to a callable function (host-imported or component-exported). See [wasmtime.md](wasmtime.md).
- **Cranelift** — Wasmtime's JIT/AOT codegen backend. Rust crate `cranelift-codegen`, currently `0.131.1`. See [wasmtime.md](wasmtime.md).
- **Fuel** — Wasmtime's per-instruction-count metering. Component charges fuel against `Store::add_fuel` budget per executed instruction. Set via `Config::consume_fuel(true)`. See [wasmtime.md](wasmtime.md).
- **Epoch** — Wasmtime's cooperative time-slicing. Host bumps an epoch counter; component yields when its epoch deadline is exceeded. Set via `Config::epoch_interruption(true)`. See [wasmtime.md](wasmtime.md).
- **`Module::serialize` / `Engine::precompile_component`** — produces an AOT-compiled artifact that can be loaded with no JIT overhead. Does not capture live instance heap state. See [wasmtime.md](wasmtime.md).

## WASI versions

- **Preview1** (`wasi_snapshot_preview1`) — the original POSIX-shaped WASI. Core-module-only; not component-model-native. Still everywhere in CLI tooling and pre-CM toolchains.
- **Preview2** — the first componentized WASI. Released as `wasi:*@0.2.x` interfaces. Current stable version `0.2.11` (2026-04-07). See [preview-status.md](preview-status.md).
- **Preview3** — the in-flight async-native WASI. Three RCs as of 2026-05-09 (`v0.3.0-rc-2026-01-06`, `-2026-02-09`, `-2026-03-15`); no `v0.3.0` final yet. Tracking-milestone open since 2023-08-22 with no `due_on`. See [preview-status.md](preview-status.md).
- **`wasi:io`** — base streams, polls, error interface. Foundation for everything else.
- **`wasi:cli`** — command-line program shape. `command` world.
- **`wasi:http`** — HTTP client + server interfaces.
- **`wasi:filesystem`** — file I/O.
- **`wasi:clocks`** — wall + monotonic clocks.
- **`wasi:random`** — PRNG and entropy.
- **`wasi:sockets`** — TCP/UDP. Browser-not-supported.

## Authoring tooling

- **`wasm-tools`** — Bytecode Alliance Swiss-army CLI: validate, parse, dump, print, embed, strip, component-from-core, wat-to-wasm. Currently `1.248.0`. See [tooling.md](tooling.md).
- **`wit-bindgen`** — host + guest binding generators. Macro form (`wit_bindgen::generate!{}`) + CLI form. Targets Rust, C, C++, TinyGo, MoonBit, JS (via jco), Python (via componentize-py). Currently `0.57.1`. See [tooling.md](tooling.md), [languages.md](languages.md).
- **`cargo-component`** — Rust toolchain integration. `cargo component build` produces a `.wasm` component. Currently `0.21.1`, possibly superseded by `cargo build --target wasm32-wasip2`. See [tooling.md](tooling.md), [languages.md](languages.md).
- **`wac` / `wac-cli`** — WebAssembly Compositions: declarative stitching of components together at the WIT level. Currently `0.10.0`. See [tooling.md](tooling.md).
- **`wkg`** — Wasm package manager (`bytecodealliance/wasm-pkg-tools`). Fetches WIT packages via OCI. Currently `0.15.0`. See [tooling.md](tooling.md).
- **`jco`** — Component Model in JavaScript. `jco transpile` (CM → JS+core wasm), `jco componentize`, `jco wit`. Currently `1.19.0`. See [browser.md](browser.md), [tooling.md](tooling.md).
- **`componentize-js`** — bundles SpiderMonkey or QuickJS into a component, so JavaScript code can ship as a CM component. Currently `0.20.0`. See [tooling.md](tooling.md), [languages.md](languages.md).
- **`componentize-py`** — bundles CPython into a component for Python guests. Currently `0.23.0`. Bundle size ~35MB for hello-world (`componentize-py#98`). See [tooling.md](tooling.md), [languages.md](languages.md).
- **`preview2-shim` / `preview3-shim`** — JS shims that emulate WASI 0.2.x / 0.3.x in browser or Node. Currently `0.17.9`. `preview3-shim` is Node-only. See [browser.md](browser.md).

## Governance

- **BA / Bytecode Alliance** — the foundation that stewards Wasmtime, the spec working impls, and most CM tooling. 501(c)(6); GitHub org created 2019-08-12; public launch 2019-11-12. See [governance.md](governance.md).
- **TSC** — Technical Steering Committee. Sets technical direction across BA-stewarded projects. See [governance.md](governance.md).
- **CG (Community Group)** — WebAssembly Community Group at the W3C. The standards body where CM proposals are drafted. See [governance.md](governance.md).
- **WG (Working Group)** — WebAssembly Working Group at the W3C. Where W3C standards (including future CM standardization) are formalized. See [governance.md](governance.md).
- **OCI-as-registry** — the convention of using OCI image registries (Docker Hub, GHCR) to publish/distribute WIT packages and components. `wkg` is the BA-stewarded client. See [tooling.md](tooling.md).

## Cross-substrate (for comparison with our Agoric folder)

- **`xsnap`** — Agoric's per-vat live-instance snapshot tool. Captures a running JS heap. **Wasmtime has no equivalent** for components — `Module::serialize` captures the compiled artifact, not the live state. See [wasmtime.md](wasmtime.md).
- **Vat** (Agoric) — analogous to a CM component instance: single-threaded, sandboxed, capability-mediated. The Agoric vat lifecycle + transcript-replay model is the closest production analog to Myrhiza's `state-apply` profile. See [`../agoric-endo/vat-model.md`](../agoric-endo/vat-model.md).
- **CapTP** (Agoric / Spritely) — distributed object-capability protocol. Orthogonal to CM; CM defines local component interfaces, CapTP defines cross-machine ones. See [`../spritely-ocapn/captp-and-ocapn.md`](../spritely-ocapn/captp-and-ocapn.md).

## Sources

- WIT spec: https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md
- Canonical ABI spec: https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md
- Wasmtime API docs: https://docs.rs/wasmtime/
- WASI proposals: https://github.com/WebAssembly/WASI
- Bytecode Alliance: https://bytecodealliance.org/
