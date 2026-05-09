**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/SwingSet — vat lifecycle, dispatch/syscall API, transcript replay

# Vat Model

A SwingSet **vat** is a unit of synchronous code with its own JS realm, isolated from every other vat by a kernel-managed message boundary. Vat code never sees system services directly — it sees a `dispatch` function (kernel pushes work in) and a `syscall` object (vat pushes work out). All state changes a vat makes are deterministic functions of its delivery sequence, which means a vat can be replayed bit-for-bit from its transcript. Transcript replay *is* SwingSet's persistence story.

For broader kernel context, see [`./architecture.md`](./architecture.md). For cross-machine messaging, see [`./captp-and-network.md`](./captp-and-network.md).

## Two flavours of vat: static, dynamic

The two kinds (per [`static-vats.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/static-vats.md), [`dynamic-vats.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/dynamic-vats.md)):

| | Static | Dynamic |
|---|---|---|
| Defined where | `config.vats` at boot | `E(vatAdminService).createVat(bundlecap, options)` runtime call |
| Source bundle | Bundled at first `initializeSwingset()`, immutable for life of the swing-store | Installed via `controller.validateAndInstallBundle(bundle)`; bundlecap then passed in |
| Default worker | `local` (in-process), unless `defaultManagerType` is set | `xs-worker` (xsnap subprocess) by default |
| Metered? | No, not by default | Yes, by default — runs out of meter → vat terminated |
| Visible at bootstrap? | Yes — root presences passed to `bootstrap(vats, devices)` | No — caller must hold `vatAdminService` capability |

Built-in static vats currently include `vatAdmin` (auto-installed by the kernel). The `comms` and `timer` vats are written by SwingSet but the host application has to add them to `config.vats` itself; the docs note these "might be turned into built-in vats in the future" ([`static-vats.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/static-vats.md)). Each Zoe contract instance is a dynamic vat.

## Building a vat: `buildRootObject`

A vat module exports a function:

```js
// vat-counter.js
import { Far } from '@endo/far';

export function buildRootObject(vatPowers, vatParameters, baggage) {
  let counter = 0;
  return Far('root', {
    increment() { counter += 1; },
    read()      { return counter; },
  });
}
```

The kernel calls this once when the vat is first created. The returned object becomes the vat's "root presence" — the only object reachable by the rest of the system at startup. From there, capability discipline takes over: connectivity begets connectivity. The bootstrap vat alone is given references to every other vat's root and every device, and is responsible for wiring the system. ([`static-vats.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/static-vats.md))

`vatPowers` is the back door for "things that don't fit into syscalls." Currently a small set: `exitVat(completion)`, `exitVatWithFailure(reason)`, and (when enabled) `disavow(presence)`. The set is deliberately narrow.

`vatParameters` is JSON-serializable data, passed at vat creation time. It's the "command-line argv" of a vat.

`baggage` (used by upgradeable vats) is a durable map that survives across vat upgrades — durable refs in this map remain valid; everything else is discarded.

### Legacy `setup()` path

A vat module can instead default-export `setup(syscall, state, helpers)` and return a `dispatch` directly. This bypasses liveslots, which is required for the comms vat (it does its own raw marshalling) and is *not* something application code should reach for. Application vats use `buildRootObject` and let liveslots construct the `dispatch`.

## Vat lifecycle

```
initializeSwingset()              ← exactly once per swing-store DB; static vats created
─────
makeSwingsetController()          ← every reboot
upgradeSwingset(kernelStorage)    ← every reboot, before controller
controller.run()                  ← repeated; cranks the run-queue
hostStorage.commit()              ← block boundary
─────
E(vatAdminService).createVat(…)   ← dynamic vat creation, any time
adminNode.upgrade(newBundlecap)   ← dynamic vat upgrade
adminNode.terminateWithFailure()  ← explicit termination
exitVat(completion)               ← self-termination from inside the vat
```

Vat creation is itself a crank — the kernel evaluates the bundle's top-level forms and calls `buildRootObject`. This is "considerably longer" than subsequent message deliveries and is currently *not metered* per [`run-policy.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/run-policy.md). Worth flagging as an attack surface on a public vat-creation API.

## Delivery types: what arrives at `dispatch`

