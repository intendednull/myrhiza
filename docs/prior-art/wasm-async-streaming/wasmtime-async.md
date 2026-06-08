**Date:** 2026-06-08
**Status:** active
**Subject:** Native delivery in Wasmtime — `Store::call_async`, epochs, async host functions, and the P3 host stream API

# Wasmtime native async

This is the *easy* side for Myrhiza: natively, the kernel is the Wasmtime
embedder and has full control. Two layers matter — the long-standing
**host-driven async** (Store/Func `call_async`, epochs) and the **new P3
component-async host API** that lets the host own the writable end of a
`stream<T>` and push values into a guest.

## Host-driven async (mature, pre-P3)

Independent of component-model streams, Wasmtime has long supported running
guests from a Rust async context:

- **`Store::call_async` / `Func::call_async` / `TypedFunc::call_async`** — call
  guest exports as Rust futures. Requires `Config::async_support(true)`. The sync
  vs async variant chosen is dictated by whether the Store is async.
- **Async host functions** — host imports can be `async` Rust fns; the guest
  "blocks" on them without blocking the host OS thread (the future returns
  `Pending`).
- **Epoch interruption** (`Config::epoch_interruption(true)`) — cooperative
  timeslicing: the embedder bumps an epoch counter periodically; instrumented
  guest code yields at the next check. On deadline the future returns `Pending`
  and re-arms. Cheaper than fuel-style instruction counting. (Fuel is the
  alternative, with per-operator cost configuration added in v43.)

This layer is enough to deliver messages by **repeatedly calling a guest export**
(the guest-callback pattern in [delivery-patterns.md](delivery-patterns.md)) —
and it works on the *existing* WASI 0.2 / preview-2 sync WIT boundary, which is
what Myrhiza targets today via submit-and-poll.

## P3 component-async host stream API (new, preview)

Wasmtime 43.0.0 (2026-03-20) ships WASIp3 snapshot `0.3.0-rc-2026-03-15` and the
host-side API to create a `stream<T>` and hand its readable end to a guest.
Async component-model support has been maturing across releases (initial WASI 0.3
previews in v37; v41 allowed intra-component stream/future reads/writes for simple
types and removed the `POLL` callback code from the canonical ABI so
`waitable-set.poll` no longer yields; v43 tightened blocking checks). Requires the
`component-model-async` feature.

Host-side types (docs.wasmtime.dev, `wasmtime::component`):

- **`StreamReader<T>`** — "the readable end of a Component Model `stream`". The
  host constructs one with `StreamReader::new(store, producer)` where `producer`
  implements **`StreamProducer<T>`**, then passes the readable end to the guest
  (the guest receives it as a stream resource).
- **`StreamProducer`** — host-owned source; its `poll_produce` is pulled to
  generate items. **`FutureProducer`** is the single-value analogue (host-owned
  write end of a `future`).
- **`StreamConsumer`** + `StreamReader::pipe` — connect a reader to a consumer
  that accepts delivered items.
- Lifecycle hazard: "StreamReader instances must be disposed of using `close`;
  otherwise the in-store representation will leak and the writer end will hang
  indefinitely." `guard()` → `GuardedStreamReader` auto-closes on drop.

So natively, **the kernel can model `host.subscribe` as: create a `stream<T>`
whose `StreamProducer` is fed by the iroh-gossip per-topic feed, and hand the
readable end to the interaction component.** This is the clean target shape — one
long-lived readable stream per subscription, no token recycling. (For multiple
topics: one stream per topic, all joinable into the guest's waitable set; or a
single `stream<envelope>` tagged with topic-id.)

## Caveats for Myrhiza (native)

- **Preview maturity.** Per wasmCloud, long-lived streams under load and
  backpressure are the unhardened edges — exactly Myrhiza's use case (a
  subscription is a long-lived stream). Treat the P3 host stream API as
  *promising but not production-proven* in 2026.
- **Backpressure ownership.** The `StreamProducer` pull model gives the host
  natural backpressure (don't produce until pulled), which aligns with Myrhiza's
  requirement that a slow consumer not stall the actor mailbox — but the kernel
  must still bound buffering of undelivered gossip messages.
- **Determinism is a non-issue here by design.** Subscription delivery is
  `interaction`-profile and per-peer; the kernel must keep it out of any
  state-apply replay (see [lessons.md](lessons.md)). Wasmtime can run a
  Deterministic Profile, but that is for `state-apply`, not for subscription
  feeds.

## Sources

- https://docs.wasmtime.dev/examples-async.html
- https://docs.wasmtime.dev/api/wasmtime/struct.Store.html
- https://docs.wasmtime.dev/api/wasmtime/struct.Config.html
- https://docs.wasmtime.dev/api/wasmtime/component/struct.StreamReader.html
- https://docs.wasmtime.dev/api/wasmtime/component/trait.FutureProducer.html
- https://github.com/bytecodealliance/wasmtime/releases (v41.0.0, v43.0.0)
- https://github.com/bytecodealliance/wasmtime/blob/main/RELEASES.md (canonical release notes; v43.0.0 WASIp3)
- https://wasmcloud.com/blog/wasi-p3-on-wasmcloud/
