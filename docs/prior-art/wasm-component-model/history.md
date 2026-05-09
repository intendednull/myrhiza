**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — chronology from MVP through WASI Preview 3

# History

A chronological reference for the Component Model substrate Myrhiza is adopting. Verified release dates come from `gh api` against upstream repos; narrative claims are sourced inline.

## Pre-CM era: 2017–2019

### 2017-03 — WebAssembly MVP

The WebAssembly Core Specification 1.0 reached **W3C Working Draft** in February 2018 and **W3C Recommendation 1.0** on **2019-12-05**. The MVP shipped four browser engines (V8, SpiderMonkey, JavaScriptCore, Chakra) by 2017. ([webassembly.org milestones](https://webassembly.org/), [W3C Rec 1.0 publication](https://www.w3.org/TR/wasm-core-1/))

Initial design: a single linear-memory, single-export, browser-targeted binary format. **No imports for I/O, no module composition, no IDL.** All host-interaction was hand-written `(import "env" "...")` glue.

### 2018-10 — Wasmer founded

The `wasmerio/wasmer` GitHub repo was created **2018-10-11T10:15:53Z**. Wasmer became the first widely-adopted standalone (non-browser) WASM runtime; its initial focus was on running WASM CLI tools, predating the BA.

### 2019-04 — WASI repo created

The `WebAssembly/WASI` repo was created **2019-04-02T18:23:05Z**, hosting what became `wasi_snapshot_preview1`. The first wasi-libc tagged release was **v0.1-alpha on 2019-03-25**.

### 2019-08-12 — Bytecode Alliance org created

The `bytecodealliance` GitHub org was created on this date, three months before the public announcement.

### 2019-11-12 — Bytecode Alliance announced

Mozilla, Fastly, Intel, and Red Hat publicly announced the alliance. Founding deliverables: Wasmtime (Mozilla's `wasmtime` crate transitioned to BA stewardship), Cranelift, Lucet (Fastly), and WAMR (Intel).

## Interface Types era: 2019–2022

### 2019–2020 — Interface Types proposal

The `WebAssembly/interface-types` repo emerged at the CG, attempting to define rich types (strings, lists, records) crossing the WASM boundary. It ran for two years, generated significant churn, and was eventually **superseded** by the Component Model approach in late 2021. The Interface Types repo is now archived as historical context for CM design decisions.

### 2020-12 — Wasmer 1.0

Wasmer's 1.0 milestone shipped late 2020, formalizing its standalone-runtime API. (The repo's earliest GH releases are sparse; the `dev` tag dates to 2022-09-28T20:31:51Z, with the production release-tagging cadence beginning at v1.x.)

### 2021-09-22 — Component Model repo created

`WebAssembly/component-model` was created on this date, launching the proposal that replaced Interface Types. The Component Model bundles together: a binary format for components, the WIT IDL, the Canonical ABI, and a typed import/export model.

### 2022-09-20 — Wasmtime 1.0

Wasmtime's `v1.0.0` release tag was published **2022-09-20T16:05:03Z**. This was the production-readiness signal for the runtime; Cranelift was simultaneously declared production-ready. Components were *not yet supported* at 1.0.

### 2022 — WASI Preview 1 stabilization

`wasi_snapshot_preview1` became the de-facto interface for CLI WASM throughout 2022. It used the **pre-component** module format with hand-rolled `(import "wasi_snapshot_preview1" "...")` declarations — what would later become a CM "world" was at this point a flat list of imports.

## Component Model era: 2023–2024

### 2023 — Preview 2 alpha cycle

The Component Model proposal moved from paper design to runnable code. Key milestones (verified via `WebAssembly/WASI/releases`):

- The first `WebAssembly/WASI` releases tracked Preview 2 alphas through 2023.
- Wasmtime began shipping component-model support behind a feature flag.

### 2024-04-24 — WASI 0.2.0 (Preview 2) released

Tag `v0.2.0` of WASI shipped on this date (verified via `gh api repos/WebAssembly/WASI/releases/tags/v0.2.0` — `published_at: 2024-04-24T22:07:06Z`) — the official Preview 2 milestone. From this point forward, a CM-based component targeting `wasip2` is the canonical way to build portable WASM modules outside the browser.

(Preview 1 remains supported for legacy code; Preview 2 is the forward-going substrate.)

### 2024 — Wasmtime component support stabilizes

Wasmtime's component-model support, which had been experimental since 2023, became stable through the 2024 release cycle. Major-version releases continued at the BA's roughly-monthly cadence; by Wasmtime v17–v20 (mid-2024), components were a first-class supported feature.

### Tooling milestones in 2024

- **jco** — JavaScript-side component tooling. Reached jco-v1.x by 2024 (latest verified: jco-v1.19.0 on 2026-04-22). Pairs with `js-component-bindgen` for binding generation.
- **cargo-component** — Rust-side. Latest verified release v0.21.1 on 2025-04-07. **Still pre-1.0.** The pre-1.0 status is intentional: cargo-component is tracking an evolving CM spec and reserves the right to break.

### 2024-2025 — WASI 0.2.x patch line

Verified releases on the Preview 2 line:

| Tag | Date |
|---|---|
| v0.2.5 | 2025-04-03 |
| v0.2.6 | 2025-06-12 |
| v0.2.7 | 2025-08-12 |
| v0.2.8 | 2025-10-07 |
| v0.2.9 | 2025-12-02 |
| v0.2.10 | 2026-02-03 |
| v0.2.11 | 2026-04-07 |

These are the stable evolution of `wasip2`; new world definitions (`wasi:cli`, `wasi:http`, `wasi:sockets`, `wasi:filesystem`) move forward on this line.

## Preview 3 era: 2025–2026

### 2026-01-06, 2026-02-09, 2026-03-15 — WASI 0.3 release candidates

Three Preview 3 release candidates have been published as of May 2026:

| Tag | Date |
|---|---|
| v0.3.0-rc-2026-01-06 | 2026-01-06 |
| v0.3.0-rc-2026-02-09 | 2026-02-09 |
| v0.3.0-rc-2026-03-15 | 2026-03-15 |

**Preview 3 is not yet final.** The headline feature is **native async** (the Component-Async working group's deliverable). RCs continue at roughly monthly cadence; based on the trajectory, a final 0.3.0 should land in 2026, but is not guaranteed.

### Wasmtime current state (May 2026)

The Wasmtime release cadence ships major versions roughly monthly. Recent verified releases:

| Tag | Date |
|---|---|
| v40.0.0 | 2025-12-22 |
| v41.0.0 | 2026-01-20 |
| v42.0.0 | 2026-02-24 |
| v43.0.0 | 2026-03-20 |
| v44.0.0 | 2026-04-20 |

Older majors are maintained on long-term-support patch lines (v36.x, v24.x are still receiving security patches in 2026 — see v36.0.9 on 2026-05-05 and v24.0.8 on 2026-04-30). For Myrhiza, this means we can pin to a Wasmtime major and expect ~12+ months of patch support.

## Spec-vs-implementation notes

The Component Model is governed by the W3C WebAssembly CG; its formal artifact is `WebAssembly/component-model`. As of May 2026:

- The Component Model is a **CG proposal**, not a W3C Recommendation.
- There is no separate W3C Recommendation track for the Component Model itself; stabilization happens via WASI Preview milestones.
- Active CG issues as of early May 2026 include open questions on: dtor function-type representation, waitable-set sync/async interaction, async-vs-sync ABI option restrictions, stream/future read-write interaction, and bounded lists. These are mostly Preview-3-era concerns, but indicate ongoing churn in the async surface.

## Major friction points historically

- **Interface Types vs Component Model (2019–2021).** Two years of design work on Interface Types were largely thrown out when the CM approach replaced it. The CM took the lessons but reset the design. Outside projects that bet early on Interface Types had to redo their work.
- **Canonical ABI redesigns.** The Canonical ABI — the binding from CM types to core WASM types — has been redesigned multiple times across Preview 2's evolution. A component built against an early Preview 2 RC will not run on a recent Wasmtime. This is the load-bearing risk for any project that ships precompiled components rather than rebuilding from source.
- **Async approach (2024–2026).** The async story for components has been the largest in-flight redesign. Three approaches have been considered: stream/future types in the CM directly (current direction), pollable-handle wrappers (Preview 2 hack), and structured-concurrency-only. Preview 3's RCs reflect the current direction; this could still shift before 0.3.0 final.
- **The "shim layer" debates.** Whether components should be able to call other components directly (no shim) versus always through a host-mediated boundary (shim) was debated extensively in 2023. Outcome: direct calls are allowed, but the host can opt to mediate; Myrhiza will run mediated for the determinism guarantees.

## Currency for Myrhiza

| Substrate piece | What we depend on | Stability |
|---|---|---|
| Core WASM 1.0 binary format | core module shape, SIMD opts | W3C Recommendation, stable |
| Core WASM 2.0 (Recommendation) | reference types, GC, multi-memory | W3C Recommendation, stable |
| Component Model binary format | component shape, type imports | CG proposal, **API-unstable** |
| WIT IDL | interface descriptions | CG proposal, syntax stabilizing |
| WASI Preview 2 (`wasip2`) | I/O surface | 0.2.x stable patch line |
| WASI Preview 3 (`wasip3`) | native async | RC, **not stable** |
| Wasmtime runtime API | host integration | semver per major; pin and upgrade deliberately |

For Myrhiza's own design, this means: build to `wasip2` initially, watch the Preview 3 RCs but do not depend on them, plan for one cross-version migration before the spec stabilizes.

## Cross-references

- [Governance & funding](governance.md)
- [Alternative runtime landscape](ecosystem.md)

## Sources

- https://github.com/WebAssembly/component-model
- https://github.com/WebAssembly/WASI
- https://github.com/WebAssembly/WASI/releases
- https://github.com/bytecodealliance/wasmtime
- https://github.com/bytecodealliance/wasmtime/releases
- https://github.com/bytecodealliance/jco
- https://github.com/bytecodealliance/cargo-component
- https://github.com/wasmerio/wasmer
- https://github.com/WebAssembly/wasi-libc/releases
- https://www.w3.org/TR/wasm-core-1/
- https://webassembly.org/
- https://hacks.mozilla.org/2019/11/announcing-the-bytecode-alliance/
