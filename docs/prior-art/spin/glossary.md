**Date:** 2026-05-09
**Status:** active
**Subject:** Glossary of Spin terms used across this folder

# Glossary

Spin-specific vocabulary used across [architecture.md](architecture.md), [triggers-and-components.md](triggers-and-components.md), [sdks-and-tooling.md](sdks-and-tooling.md), [spinkube.md](spinkube.md), and the cross-cutting files.

## Project + organization

- **Spin** — open-source serverless WASM Component Model framework. Originally Fermyon's; now Akamai-stewarded post-acquisition (2025-12-01). Apache-2.0. See [README.md](README.md), [governance.md](governance.md).
- **`spinframework`** — the GitHub organization. Created 2025-01-21 (pre-acquisition rename from `fermyon/spin` driven by CNCF vendor-neutrality requirement). See [governance.md](governance.md).
- **Fermyon Inc** — original company behind Spin. Founded ~2021 by Matt Butcher, Radu Matei, and others. $26M total funding, $20M Series A from Insight Partners (announced 2022-10-24). Acquired by Akamai 2025-12-01. See [governance.md](governance.md).
- **Akamai** — the acquirer. Operates the productized Akamai Functions service powered by Spin. Cloud Technology Group hosts the post-acquisition Spin team. See [governance.md](governance.md).
- **Bytecode Alliance (BA)** — Spin team contributes to BA-stewarded projects (Wasmtime, Component Model, WASI). See [`../wasm-component-model/governance.md`](../wasm-component-model/governance.md).

## Runtime + architecture

- **Trigger** — runtime event source that invokes a component. Two production triggers: HTTP and Redis. The trigger executor decides which component to invoke for a given event. See [triggers-and-components.md](triggers-and-components.md).
- **HTTP trigger** — listens on a port, dispatches incoming HTTP requests to components implementing `wasi:http/incoming-handler`. Most-used trigger. See [triggers-and-components.md](triggers-and-components.md).
- **Redis trigger** — pub/sub-shaped; component implements `fermyon:spin/redis-channel` (legacy) or `spin:*/redis` (current). See [triggers-and-components.md](triggers-and-components.md).
- **Component** — WASM Component Model artifact. The unit Spin invokes per trigger event. See [architecture.md](architecture.md).
- **Application** — collection of one or more components plus their triggers, defined by a `spin.toml` manifest. See [triggers-and-components.md](triggers-and-components.md).
- **`spin.toml`** — application manifest. Defines components, triggers, allowed outbound hosts, key-value stores, etc. See [triggers-and-components.md](triggers-and-components.md).
- **Factor** — Spin v4's per-host-capability runtime module (introduced in SIP-021). Each factor mediates one capability area: `factor-outbound-http`, `factor-key-value`, `factor-sqlite`, `factor-llm`, etc. Factors implement the `Factor` trait with init / configure_app / prepare lifecycle. See [architecture.md](architecture.md).
- **`Factor` trait** — Spin v4's runtime extension API; SIP-021. Three associated types (`RuntimeConfig`, `AppState`, `InstanceBuilder` — verified via `crates/factors/src/factor.rs` on `main`). See [architecture.md](architecture.md).
- **SIP** — Spin Improvement Proposal. Spin's design-RFC mechanism. SIP-021 introduced factors; SIP-023 added per-key `inherit_configuration` grants. See [architecture.md](architecture.md).
- **`inherit_configuration`** — SIP-023 fine-grained capability inheritance. Lets a parent component grant only specific configuration keys to a child component, not all of its environment. See [architecture.md](architecture.md).

## Trigger / component mechanics

- **`wasi:http/incoming-handler`** — the WASI HTTP world a component implements to handle HTTP triggers. See [triggers-and-components.md](triggers-and-components.md).
- **`wasi:http/proxy`** — the WIT world Spin instantiates with HTTP-trigger components.
- **`fermyon:spin/*`** — legacy WIT package namespace (2.0.0 and earlier). Backward-compatible; still imported.
- **`spin:*`** — current WIT package namespace (3.x / 4.x).
- **`allowed_outbound_hosts`** — `spin.toml` field declaring which outbound HTTP destinations a component can reach. Manifest-static permission. See [triggers-and-components.md](triggers-and-components.md).
- **`key_value_stores`** — `spin.toml` field declaring which KV stores a component can access. Default store is in-memory (ephemeral); production deployments swap in Redis/SQLite/durable backends.
- **Component composition** — combining multiple components into one application. Spin natively supports plug-style composition; arbitrary compositions require `wac` as a build-time step.

