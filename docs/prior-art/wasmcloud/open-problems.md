**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — open problems at the runtime layer, with Myrhiza disposition

# Open problems

The set of things wasmCloud hasn't solved (or has solved in a way Myrhiza can't adopt). Each entry: what the problem is, why it matters, what Myrhiza does about it.

Sources cited inline; URLs visited 2026-05-09.

## 1. Determinism is not a wasmCloud concern

**Problem.** wasmCloud is a live-only runtime — there is no replay, no event log, no consensus underneath. Component invocation is a fire-and-forget RPC over wRPC; the runtime makes no claims about determinism of the components themselves.

**Why it matters.** Myrhiza's `state-apply` profile is exactly the workload where determinism is *load-bearing*: cross-peer convergence depends on `state-apply(prior, event)` producing bit-identical state on every peer. wasmCloud has no precedent we can copy here.

**Myrhiza disposition.** Look to Agoric SwingSet, not wasmCloud, for the `state-apply` model. The kernel wraps `state-apply` in a deterministic call envelope (zero wall clock, zero entropy, zero non-deterministic host imports) — see [`../wasm-component-model/open-problems.md`](../wasm-component-model/open-problems.md) for inherited Wasmtime non-determinism. Treat wasmCloud as the *non-deterministic profiles* (interaction, behavior) precedent, not the state-apply precedent.

## 2. Cross-host wRPC failure semantics

**Problem.** When a wRPC call's target host dies mid-call, what happens? Lattice config says component A's `wasi:keyvalue/store.get` is satisfied by provider P on host H₂. If H₂ dies after the request leaves H₁ but before P returns, the caller gets… a NATS timeout (v1) or a K8s pod-evicted error (v2). The wRPC interface itself is interface-typed — there's no transactional / at-most-once / at-least-once contract baked in.

**Why it matters.** Myrhiza calls cross peers, not cross hosts. Peer churn is dramatically higher than datacenter host churn. If Myrhiza inherits wasmCloud's "best-effort, raise on failure" wRPC semantic, every component author has to reason about partial failure.

**Myrhiza disposition.** Capability-call semantics need to be in the spec, not implicit. Document at-most-once vs at-least-once vs idempotent per WIT-method, similar to how WASI specifies error-result types. See the Myrhiza spec for capability-RPC semantics (TBD).

## 3. Capability provider as in-process vs out-of-process

**Problem.** wasmCloud v1 capability providers are out-of-process (separate binaries, talking to the host over NATS). v2 plugins are in-process (loaded as Go/Rust libraries into the host). The tradeoffs:
- Out-of-process: process-level isolation; fault containment; language flexibility; latency overhead per call (NATS round-trip).
- In-process: zero-copy fast path; tighter integration; no isolation between provider and host.

**Why it matters.** Myrhiza's host imports are kernel-mediated. The kernel must decide per-import whether to run the implementation in-kernel (in-process), in a separate process, or even as a different WASM component. wasmCloud's experience with both is direct precedent.

**Myrhiza disposition.** Default to **out-of-process for I/O-touching capabilities** (network, disk, keys) for fault isolation. **In-process for CPU-only capabilities** (crypto primitives, codec). Document the criterion in the kernel spec. wasmCloud's v1 → v2 in-process shift is a warning we don't follow blindly: their reason was operational simplicity (one fewer thing to deploy), not security.

## 4. Link definition lifecycle — when does a re-link take effect

**Problem.** wasmCloud's link definitions are runtime-mutable: you can change which provider satisfies a component's import and the change is visible "soon." But: does a re-link take effect *mid-call*? Per-invocation? On next NATS-subject-resolution? The semantic isn't documented as a hard contract — see [#266 RFC: Overhaul and Upgrade Link Definition Management](https://github.com/wasmCloud/wasmCloud/issues/266) (closed) for the v1 design, and [#5062 \[Feature\] Add `wasmcloud:nats` interface for first-class NATS + JetStream support](https://github.com/wasmCloud/wasmCloud/issues/5062) (open) for v2-era surface.

