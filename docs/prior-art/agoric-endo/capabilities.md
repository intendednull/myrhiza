**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/Endo — Object capabilities: `E()`, `Far()`, marshal, CapTP

# Object Capabilities in Endo

Endo's ocap stack is the operational form of "no ambient authority": every
operation a guest can perform requires a reference to an object that has the
right method, and references can only be obtained by being given them. There
are no globals through which authority leaks. The protocol that carries this
discipline across address spaces is **CapTP**.

This file covers the reference-style discipline (`Far`, `pass-style`,
`marshal`), the eventual-send proxy (`E()`), promise pipelining, the CapTP wire
protocol, and distributed GC. For the underlying SES sandbox see
[./hardened-js.md](./hardened-js.md). For a richer treatment of the ocap
*network* protocol family, see [../spritely-ocapn/](../spritely-ocapn/) — Endo
implements the JS reference of the OCapN wire protocols, but the design
genealogy lives there.

## Three reference styles

`@endo/pass-style` (1.8.0, 2026-04-16) defines what may legally be marshalled
across a vat boundary. Every JS value is classified into one of:

| Pass-style | Examples | Semantics across boundary |
|---|---|---|
| **pass-by-copy** (data) | numbers, strings, `bigint`, plain hardened objects, arrays, `CopyTagged`, `Error` | Recipient gets an independent copy; values are structural-equal |
| **pass-by-presence** (remotable) | `Far(...)` objects, Exos | Recipient gets a *Presence* — a local proxy referring to the original; method calls round-trip |
| **promise** | `Promise`, `HandledPromise` | Recipient gets a remote promise that settles when the origin's settles; callable while pending |

Anything not matching one of those three is rejected. Functions are **not**
passable on their own — wrap them in `Far('iface', { method })` to declare
intent. Hardened plain objects are passable; hardened class instances
generally are not (see the `class` caveat in [./hardened-js.md](./hardened-js.md)).

The classification function is `passStyleOf(v)`. The serializer (`@endo/marshal`)
uses it to decide encode-as-data vs. encode-as-slot.

## `Far()`

```js
import { Far } from '@endo/far';
const counter = Far('Counter', {
  increment() { /* ... */ },
  read()      { /* ... */ },
});
```

`Far(iface, behaviorRecord)` produces a hardened remotable object whose
"interface label" (the first arg) is a brand string carried by `getInterfaceOf()`.
The label is advisory metadata for debugging and brand-matching, not a security
boundary; cross-vat verification of interfaces is the job of `Exo`'s
`InterfaceGuard` (see [./hardened-js.md](./hardened-js.md)).

`Far()` does *not* validate inputs. For new code, prefer `Exo`/`makeExo` which
attaches an `InterfaceGuard` that validates argument and return shapes on every
method call.

## `E()` — eventual send

`E()` is the user-facing API for "send a message to a possibly-remote,
possibly-unresolved object." From [`@endo/eventual-send`](https://github.com/endojs/endo/blob/master/packages/eventual-send) (1.5.0):

```js
const result = E(presenceOrPromise).method(arg1, arg2);
//   ^ always a Promise, regardless of whether target is local, remote, or pending
```

API surface:

- `E(t).method(args)` — eventual method call.
- `E.get(t).prop` — eventual property get.
- `E.sendOnly(t).method(args)` — fire-and-forget; no result promise.
- `E.when(p, onResolved, onRejected)` — cleaner than `.then` because it
  does not flatten remote promises that resolve to other remote promises in
  surprising ways.

Mechanically, `E(x)` returns a `Proxy` over `HandledPromise.resolve(x)`. The
proxy's `get` traps return functions which, when called, dispatch
`HandledPromise.applyMethod` (or `applyFunction` / `get`). The handler attached
to `x` (set when a Presence is created) routes the message — locally inline,
or out over CapTP.

