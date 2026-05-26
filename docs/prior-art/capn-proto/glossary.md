**Date:** 2026-05-22
**Status:** active
**Subject:** Cap'n Proto / Cap'n Web / Workers RPC system-specific vocabulary

# Glossary

System-specific terms appearing in this folder. Where a term means different things in different files, the file-specific meaning is noted.

## Capability concepts (shared across CapTP family)

**Capability.** A transferable, unforgeable token of authority. In Cap'n Proto, an interface reference passed as a struct field or return value. In Cap'n Web, an RPC stub representing a remote object. Possession of a capability is the only authority needed to invoke its methods.

**CapTP.** Capability Transport Protocol. Originally Mark Miller's wire protocol for the E language (1997). The conceptual ancestor of Cap'n Proto RPC, Cap'n Web, Spritely Goblins's RPC, and Agoric's `@endo/captp`. Per Cap'n Proto's RPC spec: *"Cap'n Proto's RPC protocol is based heavily on CapTP, the distributed capability protocol used by the E programming language."*

**Level 1 / 2 / 3 / 4.** The four levels of CapTP-shaped RPC, ordered by capability sophistication. Level 1 = object refs + promise pipelining; Level 2 = persistent caps; Level 3 = three-party handoff; Level 4 = reference equality. Cap'n Proto's RPC spec defines all four; production deployments are Level 1 + partial Level 2.

**Three-party handoff.** Level 3 CapTP. Peer A introduces peer B to a capability issued by peer C; B and C establish a direct connection without proxying through A. Specified in Cap'n Proto RPC, **never implemented in production** in any CapTP system.

**Promise pipelining.** The ability to chain RPC calls without waiting for intermediate results. `bar(foo())` sends both messages in one round-trip. The headline feature of Cap'n Proto RPC and inherited by Cap'n Web + Workers RPC.

**Sturdyref.** Spritely / E term for a persistent capability — a cap that survives across sessions. Cap'n Proto's equivalent is the `Persistent` interface but the term "sturdyref" is not standard in the Cap'n Proto family.

## Cap'n Proto specific

**.capnp file.** A schema file in Cap'n Proto's schema language. Declares structs, unions, interfaces, generics, annotations with numbered fields and a unique 64-bit file ID.

**`capnp` (command).** The Cap'n Proto compiler. Reads `.capnp` files and runs schema-language plugins (`capnpc-c++`, `capnpc-rust`, `capnpc-go`, etc.) to generate code.

**KJ.** Varda's alternative C++ standard library, used internally by Cap'n Proto's C++ reference implementation. Includes `kj::Promise`, `kj::String`, `kj::Array`, an async event loop, and an exception model. Predates many C++17/20 features.

**Word.** Cap'n Proto's fundamental unit of alignment: 8 bytes (64 bits). All primitive types are word-aligned.

**Segment.** A contiguous block of memory holding a Cap'n Proto message. Messages can be split across multiple segments; pointers carry segment IDs.

**Packed encoding.** A separate Cap'n Proto wire format that strips zero bytes to achieve Protobuf-comparable size. Slower to read than the unpacked form but smaller on the wire.

**Canonical form.** A deterministic encoding of a Cap'n Proto message that's stable across implementations — used for hashing/signing.

**Pointer.** A 64-bit value in a Cap'n Proto message encoding a typed offset to another part of the message. Pointers carry type tags (struct, list, far-pointer, capability).

**Far pointer.** A pointer that crosses a segment boundary. Used when a Cap'n Proto message is split across multiple segments.

**Generic.** Parameterized types (structs or interfaces) in `.capnp`. Constrained: *"Only pointer types (structs, lists, blobs, and interfaces) can be used as generic parameters."*

**Annotation.** Schema metadata. Custom annotations attach values to any schema element; widely used for code-generation hints, naming conventions, validation rules.

**Unique ID.** Every `.capnp` file, struct, and field carries a 64-bit ID. The `capnp id` command generates fresh ones. IDs persist across renames, enabling safe schema evolution.

**Persistent.** A Cap'n Proto interface for capability persistence (sturdyref-like). Partially implemented in the C++ ref impl; applications mostly DIY persistence.

## Cap'n Web specific

**`RpcSession`.** A Cap'n Web peer-to-peer session, owning a transport and tracking export/import tables. Generic over the remote-side's main-interface type for TypeScript hint support.

**RpcTarget.** The base class an object inherits from to be exported as the main interface of an `RpcSession`. Cap'n Web's equivalent of an RPC service.

**Stub.** A Cap'n Web client-side proxy for a remote object. Stubs are first-class JavaScript values — passable as arguments, returnable, garbage-collectable.

**Main interface.** The top-level capability one peer exposes to the other on session start. Either peer may have one; either may be absent (then that side is client-only).

**Disposer.** A `[Symbol.dispose]` method on stubs that releases their server-side reference. Manual analog of garbage collection for remote resources.

**`onRpcBroken(callback)`.** A stub method registering a callback for connection-severance. The Cap'n Web equivalent of error handling for live capabilities.

