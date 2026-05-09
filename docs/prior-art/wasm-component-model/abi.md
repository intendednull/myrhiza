**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — Canonical ABI, lifting/lowering, adapters, shared-nothing linkage

# Component Model: Canonical ABI

## Why an ABI exists at all

A core wasm module speaks only `i32`, `i64`, `f32`, `f64`, plus linear memory and tables. WIT speaks records, variants, strings, lists, resources, futures, streams. Something has to translate. That something is the *Canonical ABI*, defined in `design/mvp/CanonicalABI.md` in `github.com/WebAssembly/component-model` (HEAD `669d494`, 2026-05-07; recent activity includes `a5a7af3` "introduce Store.{lift,lower}, simplify code a bit" 2026-04-27 and `6b01cc4` "Fix typos and inconsistencies in CanonicalABI.md" 2026-05-06).

The ABI defines two operations:

- **Lift** = "raw core wasm values + a slice of caller's linear memory → typed component-level value." Used when a component receives an argument or returns a value to its caller.
- **Lower** = "typed component-level value → core wasm values + write into callee's linear memory." Used when a component calls into another component (or into the host).

Every cross-boundary call is a *lower* on the caller's side and a *lift* on the callee's side. A given call thus crosses *two* linear memories — the caller's and the callee's — with the runtime as the trusted middleman that owns the lifted intermediate form. There is no shared memory between the two sides.

## Type lowering rules (preview2 baseline)

The ABI assigns each WIT type a representation as a flat list of core wasm values when passed by-value, and a representation in linear memory when passed indirectly. Core wasm calling conventions cap argument lists at a small number of values, so anything wider than the limit gets passed via a pointer to an in-memory layout.

Highlights, drawn from `CanonicalABI.md`:

- **Numerics** are 1:1 — `u8`/`s8`/`u16`/`s16`/`u32`/`s32` lower to `i32`; `u64`/`s64` to `i64`; `f32`/`f64` straight through. `bool` lowers to `i32` with 0/1 values.
- **`char`** lowers to `i32` holding the Unicode scalar value.
- **`record { a: T1, b: T2 }`** lowers as the flattened lowering of `T1` followed by `T2`. In memory it's a struct laid out per a deterministic field-ordering and alignment rule. The ABI does not preserve source-language struct padding; it imposes its own.
- **`tuple<T1, T2>`** = `record` with positional fields. Same lowering.
- **`variant { case-a, case-b(T) }`** lowers as `(tag: i32, payload-flat...)` where the payload is the maximum lowering across all cases (zero-padded for cases that don't use the slot). In memory: tag, then padding to the alignment of the payload, then the payload area sized to the max case.
- **`enum`** = variant with no payloads; lowers to `i32` tag.
- **`option<T>`** = the variant `{ none, some(T) }`; lowers as `(tag: i32, payload-flat-of-T)`.
- **`result<T, E>`** = the variant `{ ok(T), err(E) }`; same lowering rule.
- **`flags { ... }`** lowers as one or more `i32`s (bitset, packed).
- **`list<T>`** lowers as `(ptr: i32, len: i32)` pointing at a contiguous run of `T`s in linear memory. The ABI does not heap-allocate; it uses the `realloc` callback (below) to ask the callee's memory allocator for space.
- **`string`** lowers as `(ptr: i32, len-or-codepoints: i32)`. The byte interpretation is set by the *string encoding* canonical option.

### String encodings

The ABI lets the *adapter* (not the guest source language) declare which encoding it speaks: `utf8`, `utf16`, or `latin1+utf16` (a cheap variable-encoding for languages like Java that internally store either Latin-1 or UTF-16 depending on content). When two components disagree, the runtime transcodes during lift/lower.

This is one of the load-bearing decisions of the ABI: the substrate refuses to commit to a single string encoding because guest languages pre-existed the substrate. Transcoding cost is the price of language-agnosticism.

### Variant tag encoding

The tag is always an `i32`, sized to fit the case count. Values are assigned in source-order: case 0 is `0`, case 1 is `1`, etc. Adding a case at the end is forward-compatible at the bits level; reordering cases is a breaking change.

## Resources and handles

A WIT `resource` is an opaque host-managed object. From the guest, a resource is accessed only via a *handle* — an `i32` index into a per-component-instance table maintained by the runtime. There are two handle flavors:

- **`own<R>`** — the holder owns the resource; on drop, the runtime invokes the resource's destructor.
- **`borrow<R>`** — the holder has a lease; the lease is statically scoped to a single call (so the runtime can detect dangling borrows at the call boundary).

Lifts and lowers of handles do *not* copy the underlying object. They translate between the caller's table and the callee's table: lowering an `own` from caller to callee removes the entry from the caller's table and inserts it in the callee's table; lowering a `borrow` inserts a temporary entry in the callee's table that the runtime invalidates when the call returns.

This is the load-bearing capability primitive at the binary level. A component cannot manufacture a handle. It can only receive handles from imports or store handles it has been given. The runtime's tables are the substrate's reified capability list.

## `realloc` and `post-return`: the two callback hooks

Lifting a `list<T>` or `string` into a callee requires writing bytes into the callee's linear memory. The callee owns its allocator; the substrate doesn't. The ABI bridges this with two *canonical options* the callee declares per-export:

- **`realloc`** — a core-wasm function `(old_ptr, old_size, align, new_size) -> new_ptr`. The runtime calls it during lift to acquire memory it can write the lifted bytes into. By convention this points at the guest's allocator (e.g. a wrapped `malloc` for C, or `__component_realloc` for Rust's `wit-bindgen` runtime).
- **`post-return`** — an optional core-wasm function the runtime calls *after* a returned value has been read by the caller. Used to free out-parameters that the export wrote and the caller no longer needs (e.g. a returned `string` whose backing memory the export allocated).

