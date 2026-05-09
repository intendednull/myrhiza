**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — design lessons for Myrhiza (validates / avoid / borrow)

# Lessons for Myrhiza

The consult-this-when-designing file. Synthesis from the rest of the corpus, framed as actionable design statements.

The Component Model is the most production-hardened *language-agnostic, capability-shaped, schema-driven WASM packaging substrate* in existence. Where its design choices have been validated by years of multi-language production deployment (Wasmtime in Fastly Compute, Spin / Fermyon, wasmCloud, Cosmonic, Microsoft hyperlight, Shopify functions), we should treat them as load-bearing references. Where its design has *not yet stabilized* (preview3 async, GC integration, browser support), we should treat the gap as a constraint to design *around*, not a tax to pay.

This file structures lessons as **Validates / Avoid / Borrow** tables. "Validates" entries are claims about Myrhiza dressed as observations about CM — weight them with skepticism. "Avoid" and the load-bearing items in "Borrow" are the higher-leverage content. See sibling lessons docs for shape: [`../agoric-endo/lessons.md`](../agoric-endo/lessons.md), [`../iroh/lessons.md`](../iroh/lessons.md).

## Validates

Things the Component Model's production experience confirms about choices Myrhiza has already made or is leaning toward.

| Pattern | What CM does | Why it validates Myrhiza | Myrhiza application |
|---|---|---|---|
| **Imports as the only host surface.** | A component sees nothing it didn't `import`. No ambient FS, no implicit network, no syscall. The world type is the complete inventory. | Capability discipline at the binary level. Adopted by Fastly, Fermyon, wasmCloud, Shopify across multi-million-request-per-day production traffic. | Myrhiza's "host imports = capabilities" model has direct CM precedent. Each profile (`state-apply`, `state-propose`, `interaction`, `behavior`) is a WIT world; the world's import list is the complete capability grant. **Use this as the load-bearing argument** when explaining the runtime to outsiders. |
| **WIT as the IDL boundary.** | Typed interfaces between components, with semver-shaped versions and a declarative composition model. | Composition without runtime ABI breakage. Cross-language (Rust + JS + Python + C# + Go + Moonbit) interop without bespoke marshalling. | Myrhiza's kernel imports are WIT interfaces (`myrhiza:state`, `myrhiza:peer`, `myrhiza:authority`, …). Apps speak WIT. We do not invent a private IDL. |
| **Resources as the capability primitive at the type level.** | `resource <name> { … }` with `own<R>` / `borrow<R>` handles. Per-component i32 indirection table. Drop runs the destructor. | Revoke-on-drop. Borrow-without-ownership-transfer. Type-level distinction between transferable and lease-only handles. Production-tested across all Wasmtime adopters. | Every Myrhiza capability the kernel lends a guest is a WIT `resource`: peer-handle, state-stream subscription, key-derivation slot, network connection. Revocation is "drop the table entry on the kernel side." |
| **Content-addressed packaging via OCI.** | The `wkg` + OCI artifact convention: a component's bytes have a SHA hash; the registry stores the hash; pulls are content-verified. | Content-addressed identity for code bundles is the right baseline; matches what we already plan for Myrhiza app bundles. See also `../agoric-endo/modules-and-bundling.md` (Endo's `b1-<sha512>` shape). | Myrhiza app bundles are content-addressed (hash of normalized manifest + components + assets). Wire format aligns with the OCI artifact convention so existing registries can host them. |
| **Sandboxing by spec, not by engine.** | The CM specification dictates the canonical ABI; any conformant engine must lift/lower types identically. Multiple engines (Wasmtime, jco preview2-shim, partial wasmCloud, partial wasmer-via-wai) prove the spec is implementable. | Determinism by *spec* is achievable when the spec is precise enough. Multi-engine cross-validation is real (jco preview2-shim runs in browsers; Wasmtime runs natively; both must agree). | Myrhiza's `state-apply` profile depends on bit-identical execution across peers running different host stacks. The CM precedent (canonical ABI is bit-precise; multiple engines exist) is evidence this is achievable. |
| **Adapter modules for legacy ABIs.** | The preview1 → preview2 adapter is a separate WASM module bundled by `wasm-tools component new`; it lifts old WASI-preview1 imports into preview2 imports. | Migration cost is bounded by an adapter module, not a flag day. Same shape works for any ABI break. | When Myrhiza's WIT packages bump major version, the kernel can ship an adapter component that lifts old-version guests onto the new kernel ABI. Bounded migration cost, not flag-day. |
| **Typed `Linker` resolution.** | The Wasmtime `Linker` resolves a component's imports against a typed registry of host implementations; mismatches fail at link time, not at call time. | Static catch for "this component asked for an import the host doesn't have." Avoids debugging-at-3am runtime "function not found." | Myrhiza's kernel uses Wasmtime's typed `Linker` directly; load failure on capability mismatch is a feature, not a bug. |
| **Profile-the-substrate-not-the-app.** | CM's perf knobs (Pulley vs Cranelift, Winch baseline, fuel metering, epoch interruption) are engine concerns. App authors do not configure them. | The kernel owns runtime policy; apps express *what* not *how*. Same separation Myrhiza wants between kernel and apps. | Myrhiza app bundles do not declare fuel budgets, GC frequency, or instruction-count limits. The kernel does. |
| **Custom WIT packages are first-class.** | Adopters routinely define `wasmcloud:*`, `spin:*`, `fastly:*` WIT packages alongside `wasi:*`. Tooling treats them identically. | We can publish `myrhiza:*` WIT packages without forking tools or fighting upstream. Tooling neutrality is real. | Myrhiza ships `myrhiza:state`, `myrhiza:peer`, `myrhiza:authority`, `myrhiza:tracing` as ordinary WIT packages. No special-casing required. |

