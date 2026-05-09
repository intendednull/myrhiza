**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — WASI preview1 / preview2 / preview3 lineage and stability status

# WASI preview status (preview1 / preview2 / preview3)

WASI is the [WebAssembly System Interface](https://github.com/WebAssembly/WASI), the standard library that gives WASM components a portable view of the outside world. It has gone through three numbered "preview" generations, each a different ABI and a different design philosophy. Myrhiza needs to pick which preview its applications target — the choice fixes the wire format of every host import for the next several years.

This document is the snapshot of where the previews stand as of 2026-05-09, with verified version pins. Companion files: [`wasmtime.md`](wasmtime.md), [`spec.md`](spec.md), [`abi.md`](abi.md), [`tooling.md`](tooling.md), [`languages.md`](languages.md), [`ecosystem.md`](ecosystem.md), [`history.md`](history.md), [`open-problems.md`](open-problems.md), [`lessons.md`](lessons.md), [`critiques.md`](critiques.md).

## Preview1 — `wasi_snapshot_preview1`

The original. A single core-WASM module declaring imports from the `wasi_snapshot_preview1` namespace, with a POSIX-shaped surface: file descriptors, `args_get`, `environ_get`, `clock_time_get`, `fd_read`, `fd_write`, `path_open`, etc. Predates the Component Model; types are flat C-ABI shapes packed into linear memory.

Properties:

- **Still ubiquitous.** Almost every WASM toolchain (clang, Rust `wasm32-wasip1` target, Go, Zig, AssemblyScript) emits preview1 by default. The CLI host ecosystem (Wasmtime CLI, WasmEdge, Wasmer) all keep a preview1 implementation.
- **Not component-model native.** Preview1 modules are core modules. They can be *adapted* into components by `wasm-tools component new --adapt wasi_snapshot_preview1=adapter.wasm`, using the [Bytecode Alliance preview1-to-component adapter](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-preview1-component-adapter). The adapter is a small WASM module that translates preview1 calls into preview2 component-model calls; this is how a `wasm32-wasip1` Rust binary ends up runnable as a component.
- **No semantic versioning.** It is *the* preview1; there is one. The snapshot was frozen years ago and is not the active surface for new design work.
- **Limited.** No HTTP, no proper sockets, no async, no resource handles, primitive type system.

### Implications for Myrhiza — preview1

Preview1 is a *legacy compatibility surface*, not a target. Myrhiza apps will be authored against preview2 (or preview3 once stable). The preview1-adapter exists if a Myrhiza app wants to bundle a preview1 binary as a component, but the kernel does not need to wire `wasi_snapshot_preview1` directly — the adapter handles it.

## Preview2 — the `wasi:*` 0.2.x worlds

Preview2 is the first componentized WASI. Each capability is a separate WIT package under the `wasi` namespace, versioned independently using semver-shaped tags. As of 2026-05-09 the WASI spec repo's most recent stable tag is **`v0.2.11`** (published 2026-04-07), and verified preview3 release-candidate work proceeds under tags `v0.3.0-rc-2026-01-06`, `v0.3.0-rc-2026-02-09`, `v0.3.0-rc-2026-03-15`.

Note: the `0.2.x` numbering on WASI release tags reflects the *spec repo's own version*, which advances faster than (and is independent from) the per-package version numbers embedded in WIT files like `wasi:io@0.2.3`. The interface-level pinned versions are what matter for ABI compatibility.

### The preview2 interface set

The packages and their roles:

| Package | Purpose |
|---|---|
| `wasi:io` | Streams and pollables. The substrate everything else builds on. `input-stream`, `output-stream`, `pollable`. |
| `wasi:clocks` | `wall-clock` and `monotonic-clock`. |
| `wasi:random` | `get-random-bytes`, plus an insecure variant for testing. |
| `wasi:filesystem` | `descriptor`-based file API; resource-typed handles. |
| `wasi:sockets` | `tcp`, `udp`, `instance-network`, `ip-name-lookup`. |
| `wasi:cli` | Command-line worlds (`wasi:cli/command`, `wasi:cli/environment`, stdin/stdout/stderr). The default "world" for a CLI tool. |
| `wasi:http` | `incoming-handler`, `outgoing-handler`, `types`. The first non-trivial capability that depends on `wasi:io`. |

Within `wasi:io` itself, the canonical pinned version most current tooling has consolidated on is `0.2.x` (e.g. `wasi:io@0.2.0`, `wasi:io@0.2.1`, `wasi:io@0.2.2`, `wasi:io@0.2.3`). Each pinned version is a frozen WIT file; once a `0.2.N` is published, its definition is immutable. New `0.2.M` for `M > N` may add features but is expected to be wire-compatible for callers that only use `0.2.N` types — this is the [WIT semver compatibility rule](https://github.com/WebAssembly/component-model/blob/main/design/mvp/Semver.md).

The interface uses *resources* (handle types with `own<T>` and `borrow<T>` semantics) — these are component-model native and have no preview1 analogue. `wasi:filesystem/types.descriptor` is a resource; you cannot smuggle a `descriptor` across a host boundary as an integer.

### Wasmtime ↔ WASI version matrix

The `wasmtime-wasi` crate is the host-side implementation of preview2. It moves in lockstep with `wasmtime`: both publish the same major version on the same monthly cadence (verified: `wasmtime` `44.0.1` and `wasmtime-wasi` `44.0.1` were both published 2026-04-30).

What `wasmtime-wasi` 44.x actually implements is a *specific* set of `wasi:*@0.2.N` interface versions, baked into the crate's WIT files. The WIT versions are not the same as the `wasmtime-wasi` crate version — this is the most common point of confusion. To know which `wasi:*@0.2.N` your kernel exposes, look at the WIT files vendored under `crates/wasi/wit/deps/` in the Wasmtime repository at the tag matching your `wasmtime-wasi` version.

### Implications for Myrhiza — preview2

Pinning to preview2 means:

- We commit to specific `wasi:*@0.2.N` versions in our world definitions. The kernel's host re-exports refer to those exact versions.
- Async behavior must be hand-rolled. Preview2's `pollable` + `wasi:io/poll.poll` is a poll-list primitive that Wasmtime hides behind its fiber-based fake-async (see [`wasmtime.md`](wasmtime.md)). For Myrhiza this means our kernel implements the host side of `wasi:io/poll`, integrating with our async runtime.
- We **own the async story.** If a guest wants to wait on multiple I/O events, it composes `pollable`s and calls `poll`. The kernel's job is to make that work, with Myrhiza's scheduler underneath.
- Preview2 is **stable** in the sense that `0.2.N` interfaces are immutable. No surprise ABI breaks within a pinned version. New `0.2.(N+1)` is additive.

The cost: the async ergonomics for guests are clunky relative to preview3. Library authors in Rust / C / Go will write more boilerplate than they would against native `async fn` in WIT.

## Preview3 — native async, in flight

Preview3 is the next major iteration. It introduces:

- **`async fn` in WIT.** Imports and exports can be marked `async`; the canonical ABI lowers them into the [component-model async machinery](https://github.com/WebAssembly/component-model/blob/main/design/mvp/Async.md). Fibers are spec-level, not just an embedder trick.
- **`stream<T>` and `future<T>`** as first-class WIT types. A function can return a stream of bytes (or any type) without manually composing `pollable`s.
- **Cancellation** as a spec-defined operation on a `future`/`stream` handle.
- **Reduced poll-loop boilerplate.** The `wasi:io/poll.poll` primitive becomes an implementation detail; guest code uses `await`-style ergonomics.

### Verified preview3 status as of 2026-05-09

Preview3 has been on the WebAssembly project's roadmap since 2023 and was originally targeted for late-2024. It slipped. Verified facts from the WASI repository:

- The `WebAssembly/WASI` repo's most recent stable tag is **`v0.2.11`**, published **2026-04-07**.
- Three preview3 release candidates have been cut: **`v0.3.0-rc-2026-01-06`**, **`v0.3.0-rc-2026-02-09`**, **`v0.3.0-rc-2026-03-15`**.
- A `v0.3.0` final tag has *not* been published as of 2026-05-09.

Open issues on `WebAssembly/component-model` (sample of recent open issues, verified 2026-05-09): #648 ("Why is it `(dtor (func n))` instead of `(dtor (core func n))`?"), #647 ("Prevent `waitable-set.{wait,poll}` from being used at the same time as sync built-ins"), #646 ("Only allow 'async' ABI options for 'async'-typed function imports/exports"), #642 ("Interaction between synchronous stream/future read/write and various operations"), #640 ("Define bounded lists"). Several of these are async-machinery design questions whose resolution gates `v0.3.0` final.

So: preview3 is in the **release-candidate** phase, with three RCs cut over a three-month span, and core async-semantics issues still open. A reasonable read of the trajectory is that preview3 *final* could land in mid-to-late 2026, but that is a projection, not a verified date. Myrhiza spec authors should treat preview3 as "expected within the next several months, not yet shippable."

### Wasmtime ↔ preview3

Wasmtime 44.x ships preliminary preview3 support — specifically, the host plumbing for `stream<T>`, `future<T>`, and `async` ABI lowering — but does not yet expose stable preview3 worlds in `wasmtime-wasi`. The `wasmtime-wasi` crate at 44.0.1 still pins to preview2. Once `WebAssembly/WASI v0.3.0` is final, expect a Wasmtime release that publishes a `wasmtime-wasi-p3`-shaped crate (or an updated `wasmtime-wasi`) targeting it. **Until then, building against preview3 means tracking RC tags directly and accepting churn between RCs.**

### Tooling

The `@bytecodealliance/preview2-shim` npm package (used by [jco](https://github.com/bytecodealliance/jco) for browser-side preview2 hosting) is at **`0.17.9`**, published **2026-04-17** — a useful liveness signal that preview2 tooling is still actively maintained while preview3 stabilises. There is no `preview3-shim` published yet.

## WIT version pinning conventions

WASI follows the [WIT semver rules](https://github.com/WebAssembly/component-model/blob/main/design/mvp/Semver.md):

- **`@0.x.y`** — pre-1.0. Each `0.x` is treated as a separate, ABI-incompatible major. Within an `0.x`, point releases (`0.x.y` → `0.x.(y+1)`) are additive and wire-compatible.
- **`@1.x.y` and beyond** — full semver. Major bumps break ABI; minor bumps are additive; patch bumps are bugfix-only.

So `wasi:io@0.2.3` is wire-compatible with a host implementing `wasi:io@0.2.0` *for callers that use only 0.2.0-vintage types*. A caller using a `0.2.3`-only type cannot fall back. This shapes how kernel re-exports work: Myrhiza can advance its pinned `wasi:io` patch level over time and existing apps keep working, as long as we never *remove* a type.

`wasi:io@0.2.x` and `wasi:io@0.3.x` are *not* compatible. The `0.x` change is a major version bump under WIT's pre-1.0 rules.

## Implications for Myrhiza — preview2 vs preview3 choice

**Pin preview2 today, plan for preview3 when v0.3.0 final ships.**

Pros of preview2 today:

- Stable. `wasi:*@0.2.N` interfaces are frozen; no churn.
- `wasmtime-wasi` 44.x ships a complete implementation we can wrap.
- Toolchain support is broad: `cargo-component`, `wit-bindgen`, `jco`, `wasm-tools` all target preview2.
- Myrhiza does not depend on a not-yet-stable spec for anything load-bearing.

Cons of preview2 today:

- We own the async story for guests. Our WIT worlds will use `pollable`-shaped patterns for anything async-ish, which is more boilerplate than preview3's native `stream`/`future`.
- When preview3 lands we will face an ABI migration — every guest will need to be rebuilt against the preview3 world, and the kernel will need to either dual-host or run a preview2-to-preview3 adapter.

Pros of waiting for preview3:

- Better async ergonomics for guests.
- Spec-level cancellation, native streams.
- Less Myrhiza-specific WIT design — we ride the standard.

Cons of waiting for preview3:

- We do not know when v0.3.0 final ships.
- During RC churn, every change forces rebuilds.
- Wasmtime preview3 host support is not yet stable; the kernel would have to track Wasmtime's main branch.

**Decision: preview2 is the load-bearing target.** Myrhiza ships its first runtime with preview2-pinned host interfaces, and we treat the preview3 transition as a known future cost. The transition will be tracked in a future migration plan under `docs/plans/` once v0.3.0 ships and Wasmtime's preview3 surface is stable. See [`open-problems.md`](open-problems.md) for the running list of preview-related open questions.

For comparison: [Holochain's WASM use](../holochain/architecture.md) is preview1 + Wasmer, with no component-model adoption planned in their current roadmap; they accept the limitations because they don't need cross-language interop. [Agoric's xsnap](../agoric-endo/architecture.md) does not use WASI at all. Myrhiza's choice to embrace preview2 (and eventually preview3) is what makes us the polyglot P2P runtime — the preview lineage is precisely what other runtimes traded off against.

## Sources

- WebAssembly WASI repository — https://github.com/WebAssembly/WASI
- WASI release tags (verified via GitHub API): `v0.2.11` (2026-04-07), `v0.3.0-rc-2026-03-15`, `v0.3.0-rc-2026-02-09`, `v0.2.10` (2026-02-03), `v0.3.0-rc-2026-01-06`
- WebAssembly Component Model repository — https://github.com/WebAssembly/component-model
- Component Model async design — https://github.com/WebAssembly/component-model/blob/main/design/mvp/Async.md
- WIT semver rules — https://github.com/WebAssembly/component-model/blob/main/design/mvp/Semver.md
- `wasi_snapshot_preview1` — https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md
- Wasmtime preview1-to-component adapter — https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-preview1-component-adapter
- `wasmtime-wasi` on crates.io (44.0.1, published 2026-04-30) — https://crates.io/crates/wasmtime-wasi
- `wasmtime-wasi-http` on crates.io (44.0.1, published 2026-04-30) — https://crates.io/crates/wasmtime-wasi-http
- `@bytecodealliance/preview2-shim` on npm (0.17.9, 2026-04-17) — https://www.npmjs.com/package/@bytecodealliance/preview2-shim
- `jco` — https://github.com/bytecodealliance/jco
- Component Model open issues sampled 2026-05-09 (#648, #647, #646, #642, #640) — https://github.com/WebAssembly/component-model/issues