## Tooling

- **`spin` CLI** — developer entry point. Verbs: `new`, `build`, `up`, `deploy`, `watch`, `registry push`, `registry pull`. See [sdks-and-tooling.md](sdks-and-tooling.md).
- **ComponentizeJS** — the build path for JS/TS Spin components. Wraps QuickJS as a WASM Component. See [sdks-and-tooling.md](sdks-and-tooling.md).
- **componentize-py** — the build path for Python Spin components.
- **TinyGo `wasip2`** — the build path for Go Spin components. Standard `go` toolchain does not yet target wasip2 cleanly; TinyGo fills the gap.
- **`wac`** — WebAssembly Composition tool (BA-stewarded). Combines multiple components into one. Used by Spin for build-time composition.
- **`wkg`** — wasm-pkg-tools (BA-stewarded). Resolves WIT packages from OCI-flavored registries. Used by Spin's component-package-resolution path.
- **OCI artifact distribution** — Spin components ship as OCI artifacts; `spin registry push/pull` interacts with OCI registries (GHCR, Docker Hub, etc.). See [sdks-and-tooling.md](sdks-and-tooling.md).

## SpinKube

- **SpinKube** — Kubernetes-based deployment story for Spin. CNCF Sandbox project (accepted 2025-01-21). Three components below. See [spinkube.md](spinkube.md).
- **spin-operator** — the Kubernetes operator. Go, controller-runtime. Reconciles `SpinApp` CRDs into running pods. v0.6.1 current. See [spinkube.md](spinkube.md).
- **containerd-shim-spin** — runs Spin apps as containerd-compatible workloads via runwasi. v0.24.0 current.
- **runtime-class-manager** — provisions the Spin runtime class on K8s nodes. Formerly KWasm.
- **`SpinApp` CRD** — Kubernetes custom resource defining a Spin application deployment.
- **`SpinAppExecutor` CRD** — Kubernetes custom resource defining how a SpinApp is executed (which shim, which runtime class).

## Cross-substrate (for comparison with neighbor folders)

- **Wasmtime** ([wasm-component-model](../wasm-component-model/)) — the WASM runtime Spin embeds. Spin `v4.0.0` tag pins Wasmtime `43.0.1`; `main` pins `44.0.0`.
- **WASI Preview 2 / Preview 3** ([wasm-component-model](../wasm-component-model/)) — Spin commits to Preview 2 stable; v4 ships dual-target with Preview 3 RC imports alongside.
- **Capability provider** ([wasmCloud](../wasmcloud/)) — wasmCloud-v1's runtime-bound capability mediation. Spin's *factor* is the static-binding analog. See [comparisons.md](comparisons.md), [`../wasmcloud/capability-model.md`](../wasmcloud/capability-model.md).
- **Lattice** ([wasmCloud](../wasmcloud/)) — wasmCloud-v1's NATS-based mesh. No Spin equivalent — Spin assumes single-host topology (or K8s-managed multi-host via SpinKube). See [comparisons.md](comparisons.md).
- **state-apply** (Myrhiza) — pure WASM Component Model function `(prior_state, event) → next_state`. Spin's HTTP-handler shape is the opposite design point.
- **OCI Distribution** — both Spin and wasmCloud ship components as OCI artifacts. Compatible distribution layer.

## Sources

- Spin docs: <https://spinframework.dev>
- SpinKube docs: <https://www.spinkube.dev>
- Spin SIPs: <https://github.com/spinframework/spin/tree/main/docs/content/sips>
- Akamai acquisition press release: <https://www.akamai.com/newsroom/press-release/akamai-announces-acquisition-of-function-as-a-service-company-fermyon>
- See per-file `## Sources` sections for detailed URLs.
