**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — host topology, control plane, and runtime substrate

# wasmCloud Architecture

This file documents the host-and-cluster topology of wasmCloud as it actually exists today, on top of the v2 release line. Many published descriptions of wasmCloud (including most blog posts, conference talks, and the brief that originated this document) describe the v1 architecture, which is materially different. The v1 model is summarized at the bottom under [Historical context: v1](#historical-context-v1) so that older references are not confusing.

## Status snapshot

- Current stable: `wasmCloud v2.1.0`, published 2026-05-07. `v2.0.7` is the most recent v2.0 patch (2026-05-05). `v2.0.0` was released 2026-03-22.
- Repository: `wasmCloud/wasmCloud`, Apache-2.0, ~2,301 stars, default branch `main`, created 2020-10-15.
- Implementation: pure Rust workspace. The two crates are `crates/wash-runtime` (the runtime library) and `crates/wash` (the CLI/binary that embeds it). Two Go modules in the same repo provide Kubernetes integration: `runtime-operator` and `runtime-gateway`.
- CNCF: wasmCloud is an Incubating project (promoted from Sandbox to Incubating on 2024-11-08; public announcement 2024-11-12). "Graduated" in CNCF terminology is a separate higher tier; wasmCloud is *not* Graduated.
- Companion project `wadm` is now legacy. Last release `v0.21.1` on 2026-01-29 — 3+ months stale and explicitly replaced by the v2 Kubernetes operator.

## Brief corrections (read this before continuing)

The originating brief contained several claims that turned out to be wrong against the v2 codebase. They are corrected here:

- **The Elixir/OTP host is gone.** The original `wasmCloud/wasmcloud-otp` repo (Elixir + Rust NIFs) is no longer the host. It is unarchived but has not been pushed to since 2024-10-12. Some dormant readers still describe wasmCloud as "BEAM-based"; that has not been true for the supported runtime since the Rust rewrite, and v2 finalizes the Rust-only story. A v2 host is one OS process running `wash host`, which embeds the `wash-runtime` Rust crate. Wasmtime is the underlying engine.
- **wadm is no longer the scheduler.** In v2 the scheduler is `runtime-operator`, a Go-based Kubernetes operator using CRDs in the `runtime.wasmcloud.dev/v1alpha1` group. wadm is documented as a v1 component that has been removed from the v2 deployment story.
- **NATS is no longer the data plane.** In v2 NATS carries control-plane messages (operator → host RPC over `wasmcloud.runtime.v2.WorkloadService`) and is also used by some host plugins as a backend (NATS-KV, NATS object store, NATS messaging). Inter-component RPC for components in the same workload is in-process, not over NATS. Cross-host inter-component RPC is no longer automatic; if you want it, you arrange it yourself with `wasmcloud:messaging` or wRPC.
- **Capability providers are gone as a runtime construct.** In v2 the equivalent role is filled by **host plugins** (in-process Rust `HostPlugin` impls) or **service components** (a long-running Wasm component bundled with the workload). The v1 "external provider over NATS" architecture has been retired for performance and operational reasons.
- **Link definitions are gone.** Workload manifests now declare `hostInterfaces` directly. There is no separate link-graph CRD or link table.

These are not minor renamings; they reverse several major architectural commitments the project made in 2022–2024. The migration guide at <https://wasmcloud.com/docs/migration> is explicit about which v1 subsystems were removed and why.

The rest of this document describes v2.

## v2 host topology

A wasmCloud host is one OS process. Concretely it is the `wash host` subcommand, which:

1. Builds a Wasmtime `Engine` (with the pooling allocator and configurable WASIp3 support).
2. Constructs a `ClusterHostBuilder` (in `wash_runtime::washlet`) and registers built-in plugins: `wasi:config` (`DynamicConfig`), `wasi:logging` (`TracingLogger`), `wasi:blobstore` (`NatsBlobstore`), `wasmcloud:messaging` (`NatsMessaging`), `wasi:keyvalue` (`NatsKeyValue`), and optionally `wasmcloud:postgres`.
3. Connects to NATS as the **scheduler/control-plane** transport (default `nats://localhost:4222`) and separately to NATS as a **data-plane** transport for the NATS-backed plugins. The two URLs can differ — that distinction (separate scheduler-NATS vs data-NATS) is a deliberate v2 design choice.
4. Starts an HTTP server on a configurable bind address. The HTTP server is a `HostHandler` rather than a `HostPlugin`; it is registered via `with_http_handler()`. All other plugin imports go through `with_plugin()`.
5. Emits periodic `HostHeartbeat` messages (id, hostname, host group, version, system metrics) to the operator over NATS.

A host advertises a `host_group` label (default `default`) and an optional `environment` label (typically the Kubernetes namespace, populated by the runtime-operator's Helm chart via the downward API). Operators schedule workloads onto hosts whose labels satisfy the workload's `hostSelector`.

Hosts hold no Kubernetes credentials. They never talk to the K8s API. The privilege boundary is enforced by the operator: the operator has cluster RBAC limited to `runtime.wasmcloud.dev`; it pushes scheduling decisions to hosts over NATS.

### `wash-runtime` as a library

`wash-runtime` is a standalone crate. The README describes it as "an opinionated Wasmtime wrapper that provides a runtime and workload API." It is usable as an embedded runtime with no NATS at all — `HostBuilder::new().with_engine(...).with_http_handler(...).with_plugin(...).build()` produces a host that runs workloads from in-memory `WorkloadStartRequest` payloads. The K8s+NATS scheduling layer is added on top by the `washlet::ClusterHostBuilder` path that `wash host` uses.

This split matters: the **runtime** (Wasmtime + plugin trait + workload API) is independent of the **scheduler** (NATS-mediated K8s operator). Other schedulers can be written against the same workload API. The FAQ explicitly invites such alternatives, while noting that maintainer effort is concentrated on the K8s case.

### Workloads, components, services

The unit of deployment is a **workload**, which is roughly the K8s Pod analogue. A workload contains:

- Zero or more **components** — pure Wasm components from OCI images. Components are the user's compute logic.
- Optionally one **service** — a single long-running component (also Wasm) that listens on a TCP socket. Services exist for cases that don't fit the request/response component model: cron jobs, connection pools, in-process caches, TCP servers.
- A list of **`host_interfaces`** — the WIT interfaces the components import that must be satisfied by host plugins (e.g. `wasi:keyvalue`, `wasi:config`, `wasmcloud:messaging`).
- Per-workload **volumes** (host path or empty-dir, K8s-style) and per-component **`local_resources`** (memory limit, CPU limit, env, allowed-hosts allowlist for outbound HTTP).

All components in a workload run on the same host. This is intentional and is the architectural mechanism by which v2 achieves the documented ~6× throughput improvement over v1 — there is no NATS hop between cohabiting components, and host-provided capabilities are served in-process.

### Plugin model

A host plugin is a Rust object that implements the host side of one or more WIT interfaces and registers itself with the host. Built-in plugins shipped with `wash-runtime`:

| Plugin | WIT world | Backend |
|---|---|---|
| `wasi_config::DynamicConfig` | `wasi:config/store@0.2.0-rc.1` | In-memory map |
| `wasi_logging::TracingLogger` | `wasi:logging/logging@0.1.0-draft` | `tracing` crate |
| `wasi_keyvalue::NatsKeyValue` | `wasi:keyvalue/{atomics,batch,store}@0.2.0-draft` | NATS-KV |
| `wasi_blobstore::NatsBlobstore` | `wasi:blobstore/{types,container,blobstore}@0.2.0-draft` | NATS Object Store |
| `wasmcloud_messaging::NatsMessaging` | `wasmcloud:messaging@0.2.0` | NATS pub/sub |
| `wasmcloud_postgres::WasmcloudPostgres` | `wasmcloud:postgres@0.1.1-draft` | PostgreSQL via deadpool-postgres |
| `wasi_otel` | `wasi:otel@0.2.0-rc.1` | OpenTelemetry SDK |

The full WIT world set is declared at `crates/wash-runtime/wit/world.wit`. WASIp2 interfaces (`wasi:filesystem`, `wasi:clocks`, `wasi:random`, `wasi:io`, `wasi:sockets`, `wasi:cli`) are baked into the host core and always available — they are not registered as plugins.

**Default-deny is the rule.** If no handler is registered for a particular import, the host wires in a deny-all stub. Components cannot reach a capability they did not import or that the operator did not configure.

Custom plugins are encouraged; the `HostPlugin` trait in `wash_runtime::plugin` is the extension point. Companion files: [`wrpc.md`](./wrpc.md) (transport) and [`interfaces.md`](./interfaces.md) (WIT contracts) cover the WIT side.

## v2 control plane

The control plane is the wire between `runtime-operator` and `wash host`. The protocol is gRPC-over-NATS, defined in `proto/wasmcloud/runtime/v2/`:

```proto
service WorkloadService {
  rpc WorkloadStart(WorkloadStartRequest)   returns (WorkloadStartResponse);
  rpc WorkloadStatus(WorkloadStatusRequest) returns (WorkloadStatusResponse);
  rpc WorkloadStop(WorkloadStopRequest)     returns (WorkloadStopResponse);
}
service HostService {
  rpc HostHeartbeat(HostHeartbeatRequest) returns (google.protobuf.Empty);
}
```

The operator initiates `WorkloadStart`/`Status`/`Stop` against a chosen host. Hosts initiate `HostHeartbeat` on a periodic schedule. NATS provides the queue/pub-sub fabric; the messages themselves are protobuf.

The state of record is **Kubernetes etcd** — five CRDs in `runtime.wasmcloud.dev/v1alpha1`:

| CRD | K8s analogue | Purpose |
|---|---|---|
| `WorkloadDeployment` | `Deployment` | Rollouts |
| `WorkloadReplicaSet` | `ReplicaSet` | Replica count |
| `Workload` | `Pod` | Single schedulable unit (components + service) |
| `Host` | `Node` | Capacity registration; receives scheduling |
| `Artifact` | (no analogue) | OCI resolution + caching |

The reconciliation loop runs in `runtime-operator` and is conventional `kube-rs`-style controller code. There is no eventually-consistent custom reconciler analogous to v1's wadm; convergence is whatever K8s gives you.

`runtime-gateway` is a small Go service that watches `Host` and `Workload` CRDs and forwards HTTP requests to the host running the matching workload, enriching them with `X-Real-Ip` and `X-Workload-Id` headers. It is the v2 ingress mechanism for `wasi:http` workloads.

### Multi-tenancy

Multi-tenancy in v2 is K8s-native: namespaces, NetworkPolicies, and the operator's CRD-scoped RBAC. The `environment` label on a `Host` (typically the K8s namespace) is used for tenant isolation at scheduling time. The v1 concept of "lattice" as a logical multi-tenant overlay on shared NATS is gone; what remains is a host group (a label) plus the standard K8s isolation model.

### Secrets and configuration

The `wasmcloud:secrets` interface that existed in v1 is **explicitly retired**. v2 uses `wasi:config` backed by Kubernetes ConfigMaps and Secrets. A workload manifest's `localResources.environment.secretFrom` and `configFrom` reference K8s objects by name; the operator injects their values into the workload's `LocalResources.config`/`environment` map at start time. The component never sees the K8s API and never holds a reference to the original Secret object — the host materializes the values into the component's `WasiCtxBuilder` env map.

This trades the bespoke `wasmcloud:secrets` envelope-encryption story (which was a real piece of design work) for a thin pass-through to Kubernetes. The migration guide flags this as an explicit removal.

## Inter-component calls

Two cases:

- **Same workload, same host.** Calls go through Wasmtime's component linker. No NATS, no serialization, no network. This is the path the docs cite as ~30,000 RPS.
- **Different workloads (or distributed).** No automatic transport. Components call `wasmcloud:messaging` or another explicit interface and serialize themselves. This is documented as a deliberate design choice in the FAQ. wRPC remains usable for explicit cross-component RPC but is no longer the default cross-component transport.

This is a major reversal from v1, which proudly advertised "actor-to-actor calls over the lattice" as transparent. The v2 stance is that the transparency was paying for ~6× throughput overhead on the in-process case and was worth giving up.

## Wasmtime version pinning

`wasmtime = "44"` and `wasmtime-wasi = "44"` in v2.1.0. `wash-runtime` re-exports `wasmtime` so embedders use the same major version. Optional `wasip3` cargo feature gates support for components targeting `wasi@0.3` interfaces.

## Implications for Myrhiza

wasmCloud v2's architecture is closer to Myrhiza's intended design than v1 was, but several specific choices are still misaligned:

- **In-process host plugins as the capability surface** is the same shape as Myrhiza's kernel-mediated host imports. The plugin trait, the deny-by-default stance, and the WIT-typed boundary are all direct precedent. `wash-runtime`'s `HostPlugin` and `HostHandler` traits are worth studying when designing Myrhiza's capability registration API.
- **Workload as the deployment unit** that bundles components + a service component + a declared interface set is a useful packaging idea. Myrhiza apps already think of themselves as bundles; the v2 `Workload` is the closest CM-runtime analogue.
- **Control plane vs data plane separation** is healthy and Myrhiza should preserve it. wasmCloud's separate `scheduler-nats-url` and `data-nats-url` flags are a worked example.
- **CRDs as the state-of-record + a reconciler** is *not* compatible with peer-symmetric P2P. wasmCloud v2 made the deliberate choice to depend on a centralized K8s API server; that is an explicit anti-pattern for a peer-symmetric system. Myrhiza's equivalent of "desired state" must be a CRDT-shaped artifact replicated peer-to-peer, not an authoritative server-of-truth.
- **NATS as the operator↔host RPC** is also incompatible. Even demoted to control-plane-only, NATS is a centralized coordinator. Myrhiza's analogous channel is some form of gossip or direct peer connections; the protobuf message shapes (`WorkloadStartRequest`, `HostHeartbeat`) might still be useful as a starting point for the *content* of those messages.
- **`wasmcloud:secrets` being removed in favor of K8s Secrets** is a useful negative result. The original interface was the right shape (envelope encryption, per-component scoping, no key material in component memory) but the operational cost was apparently high enough that the project preferred to delegate to the orchestrator. Myrhiza does not have an orchestrator to delegate to; the v1 design (not the v2 retreat) is the more relevant precedent. See [`interfaces.md`](./interfaces.md) and [`history.md`](./history.md) for the v1 secrets architecture.
- **Component-to-component is in-process when colocated, explicit RPC when not.** This is approximately the right shape for Myrhiza too. The wasmCloud team's stated reasoning (transparent distributed RPC was paying ~6× overhead on the hot path) is a strong argument for *not* making P2P transport invisible at the component-call level.

## Cross-references

- [`wrpc.md`](./wrpc.md) — the wRPC transport layer
- [`interfaces.md`](./interfaces.md) — the WIT package families components see
- [`tooling.md`](./tooling.md) — `wash` CLI and developer experience
- [`governance.md`](./governance.md) — CNCF and project governance
- [`history.md`](./history.md) — v1, the OTP era, waSCC roots
- [`commercial.md`](./commercial.md) — Cosmonic and the commercial steward story
- [`comparisons.md`](./comparisons.md), [`critiques.md`](./critiques.md), [`open-problems.md`](./open-problems.md), [`lessons.md`](./lessons.md) — analytical neighbors

Prior-art neighbors:

- [WASM Component Model](../wasm-component-model/) — the substrate wasmCloud builds on
- [Wasmtime](../wasm-component-model/wasmtime.md) — the actual engine wasmCloud uses
- [Iroh](../iroh/) — peer-to-peer transport prior art (contrast with NATS)
- [Holochain](../holochain/) — agent-centric P2P state model (contrast with K8s reconciler)
- [Spritely OCapN](../spritely-ocapn/) — capability-secure cross-host RPC (contrast with wRPC)

## Historical context: v1

For readers coming from v1 documentation, the v1 architecture (relevant up through wasmCloud `1.x`, ending with the v2 cutover in March 2026) was:

- **wadm** as a stateful scheduler reading and writing NATS JetStream.
- **OAM manifests** (`apiVersion: core.oam.dev/v1beta1`, `kind: Application`) as the source of truth.
- **Capability providers** as separate OS processes, communicating with components over NATS via wRPC.
- **Link definitions** as a runtime-mutable mapping from a component's import to a particular provider instance, stored in JetStream.
- **Lattice** as the named NATS topic-prefix tenancy boundary; one NATS deployment could host many lattices.
- **JWT claims** (signed with ed25519 nkeys) as the identity primitive for components and hosts. Encoded into OCI artifacts.
- **CloudEvents** for observability.
- **Actor** as the term for what is now a component.

The v2 migration retired all of those except wRPC (which still exists at `bytecodealliance/wrpc` for explicit cross-component RPC).

## Sources

- wasmCloud v2 repository: <https://github.com/wasmCloud/wasmCloud>
- v2.1.0 release: <https://github.com/wasmCloud/wasmCloud/releases/tag/v2.1.0>
- v2.0.0 release notes: <https://github.com/wasmCloud/wasmCloud/releases/tag/v2.0.0>
- `crates/wash-runtime` README: <https://github.com/wasmCloud/wasmCloud/blob/main/crates/wash-runtime/README.md>
- `crates/wash-runtime/wit/world.wit`: <https://github.com/wasmCloud/wasmCloud/blob/main/crates/wash-runtime/wit/world.wit>
- `crates/wash/src/cli/host.rs`: <https://github.com/wasmCloud/wasmCloud/blob/main/crates/wash/src/cli/host.rs>
- `proto/wasmcloud/runtime/v2/`: <https://github.com/wasmCloud/wasmCloud/tree/main/proto/wasmcloud/runtime/v2>
- `runtime-operator` README: <https://github.com/wasmCloud/wasmCloud/blob/main/runtime-operator/README.md>
- `runtime-gateway` README: <https://github.com/wasmCloud/wasmCloud/blob/main/runtime-gateway/README.md>
- v1→v2 migration guide: <https://wasmcloud.com/docs/migration>
- v2 FAQ: <https://wasmcloud.com/docs/faq>
- v2 plugins documentation: <https://wasmcloud.com/docs/overview/hosts/plugins>
- Legacy OTP host (last push 2024-10-12): <https://github.com/wasmCloud/wasmcloud-otp>
- wadm (legacy in v2): <https://github.com/wasmCloud/wadm>
