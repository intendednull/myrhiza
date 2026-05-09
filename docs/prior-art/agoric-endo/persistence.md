**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/SwingSet — vat snapshot, transcript, replay, upgrade, and the storage backend

# Persistence in SwingSet

The companion to [`determinism.md`](determinism.md). Determinism gives you "the same inputs produce the same state." Persistence is how you *durably store* those inputs and that state, recover from crashes and restarts, and upgrade running code without losing identity. SwingSet has shipped this in production on Agoric mainnet (`agoric-3`) since 2021; the design has been tested by ~23 chain upgrades to date.

Sibling docs: [`determinism.md`](determinism.md), [`architecture.md`](architecture.md), [`vat-model.md`](vat-model.md), [`chain.md`](chain.md), [`contracts.md`](contracts.md), [`history.md`](history.md), [`critiques.md`](critiques.md), [`open-problems.md`](open-problems.md), [`lessons.md`](lessons.md), [`README.md`](README.md).

## The orthogonal-persistence pattern

The headline idea: **vats are written as if they ran forever in RAM.** No `connect`/`disconnect`. No `serialize`/`deserialize`. No `save()` calls. The vat's JavaScript object graph is the database. The kernel transparently snapshots and replays.

This is the orthogonal-persistence pattern from KeyKOS / EROS / Smalltalk image-based systems. SwingSet's contribution is making it work in 2020s-vintage JS for a BFT-replicated VM. Compare with the explicit `persist`/`reify` patterns of e.g. Erlang/OTP `gen_server` (you write `init`, `terminate`, `code_change` callbacks): SwingSet asks the developer to write *less*, and absorbs the complexity in the kernel.

The price is real:

