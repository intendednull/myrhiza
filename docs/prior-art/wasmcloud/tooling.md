**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — developer and operator tooling (wash, wadm, washboard, SDKs, CI)

# wasmCloud Tooling

This file is a snapshot of wasmCloud's developer-tooling surface, written so Myrhiza spec authors can compare against our own app-developer experience. Companion files in this folder cover [architecture](./architecture.md), the [capability model](./capability-model.md), [wRPC](./wrpc.md), [interfaces](./interfaces.md), [governance](./governance.md), [history](./history.md), [commercial layer](./commercial.md), [comparisons](./comparisons.md), [critiques](./critiques.md), [open problems](./open-problems.md), and [lessons](./lessons.md). Cross-prior-art neighbours: [WASM CM tooling](../wasm-component-model/tooling.md), [Holochain dev tooling](../holochain/), [Iroh CLI tooling](../iroh/).

## Major Caveat: v1 → v2 Tooling Shift

Anyone reading wasmCloud documentation in 2026 must navigate a transition that is currently in flight:

- **wasmCloud v1.x** was a NATS-lattice runtime. Tooling: `wash` (with ~15 subcommands including `up`, `app`, `claims`, `keys`, `reg`, `spy`, `ctl`, `ctx`), `wadm` (OAM-flavoured manifests), and `washboard` (web UI).
- **wasmCloud v2.0** (released 2026-03-22; current stable v2.1.0 2026-05-07) reorients around Kubernetes CRDs. The runtime ships as `Host` and `Workload` custom resources reconciled by `runtime-operator` (Go-based K8s operator), with HTTP routed through a Go-based `runtime-gateway`. The `wash` CLI is substantially trimmed.

The brief that produced this document quoted v1 subcommands. Where the v1 surface differs materially from v2, both are described and the difference is flagged inline.

Versions verified on 2026-05-09:

