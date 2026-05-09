**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — glossary of project-specific terms

# Glossary

System-specific terms used across this folder. Cross-references to the file where the term is treated in depth. Where a term is v1-specific or v2-specific, that is flagged.

## Components and providers

- **Component** — a CM-flavored WASM component (per `../wasm-component-model/`). Pure compute, imports-only. Synonym for what v1 wasmCloud called an "actor" until the rename. See [architecture.md](architecture.md), [capability-model.md](capability-model.md).
- **Actor** — historical term for "component." Officially deprecated in 2024; kept here only for reading older blog posts.
- **Capability provider** *(v1)* — a host-side process providing a WIT-typed capability (HTTP server, KV store, blobstore, messaging, secrets). Each runs in its own OS process; communicates with components via wRPC over NATS. See [capability-model.md](capability-model.md).
- **Host plugin** *(v2)* — the v2 replacement for capability providers. Runs in-process inside the wasmCloud host. Implements a Rust trait (`HostPlugin`) and is statically linked at host build time. Loses the runtime-mutability of v1 link definitions but gains the ~6× perf win wasmCloud's v2 release post advertises (vendor figure; no independent benchmark). See [capability-model.md](capability-model.md).
- **Service component** *(v2)* — a v2 concept: a component that exports a WIT interface and is reachable in-process by other components in the same workload, without going through a host plugin. See [capability-model.md](capability-model.md).
- **Workload** *(v2)* — the v2 unit of deployment: a set of components + the host plugins they need + their config + their host-interfaces declarations. K8s CRD `Workload` in `runtime.wasmcloud.dev/v1alpha1`. See [architecture.md](architecture.md).

## Lattice and orchestration *(v1)*

- **Lattice** *(v1)* — a logical mesh of wasmCloud hosts joined via shared NATS infrastructure. All control messages and inter-component RPC flow over NATS topics scoped to a lattice ID. **Removed in v2.** See [architecture.md](architecture.md).
- **Lattice ID** *(v1)* — the multi-tenancy boundary; NATS subjects are namespaced by `wasmbus.<lattice-id>.<command>`. See [architecture.md](architecture.md).
- **Link definition** *(v1)* — a runtime-mutable record stored on the lattice that wires a component's WIT import to a specific provider's WIT export. Authority comes from the operator declaring the link, not from cryptographic ocaps. **Removed in v2.** See [capability-model.md](capability-model.md).
- **`wasmbus.*`** *(v1)* — the NATS topic prefix wasmCloud uses for control-plane messaging. See [architecture.md](architecture.md).
- **wadm** *(v1)* — wasmCloud Application Deployment Manager. Reconciles a declarative OAM-shaped manifest into actual lattice state (running components, providers, links). Last release v0.21.1 (2026-01-29). **Functionally subsumed by the K8s `runtime-operator` in v2.** See [tooling.md](tooling.md).
- **OAM manifest** *(v1)* — Open Application Model YAML schema wadm consumed. Subsumed by K8s CRDs in v2. See [tooling.md](tooling.md).

## Kubernetes-native orchestration *(v2)*

- **`runtime-operator`** *(v2)* — a Go-based Kubernetes operator that reconciles `Host` and `Workload` CRDs in `runtime.wasmcloud.dev/v1alpha1`. The K8s-native replacement for wadm + the lattice-control-plane. See [architecture.md](architecture.md).
- **`runtime-gateway`** *(v2)* — a Go-based HTTP router that fronts component HTTP exports. Replaces the v1 `wasmcloud-httpserver` provider for ingress. See [architecture.md](architecture.md).
- **Host CRD** *(v2)* — Kubernetes `kind: Host` resource specifying a wasmCloud host pod's configuration. See [architecture.md](architecture.md).
- **Workload CRD** *(v2)* — Kubernetes `kind: Workload` resource specifying a component bundle deployed onto a host group. See [architecture.md](architecture.md).
- **Host group** *(v2)* — label-selector-based set of hosts a Workload can land on. The v2 stand-in for v1 lattice scope. See [architecture.md](architecture.md).
- **`host_interfaces`** *(v2)* — the field on a Workload manifest that lists the WIT interfaces a component imports and the host plugin or service component that satisfies each. The v2 replacement for v1 link definitions. See [capability-model.md](capability-model.md).

## Inter-component RPC

- **wRPC** — wasmCloud-originated, Bytecode-Alliance-stewarded WIT-derived RPC protocol. Lives at `bytecodealliance/wrpc`. License Apache-2.0 WITH LLVM-exception. Wire protocol const `wrpc.0.0.1`. See [wrpc.md](wrpc.md).
- **wRPC transport** — the pluggable layer underneath wRPC. Currently shipped: `wrpc-transport-nats 0.30.0`, `wrpc-transport-quic 0.5.0`, `wrpc-transport-web 0.2.0`. NATS was the v1 default; v2 makes the choice explicit. See [wrpc.md](wrpc.md).
- **Subject (NATS-flavored)** — a hierarchical topic name. wRPC-over-NATS uses subjects to address remote components; the scheme is documented in `wrpc/SPEC.md`. See [wrpc.md](wrpc.md).
- **`wrpc:rpc@0.1.0`** — the WIT package defining the wRPC primitives (call, indexing, error). See [interfaces.md](interfaces.md).

