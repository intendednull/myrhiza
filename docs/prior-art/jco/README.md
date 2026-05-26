**Date:** 2026-05-22
**Status:** active
**Subject:** jco — Bytecode Alliance JavaScript toolchain for the WebAssembly Component Model. The only viable browser path for Myrhiza components.

## What it is

**jco** is the Bytecode Alliance's JavaScript-native toolchain for the WebAssembly Component Model (CM). It is two things at once:

1. **`jco transpile`** — converts a CM `.wasm` (produced by any guest language: Rust, Go, Python, C, JavaScript) into a bundle of ES modules + core WASM files that runs in Node.js and in browsers, with WASI-0.2 imports auto-shimmed via `@bytecodealliance/preview2-shim`.
2. **`jco componentize`** (which dynamically loads `@bytecodealliance/componentize-js`) — takes a JavaScript ES module + a WIT world and produces a single CM `.wasm` by snapshotting SpiderMonkey (via StarlingMonkey + Wizer) with the JS pre-loaded inside.

These point in opposite directions: transpile is component → JS+WASM; componentize is JS → component. Both ship from the same git repo (`bytecodealliance/jco`) but as separate npm packages (`@bytecodealliance/jco`, `@bytecodealliance/componentize-js`). Bailey Hayes (Cosmonic) framed jco as "five projects in one" — CLI, transpiler, componentize-js wrapper, js-component-bindgen (Rust crate that emits the binding glue), and the preview2 / preview3 WASI shims.

## Key facts

