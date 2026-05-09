**Date:** 2026-05-09
**Status:** active
**Subject:** Spin compared to wasmCloud, Cloudflare Workers, AWS Lambda, Hyperlight, Wasmer Edge

# Comparisons

How Spin's design point relates to the rest of the WASM-and-FaaS landscape. Use this as the head-to-head reference; deeper architecture analysis lives in [`architecture.md`](./architecture.md), and the parallel write-up of wasmCloud is at [`../wasmcloud/comparisons.md`](../wasmcloud/comparisons.md).

## At-a-glance table

| Property | **Spin** | **wasmCloud** | **Cloudflare Workers** | **AWS Lambda** | **Hyperlight Wasm** | **Wasmer Edge** |
|---|---|---|---|---|---|---|
| Runtime / isolation | Wasmtime | Wasmtime | V8 isolate | Firecracker microVM | Hypervisor microVM (KVM/HV) | Wasmer runtime |
| Component Model? | yes (Spin 2+) | yes | partial (WASI 0.2 in beta) | no (microVM, any binary) | yes | yes (WASIX + components) |
| Trigger model | request-driven; per-trigger invoke | long-running components on a host lattice | request-driven; per-request isolate | request-driven; warm-pool MicroVM | request-driven embeddable | request-driven HTTP |
| Default lifetime | per-request | persistent | per-request | per-request (warm pool) | per-request | per-request |
| Stateful? | no (delegate to KV / DB via factor) | yes (capabilities, actor identity) | partial (Durable Objects) | no | no | partial |
| Cold start | sub-millisecond (Wasm instance) | n/a (host already warm) | ~5ms (V8 isolate) | 100-500ms (Java/Python warm); 16ms-150ms (Rust/Firecracker boot); ~3-9ms with SnapStart | ~1-2ms (microVM create) | ~1ms |
| Open-source? | yes (Apache-2.0) | yes (Apache-2.0) | partial (workerd is OSS, runtime is not) | no | yes (Apache-2.0) | core OSS, edge proprietary |
| Stewardship | Akamai (post-2025-12-01); CNCF Sandbox | Cosmonic + multi-vendor; CNCF Incubating (2024-11) | Cloudflare | AWS | Microsoft; CNCF Sandbox | Wasmer Inc. |
| License | Apache-2.0 | Apache-2.0 | mixed | proprietary | Apache-2.0 / MIT | mixed |
| Production users | Akamai Functions; SpinKube on K8s | Adobe, Orange, MachineMetrics, TM Forum CSPs | Cloudflare's customer base | AWS's customer base | early / experimental | Wasmer Edge tenants |

## Spin vs wasmCloud

Both run Wasmtime. Both are Apache-2.0. Both are CNCF projects. They are not interchangeable — they encode opposite design choices.

| Axis | Spin | wasmCloud |
|---|---|---|
| Component lifetime | per-trigger (instantiate, run, drop) | long-running on a host |
| Identity | manifest entry inside an app | actor identity persists across hosts |
| State strategy | stateless component + factor (KV/SQLite/etc.) | capability provider abstraction; state lives in providers |
| Composition | components are siblings inside one Spin app | components ("actors") plus capability providers, linked at runtime |
| Distribution | one binary process; SpinKube for K8s | host lattice + wadm reconciler (v1, retired); K8s `runtime-operator` + CRDs (v2) |
| Fit | request/response edge functions, HTTP APIs, queue handlers | distributed microservices that need persistent identity and capability swapping |

**One-line summary:** Spin is "FaaS in the WASM Component Model"; wasmCloud is "actor-style microservices in the WASM Component Model." Both are valid responses to the same toolchain. See [`../wasmcloud/architecture.md`](../wasmcloud/architecture.md) for wasmCloud's design.

For Myrhiza, neither is a direct fit (we are not edge-FaaS and we are not a long-running-microservice substrate), but Spin's *factors* abstraction — see [`triggers-and-components.md`](./triggers-and-components.md) — is closer to the capability shape we want than wasmCloud's host-side capability providers.

## Spin vs Cloudflare Workers

Both are HTTP-handler-shaped serverless on a per-request isolate. The runtime layer is the entire difference.

|  | Spin | Cloudflare Workers |
|---|---|---|
| Isolation | Wasmtime instance per invocation | V8 isolate per invocation |
| Primary language story | any → WASM Component Model | JavaScript first; Python via Pyodide; Rust/C/etc. via WASM under V8 |
| Wasm position | first-class; the only execution path | second-class; Wasm modules instantiated *inside* a V8 isolate, alongside JS |
| Threading | none (single-threaded WASM) | none (per-isolate) |
| Open source? | yes (Apache-2.0) — runs anywhere | partial — `workerd` is OSS but the production runtime + edge platform is proprietary |
| Portability | run locally, on K8s (SpinKube), on Akamai Functions | Cloudflare's edge only (or `workerd` for self-host) |
| Cold start | sub-millisecond Wasm instance | ~5ms V8 isolate |

