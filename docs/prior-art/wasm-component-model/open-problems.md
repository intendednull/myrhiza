**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — structurally unresolved questions Myrhiza inherits by adopting CM

# Open problems Component Model doesn't solve

Problems the Component Model structurally does not solve, that Myrhiza will inherit by depending on it. None of these are bugs in the substrate; they are questions the substrate intentionally pushes onto the host. Each entry: short statement, why it matters, Myrhiza disposition with a one-line spec implication.

## 1. Async stabilization — preview3 is not landed

**State.** "Preview 3" milestone on `WebAssembly/component-model` was created [2023-08-22 by `ricochet`](https://github.com/WebAssembly/component-model/milestone/1) and is still **open** in 2026-05-09. WASI repos are at `v0.3.0-rc-2026-03-15` (latest of three RCs cut 2026-01-06, -02-09, -03-15) for the seven coordinated subsystems; no `v0.3.0` final. Wasmtime supports preview3 behind a flag; jco preview3 transpile mappings are being added *this week* ([jco#1455](https://github.com/bytecodealliance/jco/pull/1455), 2026-05-08). `Concurrency.md` is still being edited (see [#643](https://github.com/WebAssembly/component-model/pull/643), 2026-04-30).

**Why it matters.** Preview2 is poll-driven; preview3 is async-native (`stream<T>`, `future<T>`, `error-context`). Components and host imports written against preview2 do not Just Work on preview3. The migration cost is real and being paid in production code today (cf. `pulseengine/rules_wasm_component#257` "Multi-Bundle Support for WASI Preview2/Preview3 Coexistence", closed 2026-03-23).

**Myrhiza disposition.** *Spec against preview2 today; design Myrhiza's host-import worlds so that the migration to preview3 is local to byte-stream and HTTP shapes.* `state-apply` profile uses no async at all (sync, deterministic); `state-propose` and `interaction` profiles use whatever the kernel exposes. The kernel's preview2→preview3 transition is a single capability-substitution, not a runtime-wide migration. **Spec implication:** the Myrhiza spec must not bake preview2 idioms (e.g. `pollable` resources) into Myrhiza-defined WIT packages — use opaque streams typed as Myrhiza types so the underlying p2/p3 swap is invisible to apps.

## 2. Distributed component identity — CM has no story for cross-peer addressing

**State.** A component's hash identifies its bytes; nothing in the Component Model says *who* that component is on a network. The substrate has no concept of node identity, peer identity, or routable address. There is no upstream tracking issue because this is intentionally out of scope for CM.

**Why it matters.** Myrhiza is P2P. Components run on peers, talk to peers, send events to peers. Peer identity has to come from somewhere.

**Myrhiza disposition.** *Iroh supplies the transport identity (NodeID = ed25519 pubkey).* Myrhiza specs lift NodeID into a Myrhiza-layer **PrincipalID** that is stable across devices — the same shape as the open question called out in [`../iroh/open-problems.md`](../iroh/open-problems.md). **Spec implication:** the kernel's WIT package `myrhiza:peer` exposes opaque peer-handle resources; the binding from peer-handle to NodeID is kernel-private, not part of any guest's world.

## 3. Capability declaration vs runtime resolution

**State.** WIT imports declare what a component needs (`world` type lists `import wasi:io/streams; import myrhiza:state/store;` etc.). The host decides at link time which implementation satisfies each import. **There is no static guarantee that the imports a component declared match the capabilities the kernel actually grants at runtime.** The Component Model's `Linker` API checks types but not policy. See [#609 "Adding a note to WIT.md about WIT interface version interop & host downgrades"](https://github.com/WebAssembly/component-model/issues/609) (2026-02-11) for the type-level version of this gap.

**Why it matters.** A capability-style host depends on the assumption that "the imports the kernel sees in the world type" = "the imports the kernel must mediate." If a component declares `import wasi:filesystem` but the kernel intended to forbid filesystem, the kernel must either reject the load or virtualize the import to a no-op. CM gives the kernel the imports list; it does not give the kernel a policy decision.

**Myrhiza disposition.** *The kernel typechecks the world at install time, not at call time.* On install, the kernel walks the component's world type, matches each import against an allowlist for the component's profile, and rejects the load if any import is outside the allowlist. Once linked, every call is type-safe by construction. **Spec implication:** Myrhiza's profile definitions (`state-apply`, `state-propose`, `interaction`, `behavior`) each declare a *closed* set of permitted host imports. The allowlist is the load-bearing artifact, not the run-time policy check.

## 4. Determinism guarantees — CM doesn't promise them

**State.** The CM specification does not promise deterministic execution. Wasmtime exposes flags (`Config::wasm_nan_canonicalization`, `Config::cranelift_nan_canonicalization`, `Config::cranelift_pcc`) that *help*, but the substrate does not certify "this component, on this engine, produces bit-identical outputs across hosts." Compare [bytecodealliance/wasmtime-py#244 "Wasmtime-py and randomness"](https://github.com/bytecodealliance/wasmtime-py/issues/244) (2024-06-29):

> *"I am using wasmtime-py to create a fully deterministic sandbox (for reproducible build purposes) for a toolchain. […] Is there a way to tell wasmtime to only produce deterministic randomness?"*

NaN bit patterns, IEEE-754 rounding edge cases, and SIMD float ops have known non-determinism risks (see WasmEdge's [#4819 "f64x2.add and mul over non-canonical NaN inputs returns a different NaN payload (NOT PLAN TO FIX)"](https://github.com/WasmEdge/WasmEdge/issues/4819), 2026-05-05).

**Why it matters.** Myrhiza's `state-apply` profile must be deterministic. CLAUDE.md is explicit: *"Determinism is a load-bearing property. State-apply components must be pure functions of `(prior state, event)` plus the deterministic helper set."* The substrate does not guarantee this.

**Myrhiza disposition.** *Determinism comes from a kernel-side validator, not from the substrate.* The kernel restricts state-apply components to (a) a forbidden-imports list (no `wasi:clocks/wall-clock`, no `wasi:random`, no `wasi:sockets`, no SIMD floats unless canonicalized, no threads), and (b) a wasm-validator pass that rejects any component using non-deterministic instructions. **Spec implication:** there is a Myrhiza spec called *determinism-validator* that enumerates the validator's complete rule set. The validator is part of the kernel's ABI surface.

## 5. Resource lifetimes across components

**State.** Resource handles cross component boundaries via the canonical ABI (`own<R>`, `borrow<R>`). Lifetime semantics: `own` transfers ownership, drop runs the destructor; `borrow` is a lease, must not outlive the call. **What happens when both sides crash mid-call?** Recent spec edits ([#643 "Add 'Component Instance Lifetime' section to Concurrency.md"](https://github.com/WebAssembly/component-model/pull/643), 2026-04-30, [#638 "Make resource `dtor` type explicit"](https://github.com/WebAssembly/component-model/issues/638), closed 2026-04-16, [#648 "Why is it `(dtor (func n))` instead of `(dtor (core func n))`?"](https://github.com/WebAssembly/component-model/issues/648), 2026-05-05) show the spec is still settling lifetime corner cases.

**Why it matters.** Myrhiza grants every capability (peer handle, state-stream subscription, key-derivation slot, network connection) as a `resource`. If a component crashes holding a `borrow` on a kernel resource, the kernel must drop the underlying state cleanly. If the kernel crashes (process restart) holding components' `own` handles, the components must be told their resources are gone.

**Myrhiza disposition.** *The kernel owns the resource table, not the components; on any crash on either side, the kernel reissues every resource handle as a fresh i32 index in a new table.* No handle survives a process restart on either end. **Spec implication:** resource handles are not durable identifiers. Anything that needs cross-restart identity (a peer ID, a store key, a content hash) is a value type, not a resource handle.

## 6. Streams + futures + error contexts (preview3) — untested at scale

**State.** The new ABI is large. `stream<T>`, `future<T>`, `error-context` plus the `waitable-set` machinery in `Concurrency.md`. PR `#641` ([Emoji-gate synchronous future/stream read/write](https://github.com/WebAssembly/component-model/pull/641), 2026-04-27) shows the design is still feature-gated. Wasmtime ships preview3 but its async test suite has not been upstreamed to the spec repo (see [#571 "Upstream Wasmtime's async test suite"](https://github.com/WebAssembly/component-model/issues/571), 2025-10-21).

**Why it matters.** Myrhiza's interaction profile and behavior profile want futures/streams. Adopting them on a non-stable spec means absorbing whatever ABI changes land between RC and stable.

**Myrhiza disposition.** *The bet is preview2 today, preview3 later.* No Myrhiza spec adopts `future<T>` / `stream<T>` until the tracking milestone closes and Wasmtime promotes preview3 out of `unsafe_async` gating. **Spec implication:** the Myrhiza substrate spec must include a "preview3 readiness" checklist: milestone closed, Wasmtime stable, jco transpile stable, async test suite upstreamed. We adopt preview3 when all four are green.

## 7. Component composition at scale — `wac` is alpha

**State.** `wac` (`bytecodealliance/wac`) is the WebAssembly Composition language. Latest release `v0.10.0` (2026-04-17), 13 releases total since `v0.1.0` (2024-04-16). Open issues include [`#152` "wac expects \"instantiation\" of type aliaes"](https://github.com/bytecodealliance/wac/issues/152) (2025-02-06), [`#180` "Support latest WIT changes for P3"](https://github.com/bytecodealliance/wac/issues/180) (2025-08-27), [`#85` "`wac_types::Package` not having its own `wac_types::Types` requires re-parsing packages on every composition"](https://github.com/bytecodealliance/wac/issues/85) (2024-04-18, O(N²) composition cost). The "many small components" architecture is unproven beyond demo apps.

**Why it matters.** A Myrhiza app is a *bundle of WASM components*. If composition is slow, fragile, or requires manual WAC scripting, the app authoring story is bad.

**Myrhiza disposition.** *Compose at the kernel, not at build time.* The Myrhiza kernel is the composer; an app bundle is *N* loose components plus a wiring manifest, and the kernel does the equivalent of `wac` at load time using the typed `Linker` API directly. **Spec implication:** Myrhiza's app-bundle format spec defines the wiring manifest; we do not require app authors to use `wac` source format.

## 8. Versioning + compatibility — WIT semver is convention, not enforced

**State.** [#609 "Adding a note to WIT.md about WIT interface version interop & host downgrades"](https://github.com/WebAssembly/component-model/issues/609) (2026-02-11), verbatim:

> *"Linked interfaces may be downgraded to match what is in the host (i.e. `ns:pkg/iface@0.2.1` being downgraded to `ns:pkg/iface@0.2.0`) […] adding functions to an existing interface (even with `@since`) *could* become a breaking change, because guests cannot predict whether hosts will have coverage or not."*

[#540 "Incorrect references to SemVer"](https://github.com/WebAssembly/component-model/issues/540) (2025-07-01), [#573 "Consistent version syntax"](https://github.com/WebAssembly/component-model/issues/573) (2025-10-27, labeled `pre-1.0`), [#534 "Interface version / compatibilty changes"](https://github.com/WebAssembly/component-model/issues/534) (2025-08-19, labeled `0.3.x`) all open.

**Why it matters.** A Myrhiza app version-pins its host imports (`myrhiza:state@1.2.0`). If the kernel can silently downgrade to `myrhiza:state@1.1.0`, a guest written against 1.2.0's added function will fault at call time, not load time.

**Myrhiza disposition.** *Reject loads that would require a downgrade.* The kernel's load-time typecheck enforces *exact* major version match plus *minimum* minor version; if the kernel runs `myrhiza:state@1.1.0` but the guest requires `1.2.0`, the load fails. **Spec implication:** the kernel does not implement WIT-style downgrade. Myrhiza's WIT package versions are semver-strict at the kernel boundary.

## 9. Memory64 + GC + threads as required-for-X-language

**State.** Memory64: open since [#22 "Interaction with the memory64 proposal"](https://github.com/WebAssembly/component-model/issues/22) (2022-04-12, still open). Wasm GC: [#525 "Pre-Proposal: Wasm GC Support in the Canonical ABI"](https://github.com/WebAssembly/component-model/issues/525) (2025-06-03, still pre-proposal). Threads: shared-memory threads are not yet integrated with the canonical ABI for components. Exception handling: integrating EH with the CABI is open work.

**Why it matters.** A Java/Kotlin/Scala state-apply component would want Wasm GC. A Rust state-apply component does not. Myrhiza's profile spec has to draw the line.

**Myrhiza disposition.** *State-apply v1 is Rust/C/Zig only.* Profiles requiring GC, threads, or memory64 are out of scope for v1. Revisit when the corresponding upstream proposals reach Phase 4. **Spec implication:** the Myrhiza substrate spec includes a "permitted core-wasm features" list; v1 list is `multi-value`, `bulk-memory`, `reference-types`, `simd` (canonicalized for state-apply), `tail-calls`. Excluded for v1: `gc`, `threads`, `memory64`, `exception-handling`.

## 10. Component model in browsers — no native, only transpiled

**State.** No browser vendor commitment to native CM. jco transpile is the only path. [denoland/deno#31314](https://github.com/denoland/deno/issues/31314) "Support WASM components and WIT files for richer types" (2025-11-16, still open):

> *"I would like to be able to import WASM components directly from Deno and get rich type support (including complex object types). Currently it's not supported by Deno, and I can't add it myself due to Deno's lack of custom loader support."*

Mirror issue on bun: [oven-sh/bun#24867](https://github.com/oven-sh/bun/issues/24867).

**Why it matters.** Myrhiza wants apps to run on a browser-side peer. If the only path is jco-transpile, every app pays the SpiderMonkey-or-jco-shim tax (component size, cold start, runtime overhead).

**Myrhiza disposition.** *Browser peer is a shim build, not the canonical runtime.* The canonical Myrhiza peer runs on Wasmtime (native). Browser peer ships as a separate artifact built with jco-transpile, with reduced capabilities (no filesystem, polyfilled crypto, polyfilled iroh-via-WebRTC). **Spec implication:** the Myrhiza-peer spec defines two implementation profiles — *native* (Wasmtime) and *browser* (jco-transpile + JS shim) — with the browser profile a strict subset of capabilities.

## 11. Reentrance / callbacks — host cannot call back into a guest mid-call

**State.** [#412 "Support to invoke user defined callbacks inside WASM component from wasmtime"](https://github.com/WebAssembly/component-model/issues/412) (2024-11-11, labeled `pre-1.0`):

> *"Programs like eBPF usually hire a callback function to let underlying framework to invoke it when a certain kind of event happened […] Current WASM component model doesn't have an appropriate keyword/primitive to support that, and wasmtime doesn't support reentrance to WASM component."*

[psibase#1703](https://github.com/gofractally/psibase/issues/1703) (2026-02-12), citing the BA Zulip directly:

> *"There are unfortunately no good options for callbacks in the component model yet."*

**Why it matters.** Some natural designs ("the kernel calls into state-apply, which asks the kernel for a sub-state-apply on a child event") require reentrance. The substrate forbids this.

**Myrhiza disposition.** *State-apply is one-shot, no reentrance.* Any "sub-call" pattern is reshaped as a returned event the kernel applies in a fresh state-apply invocation. **Spec implication:** state-apply signature is `func(prior_state, event) -> (new_state, child_events)`; child_events are scheduled, not synchronously executed.

## 12. Observability primitives are not part of the substrate

**State.** [WebAssembly/WASI#646 "Proposal: wasi-otel"](https://github.com/WebAssembly/WASI/issues/646) (2025-03-12, still open). No standardized tracing/metrics/log WIT yet. Adopters bring their own.

**Why it matters.** A P2P kernel needs to observe what components are doing — for debugging, for quota enforcement, for replay-divergence detection.

**Myrhiza disposition.** *Define `myrhiza:tracing` ourselves.* A Myrhiza-defined WIT package for structured event emission, scoped to the calling component. **Spec implication:** the Myrhiza substrate spec defines `myrhiza:tracing@0.1.0` with `emit-event(level: enum, name: string, kv: list<tuple<string,string>>)`. Migrate to wasi-otel if/when it stabilizes.

## Implications for Myrhiza — summary

The Component Model gives Myrhiza a typed, schema-driven, capability-shaped substrate. It does not give Myrhiza determinism, peer identity, observability, browser support, or version enforcement. Each of those is a Myrhiza spec to write — *not* an upstream contribution to wait for. The list above is the inventory of what Myrhiza must own.

For neighbors carrying the same shape of inheritance: [`../iroh/open-problems.md`](../iroh/open-problems.md) (transport-only, leaves discovery + identity portability + Sybil to Myrhiza), [`../holochain/open-problems.md`](../holochain/open-problems.md) (DHT-only, leaves browser + observability), [`../agoric-endo/open-problems.md`](../agoric-endo/open-problems.md) (JS-only, leaves cross-implementation determinism). The iroh and CM stacks together cover transport + sandbox + IDL; everything else is Myrhiza.

## Sources

- `https://github.com/WebAssembly/component-model/milestone/1` (Preview 3 milestone, created 2023-08-22, still open).
- `https://github.com/WebAssembly/component-model/issues/22, /412, /525, /534, /540, /571, /573, /609, /638, /641, /643, /648` — all verified via `gh api` 2026-05-09.
- `https://github.com/WebAssembly/WASI/issues/646` — wasi-otel proposal, 2025-03-12.
- `https://github.com/bytecodealliance/wac/issues/85, /152, /180` — wac alpha-stage rough edges.
- `https://github.com/bytecodealliance/wac/releases` — 13 releases since 2024-04-16, latest `v0.10.0` 2026-04-17.
- `https://github.com/bytecodealliance/jco/pull/1455` — preview3 transpile mappings, opened 2026-05-08.
- `https://github.com/denoland/deno/issues/31314`, `https://github.com/oven-sh/bun/issues/24867` — browser-host CM gap.
- `https://github.com/bytecodealliance/wasmtime-py/issues/244` — determinism question, 2024-06-29.
- `https://github.com/WasmEdge/WasmEdge/issues/4819` — f64x2 NaN payload non-determinism, 2026-05-05.
- `https://github.com/gofractally/psibase/issues/1703` — adopter reentrance critique, 2026-02-12.
