**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/SwingSet — kernel-and-vats architecture, c-lists, run-queue, devices

# Architecture

SwingSet is the deterministic vat host that lives inside `agoric-sdk`'s `packages/SwingSet` directory. It is the layer at which Agoric runs JavaScript "vats" as if they were userspace processes, with a kernel mediating every interaction. Latest published `@agoric/swingset-vat` release on npm is `0.33.0` ([dist-tag `latest`, modified 2026-05-07](https://registry.npmjs.org/@agoric/swingset-vat)); the chain itself is currently rolling toward `agoric-upgrade-23` ([rc1 published 2026-05-06](https://github.com/Agoric/agoric-sdk/releases/tag/agoric-upgrade-23-rc1)).

The mental model the [SwingSet `delivery.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/delivery.md) leans on: vats are Unix-style userspace processes; the kernel is the OS; `syscall` is the only way out of a vat; `dispatch` is the only way the kernel pushes work in. Apps don't talk to apps; apps talk to the kernel about other apps.

## What is and isn't in the kernel

The kernel proper holds:

- **Object table** — `KernelObjectID` (`koNN`) keyed rows recording each object's owner vat. Created when a vat first exports an object; deleted when no c-list, message, or promise references it.
- **Promise table** — `KernelPromiseID` (`kpNN`) keyed rows. Each row is `Unresolved { decider, subscribers[], queued_messages[] }`, `Fulfilled(CapData)`, or `Rejected(CapData)`. A `Forwarded` state is sketched in the docs but [not yet implemented](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/delivery.md).
- **Per-vat c-list** — capability-list mapping `kref ↔ vref`. Stored as kvStore entries `${vatID}.c.${kref}` → `${flag} ${vref}` and the reverse. The flag is `R` (reachable) or `_` (recognizable only). See [`c-lists.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/c-lists.md).
- **Run-queue** — a `VecDeque<PendingOperation>` of pending `Send` and `Notify` events. Plus a higher-priority "acceptance queue" (route-only crank), a "GC actions queue", and a "reap queue" of vatIDs that need a `bringOutYourDead` delivery. The full priority order is in [`run-policy.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/run-policy.md).
- **Bundle store, snapshot store, transcript store** — all backed by SQLite via `@agoric/swing-store` (`better-sqlite3`). Confirmed in [`packages/swing-store/src/swingStore.js`](https://github.com/Agoric/agoric-sdk/blob/master/packages/swing-store/src/swingStore.js) (`import sqlite3 from 'better-sqlite3';`).

Everything else — including the `vatAdmin` vat, the `comms` vat, the `timer` vat, ERTP issuers, Zoe — lives in vats. The kernel is small and dumb on purpose; "everything is a vat" is a load-bearing slogan, not marketing.

## kref vs vref vs oref

The terms are easy to confuse. The names that the SwingSet codebase actually uses are `kref` and `vref`; "oref" appears only in Spritely/E literature for the same concept. ([`c-lists.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/c-lists.md))

| Term | Lives in | Format | Allocator |
|---|---|---|---|
| `kref` | Kernel side | `koNN`, `kpNN`, `kdNN` (object / promise / device) | Kernel |
| `vref` | Vat side | `o+`, `o-`, `p+`, `p-`, `d+`, `d-` + suffix | Sign indicates side that allocated |
| `roNN` / `rpNN` | Comms vat over the wire | Per-remote-machine c-list | Comms vat per peer |

A `vref` like `o+v6/1:0` is liveslots' encoding for `(virtual, kind 6, instance 1, facet 0)` — the kernel doesn't usually parse the suffix, just the `o+` / `o-` prefix and (for upgrade) the `T` virtual/durable flag.

The asymmetry between `+` (locally allocated) and `-` (kernel allocated) is what lets a single name space cover both directions through the syscall/dispatch boundary without renaming. The comms vat doesn't have this asymmetry across two peers, so it adds the `r` prefix and the sign tracks "did the receiver or the sender allocate this number?" — see [`captp-and-network.md`](./captp-and-network.md).

## C-list invariants

The kernel owns the c-list. Vats never see krefs and cannot read or directly modify the table. The implications matter:

- A vat cannot fabricate a `kref` reference to anything it has not been previously granted. New `o-` imports only enter the c-list as a side effect of an inbound delivery containing them. This is the wire-level enforcement of capability discipline — there is no separate ACL.
- The kernel translates *every* slot at the syscall and dispatch boundaries. Forgetting to translate is a kernel bug, not a vat bug.
- A vat's c-list grows with everything it has ever touched until GC removes entries. Liveslots' GC syscalls (`dropImports`, `retireImports`, `retireExports`) drive this; see [`garbage-collection.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/garbage-collection.md). `WeakRef` and `FinalizationRegistry` are denied to vat code because their timing is non-deterministic.

## The crank, the turn, the block

Three nested execution units, defined in [`state.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/state.md):

- **Turn** — call-stack to call-stack. One synchronous run-to-completion JS task.
- **Crank** — empty-promise-queue to empty-promise-queue. One delivery into one vat, including all its `.then` callbacks. The unit of vat-side transactional commitment. Crank state changes go to a **crank buffer**; if the crank fails (illegal syscall, out-of-meter), the buffer is discarded.
- **Block** — a host-defined batch of cranks. The unit of *durable* commitment to the SwingStore. Outbound IO is embargoed until the block commits — Agoric calls this the "hangover inconsistency" defense, citing Waterken/E.

The host application drives this loop:

```js
const { hostStorage, kernelStorage } = openSwingStore(baseDir);
upgradeSwingset(kernelStorage);
const controller = makeSwingsetController(kernelStorage, deviceEndowments);
controller.injectQueuedUpgradeEvents();
// ... per block:
for (const input of deviceInputs) { injectDeviceInput(input); await controller.run(); }
hostStorage.commit();
emitDeviceOutputs();
```

(From [`host-app.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/host-app.md).)

## Run-queue and message-dispatch order

The run policy queues, in priority order ([`run-policy.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/run-policy.md)):

1. **Acceptance queue** — routing-only crank (no vat work; just reshuffles into more specific queues). Reported to the policy as `policy.emptyCrank()`.
2. **GC Actions queue** — `dropExports`, `retireExports`, `retireImports` deliveries. Provoked by reference-count transitions.
3. **Reap queue** — vat IDs that need a `bringOutYourDead` delivery to flush dead virtual references.
4. **Run queue** — regular `Send` / `Notify`, plus vat creation.

Each `controller.run(policy)` keeps cranking while `policy.crankComplete({computrons})` returns truthy. Computrons are an XS engine count; meter limits and per-block budgets compose via the policy object.

Send dispatch when a `Send` reaches the front:

| Target | State | Action |
|---|---|---|
| Object | n/a | `dispatch.deliver` to owning vat |
| Promise | Unresolved, decider has `enablePipelining` | `dispatch.deliver` to decider |
| Promise | Unresolved, decider does not pipeline | queue inside the kernel promise table |
| Promise | Fulfilled to object | recurse with the object as new target |
| Promise | Fulfilled to data | reject result with `CannotSendToData` |
| Promise | Rejected | reject result with rejection contagion |

(From [`delivery.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/delivery.md).) Currently only the comms vat has `enablePipelining: true` in production.

## Devices: the ocean

Vats can only touch the outside world through **devices**, which are the only place in the system where host endowments — file handles, network sockets, current time, randomness — appear ([`devices.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/devices.md)).

Distinguishing properties of devices vs vats:

- Device methods are invoked **synchronously** through `syscall.callNow(deviceRef, method, capdata) -> capdata`, contrasted with the asynchronous `syscall.send` used for vats. Liveslots wraps these as `D(deviceNode).method(args)` and `E(presence).method(args)` respectively.
- Devices receive **endowments** at controller construction. Vats never do — they only see other vats and (transitively) device nodes that bootstrap shared with them.
- Devices don't get orthogonal persistence. They must call `getDeviceState`/`setDeviceState` to checkpoint after each invocation that mutates state.
- Devices cannot create kernel promises. `syscall.sendOnly` (the device → vat path) is fire-and-forget; vat → device `callNow` returns capdata synchronously and that's it.
- Devices have no transcript. They have a kvStore (raw devices can call `vatstoreGet/Set/Delete`) and that's the durable record.