The technical convergence point: **Workers added WASI 0.2 / Component Model support in beta in 2025**, which means a well-shaped Spin component can in principle run on Workers' WASI-on-V8 path, modulo factor-import differences. The strategic divergence: Cloudflare's commercial moat is the edge platform, not the runtime. Spin's open-source story (any host, any cloud) is the load-bearing differentiator — and now sits inside Akamai, which is *also* an edge CDN.

## Spin vs AWS Lambda

Both are FaaS. Different isolation primitives, very different commercial gravity.

|  | Spin | AWS Lambda |
|---|---|---|
| Unit of execution | WASM Component Model instance | Firecracker microVM running a runtime + handler |
| Cold start | sub-millisecond (Wasm instantiation) | 100-500ms typical (Python, Node); ~16ms (Rust on arm64); ~3-9ms with SnapStart snapshot restore; ~125ms raw Firecracker microVM boot |
| Memory floor | ~kilobytes per Wasm instance | ~tens of MB per warm microVM |
| Language story | any → Component Model (Rust, Go, JS, Python, .NET, Wasm-supported) | any (any binary the runtime can fork) |
| Sandboxing | WASM capability surface (deny-by-default WIT imports) | Linux/microVM boundary + IAM |
| Portability | runs anywhere Wasmtime runs | AWS only |
| Open source? | yes | no |

Spin's marketing claim is "0.5ms cold start vs Lambda's 100-500ms" ([Akamai blog](https://www.akamai.com/blog/developers/build-serverless-functions-zero-cold-starts-webassembly-spin)). The honest version: Lambda's cold start is dominated by language-runtime warmup (Python/Java initialization), not by Firecracker (which boots in ~125ms). With SnapStart, Lambda's cold start drops to ~3-9ms — still an order of magnitude slower than Wasm instantiation, but the gap is shrinking.

The structural difference is open source + portability, not raw startup speed. Spin runs on your laptop, on K8s, on Akamai's edge. Lambda runs on AWS.

## Spin vs Hyperlight Wasm

Microsoft's [Hyperlight](https://github.com/hyperlight-dev/hyperlight) is a CNCF Sandbox project (accepted 2025-03-04) that wraps each Wasm workload in a hypervisor-backed sandbox — Firecracker-shaped isolation, but for individual function invocations rather than whole microVMs.

|  | Spin | Hyperlight Wasm |
|---|---|---|
| Isolation primitive | Wasmtime in-process | hypervisor microVM ("micro-guest") with Wasmtime inside |
| Cold start | sub-millisecond Wasm instance | ~1-2ms VM creation; reported ~0.0009s function execution |
| Threat model | Wasm sandbox + host capability surface | Wasm sandbox + KVM/Hyper-V boundary (defense-in-depth) |
| Component Model? | yes | yes (Hyperlight Wasm specifically targets components) |
| Use case | single-tenant or low-isolation FaaS | multi-tenant FaaS where Wasm-only sandbox is judged insufficient |
| OS image | host OS | no OS / no kernel inside the micro-VM |

Hyperlight is *complementary* to Spin, not a replacement: Hyperlight provides a stronger sandbox under the same Component Model contract. A future Akamai Functions could plausibly run Spin components inside Hyperlight micro-VMs for tenant isolation. For Myrhiza, Hyperlight is interesting if we ever need a hypervisor boundary between mutually distrusting peers' code on the same host — not a near-term concern.

## Spin vs Wasmer Edge

Wasmer Edge is Wasmer Inc.'s commercial WASM-edge platform (GA 2024). Same general design point as Spin + Akamai Functions: HTTP-handler-shaped Wasm, sub-millisecond cold start, run-anywhere story. The differentiation is mostly stack alignment:

