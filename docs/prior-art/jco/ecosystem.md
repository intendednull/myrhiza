**Date:** 2026-05-22
**Status:** active
**Subject:** Adjacent projects, who ships on jco today, the larger CM-JS toolbox.

## 1. Production users of jco / componentize-js / StarlingMonkey

The honest answer: **the production user list is small**. Where it exists, it's concentrated in two organizations.

| User | What they ship on it |
|---|---|
| **Fastly Compute@Edge (JS Compute platform)** | Customer-authored JS components run on StarlingMonkey at the edge. Fastly is StarlingMonkey's primary production deployment. |
| **Fermyon Spin (JS SDK)** | Spin (CNCF Sandbox, serverless CM runtime — see [`prior-art/spin/`](../spin/)) ships a JS SDK that uses componentize-js to produce CM components for Spin's runtime. |
| **Cosmonic** (now Akamai-stewarded post-2025-12-01) | wasmCloud-shaped deployments using jco for JS-side hosting. |
| **Akamai Functions** | Successor to Fermyon Cloud (Akamai acquired Fermyon 2025-12); Spin-on-Akamai uses jco-side tooling. |
| **Jco itself (self-hosting)** | The wasm-tools + js-component-bindgen Rust crates are compiled to CM components and shipped *inside* the jco npm package — jco transpiles itself for its own JS runtime. |

`StarlingMonkey/ADOPTERS.md` lists additional users; the jco repo does not maintain an ADOPTERS file (flagged in [`open-problems.md` §5](open-problems.md) as a community-signal gap).

There is **no large independent application** that visibly ships on jco end-to-end. The closest analogue is Fastly's JS Compute, but that's an internal platform product, not a third-party-authored shipping app.

## 2. The CM-JS-toolbox neighborhood

jco is the JS-language entry to the broader CM tooling. The other entry points by language:

| Language | Tool | Notes |
|---|---|---|
| **Rust** | `cargo component` | The native CM-shape compile; no embedded interpreter. Bytecode Alliance. |
| **JavaScript** | `componentize-js` (via jco) | Wizer-snapshotted StarlingMonkey. Bytecode Alliance. |
| **Python** | `componentize-py` | Wizer-snapshotted CPython. Bytecode Alliance. |
| **C# / .NET** | `componentize-dotnet` | NativeAOT-backed. Bytecode Alliance, BA blog 2024-09-03. |
| **Go** | TinyGo + WASI 0.2 backend; `wazero` (host runtime). | TinyGo's CM support landed 2024. Direct compile, no interpreter. |
| **Kotlin / Java** | (limited) | Blocked on Wasm-GC integration with the CABI — see [`prior-art/wasm-component-model/open-problems.md §9`](../wasm-component-model/open-problems.md). |
| **Zig** | Direct CM-shape emission via Zig 0.13+ | No tooling-wrapper needed; emits CM-shape wasm directly. |

For a Myrhiza-shaped polyglot story, the load-bearing combinations are:

- `cargo component` (Rust) → for `state-apply` / `state-propose`. ~100 KB–few-MB components.
- `componentize-js` → for `interaction` / `behavior`. ~8 MB.
- `componentize-py` → optional, similar profile to componentize-js (CPython is ~10 MB).

jco-the-CLI is the transpile entry; it takes any of these outputs and turns them into browser-runnable JS+wasm.

## 3. Adjacent runtimes (CM hosts other than jco)

jco is a *transpile + JS-shim* host. The native CM hosts are:

| Runtime | Language | Position |
|---|---|---|
| **Wasmtime** | Rust | The canonical CM host. Native, fast. Myrhiza's native peer profile uses this. See [`prior-art/wasm-component-model/wasmtime.md`](../wasm-component-model/wasmtime.md). |
| **WasmEdge** | C++ | Alternative CM host, CNCF Incubating. Partial CM support. |
| **wasmCloud host (`wash`)** | Rust | Wasmtime-based, CM-native. See [`prior-art/wasmcloud/`](../wasmcloud/). |
| **Spin** | Rust | Wasmtime-based, CM-native. See [`prior-art/spin/`](../spin/). |
| **Wasmer** | Rust | Limited CM support; MIT-licensed (not Apache like the rest). Out of BA-orbit. |
| **jco run / serve** | Node.js | jco-the-host, only on Node. Browsers go via transpile + custom host code. |

