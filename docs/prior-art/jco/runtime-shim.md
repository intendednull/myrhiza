**Date:** 2026-05-22
**Status:** active
**Subject:** `@bytecodealliance/preview2-shim` — the JS-side WASI 0.2 implementation that ships *inside* every jco-transpiled artifact.

## 1. What it is

`@bytecodealliance/preview2-shim` is an npm package providing JavaScript implementations of every WASI 0.2 (preview2) subsystem. When `jco transpile` emits its JS glue, by default WASI imports are wired to functions in this shim — turning a component that thinks it's calling `wasi:filesystem/types#read` into a call into `preview2-shim`'s `filesystem.read()` JS function.

This is the **runtime** side of jco. The CLI (transpile / componentize) is invoked at *build* time; preview2-shim ships *to the user* alongside the transpiled output.

Weekly npm downloads ~2.57M (mostly transitive via jco-transpiled apps), more than jco itself (~370K weekly). This is the larger surface in deployed code.

## 2. Two implementations per subsystem

Each WASI subsystem in `preview2-shim` ships **two** implementations:

- **Node.js** path — uses node `fs`, `http`, `net`, `dgram`, `process`, etc. Per the README, "fully tested and conformant against the Wasmtime test suite."
- **Browser** path — uses Web APIs (fetch, Crypto, performance.now). Per the README, "considered experimental, and not currently suitable for production applications."

Selection is environment-detected at module-load (typeof `window` / `process`). You can also explicitly import the platform-specific entry point.

## 3. Subsystem-by-subsystem coverage

(Drawn from `preview2-shim/src/`; verified against the version pin `1.17.9` shipped with jco 1.19.0.)

### `wasi:cli`

- `environment` — Node: reads `process.env`. Browser: returns empty.
- `stdin` / `stdout` / `stderr` — Node: pipes through `process.stdin`/`stdout`/`stderr`. Browser: `console.log` / `console.error`; stdin returns empty.
- `exit` — Node: `process.exit(n)`. Browser: throws.

### `wasi:clocks`

- `wall-clock` — both: `Date.now()` → nanoseconds.
- `monotonic-clock` — both: `performance.now()` → nanoseconds. Resolution browser-dependent (typically 5 µs after Spectre mitigations).
- `timezone` — Node: `Intl.DateTimeFormat().resolvedOptions().timeZone`. Browser: same.

### `wasi:filesystem`

- Node: full node `fs`-backed implementation. Descriptor objects, errno mapping, sync + async paths.
- **Browser: in-memory only.** Core read/write/seek work (against an in-memory tree); advanced ops (`appendViaStream`, `advise`, certain metadata calls) log "unimplemented" via `console.log` rather than throwing. **Data does not persist across page reloads** — there's no OPFS / IndexedDB backing. For a Myrhiza-shape peer's event log, this is functionally a gap.

### `wasi:http`

- Node: backed by `node:http` / `undici`. Supports `outgoing-request` directly; `incoming-request` requires the host to construct it (jco-served apps wire this up).
- Browser: `outgoing-request` → `fetch()`. CORS-bound. Streaming-body for request bodies depends on the browser's `Request` body-init support (now universal). `incoming-request` requires a service worker; not provided.

### `wasi:io`

- `streams` — both: implements `input-stream` and `output-stream`. Browser uses WHATWG Streams; Node uses node streams.
- `poll` — both: promise-based. The sync poll-style code in some guest libraries needs restructuring.
- `error` — both: opaque error resource.

### `wasi:random`

- `random` — both: `crypto.getRandomValues()`. (Browser path via WebCrypto, Node via node `crypto`.)
- `insecure-seed` — both: similar.
- `insecure` — both: similar, with documented non-crypto-strength label.

### `wasi:sockets`

- Node: full `tcp` (`net.Socket`) + `udp` (`dgram`) implementation.
- **Browser: stub.** Browsers have no raw socket API. WebSocket and WebRTC are alternative APIs but they do not map onto `wasi:sockets`. **Second load-bearing-Myrhiza-side gap.**