Both hooks are *core* functions exported from the underlying core module, not component-level. They live at the lifting/lowering layer.

## Components vs core modules

A component is *not* a core wasm module. It is a wrapper format with its own binary layout (`design/mvp/Binary.md`). A component may contain:

- Zero or more *core modules* — the actual wasm code.
- Zero or more *core instances* — instantiations of those modules with imports satisfied.
- Zero or more *adapter modules* — special core modules that exist only to translate between two representations (most famously the WASI preview1 → preview2 adapter).
- One or more *type definitions* — WIT-level type information.
- Imports and exports at the component level, declared in WIT.
- *Canonical built-ins* — small instructions (`canon lift`, `canon lower`, `canon resource.new`, `canon resource.drop`, `canon resource.rep`, plus async-flavored ones like `canon stream.new`, `canon future.new`, `canon error-context.new`) that wire core functions to component functions.

The component file is a thin manifest declaring how the core modules and adapters are linked together with what types at the boundaries. A runtime that supports the Component Model loads the component, instantiates the core modules in dependency order, applies the adapters, and exposes the component-level exports.

## Adapters and canonical built-ins

An *adapter module* is a core wasm module the runtime treats specially: its imports/exports use the canonical-ABI representation rather than ordinary core types. Adapters exist to bridge between two representations of the same surface.

The canonical example: WASI preview1 (the old `__wasi_*` syscall set, used by every Rust/C component compiled before preview2 stabilized) is bridged to WASI preview2 by an adapter shipped with `wasi-libc`. Components targeting preview1 emit a core module with preview1 imports; the preview2 adapter is included in the component bundle; the adapter translates those preview1 calls into preview2 component-level imports. The runtime sees only preview2.

The full set of *canonical built-ins* is enumerated in `Explainer.md`'s "Canonical built-ins" section, including resource built-ins, concurrency built-ins (gated `🔀`), and error-context built-ins (gated `📝`).

## Shared-nothing linkage

The substrate's signature linkage decision: between any two components, *nothing is shared*. Not memory, not tables, not function references, not globals. Every value crossing a component boundary is lifted-then-lowered. Every resource crossing a boundary is rebound through the runtime's table.

