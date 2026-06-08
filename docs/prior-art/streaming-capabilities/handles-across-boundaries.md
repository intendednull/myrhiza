**Date:** 2026-06-08
**Status:** active
**Subject:** WASI / Component-Model resource handles + `stream<T>` — representing an in-process capability across the WASM boundary, and the borrow-vs-stream restriction Myrhiza must resolve

# Handles across the WASM boundary

This is the file that maps the abstract ocap ideas onto the *concrete substrate
Myrhiza runs on*: the WebAssembly Component Model. Full project context is in
[`wasm-component-model/`](../wasm-component-model/README.md); here we focus on
resource handles as capabilities and `stream<T>` as mediated delivery.

## Resource handles = unforgeable per-component table indices

The Component Model gives every component a private **handle table**. A `resource`
type is **generative** (each definition is a distinct, non-shareable type). The two
handle forms:

- **`own<T>`** — exclusive ownership of a resource instance; the holder is
  responsible for its destructor (`dtor`).
- **`borrow<T>`** — a temporary loan for the duration of *one call*; must not
  outlive the call.

The decisive property: a handle is "an **unforgeable index into a per-component
table**." The component sees an opaque `i32`; the runtime maps it to the real
object. The app **cannot fabricate a handle to something it was never given** — it
can only name table slots it holds. This is the ocap "reference = authority"
property, enforced by the ABI rather than by convention. **For Myrhiza this is the
answer to "unforgeable handle across the WASM boundary": a subscription is an
`own<subscription>` resource.** The kernel keeps the real subscription state; the
app holds a table index it cannot forge, share without mediation, or point at
another topic.

## `stream<T>` / `future<T>` — mediated async delivery

Component Model async (WASI 0.3-era; `🔀` features in the MVP) adds:

- **`stream<T>`** — "unidirectional unbuffered channel" carrying `0..N` values.
- **`future<T>`** — same, exactly `0..1` values.

Each has a **readable end and a writable end**. When a stream is passed as a
parameter/result, **the readable end is transferred**; the writer keeps the
writable end (created via `stream.new`, which returns the paired ends).

Canonical built-ins (names per the spec's `CanonicalABI.md`):
`stream.new`, `stream.read`, `stream.write`, `stream.cancel-read`,
`stream.cancel-write`, and `stream.drop-readable` / `stream.drop-writable` to drop
each end (the spec uses *drop*, not *close*; and the `future.*` analogues); plus
`waitable-set.wait` to block on several at once and `subtask.cancel` to cancel a
concurrent task.

### Backpressure is built in

Streams are **unbuffered** with completion-based read/write: a `stream.write`
doesn't complete until a reader has a matching `stream.read` outstanding (or
buffer space). So **a slow consumer transparently throttles the producer** — no
explicit window arithmetic, unlike Cap'n Proto's socket-buffer hack
([`capnproto-streaming.md`](capnproto-streaming.md)). Separately,
`backpressure.inc` / `backpressure.dec` adjust a component-instance-wide counter;
while nonzero, new export calls return immediately in a "starting" state without
entering the component's core code — a coarse "I'm overloaded, stop calling me"
signal at the *whole-component* granularity.

### The load-bearing restriction: streams/futures **cannot contain `borrow`**

The spec **currently** rejects `stream<T>` and `future<T>` when `T` transitively
contains a `borrow` handle — verbatim: "validation rejects `(stream T)` and
`(future T)` when `T` transitively contains a `borrow`. This restriction could be
relaxed in the future by extending the call-scoping rules of `borrow` to streams
and futures." Rationale: a `borrow` is valid only for the duration of one call; a
stream outlives any single call and crosses async suspension points, so a borrowed
handle inside it could dangle. `own<T>` is allowed (ownership transfers with the
value). The restriction is real and load-bearing *today*; treat it as a constraint
to design around, not an immutable law.

**This is precisely Myrhiza's open problem.** A subscription delivers a *stream of
per-topic state/events*. Each delivered item is itself capability-relevant (it's
authorized, topic-scoped data). If Myrhiza wanted to deliver *borrowed* handles per
message (cheap, no ownership transfer), the Component Model forbids it inside a
`stream`. The options:

1. **Deliver owned values** (`stream<event-record>` of plain data, or
   `stream<own<event>>`) — the per-message item is a value or an owned handle the
   app must drop. Safe, but every message is an allocation/transfer.
2. **Don't use `stream<T>` for delivery at all** — keep the subscription as an
   `own<subscription>` resource and have the *kernel call an app-exported
   `on-topic-event(handle, event)`* (the existing submit-and-poll re-entry shape,
   generalized from single-use token to reusable handle). Delivery is a call, not a
   stream value; backpressure is the app returning before the kernel sends the next.

