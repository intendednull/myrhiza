**Date:** 2026-05-09
**Status:** active
**Subject:** Croquet / Multisynq — how lockstep determinism actually works

# Determinism

This is the load-bearing file. Croquet is the canonical reference for the **lockstep deterministic VM** paradigm: every replica receives the same input message stream in the same order and computes byte-identical state. This is *different from* SwingSet's event-log replay (single authority + retroactive reconciliation; see [`agoric-endo/determinism.md`](../agoric-endo/determinism.md)) and *different from* CRDT merge (no message ordering, eventual convergence by algebra; see [`crdts/`](../crdts/)). Myrhiza's `state-apply` profile inherits the strict-determinism property; both architectures are candidates for how that property is realised at runtime.

Sibling docs: [`architecture.md`](architecture.md), [`programming-model.md`](programming-model.md), [`comparisons.md`](comparisons.md), [`open-problems.md`](open-problems.md), [`critiques.md`](critiques.md), [`lessons.md`](lessons.md).

## The lockstep paradigm

Croquet's contract:

1. The reflector assigns every message a `(time, seq)` stamp and broadcasts in canonical order.
2. Every client receives `RECV [time, seq, payload]` envelopes and a steady stream of `TICK time` heartbeats.
3. Every client feeds its `VirtualMachine` the same envelopes in the same order.
4. The VM is a pure function of `(snapshot, ordered message stream)`. Therefore every client lands on byte-identical state.

