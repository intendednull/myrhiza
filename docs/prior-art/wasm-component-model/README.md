**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — Bytecode-Alliance-stewarded WASM substrate (spec + Wasmtime + tooling) that Myrhiza is committing to as foundation

# WASM Component Model

The substrate Myrhiza is built on. Not one project but a coherent ecosystem under one steward:

- **The spec** — `WebAssembly/component-model` CG proposal. Defines WIT (the IDL), the component binary format, the canonical ABI for cross-language type lifting/lowering, and the world/interface/package taxonomy.
- **WASI 0.2.x** — the first standardized interface set written in WIT (`wasi:io`, `wasi:cli`, `wasi:http`, `wasi:filesystem`, `wasi:clocks`, `wasi:random`, `wasi:sockets`). Versioned independently of the spec.
- **Wasmtime** — the Bytecode-Alliance reference runtime. Rust, Cranelift JIT, monthly major-version cadence. The component-model implementation Myrhiza will embed.
- **Toolchain** — `wasm-tools`, `wit-bindgen`, `cargo-component`, `wac-cli`, `wkg`, `jco`, `componentize-js`, `componentize-py`. Authoring + composition + packaging.
- **Browser path** — no native browser CM in any engine; userland transpilation via `jco transpile`. Sole route until vendors adopt.

Unlike the Spritely / Agoric / Holochain folders, this is **not a learn-from neighbor**. It is a hard dependency. The whole Myrhiza spec rests on it. Treat the corpus accordingly: every claim, every version pin, every preview-status note will leak into Myrhiza specs that get written against it.

## Key facts

