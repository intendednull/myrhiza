**Date:** 2026-05-22
**Status:** active
**Subject:** **The consult-this-when-designing decision file.** validates / avoid / borrow.

This is where the jco corpus's value lands. The other files are evidence; this is the synthesis the Myrhiza browser-peer spec consults.

## Validates

What jco's existence + state of the art **validates** about Myrhiza's design bets:

### V1. Component-Model-to-browser is technically viable today

jco-transpile + preview2-shim Node is conformance-tested against the Wasmtime test suite. The transpile pipeline exists, has shipped at 1.x, and has measurable production use (~2.57M weekly preview2-shim npm downloads). The high-level claim in [`prior-art/holochain/open-problems.md §8`](../holochain/open-problems.md) — *"ship the same components to a native iroh runtime and to a browser jco-compiled JS shim without re-architecting"* — is structurally true today. The CM is the only WASM-ecosystem story where this works.

### V2. The transpile-then-bundle path is the only viable browser strategy

Per [`browser-viability.md` §6](browser-viability.md): native CM in browsers has no vendor commitment, no shipping implementation, and no near-term roadmap. Deno's and Bun's CM-support issues remain open with no implementation timeline. Wasmer, Wasmtime, etc. are server-side. **There is no alternative.** Myrhiza's bet on jco is not a contingent choice; it's the only path.

### V3. Two-profile (native-Wasmtime, browser-shim) architecture is what the ecosystem expects

Per [`prior-art/wasm-component-model/open-problems.md §10`](../wasm-component-model/open-problems.md), the WASM Component Model's own documentation and the BA's own positioning treat browser as a separate transpile-derived target, not as a peer of native CM hosts. Myrhiza's two-profile spec aligns with what the ecosystem expects developers to do.

### V4. Capability-style WASI overrides are the host's intended hook

The `WASIShim({ filesystem: customImpl, sockets: customImpl, ... })` pattern is documented and supported in jco. Myrhiza's design intent — a host that provides custom capabilities at component instantiation, rather than letting the component touch raw WASI defaults — is what jco was designed for. The `--no-wasi-shim` + selective `--map` flags are the explicit knobs. This is well-trodden ground.

### V5. Same-component-to-Node-and-browser is the documented happy path

