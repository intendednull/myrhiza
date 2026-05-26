**Date:** 2026-05-22
**Status:** active
**Subject:** Open problems — what jco structurally doesn't solve. Includes load-bearing-for-Myrhiza gaps.

## 1. No published performance characterisation

Two years after jco 1.0, there is no public end-to-end benchmark of:

- jco-transpiled component cold start (instantiate time).
- jco-transpiled component hot path (per-call overhead vs Wasmtime).
- Bundle size targets by component shape.
- Memory footprint of the JS glue + preview2-shim runtime.

The closest available numbers are scattered references to "8 MB component overhead from StarlingMonkey embedding" (from componentize-js README) and "fully tested against the Wasmtime test suite for Node" (from preview2-shim README — correctness only, not performance).

**Why this matters for Myrhiza.** A state-apply replay loop's per-event budget matters a lot. If jco's per-host-call overhead is e.g. 50 µs but Wasmtime's is 0.5 µs, a high-frequency state-apply replay (10K events) costs 500 ms on jco vs 5 ms on Wasmtime — two orders of magnitude. That informs whether the browser peer is "the same code, slower" or "a fundamentally different operational profile." Myrhiza has to measure this itself; jco won't tell you.

## 2. Browser support flagged "experimental" — load-bearing for Myrhiza

Quoted verbatim from `preview2-shim` README:

> "Browser is considered experimental, and not currently suitable for production applications."

The Node implementation is "fully tested and conformant against the Wasmtime test suite"; the browser one is not. Specifically (per [`browser-viability.md` §2](browser-viability.md)):

- `wasi:filesystem` — browser: **in-memory only**; no persistence across reloads. (Core read/write works; OPFS/IndexedDB not wired up.)
- `wasi:sockets` — browser: stub. **No backing implementation** (browsers have no raw socket API).
- `wasi:http/incoming-request` — browser: requires service worker; not provided.

For Myrhiza's browser-peer profile, this means Myrhiza must supply its own implementations for these subsystems. That's not a jco bug — it's a scope boundary jco has not committed to crossing. Myrhiza inherits the responsibility.

## 3. Pre-1.0 dependencies on the critical path

The component-producer side of the toolchain is pre-1.0:

| Package | Version | Status |
|---|---|---|
| `@bytecodealliance/componentize-js` | 0.21.0 (2026-05-20) | Pre-1.0; "not yet considered stable" per BA's own 2024-02-22 announcement, position unchanged in 2 years |
| `@bytecodealliance/preview2-shim` | 0.17.9 (2026-04) | Pre-1.0; semver patch+minor releases include behaviour changes |
| `@bytecodealliance/preview3-shim` | (in-development) | Not yet released stable |
| `StarlingMonkey` | 0.2.0 | Pre-1.0; SpiderMonkey version bumps drive churn |

A real production Myrhiza browser-peer build pipeline pins exact versions, not semver ranges. Cross-minor breakage is real on this stack.

## 4. Async story is JSPI-gated, Safari is the bottleneck

`--async-mode jspi` is the only async-imports path. JSPI status (verified 2026-05):

- Chrome 137: shipped.
- Firefox 139: shipped.
- **Safari: not shipped.**

If Myrhiza's browser-peer profile commits to Safari, async host imports are not available cross-browser. The fallback is structuring components without async imports — host-side async wrapping the component call. Workable, but a real constraint that propagates back into the Myrhiza state-apply API: imports for crypto, IO, etc. must be sync-shaped from the guest's perspective.

Even for browsers that have shipped JSPI, the jco docs label `--async-mode jspi` **EXPERIMENTAL** (verified jco 1.19.0 docs). Not production-grade yet on its own terms.

## 5. Documentation gaps