| Component | Version | Released | Source |
|---|---|---|---|
| `wasmCloud` host | v2.1.0 | 2026-05-07 | [GH release](https://github.com/wasmCloud/wasmCloud/releases/tag/v2.1.0) |
| `wash` (v2 line, in-monorepo) | v2.0.0-rc.7 | 2026-02-19 | [GH release](https://github.com/wasmCloud/wasmCloud/releases/tag/v2.0.0-rc.7) |
| `wash` crate on crates.io (the published v1 binary) | 0.43.0 | 2026-02-04 | [crates.io API](https://crates.io/api/v1/crates/wash) |
| `wash-cli` (older crate name; superseded by `wash`) | 0.39.0 | 2025-02-24 | crates.io |
| `wadm` | v0.21.1 | 2026-01-29 | [GH release](https://github.com/wasmCloud/wadm/releases/tag/v0.21.1) |

Note on staleness: `wash` and `wadm` v1-line releases are 3+ months old. The work has shifted into the `wasmCloud/wasmCloud` monorepo and the v2 RCs; `wadm` may be subsumed by the operator pattern entirely (see below).

## `wash` — the Wasm Shell

GitHub: [wasmCloud/wasmCloud/tree/main/crates/wash](https://github.com/wasmCloud/wasmCloud/tree/main/crates/wash) (v2). The v1 archive lives at [wasmCloud/wash-archive](https://github.com/wasmCloud/wash-archive) (read-only).

Install (v2):
```bash
curl -fsSL https://raw.githubusercontent.com/wasmcloud/wasmCloud/refs/heads/main/install.sh | bash
# or
cargo install --path crates/wash
```

### v2 subcommands (verified against `crates/wash/README.md`)

| Command | Description |
|---|---|
| `wash build` | Build a Wasm component. |
| `wash config` | View and manage hierarchical wash config. |
| `wash completion` | Generate shell completion scripts. |
| `wash dev` | Hot-reload dev loop for a component. |
| `wash host` | Run the local node as a wasmCloud host. |
| `wash new` | Scaffold a project from a git template. |
| `wash oci` | Push/pull components to/from OCI. |
| `wash update` | Self-update the binary. |
| `wash wit` | Manage WIT dependencies. |

### v1 subcommands the brief expected (still real, but legacy)

`wash up` (start a host + NATS + wadm), `wash app deploy/undeploy` (push wadm manifests), `wash inspect` (component metadata), `wash claims` and `wash keys` (Ed25519 signing for actors), `wash reg push/pull` (OCI ops, since renamed to `wash oci`), `wash spy` (lattice introspection), `wash ctl` (control-interface RPCs), `wash ctx` (context switching), `wash call` (invoke a component), `wash drain` (cache cleanup). All present in `wash-cli` 0.39.0 / wash v0.42.0 from 2025-05-29; mostly removed or folded into operator-driven flows in v2.

### What's replaced/extended

- v1 `wash up` (in-process NATS + wadm + host) is replaced in v2 by a Helm chart (`charts/runtime-operator`) that runs `Host` and `Workload` CRDs on Kubernetes. Local dev uses `kind` + `make kind-setup` per the v2 README.
- v1 `wash claims` / `wash keys` (nkeys-based actor signing) are gone in v2. Component identity now flows through OCI image references and Kubernetes service accounts.
- v1 `wash reg` is `wash oci` in v2.
- v1 `wash app deploy` (wadm) becomes `kubectl apply -f workload.yaml` in v2.

This is a substantial shrinkage of the CLI's surface area — the v1 wash was a wasmCloud-platform manager; the v2 wash is a component-developer tool. Operator concerns moved into Kubernetes. See [history](./history.md) for the rationale.

## `wash dev` — the developer inner loop

The dev loop is what `wash` is centrally about in v2. `wash dev` watches the source tree, invokes `wasm-tools component new` / `cargo component build` (or the language-appropriate equivalent for Go/TypeScript) on change, and re-instantiates the component in an in-process Wasmtime runtime with auto-wired capability providers (HTTP server on a local port, in-memory keyvalue, etc.).

How it differs from `cargo component build`:
- `cargo component build` produces a `.wasm` artifact and stops.
- `wash dev` adds: file-watching, automatic capability-provider plumbing, a one-line WADM-ish manifest, and a built-in HTTP listener so an HTTP component is reachable at `localhost:8000` immediately.
- It does not invoke the v1 lattice. It runs a host in-process.

In v2 the team has been pulling out file-watching from `wash dev` (see PR #249, "Remove 'doctor' and 'dev' file watching") in favour of a leaner runtime; expect the dev-loop UX to keep moving.

## `wadm` — wasmCloud Application Deployment Manager

GitHub: [wasmCloud/wadm](https://github.com/wasmCloud/wadm). Manifest schema: [Open Application Model](https://oam.dev/) (`apiVersion: core.oam.dev/v1beta1`, `kind: Application`).

A wadm manifest declares **components** (Wasm components and **capability providers** — the v1 term for native plugins like `httpserver` or `keyvalue-redis`) and **traits** that bind them. The reconciler watches wasmCloud CloudEvents and issues control-interface commands until current state matches desired state.

Example manifest fragment (verbatim from wadm README, 2026-05-09):
```yaml
apiVersion: core.oam.dev/v1beta1
kind: Application
metadata:
  name: hello-world
spec:
  components:
    - name: http-component
      type: component
      properties:
        image: ghcr.io/wasmcloud/components/http-hello-world-rust:0.1.0
      traits:
        - type: spreadscaler
          properties:
            instances: 1
    - name: httpserver
      type: capability
      properties:
        image: ghcr.io/wasmcloud/http-server:0.22.0
      traits:
        - type: link
          properties:
            target: http-component
            namespace: wasi
            package: http
            interfaces: [incoming-handler]
```

Driving deploys:
```bash
wash app deploy hello.yaml          # v1 + transitional v2
wash app undeploy hello-world
```

Status: `wadm` repo is alive (commits in early May 2026) but its release cadence has slowed (2 months between v0.21.0 and v0.21.1) and the v2 architecture's CRD-and-operator approach overlaps with what wadm does. The relationship between `wadm` and the new `runtime-operator` is not fully resolved in public docs as of 2026-05-09.

## `washboard` — the web UI

`washboard` was the v1 web UI bundled by `wash up --experimental` (port 3030). It read NATS lattice CloudEvents and rendered host/component status. Implementation history: originally a Phoenix/Elixir app inside `wasmcloud-otp`; reimplemented in TypeScript and now lives in [wasmCloud/typescript](https://github.com/wasmCloud/typescript). v2 community-meeting notes from 2025-04-23 list "automatic washboard standup with `wash dev`" as a roadmap item — i.e. it is in transition rather than abandoned, but currently bit-rotted.

## TypeScript SDK / templates

GitHub: [wasmCloud/typescript](https://github.com/wasmCloud/typescript) (active, last updated 2026-05-08, only 5 stars but it is a recent split-out from the monorepo). The repo is **template-and-example oriented** rather than a runtime SDK — components are scaffolded with `wash new https://github.com/wasmCloud/typescript.git --subfolder templates/<name>`. Templates include `http-hello-world-hono`, `http-hello-world-fetch`, `http-client`, `http-handler-hono`, `http-blobstore-handler-hono`, `http-kv-handler-hono`, `service-tcp-echo`. Also publishes `@wasmcloud/lattice-client-core` and `@wasmcloud/lattice-client-react` to npm (latest 0.5.10, 2025-10-30; ~75 downloads/month — small).

## Rust SDK

The guest-side Rust story rides on `wit-bindgen` plus the `wasi:*` interfaces (`wasi:http`, `wasi:keyvalue`, `wasi:blobstore`, `wasi:logging`, etc.). There is no separate "wasmcloud-rs" SDK at the guest-component layer in v2 — the guest writes against WASI interfaces and Wasmtime hosts them. The host-side runtime crates are `wasmcloud-runtime`, `wasmcloud-host` and friends inside the monorepo's `crates/`.

## OCI as the bundle registry

Every component artifact moves through OCI. `ghcr.io/wasmcloud/components/*` is the canonical home for first-party components; `ghcr.io/wasmcloud/http-server`, `ghcr.io/wasmcloud/keyvalue-redis`, etc. for capability providers. `wash oci push/pull` wraps `oci-distribution`. Component tags are version-pinned (`:0.1.0`), no content-addressed digests in the wadm manifests by convention (which is a security hazard the v2 operator will need to address — there is no enforced digest pinning).

## CI/CD integration

Pattern in practice (per cosmonic-labs/setup-wash-action and various wasmCloud examples):

```yaml
- uses: cosmonic/setup-wash-action@main
- run: wash build
- run: wash oci push ghcr.io/myorg/my-component:${{ github.sha }} ./build/my_component_s.wasm
- run: kubectl apply -f deploy/workload.yaml          # v2
# v1:
# - run: wash app deploy --replace deploy/manifest.yaml
```

GitOps with v1 wadm: a manifest in git, `wash app deploy` from CI, wadm reconciles. The v2 story collapses this into standard `kubectl apply` + Argo/Flux, because `Workload` is just a CRD.

## Implications for Myrhiza

Patterns worth borrowing:

- **`wash dev`-style inner loop.** A single-command "edit → rebuild Wasm → re-instantiate in-process → live capability wiring" loop is the right developer-experience target. Myrhiza's analogue should re-instantiate state-apply / state-propose components against an in-memory event log; for `interaction` components, hot-swap the running instance without losing local UI state.
- **Declarative manifest as the unit of deploy.** Whether OAM (wadm) or CRD (v2), the lesson holds: a developer should describe an app as a list of components plus their links, not as imperative "start this, link that" commands. Myrhiza app bundles already trend in this direction; lock it in.
- **OCI as the artifact registry.** Standard, well-tooled, content-addressable. Myrhiza should treat OCI registries as a first-class transport for app bundles, with mandatory digest pinning (a gap in wasmCloud's wadm).
- **Templates over an SDK.** The `wash new` + git-template pattern is lighter weight than a heavyweight SDK and lets per-language ergonomics evolve independently. Myrhiza should publish profile-specific templates (state-apply skeleton, interaction skeleton) rather than a monolithic SDK.

Anti-patterns to avoid:

- **Tooling churn from architectural pivots.** v1 → v2 broke the CLI surface twice in three years (2023 monorepo move, 2026 K8s pivot). Adopters who built CI on `wash claims` and `wash spy` get to rewrite. Myrhiza must treat its CLI surface as ABI: deprecation cycles, not removals.
- **Single central NATS.** v1 wasmCloud's lattice required a NATS deployment as a hard dependency. That centralised both the control plane and the data plane on a single broker, which is the opposite of what a P2P runtime wants. v2 has moved to per-cluster NATS-as-implementation-detail; the lesson is to never let an op tool become the steward of a single global broker.
- **Single-vendor-driven governance.** Cosmonic's stewardship of wasmCloud means adopters' tooling roadmap is set by Cosmonic's commercial priorities (e.g. the 2026 pivot toward Cosmonic Control / MCP). When the steward pivots, the tooling pivots. See [governance](./governance.md) and [commercial](./commercial.md). Myrhiza should keep operator tooling in a multi-implementer position from day one.
- **Capability-provider sprawl without a stable ABI.** wasmCloud's `httpserver`, `keyvalue-redis`, `blobstore-fs` providers each evolved their own configuration surface; manifests are full of provider-specific knobs that are not part of WIT. Myrhiza's host imports should be the entire surface — no sidecar config schemas.
