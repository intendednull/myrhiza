**Date:** 2026-05-22
**Status:** active
**Subject:** `jco transpile` — the component-to-JS+WASM path. The load-bearing-for-Myrhiza primary axis.

## 1. What it does

`jco transpile component.wasm -o out-dir` takes a single Component Model `.wasm` and emits a directory containing:

- **JavaScript ES module(s)** — the binding glue that lifts/lowers between the CM canonical ABI and host JS values.
- **One or more core `.wasm` files** — the un-canonicalised core modules extracted from the component, loaded by the JS glue.
- **`.d.ts` type definitions** — TypeScript types matching the WIT world.
- (Optionally, with `--js`) **a `.js`-encoded version of the core wasm** for environments that lack `WebAssembly` entirely.

The output is **not a standalone WASM file**. It is a JS-module-shaped bundle that requires a JS runtime (Node 18+ or a browser with the right surface). Anyone who says "jco produces WASM you can run anywhere" is wrong by one level — what jco produces is *JavaScript* that internally loads WASM. The component model has not crossed the canonical-ABI gap into native browser WASM yet; jco is the polyfill until it does.

## 2. Two output modes

| Mode | Flag | Shape | When to use |
|---|---|---|---|
| **ESM (default)** | (none) | `import { fn } from './out-dir/component.js'` — imports + exports are static module shape | Simple host with statically-known imports |
| **Instantiation** | `--instantiation [async\|sync]` | `out-dir/component.js` exports a single `instantiate(imports)` function | Host wants to supply imports at runtime (e.g. swap WASI subsystems per-instance) |

Instantiation mode is the relevant one for any "the host provides custom capabilities at instantiation time" pattern. This is the Myrhiza-shaped path: a Myrhiza browser peer can supply Myrhiza-specific imports (replay-event-log, deterministic-randomness-seed, peer-identity) instead of the default preview2-shim defaults.

The CLI also exposes `--instantiation async` vs `sync`. The async variant is needed when imports include `WebAssembly.compileStreaming` chains; sync needs imports pre-compiled. Most Myrhiza-realistic flows want async — the browser cannot synchronously fetch WASM.

## 3. Import bindings: four modes

`--import-bindings [mode]` controls the cost/flexibility tradeoff for **how** imported functions get called from the JS-side adapter:

| Mode | Behaviour | Cost |
|---|---|---|
| `js` (default) | High-level JS functions; binding code lifts/lowers types each call | Slowest — every call traverses the JS-to-canonical-ABI adapter |
| `hybrid` | Checks for `Symbol.for('cabiLower')` on each import; if present, uses the optimized fast path, else falls back to `js` | Modest overhead per call for the symbol check |
| `optimized` | Assumes `Symbol.for('cabiLower')` is present on every import; calls it directly | Fastest, but every host import must be pre-prepared with the cabiLower symbol |
| `direct-optimized` | Assumes imports are *already* core-wasm-shape functions (no lifting needed) | Fastest, requires host to do its own lowering |

**Myrhiza implication.** A Myrhiza browser peer that wires up high-frequency host imports (e.g. CRDT op-apply, signature verification) will want `optimized` or `direct-optimized` for the hot path and `js`/`hybrid` for the cold one. That choice is per-component, not global, so the build pipeline needs to be parameterised.

## 4. WASI auto-shimming