`HandledPromise` is a thin extension of `Promise` adding handler-based dispatch
for messages sent to *unresolved* promises. It implements the (currently
inactive) [TC39 eventual-send proposal](https://github.com/tc39/proposal-eventual-send).
Most code never touches `HandledPromise` directly.

### Promise pipelining

Pipelining is the optimization that makes ocap-over-network practical. Given:

```js
const a = E(server).getA();
const b = E(a).getB();
const c = E(b).compute(42);
```

Without pipelining, the client makes three round-trips: wait for `a`, then
send `getB`, wait for `b`, then send `compute`. With pipelining, all three
messages ship in the same outbound packet, addressed to the *promises* for `a`
and `b` respectively. The server resolves them locally and applies the
follow-up messages without ever round-tripping. CapTP's `CTP_CALL` carries a
"target" that can be either a known import slot or a yet-unresolved question
ID, which is what makes this work.

Pipelining is most of why CapTP-over-WAN is not catastrophic. Without it,
chained capability invocations would be RTT-bound and unusable.

## CapTP wire protocol

`@endo/captp` (4.5.0, 2026-02-26) is the JS reference implementation. The
[on-wire message types](https://github.com/endojs/endo/blob/master/packages/captp/src/captp.js)
are:

| Op | Direction | Purpose |
|---|---|---|
| `CTP_BOOTSTRAP` | A → B | "Send me your bootstrap object" — the well-known root capability |
| `CTP_CALL` | A → B | Invoke a method or get a property on an exported slot or pending answer |
| `CTP_RETURN` | B → A | Resolve / reject a question (answer to a CALL) |
| `CTP_RESOLVE` | A → B | Resolution of an imported promise A previously exported |
| `CTP_DROP` | A → B | A no longer holds a reference to an export of B; refcount-- |
| `CTP_DISCONNECT` | either | Fatal protocol error; tear down the session |
| `CTP_TRAP_ITERATE` | A → B | Continuation marker for `Trap` (synchronous-blocking) flows |

`Trap` / `TrapCap` is the pseudosynchronous escape hatch: it lets a guest issue
a *blocking* call to a host within the same address space (host runs the
worker, blocks on Atomics.wait, returns). The CapTP README explicitly scopes
this as "guest/host" — i.e. cooperating, not mutually-suspicious — and it has
no analog over a network.

Slot identifiers are opaque strings with sign and direction — Agoric uses
`o+0`, `o-0`, `p+0`, `p-0` etc. where `o`/`p` is object/promise and `+`/`-`
is exported/imported. The signs are flipped on the receiving side. Object
identity is per-(connection, slot); the same JS object can have different
slots in different connections.

The CapTP messages do not themselves define a serialization format for the
*payload*: that is `@endo/marshal`'s job.

## `@endo/marshal`

Marshal (1.9.1, 2026-04-16) turns a passable JS value into a `CapData` record:

```js
{ body: '<JSON-ish string>', slots: ['o-3', 'p+1', ...] }
```

The body uses one of two encodings:

- **Original**: objects with an `@qclass` key carry tagged values
  (`{ '@qclass': 'NaN' }`, `{ '@qclass': 'slot', index: 0 }`).
- **Smallcaps**: strings with reserved leading characters; e.g. `#NaN` for
  NaN, `$0` to reference slot 0, `&0` for a promise slot. More compact and
  more robust against accidental key collisions.

Smallcaps is the format `@endo/marshal` recommends for new wire formats. Both
encodings are still in active use because old persisted state was written with
the original encoding and rewriting is an upgrade-coordination problem.

The `slots[]` array is the bridge to CapTP. Anywhere the body says "slot N",
the consumer asks the local CapTP machine for a Presence/Promise corresponding
to `slots[N]`. Marshal itself does not know about the network; it knows only
"give a slot string for this remotable, give a remotable for this slot string",
via two host-supplied callbacks (`convertValToSlot`, `convertSlotToVal`).

Determinism note: `makePassableKit` exposes two ordering modes —
`legacyOrdered` and `compactOrdered`. Within a mode, ordering is stable; across
modes it is not. Cross-vat schemes that rely on byte-equal serialization must
pin the mode explicitly.

## Sturdyrefs vs live refs

Two reference lifetimes coexist:

- **Live refs** are CapTP slots scoped to a single connection. They die when
  the connection dies. They are the default for everything `E()` produces.
- **Sturdyrefs** are bearer tokens — strings that can be persisted, mailed
  across the network out-of-band, and later "swissNumbered" back into a live
  ref by presenting them. They exist for capability persistence across
  reconnects and for handoff between parties that have never directly talked.
  Endo's implementation lives in the OCapN-track packages
  ([`@endo/ocapn`](https://github.com/endojs/endo/blob/master/packages/ocapn))
  and is currently labeled experimental; the sturdyref handoff protocol is
  inherited from the OCapN spec, not invented by Agoric.

For day-to-day in-process and intra-trusted-network ocap, only live refs are
used.

## Distributed GC

Cross-vat references are reference-counted at two levels: per-vat (liveslots)
and per-kernel (the SwingSet kernel itself). The
[SwingSet GC docs](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/garbage-collection.md)
describe it precisely; the shape is:

1. Each vat tracks objects it has imported (other vats' exports it holds) and
   its own exports (objects other vats hold).
2. The vat distinguishes **reachable** ("can produce this object") from
   **recognizable** ("can detect this object as a `WeakMap` key without
   producing it"). These need separate refcounts because a vat can hold a
   `WeakMap` keyed on an import without itself reaching it.
3. When a vat's local GC drops the last in-vat reference to an imported
   Presence, liveslots emits one of:
   - `syscall.dropImports([vref])` — "I no longer reach this; you may stop
     keeping it alive on my behalf".
   - `syscall.retireImports([vref])` — "I cannot even recognize it; you may
     remove it from my c-list".
   - `syscall.retireExports([vref])` — exporter side: "this export is
     permanently destroyed".
4. The kernel maintains per-object reachable and recognizable counts across
   all vats plus its own queues. When either drops to zero it queues
   `dispatch.dropExports` / `dispatch.retireImports` deliveries to the relevant
   vats.
5. Vats use `WeakRef` and `FinalizationRegistry` to detect drops, but only
   inside liveslots; vat user code is *forbidden* `WeakRef` and
   `FinalizationRegistry` (they would reveal nondeterministic GC timing). The
   exposed `WeakMap`/`WeakSet` are reimplemented to key on SwingSet
   vrefs rather than JS object identity.

For the comms vat (inter-machine), the protocol layers a sequence-number /
ack-number race-resolution scheme on top, because drop messages can cross
re-introduction messages mid-flight. That extra complexity is *the* reason
distributed GC across Agoric machines is a known load-bearing problem rather
than a solved one.

The cleanest summary: **Endo distributed GC works, but the implementation has
"informed vs. ignorant" message phasing in the comms layer, transcripts, and
durable kernel actions to handle GC correctly under churn.** Anyone designing
a similar system should expect to spend real engineering on this; the naive
"refcount messages over a stream" design has live races.

## Comparison to Spritely Goblins (brief)

Goblins (Spritely's Racket/Guile/JS implementations) and Endo target the same
abstractions: ocap, eventual-send, promise pipelining, CapTP-style wire. The
genealogy is shared — both descend from E and Joe-E and the same Mark Miller
PhD thesis. Practical differences as of 2026:

- **Wire protocol unification.** OCapN is the standardization umbrella;
  `@endo/ocapn` (1.0.0, experimental, 2026-04-16) is Agoric's track to
  interoperate. Goblins uses OCapN natively. They are converging, not yet
  converged.
- **Serialization.** OCapN/Goblins use Syrup (binary, canonical). Agoric
  legacy uses JSON-based marshal (smallcaps or original). The OCapN package
  ships a Syrup codec for compatibility.
- **Sandbox.** Endo runs guests in SES Compartments. Goblins relies on the
  host language's module system (Racket's submodules, Scheme modules) — there
  is no equivalent of `lockdown()` because Scheme already has lexical scope as
  the primary discipline.

Cross-reference: [../spritely-ocapn/captp-and-ocapn.md](../spritely-ocapn/captp-and-ocapn.md)
covers the wire protocol from the OCapN side; this file covers Agoric's
implementation choices.

## Implications for Myrhiza

1. **Three reference styles → three handle disciplines.** The Endo three-way
   split (data / presence / promise) is exactly the discipline the WASM
   Component Model already encodes (value types / resource handles / future
   handles). When we wire components to peers we should preserve the
   distinction at the kernel boundary: data is copied, resource handles map
   to *presences* in the ocap sense, futures map to remote promises. Don't
   collapse them.
2. **`Far()` and Exo are the analog of WIT interface labels.** A WIT
   interface name is the closest WASM-side equivalent of `Far('Counter', ...)`'s
   iface label. Treat it as an advisory brand, not a security boundary; the
   actual contract enforcement is the import/export type-check at link time
   (Exo's `InterfaceGuard` is doing the same job dynamically).
3. **`E()` and promise pipelining are the right shape for cross-peer calls.**
   Component Model `future<T>` plus the planned async ABI gives us the local
   half of this. The cross-peer half — pipelining over CapTP — has no WASM
   equivalent yet; we will need to build it. Designing without pipelining
   from day one will lock us into RTT-bound cross-peer calls.
4. **Pass-style discipline must apply at the kernel boundary, not just the
   network boundary.** The lesson from marshal+pass-style is that classifying
   what may cross a trust boundary is a runtime check — the kernel needs to
   enforce "this argument is data, this is a handle, this is a promise" on
   every host call. Components that try to pass non-passable shapes get
   rejected by the kernel, not silently misinterpreted.
5. **Distributed GC is hard. Plan for it.** The naive "refcount drops over
   the wire" design has races under churn (re-introduction crossing drop).
   The SwingSet comms-layer phasing (sequence/ack tracking, durable kernel
   actions, WeakRef in liveslots only) is approximately the minimum viable
   design. We should not pretend our P2P GC will be simpler than this.
6. **Sturdyrefs are how capabilities survive process restart.** Live refs die
   with the connection; sturdyrefs let a peer hand a capability to a third
   party out-of-band. Our event log gives us at-rest persistence; we still
   need a sturdyref-equivalent for capabilities that outlive a session. The
   OCapN sturdyref/handoff protocol is the design-of-record to copy from.
7. **No `WeakRef` / `FinalizationRegistry` in `state-apply`.** This is
   directly the Agoric rule: nondeterministic finalization timing breaks
   cross-peer convergence. Our determinism requirement gives the same
   conclusion. Wasmtime does not expose these primitives; we just need to
   not add them.
8. **`Trap` is a useful escape hatch but only host-to-host.** SwingSet allows
   blocking sync calls only when the parties are in the same address space.
   Our equivalent (kernel-mediated synchronous calls) should follow the same
   rule: never across a peer boundary, only host-to-component within a
   single peer.

See also: [./hardened-js.md](./hardened-js.md) for the SES substrate;
[./modules-and-bundling.md](./modules-and-bundling.md) for how passables move
through the module/bundle layer; [./vat-model.md](./vat-model.md) for how
vats use these primitives; [./distribution.md](./distribution.md) for the
Agoric-network deployment story.

## Sources

- `@endo/pass-style` README: https://github.com/endojs/endo/blob/master/packages/pass-style/README.md
- `@endo/eventual-send` README: https://github.com/endojs/endo/blob/master/packages/eventual-send/README.md
- `@endo/captp` README: https://github.com/endojs/endo/blob/master/packages/captp/README.md
- `@endo/captp` source (wire opcodes): https://github.com/endojs/endo/blob/master/packages/captp/src/captp.js
- `@endo/marshal` README: https://github.com/endojs/endo/blob/master/packages/marshal/README.md
- `@endo/far` README: https://github.com/endojs/endo/blob/master/packages/far/README.md
- `@endo/exo`: https://github.com/endojs/endo/tree/master/packages/exo
- `@endo/ocapn` README: https://github.com/endojs/endo/blob/master/packages/ocapn/README.md
- SwingSet GC docs: https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/garbage-collection.md
- TC39 eventual-send proposal: https://github.com/tc39/proposal-eventual-send
- Endo release history: `gh api repos/endojs/endo/releases` (verified 2026-05-09)
