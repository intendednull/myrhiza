**Date:** 2026-05-22
**Status:** active
**Subject:** Browser viability — what works in browsers today, what doesn't. **Load-bearing for the Myrhiza browser-peer profile.**

## 1. The honest baseline

Per `preview2-shim` README (the runtime the jco-transpiled output depends on):

> "Browser is considered experimental, and not currently suitable for production applications. Node.js is fully tested and conformant against the Wasmtime test suite."

That is the load-bearing sentence for Myrhiza. The shipping browser path is **explicitly labelled experimental** by its maintainers as of 2026-05. The jco docs site echoes this:

> "Jco itself can be used in the browser, which provides the simpler Jco API […]" — but "browser support for WASI is currently experimental."

For Myrhiza, the spec implication is: **the browser-peer profile is a strict subset of capabilities of the native (Wasmtime) profile**, and the subset is constrained by what `preview2-shim` actually polyfills *correctly in browsers* today, not by what WASI 0.2 specifies. That gap is the load-bearing-for-Myrhiza fact.

## 2. What WASI subsystems work in browsers (per `preview2-shim`)

Each subsystem has two implementations inside `preview2-shim`: a Node.js one (battle-tested) and a browser one. The browser story is uneven.

| Subsystem | Node | Browser | Browser implementation notes |
|---|---|---|---|
| `wasi:cli/environment` | full | partial | `getEnvironment()` returns empty; `getArguments()` returns empty. No process model in browsers. |
| `wasi:cli/stdin`, `stdout`, `stderr` | full | partial | stdout/stderr → `console.log` / `console.error`. stdin → empty stream (browsers don't have terminals). |
| `wasi:cli/exit` | full | partial | calls `throw new Error('exit')`; no process to exit. |
| `wasi:clocks/wall-clock`, `monotonic-clock` | full | full | `Date.now()` + `performance.now()`. Monotonic resolution is browser-dependent (typically 5 µs–1 ms). |
| `wasi:filesystem` | full (node `fs`) | **partial / in-memory** | Browser impl is an in-memory filesystem (no persistence across reloads). Core read/write/seek work; advanced operations (appendViaStream, advise, etc.) are stubbed with `console.log`. Origin Private File System / IndexedDB-backed persistence is **not** wired up. |
| `wasi:http/incoming-request`, `outgoing-request` | full (node `http`) | partial | uses `fetch()`; CORS-bound; no streaming-body for `outgoing-request` in older browsers; no incoming-request without a service worker. |
| `wasi:io/streams`, `poll` | full | partial | Promise-based; some sync-style APIs awkward (poll-based code paths need restructuring). |
| `wasi:random/random`, `insecure` | full | full | `crypto.getRandomValues()`. |
| `wasi:sockets/tcp`, `udp` | full (node `net`/`dgram`) | **stub** | Browsers have no raw socket API. WebRTC and WebSocket are not the same API surface. Calls fail. |

**The two killers for Myrhiza are `filesystem` and `sockets`.** Sockets is a true stub (browsers have no raw socket API); filesystem is in-memory-only (data does not survive a page reload). A Myrhiza peer needs both real persistence and direct peer connectivity. Neither is provided by the default `preview2-shim` browser implementation in a Myrhiza-shape.

Both need *Myrhiza-side* engineering on top of jco:

- **Filesystem** → custom WASI override that backs to OPFS (Origin Private File System) or IndexedDB so data survives reloads. The in-memory default is not durable enough for a peer's event log.
- **Sockets** → custom WASI override that routes through WebRTC datachannels and/or relays through WebSocket-to-iroh-relay-server. Browsers do not have raw TCP/UDP — the API gap is real, not just unimplemented. This is the Myrhiza-iroh-browser-shim spec.

## 3. What the browser runtime needs

A transpiled jco artifact loaded in a browser requires:

- **ES2022** (top-level await, `Symbol`, BigInt, dynamic `import()`). All evergreen browsers since ~2022.
- **WebAssembly basic + multi-value + reference-types + bulk-memory + simd**. All evergreen browsers since 2022–2023.
- **`crypto.getRandomValues()`**. Universal.
- **`fetch()`**. Universal.
- **`structuredClone()`** for some preview2-shim paths. Universal since 2022.

Optional but useful:

- **JSPI (JavaScript Promise Integration)**. Required for `--async-mode jspi`. Status:
  - Chrome 137 — shipped stable.
  - Firefox 139 — shipped.
  - Safari — has not shipped (as of 2026-05). [WebKit's tracking bug status is unknown to this corpus; verify before relying on Safari.]
  - Node.js — V8-backed, so Chrome's support applies (gated on Node version that bundles V8 ≥ ~12.7).
- **Service Worker** (for `wasi:http/incoming-request` to be implementable).

If JSPI is required and Safari is a Myrhiza-supported browser, then the Myrhiza guest cannot use `--async-mode jspi`. The fallback is structuring the component without async imports — host-side async work happens *around* the component call, not inside it. This is the lower-friction choice for Myrhiza v1.

## 4. Bundle-size + cold-start honesty

No jco-published benchmarks exist (flagged in [`open-problems.md` §1](open-problems.md)). Practical observations from the Bytecode Alliance ecosystem:

- **A jco-transpiled component** = JS glue (~50–300 KB depending on world size) + core wasm files (varies; the component's own size).
- **A componentize-js-produced component** carries an embedded StarlingMonkey/SpiderMonkey (~8 MB). Then transpile *that* component for browser, and you ship ~8 MB of wasm + ~100 KB of JS glue per JS-authored component.
- **A cargo-component Rust component**, transpiled: the component's wasm (~100 KB–few MB) + jco JS glue.

For Myrhiza:
- **`state-apply` in Rust + jco-transpiled** → small, acceptable.
- **`state-apply` in JS + componentize-js + jco-transpiled** → ~8 MB. Not acceptable for state-apply (every peer runs this).
- **`interaction` in JS + componentize-js + jco-transpiled** → ~8 MB. Acceptable for UI surfaces (once per session, amortized).

Cold start: the jco-transpiled JS glue must `WebAssembly.instantiate()` each core module. Browser instantiate is faster than ever (single-pass tier-up since 2023) but still in the 10–100 ms range for multi-MB components. State-apply on a hot path needs to be already-instantiated and reused; jco's instantiation mode supports that pattern.

## 5. The two-profile spec implication

Per [`prior-art/wasm-component-model/open-problems.md` §10](../wasm-component-model/open-problems.md):

> "The Myrhiza-peer spec defines two implementation profiles — native (Wasmtime) and browser (jco-transpile + JS shim) — with the browser profile a strict subset of capabilities."

The strict-subset claim must be backed by something concrete. Drawing from this file's §2:

- Browser profile **excludes**: `wasi:filesystem` (default), `wasi:sockets` (default), `wasi:http/incoming-request` (without service-worker installation).
- Browser profile **requires Myrhiza-supplied substitutes** for: persistence (OPFS adapter), peer transport (WebRTC adapter), event-log (IndexedDB adapter).
- Browser profile **shares with native**: `clocks`, `random`, `cli/stdout` (logging), `io/streams`.
- Browser profile **does not get async host imports** (no JSPI in Safari, so the cross-browser baseline is sync imports). Async work happens at the host boundary, not inside guest calls.

This is the contract the Myrhiza browser-peer spec needs to lock in. Without it, "browser peer is just a shim build" becomes a load-bearing claim with no evidence.

## 6. The non-jco alternatives (and why none of them work yet)

For completeness — Myrhiza's bet on jco is not by default; there are other paths, and they were considered:

- **Native CM in browsers** — no browser vendor has committed. Per the [Deno tracking issue (denoland/deno#31314, 2025-11-16)](https://github.com/denoland/deno/issues/31314) and [Bun mirror (oven-sh/bun#24867)](https://github.com/oven-sh/bun/issues/24867), neither Deno nor Bun support CM directly either. The path doesn't exist as of 2026-05.
- **Wasmer's JS shim** — Wasmer (MIT-licensed, separate from BA) historically had a `wasm-pack`-shaped story for non-CM wasm, but not for CM. No production CM-in-browser shim from Wasmer.
- **wasmer-js / wasm-bindgen** — these are core-wasm-only, not CM-aware. You'd lose the component-model interface types.
- **Rebuild Myrhiza guest code for browser as JS directly** — defeats the whole point of CM, would mean two source-trees-per-component.

jco is the only viable path. That's the load-bearing fact. The corpus's job is not to celebrate it but to pin honestly what works, what doesn't, and what Myrhiza has to do itself.

## Sources

- `@bytecodealliance/preview2-shim` README: <https://github.com/bytecodealliance/jco/blob/main/packages/preview2-shim/README.md>
- jco transpiling docs: <https://bytecodealliance.github.io/jco/transpiling.html>
- jco repo `docs/src/manual-wasm-instantiation-with-wasi-overrides.md`: <https://github.com/bytecodealliance/jco/blob/main/docs/src/manual-wasm-instantiation-with-wasi-overrides.md>
- JSPI tracking (V8 blog): <https://v8.dev/blog/jspi>
- JSPI proposal: <https://github.com/WebAssembly/js-promise-integration>
- Deno CM tracking: <https://github.com/denoland/deno/issues/31314>
- Bun CM mirror: <https://github.com/oven-sh/bun/issues/24867>
- Myrhiza cross-refs: [`prior-art/wasm-component-model/open-problems.md §10`](../wasm-component-model/open-problems.md), [`runtime-shim.md`](runtime-shim.md), [`transpile.md`](transpile.md), [`open-problems.md`](open-problems.md)
