**Date:** 2026-05-09
**Status:** active
**Subject:** Spin runtime architecture — embedding model, factors, component lifecycle

> Sister docs: [`triggers-and-components.md`](./triggers-and-components.md) · [`glossary.md`](./glossary.md) · [`governance.md`](./governance.md) · [`sdks-and-tooling.md`](./sdks-and-tooling.md) · [`spinkube.md`](./spinkube.md) · [`comparisons.md`](./comparisons.md) · [`lessons.md`](./lessons.md)

## What Spin is

Spin is a request-driven serverless framework where each application is a bundle of WASM components that are instantiated, invoked, and dropped per event. The `spin` CLI loads a `spin.toml` manifest, builds an application graph, and the embedded Wasmtime engine instantiates a fresh component instance per trigger event. The runtime — not the component — owns sockets, storage, key-value, AI inference, and outbound HTTP. Components reach those services only through declared WIT imports that the kernel mediates via *factors*.

| Fact | Value |
|---|---|
| Repo | `github.com/spinframework/spin` (was `fermyon/spin`; 301-redirect verified 2026-05-09) |
| Latest release | `v4.0.0` (2026-04-20) |
| Maintenance line | `v3.6.3` (2026-04-09) |
| License | Apache-2.0 WITH LLVM-exception |
| Stars / forks | 6,407 / 302 |
| Stewardship | Akamai (Fermyon acquired 2025-12-01); CNCF sandbox project |
| Wasmtime | `v4.0.0` tag pins `43.0.1` (verified via `Cargo.toml`); `main` workspace pins `44.0.0` (`component-model-async`, `p3`) |
| Target | `wasm32-wasip2` (Component Model + WASI Preview 2; Preview 3 imports surfaced via `0.3.0-rc-2026-03-15`) |
| Triggers | HTTP, Redis (custom triggers authorable in Rust) |
| SDKs | Rust, JS/TS, Python, Go (official); Zig, Moonbit (community) |

## Core architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                         spin CLI (spin up)                            │
│   load manifest → resolve OCI/local → componentize → run trigger      │
└──────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌──────────────────────────────────────────────────────────────────────┐
│                            Trigger Executor                          │
│   trigger-http :  hyper listener → route → invoke component          │
│   trigger-redis:  pub/sub client → dispatch → invoke component       │
└──────────────────────────────────────────────────────────────────────┘
                                   │  per-event
                                   ▼
┌──────────────────────────────────────────────────────────────────────┐
│              Factors Executor  (linker + instance assembly)          │
│   factor-wasi · factor-outbound-http · factor-key-value · …          │
└──────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
                ┌───────────────────────────────────┐
                │  Wasmtime Engine / Store / Linker │
                │  Component instance (one per req) │
                └───────────────────────────────────┘
