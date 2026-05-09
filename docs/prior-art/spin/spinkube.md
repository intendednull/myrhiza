**Date:** 2026-05-09
**Status:** active
**Subject:** SpinKube — CNCF Sandbox Kubernetes operator + CRDs for Spin apps

# SpinKube

[SpinKube](https://www.spinkube.dev/) is a CNCF Sandbox project (accepted **2025-01-21**) that runs Spin applications natively on Kubernetes. It is the canonical "Spin-on-K8s" stack: a controller plus a containerd shim plus a node-side runtime-class manager, originally contributed jointly by Microsoft, SUSE, Liquid Reply, and Fermyon. Companion files in this folder: [glossary](./glossary.md), [architecture](./architecture.md), [triggers-and-components](./triggers-and-components.md), [SDKs and tooling](./sdks-and-tooling.md), [governance](./governance.md), [comparisons](./comparisons.md), [lessons](./lessons.md), [open-problems](./open-problems.md). Cross-prior-art neighbour: [wasmCloud architecture (v2/K8s)](../wasmcloud/architecture.md).

## What SpinKube is

A K8s-native execution path for Spin apps. Instead of running a long-lived `spin up` daemon, you `kubectl apply -f spinapp.yaml`; the cluster pulls the OCI-packaged Spin app, schedules a pod onto a node with the Spin runtime class, and the WASM workload runs inside a containerd shim. No traditional container image, no Linux userspace per app — just the WASM artifact and the shared shim.

## Components

| Component | Repo | Role | Verified version (2026-05-09) |
|---|---|---|---|
| `spin-operator` | [github.com/spinframework/spin-operator](https://github.com/spinframework/spin-operator) | Kubernetes operator (Go, Kubebuilder); reconciles `SpinApp` CRs into `Deployment` + `Service` | v0.6.1 (2025-07-09) |
| `containerd-shim-spin` | [github.com/spinframework/containerd-shim-spin](https://github.com/spinframework/containerd-shim-spin) | containerd shim (Rust, on `runwasi`); executes Spin WASM artifacts as containerd workloads | v0.24.0 (April 2026), pairs with Spin v3.6.3 |
| `runtime-class-manager` | [github.com/spinframework/runtime-class-manager](https://github.com/spinframework/runtime-class-manager) (formerly KWasm) | Node-side operator: installs the shim binary on each node, registers the `RuntimeClass`, manages lifecycle | active |

Both repos migrated from the `fermyon/` GitHub org to `spinframework/` during the project's CNCF onboarding. The shim leans on [`runwasi`](https://github.com/containerd/runwasi) (a containerd subproject) for the WASI-host plumbing — the same library wasmCloud's K8s path uses.

## CRDs

Two CRDs, both `spinoperator.dev/v1alpha1`:

- **`SpinApp`** — the workload. Fields include `image` (OCI ref of the Spin app), `replicas`, `executor` (a reference to a `SpinAppExecutor`), `variables`, `runtimeConfig`, optional `serviceAnnotations` / `podLabels`.
- **`SpinAppExecutor`** — execution policy. Selects which runtime class to use (e.g. `wasmtime-spin-v2`), what shim, and any per-cluster overrides. Lets a cluster offer multiple Spin runtimes side-by-side.

Sketch:
```yaml
apiVersion: spinoperator.dev/v1alpha1
kind: SpinApp
metadata:
  name: hello
spec:
  image: ghcr.io/example/hello:0.1.0
  replicas: 3
  executor:
    name: containerd-shim-spin
---
apiVersion: spinoperator.dev/v1alpha1
kind: SpinAppExecutor
metadata:
  name: containerd-shim-spin
spec:
  createDeployment: true
  deploymentConfig:
    runtimeClassName: wasmtime-spin-v2
```

`SpinApp` CRs can be hand-written or generated from an existing `spin.toml` via the `spin kube scaffold` plugin command.

## Architecture — the apply-to-pod path

1. `kubectl apply -f spinapp.yaml` — `SpinApp` CR enters etcd.
2. **`spin-operator`** (controller-runtime reconciler) sees the new CR, looks up the referenced `SpinAppExecutor`, and synthesizes a standard K8s `Deployment` + `Service`. The `Deployment`'s pod template sets `runtimeClassName: wasmtime-spin-v2` and uses the SpinApp's OCI image.
3. K8s scheduler places the pod on a node where the shim has been registered.
4. On that node, **`runtime-class-manager`** has previously (a) installed `containerd-shim-spin` into `/usr/local/bin`, (b) edited `/etc/containerd/config.toml` to register the shim, and (c) created the `RuntimeClass` resource.
5. **containerd** invokes `containerd-shim-spin` for the pod. The shim pulls the OCI artifact, extracts the Spin app, and starts a Wasmtime engine inside the shim process. No Linux userspace container is created — the "pod" is a thin wrapper around the shim.
6. The shim listens on the standard pod port; HTTP traffic flows through normal K8s networking (Service → kube-proxy → pod IP → shim).

Compared to a regular container: lower memory floor (no per-pod userspace), faster start (no image extraction beyond the WASM artifact), but a narrower I/O surface — the app sees only what its Spin manifest declares (HTTP, KV, SQLite, etc.), not arbitrary syscalls.

## Adoption

Public references in 2026 are mostly demos and tutorials (AKS WASI node pools, Linode, Rancher Desktop, MicroK8s walkthroughs). Production case studies are sparse and mostly Fermyon-adjacent. The CNCF blog post "Exposing Spin apps on SpinKube with GatewayAPI" (2026-02-26) suggests live-traffic deployments exist but have not generated independent post-mortems. Treat SpinKube as "in production at a few sites, on the order of dozens" — short of the bar typically expected for CNCF Incubating.

## Relationship to wasmCloud's K8s pivot

wasmCloud v2 (2026-03) reoriented around K8s CRDs (`Host`, `Workload`) — see [`../wasmcloud/architecture.md`](../wasmcloud/architecture.md). SpinKube and wasmCloud-on-K8s now overlap heavily:

| Axis | SpinKube | wasmCloud v2 |
|---|---|---|
| Execution unit | one Spin app per pod, multi-component within | one workload per pod, components in lattice |
| Runtime | `containerd-shim-spin` on `runwasi` | `runtime-operator` + custom Wasmtime host |
| Component composition | `wac` build-time | wRPC late-binding across pods |
| Capabilities | declared in `spin.toml` | capability providers as separate workloads |
| Networking | standard K8s Service | NATS or wRPC mesh |

Where they're competitive: simple HTTP services (both can run a "hello world" component as a pod). Where they diverge: wasmCloud retains lattice/late-binding (heavier, more flexible); SpinKube is pure manifest-driven (lighter, more K8s-idiomatic). They share `runwasi`, so a node can theoretically host both shims — `runtime-class-manager` even acknowledges this and can manage the WasmEdge shim alongside Spin's.

## CNCF context

- **Sandbox accepted** 2025-01-21 ([cncf/sandbox#90](https://github.com/cncf/sandbox/issues/90)).
- The Spin runtime itself is also a separate CNCF Sandbox project ([cncf/sandbox#116](https://github.com/cncf/sandbox/issues/116)).
- Path to Incubating requires "successful production use by a small number of users with a healthy contributor pool." As of 2026-05, public production references and contributor diversity remain limited; no Incubating proposal has been filed.
- For comparison: wasmCloud reached Incubating in 2024 — see [`../wasmcloud/governance.md`](../wasmcloud/governance.md).

## Akamai acquisition implications

Fermyon was acquired by **Akamai** on **2025-12-01** (co-founders Matt Butcher and Radu Matei joined the Akamai Cloud Technology Group). The press release and Akamai's developer blog explicitly committed to continuing the Spin and SpinKube CNCF projects and ongoing CNCF / Bytecode Alliance participation. The first six months under Akamai (Dec 2025–May 2026) show that commitment holding: Spin v4.0.0 shipped 2026-04-20, the shim and operator both saw releases, and no maintainers exited publicly. The longer-term risk is strategic — Akamai's investment thesis is edge-FaaS, which favours the Spin runtime over SpinKube specifically, and SpinKube's K8s-operator story is less central to Akamai's edge platform than to a generic enterprise platform team.

## Implications for Myrhiza

- **Myrhiza is P2P, not K8s-deployed.** SpinKube's runtime topology — operator, CRDs, containerd shim, scheduler — is not a model Myrhiza wants. Our "scheduling" is gossip-driven peer placement, not a centralized control plane.
- **The OCI-artifact-as-app semantic carries over cleanly.** SpinKube proves that a Spin app fits in a single OCI image and can be fetched by hash from a standard registry. Myrhiza apps should publish the same way; the difference is the puller (peer kernel vs. containerd).
- **`SpinAppExecutor` is a useful pattern for runtime-class indirection.** When Myrhiza eventually supports multiple host configurations (e.g. trusted-only vs. sandboxed-strict), an executor-style indirection between the app and the runtime selection is worth borrowing.
- **`runwasi` is shared infrastructure to keep an eye on.** Both SpinKube and wasmCloud-on-K8s lean on it. If Myrhiza ever offers a "run on K8s as a peer" mode (unlikely but plausible for institutional peers), `runwasi` is the integration point.

## Sources

- SpinKube site: [spinkube.dev](https://www.spinkube.dev/)
- SpinKube docs (architecture): [spinkube.dev/docs/topics/architecture](https://www.spinkube.dev/docs/topics/architecture/)
- CNCF project page: [cncf.io/projects/spinkube](https://www.cncf.io/projects/spinkube/)
- CNCF Sandbox acceptance issue: [github.com/cncf/sandbox/issues/90](https://github.com/cncf/sandbox/issues/90)
- spin-operator: [github.com/spinframework/spin-operator](https://github.com/spinframework/spin-operator) (v0.6.1, 2025-07-09)
- containerd-shim-spin: [github.com/spinframework/containerd-shim-spin](https://github.com/spinframework/containerd-shim-spin) (v0.24.0, April 2026)
- runtime-class-manager: [github.com/spinframework/runtime-class-manager](https://github.com/spinframework/runtime-class-manager)
- runwasi: [github.com/containerd/runwasi](https://github.com/containerd/runwasi)
- "Exposing Spin apps on SpinKube with GatewayAPI" (CNCF blog, 2026-02-26): [cncf.io/blog/2026/02/26/exposing-spin-apps-on-spinkube-with-gatewayapi](https://www.cncf.io/blog/2026/02/26/exposing-spin-apps-on-spinkube-with-gatewayapi/)
- "Introducing SpinKube" (Fermyon, 2024): [fermyon.com/blog/introducing-spinkube-fermyon-platform-for-k8s](https://www.fermyon.com/blog/introducing-spinkube-fermyon-platform-for-k8s)
- Palark CNCF Sandbox January 2025 review: [palark.com/blog/cncf-sandbox-2025-jan](https://palark.com/blog/cncf-sandbox-2025-jan/)
- Akamai acquires Fermyon (2025-12-01): [akamai.com/newsroom/press-release](https://www.akamai.com/newsroom/press-release/akamai-announces-acquisition-of-function-as-a-service-company-fermyon), [globenewswire press release](https://www.globenewswire.com/news-release/2025/12/01/3196978/0/en/Akamai-Technologies-Announces-Acquisition-of-Function-as-a-Service-Company-Fermyon.html)