The `jco transpile` output supports both Node and browser environments out-of-the-box (per [`jco docs example.md`](https://github.com/bytecodealliance/jco/blob/main/docs/src/example.md)). Myrhiza's "ship once, run on iroh-native or browser-shim" story has a precedent that already works on the Node path, with the browser path needing Myrhiza-side WASI overrides for `filesystem` and `sockets`.

## Avoid

What to **avoid** from jco's design + state of the art:

### A1. Don't depend on componentize-js for the load-bearing layers

componentize-js is pre-1.0 (currently 0.21.0), explicitly labelled "not yet considered stable" by its own maintainers, and carries an 8 MB embedded StarlingMonkey per component. **For Myrhiza's state-apply / state-propose components, this is unacceptable.** Use Rust + cargo-component (per [`prior-art/wasm-component-model/open-problems.md §9`](../wasm-component-model/open-problems.md)). componentize-js is acceptable only for `interaction` (UI) and `behavior` (bot/bridge) components, where the 8 MB cost amortizes over a long-running session.

### A2. Don't rely on the browser `preview2-shim` for `filesystem` or `sockets`

`filesystem` in the browser is in-memory only (data does not survive reload); `sockets` is a stub (no browser API exists to back it). Calls to socket functions fail; calls to filesystem succeed but lose data on next page load. Myrhiza's browser-peer profile **must** ship its own implementations:

- `filesystem` → OPFS / IndexedDB-backed.
- `sockets` → WebRTC / WebSocket-to-iroh-relay.

This is Myrhiza-side engineering, not jco-side. Plan the spec accordingly.

### A3. Don't ship `--async-mode jspi` in v1

JSPI has not shipped in Safari. Until it does (or until Myrhiza explicitly drops Safari support), use sync-shaped host imports only. Structure components so async work happens *around* the component call, not inside it.

### A4. Don't pin to npm semver ranges (`^X.Y.Z`)

Pre-1.0 dependencies on the critical path (componentize-js, preview2-shim) ship behaviour changes at minor + patch boundaries. Pin exact versions in Myrhiza's build pipeline. Re-validate the build at each pin bump.

### A5. Don't conflate jco-the-CLI with jco-the-runtime-shim

These ship together but version independently. A jco-transpiled artifact carries a runtime dep on `@bytecodealliance/preview2-shim@^X.Y` matching what the CLI bundled at transpile time. Upgrading the shim independently of the CLI is supported (semver-compatible only); cross-major upgrades require re-transpile. Treat them as separate-version-pin facts.

### A6. Don't assume jco performance matches Wasmtime

No published benchmarks (per [`open-problems.md §1`](open-problems.md)). Likely orders-of-magnitude slower on hot host-call paths. Measure before committing the browser-peer profile to high-frequency state-apply replay. If hot-path performance matters, the right answer may be "browser peer is for interaction/observation; full state-apply requires native peer."

### A7. Don't assume preview3 is "almost here"

WASI preview3 is RC. jco's preview3-shim is in-development. Treat preview2 as the long-term commitment for Myrhiza v1; revisit in 12+ months.

### A8. Don't depend on a single maintainer's continued employment

Bus factor on jco is low (one dominant committer, employed by Cosmonic-now-Akamai). Myrhiza's build pipeline should be capable of using `js-component-bindgen` directly if jco-the-CLI lapsed maintenance. Structure the build to make the CLI a convenience layer, not a critical-path tool.

### A9. Don't take "experimental" labels lightly

When BA labels something `EXPERIMENTAL` (`--async-mode jspi`, browser support in preview2-shim, the preview3-shim), they mean it. Two years of project history shows the labels are sticky — the componentize-js "not yet stable" label is unchanged since 2024-02-22 despite continuous development. "Experimental" is not "almost ready."

## Borrow

What to **borrow** from jco's design + state of the art:

### B1. The two-mode emission (ESM vs instantiation)

For Myrhiza's own build pipeline, separate "static-shape emission for simple hosts" from "runtime-supplied-imports emission for capability-rich hosts." jco's `--instantiation [async|sync]` flag is the right shape; Myrhiza's profile-spec-of-emission should follow the same pattern.

### B2. The `--no-wasi-shim` + selective `--map` flags

The transpile-time choice "auto-shim the default WASI, OR don't and let the host do everything, OR selectively swap individual imports" is the right design. Myrhiza's build pipeline should expose the same three-way choice to component authors. (Specifically: Myrhiza's build defaults to `--no-wasi-shim` plus a Myrhiza-specific shim layer that provides Myrhiza-shape capabilities, not generic preview2-shim defaults.)

### B3. The `Symbol.for('cabiLower')` host-binding fast path

jco's `--import-bindings optimized` mode (per [`transpile.md §3`](transpile.md)) is the right pattern for high-frequency host-call hot paths: skip the JS-binding indirection by attaching a known symbol to the import function that the binding code can detect and call directly. Myrhiza's hot-path host imports (signature-verify, hash, op-apply) should use this pattern.

### B4. The WIT-as-source-of-truth contract

jco emits TypeScript `.d.ts` types directly from WIT. The component's host-interface contract is the WIT file; the JS types are derived. Myrhiza should use the same pattern: the Myrhiza-host-interface WIT is the contract, and language-specific bindings (JS, Rust, Python) are derived. No hand-maintained binding files.

### B5. Self-hosting the binding generator

`js-component-bindgen` is written in Rust + compiled to a CM component + run by jco itself. The bootstrap is clean. **Myrhiza could do the same for its own binding generator** if it ever needs one: write it in Rust, emit it as a CM component, and use it from the Myrhiza build pipeline. The "tooling is itself a component" pattern is sound.

### B6. The Wizer + StarlingMonkey snapshot pattern (for JS guests in `interaction`)

Even though componentize-js itself is not load-bearing-stable, the *technique* — using Wizer to pre-initialize a runtime + your-script into a single snapshot-state wasm component — is generalizable. If Myrhiza ever wants its own "ship JS as a component" path (e.g. for `interaction` profile, where SpiderMonkey is overkill but a small JS subset would be useful), this is the proven pattern.

### B7. Independent versioning of CLI vs runtime shim

The jco repo ships multiple co-versioned-but-independent npm packages (`jco`, `componentize-js`, `preview2-shim`, `preview3-shim`). Each tagged separately, each released on its own cadence. **Myrhiza should adopt the same pattern** for its build pipeline: CLI is one artifact, runtime shim(s) are separate artifacts, transpiled bundles pin specific runtime-shim versions. Don't ship a monolith.

### B8. The transparent-fallback host-binding (`hybrid` mode)

`--import-bindings hybrid` checks for a fast-path symbol at runtime and falls back to JS bindings if absent. This lets hosts opt into the fast path per-import without forcing a global choice. Myrhiza's WASI-override layer can use the same pattern: cold-path imports use the JS-binding default, hot-path imports register a fast-path symbol that the binding adapter detects and uses.

## Cross-references

- Determinism / native-vs-browser-peer parity: [`prior-art/wasm-component-model/lessons.md`](../wasm-component-model/lessons.md), [`prior-art/holochain/open-problems.md §8`](../holochain/open-problems.md)
- Iroh as the native transport that the Myrhiza browser-peer profile shims for: [`prior-art/iroh/`](../iroh/)
- Spin / wasmCloud as adjacent CM runtimes (production-grade native-CM hosts): [`prior-art/spin/`](../spin/), [`prior-art/wasmcloud/`](../wasmcloud/)

## Sources

- This file is a synthesis; primary sources are cited in the per-subsystem files. See [`transpile.md`](transpile.md), [`componentize-js.md`](componentize-js.md), [`browser-viability.md`](browser-viability.md), [`runtime-shim.md`](runtime-shim.md), [`open-problems.md`](open-problems.md), [`governance.md`](governance.md).
