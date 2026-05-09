**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — capability provider model, plugin/service split, and isolation story

# wasmCloud Capability Model

This file documents how wasmCloud satisfies a Wasm component's WIT imports — what fills the role that "capability providers" filled in v1, what isolation guarantees the runtime gives, and how component-to-component calls work. Read [`architecture.md`](./architecture.md) first; it sets up the host topology that this doc plugs into.

## Status snapshot

- v2.1.0 (2026-05-07) is current. The capability story changed substantially in `v2.0.0` (2026-03-22) — what follows describes v2.
- The phrase "capability provider" still appears in older blog posts, conference talks, and the originating brief for this document. As of v2 the term is no longer a runtime construct — what existed in v1 has been split into **host plugins** (in-process) and **service components** (sidecar Wasm components). The v1 over-NATS provider model is retired.

## Brief corrections

- The brief asked for first-party providers including HTTP server, HTTP client, KV (Redis, NATS-KV), blobstore (S3, FS, NATS-Object-Store), messaging (NATS, Kafka), SQL, secrets, lattice-control. **In v2 the supported set is narrower and the implementations are different.** The current built-in set is HTTP, `wasi:keyvalue` (NATS-KV), `wasi:blobstore` (NATS Object Store), `wasi:config` (in-memory + K8s-injected), `wasi:logging` (tracing), `wasmcloud:messaging` (NATS), `wasi:otel`, and `wasmcloud:postgres`. Redis-backed KV, S3-backed blobstore, and Kafka-backed messaging existed as v1 capability providers; in v2 they are not in the built-in set and would be implemented either as custom host plugins or via the legacy "containerized provider" escape hatch.
- The brief described "link definitions" as the runtime binding from a component's import to a provider's export. **Link definitions do not exist in v2.** The equivalent is the `host_interfaces` array on a workload manifest, which declares which WIT interfaces the host must satisfy and what configuration to pass.
- The brief described component-to-component calls as brokered through NATS via wRPC. **In v2, components in the same workload call each other in-process via Wasmtime's component linker.** Cross-workload component-to-component calls have no automatic transport — the component must explicitly use `wasmcloud:messaging` or wRPC and serialize itself.
- "Multi-host load balancing of interface calls" was a real v1 capability. In v2 it does not exist for component-to-component. Workloads are scheduled per host; an HTTP request reaches a workload via `runtime-gateway`, which routes by hostname → workload → host.

## How a component's imports are satisfied

When a workload starts, the host walks the component's WIT imports and binds each one to one of:

1. **A WASIp2 built-in.** `wasi:filesystem`, `wasi:clocks`, `wasi:random`, `wasi:io`, `wasi:sockets`, the `wasi:cli` suite, and `wasi:http` (when an HTTP handler is registered) come from `wasmtime-wasi` and are always available. They are not registered as plugins.
2. **A registered host plugin.** Anything that implements the `wash_runtime::plugin::HostPlugin` trait can register itself with a `HostBuilder` and provide the host side of one or more WIT worlds. The set of registered plugins varies by host configuration.
3. **A peer component in the same workload.** If the workload contains a service or another component that exports the imported interface, the linker wires component A's import to component B's export. No serialization, no host hop.
4. **The deny-all stub.** If none of the above applies, the host installs a deny-all implementation. Calls return an error; the component cannot bypass.

This last point is structurally important. There is no implicit "I'll fall back to the network" — an unsatisfied import is a hard failure. This is the behavior the docs describe as "default-deny."

The binding is performed at workload start, after the operator has resolved any ConfigMap/Secret references and populated `LocalResources.config` and `LocalResources.environment`. There is no dynamic re-binding at call time; if you want different configuration, you restart the workload.

## Built-in plugins

From `crates/wash-runtime/wit/world.wit` and `crates/wash-runtime/src/plugin/`:

| WIT world | Crate path | Backend in `wash host` | Notes |
|---|---|---|---|
| `wasi:config/store@0.2.0-rc.1` | `plugin::wasi_config::DynamicConfig` | In-memory map | Populated from K8s ConfigMaps + Secrets via the operator |
| `wasi:logging/logging@0.1.0-draft` | `plugin::wasi_logging::TracingLogger` | `tracing` crate | Routes to OTel via `wasi:otel` if enabled |
| `wasi:keyvalue/{atomics,batch,store}@0.2.0-draft` | `plugin::wasi_keyvalue::NatsKeyValue` | NATS-KV | Per-component bucket scoping |
| `wasi:blobstore/{types,container,blobstore}@0.2.0-draft` | `plugin::wasi_blobstore::NatsBlobstore` | NATS Object Store | |
| `wasmcloud:messaging@0.2.0` | `plugin::wasmcloud_messaging::NatsMessaging` | NATS subject pub/sub | Always registered when running `wash host` |
| `wasmcloud:postgres@0.1.1-draft` | `plugin::wasmcloud_postgres::WasmcloudPostgres` | deadpool-postgres pool | Optional, gated on `--postgres-url` |
| `wasi:otel@0.2.0-rc.1` | `plugin::wasi_otel` | OpenTelemetry SDK | Optional, gated on `--wasi-otel` |
| `wasi:http/incoming-handler@0.2.0` (export) | `host::http::HttpServer` | hyper + custom router | Registered via `with_http_handler`, not `with_plugin` — implements `HostHandler` |
| `wasi:webgpu` | `plugin::wasi_webgpu` | wgpu | Optional, non-Windows |