This is the inverse of core wasm's *shared-everything* linkage, where two core modules instantiated in the same instance can directly share linear memory and table entries. Inside a single component, the contained core modules can do shared-everything linkage with each other (that's how a component is built — multiple core modules linked together with shared memory). Across component boundaries, that's forbidden.

The asymmetry has a load-bearing consequence: a component is the unit of mutual distrust. Two core modules inside one component trust each other (they share memory). Two components do not (they cannot). The runtime enforces this at the binary level by simply not making the linkage tools available across components.

## Implications for Myrhiza

Myrhiza's kernel is the host that satisfies a component's imports. The ABI determines the shape of that host interface at the binary level.

- **Every host import is a lift on the kernel side.** When a component calls `myrhiza:state/get-snapshot`, the kernel receives the lowered representation (i32s, pointers into the component's linear memory) and lifts to a typed value. Errors during lift — malformed lists, invalid variant tags, dangling resource handles — are kernel-detected and become traps. The kernel never sees raw guest memory; it sees lifted values.
- **Resources are how the kernel lends capabilities.** A peer handle, an authority token, a state subscription — each is a WIT `resource` whose representation the kernel owns. When the component drops the handle (or the runtime drops it on component teardown), the kernel's destructor runs. Revocation = drop the entry. This is exactly the same pattern [Spritely OCapN](../spritely-ocapn/) and [Agoric's bundle-loader](../agoric-endo/modules-and-bundling.md) use for capability lifecycle, just reified at the wasm binary layer.
- **Determinism + the ABI.** A `state-apply` component's lift behavior is deterministic: same lowered bytes → same lifted value. Determinism breaks if the host hands the component a non-deterministic resource (a clock handle, a random source). The kernel enforces determinism by *not exposing those resources in `state-apply` worlds*, not by some ABI-level toggle. This matches CLAUDE.md's stance that determinism is a property of the world type, not a runtime flag.
- **String encoding choice is per-export.** Myrhiza's host imports should declare `utf8` uniformly. The substrate's transcoding flexibility is a guest-language accommodation, not a kernel-side knob — the kernel speaks one encoding.
- **`realloc` cost.** Every host → component call that returns a `list<T>` or a `string` involves a `realloc` call into the guest. This is non-trivial; it's a guest-allocator round-trip per call. For hot-path host imports, prefer fixed-size types or `option<T>` over lists.
- **Shared-nothing across apps, shared-everything within an app.** A Myrhiza application is a *bundle of components*. Inside that bundle, components can be linked with shared-nothing linkage too (that's the only option), so even within an app the components don't share memory. This is stronger isolation than typical capability-OS designs and is a property worth exploiting in the spec.
- **No ambient kernel.** The kernel itself is not a component. It's the runtime. But every kernel-side capability the guest sees is exposed only via imports declared in the world. A guest that does not import `myrhiza:peer` literally cannot reach peers — there is no syscall-equivalent. This is the core-wasm property the substrate inherits unchanged.

For where the lifted/lowered values come from in tooling, see [tooling.md](tooling.md). For the runtime that does the actual lift/lower work, see [wasmtime.md](wasmtime.md). For known performance / sharp-edge critiques of the ABI, see [critiques.md](critiques.md) and [open-problems.md](open-problems.md).

## Sources

- `https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md` — primary ABI document, 63455 bytes, verified via `gh api` 2026-05-09.
- `https://github.com/WebAssembly/component-model/blob/main/design/mvp/Explainer.md` — canonical built-ins, gated features list.
- `https://github.com/WebAssembly/component-model/blob/main/design/mvp/Binary.md` — component binary format.
- `https://github.com/WebAssembly/component-model/commit/a5a7af3` — recent CABI refactor (Store.{lift,lower}), verified 2026-04-27.
- `https://github.com/WebAssembly/component-model/commit/6b01cc4` — recent CABI typo fix, verified 2026-05-06.
- `https://github.com/WebAssembly/component-model/commit/669d494` — HEAD as of 2026-05-09.
- `https://component-model.bytecodealliance.org/advanced/canonical-abi.html` — Bytecode Alliance Component Model book, ABI chapter.
