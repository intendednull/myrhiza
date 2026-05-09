**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — what the spec is, WIT, the four-pass compile model, WASI preview2 set

# Component Model: Spec Surface

## What "the Component Model" actually is

The WebAssembly Component Model is a specification effort hosted at `https://github.com/WebAssembly/component-model`. It is *not* a separate runtime; it is a layered binary format and IDL that wraps core WebAssembly modules to give them typed, language-agnostic interfaces. A component is a wrapper that bundles one or more core modules, optional adapter modules, and a declarative description of imports/exports expressed in WIT (Wasm Interface Types).

The repository contains the spec text under `design/`, split between `design/high-level/` (overviews) and `design/mvp/` (the load-bearing documents). The four documents that anyone writing against the substrate must know:

- `design/mvp/Explainer.md` — the AST-level explainer for the component grammar, gated features, and the JS embedding sketch.
- `design/mvp/WIT.md` — the WIT IDL grammar and package format.
- `design/mvp/CanonicalABI.md` — lifting/lowering rules; see [abi.md](abi.md).
- `design/mvp/Binary.md` — wire format for components.
- `design/mvp/Concurrency.md` — async / streams / futures semantics (preview3 territory).

Activity is live as of 2026-05-09: HEAD is `669d494` (2026-05-07, "Restrict all `context.{get,set}` in same component to use same elem type"). The repository uses tags only for proposal milestones, not GitHub releases — `gh api .../releases` returns empty. Spec versions are tracked as proposal phases, not semver.

## WIT (Wasm Interface Types)

WIT is the developer-facing IDL. A WIT *package* is a directory of `.wit` files sharing one `package <ns>:<name>@<semver>;` declaration. Packages contain *interfaces* (named bundles of functions + types) and *worlds* (named bundles of imports + exports a component conforms to).

### Primary type set (per `WIT.md`)

Numeric: `u8 u16 u32 u64 s8 s16 s32 s64 f32 f64`. `char` is a Unicode scalar value. `bool`. `string` is a Unicode string (encoding negotiated at the ABI layer — see [abi.md](abi.md)).

Container / aggregate types:

- `record { name: T, ... }` — C-struct-like, named fields.
- `variant { case-a, case-b(T), ... }` — discriminated union, optional payload per case.
- `enum { a, b, c }` — variant with no payloads.
- `option<T>` — sugar for `variant { none, some(T) }`.
- `result<T, E>` / `result<T>` / `result<_, E>` / `result` — sugar for the success/failure variant; any of the type slots may be omitted.
- `tuple<T1, T2, ...>` — positional record.
- `list<T>` — variable-length sequence.
- `flags { a, b, c }` — bitset.

Reference / capability-shaped types:

- `resource <name> { constructor(...); method-name: func(...) -> ...; }` — owns an opaque host-side object accessed via handles. Handles come in `own<R>` (transfer-of-ownership) and `borrow<R>` (lease) flavors. Resources are the substrate's capability primitive at the type level.

Asynchronous types (gated `🔀` in the explainer; landing in preview3):

- `future<T>` — single-value promise.
- `stream<T>` — multi-value channel.
- `error-context` — first-class error with structured debug info, gated `📝`.

A WIT *function* signature is `name: func(p1: T, ...) -> T` or `-> ()`. Functions live inside an interface or directly inside a world.

### Interfaces, worlds, packages

```wit
package wasi:clocks@0.2.11;

interface monotonic-clock {
    type instant = u64;
    now: func() -> instant;
}

world imports {
    import monotonic-clock;
}
```

- `interface` = a named function/type bundle.
- `world` = a named *signature* a component implements: a list of `import <iface>;` and `export <iface>;` declarations. A world is what a component is type-checked against; an interface is what fills the slots.
- `package` = a versioned namespace that owns a set of interfaces and worlds.

Two kinds of cross-package reference exist: type imports (`use other-pkg/iface.{type-a};`) and interface imports/exports (`import wasi:clocks/monotonic-clock@0.2.11;`).

### `wasm-tools component wit`

`wasm-tools` (releases on `bytecodealliance/wasm-tools`, latest verified `v1.248.0` 2026-04-28) round-trips between binary components and WIT text:

```
wasm-tools component wit my-component.wasm           # extract WIT
wasm-tools component embed world.wit core.wasm -o c  # embed a world type into a core module
wasm-tools component new core.wasm -o component.wasm # promote core module to component
```