## WIT package families

- **`wasi:*`** — WebAssembly System Interface, upstream at `WebAssembly/WASI`. wasmCloud uses `wasi:keyvalue`, `wasi:blobstore`, `wasi:config`, `wasi:logging`, `wasi:http`, `wasi:otel`. See [interfaces.md](interfaces.md), `../wasm-component-model/preview-status.md` for upstream status.
- **`wasmcloud:*`** — wasmCloud-native WIT packages. `wasmcloud:bus@1.0.0`, `wasmcloud:secrets@0.1.0-draft`, `wasmcloud:messaging@0.2.0`. The `secrets` package is **retired** in v2 in favor of `wasi:config` + K8s Secrets. See [interfaces.md](interfaces.md).
- **`wrpc:*`** — wRPC-specific WIT packages. `wrpc:rpc@0.1.0` is the only widely-used one. See [interfaces.md](interfaces.md), [wrpc.md](wrpc.md).

## CLI and tooling

- **`wash`** — the wasmCloud Shell. v1 line on crates.io as `wash-cli` (`0.43.0`, 2026-02-04). v2 line in-monorepo as `wash` (and the binary is renamed `wash-runtime` for the host process); rc.7 of v2.0.0 was 2026-02-19. See [tooling.md](tooling.md).
- **`wash-runtime`** *(v2)* — the v2 host binary that replaces the v1 `wasmcloud` binary. Runs inside a K8s pod scheduled by the `runtime-operator`. See [tooling.md](tooling.md), [architecture.md](architecture.md).
- **`wash dev`** — the live-reload developer inner-loop subcommand. Reasonably stable across v1 → v2; one of the few subcommands that survived the pivot. See [tooling.md](tooling.md).
- **OCI registry** — wasmCloud was an early adopter of OCI artifact registries (Docker Hub, GHCR, ECR) for component distribution. The `wash reg` (v1) / `wash oci` (v2) subcommand is the client. Compare `wkg` in the BA family. See [tooling.md](tooling.md), `../wasm-component-model/tooling.md`.

## Steward and governance

- **CNCF Sandbox / Incubating / Graduated** — Cloud Native Computing Foundation's three-tier project maturity. wasmCloud entered Sandbox 2021-07-13, was promoted to **Incubating on 2024-11-08** (TOC vote, public announcement 2024-11-12). Graduation has no current ETA. See [governance.md](governance.md).
- **TOC** — CNCF Technical Oversight Committee. Votes on project maturity transitions. See [governance.md](governance.md).
- **LF Projects, LLC** — the legal vehicle that holds wasmCloud's trademarks under CNCF policy. See [governance.md](governance.md).
- **Cosmonic Inc** — the commercial entity associated with wasmCloud. wasmCloud project itself was created by Liam Randall and Kevin Hoffman per the CNCF announcement; Bailey Hayes is a Cosmonic co-founder and current wasmCloud tech lead. Primary commercial steward. Bytecode Alliance member. Pivoted from "Cosmonic Connect" hosted PaaS (2023) to **Cosmonic Control** K8s control plane (2025-07 →). See [commercial.md](commercial.md), [governance.md](governance.md).

## Cross-substrate (for comparison with neighbor folders)

- **xsnap** (Agoric) — Agoric SwingSet's per-vat live-instance snapshot tool. wasmCloud has no equivalent; live-only execution. See `../agoric-endo/persistence.md`.
- **CapTP** (Spritely / Agoric / OCapN) — capability-typed cross-machine RPC. The closest neighbor to wRPC; differs in being capability-flavored rather than interface-flavored. See `../spritely-ocapn/captp-and-ocapn.md`.
- **Vat** (Agoric) — analogous to a wasmCloud component instance: single-threaded, sandboxed, capability-mediated. See `../agoric-endo/vat-model.md`.
- **Zome** (Holochain) — Holochain's analog to a wasmCloud component: a WASM module with typed callbacks. See `../holochain/`.

## Sources

- wasmCloud documentation: https://wasmcloud.com/docs
- wasmCloud CRDs: https://github.com/wasmCloud/wasmCloud/tree/main/crates/runtime-operator
- wRPC spec: https://github.com/bytecodealliance/wrpc/blob/main/SPEC.md
- CNCF maturity model: https://github.com/cncf/toc/blob/main/process/graduation_criteria.md