## 4. How a custom host overrides shims

(See [`transpile.md` §4](transpile.md) for the transpile-time view; this is the runtime-time view.)

`preview2-shim` exposes a `WASIShim` class (instantiation-mode transpile output is required):

```javascript
import { WASIShim } from '@bytecodealliance/preview2-shim';
import { instantiate } from './out/component.js';

const customShim = new WASIShim({
  filesystem: myMyrhizaOPFSImpl,      // custom impl
  sockets: myMyrhizaWebRTCImpl,        // custom impl
  // clocks, random, cli omitted → uses defaults
});

const component = await instantiate(getCompileCore, customShim.getImportObject());
```

The shim's `getImportObject()` returns the imports object the transpiled `instantiate()` expects, with overrides for the keys you supplied and defaults for the keys you didn't. Selective subsystem replacement is the intended use.

For Myrhiza, this is the hook for the browser-peer profile:

- `filesystem` → OPFS / IndexedDB-backed implementation.
- `sockets` → WebRTC / WebSocket-to-iroh-relay implementation.
- `clocks` → potentially a determinism-controlled clock for state-apply replay.
- `random` → potentially a deterministic seeded RNG for state-apply.

The last two ("potentially deterministic") are not jco's concern — they're Myrhiza's design choice. jco's shim *does not* default to deterministic; the host must explicitly substitute.

## 5. Versioning relationship

`preview2-shim` is published from the jco repo's monorepo (under `packages/preview2-shim/`). It is versioned **independently** of the jco CLI:

- jco CLI version `1.19.0`
- preview2-shim shipped with it: `^0.17.9` (pinned via jco's runtime dep)
- Recent preview2-shim versions: `0.17.9` (2026-04), `0.17.x` released roughly biweekly through 2026.

A transpiled artifact carries a `package.json` dependency on `@bytecodealliance/preview2-shim@^X.Y.Z` matching what the CLI was at when transpile ran. Upgrading the shim independently of the CLI is supported (semver-compatible only); cross-major upgrades need re-transpile.

This is a real operational concern for Myrhiza: if Myrhiza ships browser-peer bundles, the embedded `preview2-shim` version is a Myrhiza-build-time fact. Pinning it explicitly in the build pipeline (rather than `^`) avoids drift.

## 6. Preview3 status

Per [`prior-art/wasm-component-model/open-problems.md`](../wasm-component-model/open-problems.md), WASI preview3 entered RC state in late 2025–early 2026. jco's preview3 story:

- The jco repo contains `packages/preview3-shim/` as a parallel package (status: in-development).
- A separate experimental `jco-std-v0.2.0-rc.0` (2026-05-21) is exploring std-library-shaped semantics atop the shim layer.
- BA blog (2026-03-19) states: "P3 support is actively under development."

There is no shipping preview3-shim 1.0 as of 2026-05. **Treat preview3 in jco as RC, not as a build target.** Myrhiza's browser-peer profile pins preview2; revisit when preview3 reaches stable parity with preview2's "fully tested" status in Node and shipping shape in browser.

## Sources

- `@bytecodealliance/preview2-shim` README: <https://github.com/bytecodealliance/jco/blob/main/packages/preview2-shim/README.md>
- npm `@bytecodealliance/preview2-shim`: <https://registry.npmjs.org/@bytecodealliance/preview2-shim>
- jco repo `packages/preview2-shim/`: <https://github.com/bytecodealliance/jco/tree/main/packages/preview2-shim>
- jco repo `packages/preview3-shim/` (in-development): <https://github.com/bytecodealliance/jco/tree/main/packages/preview3-shim>
- jco docs site, manual instantiation: <https://bytecodealliance.github.io/jco/manual-wasm-instantiation-with-wasi-overrides.html>
- Myrhiza cross-refs: [`prior-art/wasm-component-model/open-problems.md`](../wasm-component-model/open-problems.md), [`transpile.md`](transpile.md), [`browser-viability.md`](browser-viability.md)