The `embed` + `new` pair is the canonical way a guest toolchain produces a component without itself understanding the component binary format.

## The four-pass compile model

A guest component is built in four conceptual passes. The pass names below match Bytecode Alliance toolchain conventions; the spec itself does not name them, but the substrate is structured around this pipeline.

1. **WIT → bindings.** Per-language tools (`wit-bindgen` for Rust/C/etc.; `componentize-js` for JS; `componentize-py` for Python) read a WIT world and emit guest-language stubs. Each stub does the work of marshalling the language's native types into core wasm primitives that the canonical ABI accepts.

2. **Guest source → core wasm module.** The host language's normal toolchain (rustc, clang, etc.) compiles guest source plus generated bindings into a core `.wasm` module. The module's imports and exports use core wasm signatures; the bindings made it possible to express the WIT-typed interface in those signatures.

3. **Core wasm → component.** `wasm-tools component embed` writes the WIT world into a custom section of the core module; `wasm-tools component new` then wraps it into a component, inserting any *adapter modules* needed (most commonly the WASI preview1 → preview2 adapter, for guests that still target the old WASI interface).

4. **Component link / compose.** Optionally, multiple components are composed via `wasm-tools compose` (or, increasingly, via WAC — the WebAssembly Composition language at `bytecodealliance/wac`). The composer wires one component's exports into another's imports, again at the WIT level.

The point of separating the four passes: the only thing that changes when you swap guest language is pass 1. Passes 2–4 are language-neutral.

## Imports vs exports vs worlds

A component is a black box with exactly two surfaces: what it *imports* (host-provided capabilities) and what it *exports* (its own functionality). The world type describes both surfaces simultaneously.

This is the load-bearing invariant for capability-style hosts: a component cannot reach outside the substrate except via its declared imports. There is no syscall, no ambient FS, no implicit network. Every host effect is an import named in the world. Compare [Iroh's transport story](../iroh/) and [Agoric's bundle hashing](../agoric-endo/modules-and-bundling.md) for parallel "no ambient authority" models.

## The WASI preview2 interface set

WASI is a *separate* spec organization (`WebAssembly/WASI` on GitHub) that publishes a coordinated bundle of standard worlds and interfaces. Preview2 (also written p2 or 0.2.x) is the current stable milestone. Each subsystem is a separate repo with its own tags. As of 2026-05-09:

| Repo | Latest stable tag | Latest RC tag |
|---|---|---|
| `wasi-io` | `v0.2.11` | (no RC) |
| `wasi-cli` | `v0.2.11` | `v0.3.0-rc-2026-03-15` |
| `wasi-http` | `v0.2.11` | `v0.3.0-rc-2026-03-15` |
| `wasi-filesystem` | `v0.2.11` | `v0.3.0-rc-2026-03-15` |
| `wasi-clocks` | `v0.2.11` | `v0.3.0-rc-2026-03-15` |
| `wasi-random` | `v0.2.11` | `v0.3.0-rc-2026-03-15` |
| `wasi-sockets` | `v0.2.11` | `v0.3.0-rc-2026-03-15` |
| `wasi-keyvalue` | `v0.2.0-draft` | — |

The preview2 set's load-bearing interfaces, in rough dependency order:

- **`wasi:io/streams`, `wasi:io/poll`, `wasi:io/error`** — the substrate for byte streams. Everything else with byte I/O resolves to `input-stream` / `output-stream` resources from `wasi:io`. `wasi:io` is the substrate's lowest layer; anything async-ish in preview2 is poll-driven through `pollable` resources.
- **`wasi:clocks`** — `monotonic-clock` (instants) and `wall-clock` (datetimes). Determinism note: a deterministic component cannot import `wall-clock` directly without a kernel-side substitute.
- **`wasi:random`** — `random` (non-deterministic) and `insecure` and `insecure-seed`. Same determinism caveat.
- **`wasi:filesystem`** — `types` (descriptor + dir-entry-stream resources) and `preopens` (the capability-list of root descriptors a runtime grants).
- **`wasi:sockets`** — `tcp`, `udp`, `instance-network`, `ip-name-lookup`. Network access is a granted resource.
- **`wasi:cli`** — `environment`, `exit`, `stdin`, `stdout`, `stderr`, `terminal-*`. The "I am a Unix-shaped program" world.
- **`wasi:http`** — proxy world for HTTP servers and clients, built on `wasi:io`.