Option 2 sidesteps the borrow restriction entirely and reuses Myrhiza's existing
re-entry machinery — likely the right call. `stream<T>` is the more "native"
answer but ties Myrhiza to async-Component-Model maturity and the
owned-value-per-message cost.

## Multi-topic: N handles, awaited together

The brief's defining feature is *one* component subscribing to **multiple** topics
and aggregating them in-sandbox (Discord-style channels). Two shapes:

1. **N `own<subscription>` handles, one per topic** — the app calls `host.subscribe`
   once per topic and holds a handle per channel. Each handle is independently
   attenuable and independently revocable (kill one channel without touching the
   others), which matches the per-topic convergence model exactly: one handle ↔ one
   topic ↔ one kernel replay engine. This is the natural fit.
2. **One handle multiplexing N topics** — a single `own<subscription>` carrying a
   tagged union of per-topic events. Simpler handle bookkeeping, but it couples the
   channels: you cannot revoke or attenuate one topic without re-minting, and the
   handle's scope is no longer "a topic" but "a set," muddying the structural-
   attenuation story above.

**Shape 1 (N handles) is the better default** — it keeps each capability leaf-shaped
and single-topic, so caretaker revocation stays O(1)-per-channel and attenuation
stays structural-per-topic.

The cost of N handles is *awaiting across them*. This is exactly what the Component
Model's **`waitable-set.wait`** is for: the app puts each subscription's readable
end (or its delivery-future) into a **waitable set** (`waitable-set.new`,
`waitable.join`) and blocks on the set; `waitable-set.wait` returns as soon as *any*
one channel has an event, naming which. The spec: "a single waitable set can
uniformly wait on all the kinds of heterogeneous I/O available in the Component
Model." So the sandbox aggregates N topics with one wait loop, no busy-poll, and the
kernel still delivers per-topic. If delivery is the host-driven-callback shape
(option 2 above) rather than `stream<T>`, the analogue is the kernel calling
`on-topic-event(handle, event)` on whichever channel fired — the multiplex lives in
the kernel's dispatch, and the sandbox aggregates across the N handles it holds.

## Native vs browser (Wasmtime vs jco)

Resource handles and the async ABI must work under both Wasmtime (native) and jco
(browser transpile; see [`jco/`](../jco/README.md)). Resource handles are stable
across both. The async `stream<T>` built-ins are newer and less uniformly
supported — another reason option 2 (plain resource handle + host-driven callback)
de-risks the browser target.

## Implications for Myrhiza

- **Subscription = `own<subscription>` resource.** Unforgeable, per-component,
  revocable by the kernel dropping/poisoning the table entry. This is the WIT
  realization of the caretaker from [`ocap-revocation.md`](ocap-revocation.md).
- **The WIT-handle ↔ cap-token mapping** is real and partly solved: the *in-session*
  cap is the resource handle; the *durable* cap (the grant that survives restart and
  can be re-minted) is a token-like artifact (UCAN-delegation-shaped, see
  [`token-attenuation.md`](token-attenuation.md)). The handle is the enlivened form
  of the token — same split as sturdyref↔live-ref.
- **Backpressure: prefer host-driven callback** (kernel calls `on-topic-event`,
  waits for return) over `stream<T>` until the async ABI is proven under jco. Either
  way, per-topic backpressure must be explicit — one global window cannot express
  it.
- **Mind the borrow restriction** if `stream<T>` is ever used: deliver owned values
  or plain records, never borrowed handles.

## Sources

- Component Model Explainer (resource types, own/borrow, generativity; borrow-in-stream restriction "could be relaxed in the future"): <https://github.com/WebAssembly/component-model/blob/main/design/mvp/Explainer.md>
- Component Model Concurrency.md (stream/future semantics, ends, `waitable-set.wait`, backpressure): <https://raw.githubusercontent.com/WebAssembly/component-model/main/design/mvp/Concurrency.md>
- Component Model CanonicalABI.md (`stream.{new,read,write,cancel-read,cancel-write,drop-readable,drop-writable}` built-ins): <https://raw.githubusercontent.com/WebAssembly/component-model/main/design/mvp/CanonicalABI.md>
- WASI 0.3 native async overview: <https://progosling.com/en/dev-digest/2025-08/wasi-0-3-native-async-aug-2025>
- Sibling corpus: [`wasm-component-model/`](../wasm-component-model/README.md), [`jco/`](../jco/README.md)