- The vat's heap must be *deterministically* reconstructible from a transcript. See [`determinism.md`](determinism.md) — it's load-bearing for everything in this file.
- The "developer never thinks about persistence" promise leaks at upgrade time, because new code can't run on old object graphs. SwingSet's answer is the **baggage** convention. See [vat upgrade](#vat-upgrade) below.
- The runtime cost of replay scales with transcript length. SwingSet's answer is **periodic heap snapshots** so replay starts from the most recent snapshot rather than genesis.

## Three-layer persistence: snapshot, transcript, kvStore

SwingSet's per-vat durable state has three pieces ([SwingSet: state.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/state.md), [SwingSet: transcript.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/transcript.md)):

1. **XS heap snapshot.** A binary blob — the full live JS object graph of the vat, taken by `xsSnapshot`. 2–20 MB compressed. Re-loading the snapshot reproduces the in-memory state of the vat at snapshot time.

2. **Transcript.** Append-only log of every delivery the vat received and every syscall it made, plus the result of the delivery. Stored as JSON-ish records. Replaying a transcript entry calls `dispatch()` with the same delivery, intercepts any syscalls the vat tries to make, compares them to the recorded syscalls, and panics if they diverge.

3. **vatstore (a slice of kvStore).** A key-value space scoped to the vat, used by liveslots for *virtual* and *durable* objects — JS objects whose state lives in kvStore rather than (or in addition to) RAM. This is how vats hold large data without exceeding heap limits, and is the substrate the baggage convention is built on.

To boot a vat from cold:

1. Load the most recent **heap snapshot** for that vat into a fresh `xsnap` worker.
2. Replay every **transcript** entry recorded *after* that snapshot, asserting syscall equivalence at each step.
3. The vat is now in the same state it was at end-of-last-crank.

If there's no snapshot (e.g. vat just created), step 1 is "load the source bundle and run `buildRootObject`," and step 2 starts from delivery 0.

If there's a snapshot, step 2 typically replays only the few-hundred-to-few-thousand deliveries since the snapshot rather than the millions since vat creation. Snapshot frequency is tunable.

## Spans and incarnations

The transcript is divided structurally ([SwingSet: transcript.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/transcript.md)):

- **Incarnation.** A run of a vat under a particular code bundle. Each `upgradeVat` starts a new incarnation. Incarnations are numbered.
- **Span.** A run of a vat under a single `xsnap` worker process. Spans are bounded by `initialize-worker` / `load-snapshot` events at the start, and `save-snapshot` / `shutdown-worker` events at the end. A new span begins whenever a snapshot is taken.

So the structure is: incarnation 1 contains spans 1.1, 1.2, 1.3 (with snapshots between them); upgrade triggers incarnation 2 starting with span 2.1; etc. Replay of the *current* incarnation only needs the current span — earlier spans are already represented in the most recent snapshot.

This nesting matters for export and pruning: old incarnations' transcript spans can be pruned (the upgraded vat no longer cares about its old code's deliveries), but cross-incarnation invariants (durable state, baggage) survive. See [vat upgrade](#vat-upgrade).

## Snapshot frequency and the snapshot-as-cache stance

When does the kernel snapshot? Configurable per-vat via `snapshotInterval`, defaulting to every 200 deliveries (this number has drifted across releases; check the source). The kernel can also force a snapshot at upgrade time, at end-of-block during certain validator events, and on-demand for state-sync.

Crucially: **XS heap snapshots are not part of cross-validator consensus state** ([`agoric-sdk#5227`](https://github.com/Agoric/agoric-sdk/issues/5227)). The kvStore (in the kernel's IAVL tree) and the transcript content (hashed) are consensus-critical. Snapshots are a local-validator optimization: each validator independently snapshots its own xsnap workers, and validators are *expected* to produce identical snapshot bytes (and they mostly do), but if they don't the chain doesn't fork — replay-from-transcript is the canonical recovery.

This is a deliberate hedge against the exact failure mode that hit Emerynet in [`#7829`](https://github.com/Agoric/agoric-sdk/issues/7829): the gcc-9 incident produced *divergent snapshot bytes*. If snapshots were consensus, the chain would have halted. Because they aren't, validators that detected the divergence could re-replay from transcript and recover, and the bug was fixable as a build-toolchain patch rather than a chain rollback.

The pattern: **transcript is canonical, snapshot is cache.** Worth tattooing somewhere.

## The swing-store: SQLite-backed unified state

All of the above lives in a single durable store: **swing-store** ([`@agoric/swing-store` on npm](https://www.npmjs.com/package/@agoric/swing-store), `0.10.0` at time of writing). swing-store is the backing store for the kernel; the entire durable state of the kernel lives in it ([`swingstore.md`](https://github.com/Agoric/agoric-sdk/blob/master/packages/swing-store/docs/swingstore.md)).

Four sub-stores:

| Sub-store | Holds |
|---|---|
| `kvStore` | String-to-string KV. Holds c-lists (cross-vat reference tables), kernel run-queue, vatstore (per-vat durable state), miscellaneous metadata. |
| `transcriptStore` | Append-only vat deliveries, organized into spans and incarnations. |
| `snapStore` | XS heap snapshot blobs (compressed). |
| `bundleStore` | Source-code bundles for vats. |

All four live in **one SQLite database file**: `swingstore.sqlite` (with the usual `-wal` and `-shm` companions). One commit point for the whole thing: `hostStorage.commit()`. ACID atomicity across all four sub-stores.

### From LMDB to SQLite

This wasn't always the case. Earlier versions used a more fragmented backend:

- **`agoric-sdk#899` (2020):** kernel state was moved to **LMDB** (`switch cosmic-swingset to use LMDB for kernel storage`).
- **Pre-SQLite era:** kvStore was LMDB, snapStore was loose files in a directory (hash-named `.gz` files), commits were not atomic across stores. The application had to interlock: snapStore committed first, kvStore second, host IAVL third.
- **`agoric-sdk#6742` (filed Jan 2023):** "move snapstore (XS heap snapshots) into SQLite." Motivation: clean atomicity across snapshot creation and the kvStore reference to that snapshot. The split-database design had bugs ([`#5901`](https://github.com/Agoric/agoric-sdk/issues/5901): refcount confusion, double-`unlink()`).
- **`agoric-sdk#3087` (umbrella):** merge all swing-store DBs into a single SQLite instance.
- **`agoric-sdk#6254`:** considered per-worker SQLite vatstore DBs (rejected/deferred in favor of monolithic).

By the current generation of the code, all four sub-stores are SQLite tables in one file. The migration was pushed by operational pain: cross-store consistency bugs, the difficulty of state-sync export with multiple storage backends, and the validator-operator complaint that "what's even on disk?" had a multi-paragraph answer.

**Why SQLite specifically:** widely audited, ACID, exists-on-every-validator-already, supports indexed iteration (which the kvStore relies on for c-list scans). The swing-store docs note: *"swing-store takes advantage of SQL's indexing and iteration abilities."* Replacing SQLite would be hard.

### Keying scheme

kvStore keys are flat strings with structured prefixes. Examples (paraphrased from agoric-sdk source):

- `kernel.<...>` — kernel-wide tables (vatID counters, run-queue head, etc.)
- `vNN.<...>` — per-vat state, where `NN` is the vat ID
- `vNN.vs.<userKey>` — vatstore (vat's own durable kv space, used by liveslots for virtual/durable objects)
- `local.vNN.lastSnapshot` — pointer from a vat to its current snapshot ID (in snapStore)
- `local.snapshot.<id>` — snapshot metadata

The `local.*` prefix is a marker: keys under `local.*` are *not* part of the consensus state hash — they're local-validator metadata. This is how snapshots are kept out of consensus while still being indexed in the same DB.

### Consensus surface and state-sync

Every validator's swing-store must agree on the consensus-critical sub-keys. The kernel exposes an export API ([`#6773 swing-store export/restore API for state-sync`](https://github.com/Agoric/agoric-sdk/issues/6773)) that produces a deterministic byte stream of:

- All non-`local.*` kvStore keys (consensus state).
- Transcript spans, with metadata.
- Snapshot artifacts, with metadata records (the artifact bytes themselves are local; the *metadata* — vatID, span boundary, hash — is consensus).
- Bundle store contents.

This export is hashed and the hash is part of the AppHash that Tendermint reaches consensus on. Validators joining via state-sync receive the exported artifacts and can either accept the snapshot bytes verbatim (fast) or re-replay from transcript (paranoid) to reconstruct vat state.

This is the cleanest production answer to "how do new validators join a deterministic-replay chain?" available. Most chains' state-sync schemes are simpler because their VM doesn't have non-consensus per-validator caches; SwingSet's complication is that it *does* (snapshots), and it has carefully separated the consensus-critical bytes from the optimizable cache.

## Vat upgrade

You ship code. The code has bugs or needs new features. The vat has been running for two years and has 100,000 references held by users / other vats. You can't kill it and start over without breaking promises (literally).

SwingSet's answer: **`upgradeVat`** ([SwingSet: vat-upgrade.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/vat-upgrade.md), [`agoric-sdk#1848`](https://github.com/Agoric/agoric-sdk/issues/1848)).

```
E(adminNode).upgrade(newBundleCap, options)
```

This enqueues an upgrade event on the kernel run-queue. When processed:

1. The old worker (and its xsnap process) is **shut down**.
2. The vat's transcript spans for the old incarnation are sealed. The snapshot for the old incarnation is **deleted** — it's now unreachable, since v2 won't replay v1's deliveries.
3. **Identity is preserved:** the vat keeps its vatID, its c-list (cross-vat reference table), its imports/exports as kernel objects, and its vatstore contents.
4. v2's source bundle is loaded into a **fresh xsnap process** (new incarnation #N+1, span 1).
5. v2's `buildRootObject(vatPowers, vatParameters, baggage)` is called. It receives:
   - Fresh `vatPowers` (host-provided helpers).
   - The new `vatParameters` from the upgrade call.
   - The **baggage**, a durable Map that v1 left for v2.

What survives the upgrade:

- **Durable objects.** Created via `prepareExoClass` / `prepareDurableExoClass` / zone APIs with `{ durable: true }`. Their state is in vatstore; v2 must re-declare the kinds (with the same kind handles) to access them.
- **Durable collections.** `makeScalarBigMapStore('foo', { durable: true })`.
- **Imported references** (Presences pointing at other vats' objects), if held in durable storage.
- **Anything in baggage.** Baggage is the bootstrap entry point — it's how v2 finds everything else.

What does *not* survive:

- **Heap-only objects.** Anything that wasn't durable is gone.
- **Merely-virtual objects** (virtual-but-not-durable). Their state was being paged to vatstore for memory savings, but that storage is wiped at upgrade.
- **Outstanding promises.** Promises that were unresolved when upgrade ran are *rejected* with an upgrade-disconnect error. Callers must re-issue.
- **Unexported in-flight messages.** The run-queue is preserved at the kernel level, but messages targeting non-durable objects fail.

### The baggage convention

Critical detail: at least one baggage key must be a **plain string** (or other primitive), because v2 starts with no object references from v1. The string key is how v2 bootstraps access to the durable graph. From `vat-upgrade.md`: *"v2 starts with only source code and baggage — no object references from v1."*

So the v1 → v2 contract is: v1 packs important roots into baggage under known string keys, v2 looks them up by string and reattaches.

```js
// v1
baggage.init('mainStore', makeScalarBigMapStore('main', { durable: true }));

// v2
const mainStore = baggage.get('mainStore');
// reattach kinds, define new ones
```

If v2 fails to re-declare a kind that has surviving instances, the upgrade aborts. Kind handles must be reachable from baggage so they can be redefined; this is enforced and a common source of upgrade failures ([`agoric-sdk#7578`](https://github.com/Agoric/agoric-sdk/issues/7578)).

### "Null upgrade"

A degenerate but useful case: upgrade v1 → v1 (same bundle). This is the validation move — if your vat survives a null upgrade cleanly, your durable / kind declarations are correct. If it doesn't, you have an upgrade-correctness bug *before* you ever try real upgrades. Agoric's contract-upgrade tests use null upgrade as a basic smoke test.

### Upgrade rollback / failure

If v2's `buildRootObject` throws, the upgrade fails. The kernel:

- Rolls back the transition (the new incarnation is discarded).
- v1 stays alive at the prior state — old worker is restarted, old transcript replayed.
- The upgrade promise rejects with the v2 error.

This is non-trivial because the old worker was already shut down and its snapshot deleted. In practice the kernel doesn't delete the v1 snapshot until v2 has successfully bootstrapped; the order of operations is `start v2 → success → delete v1` rather than `delete v1 → start v2`.

## Distributed GC and finalization

Vat A holds a reference to an object exported by vat B. Vat A drops that reference. How does vat B learn its object is no longer reachable?

Mechanism (high level — see [SwingSet: garbage-collection.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/garbage-collection.md) and [`determinism.md`](determinism.md) for the GC-determinism story):

1. **In-vat tracking via liveslots.** Each vat's liveslots layer maintains `valToSlot` (WeakMap from JS value to vref) and `slotToVal` (Map from vref to WeakRef of the JS value). When a JS object becomes unreachable, the WeakRef goes empty and a finalizer fires.

2. **End-of-crank reconciliation.** After every delivery, liveslots calls `gcAndFinalize()`, which forces XS to complete a GC cycle and run finalizers. Newly-dead imports are placed in a `deadSet`.

3. **Syscalls upward.** Liveslots reports newly-dead imports to the kernel via `syscall.dropImports(vrefs)` and `syscall.retireImports(vrefs)`. "Drop" means "I no longer hold this strongly"; "retire" means "I have no record of it at all and can't even recognize it again."

4. **Kernel reference counting.** Per kernel object, two counters: **reachable** (number of vats with a live reference) and **recognizable** (number that could still recognize the object if it came back). The kernel updates these on every drop/retire syscall.

5. **Push-down to exporting vat.** When `reachable` drops to zero, the kernel sends a `dispatch.dropExports(vrefs)` delivery to the exporting vat. When `recognizable` drops to zero, it sends `dispatch.retireExports(vrefs)`. The exporting vat can now actually free the object.

6. **`bringOutYourDead` deliveries.** Periodic kernel-initiated deliveries (default policy: every 10 deliveries per vat, or on a kernel-wide LRU sweep) that ask the vat to do `gcAndFinalize` and report any pending dead imports. Without these, a vat that never receives messages would never collect its garbage.

The whole protocol must be deterministic. Each `dropImports`/`retireImports` syscall, each `dropExports`/`retireExports` delivery, is a transcript event. Two validators replaying the same transcript see the same GC operations in the same order.

## Failure modes

### Replay divergence

The headline failure: replay produces different syscalls than the original execution.

- Detection: replay machinery compares each replayed syscall against the recorded one and panics on first mismatch.
- Diagnosis: typically a determinism leak (a non-deterministic primordial slipped past lockdown, a Map-iteration order surprise, a timing-dependent code path).
- Recovery: there is no recovery in consensus mode. The vat is wedged. Validator restart doesn't help (replay diverges again the same way). The chain halts; humans investigate.
- Real example: [`agoric-sdk#4911`](https://github.com/Agoric/agoric-sdk/issues/4911) — a change in how vat creation was wrapped in a crank caused initial-run vs. restart-run divergence.

### Snapshot corruption

Less catastrophic than replay divergence because of the cache-not-canonical stance. If a snapshot is bad on one validator:

- Replay-from-transcript path skips the snapshot entirely and reproduces state from the start of the current span (or, in the worst case, from the start of the incarnation).
- Other validators with good snapshots are unaffected.
- Real example: [`agoric-sdk#5901`](https://github.com/Agoric/agoric-sdk/issues/5901) — a refcounting bug deleted a snapshot that the kvStore still pointed at, validator crashed with `ENOENT`. Single-validator crash, not a chain halt.

### Cross-validator snapshot divergence

Snapshots aren't consensus, so divergence is *not directly* fatal. But it indicates a determinism bug that *will* eventually become a kvStore divergence. [`agoric-sdk#5227`](https://github.com/Agoric/agoric-sdk/issues/5227) reports cases where validators producing different snapshot hashes despite identical inputs. [`agoric-sdk#7829`](https://github.com/Agoric/agoric-sdk/issues/7829) is the gcc-9 case where divergence escalated to a different kvStore byte and an actual chain split on Emerynet. Treating snapshot-hash divergence as a leading indicator (even though it isn't directly consensus-breaking) is the right validator-ops posture.

### Upgrade rollback

When v2 fails to boot — kind not redefined, baggage missing required key, throw in `buildRootObject` — the upgrade aborts and v1 stays live. This is correct behavior, but it means a botched upgrade leaves the chain wedged on the *old* code, with users' upgrade-resolved promises hanging. Operators have to ship a fixed v2 quickly. Agoric's contract-upgrade workflow ([Agoric docs: Contract Upgrade](https://docs.agoric.com/guides/zoe/contract-upgrade)) recommends extensive testing of null upgrades and v1→v2 in a local test before mainnet.

### Kernel-restart-after-crash

The host application (cosmic-swingset) wraps the kernel. If the kernel-process crashes mid-block, Cosmos-SDK rolls back the block (Tendermint hasn't committed yet). On restart, the kernel reloads from swing-store at the last commit, replays the block's transactions, and (deterministically) reaches the same state. The "banana" halt ([`#4297`](https://github.com/Agoric/agoric-sdk/issues/4297)) is an example: every validator hit the same kernel crash on the same block, the chain halted, the fix was deployed, validators restarted with a patched binary, and the block was re-attempted. Crash-recovery via deterministic-replay is the same mechanism that gives you cross-validator consensus — they're the same engineering artifact.

## Implications for Myrhiza

The most concrete one alongside [`determinism.md`](determinism.md). Myrhiza's architecture choices in this layer should be made with these lessons explicit.

### Design choices to borrow

1. **Three-layer persistence: snapshot, transcript, kvStore.** Adopt this almost verbatim:
   - **Event log** (= transcript). Append-only, the canonical state-derivation source. Hashed, replicated, part of consensus / convergence.
   - **State snapshot** (= XS heap snapshot). The component's serialized memory at a point in time. *Not* part of consensus. A local optimization to avoid re-running the whole event log on restart.
   - **Component KV** (= vatstore). Per-component durable KV storage backed by the same store as the kernel's, scoped by component ID. Useful for state too large to fit in linear memory.

2. **Make the event log the consensus surface; treat snapshot as cache.** This is the single most important architectural bet SwingSet makes. Codify it: Myrhiza's `state-digest()` runs over event-log-derived state, *not* over raw memory. Snapshots are recoverable from event log; if a peer's snapshot diverges, replay from log catches up. Cross-peer convergence is checked at semantic level, not byte level.

3. **One backend, ACID across all sub-stores.** SwingSet started with three storage subsystems (LMDB kvStore + loose files for snapshots + Cosmos IAVL) and migrated to one SQLite database after years of cross-store-consistency pain. Skip the migration: pick one storage backend with ACID transactions across all categories of state. SQLite is the obvious candidate (ACID, audited, indexable, on every platform). Alternatives: redb, sled — but verify ACID story before committing.

4. **Transcript spans and snapshot intervals.** Adopt the "span = run-of-execution-between-snapshots" structuring. A snapshot at the start of every span lets you bound replay cost. The kernel decides snapshot timing based on a policy (every N events, on idle, before upgrade, before block-end on a timer); this is tunable, not on the critical path of correctness.

5. **Component upgrade with identity preservation + baggage convention.**
   - Component upgrade keeps the component's identity (its address, its capability handles, its KV namespace).
   - Old memory is wiped, old code is unloaded, new code's `init`/`buildRoot` runs.
   - A typed "baggage" interface — at minimum a string-keyed durable map — bridges versions. New code looks up old roots by string key and reattaches.
   - **Null upgrade is a smoke test.** Every component upgrade story should include "v1 → v1 (same bundle) preserves all state." If it doesn't, the durable-state declarations are wrong.

6. **Distributed reference counting at the kernel level.** When components hold capability references to each other, the kernel maintains the (reachable, recognizable) counts and pushes down `drop`/`retire` notifications to the exporting component. Borrow this; don't try to do cross-component GC at the application level.

7. **`bringOutYourDead` analogue.** Periodic kernel-initiated GC events to components, especially ones not currently receiving traffic, so capability cleanup happens promptly even on idle components.

### Determinism gotchas Myrhiza will face (persistence-specific)

1. **Snapshot bytes will diverge across hosts.** Even if Myrhiza's `state-apply` is deterministic at the WASM-spec level, the *engine's* internal representations (Wasmtime's `mmap`'d linear memory layout, control-flow recovery state, internal hash table iteration order) won't be byte-identical across hosts running different libc versions, kernels, or CPUs. Don't try. State convergence is `state-digest()`, not snapshot hash. Confirm this in spec.

2. **Replay equivalence is its own test surface.** Agoric only caught [`#4911`](https://github.com/Agoric/agoric-sdk/issues/4911) by running a load-test that exercised the snapshot-restart path. Myrhiza needs from-genesis vs. from-snapshot-restart equivalence tests as a continuous-CI matrix.

3. **Upgrade testing matters more than it looks.** Botched upgrades on mainnet are extremely hard to recover from. Agoric's solution: an explicit `agoric-3-proposals` repo where every mainnet upgrade is rehearsed against a snapshot of the live chain before deployment ([Agoric/agoric-3-proposals](https://github.com/Agoric/agoric-3-proposals)). Borrow the *practice*: a runnable rehearsal of every Myrhiza protocol upgrade against captured production transcripts.

4. **Cross-validator snapshot divergence is a leading indicator.** Even if snapshots aren't consensus, validators should compare snapshot hashes. Divergence means there's a determinism bug brewing, even if it hasn't yet escalated to kvStore divergence.

5. **Storage backend changes are extremely expensive.** SwingSet's LMDB-to-SQLite migration took years and required careful per-validator data migration. Pick once, commit hard.

6. **Kernel input validation = consensus safety.** The "banana" halt was fundamentally a missing type check at the kernel/device boundary. Every input from non-deterministic adapters into the deterministic core needs validation, and that validation is itself part of the consensus surface — change it carelessly and old transcripts replay differently.

### Questions Myrhiza specs will need to answer

1. **What is the canonical Myrhiza event log format?** Schema, encoding (CBOR? Postcard? SCALE?), per-event size budget. Must be deterministic to encode, deterministically hashable, and append-friendly.

2. **What does the per-component KV namespace look like at the WIT level?** SwingSet's vatstore is a string→string map exposed via syscalls. Myrhiza's analogue is probably an imported WIT interface on the deterministic side. Spec it: typed keys? structured values? versioning?

3. **Snapshot frequency policy.** Per-component? Configurable by the component? Forced by kernel at certain boundaries (block end, upgrade, idle)? Default value?

4. **What is in baggage at upgrade time?** The shape of the upgrade-bridge interface. Plain strings + capability handles? Typed migration helpers? See [Holochain](../holochain/determinism.md)'s migration story for contrast.

5. **Upgrade atomicity: when does the old version actually go away?** SwingSet keeps v1 snapshot until v2 boots. Myrhiza needs the same discipline; spec it.

6. **What's the failure-mode response matrix?** Replay divergence = ? Snapshot corruption = ? Component-init throw on upgrade = ? Distinguish recoverable (single-peer issue, replay from log) from unrecoverable (cross-peer divergence, requires human intervention).

7. **State-sync / new-peer-join protocol.** SwingSet's swing-store export API is the model. Myrhiza needs an analogue: when a new peer joins a Myrhiza app, how does it bootstrap component state? Replay from genesis (slow but verifiable)? Accept exported snapshots from a trusted peer (fast but trust-dependent)? Hybrid (accept snapshots with metadata hashes from quorum, replay from log if hashes diverge)?

8. **Upgrade rehearsal workflow.** Agoric's `agoric-3-proposals` is the right shape. Myrhiza needs a similar workflow doc: how is a runtime change (or a host-WIT change, which is even more sensitive) rehearsed before deployment?

9. **GC syscall protocol at the WIT level.** Drop / retire / bring-out-your-dead. How do these map onto Component Model resource types, which already have explicit lifetimes? There may be cleanup needed only for *application-level* references (not Component Model resources), and that's where the SwingSet-style protocol applies.

10. **What's the maximum supported transcript length before snapshot is required?** SwingSet operationally tolerates ~hundreds-of-thousands of deliveries between snapshots; replay scales linearly. Myrhiza should set bounds and instrument them.

## Sources

- [`@agoric/swing-store` on npm](https://www.npmjs.com/package/@agoric/swing-store) — version `0.10.0`, published 2026-04-08
- [`@agoric/swingset-vat` on npm](https://www.npmjs.com/package/@agoric/swingset-vat) — version `0.33.0`, published 2026-04-08
- [Agoric/agoric-sdk: SwingSet package](https://github.com/Agoric/agoric-sdk/tree/master/packages/SwingSet)
- [SwingSet: state.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/state.md)
- [SwingSet: transcript.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/transcript.md)
- [SwingSet: vat-upgrade.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/vat-upgrade.md)
- [SwingSet: garbage-collection.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/garbage-collection.md)
- [SwingSet: persistence.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/persistence.md)
- [swing-store: swingstore.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/swing-store/docs/swingstore.md)
- [swing-store: snapstore.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/swing-store/docs/snapstore.md)
- [swing-store: transcriptstore.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/swing-store/docs/transcriptstore.md)
- [swing-store: kvstore.md](https://github.com/Agoric/agoric-sdk/blob/master/packages/swing-store/docs/kvstore.md)
- [Agoric docs: Contract Upgrade](https://docs.agoric.com/guides/zoe/contract-upgrade)
- [Agoric/agoric-3-proposals](https://github.com/Agoric/agoric-3-proposals)
- [`agoric-sdk#511` swingset JS-engine heap snapshots](https://github.com/Agoric/agoric-sdk/issues/511)
- [`agoric-sdk#899` switch cosmic-swingset to use LMDB for kernel storage](https://github.com/Agoric/agoric-sdk/issues/899)
- [`agoric-sdk#1846` Strawman ideas for vat secondary storage](https://github.com/Agoric/agoric-sdk/issues/1846)
- [`agoric-sdk#1848` kernel API for upgrading vats](https://github.com/Agoric/agoric-sdk/issues/1848)
- [`agoric-sdk#2273` design "snapstore" API: immutable hash-named XS snapshot files](https://github.com/Agoric/agoric-sdk/issues/2273)
- [`agoric-sdk#3767` implement dispatch.bringOutYourDead](https://github.com/Agoric/agoric-sdk/issues/3767)
- [`agoric-sdk#4297` Chain halt: setWakeup(0, "banana")](https://github.com/Agoric/agoric-sdk/issues/4297)
- [`agoric-sdk#4911` Consensus failure since vat creation as a crank](https://github.com/Agoric/agoric-sdk/issues/4911)
- [`agoric-sdk#5227` XS snapshot hash determinism](https://github.com/Agoric/agoric-sdk/issues/5227)
- [`agoric-sdk#5811` add tentative API to upgrade static vats](https://github.com/Agoric/agoric-sdk/issues/5811)
- [`agoric-sdk#5901` chain node crashed trying to delete missing XS heap snapshot](https://github.com/Agoric/agoric-sdk/issues/5901)
- [`agoric-sdk#6254` idea for giving each worker its own (local) SQLite vatStore DB](https://github.com/Agoric/agoric-sdk/issues/6254)
- [`agoric-sdk#6742` move snapstore (XS heap snapshots) into SQLite](https://github.com/Agoric/agoric-sdk/issues/6742)
- [`agoric-sdk#6773` swing-store export/restore API (for state-sync)](https://github.com/Agoric/agoric-sdk/issues/6773)
- [`agoric-sdk#7244` Don't send stopVat during upgrade](https://github.com/Agoric/agoric-sdk/pull/7244)
- [`agoric-sdk#7578` Audit makeScalarBigMapStore usages for pseudo baggage](https://github.com/Agoric/agoric-sdk/issues/7578)
- [`agoric-sdk#7829` divergent XS heap snapshots on gcc-9](https://github.com/Agoric/agoric-sdk/issues/7829)