What's standardized vs in flight:

- **Preview2 / 0.2.x — standardized.** Frozen interface shapes, multi-runtime support (Wasmtime, jco preview2-shim, others). What you write specs *against* today.
- **Preview3 / 0.3.x — in flight.** Native async (replaces poll-driven preview2 streams). Three release candidates have been cut so far (`v0.3.0-rc-2026-01-06`, `v0.3.0-rc-2026-02-09`, `v0.3.0-rc-2026-03-15`); no `v0.3.0` final. `wasi-keyvalue` is still pre-1.0 draft (`v0.2.0-draft`) — pre-1.0 packages have erratic version histories, so do not assume a recent up-bump exists. Preview3 is *not* what specs should commit to as a baseline yet, but a forward-looking design should anticipate the async transition.

The preview3 transition is the single biggest in-flight change in the substrate. The Concurrency.md document drives it; the most recent Concurrency.md PR merged 2026-05-07 (`#643`, "Add 'Component Instance Lifetime' section to Concurrency.md").

## Implications for Myrhiza

Myrhiza commits to the Component Model as the application substrate, with a kernel that brokers all I/O. That commits Myrhiza to a small number of derived constraints:

- **Worlds are the contract.** Each Myrhiza component profile (`state-apply`, `state-propose`, `interaction`, `behavior` — see CLAUDE.md) is a distinct WIT world the kernel checks at load time. The world type is the load-bearing artifact for sandbox decisions, not anything in the wasm binary itself.
- **No ambient WASI.** A `state-apply` world must not import `wasi:clocks/wall-clock`, `wasi:random`, or anything else non-deterministic. A `state-apply` world's complete import list *is* its determinism proof. The kernel rejects any component whose world references a forbidden interface.
- **Custom WIT packages, not WASI re-use.** Myrhiza's host imports (`myrhiza:state`, `myrhiza:authority`, `myrhiza:peer`, etc., names TBD) are first-class WIT packages, not WASI extensions. Reusing WASI-preview2's `wasi:io/streams` for byte channels is fine and recommended; reusing `wasi:filesystem` is wrong because the kernel does not expose a filesystem.
- **Adopt 0.2.x now, plan for 0.3.x.** Spec against preview2 today. Keep the async-transition cost in mind: the design should not bake in poll-driven idioms in places where streams or futures are the obvious right shape post-preview3.
- **Resources = capabilities.** Use WIT `resource` for every capability the kernel lends to a component (a peer handle, a state-stream subscription, a key-derivation slot). Handles are i32 indices into a per-component table — see [abi.md](abi.md) — which means revoke-on-drop is a property the kernel enforces by dropping the table entry.

For the toolchain dimensions of these implications, see [tooling.md](tooling.md) and [wasmtime.md](wasmtime.md). For the browser-side substrate, see [browser.md](browser.md). For where preview3 is in the standardization pipeline, see [preview-status.md](preview-status.md).

## Sources

- `https://github.com/WebAssembly/component-model` — repo root.
- `https://github.com/WebAssembly/component-model/blob/main/design/mvp/Explainer.md` — verified from `gh api repos/WebAssembly/component-model/contents/design/mvp/Explainer.md`, head of repo at `669d494` 2026-05-07.
- `https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md` — verified.
- `https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md` — verified to exist (63455 bytes).
- `https://github.com/WebAssembly/component-model/blob/main/design/mvp/Binary.md`
- `https://github.com/WebAssembly/component-model/blob/main/design/mvp/Concurrency.md` — most recent meaningful PR `#643` 2026-05-07.
- `https://github.com/WebAssembly/WASI` — WASI org root.
- `https://github.com/WebAssembly/wasi-cli/tags`, `wasi-http`, `wasi-filesystem`, `wasi-clocks`, `wasi-random`, `wasi-sockets`, `wasi-io`, `wasi-keyvalue` — tags verified via `gh api` 2026-05-09.
- `https://github.com/bytecodealliance/wasm-tools/releases` — `v1.248.0` 2026-04-28 verified via `gh api`.
- `https://component-model.bytecodealliance.org/` — Bytecode Alliance Component Model book (companion docs to the spec).
