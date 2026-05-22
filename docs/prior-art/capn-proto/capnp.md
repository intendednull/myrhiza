**Date:** 2026-05-22
**Status:** active
**Subject:** Cap'n Proto — binary serialization format, schema language, RPC system (C++ reference impl + multi-language bindings)

# Cap'n Proto

The original project: a binary interchange format and capability-based RPC system. Created by Kenton Varda starting in 2013, originally inside Sandstorm; the C++ reference implementation is now primarily developed at Cloudflare as part of `workerd`.

## Three things in one project

Cap'n Proto bundles three distinct designs under one name:

1. **The wire format** — a zero-copy, mmap-friendly binary encoding. Designed to avoid the encode/decode step entirely: the on-disk and in-memory representations are the same.
2. **The schema language** — a `.capnp` file format (similar in spirit to Protobuf `.proto`) declaring structs, unions, lists, interfaces, generics, annotations, and explicit field numbering for evolution.
3. **The RPC protocol** — a four-level capability-based RPC layer derived from Mark Miller's CapTP. Built on top of the wire format. Production deployment is currently Level 1 (object refs + promise pipelining); Levels 2-4 are partially implemented.

These three are independent — many Cap'n Proto users use only the wire format (serialization-only bindings exist for Java, D, Lua, Ruby, Nim, Scala) and never touch RPC.

## Wire format

Words are 64-bit aligned. Messages are split into segments that can be loaded into contiguous blocks of memory (mmap-friendly). Primitive types must always be aligned to a multiple of their size. Pointers carry offsets, types, list lengths — readers traverse the pointer graph lazily.

The marketing claim is "**infinitely faster**" (literal banner on capnproto.org), justified by: no parsing step on read for the basic case; canonical form is well-defined; lazy validation gives O(1) cold-path access. Per the Cap'n Proto FAQ: *"Cap'n Proto's serialization layer is designed to be safe against malicious input"* — but *"the C++ reference implementation has not yet undergone a formal security review."*

A separate **packed encoding** strips zero bytes to compete with Protobuf size-on-wire while keeping the unpacked-zero-copy advantage on the hot path. Per the FAQ: packing achieves *"similar encoding size to Protocol Buffers while still being faster."*

CVE history: integer overflow / underflow in pointer validation (2015-03-02), CPU amplification (2015-03-05), CVE-2022-46149 list-of-pointers OOB read (2022-11-30). All disclosed transparently on the news page.

## Schema language

`.capnp` files declare:

- **Primitives:** `Void`, `Bool`, `Int8`–`Int64`, `UInt8`–`UInt64`, `Float32`, `Float64`, `Text`, `Data`, parameterized `List(T)`.
- **Structs** with numbered fields (for evolution).
- **Unions** as a property of struct fields — *"Only one of these fields can be set at a time, and a separate tag is maintained to track which one is currently set."* Named and unnamed.
- **Groups** for nesting without changing the wire layout.
- **Enums** with numbered values.
- **Interfaces** with numbered methods, returning structs, supporting (multiple) inheritance. Interfaces can be passed by reference over RPC — this is how capability-based RPC works.
- **Generics** on structs and interfaces. *"Only pointer types (structs, lists, blobs, and interfaces) can be used as generic parameters."*
- **Annotations** for attaching metadata to any element.
- **Unique IDs:** every file, struct, field has a 64-bit ID. The `capnp id` command generates new ones; collisions are practically impossible.

The schema language is the unit of cross-implementation interop. A Rust client speaks to a C++ server because both run code generated from the same `.capnp` file.

**Backward/forward compatibility rules** are explicit and unforgiving. Safe changes: adding new types, adding new fields/enumerants/methods at *higher* numbers, renaming symbols while preserving IDs, moving fields into new unions. The spec says: *"Any change not listed above should be assumed NOT to be safe."*

## RPC: four levels, promise pipelining

Cap'n Proto's RPC layer is the load-bearing claim against Spritely. The RPC spec opens with: *"Cap'n Proto's RPC protocol is based heavily on CapTP, the distributed capability protocol used by the E programming language."* That is the verbatim canonical lineage statement.

Four levels are specified, mirroring CapTP:

| Level | Capability | Status in Cap'n Proto |
|---|---|---|
| **1** | Object references + promise pipelining | Implemented in C++ reference and capnproto-rust. The base of all RPC. |
| **2** | Persistent capabilities saved/restored across connections (sturdyrefs / `SaveAs`) | Partially implemented; ABI specified but applications mostly bring their own persistence. |
| **3** | Three-way introductions (direct connection between two peers introduced by a third) | Specified but not implemented in the reference impl. The hard one. |
| **4** | Reference equality / joining (verify multiple capabilities point to the same object) | Specified but not implemented. |

Production Cap'n Proto traffic — including Cloudflare Workers RPC — is **Level 1 only** plus app-layer persistence on top. The CapTP ambition was levels 1-4 universally; what shipped was the Level-1 subset that solves the round-trip-latency problem (promise pipelining) and the capability-passing problem (interface pointers in struct fields).

### Promise pipelining

The single feature Cap'n Proto is most cited for. The RPC spec: *"when calling `bar(foo())`, both messages can be sent simultaneously rather than waiting for `foo()` to complete first."* This collapses *N* sequentially-dependent RPCs into 1 round trip regardless of diamond dependencies in the call graph.

