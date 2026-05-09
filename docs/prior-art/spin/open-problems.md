**Date:** 2026-05-09
**Status:** active
**Subject:** Structural gaps in Spin's model — what the runtime explicitly does not solve

Cross-refs: [`glossary.md`](glossary.md) · [`architecture.md`](architecture.md) · [`governance.md`](governance.md) · [`comparisons.md`](comparisons.md) · [`lessons.md`](lessons.md) · [`critiques.md`](critiques.md)

Scope: gaps the Spin team or its community openly acknowledges, plus structural mismatches between Spin's request-driven model and workload classes Myrhiza must support. Verified against Spin v4.0.0 (2026-04-20) and the SIP set on `github.com/spinframework/spin`.

## 1. Stateful workloads are out of scope by design

Spin's execution unit is "create a new isolated Wasm instance corresponding to the Wasm module for the matching component, execute the handler function, then terminate the instance" (Fermyon, *Develop serverless WebAssembly apps with Spin*). Long-running workloads — databases, message brokers, websocket servers, P2P swarms — do not fit. Spin's own SIP-016 calls the runtime a "short-lived, stateless instance" model. State is shifted to host-mediated factors (KV, SQLite, Redis), but the application stays stateless. This is a non-goal, not a roadmap item — and it is exactly the shape Myrhiza's `state-apply` profile cannot adopt.

## 2. WebSocket support is best-effort and forces external state

SIP-016 (inbound websockets) is the only path to long-lived connections, and ships with explicit caveats: "any state must be kept in a persistent store outside of a given instance's memory, and the behavior of an application is highly sensitive to the consistency model of the persistent store(s) it uses." Frame delivery is "best-effort"; "applications are responsible for recovering from unexpected connection closures." Each frame may land on a different node. There is no first-class lifecycle for a "connection" the way a kernel-managed actor would have one.

## 3. Cold-start claims are bounded by toolchain

The "<1ms cold start" headline applies to Rust/Go components compiled directly to native Wasm. Python and TypeScript guests load an interpreter compiled to Wasm and pay 50–300ms first cold start (Fermyon docs). Pre-warmed runtime is assumed. Cold-cold start (no module cached, no engine warm) is unmeasured publicly. Pooling allocator, lazy table init, and copy-on-write linear memory mitigate steady-state instantiation cost (Wasmtime fast-instantiation docs), but a Store is still allocated per request.

## 4. Async story is mid-migration

Spin v3.5 shipped first WASIp3 RC support in November 2025. WASI 0.3 (the async/streams milestone) is still RC — Wasmtime has experimental support; "0.3 release expected in February with a potential 1.0 release by the end of 2026 or early 2027" (eunomia, *WASI and the Component Model: Current Status*). Until then, streaming, integrated cancellation, and composable concurrency are partial. Synchronous third-party libraries cross the host boundary via blocking imports.

## 5. Determinism is not a property the runtime provides

Spin makes no claim about cross-replica convergence. Wasmtime is deterministic in execution semantics, but Spin's host imports (clocks, randomness, KV, HTTP outbound, variables) inject non-deterministic inputs that the runtime does not constrain. Cross-peer convergence is out of scope. **Direct consequence for Myrhiza:** Spin is the wrong shape for `state-apply`. Reuse the WIT vocabulary, not the host policy.

## 6. Composition is build-time only

`wac` composes components at build time and produces one `.wasm` (Bytecode Alliance, `bytecodealliance/wac`). "The result of composition is one binary (.wasm) that can contain several isolated components." Runtime composition — wasmCloud's lattice-style late binding, hot-swapping a capability provider without redeploy — is not a Spin primitive. Late binding requires going outside Spin (kubectl rollout, registry pull, restart).

## 7. Observability is acknowledged-incomplete

Spin issue #2293 enumerates four gaps: runtime ("spans for any critical background work e.g. garbage collection"), trigger ("trace context propagation from incoming headers"), component ("automatically emit spans as the component composition graph is traversed"), and guest ("emit telemetry with spans, metadata, and metrics unique to their own use case"). OpenTelemetry hooks landed incrementally; a full standard for alternative Spin runtimes to follow is missing.

## 8. Component portability is qualified

A Spin component depends on the `fermyon:spin/*` WIT family in addition to `wasi:*`. Running a Spin component unmodified on wasmCloud or Wasmer Edge requires either reimplementing the Spin host imports or adapter shims. Spin 2.0's portability claim is "lays a foundation for portability across runtimes" (Fermyon blog) — i.e., aspirational, gated on host import standardization that has not happened.

## 9. Storage durability is deployment-side configuration

KV defaults to a local SQLite file (`.spin/sqlite_key_value.db`); production deployments swap in Redis/managed KV. SIP-016 explicitly notes SQLite gives no "explicit consistency guarantees" across nodes, so consecutive `handle-frames` invocations on the same connection "might not see the result of [a] write" without explicit transaction handling. Durability is the operator's job; the runtime does not provide it.

## 10. Component Model maturity gap

Spin commits to WASI 0.2 stable. Component Model features still in flight (preview3 streams, native async, threads, shared-nothing concurrency primitives, GC integration) are not generally available to Spin app authors. WebAssembly 3.0 (W3C, Sept 2025) standardized WasmGC, exception handling, tail calls, 64-bit memory, 128-bit SIMD — runtime support reaches Spin only as Wasmtime ships it.

## Sources

- Fermyon, *Develop serverless WebAssembly apps with Spin* — https://www.fermyon.com/spin
- spinframework/spin SIP-016 (inbound websockets) — https://github.com/spinframework/spin/blob/main/docs/content/sips/016-inbound-websockets.md
- spinframework/spin issue #2293 (observability) — https://github.com/spinframework/spin/issues/2293
- spinframework/spin issue #2321 (HTTP trigger overhead) — https://github.com/spinframework/spin/issues/2321
- bytecodealliance/wac — https://github.com/bytecodealliance/wac
- Wasmtime fast-instantiation — https://docs.wasmtime.dev/examples-fast-instantiation.html
- eunomia, *WASI and the WebAssembly Component Model: Current Status* (2025-02-16) — https://eunomia.dev/blog/2025/02/16/wasi-and-the-wasi-component-model-current-status/
- Akamai, *Build Serverless Functions with Zero Cold Starts* — https://www.akamai.com/blog/developers/build-serverless-functions-zero-cold-starts-webassembly-spin
- Fermyon, *Spin 2.0 shines on Wasm component composition, portability* (InfoWorld) — https://www.infoworld.com/article/2335330/spin-20-shines-on-wasm-component-composition-portability.html