The hash of the model state is periodically voted across clients (see [Snapshot-equality](#what-snapshots-prove) below). Hash agreement *is* the convergence proof. Hash disagreement *is* the bug.

Compare to alternatives:

- **CRDT merge.** No ordering required; correctness comes from the algebra (commutative/associative/idempotent ops). Strictly weaker convergence — operations are reordered freely, which precludes any application semantics that depend on sequence (auctions, ordered lists with global indices, vector clocks for causality). Myrhiza's `state-apply` cannot be a CRDT-only design.
- **Event-log replay** (SwingSet). Single-author log; consensus is on the log content + per-delivery transcript, not on real-time ordering. Replay reconstructs state from log + initial state. Tolerates much higher latency between authority and replication. Croquet replays nothing on the steady-state path — it advances live.

## Simulated pseudo-time

The cornerstone. Inside model code, "now" is **not** `Date.now()`, **not** `performance.now()`, **not** any wall-clock or monotonic-clock reading. It is a **virtual clock** advanced exclusively by reflector-stamped messages.

From [`tutorials/sim-time-future`](https://docs.multisynq.io/tutorials/sim-time-future):

> "Simulation time only advances when heartbeat ticks are received from the reflector."

> "Source: Multisynq reflector heartbeat ticks. Identical for all users."

`this.now()` returns simulation time in milliseconds since session start (`time` field of the most recent `RECV`/`TICK` envelope processed). Multiple calls within a single message handler return the **same** value — pseudo-time advances *between* messages, never *within* one. This is the property that makes a model handler a pure function: nothing about its execution depends on how long it takes to run on the local machine.

`this.future(ms).method(args)` is the deterministic-scheduling primitive. It enqueues an internal message at simulation time `now + ms`. The VM has a per-session priority queue (`priorityQueue.js`, `fastpriorityqueue` package); when the next external message would advance time past a queued future-call, the VM processes the future-call first. Queue contents are part of the snapshot, so future-calls survive snapshot/replay/rejoin.

Implementation reference: `client/teatime/src/vm.js` and `priorityQueue.js`.

## Message ordering

The reflector produces a strict global total order over all `SEND` events plus emitted `TICK` events. Properties:

- Every client sees the same `(time, seq)` sequence.
- `seq` is a uint32 (`controller.js`: `msg[1] >>>= 0` defensively coerces, since the reflector "used to send int32").
- Out-of-order delivery from the WebSocket transport is impossible at the application layer because the reflector is the serialising authority — the order on the wire from reflector to a given client is the canonical order.
- A client lagging on the network sees the same sequence as a client ahead, just later. Caught-up clients pause via the `synced(true|false)` view event when the gap exceeds `SYNCED_MAX = 2000 ms` or `SYNCED_MAX_FACTOR = 0.2 × msPerTick`.

This is what eliminates the merge problem entirely. If `seq` 100 is "user A bid 5" and `seq` 101 is "user B bid 5", every client agrees A wins. There is no concurrent state to reconcile.

## The determinism contract — what model code may do

From the [`writing-multisynq-model`](https://docs.multisynq.io/tutorials/writing-multisynq-model) tutorial, [`vm.js`](https://github.com/multisynq/multisynq-client/blob/main/client/teatime/src/vm.js), and the README's "Prime Directive":

**Allowed:**

- Read replicated state (own fields, fields of other models reachable through the model graph).
- Read pseudo-time via `this.now()`.
- Generate randomness via `this.random()` or `Math.random()` (patched — see below).
- Use deterministic transcendentals via `Math.sin`, `cos`, `pow`, etc. (patched — see [Floating-point determinism](#floating-point-determinism-the-actual-engineering)).
- Schedule deterministic future work via `this.future(ms).method(args)`.
- Publish events via `this.publish(scope, event, data)` and subscribe via `this.subscribe(scope, event, handler)`.
- Read frozen `Multisynq.Constants` (validated as such — non-frozen access is a violation).

**Forbidden** (enforced by the patched globals in `vm.js#patchBrowser`, by the SES-style discipline in the docs, and by snapshot-serialisation failures):

- `Date.now()` or `new Date()` from clock — patched in model context to return VM time and emit a `MultisynqWarning` once.
- `Math.random()` from outside the model — throws `"synchronized random accessed from outside the model"`.
- `setTimeout`, `setInterval`, `requestAnimationFrame`, `queueMicrotask`, `Promise.resolve()` — async work cannot survive snapshot/replay because pending microtasks are not serialised.
- Functions stored as state — JS functions are not introspectable for serialisation. The snapshot serialiser would refuse them. (Use method-name strings + lookup instead.)
- DOM, `window`, `document`, `localStorage`, `fetch`, `XMLHttpRequest`, `WebSocket`, file I/O — anything reading external state.
- Mutable global variables outside `Multisynq.Constants` — would not be in the snapshot, so a new joiner would see different state.
- External library state with non-snapshottable content.

The enforcement mechanism is a JavaScript-language hybrid: some violations throw immediately (`this.random` outside model code), some warn (`Date.now()` in model code, replaced with VM time), and some are caught only at snapshot time (functions in state) or only by divergence (mutable globals). It is *not* a sandbox in the WASM sense; a determined application author can write non-deterministic model code and the runtime cannot stop them. They will discover this when their session diverges.

## Seeded RNG

`vm.js` line 2 imports `seedrandom` (the David Bau ARC4-based PRNG) from a vendored, patched copy in `thirdparty-patched/seedrandom/`. At VM construction (line ~481):

```js
// seed with session id so different sessions get different random streams
this._random = new SeedRandom(snapshot.id, { state: true });
```

`{ state: true }` makes the generator's internal state **serialisable**, so it survives snapshots. A session's random stream is fully determined by the session id (a hash of session name, options, and code). Different sessions produce different streams; the same session produces the same stream on every replica.

Two model-side APIs:

- `this.random()` → `this._random()` from `seedrandom`. Returns a float in `[0, 1)`.
- `this.randomID()` → 4 × `this._random.int32()` concatenated as hex. Used for generating model IDs deterministically.

Top-level `Math.random()` is patched in `vm.js#patchBrowser`:

```js
globalThis.MultisynqMath.random = () => CurrentVM.random();
// ...
Math.random = (...) => CurrentVM ? modelRandom(...) : viewRandom(...);
```

So in model context, `Math.random()` dispatches to the seeded session RNG; in view context it dispatches to the original `Math.random`. View randomness is intentionally non-deterministic — it's there for visual variety (particle effects, idle animation), and the docs explicitly warn: "Never use view random for game logic!"

There is one notable footgun: a model-context `Math.evaluate(fn)` call (used for `Multisynq.Constants` initialisation) installs a `FakeVM` whose `random()` throws — so even the seeded RNG is unavailable during constant-init, on the principle that constants must be deterministic without depending on session identity.

## Floating-point determinism — the actual engineering

JavaScript engines do **not** guarantee bit-identical results for `Math.sin`, `cos`, `tan`, `exp`, `log`, `pow`, etc. across browsers/CPUs. (The IEEE 754 spec leaves transcendentals implementation-defined; v8 vs SpiderMonkey vs JavaScriptCore use different libm implementations, and AMD vs Intel chips historically differ on the boundary inputs.) For a lockstep VM, this is a correctness hazard.

Croquet's posture is *not* "avoid float in models." It is **"replace transcendentals with a deterministic library."**

`client/math/math.js` imports the `@stdlib/math/base/special/*` implementations (from the [stdlib project](https://stdlib.io/), a runtime-version-pinned set of pure-JS math primitives) and installs them as `globalThis.MultisynqMath.{sin,cos,tan,sinh,cosh,tanh,asin,acos,atan,atan2,asinh,acosh,atanh,exp,expm1,log,log1p,log2,log10,cbrt,pow}`. `vm.js#patchBrowser` then *swaps* the global `Math.{...}` methods to dispatch:

```js
Math[funcName] = arg => CurrentVM ? modelFunc(arg) : viewFunc(arg);
```

Inside model code (`CurrentVM` is set), `Math.sin` is `@stdlib/math/base/special/sin`. Outside model code, it is the original engine `Math.sin`. The application author writes `Math.sin(x)` and gets the right behaviour automatically.

`pow` is special-cased because *even `@stdlib`'s pow* hit a real-world divergence ([`math.js` comment](https://github.com/multisynq/multisynq-client/blob/main/client/math/math.js)):

> "workaround for iOS Safari bug giving inconsistent results for stdlib's pow()"

The custom `MultisynqMath.pow` short-circuits trivial cases (`y === 1, 2, 3, 4`, integer exponents, NaN/Inf) and falls back to `exp(log(x) * y)` using the now-deterministic `exp`/`log`. The comment explicitly acknowledges:

> "even on integer cases, the base Math.pow can be inconsistent across browsers (e.g., 5,-4 giving 0.0016 or 0.0015999999999999999)"

This is the kind of detail Myrhiza will hit. It is not avoidable by spec or by sandboxing — it has to be engineered against, per primitive, with a regression test for every browser/architecture pair.

Basic `+ - * /` and IEEE 754 comparison are deterministic across compliant JS engines and are not replaced.

## What snapshots prove

The cross-client snapshot vote is Croquet's empirical determinism check. From `controller.js`:

1. Reflector triggers a snapshot poll (every `SNAPSHOT_AFTER_CPU = 5000 ms` of model CPU, or every `DEPIN_SNAPSHOT_AFTER_TEATIME = 5 min` of pseudo-time on DePIN).
2. Each client serialises its model state with `fast-json-stable-stringify` (canonical key order — eliminates trivial JSON-shape differences) and computes a hash.
3. Clients vote `{ hash, viewId }` via the **TUTTI** vote protocol.
4. Reflector tallies and rebroadcasts.
5. Clients group votes by hash. **If `numberOfGroups > 1`, the session has diverged.**

Divergence handling (`controller.js` lines ~778–793, 931–948):

```js
console.error(this.id, `Session diverged (#${previous})! Snapshots fall into ${numberOfGroups} groups`);
```

Every client logs the divergence. The "dissident" group (minority by vote count) gets `dissidentFlag` set; one client per group uploads its snapshot URL via the `__VM__#__diverged__` event. `diffDivergedSnapshots()` then downloads two divergent snapshots and JSON-diffs them, dumping the diff to console + `debugger;` for human investigation.

Critically: **Croquet does not automatically recover from divergence.** It detects it, logs it, surfaces it for debugging, and lets the session continue (potentially with multiple state branches). There is no mechanism to expel the dissident, force a re-sync, or vote on the canonical state. Operator intervention is the implied recovery path. (Compare SwingSet, where divergence is a **chain halt** that requires a coordinated upgrade to clear.)

## What Croquet *does not* solve

1. **Byzantine peers.** A malicious or compromised client can inject corrupted state into its own snapshot vote. The system detects the divergence via TUTTI but has no consensus mechanism to identify which group is "correct." There is no BFT signing, no slashing, no operator override. The reflector sees votes and can in principle weight them, but the open-source SDK has no such logic.
2. **Long-term snapshot durability.** Snapshots are stored in cloud blob storage (the multisynq.io platform) keyed by session id. There is no first-party guarantee about retention, no Merkle-DAG of historical snapshots, no genesis-replay path. If the cloud loses your snapshot, your session restarts from `init()`.
3. **Cross-version migration.** Code-hash is part of session id. Changing your model code creates a *new* session — the old snapshot is unreachable. The [persistence](https://docs.multisynq.io/tutorials/persistence) feature is a developer-managed escape hatch: you serialise a chosen subset of state to a stable `persistentId = hash(appId, name)` keyed blob, and your new model's `init()` deserialises it. There is no automatic migration story; the developer writes `read()`/`write()` for every long-lived class.
4. **Determinism of the JavaScript engine itself.** Croquet patches user-visible non-determinism (`Math`, `Date`, RNG). It does not address engine-level non-determinism: V8/JSC/SpiderMonkey JIT tier-up effects, GC timing, Map/Set iteration order on resize, Number internal representation. In practice these are deterministic enough for browser-grade JS at JSON-snapshot granularity, because the JSON serialisation hides internal representation. They would not be deterministic enough for byte-level heap-snapshot consensus (which is why SwingSet uses XS, not V8 — see [`agoric-endo/determinism.md`](../agoric-endo/determinism.md)).
5. **Determinism enforcement.** Model code that calls `setTimeout` works locally but breaks the model. Snapshot divergence is the only signal, and it lags by up to ~5 s of CPU or 5 min of pseudo-time. A static-analysis layer or runtime sandbox would catch these earlier; Croquet has neither.

## Comparison to Myrhiza

Myrhiza's `state-apply` is **WASM Component Model**, not patched JS. The mapping:

| Croquet/Multisynq mechanism | Myrhiza analogue |
|---|---|
| Reflector orders messages | Sequencer / event-ordering authority for `state-apply` events. Could be reflector-shaped, BFT-quorum-shaped, or DHT-shaped. |
| Pseudo-time advanced by `TICK` | Event-stamp time. `state-apply` reads `(prior state, event)` where event includes a deterministic time stamp. Heartbeat events for animation/decay. |
| Patched `Math.sin`/`pow`/etc. | Wasmtime gives stronger float determinism by default (canonical-NaN mode, no transcendental builtins — they're lib-side). With a deterministic libm in component code, transcendentals are bit-identical without engine cooperation. |
| Patched `Math.random` → seeded | `state-apply` import takes `(seed, state) → (random, new_state)` from the kernel; seed derived from event hash. Or no random at all — events carry pre-derived randomness. |
| Patched `Date.now` → VM time | No host clock import in `state-apply`; events carry timestamps. |
| Forbid functions in state | Component Model linear memory has no functions-as-values; this is structurally impossible, not enforced by audit. |
| Snapshot via `fast-json-stable-stringify` + hash | Myrhiza's planned `state-digest()` component export — same idea, application-defined canonical hashing. Avoids byte-level memory hashing's gcc-9-style fragility (see [Agoric `#7829`](https://github.com/Agoric/agoric-sdk/issues/7829)). |
| TUTTI snapshot vote | Cross-peer state-digest comparison at snapshot cadence. **Worth borrowing.** |
| `diffDivergedSnapshots` for debugging | A diff-tool over two divergent state-digests would be valuable — design tip: serialise state in a structured form a JSON-diff can navigate. |
| No BFT, no automatic divergence recovery | Myrhiza either accepts the same posture (peer-local correctness; expulsion is operator policy) or layers BFT/quorum on top. This is a spec decision. |

The single biggest lesson from Croquet for Myrhiza: **the engineering effort to make a JS-runtime deterministic is enormous and never finished.** Every transcendental is a divergence risk; every browser update is a regression risk; every external library is an attack surface. Myrhiza's choice of WASM gives a much better starting point (canonical NaN, no engine-level RNG, no `Date`) but inherits its own list — see [`open-problems.md`](open-problems.md).

## Sources

- [`@multisynq/client` 1.1.0 source](https://github.com/multisynq/multisynq-client) (Apache-2.0, 241 ★, current as of 2026-04-13)
  - [`client/teatime/src/vm.js`](https://github.com/multisynq/multisynq-client/blob/main/client/teatime/src/vm.js) — VM construction, seeded RNG init, `patchBrowser`, evaluation guards.
  - [`client/teatime/src/controller.js`](https://github.com/multisynq/multisynq-client/blob/main/client/teatime/src/controller.js) — TUTTI voting, divergence detection, snapshot policy, RECV/TICK/SYNC handlers.
  - [`client/math/math.js`](https://github.com/multisynq/multisynq-client/blob/main/client/math/math.js) — deterministic transcendentals, iOS pow workaround.
  - [`client/teatime/thirdparty-patched/seedrandom/`](https://github.com/multisynq/multisynq-client/tree/main/client/teatime/thirdparty-patched/seedrandom) — vendored seedrandom.
- [Multisynq docs: Sim Time & Future](https://docs.multisynq.io/tutorials/sim-time-future)
- [Multisynq docs: Random](https://docs.multisynq.io/tutorials/random)
- [Multisynq docs: Snapshots](https://docs.multisynq.io/tutorials/snapshots)
- [Multisynq docs: Persistence](https://docs.multisynq.io/tutorials/persistence) — developer-managed cross-version state.
- [Multisynq docs: Writing a Multisynq Model](https://docs.multisynq.io/tutorials/writing-multisynq-model) — Prime Directive, allowed/forbidden APIs.
- [Multisynq docs: Conflict Resolution](https://docs.multisynq.io/essentials/conflicts) — confirms "deterministic execution prevents conflicts" framing.
- [stdlib (`@stdlib/stdlib`)](https://stdlib.io/) — pure-JS deterministic math library, dependency of `@multisynq/client`.
- [seedrandom by David Bau](https://github.com/davidbau/seedrandom) — ARC4-based JS PRNG, vendored.
- Smith, Kay, Raab, Reed — *Croquet: A Collaboration System Architecture*, C5 2003 (TeaTime origin).