Initially only the bootstrap function holds device references; capability discipline forces the bootstrap to deliberately hand them out. Standard kernel devices: `vatAdmin` (paired with `vats.vatAdmin`, used to create dynamic vats), `mailbox` (out-of-band byte transport for the comms vat), `timer` (paired with `vats.timer`), `command` (HTTP/WebSocket bridge), `bundle` (`bundleID` → bundlecap conversion).

The mailbox device deserves attention: it is *not* a network stack. It is a kernel-state-vector slot. The host loop is expected to "let the kernel quiesce, then examine this mailbox for new outbound messages, then deliver them externally." The device pushes bytes into a slot of the durable state; the host's after-commit hook copies them onto the wire (in Agoric's chain case: into the IBC packet stream). See [`captp-and-network.md`](./captp-and-network.md).

## Static vats vs dynamic vats vs kernel-built-in

Three populations, all reachable through the same `dispatch`/`syscall` API:

- **Static vats** — defined in `config.vats` at boot time. Source bundles are fixed for the life of the swing-store database. Examples include the application's bootstrap vat. ([`static-vats.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/static-vats.md))
- **Built-in vats** — currently `vatAdmin`. Auto-added; the host doesn't list them in `config.vats`. The timer and comms vats *are defined by SwingSet* but the host application has to add them to `config.vats` itself, so they are static-vats-with-a-canonical-implementation rather than built-in. The README notes these "might be turned into built-in vats in the future."
- **Dynamic vats** — created at runtime via `E(vatAdminService).createVat(bundlecap, options)`. Source arrives over the network as a bundle, gets installed via `controller.validateAndInstallBundle`, and a bundlecap (capability for the bundle) is what `createVat` accepts. Dynamic vats are *metered by default* and run in xsnap workers. Each Zoe contract instance is a dynamic vat. ([`dynamic-vats.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/dynamic-vats.md))

The ability to create vats is not ambient — `vatAdminService` is itself a capability that bootstrap must explicitly hand out.

## State storage: SwingStore

The kernel state DB is `@agoric/swing-store` (npm latest `0.10.0`, modified 2026-05-07). Backing engine is **`better-sqlite3`**. It used to be LMDB; the [v0.9.0 changelog](https://github.com/Agoric/agoric-sdk/blob/master/packages/swing-store/CHANGELOG.md) records a "convert swing-store from LMDB to Sqlite" feature in May 2023, plus moving snapshots into the same SQLite DB.

The SwingStore exposes four facets:

- **kvStore** — string→string key-value, holds c-lists, kernel object/promise tables, run queue heads, vat metadata.
- **transcriptStore** — append-only per-vat delivery log, segmented into "incarnations" for vat upgrades.
- **snapStore** — XS heap snapshots, taken every N deliveries to truncate transcripts.
- **bundleStore** — content-addressed bundle source by `bundleID`. Bundle IDs follow the Endo `b1-<lowercase-hex(SHA-512(compartment-map.json))>` scheme — the hash is over the structured manifest, not the raw zip bytes — see [`./modules-and-bundling.md`](./modules-and-bundling.md).

Crank-buffer / block-buffer transactional layering is built on SQLite save points. Per the [v0.9.0 changelog](https://github.com/Agoric/agoric-sdk/blob/master/packages/swing-store/CHANGELOG.md): "use Sqlite save points for crank commit, integrate activity hash into swing-store."

## Honest unflattering bits

- **The mailbox device is the network "stack."** It's a kernel-state slot the host scrapes after commit. Off-chain transport, retries, framing — all host responsibilities, not SwingSet's.
- **Pipelining is barely turned on.** The README explicitly notes pipelining is "not currently the case for any Liveslots Vats" and is "most (only?) useful on the Comms Vat" ([`delivery.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/delivery.md)). The code path exists; the gain in production is comms-vat-only.
- **`Forward` resolution is not implemented.** Promise-to-promise forwarding is described in the spec but absent from the runtime. Kernel-side state machine treats forward as a TODO.
- **Determinism is bought with denial.** `WeakRef`, `FinalizationRegistry`, `Math.random`, `Date.now`, native modules — all withheld. SES locks the realm down before any vat code runs. This buys replayability at the cost of "ordinary JS code mostly works" being a half-truth.
- **Replay cost.** Restoring a vat means replaying its transcript from the last snapshot. Vats accumulate snapshots on a `snapshotInterval`-driven cadence, but a long-lived vat with infrequent snapshots can take a measurable startup hit on reboot.
- **Liveslots is large.** A meaningful fraction of SwingSet's complexity (virtual objects, durable kinds, weak collections, GC) lives in `swingset-liveslots` rather than the kernel proper. The kernel's small surface is real, but "everything is a vat" hides a lot of code under the vat layer.

