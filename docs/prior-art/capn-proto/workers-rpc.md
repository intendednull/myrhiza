**Date:** 2026-05-22
**Status:** active
**Subject:** Workers RPC — Cloudflare's intra-Workers JavaScript-native RPC system, *"under the hood, built on Cap'n Proto"* (announced 2024-04-05)

# Workers RPC

The deployed CapTP at hyperscale. Announced by Kenton Varda on 2024-04-05 in [*"Use one RPC system for browser-to-server, server-to-server, and worker-to-worker calls"*](https://blog.cloudflare.com/javascript-native-rpc/). Open-source as part of [`cloudflare/workerd`](https://github.com/cloudflare/workerd). The implementation file referenced in the announcement is [`src/workerd/api/worker-rpc.c++`](https://github.com/cloudflare/workerd/blob/main/src/workerd/api/worker-rpc.c++); the schema is [`src/workerd/io/worker-interface.capnp`](https://github.com/cloudflare/workerd/blob/main/src/workerd/io/worker-interface.capnp).

This is *not* a separate library. It is a JavaScript-language binding to Cap'n Proto RPC, surfaced inside workerd as the way you write code that calls *other* Workers and Durable Objects.

## Key facts

| Fact | Value |
|---|---|
| Announced | 2024-04-05 (Kenton Varda, blog.cloudflare.com) |
| Implementation | `cloudflare/workerd` C++ code (Apache-2.0) |
| Schema (Cap'n Proto) | `src/workerd/io/worker-interface.capnp` — defines `EventDispatcher`, `JsRpcTarget`, `WorkerdBootstrap`, `TailStreamTarget`, `Trace`, `JsValue`, etc. |
| Wire format | Cap'n Proto binary (the workerd-internal RPC), exposed to JS via a Proxy-based API |
| Canonical statement | *"Workers RPC is a JavaScript-native RPC system. Under the hood, it is built on Cap'n Proto."* — Cloudflare blog, 2024-04-05 |
| Scope | Worker-to-Worker via Service Bindings; Worker-to-Durable-Object; same-account only (*"only allow communication between Workers running on the same account"*) |
| Schemas required for app authors | No |
| Levels (CapTP) | Level 1 (object refs + promise pipelining) implemented. Levels 2-4 not exposed at the JS layer. |

## What "built on Cap'n Proto" actually means

The canonical statement is unambiguous in the blog post:

> *"Workers RPC is a JavaScript-native RPC system. Under the hood, it is built on [Cap'n Proto](https://capnproto.org/rpc.html)."*

> *"The [protocol](https://github.com/cloudflare/workerd/blob/03629a6553751d3614a8b91926e380213e100d94/src/workerd/io/worker-interface.capnp#L302) and [implementation](https://github.com/cloudflare/workerd/blob/03629a6553751d3614a8b91926e380213e100d94/src/workerd/api/worker-rpc.c++) are fully open source as part of [workerd](https://github.com/cloudflare/workerd/)."*

Concretely:
- The wire format crossing the Worker boundary is Cap'n Proto binary.
- The schema (`worker-interface.capnp`) defines the message types — `JsRpcTarget` with methods like `call(methodPath, args) -> result`, `JsValue` for serialized JavaScript values, plus `EventDispatcher`, `Trace`, queue/scheduled/alarm event types.
- The JavaScript API surface — `env.MY_BINDING.someMethod(args)` — is a Proxy that translates property accesses + method calls into Cap'n Proto RPC messages and back.
- The schema explicitly declares `using JsValue = ...` allowing arbitrary structured-clonable values, plus *externals* for streams, RPC stubs, and other non-copyable references.

## What's transferable on the wire

From the blog (verbatim quotes):

> *"You can pass Structured Clonable types as the params or return value of an RPC. (That means that, unlike JSON, Dates just work, and you can even have cycles.)"*

> *"You can additionally pass functions in the params or return value of other functions. When the other side calls the function you passed to it, they make a new RPC back to you. Similarly, you can pass objects with methods. Method calls become further RPCs."*

So: structured-clonable values (Date, Map, Set, ArrayBuffer, typed arrays, cycles), plus first-class function references and class-instance references. The latter two are the capability-passing primitives — *"all class instances are replaced with RPC stubs"* on the receiving side.

The capability discipline holds:

> *"The RPC client cannot create a User object out of thin air, and cannot call methods of an object without first explicitly receiving a reference to it."*

This is the production ocap discipline shipping at hyperscale: a Worker can only invoke methods on objects whose stubs have been explicitly passed to it.

## The JavaScript Proxy trick

The API is implemented with `Proxy`. Quote from the blog:

> *"The RPC stub is a special object called a 'Proxy'. It implements a 'wildcard method', that is, it appears to have an infinite number of methods of every possible name."*

So writing `env.MY_WORKER.someMethod(arg1, arg2)` looks like a method call but is intercepted: the Proxy converts the property access (`someMethod`) and the call (`(arg1, arg2)`) into a single Cap'n Proto RPC `call(["someMethod"], [arg1, arg2])` message, then returns a promise.

Property *chaining* is also intercepted, so `env.MY_WORKER.foo.bar.baz()` becomes a single RPC with method path `["foo", "bar", "baz"]` — this is how schemaless dotted-path access works.

## Service Bindings + Durable Objects: the deployment shapes

The two production surfaces:

**Service Bindings.** *"A Service Binding, which is a way of configuring one Worker with a private channel to talk to another, without going through a public URL."* The killer optimization: *"RPC to another Worker (over a Service Binding) usually does not even cross a network. In fact, the other Worker usually runs in the very same thread as the caller, reducing latency to zero."* On the same thread, "RPC" is a function call with a Cap'n Proto-shaped boundary; cross-machine, it's a Cap'n Proto wire transit.

**Durable Objects.** *"Durable Objects allow you to create a 'named' worker instance somewhere on the network that multiple other workers can then talk to, in order to coordinate between them."* Durable Objects are the "stateful Workers" — singleton-by-name across the Cloudflare network. RPCs to them are always Cap'n Proto wire (the DO is on a specific machine; the caller is wherever). *"RPCs must cross the network."*

So Workers RPC is the *same API* with vastly different cost profiles depending on routing — and the system picks the optimal path automatically.

## Promise pipelining (carried forward)

> *"Although it isn't explicitly a security feature, it is commonly provided by object-capability RPC systems like Cap'n Proto."*

Workers RPC inherits Cap'n Proto's promise pipelining. Writing `env.A.lookup(id).getName()` makes two RPC calls, but the second can be pipelined onto the first without a round-trip. For Durable Objects, where the round-trip is real network latency, this is a meaningful speedup.

## CapTP level

The Workers RPC blog does *not* claim any specific CapTP level. By implementation it's **Level 1 + structured-clonable JSValue**: object references and promise pipelining are present; Level 2 (persistent capabilities across sessions) is not exposed because each Durable Object invocation creates a new RPC session against the DO's isolate; Level 3 (three-party handoff) is not implemented (this is also missing from upstream Cap'n Proto C++); Level 4 (reference equality) is irrelevant inside a single-account scope where stubs always come from named bindings.

## Same-account boundary

The hard production limit: *"For now, Service Bindings and Durable Objects only allow communication between Workers running on the same account."* Cross-account Worker-to-Worker RPC is not supported. Cap'n Web exists in part to fill this gap (Worker-to-browser, Worker-to-arbitrary-JS).

## The Cap'n Proto schema (excerpt)

From [`worker-interface.capnp`](https://github.com/cloudflare/workerd/blob/main/src/workerd/io/worker-interface.capnp):

```capnp
# Copyright (c) 2017-2022 Cloudflare, Inc.
# Licensed under the Apache 2.0 license

@0xb665d6c0fe7eb6e0;
using import "/capnp/persistent.capnp".Persistent;
# ... imports for HTTP, byte streams, trace types

interface EventDispatcher {
  # Dispatches HTTP requests, scheduled tasks, alarms, queue messages, etc.
  # ...
}

interface JsRpcTarget {
  # The method-invocation surface exposed to JavaScript Workers RPC.
  call @0 (methodPath :List(Text), args :JsValue) -> (result :JsValue);
  # ... (additional methods)
}

struct JsValue {
  # Serialized JavaScript value with externals for streams + RPC stubs.
  # ...
}
```

Major interfaces defined in this file: `EventDispatcher`, `JsRpcTarget`, `WorkerdBootstrap`, `TailStreamTarget`. Major data types: `Trace`, `JsValue`, `HibernatableWebSocketEventMessage`, `QueueMessage`, `QueueResponse`, `Onset`/`Outcome`.

## What workerd users see

Inside a Worker:
```typescript
export default {
  async fetch(req, env) {
    // env.SERVICE is a Service Binding configured in wrangler.toml
    let user = await env.SERVICE.getUser(userId);   // Cap'n Proto under the hood
    let name = await user.getName();                // pipelinable
    return new Response(name);
  }
};
```

Inside a Durable Object class:
```typescript
export class Counter {
  async increment() { /* ... */ return this.state.count; }
  // ^ callable from another Worker as: env.COUNTER.get(id).increment()
}
```

The schema is in `worker-interface.capnp`. The app author never writes Cap'n Proto.

## Why this matters strategically

Two production milestones the broader CapTP lineage spent 25+ years not achieving:

1. **Capability-secure RPC at hyperscale.** Cloudflare Workers processes a substantial fraction of global web traffic. Cap'n Proto's Level 1 RPC is the wire on which intra-Worker calls cross the boundary at that scale. The E language never reached this; Spritely Goblins has not reached this; Agoric's smart-contract usage of `@endo/captp` is at a smaller scale.
2. **JS-native ocap RPC ergonomics, no schema overhead, ships.** Workers RPC removes the schema-writing requirement for app authors. The trade-off: tight coupling to one runtime (workerd) and one language (JS/TS). Whether this is a strategic win or a strategic narrowing depends on how Myrhiza answers the same trade-off.

## Implications for Myrhiza

- **The "schema is internal, app surface is schemaless" pattern is the right one for friction-reduction.** Workers RPC keeps a Cap'n Proto schema (`worker-interface.capnp`) as the on-the-wire contract, then surfaces a Proxy-based schemaless API to apps. Myrhiza's app authors can analogously work against typed handles (WIT resources) without ever writing `.capnp`-style schemas, even if the kernel uses Cap'n Proto internally.
- **The same-thread "RPC is a function call" optimization is the load-bearing performance trick.** Workers RPC's killer feature is that worker-to-worker on the same machine costs nothing; only cross-machine RPCs pay wire-format costs. Myrhiza's kernel-app boundary should be similar: same-process WASM-to-WASM via Component Model resources is free; only cross-process / cross-peer RPCs pay encoding cost.
- **Single-account scope is a real production constraint.** The hardest CapTP problem Cloudflare hasn't shipped is cross-account RPC (= cross-trust-boundary). Myrhiza's identical structural problem is cross-peer RPC at scale; expect it to be hard and ship the easier scope first.
- **`Proxy`-based wildcard method dispatch is a JS-only trick.** Rust doesn't have it; WIT doesn't have it. Don't try to copy the *ergonomics* of dotted-path method dispatch into a typed language; copy the *idea* (an interface is a callable surface) but use WIT's typed resource interface instead.
- **Open-sourcing the protocol-on-wire while keeping the runtime proprietary is a smart governance choice.** Workers RPC's wire format is in workerd (Apache-2.0); Cloudflare's optimizations to the runtime are not all there. Myrhiza should similarly open-source the wire spec + reference impl while feeling no obligation to open-source every helper crate.

## Sources

- [*"Use one RPC system for browser-to-server, server-to-server, and worker-to-worker calls"*](https://blog.cloudflare.com/javascript-native-rpc/) (Cloudflare blog, Kenton Varda, 2024-04-05) — canonical Workers RPC announcement
- [`worker-interface.capnp`](https://github.com/cloudflare/workerd/blob/main/src/workerd/io/worker-interface.capnp) — the schema
- [`worker-rpc.c++`](https://github.com/cloudflare/workerd/blob/main/src/workerd/api/worker-rpc.c++) — the C++ implementation
- [Cloudflare Workers docs — RPC](https://developers.cloudflare.com/workers/runtime-apis/rpc/) — user-facing
- [github.com/cloudflare/workerd](https://github.com/cloudflare/workerd) — workerd runtime (Apache-2.0)
