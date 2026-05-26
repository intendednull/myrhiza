**Date:** 2026-05-22
**Status:** active
**Subject:** jco-specific terms. Cross-reference [`prior-art/wasm-component-model/glossary.md`](../wasm-component-model/glossary.md) for CM-level terms.

---

**Bytecode Alliance (BA)** — 501(c)(6) industry consortium hosting WASM-ecosystem projects: Wasmtime, Component Model spec, WASI standard, jco, ComponentizeJS, StarlingMonkey, cargo-component, wasm-tools. Founded 2019. Member companies include Fastly, Mozilla, Intel, Microsoft, Akamai (formerly Cosmonic + Fermyon).

**Canonical ABI (CABI)** — the binary calling convention defined by the Component Model spec for moving values across component boundaries. jco's binding generator (`js-component-bindgen`) emits JS code that lifts values from the CABI representation into JS values, and lowers JS values into the CABI. See [`prior-art/wasm-component-model/abi.md`](../wasm-component-model/abi.md).

**ComponentizeJS** — the BA project for converting JavaScript → WebAssembly Component, via Wizer-snapshotted StarlingMonkey. Repo `bytecodealliance/ComponentizeJS`; npm `@bytecodealliance/componentize-js`. Pre-1.0 (currently 0.21.0). See [`componentize-js.md`](componentize-js.md).

**Cosmonic** — company stewarding jco's primary maintainers (`@vados-cosmonic`, `tschneidereit`, `cfallin`). Founded 2021 by Liam Randall and Kevin Hoffman; primary corporate steward of wasmCloud; acquired by Akamai 2025-12-01. Post-acquisition, Cosmonic-employed jco maintainers are Akamai-employed via that path.

**cargo-component** — BA tool for compiling Rust to WebAssembly Components directly. The "no embedded runtime" path; produces small (100 KB–few MB) components. Not part of jco but adjacent in the CM ecosystem.

**jco** — JavaScript-native toolchain for WebAssembly Components. The CLI lives at `bytecodealliance/jco`; npm `@bytecodealliance/jco`. Five projects in one (per Bailey Hayes, BA blog 2026-03-19): CLI, transpiler (`js-component-bindgen`), componentize-js wrapper, wasm-tools wrapper, WASI shim (`preview2-shim`).

**JSPI (JavaScript Promise Integration)** — WebAssembly proposal allowing WASM calls to yield to JS Promises without CPS rewriting. W3C Wasm CG Phase 4 (effectively standardized). Shipped Chrome 137, Firefox 139. Not shipped in Safari as of 2026-05. jco uses JSPI for `--async-mode jspi` (EXPERIMENTAL).

**`js-component-bindgen`** — Rust crate inside the jco workspace that generates JS binding code from a WIT world. Compiled to a CM component (`crates/js-component-bindgen-component`) and run by jco itself. The "real" binding generator; jco-the-CLI is a Node.js wrapper. Versioned independently (e.g. 1.19.0 2026-05-18).

**`@bytecodealliance/preview2-shim`** — npm package shipping JavaScript implementations of every WASI 0.2 (preview2) subsystem. Loaded by transpiled artifacts at runtime. Node implementation is fully tested; browser implementation is experimental (filesystem is in-memory only, sockets is a stub). Currently 0.17.9 (2026-04). See [`runtime-shim.md`](runtime-shim.md).

**`@bytecodealliance/preview3-shim`** — in-development sibling for WASI 0.3 (preview3). Not yet released stable.

**SpiderMonkey** — Mozilla's JavaScript engine, also the engine inside Firefox. StarlingMonkey is a WASI-targeted build of SpiderMonkey.

**StarlingMonkey** — SpiderMonkey-based JavaScript runtime built as a WebAssembly Component, targeting WASI 0.2.0. Repo `bytecodealliance/StarlingMonkey`. Production users: Fastly JS Compute, Fermyon Spin JS SDK. Embedded inside componentize-js-produced components (~8 MB tax).

**transpile (jco-transpile)** — convert a CM `.wasm` into a JS+core-wasm bundle that runs in Node or browser. **The primary jco workflow.** Output is not a standalone wasm; it is JS that loads wasm internally. See [`transpile.md`](transpile.md).

**WASI (WebAssembly System Interface)** — standardized host-interface for WASM components. Subsystems include `cli`, `clocks`, `filesystem`, `http`, `io`, `random`, `sockets`. WASI 0.2 (preview2) is the current stable; 0.3 (preview3) is RC. See [`prior-art/wasm-component-model/preview-status.md`](../wasm-component-model/preview-status.md).

**WIT (WebAssembly Interface Type)** — the IDL the Component Model uses to define interfaces. jco emits TypeScript types directly from WIT.

**Wizer** — BA pre-initialization tool. Takes a WASM module + an entry function, runs the function to a snapshot point, emits a new WASM that starts from that snapshot. Used by componentize-js to pre-init StarlingMonkey with the user's JS already loaded. Repo `bytecodealliance/wizer`.

## Sources

- jco, ComponentizeJS, StarlingMonkey, Wizer repos in the BA org
- BA "Five ways of looking at Jco, Part 1" (2026-03-19): <https://bytecodealliance.org/articles/five-ways-of-looking-at-jco-part-1>
- Component Model glossary: [`prior-art/wasm-component-model/glossary.md`](../wasm-component-model/glossary.md)