Promises are first-class. They return immediately; you can pass a promise to another RPC call as if it were the resolved value, and the receiver buffers the call until resolution. This is the same shape that GraphQL's "flatten REST waterfalls" insight is — but at a lower level and without a separate query language. The Cap'n Web blog (2025-09-22) frames it: *"GraphQL gave us a way to flatten REST's waterfalls. Cap'n Web lets us go even further: it gives you the power to model complex interactions exactly the way you would in a normal program."*

## C++ reference implementation + KJ

The C++ implementation includes **KJ**, an alternative C++ standard library that Cap'n Proto uses internally. KJ predates Cap'n Proto's wider success and has become the *raison d'être* of the 2.0 / `v2` branch (see release history).

The `master` branch is the **1.0 LTS** line (frozen API since 2023-07-28). The `v2` branch is the upcoming 2.0 line; per Varda's 2023-07-28 announcement: *"v2 will require C++20/C++23 with coroutines"* and includes major breaking changes to *both* the KJ toolkit and the C++ API surface — *"motivated by experience building Cloudflare Workers runtime (workerd)."* The wire format and RPC protocol are unchanged; the breakage is the C++-side ergonomics.

## Cap'n Proto for Rust (`capnp`, `capnp-rpc`, `capnpc`)

Maintained by **David Renshaw (@dwrensha)** since 2013, the same year Varda announced the project. The Rust impl lives at [`github.com/capnproto/capnproto-rust`](https://github.com/capnproto/capnproto-rust) (2,460 stars, 257 forks). MIT-licensed.

Three crates ship in lockstep:
- `capnp` — runtime library. v0.25.4 published 2026-04-12. 11.5M lifetime downloads.
- `capnp-rpc` — RPC implementation. v0.25.1 published 2026-04-29. 3.8M lifetime downloads.
- `capnpc` — code generator. v0.25.3 published 2026-04-02. 8.5M lifetime downloads.

Release cadence: roughly monthly minors through 2026. Pre-1.0 versioning — every minor can be breaking. MSRV is Rust 1.81.0 on the v0.25.x series.

**Bus factor:** @dwrensha has been the sole maintainer for ~13 years. He also maintains `capnproto-java`. The `capnproto-rust` Cargo.toml lists him as the only author. If he becomes unavailable, the practical impact on Myrhiza would be significant — see [`critiques.md`](critiques.md) and [`open-problems.md`](open-problems.md).

## What's not here (deferred)

- **No HTTP/REST translation in C++ ref impl** — separate `HTTP-over-Cap'n-Proto` (introduced in 0.8, 2020-04-23) is a different protocol.
- **No first-class observability hooks in the wire format** — tracing/metrics ride alongside in app-layer.
- **No three-party handoff in production** — Level 3 is specified but unimplemented. Two parties holding capabilities issued by a third party cannot establish a direct connection without proxying through the issuer. See [`open-problems.md`](open-problems.md).
- **No post-quantum signature story** — Cap'n Proto signing (if you build it on top via `CapHostId` schemes) inherits whatever crypto the application picks.

## Implications for Myrhiza

- The **schema-language + monthly Rust release cadence** is the load-bearing reason capnproto-rust is a credible production choice today. If Myrhiza wants a binary IPC format with capability semantics, the choice is between (a) building on capnp-rpc, (b) re-implementing Cap'n Proto RPC ourselves on top of the wire format, (c) inventing something on Component Model resources + ABI. Option (a) is the most pragmatic but inherits the single-maintainer bus factor.
- The **four-level taxonomy** is the right vocabulary to talk about CapTP semantics — even when borrowing only Level 1, name the levels explicitly so the gap is visible. Myrhiza's spec should distinguish "Level 1 only, like production Cap'n Proto" from "Level 3 ambition, like OCapN aspirational" early.
- The **packed-vs-zero-copy split** is the right shape for Myrhiza's event/state separation: events on the wire are packed (small), state in memory is zero-copy-mmap-friendly (fast). Borrow the dual-encoding pattern rather than the specific bit layout.
- The **schema-evolution rules** are unforgiving but explicit. Myrhiza's state-apply ABI evolution rules should be similarly explicit; "any change not in the safe list is unsafe" is a sound default.

## Sources

- [Cap'n Proto homepage](https://capnproto.org/)
- [Cap'n Proto RPC spec](https://capnproto.org/rpc.html) — *"based heavily on CapTP"* canonical quote
- [Cap'n Proto schema language](https://capnproto.org/language.html)
- [Cap'n Proto wire encoding](https://capnproto.org/encoding.html)
- [Cap'n Proto FAQ](https://capnproto.org/faq.html) — *"The Cloudflare Workers team are now the primary developers and maintainers"*
- [Cap'n Proto news / release history](https://capnproto.org/news/)
- [Cap'n Proto 1.0 release post (2023-07-28)](https://capnproto.org/news/) (LTS announcement)
- [github.com/capnproto/capnproto](https://github.com/capnproto/capnproto)
- [github.com/capnproto/capnproto-rust](https://github.com/capnproto/capnproto-rust)
- [crates.io/crates/capnp](https://crates.io/crates/capnp), [crates.io/crates/capnp-rpc](https://crates.io/crates/capnp-rpc), [crates.io/crates/capnpc](https://crates.io/crates/capnpc)