A few observations relative to v1:

- The v1 first-party providers list was much wider — HTTP client, Redis, FS-backed blobstore, S3 blobstore, Kafka messaging, SQL (`wasmcloud:sqldb`), lattice-control, secrets (`wasmcloud:secrets` with NATS-KV backend). Most of those have not been ported to v2 host plugins. The ones that survived are the ones with NATS-native backends.
- The v2 model treats NATS-KV/Object-Store as the obvious storage backend because the host already has a NATS connection for the data plane. Redis/S3 backends would need their own connection management; nobody has shipped them yet.
- The `wasmcloud:secrets` interface was retired entirely; see [Secrets](#secrets) below.

## The `HostPlugin` trait

A custom plugin implements one trait. Roughly (paraphrased from the v2 source):

```rust
trait HostPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn add_to_linker(
        &self,
        linker: &mut wasmtime::component::Linker<HostState>,
        config: &PluginConfig,
    ) -> anyhow::Result<()>;
    // lifecycle hooks: init, ready, shutdown...
}
```

The plugin is responsible for calling `linker.instance(...).func_wrap(...)` (or the WIT-bindgen-generated `add_to_linker` helper) for each WIT function it implements. The runtime guarantees:

- The plugin's functions are called from a Wasmtime guest call frame, with access to the component's `Store<HostState>`.
- Per-component configuration is available via `PluginConfig` (sourced from `LocalResources.config`).
- The plugin may hold per-host state (e.g. a connection pool) and per-component state (e.g. KV-namespace prefix).

The trait is not stable — it is in a `0.x` runtime crate and is expected to evolve. But the shape (host-side WIT impl + linker registration + per-component config) is established.

## Service components

A service is a long-running Wasm component bundled into the same workload as its consumers. The use cases the v2 docs cite:

- Cron jobs and scheduled tasks
- Connection pools that don't fit the request/response component model
- TCP servers that need to listen on a workload-private port
- In-memory caches shared between components in the workload

A service runs on the same host as the rest of the workload, can listen on a TCP socket, and is restartable up to `max_restarts` times. Services are themselves Wasm — they are sandboxed by Wasmtime exactly like components, and they get their imports satisfied by the same plugin set.

The split between **host plugin** (one implementation, many workloads) and **service component** (per-workload, runs as a peer) is the v2 answer to the v1 capability provider question. Roughly:

- Need a shared, performance-critical capability that many workloads use? → **host plugin**
- Need a workload-specific long-running thing? → **service component**
- Have an existing v1 provider you don't want to rewrite? → containerize it and have your component talk to it via wRPC. The migration guide marks this as the minimal-change path.

## Component-to-component calls

In v2:

- **Same workload.** Calls are resolved through Wasmtime's component linker at workload-instantiate time. The call is a normal function call into another component instance; argument lifting/lowering uses the canonical ABI. No serialization, no NATS, no network. Documented at ~30,000 RPS in the v2 migration guide.
- **Different workload.** No automatic transport. Components import `wasmcloud:messaging` (or some other explicit interface) and serialize themselves. The v2 FAQ describes this as "intentionally more explicit" and quotes ~5,000 RPS for the messaging path.

This is a major simplification from v1, which advertised "actor-to-actor calls over the lattice" — a transparent NATS-mediated wRPC call between actors regardless of host. The v2 stance is that the transparent distributed-RPC sugar was paying ~6× overhead on the hot path and was worth retiring.

If you do want cross-workload typed RPC, wRPC at <https://github.com/bytecodealliance/wrpc> is still the path. But you wire it up yourself; the runtime does not arrange it.

## Authority and isolation

Each component runs in its own Wasmtime `Store<HostState>`. Cross-component memory is impossible at the engine level — there is no mechanism by which one component's linear memory is visible to another. This is a Wasmtime guarantee, not a wasmCloud guarantee.

Cross-component authority comes from two sources:

1. **The workload manifest's `host_interfaces`.** This declares which WIT interfaces the host must satisfy for this workload's components. If `wasi:keyvalue` isn't in the list, no component in the workload can use it — the deny-all stub is wired in.
2. **The component-to-component linkage.** If component A imports `my:thing/foo` and component B exports it, the linker wires the import. If no peer exports it and no plugin satisfies it, deny-all.

There is no separate ACL layer. Authority is the union of the WIT-level types and the manifest-declared `host_interfaces`. This is approximately the "capability-as-interface-import" stance familiar from the WASI design discussions.

The host plugin layer adds per-component configuration, but it does *not* mediate per-call authorization. If a component imports `wasi:keyvalue/store`, every call goes through. The plugin can constrain per-component scope at registration time (e.g. namespace the KV prefix by component name), but this is the plugin's responsibility, not the runtime's.

`LocalResources.allowed_hosts` is the explicit allowlist for `wasi:http/outgoing-handler` — an outbound HTTP request to a host not in the list is denied at the plugin level. This is a coarse-grained network-egress capability, not a per-request policy.

### Multi-host load balancing

In v2, **the runtime does not load-balance interface calls across hosts**. A workload runs on a single host (chosen by the operator at scheduling time according to the workload's `hostSelector`); all calls to that workload land on that host. If you want a workload to be replicated, you set `replicas: N` on its `WorkloadDeployment` and the operator schedules N independent workloads — each is its own running instance, and external clients reach them via `runtime-gateway` HTTP routing or via whatever messaging fabric the consumer uses.

This is a step back from v1, which routed `wasmbus.<lattice>.actor.<actor-id>` messages to whichever host had a live actor instance via NATS queue groups. The v2 stance is that this transparent multi-host routing was both expensive and confusing — it conflated scaling, placement, and call routing into one mechanism. v2 separates them: scaling is handled by `WorkloadDeployment` replicas, placement by `hostSelector`, and routing by an explicit ingress (`runtime-gateway` for HTTP, the consumer for messaging).

### Sandboxing the host plugin itself

A host plugin runs in the host process. It is **not** sandboxed. It has full host-process privileges. This is a deliberate trust boundary: plugins are first-party code (or carefully audited third-party code) and are part of the trusted computing base. Components in workloads are not trusted; plugins are.

This is the same model as Wasmtime's own host functions. The capability-secure boundary is between the component and the host, not within the host.

## Secrets

The v1 `wasmcloud:secrets` interface was a custom envelope-encryption protocol. Components imported `wasmcloud:secrets/store/secret`, the host fetched encrypted secret material from a NATS-KV-backed secrets store, the host decrypted with a per-host xkey, and component code received decrypted secret values via the WIT interface. The design avoided putting key material in the component's linear memory.

In v2 this is **gone**. The replacement is:

- Secrets live as Kubernetes `Secret` objects.
- A workload manifest references them via `localResources.environment.secretFrom` (and similar fields on components).
- The operator reads the Secret values at scheduling time and passes them to the host as part of the `WorkloadStartRequest` over NATS.
- The host injects them into the component's `WasiCtxBuilder` env map (visible via `wasi:cli/environment.get-environment`) or into `wasi:config/store` lookups.
- Components see them as ordinary config values via `wasi:config`.

What was lost in this transition:

- The xkey-based encryption-at-rest story is gone. Secrets are stored in K8s etcd with whatever encryption the cluster operator has configured (which is "none" in default kind/k3s clusters).
- The "no key material in component memory" guarantee is gone. Secret values are now part of the component's environment dictionary.
- Per-component scoping is now manifest-level rather than enforced by the secrets store. The component can read every config key the manifest gave it.

What was gained:

- Standard K8s tooling (kubectl, sealed-secrets, external-secrets-operator) works.
- One less custom subsystem to operate.

The v2 docs are explicit that this trade was made for operational simplicity. For a system that doesn't have a Kubernetes orchestrator to delegate to (such as Myrhiza), the v1 model is the more relevant precedent.

## Multi-tenancy via lattices

In v1, a "lattice" was a NATS topic-prefix tenancy boundary: all hosts subscribed to `wasmbus.<lattice-id>.>` saw each other; hosts in different lattices were invisible to each other even on the same NATS deployment. wadm scoped state per lattice. This was the primary multi-tenancy primitive.

In v2 the concept is gone in this form. What exists:

- **Host group** — a label on the `Host` CRD (default `default`). Workloads pick host groups via `hostSelector`. This is closer to "node pool" than "tenant."
- **Environment label** — typically the K8s namespace. Used at scheduling time to enforce tenant isolation.
- **K8s namespaces** — the actual tenancy boundary. Network policies, RBAC, and resource quotas all hang off these.

So multi-tenancy is now whatever K8s gives you, plus a label on the host. The lattice abstraction has been replaced by K8s primitives.

## Implications for Myrhiza

- **The plugin trait is the right shape for kernel-mediated host imports.** A `HostPlugin` impl with a `linker.instance(...).func_wrap(...)` body is exactly how Myrhiza's kernel will register host functions for app components. The v1 over-NATS provider model is a design Myrhiza should *not* copy; the v2 in-process plugin model is.
- **Default-deny on unsatisfied imports** is the right stance for Myrhiza. wasmCloud's deny-all stub for unmatched imports is the precedent.
- **Per-component authority via WIT imports + manifest-declared `host_interfaces`** is approximately the right shape. Myrhiza's app manifests will need an equivalent declaration of which kernel capabilities the app's components consume; the kernel binds only what the manifest declared.
- **The split between host plugin (TCB) and service component (sandboxed peer)** is a useful pattern. Some Myrhiza system code will naturally be host-side (network, storage); other system code will be component-side (state-apply, behaviors). The wasmCloud plugin/service split is a worked example of how to slice that.
- **Same-bundle component-to-component calls in-process via the linker** is the right default. Myrhiza apps will frequently have multiple components (state-apply, state-propose, interaction); they should call each other through the linker, not through a P2P transport. The v2 stance that "transparent distributed RPC is a footgun" is a strong negative result Myrhiza should heed.
- **`wasmcloud:secrets` v1 design (envelope encryption, no key material in component memory)** is a more relevant precedent for Myrhiza than the v2 retreat to K8s Secrets. Myrhiza has no orchestrator to delegate to. The v1 architecture — encrypted-at-rest secrets, host-mediated decryption, per-component scoping enforced at the host boundary — is closer to what Myrhiza needs. See [`history.md`](./history.md) for the v1 design.
- **Multi-host load balancing of interface calls is an anti-pattern.** v2 explicitly removed it because it conflated scaling, placement, and routing. Myrhiza should keep these separate from the start.
- **Lattice → K8s namespace** is a reminder that even strong custom multi-tenancy abstractions tend to get replaced by the host environment's primitives. Myrhiza's tenancy boundary will need to be peer-native (some kind of group or topic identity) since there is no host environment to delegate to.
- **The v1 `link-defs` / runtime-mutable binding model is precedent for capability revocation.** Myrhiza will need this even if wasmCloud retired it: the kernel must be able to revoke a capability handle without restarting the app. wasmCloud's v1 link-table is one way to think about that table.

## Cross-references

- [`architecture.md`](./architecture.md) — host topology and control plane
- [`wrpc.md`](./wrpc.md) — wRPC transport
- [`interfaces.md`](./interfaces.md) — WIT package families and the interface contracts
- [`tooling.md`](./tooling.md) — `wash` developer experience
- [`history.md`](./history.md) — v1 capability providers, link defs, `wasmcloud:secrets`
- [`comparisons.md`](./comparisons.md) — how this stacks against other component-host runtimes
- [`critiques.md`](./critiques.md), [`open-problems.md`](./open-problems.md), [`lessons.md`](./lessons.md) — analytical neighbors

Prior-art neighbors:

- [WASM Component Model](../wasm-component-model/) — what plugins implement against
- [Wasmtime](../wasm-component-model/wasmtime.md) — engine, linker, store isolation guarantees
- [Iroh](../iroh/) — peer-to-peer transport (vs NATS data plane)
- [Holochain](../holochain/) — agent-centric capability + state model
- [Spritely OCapN](../spritely-ocapn/) — capability-secure cross-host RPC, stronger isolation guarantees than wRPC

## Sources

- v2 plugins doc: <https://wasmcloud.com/docs/overview/hosts/plugins>
- v2 services doc: <https://wasmcloud.com/docs/overview/workloads/services>
- v2 migration guide: <https://wasmcloud.com/docs/migration>
- v2 FAQ ("How do distributed applications work in wasmCloud v2?"): <https://wasmcloud.com/docs/faq>
- `crates/wash-runtime/src/plugin/`: <https://github.com/wasmCloud/wasmCloud/tree/main/crates/wash-runtime/src/plugin>
- `crates/wash-runtime/wit/world.wit`: <https://github.com/wasmCloud/wasmCloud/blob/main/crates/wash-runtime/wit/world.wit>
- `crates/wash/src/cli/host.rs` (built-in plugin registration): <https://github.com/wasmCloud/wasmCloud/blob/main/crates/wash/src/cli/host.rs>
- v1 secrets RFC and `wasmcloud:secrets` WIT (legacy): <https://github.com/wasmCloud/wasmCloud/tree/main/wit> and <https://wasmcloud.com/blog>
- wRPC: <https://github.com/bytecodealliance/wrpc>
- `wadm` (legacy in v2): <https://github.com/wasmCloud/wadm>