**Why it matters.** Myrhiza wants capability *revocation* to be a real-time guarantee. If a user revokes app A's network capability, A's next call must fail — not the call after that, not "soon." wasmCloud's hand-wavy semantic is not enough.

**Myrhiza disposition.** Revocation is **synchronous as observed by the component** — the next host-import call sees the new authority state. The kernel doesn't queue stale capability handles. Specify this in the capability-revocation spec with a per-WIT-method exit gate.

## 5. Multi-tenancy: lattice-id-based isolation, enforced or convention?

**Problem.** wasmCloud v1 multi-tenants by *lattice ID* — a string prefix that namespaces NATS subjects. The actual enforcement of cross-lattice isolation depends on whether the NATS broker is configured to deny cross-account messaging (which an operator can forget). v2's K8s namespace-scoped Host CRD ([#5096](https://github.com/wasmCloud/wasmCloud/issues/5096), closed 2026-05-07) tightens this, using K8s namespaces as the isolation boundary.

**Why it matters.** Myrhiza is intrinsically multi-tenant — every app is a tenant of the kernel, every peer is a tenant of the network. We can't depend on operator config; the kernel must enforce.

**Myrhiza disposition.** Capability-discipline is the isolation primitive. App A cannot name app B's components or capabilities; cross-app calls require an explicit cap exchange. This is stronger than wasmCloud's lattice-ID model and removes the "operator forgot to configure" failure mode.

## 6. Secrets at scale — rotating across providers without restart