## Implications for Myrhiza

Concrete things to import or avoid:

- **kref/vref split is the right model for Myrhiza's per-component capability tables.** A component never sees the kernel-side identifier, only its own; the kernel translates every slot at the boundary. This is the mechanism that gives us "components cannot fabricate references they were not granted" without runtime ACL checks. Plan to adopt this directly.
- **Crank/block layering maps to our event-apply model.** A `state-apply` invocation should be exactly one crank in SwingSet terms — purely deterministic, runs against a per-event buffer, commits or aborts atomically. A "block" in our world is whatever we choose for state-sync boundaries; for now, one crank = one block is fine, and we can introduce batching later. The crank-buffer pattern is the right way to keep aborted events from corrupting state.
- **Devices are the model for our capabilities.** Host-mediated I/O surfaces, synchronous read paths, asynchronous write paths, no promises crossing the boundary. The "endowment at controller construction" pattern matches our "kernel-mediated capability declared at component install" intent. Steal the asymmetry: capabilities have a synchronous request/response (like `callNow`) and a fire-and-forget callback (like `sendOnly`).
- **Don't put anything you wouldn't trust into "everything is an X."** SwingSet's "everything is a vat" works because static vats live in the kernel bundle and run in the same process; dynamic vats run in xsnap workers and are metered. Myrhiza needs the same separation: trusted static components (timer, network) vs untrusted dynamic apps. Don't hand-wave the difference.
- **State sync via export/import.** SwingStore implemented `export/import` for state-sync support of Cosmos validators. We will need an equivalent for joining peers; copy the design pattern (content-addressed artifacts + a metadata stream) instead of inventing one.
- **Skip the Forward-resolution rabbit hole.** Agoric described it years ago and still hasn't shipped it. If a feature is simple in the spec but absent in the implementation after years, the simple-in-spec is misleading. Be conservative about adopting promise forwarding in our state-apply model.
- **The mailbox device pattern is mediocre but explainable.** It is *not* a clean network abstraction; it's "write bytes into kvStore, host reads them after commit." For Myrhiza, our network capability should look more like the netlayer interface from OCapN ([../spritely-ocapn/captp-and-ocapn.md](../spritely-ocapn/captp-and-ocapn.md)) than the mailbox device — but the *commit-before-emit* invariant is non-negotiable and we should adopt it.

## Sources

- [@agoric/swingset-vat on npm](https://registry.npmjs.org/@agoric/swingset-vat) — version anchor `0.33.0`, modified 2026-05-07
- [@agoric/swing-store on npm](https://registry.npmjs.org/@agoric/swing-store) — version anchor `0.10.0`, modified 2026-05-07
- [agoric-upgrade-23-rc1 release](https://github.com/Agoric/agoric-sdk/releases/tag/agoric-upgrade-23-rc1) — published 2026-05-06
- [SwingSet README](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/README.md)
- [`docs/delivery.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/delivery.md)
- [`docs/c-lists.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/c-lists.md)
- [`docs/devices.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/devices.md)
- [`docs/static-vats.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/static-vats.md)
- [`docs/dynamic-vats.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/dynamic-vats.md)
- [`docs/state.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/state.md)
- [`docs/host-app.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/host-app.md)
- [`docs/run-policy.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/run-policy.md)
- [`docs/garbage-collection.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/garbage-collection.md)
- [swing-store CHANGELOG (LMDB→SQLite at v0.9.0, 2023-05)](https://github.com/Agoric/agoric-sdk/blob/master/packages/swing-store/CHANGELOG.md)
- [`packages/swing-store/src/swingStore.js`](https://github.com/Agoric/agoric-sdk/blob/master/packages/swing-store/src/swingStore.js)
