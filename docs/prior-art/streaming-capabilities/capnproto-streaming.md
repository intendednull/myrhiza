**Date:** 2026-06-08
**Status:** active
**Subject:** Cap'n Proto capability streaming — the `-> stream` return type, window-based flow control, promise pipelining

# Cap'n Proto streaming

Cap'n Proto is the production CapTP-lineage RPC system (Kenton Varda; see the
sibling folder [`capn-proto/`](../capn-proto/README.md) for the full project
context). This file is narrowly about how it models a *long-lived, flow-controlled
stream of calls on a capability* — directly the shape Myrhiza's `host.subscribe`
needs.

## The `-> stream` return type (since 0.8, 2020-04-23)

A method is declared streaming by giving it the special return type `stream`:

```capnp
sendChunk @0 (chunk :Data) -> stream;
```

Per the [0.8 release notes](https://capnproto.org/news/2020-04-23-capnproto-0.8.html):
methods declared `-> stream` "behave like methods with empty return types
(`-> ()`), but with special behavior when the call is sent over a network
connection." `-> stream` is *wire-compatible* with `-> ()` — it is a **hint**, not
a new wire type. Implementations without streaming support treat it identically to
`-> ()`. Flow control is **only implemented in the C++ library** to date.

Critically: the stream is a sequence of *separate calls on one capability*, not a
single call returning many values. The capability is the long-lived grant; each
`sendChunk` is one delivery. This is the delegation-vs-invocation split (see
[`token-attenuation.md`](token-attenuation.md)) expressed in an RPC system.

## Window-based flow control

The client library "will act as if the call has 'returned' as soon as it thinks
the app should send the next call." This lets the app write a naive `while` loop;
each call appears to complete instantly until the connection saturates, then
blocks. The window is set by a deliberately crude heuristic, quoted from the
release notes:

> Cap'n Proto currently implements flow control using a simple hack: it queries
> the send buffer size of the underlying network socket, and sets that as the
> "window size" for each stream.

Because the OS grows the socket send buffer to track the TCP congestion window,
the streaming window scales with the actual link. There is also a coarser global
throttle: `RpcSystem::setFlowLimit(words)` caps how many words of in-flight call
messages the whole RPC system will tolerate before throttling — a backstop against
a peer that floods you with calls.

## Error handling: `done()`

Because streaming calls "return" eagerly (before the server has processed them),
errors arrive late. The rule: "If a streaming call ends up throwing an exception,
then all later method invocations on the same object (streaming or not) will also
throw the same exception." The app must call a final `done()` method to flush and
observe any error from prior streaming calls. **Lesson for Myrhiza:** eager-ack
flow control means errors are asynchronous to the call that caused them — the
error channel must be separate from the per-call return.

## Promise pipelining underneath

Streaming reuses Cap'n Proto's promise pipelining: you can call methods on the
*result of a call that hasn't returned yet*, and those calls are sent immediately,
addressed to the eventual result. This collapses round-trips. For streaming it
means a client can begin issuing the next call before the previous one's promise
resolves. Pipelining is the feature that makes "call the subscription handle, then
immediately call methods on what it returns" cheap — see
[`ocap-revocation.md`](ocap-revocation.md) for the same idea in CapTP/Goblins.

## Implications for Myrhiza

- A subscription = a long-lived **capability**; each delivered message = a **call
  on it**. Don't model the subscription as one call returning a stream value;
  model it as a handle the kernel repeatedly calls (`on-topic-event`). This fits
  the existing submit-and-poll re-entry pattern far better than a single-use token
  does — the handle is reusable by construction.
- The socket-buffer-as-window hack is a *warning*, not a model to copy: it works
  because Cap'n Proto rides one TCP socket per connection. Myrhiza multiplexes many
  topics over iroh-gossip; a single OS-buffer window cannot express per-topic
  backpressure. Myrhiza needs an **explicit per-subscription window** (see the
  Component Model's per-stream model in
  [`handles-across-boundaries.md`](handles-across-boundaries.md)).
- Eager-ack + late error means the cancel/revoke path and the error path must be
  first-class, not piggybacked on a call return.

## Sources

- Cap'n Proto 0.8 release notes (streaming flow control), 2020-04-23: <https://capnproto.org/news/2020-04-23-capnproto-0.8.html>
- Cap'n Proto RPC / pipelining: <https://capnproto.org/rpc.html>
- DeepWiki summary of the RPC system (setFlowLimit): <https://deepwiki.com/capnproto/capnproto/3.3-rpc-system>
- Sibling corpus: [`capn-proto/`](../capn-proto/README.md)