jco is unique in this list as a *non-native* CM host — it lifts/lowers in JS rather than in compiled native code. That's exactly why it's the browser path: browsers can run JS, can run core wasm, cannot run native code. jco bridges the two.

## 4. Bundlers + frameworks integrating jco

The "consume jco output in a build pipeline" story:

- **Vite** — no official plugin; the jco-emitted output is a normal ES module, so it works out-of-the-box if the bundler can resolve `.wasm` imports. WASM-loading plugins help.
- **Webpack** — same; `webpack@5` has built-in WASM support; transpiled output loads via dynamic `import()`.
- **Rollup / Rolldown** — same.
- **Next.js** — jco-transpiled output can be imported from a React component; bundling concerns are framework-side, not jco-side.

There is no "jco-friendly bundler" — the design is that the emitted output is plain ES modules + `.wasm` files that any bundler handles. This is a design strength: jco doesn't take a position on bundling.

## 5. Build-pipeline shape

For Myrhiza, a likely build pipeline:

```
component-source (Rust / JS / Python / Zig)
  ↓  (per-language compiler)
component.wasm  (CM, WASI 0.2 import shape)
  ↓  (jco transpile --instantiation async --import-bindings hybrid --no-wasi-shim)
out/component.js + out/component.core*.wasm
  ↓  (Myrhiza-side: wrap in Myrhiza host glue providing custom WASI overrides)
myrhiza-peer-component-bundle.js
  ↓  (browser bundler: vite/webpack/rolldown)
peer.js  (final browser bundle)
```

The Myrhiza-side host glue layer is where Myrhiza's spec lives — it provides the `filesystem` (OPFS), `sockets` (WebRTC), and any Myrhiza-specific imports the component needs (peer-id, replay-event-log, etc.). See [`runtime-shim.md` §4](runtime-shim.md).

## 6. Community size + signal

Hard numbers:

- **GitHub stars:** jco 953, ComponentizeJS 370, StarlingMonkey 270.
- **Open issues:** jco ~150 open / ~1400 closed (across releases). High closed-rate suggests active maintenance, not abandoned.
- **Recent commit cadence:** May 2026 alone shows 30+ commits to jco main, almost all from vados-cosmonic.
- **npm weekly downloads:** jco ~370K, componentize-js ~410K, preview2-shim ~2.57M.
- **Mailing list / forum:** Bytecode Alliance Zulip ([`bytecodealliance.zulipchat.com`](https://bytecodealliance.zulipchat.com)) has a `#jco` channel. Active.

The 2.57M weekly preview2-shim downloads suggest substantial deployed surface (every jco-transpiled artifact pulls preview2-shim transitively). That's the signal of real production use — orders of magnitude more than the jco-CLI download count, because the CLI is install-once-per-developer but the shim ships in every artifact.

## Sources

- StarlingMonkey ADOPTERS.md: <https://github.com/bytecodealliance/StarlingMonkey/blob/main/ADOPTERS.md>
- Spin JS SDK: <https://github.com/spinframework/spin-js-sdk>
- cargo-component: <https://github.com/bytecodealliance/cargo-component>
- componentize-py: <https://github.com/bytecodealliance/componentize-py>
- componentize-dotnet (BA blog): <https://bytecodealliance.org/articles>
- Bytecode Alliance Zulip: <https://bytecodealliance.zulipchat.com>
- npm download stats: <https://api.npmjs.org/downloads/point/last-week/@bytecodealliance/jco>
- Myrhiza cross-refs: [`prior-art/wasm-component-model/`](../wasm-component-model/), [`prior-art/spin/`](../spin/), [`prior-art/wasmcloud/`](../wasmcloud/)
