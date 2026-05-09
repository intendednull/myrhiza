**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — alternative-runtime landscape and what BA-stewarded substrate competes/coexists with

# Ecosystem & alternative runtimes

A reference for Myrhiza spec authors choosing among WASM runtimes. The Component Model is a Bytecode Alliance deliverable, but standalone WASM has multiple production runtimes — not all support CM, and the picture has churned. Current as of May 2026.

## The runtimes

### Wasmtime — BA reference, our default

- **Repo:** [bytecodealliance/wasmtime](https://github.com/bytecodealliance/wasmtime), Apache-2.0, 17,977 stars, 1,697 forks (May 2026).
- **First release:** v1.0.0 on **2022-09-20**. Currently shipping major versions roughly monthly (v44.0.0 on 2026-04-20).
- **Codegen:** Cranelift (BA project), with Winch (single-pass) and Pulley (interpreter) backends added since 2024.
- **Component Model:** **first-class.** Wasmtime is the reference implementation; CM features land here before they land elsewhere.
- **WASI:** Preview 1 stable, Preview 2 stable, Preview 3 RC (tracking the WASI repo).
- **Languages targeting it:** Rust (cargo-component), JavaScript (jco / StarlingMonkey), Go (TinyGo, full Go via wazero-shim or Go 1.21+ wasip1), Python, .NET, etc.
- **Production users:** Fastly Compute@Edge, Microsoft Hyperlight, Cosmonic wasmCloud, Fermyon Spin, Shopify Functions, Vercel WASM workers.

For Myrhiza this is the default choice. Rust-native (no FFI tax), full CM support, BA-stewarded so updates track the spec, monthly release cadence with long-term patch lines on older majors.

### Wasmer — the longest-running competitor

- **Repo:** [wasmerio/wasmer](https://github.com/wasmerio/wasmer), **MIT** (note: not Apache-2.0), 20,654 stars (May 2026).
- **First release:** repo created **2018-10-11**, predating the Bytecode Alliance.
- **Latest verified releases:** v7.1.0 on **2026-03-27**, with v7.2.0-alpha.2 on 2026-04-24.
- **Codegen:** Singlepass, Cranelift (yes, the BA project — Wasmer uses it as a backend), LLVM.
- **Component Model:** **partial / evolving.** Wasmer Edge and Wasmer's runtime support core WASM and WASI Preview 1 first-class; Component Model support has been claimed but is not at parity with Wasmtime as of May 2026. Treat CM-on-Wasmer as experimental.
- **WASI:** Preview 1 first-class. Preview 2 support trails Wasmtime.
- **Commercial:** Wasmer Edge (the commercial offering), `wasmer.io` package registry. The company is VC-backed (Series A 2021).
- **Notable history:** Wasmer is **not** a Bytecode Alliance member. The company has historically positioned itself as an independent alternative; from the BA's launch (2019-11-12) Wasmer's stance has been that the alliance over-coordinates around member-company priorities. There is no public formal "governance dispute" of the kind sometimes characterized — what exists is a long-standing strategic divergence: Wasmer ships its own package format (WAPM, now `wasmer.io` packages) instead of using OCI-component-registry conventions; it ships its own ABI extensions; it keeps tighter control over the runtime (single primary corporate steward) than the BA's foundation model.

For Myrhiza: Wasmer is a viable fallback if BA stewardship becomes problematic, but adopting Wasmer means accepting weaker CM support and a single-vendor governance model.

### WasmEdge — CNCF, edge/AI focus

- **Repo:** [WasmEdge/WasmEdge](https://github.com/WasmEdge/WasmEdge), Apache-2.0, 10,580 stars (May 2026).
- **Created:** 2019-11-29 (immediately post-BA-announcement, by Second State).
- **Latest verified releases:** v0.16.3 on **2026-05-08** (with v0.17.0-alpha.5 also on 2026-05-08). **Still pre-1.0** after six years of development — version numbers have been intentionally kept low while the project iterates.
- **Status:** **CNCF Sandbox** project (accepted 2021), focus on edge compute, plugin ecosystem, AI inference (TensorFlow, PyTorch, ggml/llama.cpp plugins).
- **Codegen:** AOT via LLVM, interpreter mode for fast startup.
- **Component Model:** limited; WasmEdge prioritizes plugin-native APIs over CM as the primary extensibility surface.
- **WASI:** Preview 1 stable. Preview 2 partial.
- **Notable:** the largest plugin ecosystem of any runtime — TLS, gRPC, image processing, AI, database connectors are all available out-of-the-box.

For Myrhiza: WasmEdge's plugin model is the wrong shape for capability-mediated host access (plugins are loaded in-process, breaking the kernel-mediated I/O model). Not a fit.

### WAMR — embedded/IoT

- **Repo:** [bytecodealliance/wasm-micro-runtime](https://github.com/bytecodealliance/wasm-micro-runtime), Apache-2.0, 5,916 stars.
- **Created:** 2019-05-02 (pre-dates the BA's public launch; transitioned in).
- **Latest verified release:** WAMR-2.4.4 on **2025-11-24**.
- **Status:** Bytecode Alliance hosted project. Focus: tiny-footprint runtimes for embedded, IoT, microcontrollers (Cortex-M, ESP32, RISC-V).
- **Footprint:** sub-100KB interpreter mode on ARM Cortex-M; AOT mode adds more. By comparison, Wasmtime's binary is multiple MB.
- **Component Model:** **limited.** WAMR's MicroRuntime profile prioritizes minimal core-WASM compatibility over full CM. Preview 2 support has landed in stages but is not at Wasmtime parity.

For Myrhiza: relevant if the runtime ever needs to fit on resource-constrained peers (IoT, small embedded). Not the choice for desktop/mobile/server peers.

### wazero — Go-native

- **Repo:** [tetratelabs/wazero](https://github.com/tetratelabs/wazero), Apache-2.0, 6,107 stars.
- **Created:** 2020-05-04 by Tetrate Labs.
- **Latest verified release:** v1.11.0 on **2025-12-19**.
- **Status:** Pure-Go runtime, **zero CGO dependencies** (the headline feature). Used by Tetrate, Mailgun, Tailscale (parts), and a long tail of Go programs that need to embed WASM.
- **Codegen:** custom interpreter and compiler, pure Go.
- **Component Model:** **experimental / partial.** wazero focuses on core WASM and WASI Preview 1; CM support has been added piecemeal but is not production-grade.
- **WASI:** Preview 1 stable, Preview 2 partial.

For Myrhiza: irrelevant unless we ever need to embed in a Go application. Our host is Rust-native.

### WAVM — academic/research

- **Repo:** WAVM/WAVM. Less actively maintained; recent commits sparse compared to Wasmtime/Wasmer.
- **Status:** academic-leaning, LLVM-based. WAVM was historically used for performance research; in production it has been displaced by Wasmtime and Wasmer.
- **Component Model:** none (development effectively paused before CM matured).

For Myrhiza: not a candidate.

### Lucet — deprecated

- **Repo:** [bytecodealliance/lucet](https://github.com/bytecodealliance/lucet), **archived**.
- **History:** Originally Fastly's standalone runtime. When Fastly joined the BA in 2019, Lucet was contributed; Fastly subsequently consolidated effort into Wasmtime. Lucet is archived; production deployments at Fastly migrated to Wasmtime.

For Myrhiza: not a candidate. Cited here for historical context.

### Browser-side: V8, SpiderMonkey, JavaScriptCore

These are not standalone runtimes for our purposes — they ship inside browsers and Node.js (V8). Each implements core WASM and WASI Preview 1 (via polyfills); Component Model support is via [jco](https://github.com/bytecodealliance/jco), which compiles components to JavaScript glue.

## Cross-table: feature support

| Runtime | Core WASM | WASI p1 | WASI p2 (CM) | WASI p3 (async) | License | Steward |
|---|---|---|---|---|---|---|
| Wasmtime | yes | yes | **yes** | RC tracking | Apache-2.0 | BA |
| Wasmer | yes | yes | partial | no | MIT | Wasmer Inc. |
| WasmEdge | yes | yes | partial | no | Apache-2.0 | CNCF (Second State) |
| WAMR | yes | yes | limited | no | Apache-2.0 | BA |
| wazero | yes | yes | experimental | no | Apache-2.0 | Tetrate Labs |
| V8 / Node | yes | via polyfill | via jco | no | BSD-style | Google |
| SpiderMonkey | yes | via polyfill | via jco | no | MPL-2.0 | Mozilla |

"yes" / "partial" / "limited" / "experimental" reflect the runtime's stated support level for CM, not just whether the bit-format parses. Wasmtime is the only runtime where CM is treated as a first-class feature with parity to core WASM.

## Production deployments

- **Fastly Compute@Edge** — Wasmtime, the largest commercial WASM-edge platform.
- **Cloudflare Workers** — historically V8 isolates rather than standalone WASM; CF has its own `workerd` runtime that is *not* CM-native.
- **Cosmonic / wasmCloud** — Wasmtime, fully CM-native, capability-pattern-aligned with Myrhiza's design.
- **Fermyon Cloud / Spin** — Wasmtime; Spin is the CLI/SDK layer over CM components.
- **Shopify Functions** — Wasmtime; Shopify-merchant scripts run as components.
- **Vercel** — WASM-on-Edge functions; Wasmtime-based.
- **Microsoft Hyperlight** — Wasmtime in a hardware-isolation wrapper for security-critical workloads.
- **DFINITY Internet Computer** — custom WASM runtime forked early; not CM-aligned, but DFINITY is a BA member tracking the spec.

## Observability and tooling per runtime

- **Wasmtime:** built-in profiler, perf-map output, gdb/lldb debugging via DWARF, structured tracing via `tracing` crate, Pulley interpreter for debugging codegen issues.
- **Wasmer:** native CLI metrics, `wasmer inspect` for binaries.
- **WasmEdge:** plugin-based observability (Prometheus, OpenTelemetry).
- **WAMR:** minimal — designed for resource-constrained environments where rich tooling isn't viable.
- **wazero:** native Go pprof integration.

For Myrhiza, Wasmtime's tracing/debugging story is the strongest. The host (kernel) integrates `tracing` and we get free observability of guest call boundaries.

## Why we pick Wasmtime

1. **Rust-native.** Embedding Wasmtime in a Rust host is a `cargo add`, not an FFI exercise. Crate-level type safety extends across the host boundary via wit-bindgen.
2. **Component Model first-class.** Myrhiza's component-profile architecture (`state-apply`, `state-propose`, `interaction`, `behavior`) maps cleanly onto CM components and worlds. Other runtimes' partial CM support would force us to maintain compatibility shims.
3. **BA stewardship.** No single-vendor governance risk; permissive licensing; multi-company contributors.
4. **Determinism configurability.** Wasmtime supports configurable determinism (NaN canonicalization, deterministic floats, fuel metering) at the host config level — this is **load-bearing** for our `state-apply` profile.
5. **Release cadence.** Monthly major releases with long-term patch support means we can pin, ship, and upgrade on a known schedule.

## What we lose by choosing Wasmtime

- **Footprint.** A Wasmtime-embedded binary is multi-MB; WAMR could deliver sub-MB footprints. If Myrhiza ever needs to run on microcontrollers, we'd need a separate WAMR-based code path.
- **Plugin ecosystem.** WasmEdge's bundled plugins (TLS, ML, gRPC) would be turn-key with that runtime; with Wasmtime we wire those capabilities ourselves.
- **Pure-Go hosts.** wazero is the only pure-Go runtime; if Myrhiza ever shipped a Go-native peer, we couldn't use Wasmtime there.

These are all acceptable tradeoffs for the runtime substrate. Myrhiza's host is and remains Rust.

## Cross-references

- [Governance & funding](governance.md) — BA structure
- [History](history.md) — release timeline, milestones
- [Holochain's WASM use](../holochain/) — uses Wasmer historically; relevant comparison point
- [Iroh as transport substrate](../iroh/) — Rust-native peer-discovery layer, complementary to Wasmtime
- Companion files: [`spec.md`](spec.md), [`abi.md`](abi.md), [`wasmtime.md`](wasmtime.md), [`tooling.md`](tooling.md), [`languages.md`](languages.md), [`browser.md`](browser.md), [`preview-status.md`](preview-status.md), [`critiques.md`](critiques.md), [`open-problems.md`](open-problems.md), [`lessons.md`](lessons.md)

## Sources

- https://github.com/bytecodealliance/wasmtime
- https://github.com/wasmerio/wasmer
- https://github.com/WasmEdge/WasmEdge
- https://github.com/bytecodealliance/wasm-micro-runtime
- https://github.com/tetratelabs/wazero
- https://github.com/bytecodealliance/lucet
- https://github.com/WAVM/WAVM
- https://wasmer.io/
- https://wasmedge.org/
- https://wasmcloud.com/
- https://www.fermyon.com/spin
- https://www.fastly.com/products/edge-compute
- https://hyperlight.dev/
