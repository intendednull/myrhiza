**Date:** 2026-05-22
**Status:** active
**Subject:** Cap'n Web — JavaScript/TypeScript-native object-capability RPC. Announced 2025-09-22 by Cloudflare. MIT-licensed, currently v0.8.0.

# Cap'n Web

Cap'n Web is Cloudflare's 2025 JavaScript/TypeScript-native RPC library. *"A spiritual sibling to Cap'n Proto (and is created by the same author), but designed to play nice in the web stack."* Kenton Varda + Steve Faulkner announced it on 2025-09-22 in [the Cloudflare blog](https://blog.cloudflare.com/capnweb-javascript-rpc-library/). The repo had been live since 2025-06-08 internal-only; first public npm publish was `capnweb@0.0.1` on 2025-09-12 (10 days pre-announce); v0.1.0 launched on 2025-09-21.

It is **NOT** Cap'n Proto for JavaScript. It is a **separate protocol** with overlapping ocap semantics but a fundamentally different wire shape and design philosophy. The two systems interoperate at the Workers boundary (see below), but their on-the-wire formats are not compatible.

## Key facts

| Fact | Value |
|---|---|
| Repo | [`github.com/cloudflare/capnweb`](https://github.com/cloudflare/capnweb) |
| Created | 2025-06-08 (private), 2025-09-22 (public announcement) |
| License | MIT |
| Authors | Kenton Varda (kentonv) — Cloudflare Principal Engineer + Cap'n Proto creator; Steve Faulkner |
| Languages | TypeScript (canonical), JavaScript-compatible |
| npm package | `capnweb`. v0.8.0 (2026-05-11), v0.7.0 (2026-04-27), v0.1.0 launch tag (2025-09-21) |
| Stars | 3,813 |
| Bundle size | *"compresses (minify+gzip) to under 10 kB with no dependencies"* (per blog) |
| Schemas | **None.** This is the headline departure from Cap'n Proto. |
| Wire format | JSON, with array-tagged extensions for non-JSON types |
| Transports | HTTP (batch), WebSocket, `postMessage()`, `MessagePort` out-of-box; pluggable via `RpcTransport` interface |
| Runtimes | All modern browsers, Cloudflare Workers, Node.js, Bun |
| Status | *"new and still highly experimental"* (per 2025-09-22 blog) |
| Production users | Cloudflare Wrangler "remote bindings" feature (mentioned in announcement). No others publicly disclosed. |

## What's different vs Cap'n Proto

Per the announcement post and the README, three explicit divergences:

1. **No schemas.** *"Unlike Cap'n Proto, Cap'n Web has no schemas. In fact, it has almost no boilerplate whatsoever."* The cost of this is no compile-time type safety on the wire — though TypeScript types on the API surface still help in-process. The protocol.md notes: *"No runtime type checking; malicious clients can send unexpected types."*
2. **JSON-based wire format.** *"The protocol uses JSON as its basic serialization, with a preprocessing step to support non-JSON types."* Non-JSON values (Date, Error, Blob, RPC stubs) are encoded as arrays where the first element is a type tag. HTTP transport uses newline-delimited JSON.
3. **Two tables instead of four.** *"In CapTP and Cap'n Proto, there are four tables instead of two: imports, exports, questions, and answers. In this library, we have unified questions with imports, and answers with exports."* A protocol-level simplification: a question is just a placeholder for an import you don't have yet.

## Object-capability semantics retained

Despite the simplification, the ocap shape is preserved:

- **Stubs are first-class.** When you pass a JavaScript object or function over the wire, the receiver gets an RPC stub — a Proxy whose method calls become further RPC calls back to the sender.
- **Bidirectional.** *"Neither side is defined as the 'client' nor the 'server'. Each side can optionally expose a 'main interface' to the other."* Either party can be the introducer.
- **Promise pipelining preserved.** *"Supports promise pipelining. When you start an RPC, you get back a promise."* Same shape as Cap'n Proto, on a different wire.
- **Capability-passing.** A stub passed in an argument or return value is itself a capability — the receiver gains the authority to invoke it. Reference counting is explicit: the `release` message carries a refcount of how many times the stub was introduced.

## The Workers RPC interop point

Critical for understanding placement: Cap'n Web is *deliberately* designed to interop with [Workers RPC](workers-rpc.md), even though they have different wire formats. From the README:

> *"Cap'n Web is designed to be compatible with Workers RPC, meaning you can pass Cap'n Web RPC stubs over Workers RPC and vice versa. The system will automatically wrap one stub type in the other and arrange to proxy calls."*

So the production deployment pattern Cloudflare is targeting is: **Workers RPC** for Worker-to-Worker (or Worker-to-Durable-Object) intra-Cloudflare, and **Cap'n Web** for browser-to-Worker (or any non-Workers JS runtime). The two systems share a model; the wire format choice is transport-driven.

## Wire format sketch

From `protocol.md`:

- **Top-level messages.** `push`, `pull`, `resolve`, `reject`, `release`, `abort`. Tagged JSON arrays: `["push", importId, expression]`, etc.
- **Imports and exports.** Numbered with signed integers. The sign disambiguates which side originated the ID: positive on the introducer side, negative on the receiver's view of the same ID.
- **Expressions.** A small DSL for "the result of calling property X on stub Y with arguments Z" — encoded as `["import", importId, propertyPath, callArgs]` or `["export", exportId]` or constant values.
- **Type tags.** `["date", iso8601]`, `["error", "TypeError", message, ...]`, `["bigint", "12345"]`, `["bytes", base64]`, `["undefined"]`, `["import", ...]`, `["export", ...]`.

Cycles, `Map`, `Set`, `RegExp` are *not* supported in serialized values — the README is explicit. This is a deliberate restriction to keep the wire format small and to avoid recursive-cycle handling.

## Hello world

```javascript
// Browser-side
import { newWebSocketRpcSession } from "capnweb";
let api = newWebSocketRpcSession("wss://example.com/api");
let result = await api.hello("World");
console.log(result);
```

```typescript
// Worker-side
import { RpcSession, RpcTarget } from "capnweb";
class Api extends RpcTarget {
  hello(name: string) { return `Hello, ${name}!`; }
}
// transport setup wires this to a WebSocket
let session = new RpcSession<undefined>(transport, new Api());
```

The TypeScript generic on `RpcSession<RemoteMainInterface>` carries the *expected* shape of the other side's main interface — a compile-time hint, not a runtime check. This is the entire schema-replacement strategy.

## Lifecycle and disposal

Garbage collection of remote resources is the README's most explicitly-flagged limitation: *"garbage collection does not work well when remote resources are involved."* The two recommended patterns:

- **Explicit dispose.** Stubs expose a `[Symbol.dispose]` or equivalent; the caller calls it when done.
- **Short-lived sessions.** Build sessions for a single request-response interaction; tear down at the end.

The protocol-level mechanism is a `release` message carrying a refcount, so the sender can drop their export when the receiver has zero outstanding references. Robustness in the face of network failure: *"Stubs expose `onRpcBroken((error) => { ... })` to react when the connection breaks."*

## Implementation status (2025-09 onward)

Per the announcement post, Cap'n Web is *"new and still highly experimental. There may be bugs to shake out."* The version trajectory backs this:

| Version | Date | Notes |
|---|---|---|
| 0.0.1 | 2025-09-12 | First npm publish (10 days before public blog) |
| 0.1.0 | 2025-09-21 | Launch-week tag |
| 0.2.0 | 2025-11-05 | First minor after launch |
| 0.3.0 | 2025-12-16 | |
| 0.4.0 | 2025-12-24 | |
| 0.5.0 | 2026-02-18 | |
| 0.6.0 | 2026-03-09 | |
| 0.6.1 | 2026-03-09 | Same-day patch |
| 0.7.0 | 2026-04-27 | |
| 0.8.0 | 2026-05-11 | Current |

Roughly monthly minors. Pre-1.0; every minor can break. The npm package's `beta` dist-tag points to a prerelease (`0.0.0-5eb7701` style commit-sha pin) — internal flow uses dist-tags, not separate branches.

## What's not here

- **No three-party handoff** (Level 3 CapTP). Cap'n Web's two-table simplification likely makes this harder, not easier, to retrofit.
- **No persistent capabilities across sessions** (Level 2). Capabilities are tied to a `RpcSession` lifetime; once the WebSocket closes, the export table is gone.
- **No browser-side ocap discipline enforcement.** The browser's JavaScript engine doesn't enforce capability discipline; "the client can call the server" is true at the protocol level, but at the language level any code in the same origin can call any RPC stub it can reach.
- **No formal security review.** The "new and experimental" framing is the disclaimer.

## Implications for Myrhiza

- **The two-table simplification is a real ergonomic win** that the four-table CapTP design has paid for since E. If Myrhiza ships an internal CapTP-shaped RPC, weigh adopting the two-table model.
- **JSON-tagged-arrays is a portable enough wire format** to be re-implemented in any language. Cap'n Web has done the design work of "what's the smallest JSON-with-extensions you need for ocap RPC" — borrow the type-tag taxonomy if Myrhiza needs a debug/text RPC alongside the binary one.
- **The interop-with-Workers-RPC pattern is the right answer to wire-format heterogeneity.** When two ocap systems must coexist with different wire formats, automatic stub-proxying at the bridge point is the correct shape. Myrhiza's WIT-resource ↔ ocap-stub boundary will face the same problem.
- **The "no schema" departure is a Cloudflare-scale ergonomic gain that Myrhiza probably can't repeat.** Cap'n Web works because TypeScript is the only target language, and TS types ride alongside the API surface. Myrhiza is cross-language by construction (Rust + WIT + apps in arbitrary languages); we need schemas. Take Cap'n Web's *protocol* lessons; don't take the "no schema" choice.
- **Cap'n Web is too new to bet on.** v0.8.0, experimental status, single-major-product anchor (Wrangler remote bindings). If Myrhiza wants a CapTP-shaped wire today, capnp-rpc is the safer choice; revisit Cap'n Web in 12-18 months when it has either reached 1.0 or proven adoption beyond Cloudflare.

## Sources

- [Cap'n Web announcement post (blog.cloudflare.com, 2025-09-22)](https://blog.cloudflare.com/capnweb-javascript-rpc-library/) — Varda + Faulkner
- [github.com/cloudflare/capnweb](https://github.com/cloudflare/capnweb)
- [github.com/cloudflare/capnweb/blob/main/protocol.md](https://github.com/cloudflare/capnweb/blob/main/protocol.md) — wire-format spec
- [github.com/cloudflare/capnweb/blob/main/README.md](https://github.com/cloudflare/capnweb/blob/main/README.md) — quickstart + interop notes
- [npmjs.com/package/capnweb](https://www.npmjs.com/package/capnweb)
- [npm registry capnweb time field](https://registry.npmjs.org/capnweb) — verified version dates