**Problem.** wasmCloud has the [#5016 Implement wasmcloud:secrets plugin backed by Kubernetes Secrets](https://github.com/wasmCloud/wasmCloud/issues/5016) (open, 2026-04-15) thread for v2 secrets. Rotating a secret across all providers/components that hold it without restarting them is acknowledged but unsolved.

**Why it matters.** Myrhiza apps will hold secrets (private keys, OAuth tokens, app-specific creds). Rotation without restart is necessary because the kernel may not be able to "restart" a component without losing its in-flight peer-to-peer streams.

**Myrhiza disposition.** Secrets are a kernel-managed capability, not a component-held value. The component imports `myrhiza:secret/load(name)` and the kernel returns the current value. Rotation is a kernel-side cache invalidation; the component sees the new value on next call. Direct precedent for the design.

## 7. Component upgrade — hot-swap without losing in-flight state

**Problem.** wasmCloud v1's answer was *drain + restart*: stop sending new requests to the old version, wait for in-flight to complete, swap. wasmCloud v2 leans on K8s rolling updates (Deployment-style). Neither preserves in-component state across the swap, because components are functions, not stateful actors.

**Why it matters.** Most Myrhiza components match the function-style assumption: state lives in the kernel-mediated state store, not in the component instance. So drain-and-restart is fine for most. But components with long-lived peer streams (interaction profile rendering an active UI; behavior profile running a multi-day bridge) need a graceful upgrade story.

**Myrhiza disposition.** **Adopt drain + restart as the default**, matching wasmCloud's posture. For long-lived interactions, document a "preserve session through component upgrade" pattern: the kernel holds the session token across the swap; the new component instance picks it up. Don't try to migrate component memory across versions — that's a category error.

## 8. Observability — WIT-typed observability isn't standard

**Problem.** wasmCloud uses OpenTelemetry for tracing/metrics, but the WIT interface for observability is not WASI-standard. Each component re-implements its tracing surface. See [#5127 feat(examples/otel-config): demo workload env/config/secrets](https://github.com/wasmCloud/wasmCloud/issues/5127) — workload OTEL config is still a manual demo.

**Why it matters.** Myrhiza wants WIT-typed everything. If `myrhiza:trace/span` is a kernel-mediated capability, components emit structured traces without each one rolling its own telemetry layer. This is also the only way to record cross-peer trace context.

**Myrhiza disposition.** Define `myrhiza:obs/{trace,log,metric}` as kernel-mediated WIT interfaces from day one. Don't import OpenTelemetry as a host concern; export structured events to whatever sink the operator configures. wasmCloud's experience says: if you don't standardize this early, every component reinvents it.

## 9. Browser path — server-only as of 2026-05-09

**Problem.** wasmCloud is server-only. The browser-host RFC ([wasmCloud/wasmCloud#27](https://github.com/wasmCloud/wasmCloud/issues/27)) is closed; the `wash ui` / washboard browser app is for *managing* a wasmCloud lattice, not for *running* components in the browser. There is no `wasmcloud-host` that runs in `wasm32-unknown-unknown`.

**Why it matters.** Myrhiza apps need a browser path — interaction-profile components rendering UI must run client-side. The browser is a peer in Myrhiza's topology, not just a control surface.

**Myrhiza disposition.** Take the browser path seriously from the start. The kernel has at least two flavors: native (Wasmtime + native I/O) and browser (jco-compiled glue + Web platform I/O). wasmCloud has nothing to copy here — see [`../wasm-component-model/browser.md`](../wasm-component-model/browser.md) for the actual prior art.

## 10. Determinism gaps inherited from Wasmtime

**Problem.** Wasmtime executes components on host-native floating point, with `wasi-clocks` returning real wall time, `wasi-random` returning real entropy. wasmCloud doesn't override any of this — components see whatever Wasmtime gives them.

**Why it matters.** Myrhiza's `state-apply` profile cannot use real wall clocks or real entropy without breaking convergence. Inherited from Wasmtime's "live execution" defaults.

**Myrhiza disposition.** The kernel intercepts every non-deterministic host import for state-apply components — wall-clock returns the event timestamp; random returns the deterministic helper. See [`../wasm-component-model/open-problems.md`](../wasm-component-model/open-problems.md). Floating-point determinism is the harder problem; document the policy explicitly.

## 11. Versioning of `wasmcloud:*` WIT packages

**Problem.** wasmCloud's WIT packages (`wasmcloud:bus/lattice`, `wasmcloud:secrets/store`, etc.) version ad-hoc. There's no semver story for "what does it mean to change a WIT package". Component-to-component link definitions assume both sides agree on the package version, but the agreement is implicit.

**Why it matters.** Myrhiza apps will be authored at different times against different WIT package versions. Cross-version compatibility — which interface evolutions are breaking? which are additive? — is a real interop problem.

**Myrhiza disposition.** Adopt the [Component Model's WIT semver work](https://github.com/WebAssembly/component-model/blob/main/design/mvp/Versioning.md) as it lands. For Myrhiza-defined packages (`myrhiza:*`), document the compatibility rules in the relevant spec. wasmCloud's "we'll figure it out" posture is what we don't copy.

## 12. Cosmonic vendor lock-in for management UX

**Problem.** wasmCloud's reference management UX is washboard (the OSS UI bundled with `wash ui`) and Cosmonic Control (the commercial product). The OSS path is functional but feature-thin; the commercial path is the supported one. Operators tend to adopt Cosmonic Control; once adopted, their operational tooling is Cosmonic-specific.

**Why it matters.** Myrhiza is single-vendor (us) at the spec layer, but we don't want apps to be locked into our management UX. Apps should be portable across kernel implementations.

**Myrhiza disposition.** App-bundle format and capability-grant format are both WIT-typed and implementation-independent. The Myrhiza CLI is one of N possible operator surfaces; the bundle should run on any conformant kernel. wasmCloud's vendor-lock at the management layer is a thing we explicitly avoid for app-portability reasons.

## See also

- [`architecture.md`](architecture.md) — host / lattice / provider topology that these problems live within.
- [`wrpc.md`](wrpc.md) — wRPC failure modes and semantics.
- [`capability-model.md`](capability-model.md) — link definitions and the authority model.
- [`tooling.md`](tooling.md) — observability, secrets, ops surface state.
- [`critiques.md`](critiques.md) — broader project-level concerns.
- [`lessons.md`](lessons.md) — what we borrow and avoid.