- **Wasmer Edge** uses the Wasmer runtime + WASIX (Wasmer's POSIX-flavored extension to WASI). Components are second priority.
- **Spin** uses Wasmtime + WASI 0.2 + the official Component Model. Closer to the BA mainline.
- **Pricing:** Wasmer pitches "CDN-like costs"; Akamai Functions is bundled into Akamai's edge product line.

For Myrhiza, the Wasmtime / Component Model alignment makes Spin the better reference even where Wasmer Edge has equivalent features.

## The "WASM serverless" landscape

Sketch of the design space, with each player on it:

```
                     long-running
                          ^
                          |  wasmCloud
                          |
   actor-style ───────────┼───────────── microservice
                          |
                          |
                          |  Spin · Wasmer Edge · Akamai Functions
                          |  Cloudflare Workers
                          |  AWS Lambda
                          |  Hyperlight Wasm
                          v
                     per-request
```

Three rough clusters:

1. **Edge-FaaS / per-request Wasm**: Spin, Wasmer Edge, Akamai Functions, Cloudflare Workers (modulo runtime). Stateless handlers, sub-ms cold start, HTTP-shaped triggers.
2. **Microservice runtime**: wasmCloud. Long-running components, capability providers, persistent identity.
3. **Hypervisor-isolation Wasm**: Hyperlight Wasm, AWS Lambda (Firecracker). Each invocation gets its own VM boundary.

Spin sits firmly in cluster 1. Cluster 1 is the most crowded.

## Implications for Myrhiza

- **Spin's design point is the opposite of `state-apply` purity.** Spin assumes its components do non-deterministic things (random, time, network I/O) and that determinism is *not* a property the runtime enforces. We need a `state-apply` profile where the runtime denies non-determinism by *not granting the imports*. This is closer to wasmCloud-with-restricted-providers than to Spin-as-shipped — but the mechanism (deny imports, declare capabilities in WIT) is the same.
- **The factors / WIT-imports-as-permissions pattern is directly applicable.** Spin's manifest-level declaration "this trigger gets these factor imports" maps almost 1:1 to Myrhiza's `state-apply` capability declaration. Steal the pattern wholesale; ignore the trigger model.
- **Cluster 1 is not where Myrhiza lives.** We are not an edge FaaS substrate. We are a P2P state-replication substrate. Read Spin for the component-isolation patterns; do not let its FaaS-shaped assumptions (HTTP-trigger-as-default, stateless-by-default, factor-imports-as-IO) leak into our authority/identity model.
- **Wasmtime is the load-bearing dep, not Spin.** Every player in cluster 1 except Cloudflare uses Wasmtime. If Wasmtime ships a feature, Myrhiza inherits it. If Spin ships a feature, we don't unless Akamai's roadmap aligns with ours.
- **Hyperlight is a possible future, not a present concern.** If we ever need hypervisor isolation between peers' code on the same host, Hyperlight's micro-guest model is the closest reference.
- **Cloudflare Workers' V8-and-Wasm posture is the cautionary tale.** It's what happens when Wasm is a second-class runtime under a JS-first platform. We want the inverted posture — Wasm-first, with no second runtime — and Spin/wasmCloud/Hyperlight are the references for that.

## Sources

- [`spinframework/spin` README](https://github.com/spinframework/spin)
- [Akamai blog — "Build Serverless Functions with Zero Cold Starts: WebAssembly and Spin"](https://www.akamai.com/blog/developers/build-serverless-functions-zero-cold-starts-webassembly-spin)
- [Cloudflare Workers — WebAssembly runtime docs](https://developers.cloudflare.com/workers/runtime-apis/webassembly/)
- [Cloudflare Workers — Languages](https://developers.cloudflare.com/workers/languages/)
- [Cloudflare blog — "WebAssembly on Cloudflare Workers"](https://blog.cloudflare.com/webassembly-on-cloudflare-workers/)
- [AWS Lambda SnapStart deep dive](https://aws.amazon.com/blogs/compute/under-the-hood-how-aws-lambda-snapstart-optimizes-function-startup-latency/)
- [Microsoft Open Source Blog — "Hyperlight Wasm: Fast, secure, and OS-free"](https://opensource.microsoft.com/blog/2025/03/26/hyperlight-wasm-fast-secure-and-os-free/)
- [Microsoft Open Source Blog — "Introducing Hyperlight"](https://opensource.microsoft.com/blog/2024/11/07/introducing-hyperlight-virtual-machine-based-security-for-functions-at-scale/)
- [`hyperlight-dev/hyperlight`](https://github.com/hyperlight-dev/hyperlight)
- [Wasmer Edge product page](https://wasmer.io/products/edge)
- [Wasmer Edge architecture docs](https://docs.wasmer.io/edge/architecture)
- [The New Stack — "Why Platform Engineers Are Embracing WebAssembly for Serverless"](https://thenewstack.io/why-platform-engineers-are-embracing-webassembly-for-serverless/)
- [mizchi — "Trying out spin and wasmCloud as wasm platforms"](https://gist.github.com/mizchi/47ba840722f593a32a0587df334ee2f2)
- [`../wasmcloud/architecture.md`](../wasmcloud/architecture.md), [`../wasmcloud/comparisons.md`](../wasmcloud/comparisons.md)
