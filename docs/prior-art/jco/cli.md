**Date:** 2026-05-22
**Status:** active
**Subject:** The `jco` CLI — surface area, command summary, install footprint.

## 1. Install + invocation

```
npm install -g @bytecodealliance/jco
npm install -g @bytecodealliance/componentize-js  # required for `jco componentize`
```

componentize-js is **dynamically imported** by jco when `jco componentize` is invoked — it is *not* a hard dependency. jco's `package.json` has it as a peer/optional dependency. This is intentional: a user who only wants to *transpile* (not produce JS components) does not need the 8 MB StarlingMonkey embedding componentize-js carries.

Node 18.x quirk per the jco README: with componentize-js ≥ 0.18.3, `oxc-parser` may need manual install. Node 20+ avoids this.

The jco binary is a Node.js script (`#!/usr/bin/env node`); the Rust crates under `crates/` ship as WASM-compiled modules embedded in the npm package (`crates/js-component-bindgen-component`, `crates/wasm-tools-component`) — these are themselves CM components, run by jco against its own runtime. jco is self-hosting.

## 2. Command list

Verified against `jco --help` output of jco 1.19.0:

| Command | Purpose |
|---|---|
| `jco transpile <component.wasm> -o <dir>` | Convert CM `.wasm` → JS+core-wasm bundle. **The primary command.** See [`transpile.md`](transpile.md). |
| `jco componentize <input.js> --wit <world.wit> -o <component.wasm>` | (Delegates to componentize-js.) JS → CM `.wasm`. See [`componentize-js.md`](componentize-js.md). |
| `jco run <component.wasm> [args...]` | Execute a `wasi:cli/command` component via Node. Wasmtime-style CLI app runner. |
| `jco serve <component.wasm>` | Execute a `wasi:http/proxy` component as an HTTP server. Wasmtime `serve`-style. |
| `jco wit <component.wasm>` | Print the WIT interface(s) the component implements/imports. |
| `jco types <component.wasm>` | Emit standalone TypeScript `.d.ts` types for the component's WIT. |
| `jco print <component.wasm>` | Print the component as WAT. Wraps `wasm-tools print`. |
| `jco parse <component.wat>` | Parse WAT → wasm binary. Wraps `wasm-tools parse`. |
| `jco component new <core.wasm>` | Convert core wasm + WIT adapter → CM component. Wraps `wasm-tools component new`. |
| `jco component embed <core.wasm> --wit <world.wit>` | Embed WIT into a core wasm as a custom section. |
| `jco component wit <component.wasm>` | (Same as `jco wit`.) |
| `jco metadata-add` / `metadata-show` | Manipulate component metadata custom sections. |
| `jco opt <component.wasm>` | Run Binaryen `wasm-opt` on the component. |

A handful of these (`print`, `parse`, `component new`, etc.) wrap the upstream `wasm-tools` Rust crate, compiled to a CM component (`crates/wasm-tools-component`) and run inside jco's own runtime — i.e., jco uses jco to ship Rust-implemented wasm tooling to JS users. The five-projects-in-one framing applies.

## 3. The `transpile` flag matrix

The flag surface for transpile is the largest of any command. The Myrhiza-relevant ones, condensed:

| Flag | Purpose |
|---|---|
| `--name <name>` | Override the output module name. |
| `--instantiation [async\|sync]` | Emit an `instantiate()` function instead of static module shape. |
| `--import-bindings [js\|hybrid\|optimized\|direct-optimized]` | Choose the host-binding fast-path strategy. See [`transpile.md` §3](transpile.md). |
| `--map <wit-import>=<js-spec>` | Remap a WIT import to a custom module spec. |
| `--no-wasi-shim` | Don't auto-map `wasi:*` imports to preview2-shim; treat as user-supplied. |
| `--async-mode [jspi]` | EXPERIMENTAL. Use JSPI for async exports. JSPI gated on Chrome 137+ / Firefox 139+ / Safari (not shipped). |
| `--async-imports <list>` / `--async-exports <list>` | Selectively mark specific functions as async. |
| `--minify` | Run terser on emitted JS. |
| `--optimize` | Run Binaryen on emitted core wasm. |
| `--base64-cutoff <N>` | Inline core wasm < N bytes as base64 in JS. |
| `--tla-compat` | Avoid top-level await in emitted JS (for older runtimes). |
| `--no-namespaced-exports` | Flatten interface namespacing in emitted exports. |
| `--multi-memory` | Allow multi-memory core wasm in transpile output. |
| `--js` | Emit a `.js`-encoded version of the core wasm for environments without `WebAssembly`. (Niche.) |
| `--valid-lifting-optimization` | Skip some lifting-time validity checks (small perf win, requires trusted input). |
| `--tracing` | Emit trace events for every host-call boundary; debug aid. |

The exhaustive list is longer (`jco transpile --help` is ~50 flags); the above is the Myrhiza-curated set.

## 4. Run + serve

`jco run` and `jco serve` are notable for Myrhiza only as *examples* of how jco's own runtime hosts a transpiled component. They are not the path a Myrhiza browser peer takes — Myrhiza ships its own host code, not `jco run`. They demonstrate the API:

- `jco run` calls `wasi:cli/run.run()` on the component, with stdin/stdout wired to the terminal.
- `jco serve` calls `wasi:http/proxy.handle()` on the component for each incoming HTTP request, with Node's `http.Server` providing the listening socket.

Both run inside Node only. There is no `jco run` for browsers — the equivalent is "include the jco-transpiled output in your bundler and call `instantiate()` yourself."

## 5. Install footprint

(As of jco 1.19.0, 2026-04-22 npm tarball.)

- Total install size of `@bytecodealliance/jco`: ~25 MB unpacked (mostly the embedded wasm-tools / js-component-bindgen wasm components).
- componentize-js adds ~30–50 MB (the StarlingMonkey bundle).
- preview2-shim adds ~1 MB.

These are *build-time* costs only. The shipped artifacts (transpiled components) carry only their own size + preview2-shim runtime dep.

## Sources

- jco repo `crates/jco/src/cli.rs` (CLI definitions): <https://github.com/bytecodealliance/jco/tree/main/crates/jco>
- jco docs site (live): <https://bytecodealliance.github.io/jco/>
- jco repo `README.md`: <https://github.com/bytecodealliance/jco/blob/main/README.md>
- jco repo `docs/src/example.md`: <https://github.com/bytecodealliance/jco/blob/main/docs/src/example.md>
- npm `@bytecodealliance/jco`: <https://registry.npmjs.org/@bytecodealliance/jco>