The official jco book (<https://bytecodealliance.github.io/jco/>) has a shallow chapter set: Introduction → Transpiling → Example → a few Advanced topics → Troubleshooting → Contributing. Conspicuously missing:

- **`componentize` chapter** — the JS-to-component flow has no dedicated doc page in the jco book. (componentize-js has its own README, but jco's book treats it as out-of-scope.)
- **`run` / `serve` deep-dive** — the CLI help is the only authoritative source.
- **Versioning / compatibility matrix** — no table of "jco X works with preview2-shim Y." Must be inferred from the monorepo + npm dependency declarations.
- **Performance / sizing guide** — see §1.
- **Migration guides** — between minor / major versions. None provided.
- **ADOPTERS.md** — jco repo does not maintain one (StarlingMonkey does).

**Why this matters for Myrhiza.** The Myrhiza-browser-peer spec needs source-of-truth references for: which jco/preview2-shim version-pair to pin, what the import contract is, what's stable vs experimental. The jco docs do not provide this; the spec author has to dig through release notes, source, and the CODEOWNERS chat.

## 6. The js-component-bindgen / jco-CLI split is fragile

Internally, jco-the-Rust-crate (`crates/js-component-bindgen` and `crates/js-component-bindgen-component`) is the "real" binding generator. jco-the-CLI is a Node.js wrapper that loads it as a WASM component and shells out.

For Myrhiza, this is **good news** (the underlying Rust crate is a clean separation point — Myrhiza could embed it directly) and **a warning** (the CLI surface is what's documented; the crate-level API is not stable across releases). If Myrhiza wants to embed js-component-bindgen directly (per [`governance.md` §3](governance.md) bus-factor mitigation), that's plausible but uncharted.

## 7. Preview3 transition is in-progress

WASI preview3 RCs have been published since 2025-09 (per [`prior-art/wasm-component-model/preview-status.md`](../wasm-component-model/preview-status.md)). jco's preview3-shim is in-development. BA stated in 2026-03 that "P3 support is actively under development." There is no published timeline for jco's preview3-stable release.

Myrhiza pinning preview2 today is the right call. Revisit when:

- preview3 reaches Phase 4 / stable in the CM spec.
- preview3-shim ships at parity with preview2-shim's "fully tested" status.
- The transition story for existing preview2 components is documented.

None of these is true as of 2026-05.

## 8. Reentrance / sync host callbacks into guest mid-call

Per [`prior-art/wasm-component-model/open-problems.md` §11](../wasm-component-model/open-problems.md), the Component Model itself has no reentrance — the host cannot call back into a guest mid-call. This is a CM-spec issue, not a jco issue, but it propagates: jco-transpiled output inherits the same constraint. Any Myrhiza pattern that wanted "the host invokes a guest callback while the guest is computing" (e.g. progress notifications, partial-result streaming) cannot be expressed.

The CM async proposal would address this. Status: pre-1.0 in the CM spec; jco's `--async-mode jspi` is the leading edge.

## 9. Single-engine embedding tax for JS guests

componentize-js components carry a full SpiderMonkey/StarlingMonkey embedding (~8 MB) per component. The roadmap item "share the engine across components" exists but is not shipped (per [`componentize-js.md` §2](componentize-js.md)). Until shipped:

- 10 JS-authored components = 10 × 8 MB = 80 MB of engine code.
- Cold start per component is non-trivial (~50 ms per Wizer-snapshot rehydration).

**Implication for Myrhiza.** The `interaction` profile can use componentize-js but the cost is real. A Myrhiza application with 5 interaction components ships 40+ MB of JS-runtime overhead per peer. The native-Wasmtime peer can amortize this across components running in one host process; the browser peer cannot share engines across browser tabs (each tab is its own JS world).

## 10. No formal correctness story

There is no formal verification of jco. The transpile pipeline is conformance-tested against the Wasmtime test suite (preview2-shim Node target only). The binding generator (`js-component-bindgen`) is hand-written Rust with unit tests.

That's standard for the CM ecosystem (per [`prior-art/wasm-component-model/critiques.md`](../wasm-component-model/critiques.md), the CM has no formal verification). Not a jco-specific gap, but worth noting: jco is not a hardened-correctness build target. A Myrhiza spec relying on jco for state-apply replay determinism should layer its own validation, not assume jco's lifting/lowering is byte-perfect under all input shapes.

## Sources

- `preview2-shim` README quote: <https://github.com/bytecodealliance/jco/blob/main/packages/preview2-shim/README.md>
- componentize-js README: <https://github.com/bytecodealliance/ComponentizeJS/blob/main/README.md>
- jco docs site: <https://bytecodealliance.github.io/jco/>
- BA "Announcing Jco 1.0" (2024-02-22): <https://bytecodealliance.org/articles/jco-1.0>
- BA "Five ways of looking at Jco, Part 1" (2026-03-19): <https://bytecodealliance.org/articles/five-ways-of-looking-at-jco-part-1>
- V8 blog on JSPI: <https://v8.dev/blog/jspi>
- Myrhiza cross-refs: [`prior-art/wasm-component-model/open-problems.md`](../wasm-component-model/open-problems.md), [`prior-art/wasm-component-model/preview-status.md`](../wasm-component-model/preview-status.md), [`browser-viability.md`](browser-viability.md), [`governance.md`](governance.md)