| | |
|---|---|
| **Repo** | [`github.com/bytecodealliance/jco`](https://github.com/bytecodealliance/jco) |
| **License** | Apache-2.0 WITH LLVM-exception (verified `crates/jco/LICENSE`) |
| **Stars** | 953 (jco), 370 (ComponentizeJS), 270 (StarlingMonkey) |
| **Latest jco** | `@bytecodealliance/jco@1.19.0` (2026-04-22) |
| **Latest componentize-js** | `@bytecodealliance/componentize-js@0.21.0` (2026-05-20) — **pre-1.0** |
| **First npm publish** | `@bytecodealliance/jco@0.4.0` 2023-02-17 (author Guy Bedford) |
| **jco 1.0** | 2024-01-25 npm publish; 2024-02-22 announcement post |
| **Weekly npm downloads** | ~370K jco / ~410K componentize-js / ~2.57M preview2-shim |
| **Active maintainers (codeowners)** | `@vados-cosmonic` (Victor Adossi), `@andreiltd` (Andrei Stanciu) |
| **Original author** | Guy Bedford (first npm publish 2023-02-17; historical Fastly affiliation; remains an npm maintainer but no recent main-branch commits) |
| **Steward** | Bytecode Alliance; Cosmonic (acquired by Akamai 2025-12-01) is the primary employer of the active maintainer pair |
| **Workspace** | Rust workspace (edition 2024). Crates: `js-component-bindgen`, `js-component-bindgen-component`, `wasm-tools-component`, `jco`, `test-components`, `xtask`. |
| **CM target** | WASI 0.2 (preview2). Preview3 in-progress per March-2026 BA blog ("P3 support is actively under development"). |
| **Async** | `--async-mode jspi` — flagged **EXPERIMENTAL**. JSPI shipped in Chrome 137 / Firefox 139; Safari has not shipped. |

## Why Myrhiza cares

Per [`prior-art/wasm-component-model/open-problems.md` §10](../wasm-component-model/open-problems.md): the WASM Component Model has no browser-native runtime, and no browser vendor has committed to one. The only path for a CM component to run in a browser is **`jco transpile` + the `preview2-shim` runtime**. Per [`prior-art/holochain/open-problems.md` §8](../holochain/open-problems.md): "Myrhiza's bet on Component Model + jco lets you ship the same components to a native iroh runtime and to a browser jco-compiled JS shim *without re-architecting*."

That bet is load-bearing. This folder is the version-pinned, in-tree reading future Myrhiza spec authors will consult before writing the browser-peer profile spec.

## Reading order

1. **[`README.md`](README.md)** (this file) — orientation + key facts.
2. **[`transpile.md`](transpile.md)** — the `component → JS + WASM` path. This is the *primary* axis for Myrhiza.
3. **[`browser-viability.md`](browser-viability.md)** — what works in browsers today, what doesn't. **Load-bearing for the Myrhiza browser-peer spec.**
4. **[`componentize-js.md`](componentize-js.md)** — the `JS → component` path. Probably **not** on Myrhiza's critical path (Myrhiza guest language for state-apply is Rust/C/Zig per [`prior-art/wasm-component-model/open-problems.md` §9](../wasm-component-model/open-problems.md)), but documented because it shapes the jco repo's design.
5. **[`runtime-shim.md`](runtime-shim.md)** — the `preview2-shim` package, the JS-side host implementation, WASI subsystem coverage.
6. **[`cli.md`](cli.md)** — the full CLI surface (`transpile`, `componentize`, `run`, `serve`, `wit`, `parse`, etc.).
7. **[`ecosystem.md`](ecosystem.md)** — adjacent projects, who ships on jco today.
8. **[`governance.md`](governance.md)** — maintainer set, bus factor, release cadence, funding context.
9. **[`open-problems.md`](open-problems.md)** — what jco structurally doesn't solve.
10. **[`lessons.md`](lessons.md)** — **the consult-this-when-designing decision file.** validates / avoid / borrow.
11. **[`glossary.md`](glossary.md)** — terms.

## How to use this folder

When writing a Myrhiza spec or plan that touches the browser-peer profile, the build pipeline, or any "components ship to multiple environments" story:

- **Cite specific file + section** (e.g. `prior-art/jco/browser-viability.md §3`) rather than the folder.
- **Pin the version** — jco is pre-feature-freeze in async/preview3; claims about what jco "supports" must name the version (the current pin is `jco 1.19.0` / `componentize-js 0.21.0`, both 2026-04 to 2026-05).
- **Distinguish jco-the-CLI from jco-the-runtime-shim**. They co-version-released but a transpiled artifact has the *shim's* compatibility, not the CLI's.

**Framing disclosure.** These docs are written from a *Component-Model-as-Myrhiza-foundation + browser-peer-is-mandatory* stance. The "Implications for Myrhiza" sub-sections frame jco's choices through that lens. Additionally, because jco is a **load-bearing dependency** Myrhiza is committing to (not a competitor), the corpus has an incentive to soft-pedal its rough edges; treat unflattering callouts here as *more* trustworthy than flattering ones, and consult `open-problems.md` + `lessons.md` (avoid) for the honest picture. Future readers auditing whether browser-peer-via-jco is itself the right primitive should read the corpus accordingly: it's a learn-from-jco-into-Myrhiza-browser-profile artifact, not a neutral catalog.

## Sources

- jco repo: <https://github.com/bytecodealliance/jco>
- ComponentizeJS repo: <https://github.com/bytecodealliance/ComponentizeJS>
- StarlingMonkey repo: <https://github.com/bytecodealliance/StarlingMonkey>
- npm `@bytecodealliance/jco`: <https://registry.npmjs.org/@bytecodealliance/jco>
- npm `@bytecodealliance/componentize-js`: <https://registry.npmjs.org/@bytecodealliance/componentize-js>
- npm `@bytecodealliance/preview2-shim`: <https://registry.npmjs.org/@bytecodealliance/preview2-shim>
- BA announcement "Announcing Jco 1.0", Yoshua Wuyts, 2024-02-22: <https://bytecodealliance.org/articles/jco-1.0>
- BA blog "Five ways of looking at Jco, Part 1", Eric Gregory, 2026-03-19: <https://bytecodealliance.org/articles/five-ways-of-looking-at-jco-part-1>
- jco docs site: <https://bytecodealliance.github.io/jco/>
- Myrhiza cross-refs: [`prior-art/wasm-component-model/open-problems.md §10`](../wasm-component-model/open-problems.md), [`prior-art/holochain/open-problems.md §8`](../holochain/open-problems.md)