| Fact | Value |
|---|---|
| Steward | Bytecode Alliance Foundation (501(c)(6) industry consortium) |
| Founders | Mozilla, Fastly, Intel, Red Hat (public launch 2019-11-12; GitHub org `bytecodealliance` created 2019-08-12) |
| Members | 23 organizations as of 2026-05-09 (incl. Amazon, Microsoft, Intel, Mozilla, Fastly, Cosmonic, Fermyon, Shopify, DFINITY, Igalia, Stellar Development Foundation; full list in [governance.md](governance.md)) |
| Governance | Board of Directors (Holley chair), TSC (Spencer chair), working groups; 107 public BA repos |
| Spec status | CG proposal at the WebAssembly Community Group; **not a W3C Recommendation** |
| Spec repo HEAD | `WebAssembly/component-model` `669d494` on 2026-05-07 (no GitHub releases or git tags — spec ships via main) |
| Canonical ABI doc | `design/mvp/CanonicalABI.md` (~63KB; recent edits 2026-04-27 `Store.{lift,lower}` refactor, 2026-05-06 typo pass) |
| WASI 0.2.x current | `v0.2.11` released 2026-04-07; previous stable `v0.2.10` 2026-02-03 |
| WASI 0.3.0 status | Three pre-release RCs only: `v0.3.0-rc-2026-01-06`, `-2026-02-09`, `-2026-03-15`; **no `v0.3.0` final** |
| WASI 0.2.0 first release | 2024-04-24 |
| Wasmtime current | `44.0.1` on crates.io 2026-04-30 (patch); major `v44.0.0` published 2026-04-20; monthly cadence (cut on 5th, publish on 20th); LTS on majors divisible by 12 (24-month support); `36.0.9` LTS patch published 2026-05-05 |
| Recent Wasmtime majors | v40 (2025-12-22), v41 (2026-01-20), v42 (2026-02-24), v43 (2026-03-20), v44 (2026-04-20) |
| Cranelift codegen | `0.131.1` (Wasmtime's JIT backend) |
| `wit-bindgen` | `0.57.1` (2026-04-17) — host + guest binding generators |
| `wasm-tools` | `1.248.0` (2026-04-28) — Swiss-army CLI |
| `cargo-component` | `0.21.1` (2025-04-07) — **stale, possibly superseded by `cargo build --target wasm32-wasip2`** |
| `wac-cli` | `0.10.0` (2026-04-17) — declarative composition (alpha-ish; 13 releases since 2024-04-16) |
| `wkg` | `0.15.0` (2026-02-06) — package manager (`bytecodealliance/wasm-pkg-tools`) |
| `jco` | `1.19.0` (2026-04-22) — Component Model in JS / browser transpile |
| `componentize-js` | `0.20.0` (2026-04-14) — bundles SpiderMonkey/QuickJS into a component |
| `componentize-py` | `0.23.0` (2026-04-15) — bundles CPython into a component |
| `preview2-shim` (browser) | `0.17.9` (2026-04-17) |
| TinyGo | `0.41.1` (2026-04-22) — `tinygo build -target=wasip2` |
| Wasmtime stars | 17,977 |
| Largest production | Fastly Compute (Wasmtime + CM); also Cosmonic / wasmCloud, Fermyon Cloud, Shopify Functions |
| License | Apache-2.0 with LLVM exception across BA reference impls |

(All version numbers and dates verified via `gh api`, `crates.io`, `npmjs.org` on 2026-05-09. Wasmer is MIT-licensed, not Apache-2.0; BA stewardship does not bind non-BA runtimes.)

## Contents

15 files, ~2,650 lines. Each file independently skimmable.

**Spec layer**
- [**Spec**](spec.md) — what the Component Model is. WIT syntax (interfaces, worlds, packages, types, resources), four-pass compile model (WIT → bindings → core wasm → component), preview2 interface set.
- [**ABI**](abi.md) — Canonical ABI: lifting/lowering, type lowering rules, string encodings, variant tag encoding, list representation, resource handles (own/borrow), `realloc` and `post-return`, components-vs-core-modules.

**Runtime**
- [**Wasmtime**](wasmtime.md) — embedder API, `Linker`/`Component`/`Instance`/`Func`, security model (instance isolation, no shared memory), determinism levers (NaN canonicalization, no threads), fuel-vs-epoch metering, `Module::serialize` / `Engine::precompile_component`, async support, **explicit absence of live-instance heap snapshot** (load-bearing for Myrhiza vs Agoric's `xsnap`).
- [**Preview status**](preview-status.md) — WASI preview1 → preview2 → preview3 (RC) lineage, the Wasmtime↔WASI version matrix, the preview3-async story still in flight.

**Authoring surface**
- [**Tooling**](tooling.md) — `wasm-tools`, `wit-bindgen`, `cargo-component`, `wac-cli`, `wkg`, `jco`, `componentize-js`, `componentize-py`. The OCI-as-WASM-registry convention.
- [**Languages**](languages.md) — Rust (de-facto reference), Go (TinyGo), JavaScript (componentize-js), Python (componentize-py), C/C++, Java (TeaVM), MoonBit, Swift (none), Zig (limited). Per-language gaps, resource-type ergonomics, async story.
- [**Browser**](browser.md) — `jco transpile` is the only path; no native browser CM. componentize-js for the reverse direction. What's missing in browser context.

**Project lens**
- [**Governance**](governance.md) — BA Foundation, 23-org member roster, TSC composition, the CG/WG/BA tripartite relationship, single-vendor-vs-consortium tradeoffs.
- [**History**](history.md) — 2017 WASM 1.0 W3C Rec → 2019-11-12 BA founded → 2022-09-20 Wasmtime 1.0 → 2024-04-24 WASI 0.2.0 → 2025-09 preview3 RCs begin → 2026-05 current.
- [**Ecosystem**](ecosystem.md) — Wasmer (MIT-licensed competitor), WasmEdge (CNCF, still pre-1.0 at 0.16.3), WAMR (BA, embedded), wazero (Go-native, no CM), Lucet (archived). Cross-table of CM/WASI support per runtime.
- [**Critiques**](critiques.md) — third-party + insider critiques with verbatim quotes: preview3 slippage (milestone open since 2023-08-22 with no `due_on`), 35MB hello-world componentize-py overhead (`componentize-py#98`), Wasmtime CI breakage on GHCR-as-WIT-registry (`WASI#886`), p3 template breakage (`spinframework/spin#3485`), leptos-wasi p3 venting (`leptos_wasi#18`).
- [**Open problems**](open-problems.md) — 12 structurally unresolved questions with Myrhiza disposition: async stabilization, distributed identity, capability declaration vs runtime resolution, determinism guarantees, resource lifetimes across components, preview3 surface area, composition at scale, semver-downgrade typecheck, GC/threads/memory64 per-language, browser CM, reentrance/callbacks, observability primitives.

**Reference**
- [**Lessons for Myrhiza**](lessons.md) — validates / avoid / borrow — **the consult-this-when-designing file.**
- [**Glossary**](glossary.md) — WIT, world, interface, package, resource, lift/lower, canonical ABI, `Linker`, fuel, epoch, preview1/2/3, jco, componentize-js, etc.

## Recommended reading order

For a Myrhiza spec author working on **the kernel-import surface** (capabilities, host imports, ABI): start with [**lessons.md**](lessons.md), then [**abi.md**](abi.md) and [**wasmtime.md**](wasmtime.md), then [**spec.md**](spec.md) for the type system. The kernel's "host import = capability" model is mechanically the WIT-imports model; the question is which interfaces we expose and how we mediate them.

For a spec author working on **bundle distribution** (component hashing, registry, install UX): [**lessons.md**](lessons.md), then [**tooling.md**](tooling.md) (sections on `wkg` + OCI registry convention), then [`../agoric-endo/modules-and-bundling.md`](../agoric-endo/modules-and-bundling.md) for the cross-substrate comparison.

For a spec author working on **state-apply determinism**: [**wasmtime.md**](wasmtime.md) (determinism levers + the snapshot-absence section), then [**open-problems.md**](open-problems.md) §4 (determinism), then [`../agoric-endo/determinism.md`](../agoric-endo/determinism.md) and [`../holochain/`](../holochain/) for prior-art patterns. CM gives Myrhiza no determinism guarantee; we have to constrain it.

For a spec author working on **browser viability**: [**browser.md**](browser.md), then [**preview-status.md**](preview-status.md) for what's missing in the browser context, then [**open-problems.md**](open-problems.md) §10 (browser CM).

For anyone evaluating "should we wait for preview3 vs commit to preview2": [**preview-status.md**](preview-status.md), [**critiques.md**](critiques.md) §1 (async slippage), [**open-problems.md**](open-problems.md) §1 (async stabilization).

## How to use this prior-art doc

This corpus is reference for future Myrhiza spec writing. Pin numbers and dates accurate as of the **Date:** in this README; bump the date when meaningful churn happens upstream (new Wasmtime major LTS, new WASI 0.2.x, preview3 final ships).

**Framing disclosure.** These docs are written from a Component-Model-as-foundation, P2P, capability-mediated-host-imports stance. Most "Implications for Myrhiza" sub-sections frame BA's choices through that lens. Future readers auditing whether *Component-Model-as-foundation* is itself the right primitive should weigh the corpus accordingly: it is a learn-from-CM-into-Myrhiza artifact, not a neutral catalog. The Spritely, Holochain, and Agoric folders carry the same disclosure for the same reason.

**Load-bearing-dependency disclosure.** The Component Model + Wasmtime are dependencies Myrhiza will hard-bake against — there is no alternative WASM-Component-Model runtime in the same league. This corpus has an incentive to soft-pedal problems Myrhiza will inherit (preview3 slippage, single-vendor stewardship via BA, lack of browser native CM, no determinism guarantee, cargo-component possibly stale). Readers should weight the [**critiques.md**](critiques.md) and [**open-problems.md**](open-problems.md) sections proportionally — they are the corrective. The Iroh folder carries the same load-bearing-dependency disclosure for the same reason.

**Not a tutorial.** Upstream documentation (`component-model.bytecodealliance.org`, `docs.wasmtime.dev`) is the correct source for hands-on use. This folder is the curated, version-pinned, Myrhiza-perspective synthesis those docs do not provide.

## Sources

- WebAssembly Component Model spec: https://github.com/WebAssembly/component-model
- Bytecode Alliance: https://bytecodealliance.org/
- Wasmtime: https://github.com/bytecodealliance/wasmtime
- WASI: https://github.com/WebAssembly/WASI
- jco: https://github.com/bytecodealliance/jco
- wasm-tools: https://github.com/bytecodealliance/wasm-tools
- wit-bindgen: https://github.com/bytecodealliance/wit-bindgen
- cargo-component: https://github.com/bytecodealliance/cargo-component
- wac: https://github.com/bytecodealliance/wac
- wasm-pkg-tools (`wkg`): https://github.com/bytecodealliance/wasm-pkg-tools
- componentize-js: https://github.com/bytecodealliance/ComponentizeJS
- componentize-py: https://github.com/bytecodealliance/componentize-py
