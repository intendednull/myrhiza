**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/SwingSet — how cross-validator determinism is engineered for production JavaScript

# Determinism in SwingSet

This is the load-bearing one. Agoric's SwingSet is the only system shipping deterministic-replay JavaScript at consensus quality on a public chain (Agoric mainnet, `agoric-3`, since 2021). The lessons here map almost line-for-line onto Myrhiza's `state-apply` purity story — see [Implications for Myrhiza](#implications-for-myrhiza) at the end.

Sibling docs: [`persistence.md`](persistence.md), [`architecture.md`](architecture.md), [`vat-model.md`](vat-model.md), [`hardened-js.md`](hardened-js.md), [`chain.md`](chain.md), [`critiques.md`](critiques.md), [`open-problems.md`](open-problems.md), [`lessons.md`](lessons.md), [`README.md`](README.md).

## What "deterministic" has to mean here

For a vat to participate in chain consensus, every validator running the same vat against the same delivery sequence must produce **byte-identical state** afterward. Not "semantically equivalent." Identical. If validator A's vat ends a crank with a JS Number and validator B's ends with a JS Integer (an XS-internal optimization), the snapshot hashes diverge and the chain forks. This actually happened — see [the gcc-9 incident](#real-incidents) below.

So "deterministic" here is a much stronger property than what most JS engines aim for. It means:

1. The same source code,
2. plus the same delivery sequence,
3. running on different hardware, OS versions, compilers, and wall-clock times,
4. must produce the same internal heap layout, byte-for-byte, at every commit boundary.

V8, SpiderMonkey, and JSC all violate this on purpose: they tier between interpreters and JITs based on heuristics, run timer-based GCs, expose performance counters, and reserve the right to reorder arbitrary internal state for throughput. None of them are usable as a consensus VM. SwingSet picked **XS** specifically because it is small, single-tier, and has a maintainer (Moddable) willing to treat byte-level determinism as a bug-fix-priority property.

## XS: the engine choice

XS is Moddable's JavaScript engine, originally built for embedded microcontrollers. SwingSet drives XS through a process called **`xsnap`** — a thin Node-managed wrapper that owns the XS heap and speaks a netstring protocol to the kernel ([`@agoric/xsnap` on npm](https://www.npmjs.com/package/@agoric/xsnap), `0.15.0` at the time of writing). Each vat worker is one `xsnap` process.

Why XS works for consensus where browser engines don't:

- **No JIT.** XS is a bytecode interpreter. There's no tier-up, no inline-cache shape, no profiler-driven recompilation. Two validators run the same instructions in the same order.
- **Heap snapshots are first-class.** XS has a built-in `xsSnapshot` mechanism that walks live objects and writes them out to a file. The file format is stable enough (with caveats — see incidents) that two validators starting from the same snapshot file resume execution at the same JS object identities.
- **Deterministic GC.** XS exposes an explicit `gc()` primitive and does not run GC on a timer. SwingSet calls `gc()` deterministically at end-of-crank rather than letting the engine pick.
- **Maintainership alignment.** Moddable's [Customer Stories page](https://www.moddable.com/agoric) is explicit: *"Moddable and Agoric have also collaborated to ensure fully deterministic execution of contracts under XS."* This is not an accident; it's a multi-year contract.

The XS source is vendored into agoric-sdk and pinned at a specific commit. Bumping XS is itself a chain-upgrade-class event because heap layout and bytecode encoding are part of consensus. See [`agoric-sdk#6361` "how to change XS on a deployed system"](https://github.com/Agoric/agoric-sdk/issues/6361).

## SES `lockdown()`: removing primordial non-determinism

XS handles bytecode-level determinism. **SES** (Hardened JavaScript) handles language-level determinism. Before any vat code runs, SwingSet calls `lockdown()` from the SES shim, which mutates the global intrinsics to remove obvious sources of nondeterminism ([Hardened JavaScript guide](https://github.com/endojs/endo/blob/master/packages/ses/docs/guide.md), [Agoric docs: Hardened JavaScript](https://docs.agoric.com/guides/js-programming/hardened-js)):

- `Math.random()` is removed (calling it throws).
- `Date.now()` returns `NaN`. `new Date()` constructed from clock returns "Invalid Date".
- Locale-sensitive functions (`toLocaleString`, etc.) are normalized to a single canonical locale.
- Built-in primordials are *frozen* — no monkey-patching `Array.prototype.map`. This isn't strictly determinism, but it cuts off a whole class of "vat A patched a primordial, vat B's replay sees a different one" bugs.

In addition, the SwingSet vat environment removes a long list of host globals that aren't deterministic by their nature ([SwingSet: vat-environment.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/vat-environment.md)):

- Timers: `setTimeout`, `setInterval`, `setImmediate`, `queueMicrotask`.
- I/O: `fetch`, `XMLHttpRequest`, `WebSocket`, all of `http`, `crypto` (Node), filesystem.
- Environment access: `process`, `global`, `URL`, `URLSearchParams`.
- WASM: yes, even `WebAssembly` is removed — its non-deterministic NaN canonicalization and float behavior is too risky.
- Encoding: `TextEncoder`, `TextDecoder`, `Buffer` — partly determinism, partly because vats shouldn't be reaching into Node primitives.
- `WeakRef` and `FinalizationRegistry` are denied to vat code (kernel/liveslots use them internally; see [GC](#gc-determinism)).

What's left is approximately: the language core (objects, arrays, math on regular numbers, regexes), `Promise`, `Compartment` (the SES module-realm primitive), `harden` (deep-freeze), `console`, and the syscall interface that liveslots provides. That's the deterministic kernel of vat code.

## Mediated I/O: the syscall boundary

A vat cannot perform I/O directly. The only way it can communicate with anything outside its own heap is through `syscall.*` calls to the kernel, and the only way the kernel can deliver anything to the vat is through `dispatch.deliver()`. Both directions are routed through the netstring pipe between the kernel process and the `xsnap` worker process.

This means: the vat's view of the world is fully captured by the *transcript* of `(delivery, [syscalls...], result)` tuples. Replaying the transcript reproduces the vat's behavior exactly, because the vat had no other inputs.

I/O that needs to happen — chain timers, IBC packets, validator signatures, oracle data — is owned by the kernel and by **devices** (kernel-side adapters that translate external events into deliveries to vats). Devices live outside the determinism boundary; the kernel re-applies their outputs deterministically by feeding them into the run-queue as ordinary deliveries.

This is the same architectural move Myrhiza is making with capabilities-as-imports: the deterministic core has no ambient authority, and every external signal arrives as a typed event.

## Metering: deterministic CPU accounting

Two problems show up once you have deterministic execution:

1. **Termination.** A vat can write `while(true){}`. The kernel needs to terminate it without that decision diverging across validators.
2. **Cost.** Vats consume validator CPU; the chain needs a fee model.

SwingSet's answer is **computrons** ([SwingSet: metering.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/metering.md)). XS is instrumented to increment a counter on basic operations (property access, arithmetic, function call, allocation). The counter is cheap, deterministic (it's a function of the bytecode executed, not wall-clock), and visible to the kernel.

Two limit tiers:

- **Per-crank limit.** Hardcoded at `DEFAULT_CRANK_METERING_LIMIT = 1e8` computrons. If a single delivery burns through 100M computrons, the vat is *terminated for cause* — its state is wiped and any waiting promises are rejected. This is the runaway-vat backstop.
- **Per-vat Meter objects.** Variable-capacity reservoirs that vats can opt into. End-of-crank, the consumed computrons are deducted; if the meter goes negative the vat halts. Meters can be replenished by a managing vat (e.g. Zoe for contracts).

Cross-validator agreement on computron counts is itself a determinism property. If validator A counts 1,234,567 computrons for a delivery and B counts 1,234,568, that's a consensus failure even if the vat's state is identical. The XS computron counter has been carefully tuned — and patched in response to incidents like [`#5040`](https://github.com/Agoric/agoric-sdk/issues/5040) "xsnap launches with different metering limit from snapshot vs from empty," where snapshot resumption initially produced different metering limits than fresh starts.

**Vat starvation.** Per-block, the kernel's run-policy decides how many cranks to run before yielding to the host application (see [SwingSet: run-policy.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/run-policy.md)). The policy uses computron totals to budget block work. A misbehaving vat can't starve others within a crank (per-crank limit) and can't starve them across cranks if the run-policy is round-robin / fair-queued.

## GC determinism

This is where the engineering gets uncomfortable. JavaScript GC is canonically nondeterministic — the spec gives implementations enormous latitude. SwingSet's solution is to **make GC happen deterministically at boundaries the kernel controls** rather than try to make the GC itself deterministic.

Mechanism ([SwingSet: garbage-collection.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/garbage-collection.md), [`#2615` sufficiently-deterministic GC](https://github.com/Agoric/agoric-sdk/issues/2615)):

1. **No GC inside a delivery.** Vats don't observe finalizer effects mid-crank.
2. **`gcAndFinalize()` at end-of-crank.** Liveslots calls a sequence — empirically `setImmediate, setImmediate, gc(), setImmediate` — that is sufficient to force XS to complete a full GC cycle and run all finalizers before returning control to the kernel.
3. **`bringOutYourDead` deliveries.** A separate kernel-initiated delivery type that runs GC and asks the vat to report newly-unreachable imported references via `syscall.dropImports` / `syscall.retireImports`. The kernel schedules these on a policy (initial proposal: every 10 deliveries per vat).
4. **No `WeakRef` / `FinalizationRegistry` in vat code.** These are reserved for liveslots, which uses them in its own bookkeeping but exposes only deterministic syscalls upward.
5. **Iteration order.** Vat code uses `Map` and `Set` for ordered iteration; objects' own-property order is well-defined since ES2015. SES `harden()` deep-freezes, which prevents mutation that would silently reorder.

Two modes coexist ([`#2615`](https://github.com/Agoric/agoric-sdk/issues/2615)):

- **Consensus mode** (on-chain): vats run under XS, GC fires deterministically at end-of-crank, `dropImports` is called immediately. Strict determinism required.
- **Solo mode** (single-machine, e.g. dev wallets): vats can run under Node.js, GC is "sufficiently deterministic" — finalizers may or may not run in any given crank, but eventually do. The chain doesn't care because there's no consensus to violate.

This split is itself a lesson: the system bifurcates the runtime environment based on whether consensus is required. Myrhiza's `state-apply` profile is the consensus-mode analogue.

## What's *outside* the determinism boundary

SwingSet is precise about what it does and doesn't make deterministic:

- **The kernel itself**: written in JS but runs in the host Node process, not under XS. The kernel's internal data structures don't need byte-level determinism *as long as* its observable outputs (syscalls dispatched, state writes committed) are a deterministic function of inputs. In practice the kernel is written carefully but is not held to the same bar as vat code.
- **Devices**: `mailbox`, `timer`, `bridge`, `vat-admin`. Devices are kernel-side adapters that handle external I/O. Their internals are non-deterministic; their *interface* to vats (the deliveries they enqueue) is what consensus rests on.
- **Host application**: cosmic-swingset (the Cosmos-SDK wrapper). Block production, IBC, P2P gossip — pure Cosmos-SDK / Tendermint territory, deterministic by Cosmos's own contract, not SwingSet's.
- **Snapshot bytes themselves.** As of this writing, **XS heap snapshots are *not* part of consensus state**. Validators don't agree on snapshot bytes. They agree on transcript content and on the kernel's kvStore state. See [`#5227 XS snapshot hash determinism`](https://github.com/Agoric/agoric-sdk/issues/5227): *"Snapshots are not currently part of consensus state."* Snapshots are a local optimization — replay from transcript reproduces state regardless of whether two validators have byte-identical snapshots. (This has tradeoffs; see [`persistence.md`](persistence.md).)

## Real incidents

Honesty section. SwingSet has had cross-validator divergence in production. Each is instructive.

### gcc-9 XS heap divergence (Emerynet, May 2023)

Documented in [`agoric-sdk#7829`](https://github.com/Agoric/agoric-sdk/issues/7829). During an Emerynet (one of Agoric's testnets, but a real validator network) upgrade, validators split into two groups producing different AppHashes. Diff localized to a single slot in a vault-manager vat: one cohort had it as `XS_NUMBER_KIND` (JS Number, IEEE-754 float), the other as `XS_INTEGER_KIND` (XS-internal 32-bit fast path).

Root cause: an XS optimization that promotes integer multiplication results to the integer fast path uses `__has_builtin()`, a clang/gcc-10+ macro. **gcc-9** (Ubuntu 20.04 LTS's default) doesn't define `__has_builtin()`, so the preprocessor branch silently took the slow `XS_NUMBER_KIND` path. Two validators on different host OSes diverged.

Fix: redefine `__has_builtin(x)=1` via a compiler flag in the build. The takeaway is that **even the C compiler used to build the JS engine is part of the consensus surface.** This is the kind of thing that doesn't show up on any spec.

### `setWakeup(0, "banana")` chain halt (January 14, 2022)

[`agoric-sdk#4297`](https://github.com/Agoric/agoric-sdk/issues/4297). A user submitted `E(agoric.chainTimerService).setWakeup(0, "banana")`, passing a string where a Presence was expected. The timer device's polling code crashed with `"SO(x) must be called on a Presence, not undefined"`. Because the crash was deterministic, every validator hit it on the same block — chain halt. Fixed in PR #5534 by adding input validation at the device boundary.

This is a classic deterministic-replay failure mode: a bug that *would* have been a single user's problem in a non-replicated system became a network-wide halt because every validator faithfully reproduced it. Lesson: kernel/device input validation is a consensus-safety property, not a UX property.

### Consensus failure on restart (March 2022)

[`agoric-sdk#4911`](https://github.com/Agoric/agoric-sdk/issues/4911), traced to [PR #4575](https://github.com/Agoric/agoric-sdk/pull/4575). A change to how vat creation was wrapped in a crank introduced divergence between the initial run and a restart-from-replay run: the replay produced different compute meter values and ultimately a different exception (`String.prototype.replace: this is undefined` vs. a missing-method error). Triggerable via the loadgen runner.

Lesson: replay equivalence is *itself* a property that requires testing, and "load and run" vs. "load, snapshot, restart, load, replay" are subtly different code paths. SwingSet ended up with a `loadgen` integration test specifically to catch replay divergence.

### Snapshot refcount double-delete crash (August 5, 2022)

[`agoric-sdk#5901`](https://github.com/Agoric/agoric-sdk/issues/5901). Snapshot refcounting confused itself, tried to `unlink()` a snapshot file that had already been removed, validator crashed with `ENOENT`. Not a consensus divergence — just a node-down event — but a useful reminder that the persistence layer needs the same care as the determinism layer.

## Relationship to chain consensus

The Agoric chain is a Cosmos-SDK chain using Tendermint (now CometBFT) for BFT consensus. Every block, validators:

1. Tendermint orders the transactions.
2. Each validator runs SwingSet on those transactions, producing kernel state changes (kvStore writes, transcript appends, etc.).
3. The resulting kvStore is stored in a Cosmos IAVL+ Merkle tree. The root hash of that tree is part of the AppHash that Tendermint reaches consensus on.
4. Validators sign the AppHash. Disagreement → fork → operator alerting → chain halt.

So the byte-level state that has to match across validators is **the kvStore (in IAVL)** and **the transcript content (now in SQLite, hashed and exported via the swing-store export API)**. XS heap snapshots are *not* in this set — they're treated as a local optimization. Replay from transcript is the canonical recovery path.

This is the cleanest production answer to "what state is consensus, what is local cache?" that exists in the BFT-replicated-VM space, and it's the model Myrhiza should copy almost verbatim.

## Implications for Myrhiza

This is the most concrete one. Borrow the design choices below; expect to face the gotchas; the questions will need answering in Myrhiza specs.

### Design choices to borrow

1. **Pick the WASM engine on determinism grounds, not performance grounds.** SwingSet picked XS because it's deterministic, even though it's slower than V8. Myrhiza should do the same: pick the WASM runtime (Wasmtime? wasmi? a custom interpreter?) that gives byte-level deterministic execution including float behavior, NaN canonicalization, and trap timing — *not* the fastest one. WASM has a slight advantage over JS here (the spec is tighter on floats and on memory model), but it isn't a free deterministic engine: see "Implementation determinism" in the WASM spec. Wasmtime has a `consume_fuel` mode and disables non-deterministic features; commit to that mode for `state-apply`.

2. **Two execution modes, not one.** SwingSet runs vats under XS in consensus mode and Node in solo mode. Myrhiza should do the analogue: `state-apply` runs under the strict deterministic profile (no SIMD, no threads, no waker-based async, fuel metering on, single-tier interpreter or AOT with deterministic codegen). `interaction` and `behavior` profiles can run under a faster engine. This is what the four-profile split is *for*; XS-vs-Node is the same distinction.

3. **Deterministic metering as a first-class kernel responsibility.** Computrons in SwingSet, fuel in WASM. Myrhiza needs:
   - A per-event hard limit (analog of per-crank limit) so a misbehaving `state-apply` doesn't hang a peer.
   - A counter that's a deterministic function of executed bytecode, so cross-peer fuel totals match.
   - A run-policy at the kernel level for budgeting per-block work across multiple state-apply executions.

4. **GC determinism by boundary, not by engine.** WASM Component Model uses linear memory; explicit GC isn't a thing in the same way. But there *is* an analogue: any kernel-side reference table (capability table, host-resource handles) must be cleaned up at deterministic boundaries. Borrow the `bringOutYourDead`-style explicit cleanup syscall: kernel asks the component to enumerate live capability handles at end-of-event, kernel reconciles its own table.

5. **Snapshots as cache, transcript as source of truth.** This is the single most valuable architectural decision in SwingSet, and it directly applies. Make Myrhiza's persistence layer:
   - **Transcript-of-events is consensus state** (Merkle-hashed, replicated).
   - **Memory snapshots are local cache** (not consensus, not Merkle-hashed for cross-peer agreement, just an optimization to avoid replaying from genesis).
   - Replay from transcript must reproduce state byte-identically *modulo the application-defined `state-digest()` export*. (Myrhiza already plans `state-digest()` instead of raw memory hash — that's the right call. SwingSet's pain with snapshot-byte-level determinism in `#7829` is precisely the bug `state-digest()` avoids.)

6. **SES `lockdown()` analogue at WIT level.** SES achieves determinism by mutating intrinsics at runtime. Component Model achieves it *better*: imports are part of the type, statically checkable. A `state-apply` component declaring an import of `wasi:clocks` should fail at link, not at runtime. Codify the deterministic import set in WIT and validate at link.

### Determinism gotchas Myrhiza will face

1. **The compiler is part of consensus.** The gcc-9 incident is going to happen to Myrhiza in some form. If Myrhiza compiles its WASM runtime AOT with LLVM, two peers building with different LLVM versions can produce different generated code that produces different float rounding, different trap order, or different fuel costs. Either: (a) ship a single canonical runtime binary, distributed not built; (b) require a specific toolchain version pinned and reproduced; (c) use an interpreter, accept the perf hit, and minimize the consensus-relevant codegen surface. SwingSet effectively chose (b)+(c). Pick deliberately.

2. **NaN, denormals, float rounding.** WASM's spec is stricter than JS but still has `"canonical NaN"` non-determinism in some operations. Myrhiza spec needs an explicit position: forbid float in `state-apply`? Require canonical-NaN-only? Use a deterministic-float subset? Don't punt this.

3. **Replay equivalence is its own test surface.** Agoric only caught the [`#4911`](https://github.com/Agoric/agoric-sdk/issues/4911) restart divergence with a load-test runner that exercised the snapshot-restart path. Myrhiza's CI needs `(run from genesis)` vs. `(run, snapshot, restart, replay)` equivalence tests from day one. They will catch real bugs.

4. **Input validation at the kernel/device boundary is a consensus-safety property.** The "banana" halt was *not* malicious — it was a missing type check. Every kernel-side adapter that takes data from outside the deterministic core (network, user input, oracle) needs validation that itself is deterministic. A kernel panic is a chain halt.

5. **Metering counter changes are themselves consensus-breaking.** If Myrhiza tunes fuel costs after launch (cheaper for a hot opcode, say), every existing transcript replays differently. Either: never tune costs, or version the cost table and apply it per-event-epoch. SwingSet chose the latter and it's still painful.

6. **Determinism of `state-apply` *relative to a deterministic helper set*.** SwingSet allows `console.log` (no observable effect on state), structured cloning, `Promise` (deterministic given deterministic scheduling), etc. Myrhiza needs an equivalent allowlist of deterministic helpers — and a discipline for adding to it (each addition is a determinism audit).

### Questions Myrhiza specs will need to answer

1. **Which WASM runtime is the consensus runtime?** Pin a specific implementation and version. Document why. (Wasmtime with fuel and `epoch_interruption` disabled is the obvious starting candidate, but verify the float-determinism and resource-table-iteration-order story.)

2. **What is in the deterministic import set for `state-apply`?** Spell out the allowlist of WIT interfaces. `wasi:io`, `wasi:clocks`, `wasi:random` are *out*. What's *in*? Likely just: storage-read on pre-event state, structured clone, hashing primitives, deterministic decoders.

3. **What is the per-event fuel limit, and how is it set?** SwingSet picked 1e8 computrons by feel and rarely adjusted. Pick a number; document the empirical basis.

4. **What is Myrhiza's analogue of `bringOutYourDead`?** Component Model has explicit resource lifetime, so this might not be needed in the same form — but the kernel still has a capability table that needs deterministic GC. Spell out the protocol.

5. **What survives state-apply restart, and what doesn't?** Mirrors of SwingSet's "durable vs. ephemeral" distinction. See [`persistence.md`](persistence.md) — these go together.

6. **Is the WASM heap part of consensus, or only `state-digest()`?** Myrhiza's design choice is `state-digest()`, but it needs to be in the spec explicitly with the failure modes named. The XS-snapshot-hash divergence in [`#5227`](https://github.com/Agoric/agoric-sdk/issues/5227) is the precedent for why raw heap hashing is fragile.

7. **How does Myrhiza handle metering-cost changes across versions?** Inevitable. Need a story.

8. **What is the test matrix for cross-peer determinism?** At minimum: (a) genesis-to-current vs. (b) snapshot-restart-to-current, on (c) different host OS, (d) different CPU architecture, (e) different compiler version of the runtime. SwingSet learned the hard way that all four axes matter.

## Sources

- [`@agoric/swingset-vat` on npm](https://www.npmjs.com/package/@agoric/swingset-vat) — version `0.33.0`, published 2026-04-08 (current as of writing)
- [`@agoric/xsnap` on npm](https://www.npmjs.com/package/@agoric/xsnap) — version `0.15.0`, published 2026-04-08
- [Agoric/agoric-sdk: SwingSet package](https://github.com/Agoric/agoric-sdk/tree/master/packages/SwingSet)
- [SwingSet: vat-environment.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/vat-environment.md)
- [SwingSet: garbage-collection.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/garbage-collection.md)
- [SwingSet: metering.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/metering.md)
- [SwingSet: run-policy.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/run-policy.md)
- [Hardened JavaScript / SES guide](https://github.com/endojs/endo/blob/master/packages/ses/docs/guide.md)
- [Agoric docs: Hardened JavaScript](https://docs.agoric.com/guides/js-programming/hardened-js)
- [Moddable customer story: Agoric](https://www.moddable.com/agoric)
- [`agoric-sdk#47` SwingSet on xs: exploratory prototypes](https://github.com/Agoric/agoric-sdk/issues/47)
- [`agoric-sdk#2615` sufficiently-deterministic GC](https://github.com/Agoric/agoric-sdk/issues/2615)
- [`agoric-sdk#5227` XS snapshot hash determinism](https://github.com/Agoric/agoric-sdk/issues/5227)
- [`agoric-sdk#5040` xsnap launches with different metering limit from snapshot vs from empty](https://github.com/Agoric/agoric-sdk/issues/5040)
- [`agoric-sdk#6361` how to change XS on a deployed system](https://github.com/Agoric/agoric-sdk/issues/6361)
- [`agoric-sdk#7829` divergent XS heap snapshots on gcc-9](https://github.com/Agoric/agoric-sdk/issues/7829)
- [`agoric-sdk#4297` Chain halt: setWakeup(0, "banana")](https://github.com/Agoric/agoric-sdk/issues/4297)
- [`agoric-sdk#4911` Consensus failure since vat creation as a crank](https://github.com/Agoric/agoric-sdk/issues/4911)
- [`agoric-sdk#5901` chain node crashed trying to delete missing XS heap snapshot](https://github.com/Agoric/agoric-sdk/issues/5901)