**Push / Pull / Resolve / Reject / Release / Abort.** The six top-level RPC message types in Cap'n Web's protocol.

**Expression.** A tagged-array form encoding "the value of property P on stub S called with args A" or similar. Cap'n Web's serialization of nested method calls without intermediate await.

**Type tag.** A first-element-of-array convention for serializing non-JSON values. `["date", iso8601]`, `["error", "TypeError", message]`, `["bigint", "12345"]`, `["bytes", base64]`, `["import", importId]`, `["export", exportId]`.

## Workers RPC specific

**Service Binding.** A `wrangler.toml` configuration declaring one Worker can call another via a named binding. *"A way of configuring one Worker with a private channel to talk to another, without going through a public URL."*

**Durable Object.** A stateful Cloudflare Worker — a singleton-by-name across the Cloudflare network. Receives Workers RPC calls from other Workers. *"A 'named' worker instance somewhere on the network that multiple other workers can then talk to."*

**JSValue.** The Cap'n Proto schema type representing arbitrary JavaScript values, including structured-clonables and externals (streams, RPC stubs). Defined in `worker-interface.capnp`.

**JsRpcTarget.** A Cap'n Proto interface declared in `worker-interface.capnp`: the method-invocation surface exposed to JavaScript Workers RPC. Method path + args in, result out.

**workerd.** Cloudflare's open-source Workers runtime. `cloudflare/workerd`, Apache-2.0. Hosts Workers RPC, the JsRpcTarget implementation, and all of Cloudflare's Worker-runtime features.

**Same-thread RPC.** Workers RPC's optimization: when source and destination Workers are in the same workerd process and thread, RPC is a function call rather than serialized wire transit. *"Reducing latency to zero."*

**Structured Clonable.** JavaScript's standardized cloneable-value taxonomy (Date, Map, Set, ArrayBuffer, typed arrays, plain objects with cycles, etc.). Workers RPC's `JsValue` supports all of these.

**Externals.** Non-cloneable references in `JsValue`. Streams, RPC stubs, and other handle-shaped values that must be passed by reference rather than by value.

## Sandstorm specific

**Grain.** A Sandstorm app instance. Each grain runs in a capability-confined Linux container; capabilities are brokered by the Sandstorm shell.

**.spk file.** A Sandstorm app package format. Bundles all the code + assets + manifest needed to install an app onto a Sandstorm server.

**Sandstorm Community.** The post-2024-01-14 community-led project owner under Open Source Collective.

**Tempest.** A from-scratch Sandstorm rewrite in Go, started late-2022 by Ian Denhardt. Stalled after his death mid-2023.

## Cross-family / external references

**Protocol Buffers / Protobuf.** Google's schema-driven serialization format that predates and competes with Cap'n Proto. Varda was its lead maintainer at Google before creating Cap'n Proto.

**gRPC.** Google + CNCF's RPC framework on top of Protocol Buffers + HTTP/2. Cap'n Proto's main competitor for service-to-service RPC. Does *not* implement capability semantics.

**Spritely Goblins.** Christine Lemmer-Webber's distributed-ocap runtime in Guile Scheme. Same CapTP lineage as Cap'n Proto, different design philosophy. See [`../spritely-ocapn/`](../spritely-ocapn/).

**`@endo/captp`.** Agoric's JavaScript CapTP implementation. Same lineage; SES-Hardened-JS host; production at Agoric chain + MetaMask Snaps. See [`../agoric-endo/`](../agoric-endo/).

**OCapN.** Object Capability Network. A draft cross-implementation CapTP specification being co-designed by Spritely, Agoric, MetaMask, and (aspirationally) Cap'n Proto. Pre-specification as of 2026.

**E.** Mark Miller's 1997 capability-secure language. Intellectual root of all CapTP-shaped systems.

**Syrup.** Spritely / OCapN's self-describing wire format. Lisp-native; distinct from both Cap'n Proto's binary and Cap'n Web's JSON.

**SES.** Secure ECMAScript. A subset of JavaScript that enforces ocap discipline at the language level. Agoric's `@endo/*` packages run in SES; Cap'n Web does *not* (it runs in standard JS).

**HardenedJS.** Term used by Agoric for SES-secured JavaScript.

## Sources

- [Cap'n Proto schema language](https://capnproto.org/language.html)
- [Cap'n Proto RPC spec](https://capnproto.org/rpc.html)
- [Cap'n Proto wire format](https://capnproto.org/encoding.html)
- [Cap'n Web `protocol.md`](https://github.com/cloudflare/capnweb/blob/main/protocol.md)
- [Cap'n Web `README.md`](https://github.com/cloudflare/capnweb/blob/main/README.md)
- [Workers RPC announcement](https://blog.cloudflare.com/javascript-native-rpc/)
- [`worker-interface.capnp`](https://github.com/cloudflare/workerd/blob/main/src/workerd/io/worker-interface.capnp)
- [`../spritely-ocapn/glossary.md`](../spritely-ocapn/glossary.md) — cross-referenced for shared CapTP terminology