```

The pieces map directly to crates in the repo: `crates/trigger`, `crates/trigger-http`, `crates/trigger-redis`, `crates/factors`, `crates/factors-executor`, `crates/runtime-factors`, and the per-capability `crates/factor-*` set.

## Component lifecycle

Spin's lifecycle is **per-event, short-lived**:

1. Trigger arrives (HTTP request, Redis message).
2. The trigger executor selects the target component from the locked-app routing table.
3. A Wasmtime `Store` is created, factor `InstanceBuilder`s assemble per-instance state, and a fresh component `Instance` is produced from the cached `Component`.
4. The trigger invokes the exported entry point (e.g. `wasi:http/handler.handle`) through generated `wasmtime::component::bindgen!` bindings.
5. On return, the `Store` is dropped — instance state is released.

`v3.x` introduced opt-in **instance reuse** via `InstanceReuseConfig` (`max_instance_reuse_count`, `idle_instance_timeout`) on the HTTP trigger to amortize warm-start cost; the default remains fresh-per-request. Cold-start work is pushed left: the `Component` is precompiled to AOT artifacts at load and shared across instances; only `Store` and instance-state allocation happen on the hot path.

## Wasmtime embedding

Spin is a thin shell around Wasmtime's component-model API:

- `Engine` is built once per process with async + component-model + Cranelift configured.
- `Component` is loaded from the OCI/locked-app cache, AOT-compiled where possible, and held in `TriggerApp<F>`.
- `Linker<T>` is populated by each factor's `init` hook — every host import that the manifest grants is added here.
- `Store<T>` carries per-instance state, including each factor's `InstanceState`. WASI resources (preopens, env, sockets) are scoped to this `Store`.
- Bindings are generated with `wasmtime::component::bindgen!({ ..., async: true, imports: { default: async | trappable } })` so host calls suspend the guest cooperatively on Tokio.

## Factors (SIP-021 / v3) and fine-grained inheritance (SIP-023 / v4.0)

A *factor* is the unit of host capability. Each factor is a Rust type implementing `trait Factor` with three associated types — `RuntimeConfig`, `AppState`, `InstanceBuilder` — and three lifecycle hooks:

- `init(&mut InitContext)` — once at engine startup; mutates the `Linker` to add WIT imports.
- `configure_app(&self, ctx)` — once per loaded app; validates manifest and computes shared state.
- `prepare(&self, ctx)` — per instance; produces an `InstanceBuilder` whose `build()` yields per-`Store` state. May read other factors' builders (acyclic dependency graph).

Stock factors include `factor-wasi`, `factor-outbound-http`, `factor-outbound-networking`, `factor-key-value`, `factor-sqlite`, `factor-llm`, `factor-variables`, `factor-otel`, `factor-outbound-pg`, `factor-outbound-mysql`, `factor-outbound-redis`, `factor-outbound-mqtt`. The `Trigger` trait is generic over a `RuntimeFactors` set (`crates/runtime-factors`), so a custom trigger picks its capability surface.

**SIP-023** (v4.0) replaced the all-or-nothing `dependencies_inherit_configuration: bool` with per-dependency `inherit_configuration` that takes `true`, `false` (default), or a list of keys (`"allowed_outbound_hosts"`, `"key_value_stores"`, `"sqlite_databases"`, `"ai_models"`, `"environment"`, `"files"`, `"variables"`). This lets a parent grant an `aws:client/s3` dependency outbound HTTP without also handing it the parent's KV stores — proper least-privilege capability composition across the dependency graph.

## Application loading and distribution

`spin.toml` is parsed by `crates/manifest`, lowered into a `LockedApp` (`crates/locked-app`) — a fully-resolved description with content addresses for every component, asset, and sub-dependency. Components ship as **OCI artifacts**: `spin registry push <ref>` uploads the locked-app manifest and content blobs; `spin up <ref>` pulls and runs. Spin keeps a local content cache keyed by digest, so warm runs avoid network. A Spin app is *not* a single artifact — it is the locked manifest plus N component blobs plus N asset blobs, all addressed by content digest.

## Async story

v4.0's headline change is **async interfaces end-to-end**. Generated bindings declare imports `async | trappable`; host implementations are `async fn` running on Tokio; outbound APIs (`spin_sdk::http::send`, async PostgreSQL) drive concurrent I/O from inside a single guest invocation. Component-model async (Wasmtime's `component-model-async` feature, plus WASI Preview 3 RC `0.3.0-rc-2026-03-15` for `wasi:http/handler@0.3.0`) lets a single guest hold multiple in-flight host calls without spawning instances.

## Determinism — why Spin is *not* a fit for `state-apply`

Spin makes no determinism guarantees: the request shape is inherently non-deterministic (clock, network, randomness, headers, request order, peer races). The runtime is built around this — it gives every component WASI clocks, random, sockets, and outbound HTTP by default. Replicas behind a load balancer produce divergent state on purpose; convergence is delegated to whatever backing store the component talks to (Postgres, Redis, KV). For Myrhiza this is the **explicit anti-pattern** for `state-apply`: Spin's surface is what `interaction` and `behavior` profiles look like, not what `state-apply` looks like.

## How Spin mediates capabilities

The pipeline is: manifest declares capability → factor's `configure_app` validates and computes `AppState` → factor's `prepare` builds `InstanceBuilder` populated with only the granted resources → the `Linker` exposes host functions whose closures consult that per-instance state on every call. A component cannot widen its grant at runtime. Outbound HTTP is gated by `allowed_outbound_hosts` (scheme://host:port glob), KV by `key_value_stores`, SQLite by `sqlite_databases`, etc. Compared to **wasmCloud's link-definition model** (capability bound to a named provider at link-time, externally re-bindable), Spin's model is **manifest-static**: the grant is baked into the locked app and the only knob at runtime is which factors the trigger registers.

## Implications for Myrhiza

- **Factor pattern is directly transferable.** Myrhiza's kernel-mediated capability surface should be a set of factor-shaped modules, each owning `init` (linker setup), `configure_app` (manifest validation), and `prepare` (per-instance state). Acyclic dependency graph between them is the right constraint.
- **SIP-023 fine-grained inheritance is the right shape for app-bundle composition.** When Myrhiza apps gain sub-component dependencies, copy the per-key allow-list rather than `inherit: bool`.
- **Locked-app + content-addressed OCI distribution is a proven pattern** for shipping multi-component bundles. Myrhiza's app-bundle should be lockfile + content-digest-addressed blobs; OCI is one transport, not the only one.
- **Manifest-static capability binding** beats wasmCloud's runtime link-rebinding for the determinism we need; an app's capability surface should not change between the moment its `state-apply` is hashed and the moment it runs.
- **Spin's request-handler shape is the wrong shape for `state-apply`.** Reuse Spin's mechanics (factors, linker assembly, OCI distribution) but reject its lifecycle (per-event instance with full WASI). `state-apply` invocations need a stripped-down WASI surface — no clocks, no random, no sockets — and a content-addressed deterministic helper set.
- **Async + component-model-async is mature enough to lean on** (Wasmtime 44, wasip3 RC). Myrhiza's host can adopt the same async-bindgen pattern for kernel-mediated host calls without inventing a parallel concurrency story.

## Sources

- spinframework/spin repo (license, version, structure): `gh api repos/spinframework/spin` (verified 2026-05-09)
- v4.0.0 release notes: `gh api repos/spinframework/spin/releases/latest`
- Workspace `Cargo.toml`: `crates/world/wit/world.wit`, `Cargo.toml` (Wasmtime 44.0.0 with `component-model-async` + `p3`)
- SIP-021 (Spin Factors): `docs/content/sips/021-spin-factors.md`
- SIP-023 (fine-grained capability inheritance): `docs/content/sips/023-fine-grained-capability-inheritance.md`
- SIP-008 (OCI registries): `docs/content/sips/008-using-oci-registries.md`
- Spin docs: `spinframework.dev/v3/extending-and-embedding`, `spinframework.dev/v3/manifest-reference`
- Akamai acquisition press release (2025-12-01): `akamai.com/newsroom/press-release/akamai-announces-acquisition-of-function-as-a-service-company-fermyon`
- Crate sources: `crates/factors/src/factor.rs`, `crates/trigger-http/src/lib.rs`, `crates/world/src/lib.rs`