**Skepticism check on this section:** every entry above is a Myrhiza decision we *want* validated. CM's success at any of these is partial evidence, not proof. The strongest items are *imports-as-only-surface* and *resources-as-capabilities* (multi-million-request-per-day production validation across multiple adopters). The weakest is *sandboxing by spec*: in practice, only Wasmtime is fully spec-conforming; jco's preview2-shim and Wasmer's wai are partial implementations. Cross-engine bit-identical execution is closer to "one engine + a verifier" than to "many engines."

## Avoid

Things the Component Model has done that did not work, or that Myrhiza should not copy because they encode upstream-specific assumptions.

| Anti-pattern | What CM does | Why Myrhiza should avoid copying | Myrhiza alternative |
|---|---|---|---|
| **Async story decided in stages.** | Preview2 (sync, poll-driven) shipped 2023; preview3 (async-native, `stream<T>`/`future<T>`) still in flight in 2026 — milestone open since 2023-08-22. Adopters live through ABI churn (cf. `pulseengine/rules_wasm_component#257` "Multi-Bundle Support for WASI Preview2/Preview3 Coexistence"). | A peer-symmetric P2P runtime cannot afford a flag-day async migration. Two async models simultaneously means two state-apply ABIs, two replay shapes, two kernel adapters. | **Myrhiza commits to one async model up front.** State-apply is sync, period. Interaction/behavior profiles use whatever the kernel exposes via opaque Myrhiza-typed streams. The preview2→preview3 swap is invisible to apps because no Myrhiza WIT package exposes preview2 idioms directly. |
| **Host downgrade of WIT versions.** | Per [#609](https://github.com/WebAssembly/component-model/issues/609): `ns:pkg/iface@0.2.1` may be silently downgraded to `0.2.0` if the host has the older version. Adding `@since`-gated functions can become a breaking change. | Silent downgrade gives runtime call-time failures instead of load-time failures. Defeats the static type-safety the substrate otherwise gives us. | **The Myrhiza kernel rejects a load if the host's interface version is below what the guest world required. No silent downgrade.** Compatibility is by version-pinning, not by hopeful runtime fallback. |
| **Bundling whole language engines into components.** | componentize-js bundles SpiderMonkey (~5MB+); componentize-py bundles CPython (35MB+ for hello-world per [componentize-py#98](https://github.com/bytecodealliance/componentize-py/issues/98)). | A Myrhiza app bundle that grows by 35MB-per-language is not P2P-distributable. We are content-addressing bundles and gossiping them; bundle size is wire weight. | **State-apply profile v1 is Rust/C/Zig only.** No JS, no Python, no engine-bundling profiles. Other profiles (interaction, behavior) may bundle engines if they want, but the kernel's quota system makes this expensive. |
| **Reentrance via callbacks deferred to "future work."** | [#412 "Support to invoke user defined callbacks inside WASM component"](https://github.com/WebAssembly/component-model/issues/412) open since 2024-11; cited as "no good options for callbacks in the component model yet." | Some designs naturally want kernel→guest reentrance ("call state-apply, which asks the kernel to apply a sub-event"). If we design around reentrance and the substrate forbids it, we hit a wall. | **State-apply is one-shot.** No host→guest reentrance from inside a guest call. Sub-events are *returned* by state-apply, scheduled by the kernel, applied in a fresh invocation. The shape compiles cleanly to the substrate's restriction. |
| **Single canonical registry that doesn't exist.** | The "use OCI, host of your choice" answer leaves canonical-registry as a vibe rather than a spec. Recent failures: `WebAssembly/WASI#886` "Hosting WITs via OCI on GHCR is flaky" (Wasmtime's *own* security release CI broke). | Myrhiza distributes app bundles over a P2P network. We cannot delegate "where do bundles live" to OCI-the-protocol-on-some-server. | **Myrhiza specifies its own bundle distribution: content-addressed bundles gossiped over iroh, fetched from peers via iroh-blobs.** OCI compatibility is a wire-format alignment, not a distribution mechanism. |
| **Spec-velocity churn while shipping.** | The CM repo has had 10+ substantive commits to spec text in the past 30 days as of 2026-05-09; `Concurrency.md` is still being edited. PR titles include "Restrict all `context.{get,set}` in same component to use same elem type" (substantive ABI restriction, merged 2026-05-07). | If Myrhiza's spec edit cadence matches CM's, Myrhiza apps cannot pin a stable target. | **Myrhiza substrate spec versions are pinned to specific upstream commit-hashes.** When upstream changes, we re-pin deliberately, not by tracking `main`. **Spec implication:** the Myrhiza substrate spec includes a `pinned_versions.toml` with exact commit hashes for `wasm-tools`, `wasmtime`, `wit-bindgen`, and the CM spec text. |
| **Sandboxing-by-engine-flags for determinism.** | Wasmtime exposes `wasm_nan_canonicalization`, `cranelift_pcc`, etc.; getting a deterministic execution requires picking the right combination of flags per-host. There is no single "deterministic mode." | Per-host flag tuning is brittle. A peer running with the wrong flag combo silently produces non-deterministic output, and the divergence shows up as a state-hash mismatch hours later. | **The Myrhiza determinism validator is a wasm-validator pass, not a runtime flag set.** A state-apply component is rejected at load if it uses any non-deterministic instruction or imports any non-deterministic capability. The kernel doesn't have to "set the right flags" — the validator already proved the binary is deterministic. |
| **Non-uniform language ergonomics treated as a per-language fix.** | Rust path is mature; C++/Go/Moonbit/C#/Python paths have known bugs in resource handling, generated bindings, and ABI compliance (`wit-bindgen` open issues `#1604`, `#1587`, `#1585`, `#1582`, `#1518`, `#1516` all 2026). The fix is per-language, on a long tail. | If Myrhiza pretends to support N languages, we inherit the long tail. | **Myrhiza v1 supports exactly the languages whose Component Model toolchain is in good shape: Rust + (where the binary is precompiled and Myrhiza never invokes a CM toolchain itself) any language whose author can produce a valid component.** The Myrhiza kernel does not host wit-bindgen, componentize-js, componentize-py, or any guest toolchain; we receive components, not source. |

## Borrow

Specific design choices from the Component Model that Myrhiza should adopt with attribution.

### The 4-pass authoring model (WIT → bindings → core wasm → component)

CM separates authoring into four conceptual passes (see [`spec.md`](spec.md) for the long form):

1. WIT → bindings (per-language, via wit-bindgen / componentize-*).
2. Guest source + bindings → core wasm.
3. Core wasm → component (wasm-tools embed + new).
4. Component composition (wac or kernel-time linking).

**Myrhiza application.** App authors produce components (passes 1–3) using whatever guest toolchain they like; the Myrhiza kernel does pass 4 at load time. The kernel never invokes a guest toolchain. **Spec implication:** Myrhiza's app-bundle format includes pre-built components, never source. The kernel's only authoring-side dependency is wasmtime + a Myrhiza-specific load-time validator.

### The canonical ABI for cross-language data

The CABI specifies bit-precise lifting/lowering of every WIT type into core-wasm primitives. Strings have a designated encoding (host-negotiated, but pinned per-component). Records are flattened. Variants are tag + payload. Resources are i32 handles into per-component tables. See [`abi.md`](abi.md).

**Myrhiza application.** Myrhiza inherits the canonical ABI verbatim. We do not invent our own marshalling. This means: any WIT-typed function in a Myrhiza package compiles to a bit-precisely-defined wire format that any conformant CM engine produces identically. **Spec implication:** Myrhiza's "what does state-apply receive on the wire" question is answered by the CABI section of the spec, not by anything Myrhiza-specific.

### Resource handles via per-component i32 tables

Each component instance has a private table indexed by i32. Resource handles (`own<R>`, `borrow<R>`) are i32 indices into this table. The kernel side controls what each i32 maps to.

**Myrhiza application.** This is the load-bearing mechanism for revoke-on-drop and for sandboxing. Two components cannot forge each other's handles because the i32 namespaces are private. The kernel can reissue handles freely on restart because handles are not persistent. **Spec implication:** every Myrhiza capability is a resource handle. The kernel's resource table is the *complete* runtime policy enforcement point — nothing gets through that isn't an entry in the table.

### The world / interface split

A component conforms to a *world* (named bundle of imports + exports). An *interface* (named bundle of functions + types) fills slots in a world. Worlds are signature-level; interfaces are implementation-level.

**Myrhiza application.** Each Myrhiza profile is a *world*. Each capability the kernel exposes is an *interface*. The world says "this component imports `myrhiza:state/store`"; the interface says "store has `get`, `put`, `subscribe`". The split lets us version interfaces independently of profiles. **Spec implication:** the four Myrhiza profiles are four named worlds defined in `myrhiza:profiles@1.0.0`. Adding a new profile is a world declaration; adding a capability is an interface declaration.

### The OCI content-addressed registry convention

CM defines a wire shape (an OCI artifact descriptor + manifest + layers) for storing components in any OCI registry. `wkg` is the reference tool. Hash-addressed pulls are content-verified.

**Myrhiza application.** We content-address Myrhiza app bundles. The wire shape *aligns with* the OCI artifact convention so existing registries can host Myrhiza bundles, but the canonical distribution is iroh-blobs over a P2P network, not OCI-over-HTTPS. **Spec implication:** Myrhiza bundle hashes are SHA-256 over a normalized manifest. The bundle on iroh-blobs is bit-identical to the bundle on a hypothetical OCI registry. Cross-distribution interop is by hash equality.

### The Wasmtime `Linker` typed-import-resolution pattern

Wasmtime's `Linker` (per [`wasmtime.md`](wasmtime.md)) is a typed registry of host implementations; a component's imports are resolved against the linker at link time. Type mismatches are link errors, not runtime errors.

**Myrhiza application.** The Myrhiza kernel uses Wasmtime's `Linker` directly to enforce the profile-allowlist. On `instantiate`, the kernel populates the linker with exactly the imports permitted for the component's declared profile. Imports that are not in the allowlist simply do not exist in the linker, so the component fails to instantiate. **Spec implication:** the kernel does not need a runtime "is this call allowed?" check — the typed linker enforces it once, at instantiation, by construction.

### Fuel + epoch metering combined

Wasmtime supports two metering primitives:

- **Fuel** — instruction-count budget; component traps when fuel runs out. Precise, but ~10–20% perf hit (per [`wasmtime#4109 "Slacked fuel metering"`](https://github.com/bytecodealliance/wasmtime/issues/4109)).
- **Epoch interruption** — cooperative time-slicing; a host thread bumps an epoch counter, components check it at function entry. Cheap, but coarse.

**Myrhiza application.** Myrhiza uses **fuel for state-apply** (deterministic budget that's part of the consensus invariant — every peer applies the event with the same fuel limit, terminates at the same instruction count) and **epoch for interaction/behavior** (wall-clock time-slicing for fairness, no consensus implication). The two primitives carry different semantic loads; mixing them is the right answer. **Spec implication:** the determinism spec defines a fixed `STATE_APPLY_FUEL_BUDGET` per profile; the scheduling spec defines epoch-tick frequency for non-deterministic profiles. These are separate knobs in separate specs.

### Adapter components for ABI migration

The preview1→preview2 adapter is a *component itself* that wraps a preview1 guest into a preview2 ABI. The adapter is shipped in the component bundle, not in the host.

**Myrhiza application.** When Myrhiza's WIT packages bump a major version, the kernel can ship an adapter *component* that lifts old-version guests onto the new kernel ABI. The adapter is loaded transparently when an old guest is detected. **Spec implication:** Myrhiza's upgrade spec defines an adapter-component format; old apps continue to run via adapter without re-publishing.

### The `world` as the unit of capability declaration

A component declares its full capability footprint in its world type. The kernel does not need to introspect imports separately; the world is the declaration.

**Myrhiza application.** Myrhiza apps declare their profile world in the bundle manifest. The kernel verifies that the component's actual world type matches the declared profile (cryptographically — the world hash is part of the bundle hash). **Spec implication:** the Myrhiza bundle manifest includes a profile name and a world-hash; on load, kernel re-derives world-hash from the component bytes and rejects any mismatch.

### Validator as a separate pass from the runtime

The CM has multiple validators: `wasmtime::Component::validate` (structural), wasm-tools `validate` (binary format), the canonical ABI lift/lower checks (type-level). Each is a pure function over the binary, runnable offline.

**Myrhiza application.** Myrhiza's determinism validator is a *separate offline-runnable function* over the component binary. It can be run by app authors as a pre-flight check; it can be run by app stores as a publish-gate; it can be re-run by every peer on first load. The validator is not coupled to the runtime. **Spec implication:** the Myrhiza determinism-validator spec defines an offline binary tool plus a library API. Determinism is a property the binary has, not a property the runtime grants.

## Open questions Myrhiza specs need to answer

The Component Model gives us a substrate, not a P2P runtime. The substrate-level questions Myrhiza specs must address:

- **Determinism enforcement.** The CM doesn't promise determinism. Myrhiza does. The validator spec is load-bearing — write it before any state-apply spec.
- **Capability discovery.** A component's world says what it imports. *How* a user discovers, grants, and revokes those capabilities is not a CM concern. Myrhiza UX spec.
- **Resource-handle persistence.** CM resource handles are per-instance, non-durable. Anything Myrhiza wants to persist across restart must be a value type (a content hash, a key, a peer ID), not a handle.
- **Cross-peer state-apply convergence.** Two peers running the same component on the same event with the same prior state must produce the same new state. CM gives us bit-precise CABI; Myrhiza must verify the entire pipeline is bit-precise (not just CABI: also fuel exhaustion, NaN canonicalization, SIMD float semantics).
- **Bundle distribution.** CM gives us OCI as a hint; Myrhiza distributes over iroh-blobs. The iroh ↔ OCI wire-shape alignment is a Myrhiza spec.
- **Profile evolution.** Today: four profiles. What is the upgrade path when we need a fifth? Adapter components answer the per-component case; profile evolution is the meta-question.
- **Browser-peer subset.** The browser peer (jco-transpile path) cannot expose every capability a native peer can. The capability subset has to be specified, not improvised.

Each question above maps to a Myrhiza spec we'd write, with the CM substrate as the foundation.

For neighbors that have already written some of these specs in their problem domain: [Holochain's deterministic-validation pattern](../holochain/) (deterministic zome calls, integrity vs coordinator zomes), [Agoric SwingSet's transcript-driven replay](../agoric-endo/persistence.md) (orthogonal-persistence + crank metering), [Spritely's ocap discipline](../spritely-ocapn/) (ocap = entire authority story, with no kernel beneath), [Iroh as load-bearing dependency precedent](../iroh/) (a transport library upon which higher-layer P2P is built, exactly Myrhiza's relationship to CM).

## Sources

Verified facts in this file are drawn from companion docs in this directory: [`spec.md`](spec.md) (WIT, four-pass model, preview2 set), [`abi.md`](abi.md) (canonical ABI), [`wasmtime.md`](wasmtime.md) (Linker, fuel, epoch), [`tooling.md`](tooling.md) (wit-bindgen, componentize-js, componentize-py), [`languages.md`](languages.md) (per-language maturity), [`browser.md`](browser.md) (jco-transpile), [`preview-status.md`](preview-status.md) (preview2/3 readiness), [`governance.md`](governance.md), [`history.md`](history.md), [`ecosystem.md`](ecosystem.md), and to the load-bearing critiques and unresolved gaps in [`critiques.md`](critiques.md) and [`open-problems.md`](open-problems.md). Upstream issue/PR citations live in those files; this file does not duplicate them, only synthesizes.
