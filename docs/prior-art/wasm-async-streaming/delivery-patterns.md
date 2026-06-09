**Date:** 2026-06-08
**Status:** active
**Subject:** Three shapes for pushing a stream into a sandboxed guest — submit-and-poll vs stream-resource vs guest-callback — with determinism notes

# Delivery patterns

Three concrete ways a host can deliver a *stream* of messages into a sandboxed
guest. Each is rated on: works natively (Wasmtime)? works in-browser (jco)?
requires async at the WIT boundary (→ JSPI → Safari gap)? fits Myrhiza's existing
single-use submit-and-poll token model?

## 1. Submit-and-poll (Myrhiza's current ABI, §8.5)

A `*-submit` returns a single-use `request-token`; the kernel re-enters via an
exported `on-*-completion(token, result)`. WIT boundary stays **synchronous**.

```wit
host.broadcast-submit(topic, msg) -> request-token        // returns immediately
on-broadcast-completion(token, result<unit, error>) -> () // kernel re-enters
```

- Native: ✅  Browser: ✅ (no JSPI; kernel/runtime drives re-entry).  Safari: ✅.
- **Single-use token is the problem for streaming.** One token = one completion.
  A subscription is N messages over time; you would either (a) burn one token per
  message — but a submit *initiates*; there is no "submit" for an unsolicited
  inbound message, and the outstanding-token cap (default 256) throttles — or
  (b) re-arm a token, which the kernel *rejects as replay* (single-use is
  enforced for replay protection). **Submit-and-poll models request→response, not
  subscribe→stream.** This is the open problem the corpus exists to surface.

## 2. Stream-resource (Component Model `stream<T>`)

The host creates a `stream<T>`, keeps the writable end, and hands the readable
end to the guest as an unforgeable handle (resource). The guest `stream.read`s in
a loop; the host writes as gossip messages arrive.

- Native: ✅ — Wasmtime `StreamReader::new(store, StreamProducer)` + `pipe`
  (Wasmtime 43 / WASIp3, preview). Clean: one long-lived readable end per
  subscription; backpressure via the producer pull model; unforgeable handle is
  exactly the capability shape Myrhiza wants.
- Browser: ⚠️ — `stream<T>` at the WIT boundary is **async** ⇒ JSPI ⇒ **no
  Safari**, Firefox flagged. Also experimental in jco and exposed to the in-task
  deadlock hazard (wit-bindgen#1609).
- Fits token model: it *replaces* it. The capability becomes the stream handle,
  not a token. This is the architecturally-right native shape and the documented
  migration target (§8.5: "When jco preview3 stabilizes async at the WIT
  boundary, the kernel-side adapter migrates without API churn for app
  authors").
- Verdict: **best native; not yet portable** because of Safari.

## 3. Guest-callback (host invokes guest's `on-message` per delivery)

The guest exports a handler (e.g. `on-subscription-message(sub, topic, msg)`);
the host *calls it* once per delivered message. The WIT boundary stays
**synchronous** (the handler returns quickly; it just enqueues into the guest's
in-sandbox state). This is the event-handler style and maps onto the Component
Model **stackless/callback** export flavor (the runtime re-invokes the guest's
callback until it returns done).

**The key inversion: `host.subscribe` *is* the "submit" that submit-and-poll
lacks.** §1's model breaks on subscriptions because "there is no submit for an
unsolicited inbound message" — but `host.subscribe` supplies exactly that missing
acquire. It is a **submit-once, receive-many** primitive: one synchronous acquire
authorizes an open-ended inbound flow, then the kernel delivers N messages by
re-entering the guest export. Submit-and-poll is submit-once/receive-once
(request→response); subscribe is submit-once/receive-many (subscribe→stream).
That single change — decoupling the *acquire* from the *delivery count* — is the
whole reason this pattern fits where the token model cannot.

```wit
// capability acquired via a sync submit that returns a subscription handle:
host.subscribe(topic) -> subscription          // sync, returns a handle
// kernel pushes each message by calling the guest export:
on-subscription-message(sub: subscription, topic: topic-id, msg: list<u8>) -> ()
```

- Native: ✅ — host calls the export via `TypedFunc::call_async` / repeated
  invocation; epochs bound runaway handlers.
- Browser: ✅ — the jco runtime shim calls the exported handler from JS; **no
  JSPI needed** because the export is sync and the host owns the loop. Safari: ✅.
- Fits token model: extends it naturally. `host.subscribe` is a sync acquire (like
  a submit) returning a *persistent* handle (subscription) instead of a
  single-use token; delivery is the kernel calling an export rather than the
  guest polling a completion. The handle is revocable/attenuable just like a
  capability should be.
- Cost: per-message host→guest call overhead; ordering and buffering are the
  host's responsibility.
- Revocation: unlike §2 there is **no `stream.close` resource** to drop — revoke
  is purely kernel-side. The kernel stops calling the export, removes the
  `subscription` from its handle table, and tears down the host gossip producer;
  later guest use of the stale handle must trap or return `error`, never no-op.
  See [lessons.md](lessons.md) → Borrow.
- Verdict: **the portable shape that works in every browser today** and reuses
  Myrhiza's "kernel re-enters via an exported handler" machinery.

## Comparison

| Pattern | Native | Browser (all incl. Safari) | WIT async? (JSPI) | Streaming fit | Capability shape |
|---|---|---|---|---|---|
| Submit-and-poll | ✅ | ✅ | no | ✗ single-use, request/response only | single-use token |
| `stream<T>` resource | ✅ (preview) | ✗ Safari, ⚠️ FF flag | yes | ✅ native ideal | unforgeable stream handle |
| Guest-callback export | ✅ | ✅ | no | ✅ | persistent subscription handle |

## Determinism note

Subscription delivery is **`interaction`-profile**: non-deterministic and
per-peer. Which topics, which messages, and delivery order are all peer-local and
**must never enter a canonical state-digest** — cross-peer convergence depends on
`state-apply` being a pure function of `(prior state, event)`. The Component
Model's own Nondeterminism section confirms waitable-set event ordering and
host-driven `stream.read`/`write` ordering are nondeterministic, so *any* of the
above patterns is fine for ordering **as long as the ordering is never hashed
into state**. Concretely: `state-apply` must *reject* `host.subscribe` /
subscription handles entirely; only `interaction` (and possibly `behavior`)
profiles may hold them. Convergence stays in the kernel's per-topic state-apply
replay engine; the stream only carries *already-converged* per-topic state/events
into the sandbox for projection.

## Sources

- docs/specs/2026-05-09-myrhiza-master-design/abi.md §8.5 (submit-and-poll, token lifecycle, jco-preview3 migration note)
- https://github.com/WebAssembly/component-model/blob/main/design/mvp/Concurrency.md (streams/futures, stackless callback, nondeterminism)
- https://docs.wasmtime.dev/api/wasmtime/component/struct.StreamReader.html
- https://bytecodealliance.github.io/jco/transpiling.html
- https://github.com/bytecodealliance/wit-bindgen/issues/1609