> "Components relying on WASI bindings will contain external WASI imports, which are automatically updated to the `@bytecodealliance/preview2-shim` package."
> — [jco transpiling docs](https://bytecodealliance.github.io/jco/transpiling.html)

Subsystems auto-mapped: `cli`, `clocks`, `filesystem`, `http`, `io`, `random`, `sockets`. The mapping is done at transpile time, baked into the emitted JS module's `import` statements. `--no-wasi-shim` disables this behaviour and emits WASI imports unmapped (the host then provides them).

For a Myrhiza-style host that wants to substitute its own implementations (e.g. a deterministic `clocks` impl, or a `sockets` impl that routes through WebRTC instead of TCP), the choice is:

1. Use the default auto-shim and override at instantiation via `WASIShim` config — see [`runtime-shim.md`](runtime-shim.md). Easier; limits which subsystems can be swapped to what `preview2-shim` lets you swap.
2. Use `--no-wasi-shim` and provide all WASI imports manually. Harder, more control, you write more glue, but you can ship a Myrhiza-shaped WASI implementation that does not look like Node's.

`--map` lets you remap individual imports to custom modules at transpile time (e.g. `--map wasi:http/types@0.2.0=./my-http.js`). Stepping-stone option between the two extremes.

## 5. Size / performance characteristics

Transpile produces *more* total bytes than the input `.wasm` plus a tax for the JS glue and the preview2-shim dependency. Concrete characteristics:

- **Inlining vs separate files.** `--base64-cutoff <N>` controls whether tiny core wasm files get inlined as base64 (cheaper fetch, larger JS) or kept separate (more fetches, smaller JS).
- **Minification.** `--minify` applies terser to the emitted JS.
- **Wasm optimization.** `--optimize` runs Binaryen on the extracted core wasm files. (Cumulative with whatever optimisation the component build pipeline did upstream.)
- **Top-level await compatibility.** `--tla-compat` rewrites the output to not use top-level await, for older runtimes that don't support it (most browsers shipped TLA in 2021–2022; rarely needed today).

The preview2-shim runtime cost is real and a key honesty axis for Myrhiza. No published "jco transpile X ms; Wasmtime runs X ns" benchmark exists in upstream docs as of 2026-05 — flagged in [`open-problems.md` §1](open-problems.md).

## 6. The async / JSPI experiment

> "`--async-mode [mode]`: EXPERIMENTAL: For the component imports and exports, functions and methods on resources can be specified as `async`. The only option is `jspi`."
> — [jco transpiling docs](https://bytecodealliance.github.io/jco/transpiling.html)

JSPI (JavaScript Promise Integration) is the underlying browser feature that lets a WASM call yield to a JS Promise without rewriting in CPS. Status as of 2026-05:

- **Phase 4 standardized** (W3C Wasm CG vote complete; "effectively standardized" per the V8 team's [`v8.dev/blog/jspi`](https://v8.dev/blog/jspi)).
- **Chrome 137:** shipped to stable.
- **Firefox 139:** shipped.
- **Safari:** has not shipped.
- **Node.js:** V8-based, so Chrome's support applies (gated on Node version that bundles V8 ≥ 12.7-ish).

Until Safari ships JSPI, any Myrhiza component that needs async imports (e.g. async signature verification, async storage IO) cannot run in Safari with `--async-mode jspi`. The fallback for that case is structuring the component without async imports — see [`browser-viability.md` §4](browser-viability.md).

The async-related flags are still tagged EXPERIMENTAL in the jco CLI help (verified jco 1.19.0 docs). Treat the entire async-import story as "shipping but not load-bearing yet."

## 7. The Myrhiza shape

The Myrhiza browser-peer build pipeline (per [`prior-art/wasm-component-model/open-problems.md` §10](../wasm-component-model/open-problems.md)) is approximately:

```
guest-language source
  ↓  (cargo component build / componentize-py / etc.)
component.wasm (CM canonical, WASI 0.2)
  ↓  (jco transpile --instantiation async --import-bindings hybrid --no-wasi-shim)
out/component.js + out/component.core.wasm
  ↓  (Myrhiza browser-peer bundler: webpack/vite/rolldown)
peer-bundle.js (loaded in browser)
```

The interesting Myrhiza-specific knobs are at the `jco transpile` step:

- `--no-wasi-shim` + custom imports = a Myrhiza-shaped host, not a generic WASI host.
- `--instantiation async` = run-time-supplied host capabilities; necessary for capability-per-instance.
- `--import-bindings hybrid` = hot-path optimized; cold-path readable.
- No `--async-mode jspi` for v1 = avoid the Safari-doesn't-ship-yet trap.

This is what the (not-yet-written) Myrhiza browser-peer spec needs to lock in.

## Sources

- jco transpiling docs (live): <https://bytecodealliance.github.io/jco/transpiling.html>
- jco repo `docs/src/transpiling.md`: <https://github.com/bytecodealliance/jco/blob/main/docs/src/transpiling.md>
- jco repo `docs/src/manual-wasm-instantiation-with-wasi-overrides.md`: <https://github.com/bytecodealliance/jco/blob/main/docs/src/manual-wasm-instantiation-with-wasi-overrides.md>
- BA blog "Five ways of looking at Jco, Part 1", 2026-03-19: <https://bytecodealliance.org/articles/five-ways-of-looking-at-jco-part-1>
- V8 blog on JSPI: <https://v8.dev/blog/jspi>
- Myrhiza cross-refs: [`prior-art/wasm-component-model/open-problems.md §10`](../wasm-component-model/open-problems.md), [`runtime-shim.md`](runtime-shim.md), [`browser-viability.md`](browser-viability.md)