The kernel calls one of these on the vat (per [`delivery.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/delivery.md) and `kernel.js`):

- **`message`** — `dispatch.deliver(target, msg)`. Inbound method invocation. `target` is an `Object` or a `Promise` (the latter only if the vat is the decider for an unresolved promise *and* is a pipelining vat). `msg = { method, args: CapData, result: PromiseID? }`.
- **`notify`** — `dispatch.notify(resolutions)`. Inbound batch of promise resolutions. Each resolution is `(subject, Fulfill | Reject)`. Multiple resolutions in one syscall is for batches of mutually-referencing promises.
- **`bringOutYourDead`** — `dispatch.bringOutYourDead()`. The kernel periodically tells the vat "tell me what you've collected." Liveslots flushes pending GC syscalls. Not metered — no user code runs.
- **`dropExports(refs)`** / **`retireExports(refs)`** / **`retireImports(refs)`** — GC announcements about objects the kernel has noticed are unreachable or unrecognizable.
- **`startVat`** / **`stopVat`** — incarnation transitions for upgrade.

Each delivery is one crank. Inside it the vat may invoke any number of syscalls. The crank produces a "delivery results object" containing the success/failure status and (for xsnap workers) a **computron count** — an XS-engine-counted approximation of CPU work, used by the host's run policy.

## Syscalls the vat issues

From [`delivery.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/delivery.md), captured in Rust pseudocode:

```rust
trait Syscall {
    fn send(target: CapSlot, msg: Message);
    fn callNow(target: CapSlot, msg: Message) -> CapData;       // device only
    fn subscribe(id: PromiseID);
    fn resolve(resolutions: Vec<Resolution>);
    fn exit(isFailure: bool, info: CapData);
    fn vatstoreGet(key: String) -> String;
    fn vatstoreSet(key: String, value: String);
    fn vatstoreDelete(key: String);
    fn dropImports(refs: &[CapSlot]);
}
```

Notes:

- `send` is the asynchronous eventual-send. Wrapped in liveslots as `E(presence).method(args) → Promise`. Returns nothing synchronously; the vat receives the result via a future `notify` if it `subscribe`s.
- `callNow` is **synchronous** and only valid against device nodes. Wrapped as `D(deviceNode).method(args)`. Returns capdata immediately. This is the only synchronous syscall.
- `resolve` settles one or more promises the vat has decider authority over. Resolution forms: `Fulfill(CapData)` (a value or a single object), `Reject(CapData)` (an error). `Forward(PromiseID)` is in the spec but **not implemented**.
- `exit` self-terminates the vat at end of crank. `isFailure: true` aborts the crank's state changes; `false` commits them.
- `vatstoreGet/Set/Delete` is the per-vat string→string KV store. Liveslots uses it heavily for virtual and durable objects (the millions-of-purses use case). Not user-facing in most application code; the application uses durable kinds.
- `dropImports` is the GC syscall — vat tells kernel "I have no more strong reference to these imports."

## The transcript

Every delivery and the *full set of syscalls it issued* (with their return values) is appended to the vat's transcript. Transcripts are stored in `transcriptStore` (SQLite-backed), segmented into "incarnations" — a new incarnation begins on each upgrade.

Reload procedure (per [`persistence.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/persistence.md)):

1. Restore the most recent **heap snapshot** from `snapStore` if one exists for this incarnation. Snapshots are XS engine snapshots — full linear memory state of the xsnap worker.
2. Replay the transcript entries since that snapshot. Replace the syscall handler with one that *returns the recorded answer instead of executing the syscall*. The kernel state was already restored in step 0; we don't want syscalls to run twice.
3. If a syscall the replayed vat tries to make doesn't match the recorded transcript, **abort** — the vat is no longer deterministic; something has drifted.

Snapshot interval is governed by `snapshotInterval` (kernel option). Between snapshots, transcripts grow linearly with delivery count, so a long-running vat with no snapshots takes proportionally long to replay. xsnap-based vats can snapshot; `local`-worker vats cannot, so they replay from the start of the incarnation every reboot.

The transcript thus *is* the durable execution record of the vat. Determinism is the load-bearing assumption: any non-determinism (true RNG, wall-clock time, thread scheduling) makes replay impossible. SES + denying `Math.random` / `Date.now` / `WeakRef` / `FinalizationRegistry` to vat code is what enforces this.

## Vat upgrade

Dynamic vats can be upgraded via `E(adminNode).upgrade(newBundlecap, options)`:

- Most vat state is **discarded**, including the JS heap and any non-durable virtual objects.
- "Durable" collections and durable kinds are retained; the new vat code can rehydrate them from `baggage` and the vatstore.
- A "null upgrade" (re-using the same bundle) is a legitimate way to delete accumulated junk.
- The upgrade kicks off a new incarnation — transcript and snapshot streams reset for the new code.

Upgrade is not synchronous from the vat's perspective: the old crank completes, the kernel terminates the old worker, then starts the new one. All in-flight messages whose result promises have not been settled are rejected with `vat terminated` errors.

## Critical vats

The `critical: criticalVatKey` option (only available with the criticalVatKey object obtained from `vats.vatAdmin`'s root) marks a vat as so important that **the entire kernel panics** if it dies. `controller.run()` rejects, the host should refuse to commit. Used for, e.g., the chain's central economic vats. ([`dynamic-vats.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/dynamic-vats.md))

A normal `vatAdminService` does not let callers set `critical:`. The criticalVatKey gate is a deliberate POLA (principle of least authority) move — anyone given `vatAdminService` can create vats; only someone with `vatAdmin`'s root can mark them critical.

## Vat results, vat stats

The `adminNode` returned by `createVat` exposes:

- `done()` → Promise that fulfills (`exitVat`) or rejects (any other termination)
- `terminateWithFailure(reason)` → kill the vat
- `adminData()` → `{ objectCount, promiseCount, deviceCount, transcriptCount }`
- `upgrade(newBundlecap, options)` → upgrade in place

`transcriptCount` is a rough proxy for "messages delivered."

## Honest unflattering bits

- **Determinism is paid for, not given.** Every vat must be SES-locked, denied access to `WeakRef`/`FinalizationRegistry`/`Math.random`/`Date.now`. A real WASM-based vat (which Myrhiza wants) needs equivalent denials at the import-binding level.
- **`Forward`-resolution is still vapor.** Multiple SwingSet docs reference it as a planned feature; the kernel has a `Forward` enum variant marked `NOT YET IMPLEMENTED`. Don't plan on it being there.
- **Pipelining is comms-only in production.** Liveslots vats default to `enablePipelining: false`. The kernel queues messages on the kernel-side promise table until resolution, only delivering them speculatively for pipelining-opted-in vats.
- **Transcripts grow without snapshots.** A `local`-worker vat (which doesn't support xsnap snapshots) replays its entire incarnation every reboot. This is fine for a single-process dev runtime; a problem for any process expected to restart in production.
- **`dispatch.deliver()` is sometimes called `message` in the code.** The README admits "we're still in the process of refactoring and unifying the codebase." Mind the spelling when reading.
- **The crank-failure abort uses transcript replay to reset JS state.** If a crank exceeds its meter mid-execution, the kernel must rebuild the JS engine state to the start of the crank — by replaying the transcript. This is not cheap, and any vat that runs near its meter limit pays for it repeatedly.
- **Vat creation is unmetered.** `buildRootObject` and the bundle's top-level can do arbitrary work. A public `createVat` API is a DoS vector unless wrapped.
- **`disavow` is admitted-experimental.** The static-vats doc says "It's not clear that `disavow` is a good idea: it may be removed once the GC implementation is complete." A reminder that vat-side capability handling has rough edges.

## Implications for Myrhiza

- **Adopt the `buildRootObject` shape.** A WASM component exports one function that returns the root capability table. The kernel calls it once at component install time. This is a clean primitive and fits the Component Model nicely.
- **Map the four delivery types to our event API.** `dispatch.deliver` ↔ `state-apply` invocation; `dispatch.notify` ↔ promise/future settlement (we may not have promises in the same sense; Myrhiza's `state-propose` is closer to a request-with-future-result); `bringOutYourDead` is a periodic-GC tick we will eventually need; the export/import GC deliveries are reference-count signaling.
- **Transcript replay = our event log replay.** Myrhiza's `state-apply` profile already requires "pure function of (prior state, event)." That *is* SwingSet's invariant for the syscall transcript. Steal the design: per-component append-only log, periodic snapshot, replay-from-snapshot on reboot, syscall-result-recording so the kernel doesn't double-execute.
- **Computron-equivalent metering.** Whatever WASM engine we use needs deterministic gas accounting. SwingSet uses XS engine instruction counts; we'll likely use `wasmtime`'s fuel or a custom `metering` middleware. Make this a hard requirement up front — retrofitting metering is painful.
- **Critical-component flag, gated by capability.** SwingSet's `criticalVatKey` is a pattern worth copying. Our kernel should let an app declare itself critical only if it holds a capability for that authority — not by ambient flag.
- **Vat upgrade as new incarnation, durable baggage retained.** Myrhiza needs upgrade. The "discard most state, keep durable map, increment incarnation number" model is right. Avoid the temptation to make upgrades transparent — explicit incarnation boundaries make replay sane.
- **Don't expose vat-creation as ambient.** Components that can spawn other components must hold an explicit capability. Bootstrap distributes it; nobody else creates one out of thin air.
- **Static vs dynamic distinction is real.** Trusted runtime services (network, state-apply dispatcher, time) are static and live in the kernel bundle. Dynamic apps run with metering and isolation. We should not blur this line.
- **Reject `setup()`-style escape hatches.** SwingSet keeps a legacy `setup()` path for the comms vat to bypass liveslots. We won't have a comms vat in this exact form, and adding "raw vat" escape hatches creates determinism and security holes. If a component needs lower-level access, it should ask for a more powerful capability — not skip the framework.

## Sources

- [`packages/SwingSet/docs/delivery.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/delivery.md)
- [`packages/SwingSet/docs/static-vats.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/static-vats.md)
- [`packages/SwingSet/docs/dynamic-vats.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/dynamic-vats.md)
- [`packages/SwingSet/docs/persistence.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/persistence.md)
- [`packages/SwingSet/docs/state.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/state.md)
- [`packages/SwingSet/docs/run-policy.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/run-policy.md)
- [`packages/SwingSet/docs/host-app.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/host-app.md)
- [`packages/SwingSet/README.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/README.md)
- [`packages/SwingSet/src/kernel/kernel.js`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/src/kernel/kernel.js) — confirms `bringOutYourDead` delivery type
