**Date:** 2026-05-22
**Status:** active
**Subject:** `componentize-js` — the JavaScript-to-component path. Pre-1.0, probably not on Myrhiza's critical path but shapes the jco repo.

## 1. What it does

[`@bytecodealliance/componentize-js`](https://github.com/bytecodealliance/ComponentizeJS) takes a JavaScript ES module + a WIT world definition and outputs a single `.wasm` component that implements that world. Inputs:

- `input.js` — JavaScript ES module with `export`s matching the WIT world's exports.
- `world.wit` — WIT file describing the component's imports + exports.
- Output: `component.wasm` — a single Component-Model `.wasm` ready to be run by Wasmtime, transpiled by jco, etc.

This is the **opposite direction** from [`jco transpile`](transpile.md). Don't confuse them. The naming is unfortunate; the brief for this folder explicitly flags conflation as an anti-pattern.

The CLI invocation is either standalone:

```
componentize-js -w world.wit input.js -o component.wasm
```

Or via jco (which `import()`s the componentize-js package dynamically if installed):

```
jco componentize input.js --wit world.wit -o component.wasm
```

## 2. How it works: SpiderMonkey + StarlingMonkey + Wizer

componentize-js does not *compile* JS to WASM. There is no JS-to-WASM compiler at the bottom of the stack — JS is too dynamic, and standardized AOT is not on any vendor's roadmap. Instead, it does this:

1. Take **StarlingMonkey**, a SpiderMonkey-based JS runtime that has been pre-built to WASM as a CM component. StarlingMonkey targets WASI 0.2.0 and provides web-platform builtins (fetch, Streams, URL, TextEncoder, etc.).
2. Use **Wizer** (pre-initialization tool) to load the user's `input.js` into a StarlingMonkey instance and run the engine to a "ready" snapshot point.
3. Bind the WIT world's exports to the JS module's exported functions inside that snapshot.
4. Emit the snapshotted state as a new `.wasm` component.

Each resulting component carries its own embedded StarlingMonkey: per the [ComponentizeJS README](https://github.com/bytecodealliance/ComponentizeJS), this adds **approximately 8 MB** to the component's wasm size. Per-component isolation is preserved (each instance has its own JS heap), at the cost of bundle size and cold-start.

A roadmap item ("share the engine across components") would slim this down but is not shipped as of componentize-js 0.21.0.

## 3. What JS features are supported

Per the README (verified 2026-05):

**Supported:** ES modules, `async` functions (with automatic promise resolution at the call boundary), Streams, URL/URLSearchParams, TextEncoder/Decoder, fetch, FormData, Crypto APIs, AbortController, compression, timers, `console`.

**Not supported / limited:**

- **Imported functions cannot be async.** This is the critical gotcha — the JS module can have `async export`s (componentize-js handles the promise unwrapping), but JS host imports must be sync. Going async on imports would require JSPI in the engine, and StarlingMonkey doesn't surface that.
- **Random number generation** becomes deterministic when the `random` WASI subsystem is disabled at build time. (Useful for Myrhiza-shape determinism; flagged in [`lessons.md`](lessons.md).)
- **Timer functions** will panic if `clocks` is disabled at build time. Same shape: build-time feature gates.

## 4. Stability + version honesty

**componentize-js has never shipped 1.0.** Current is `0.21.0` (2026-05-20). The BA's own [Jco 1.0 announcement](https://bytecodealliance.org/articles/jco-1.0) (2024-02-22) explicitly said:

> "ComponentizeJS is a JavaScript → Wasm Component toolchain with full support for WASI 0.2 built using the SpiderMonkey JavaScript engine. […] this project is newer and not yet considered stable"

That posture has not changed in two years of subsequent releases. Treat componentize-js as **evolving 0.x** through 2026, not as a stable build target.

The pace is real: 0.19.0 → 0.21.0 in 8 months (2025-09 to 2026-05), and the most recent two majors (0.20.0 in 2026-04, 0.21.0 in 2026-05) both shipped within weeks of their RCs. Breaking changes happen at minor boundaries.

## 5. Why this is (probably) not Myrhiza's path

Per [`prior-art/wasm-component-model/open-problems.md` §9](../wasm-component-model/open-problems.md), Myrhiza's `state-apply` v1 target language list is "Rust/C/Zig only" — chosen for tight binary size, no GC, no big runtime dependency. A componentize-js-produced component is ~8 MB before the user's code, all of which is StarlingMonkey/SpiderMonkey. That is **not** acceptable for a `state-apply` component, where the *whole component* should fit in the kilobyte-to-low-megabyte range and load fast enough to be re-run cheaply.

componentize-js *is* a reasonable target for:

- **`interaction` components** — UI surfaces, where 8 MB cold-start + ~50 ms init is fine because it happens once per user session, and the developer ergonomics of "write it in JavaScript" outweigh the tax.
- **`behavior` components** — bots and bridges, where the host gets to amortize cold start across many events and the developer convenience is high-value.

It is **not** a reasonable target for `state-apply` (every peer needs to run this; size matters) or `state-propose` (called on every transaction; latency matters).

The honesty axis: Myrhiza could promise "write your interaction component in JS" as a developer-facing feature without taking on componentize-js's size or stability story for the load-bearing layers. This is the right shape of the bet.

## 6. Other paths in the same direction

componentize-js is not the only "guest-language → component" tool. The CM ecosystem has:

- **`componentize-py`** — Python → component, also Wizer-based, similar shape (CPython embedded). Bytecode Alliance.
- **`componentize-dotnet`** — C# / .NET → component. NativeAOT-backed (no JS-engine-style embedding). BA blog 2024-09-03.
- **`cargo component`** — Rust → component. The "native" path. No embedded interpreter; the Rust compiler emits a CM-shaped wasm directly.
- **TinyGo** — Go → component. Direct compile, no embedded interpreter.

`cargo component` is the relevant comparison for state-apply: it emits 100 KB–few-MB components, no embedded runtime, the size you'd expect from a "real" compiler. The 8 MB tax of componentize-js is the price you pay for a dynamically-typed language with a polymorphic runtime; the alternative is a statically-typed compiler. Myrhiza picks the latter for the hot path.

## Sources

- ComponentizeJS repo: <https://github.com/bytecodealliance/ComponentizeJS>
- npm `@bytecodealliance/componentize-js`: <https://registry.npmjs.org/@bytecodealliance/componentize-js>
- StarlingMonkey repo: <https://github.com/bytecodealliance/StarlingMonkey>
- Wizer (pre-init tool): <https://github.com/bytecodealliance/wizer>
- BA announcement "Announcing Jco 1.0", 2024-02-22: <https://bytecodealliance.org/articles/jco-1.0>
- BA blog "Simplifying components for .NET/C# developers with componentize-dotnet", 2024-09-03: <https://bytecodealliance.org/articles>
- Myrhiza cross-refs: [`prior-art/wasm-component-model/open-problems.md §9`](../wasm-component-model/open-problems.md) (language-pluralism scope), [`lessons.md`](lessons.md)
