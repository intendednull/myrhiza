**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — Wasmtime as the load-bearing CM runtime

# Wasmtime

Wasmtime is the [Bytecode Alliance](https://bytecodealliance.org/)'s reference WebAssembly runtime, written in Rust, with [Cranelift](https://cranelift.dev/) as its primary code generator and [Winch](https://docs.wasmtime.dev/cli-options.html#run) as a faster baseline compiler. It is the most complete public implementation of the [WebAssembly Component Model](https://github.com/WebAssembly/component-model) and Myrhiza's intended host runtime — every WASM component the kernel loads runs inside a Wasmtime `Engine`.

This document is the load-bearing prior-art reference for that choice. Sibling files: [`spec.md`](spec.md), [`abi.md`](abi.md), [`tooling.md`](tooling.md), [`languages.md`](languages.md), [`browser.md`](browser.md), [`governance.md`](governance.md), [`history.md`](history.md), [`ecosystem.md`](ecosystem.md), [`critiques.md`](critiques.md), [`open-problems.md`](open-problems.md), [`lessons.md`](lessons.md), [`preview-status.md`](preview-status.md).

## What Wasmtime is

Wasmtime is a standalone, embeddable WASM runtime designed to be linked into other programs as a library and used as the basis for a CLI host. The default execution backend is Cranelift, which JIT-compiles each module / component the first time it is instantiated. The CLI also supports ahead-of-time compilation (`wasmtime compile`, `Engine::precompile_module`, `Engine::precompile_component`), producing native object files that can be loaded later without re-compiling. An interpreter mode (Wasmtime's *Pulley*) exists for platforms where JIT is undesirable; it is opt-in.

Wasmtime is a Bytecode Alliance flagship project. Its development is shared between Fastly, Microsoft, Intel, AMD, Arm, IBM, Shopify, Cosmonic, and a long tail of contributors. The runtime is released on a fixed monthly cadence: a new `release-X.0.0` branch is cut on the 5th of every month, and the major version is published on or around the 20th ([Wasmtime release process docs](https://docs.wasmtime.dev/stability-release.html)). Verified: v40.0.0 was published 2025-12-22, v41.0.0 on 2026-01-20, v42.0.0 on 2026-02-24, v43.0.0 on 2026-03-20, v44.0.0 on 2026-04-20, with v36, v43, and v44 all receiving point releases on 2026-04-30 — that is the monthly major + LTS-backport pattern in action. Releases that are a multiple of 12 are LTS for 24 months; others are supported for 2 months.

The current stable release at time of writing is **`wasmtime` 44.0.1** (crates.io published 2026-04-30). `wasmtime-wasi` and `wasmtime-wasi-http` are pinned to the same `44.0.1` major version.

## Crate layout

The Wasmtime workspace publishes a constellation of crates. The ones Myrhiza will depend on directly:

| Crate | Role |
|---|---|
| `wasmtime` | Embedder API. `Engine`, `Store<T>`, `Module`, `Component`, `Linker`, `Instance`, `Func`. The only crate Myrhiza's kernel needs to link in to instantiate components. |
| `wasmtime-wasi` | Host-side implementation of the `wasi:cli`, `wasi:filesystem`, `wasi:clocks`, `wasi:random`, `wasi:io`, `wasi:sockets` worlds. Optional — you only link it in if you want components to see real WASI imports. Myrhiza will *not* link this in for `state-apply` components; it may link it in (carefully filtered) for `interaction` and `behavior` components. |
| `wasmtime-wasi-http` | Host-side implementation of `wasi:http`. Same rule: gated per-profile. |
| `wasmtime-wast` | Test harness for `.wast` spec tests. Useful in our test suite, not at runtime. |
| `wasmtime-cli` | The `wasmtime` binary. Not used by Myrhiza directly. |
| `cranelift-codegen` (`0.131.1` at v44) | Lower-level codegen crate, transitively pulled in. Myrhiza never depends on it directly. |
| `wit-bindgen` (`0.57.1` at time of writing) | Code generator. **Build-time** dependency on the *guest* side and a runtime-helper crate on the host side; see below. |

See [`ecosystem.md`](ecosystem.md) for the broader Bytecode Alliance crate set (`wac`, `wasm-tools`, `cargo-component`, `jco`).

## Component Model embedder API

Wasmtime's component-model surface lives under `wasmtime::component::*`. The flow is:

1. **`Engine`** — heavy, expensive to construct, configured once. Holds compiled code caches, the JIT, allocator. Cheap to share by `Arc` between threads.
2. **`Component::from_file(&engine, path)`** or **`Component::from_binary(&engine, bytes)`** — parses and compiles a component. Cacheable across `Store`s.
3. **`Linker<T>::new(&engine)`** — type-checked symbol table. The host registers its imports here. `Linker` is where capability mediation happens: every host function the component will call has to be added explicitly. There is no "default everything available" mode.
4. **`Store<T>`** — per-instance state. Owns the WASM linear memory(s), tables, instance state, and a generic `T` of host data. Stores are *not* `Send` while a call is in progress, and *no two stores share mutable state*. This is the unit of isolation.
5. **`Linker::instantiate(&mut store, &component)`** → **`Instance`**. Returns the component's exports.
6. **`Instance::get_typed_func::<Args, Ret>(&mut store, "name")`** — resolves an export and type-checks it once.

Type-checking happens at link time (`Linker` ↔ `Component`), so wiring errors show up before the first instantiation. The host side of bindings is normally generated by `wit_bindgen::generate!` (host mode), which expands to a Rust trait whose methods are the component's imports plus a struct of the exports. Myrhiza's kernel will use this on the host side; guest crates (in third-party app projects) will use it in guest mode.

For comparison with sibling component-model docs see [`abi.md`](abi.md) (the canonical ABI: how WIT types lower into core WASM) and [`tooling.md`](tooling.md) (`cargo-component`, `jco`, `wac`).

### Imports as the only host surface

Wasmtime's component-model guest cannot reach the host except through a function its world declared as an import and whose binding the host registered with `Linker::func_wrap` / via `wit_bindgen` host code. There is no environment-variable side channel, no global table of host functions, no implicit FS namespace. This matches Myrhiza's "capabilities are the only host surface" rule one-for-one — the runtime gives us the property mechanically and we get to decide policy in the kernel.

## Security model

Wasmtime's isolation story has three layers:

- **Process isolation** is *not* provided. Wasmtime runs in-process. If you need a hard kernel boundary between guest and host, you must run Wasmtime itself in a sandboxed process (e.g. `seccomp`, jailed container). The Bytecode Alliance project that does this for you is [`wasi-virt`](https://github.com/bytecodealliance/wasi-virt) at the WASI layer, not Wasmtime itself.
- **Instance isolation** is the primary boundary. Each `Store` has its own linear memory (or memories); cross-store memory sharing is impossible without an explicit shared `SharedMemory` (and even then only for core modules with the threads proposal enabled). Two components in two stores can only communicate via host functions explicitly wired by the embedder.
- **Linear-memory bounds** are enforced by Cranelift codegen using guard pages on 64-bit platforms (default) or explicit bounds checks on 32-bit. Out-of-bounds access traps deterministically.

For a peer-to-peer runtime where untrusted apps run on the same host as kernel state, instance isolation + capability-only imports is the load-bearing property. Myrhiza inherits both for free.

## Determinism levers

Wasmtime offers a handful of `Config` toggles that affect determinism. None of them adds up to a "deterministic mode" — Wasmtime explicitly does not promise bit-for-bit reproducibility of execution between hosts of different vintages, CPU architectures, or even wasmtime versions. What it does promise is that within the levers below, behaviour is well-defined.

The relevant `Config` methods (all on `wasmtime::Config`):

- **`wasm_threads(false)`** — disables the threads proposal (`wasi:threads`, `shared` linear memory, atomics). Threads are a primary source of nondeterminism. Off by default in component-model contexts; we keep it off for `state-apply`.
- **`wasm_simd(false)`** — disables the SIMD proposal. Most SIMD ops are deterministic in the spec, but not all (NaN payload propagation, some integer→float conversions). Off in `state-apply` for safety; keep on elsewhere.
- **`wasm_relaxed_simd(false)`** — disables relaxed-simd. *Always off* for `state-apply`; relaxed-simd is explicitly nondeterministic.
- **`cranelift_nan_canonicalization(true)`** — forces every `f32`/`f64` NaN result emitted by Cranelift to a canonical bit pattern. Costs ~1–2% throughput but eliminates the only well-known source of float nondeterminism on x86 vs ARM.
- **`consume_fuel(true)`** — see *Resource limits* below.
- **`epoch_interruption(true)`** — see *Resource limits* below.

A useful invariant: `state-apply` components run with `wasm_threads(false)`, `wasm_relaxed_simd(false)`, `cranelift_nan_canonicalization(true)`, and a fixed Wasmtime major version. Combined with WIT's deterministic ABI (see [`abi.md`](abi.md)) and a deterministic helper set in the host imports, this gets us a well-defined function from `(prior state, event)` to next state. It does *not* get us byte-equivalent behaviour across, e.g., x86 and ARM — Cranelift compiles per-host, so the executable bits differ. Cross-peer convergence relies on observable WIT-level outputs, not on machine code.

### Implications for Myrhiza — determinism

`state-apply` profile must set the conservative flags. We treat any divergence from those defaults as a correctness bug. Pre-check (kernel dry-run) uses the same flags — it is mechanically the same WASM function as `state-apply`, called by the kernel in dry-run mode, so determinism flags applied at `Engine` level cover both paths automatically.

Compare with [Agoric SwingSet's xsnap](../agoric-endo/determinism.md), which restricts the *language* (Hardened JS) and uses [snapshot-replay-with-syscall-comparison](../agoric-endo/persistence.md) to catch nondeterminism. Wasmtime gives us less restrictive defaults but a wider language ecosystem (any language that targets WASM); Myrhiza's contract is that `state-apply` opts into the strict subset. See [`critiques.md`](critiques.md) for the case against trusting the runtime's defaults.

## Resource limits: fuel vs epoch

Wasmtime has two independent mechanisms for bounding guest execution. They serve different purposes and Myrhiza will use both.

### Fuel (`Config::consume_fuel(true)`)

Per-instruction count. Cranelift inserts a decrement-and-check before each WASM instruction (or each block, with peephole optimisations). The host calls `Store::set_fuel(N)` to budget; the guest traps with a `Trap::OutOfFuel` when fuel hits zero. Fuel is *deterministic* — same instruction count for the same input regardless of wall clock — and that is its key property. A fuel budget *is* a deterministic notion of "how much computation" a guest may do.

Cost: ~5–15% throughput overhead from the per-block check.

### Epoch interruption (`Config::epoch_interruption(true)`)

Cooperative time-slicing. The host runs a thread that calls `Engine::increment_epoch()` periodically (say, every 1 ms). Cranelift inserts a single load-and-compare against the engine's current epoch at each function entry and loop back-edge. When the epoch changes, the guest traps. Epoch is *non-deterministic* — it depends on wall clock, scheduler, and how busy the host is — but vastly cheaper than fuel (~1% overhead) and ideal for "kill any guest that runs longer than N ms."

### Implications for Myrhiza — fuel vs epoch

- `state-apply`: **fuel only**. Determinism is mandatory; epoch traps would diverge across peers. The fuel budget per event is part of the system spec — it must be the same on every peer, derived from `(component digest, event)`.
- `state-propose`: **fuel only** for the same reason — it produces a candidate event which `state-apply` will re-execute.
- `interaction`, `behavior`: **both**. Use epoch as the cheap watchdog ("don't let a UI handler hang the peer") and fuel as a hard cap ("don't let a behavior burn an entire core").

The `Store::limiter` API additionally caps memory growth and table size. Myrhiza will set this per-profile.

## Snapshots and pre-initialised images

Wasmtime supports two forms of "pre-do the slow part":

- **`Module::serialize` / `Engine::precompile_module`** — produces an opaque, Wasmtime-version-and-target-pinned native blob from a `.wasm` file. Loading it via `Module::deserialize` skips Cranelift codegen entirely. Usable for component sub-modules; the component-level analogue is `Engine::precompile_component`. Myrhiza will use this aggressively — kernel ships precompiled images, peers deserialise on first use.
- **`InstancePre`** — a "ready to instantiate" object that has done all the linker type-checking up front. Cheap to instantiate from. Use this when the same component is instantiated repeatedly with different `Store` data (which is what `state-apply` looks like — one per event).

**What Wasmtime does *not* offer** is a snapshot of a *live instance heap*. There is no equivalent to [Agoric SwingSet's xsnap-snapshot-of-running-vat](../agoric-endo/persistence.md), where xsnap freezes a JS heap mid-execution and later restores it byte-for-byte into a new process. Wasmtime has no `Instance::snapshot() -> Vec<u8>` and no plan for one. Linear memory plus globals plus tables plus stack are observable, in principle, but the JIT's compiled code refers into them in ways that a generic snapshotter cannot safely capture (return addresses, instance pointers in stub vectors, etc).

### Implications for Myrhiza — no live-instance snapshot

This is a load-bearing constraint and it shapes the rest of the runtime design.

- We cannot interrupt a `state-apply` invocation, persist its in-flight stack, and resume it later. Either it runs to completion in a single `call`, or we replay from the prior committed state plus the event transcript.
- Myrhiza's model has to be *run-to-completion-per-event*. A single `state-apply` invocation per event, bounded by fuel, with no host calls that suspend (no async waits, no I/O that blocks waiting on the network).
- For long-running computation we *cannot* "checkpoint at instruction N" the way Agoric does. We can only checkpoint between events. That is a deliberate restriction on what `state-apply` can express.
- The transcript model carries over from Agoric's lessons: persist `(prior state digest, event)` pairs; replay from a known-good state by re-running the whole sequence. See [`../agoric-endo/persistence.md`](../agoric-endo/persistence.md) for the design we are *not* getting for free.

This is likely the largest gap between Wasmtime and a hypothetical "ideal" P2P-app runtime. It is the price of using a conventional JIT instead of an image-based VM.

## Async support

Wasmtime supports calling guest functions from async Rust via:

- **`Config::async_support(true)`** — enables async at the engine level.
- **`Store::call_async(...)`** — returns a `Future<Output = Result<...>>` you can `.await`.

Internally, Wasmtime uses [stack switching](https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#method.async_support) to make synchronous WASM code yield from inside an async runtime: each call gets a guard-page-protected fiber stack, host-imported async functions yield by suspending the fiber, and the host-side `Future` resumes it. The guest code is unchanged; from its perspective every host call is synchronous.

This is how `wasmtime-wasi-http` lets a synchronous WASM component perform an HTTP request: the host implementation of `wasi:http/outgoing-handler.handle` is async on the host side, but the fiber abstraction lets it look synchronous to the guest.

For preview3's native async (`stream<T>`, `future<T>`, `async fn` in WIT) Wasmtime support is in flight; see [`preview-status.md`](preview-status.md). The fiber-based "fake async" remains the workhorse until preview3 ships.

### Implications for Myrhiza — async

`state-apply` invocations are synchronous by design (no host call may yield). For `behavior` and `interaction` profiles, fiber-based async is fine — they are non-deterministic per peer anyway.

## Embedder surface for capability mediation

The mechanical recipe for "this component can only do what the kernel let it do":

1. Construct an `Engine` with the determinism flags appropriate to the component's profile.
2. Construct a `Linker<T>` against that engine. **Do not** call `wasmtime_wasi::add_to_linker_sync` unconditionally — that wires up *all* of WASI. Only add the imports the kernel chose to expose.
3. For each capability you do expose, register a host function via `Linker::func_wrap("interface", "fn-name", |store_ctx, args| -> result { ... })` or via `wit_bindgen::generate!`-produced trait impls. The host function closes over the kernel's policy data (held in `T` of the `Store`).
4. Instantiate the component into a `Store<T>` whose `T` carries the per-instance policy state (capability handles, log sink, fuel budget, epoch deadline).
5. Call exports via `Instance::get_typed_func`.

The kernel's `T` is where capability *enforcement* lives. The `Linker` decides what the component *sees*; `T` decides what each call *does*.

## Implications for Myrhiza — feature inventory

A condensed map of what Wasmtime gives us, what we wrap, and what we deliberately don't use.

**Load-bearing (we depend on these working):**

- Component model embedder API (`wasmtime::component::*`).
- Instance isolation (one `Store` per instance, no shared mutable memory).
- `Linker` as the only host-import surface.
- Fuel metering for `state-apply` determinism.
- Cranelift NaN canonicalisation for cross-CPU float consistency.
- `Engine::precompile_component` for warm-start latency.

**We wrap:**

- `wasmtime-wasi` — we expose a heavily filtered subset, never the default `wasi:cli` world. The kernel re-exports curated capabilities (storage, key access, peer messaging) under Myrhiza's own WIT package.
- Async / `call_async` — wrapped behind kernel scheduler; profile-specific.
- Resource limits (`Store::limiter` + fuel + epoch) — wrapped behind a profile-aware policy struct.

**We don't use:**

- `wasi:threads` and `shared` linear memory — incompatible with determinism.
- `wasm_relaxed_simd` — explicitly nondeterministic.
- `wasmtime-cli` — Myrhiza's daemon embeds the runtime directly.
- Any plan to snapshot live instances — we accept run-to-completion-per-event instead.

**Open questions** (tracked in [`open-problems.md`](open-problems.md)):

- Stability of `wasmtime::component` API across major versions: we will pin to a specific Wasmtime major and pay the migration cost on bumps.
- Determinism of fuel costs across Wasmtime versions: a v43-compiled module may consume different fuel than the same source compiled by v44. This affects pre-check ↔ apply equivalence; needs spec-level treatment.
- Whether to use Pulley (interpreter) for very-low-resource peers or require Cranelift availability everywhere.

For comparison with the other WASM-host designs in this prior-art set, see [`../holochain/`](../holochain/) (uses Wasmer, not Wasmtime; their constraints inform ours) and [`../agoric-endo/`](../agoric-endo/) (uses xsnap, not WASM at all — their solution to the "no live snapshot" problem is the alternative we are deliberately *not* taking).

## Sources

- Wasmtime repository — https://github.com/bytecodealliance/wasmtime
- Wasmtime release process documentation — https://docs.wasmtime.dev/stability-release.html
- Wasmtime API docs — https://docs.rs/wasmtime/44.0.1/wasmtime/
- `wasmtime` on crates.io — https://crates.io/crates/wasmtime (verified `max_stable_version` `44.0.1`, published 2026-04-30; `36.0.9` LTS published 2026-05-05)
- `wasmtime-wasi` on crates.io — https://crates.io/crates/wasmtime-wasi (`44.0.1`)
- `wasmtime-wasi-http` on crates.io — https://crates.io/crates/wasmtime-wasi-http (`44.0.1`)
- `cranelift-codegen` on crates.io — https://crates.io/crates/cranelift-codegen (`0.131.1`)
- `wit-bindgen` on crates.io — https://crates.io/crates/wit-bindgen (`0.57.1`)
- Wasmtime release tags (verified via GitHub API): v40.0.0 2025-12-22, v41.0.0 2026-01-20, v42.0.0 2026-02-24, v43.0.0 2026-03-20, v44.0.0 2026-04-20
- Bytecode Alliance — https://bytecodealliance.org/
- WebAssembly Component Model spec — https://github.com/WebAssembly/component-model
- `wasi-virt` — https://github.com/bytecodealliance/wasi-virt
- Wasmtime async docs — https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#method.async_support
- Wasmtime fuel / epoch docs — https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#method.consume_fuel and https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#method.epoch_interruption
