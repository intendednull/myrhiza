**Date:** 2026-05-09
**Status:** active
**Subject:** Croquet / Multisynq — runtime architecture: TeaTime, reflector, model/view split, session lifecycle

# Architecture

Croquet is a **peer-replicated client-side runtime** in which every connected client runs a bit-identical virtual machine over the same input message stream and converges on byte-identical state. The server-side component is a **reflector** that orders messages and broadcasts them, but never executes application logic. This is the canonical implementation of the **lockstep deterministic VM** paradigm — the alternative-of-record to event-log replay (see [`agoric-endo/architecture.md`](../agoric-endo/architecture.md)) and to CRDT-merge ([`crdts/`](../crdts/)).

Sibling docs: [`determinism.md`](determinism.md), [`programming-model.md`](programming-model.md), [`multisynq-platform.md`](multisynq-platform.md), [`comparisons.md`](comparisons.md), [`governance.md`](governance.md), [`lessons.md`](lessons.md), [`open-problems.md`](open-problems.md), [`critiques.md`](critiques.md), [`glossary.md`](glossary.md).

## Key facts

| | |
|---|---|
| Origin | David A. Smith, Alan Kay, David P. Reed, Andreas Raab — *Croquet: A Collaboration System Architecture*, **C5 2003** (Conference on Creating, Connecting and Collaborating through Computing). Original implementation in **Squeak Smalltalk**. |
| Modern stack | JavaScript / WebAssembly. SDK `@multisynq/client` 1.1.0 (Apache-2.0, npm). Source at [`multisynq/multisynq-client`](https://github.com/multisynq/multisynq-client). |
| Legacy SDK | `@croquet/croquet` 2.0.4 — earlier versions shipped as proprietary "SEE LICENSE.md"; **the 2.0.4 republish (2025-06-09) carries `Apache-2.0`** in npm registry, aligning with the Multisynq open-source rebrand. |
| Reflector network | Multisynq DePIN reflector network. Croquet network deprecated **2025-07-30**; Croquet Labs is "primary provider to Multisynq Network." |
| Core architecture | Model-View-Synchronizer (MVS). The "Synchronizer" is the reflector. |
| Determinism | Strict. Models advance only via reflector messages; randomness via session-seeded PRNG; transcendentals via `@stdlib/math`. See [`determinism.md`](determinism.md). |

## Core abstractions: Model vs View

A Croquet/Multisynq application is partitioned into two trees of objects:

- **Models** — the replicated, deterministic core. Every client runs an identical instance, advancing in lockstep. Models hold all canonical application state. They subclass `Multisynq.Model` and are created via `Model.create()` (never `new`); each must call `Model.register("ClassName")` at module load so the snapshot serializer can find them ([`Model.html`](https://multisynq.github.io/multisynq-client/Model.html)).
- **Views** — the per-client, non-deterministic UI surface. Views subclass `Multisynq.View`, receive a reference to the synchronized model, and may *read* from it freely but must *never write*. Views handle DOM, input, animation, anything time-dependent on the local device. Each user runs their own view; views do not need to agree.

The "Prime Directive" from the upstream README is the load-bearing rule:

> *Your Multisynq Model must be completely self-contained.* The model must only interact with the outside world via subscriptions to user input events that are published by a view. Everything else needs to be 100% deterministic. The model must not read any state from global variables, and the view must not modify model state directly, only via publishing events.

Communication is publish/subscribe ([`tutorials/events-pub-sub`](https://docs.multisynq.io/tutorials/events-pub-sub)):

- **View → Model** (input events): routed through the reflector, broadcast to *every* replica in the same canonical order.
- **Model → View** (output events): generated locally on each replica, no network roundtrip — every view sees its model emit events in lockstep with every other.
- **Model → Model**: handled locally per-replica, never serialised onto the wire.
- **View → View**: local only.

## The reflector: a sequencer, not a server-of-truth

The reflector is the single most important architectural decision. From [`docs.multisynq.io/essentials/sync`](https://docs.multisynq.io/essentials/sync):

> "Reflector servers order all events into a single canonical stream"
> "Reflectors only pass messages — all logic runs on clients."

Concretely the reflector ([`controller.js`](https://github.com/multisynq/multisynq-client/blob/main/client/teatime/src/controller.js)):

1. Accepts encrypted client messages over a WebSocket.
2. Stamps each with a **(time, seq)** pair — `time` in reflector pseudo-milliseconds, `seq` a uint32 sequence number.
3. Broadcasts a `RECV` envelope `[time, seq, payload, …]` back to every client in the session in canonical order.
4. Emits `TICK` envelopes at the session's tick rate (default 20 Hz, configurable 1/30 Hz – 60 Hz via the `tps` option) so simulation time advances even with no user input.
5. Handles session-control protocol: `SYNC` on join, `TUTTI` voting (used for snapshot agreement and divergence detection), `SNAP` snapshot-URL announcement.

What the reflector explicitly does **not** do:

- It does not interpret message payloads (they are end-to-end encrypted by the session password — `controller.js` uses `crypto-js` PBKDF2/AES; the reflector cannot decrypt).
- It does not execute model code.
- It does not hold authoritative application state.
- It does not validate semantics; only ordering and transport.

This makes the reflector analogous to a **game-engine lockstep server** (Age of Empires, StarCraft) rather than to an authoritative MMO server. The compute model is peer-to-peer; only message ordering is centralised. A session's correctness depends on every client running the same code over the same `(time, seq)`-ordered stream.

## Session and instance lifecycle

`Multisynq.Session.join({ apiKey, appId, name, password, model, view, tps })` ([`Session.html`](https://multisynq.github.io/multisynq-client/Session.html)) is the only entry point. The flow ([`controller.js` SYNC handler, line ~1301](https://github.com/multisynq/multisynq-client/blob/main/client/teatime/src/controller.js)):

1. Client opens WS to a nearby reflector and sends `JOIN { sessionId, viewId, … }` where `sessionId = hash(name, codeHash, options)`.
2. Reflector replies with `SYNC { messages, url, persisted, time, seq, snapshotSeq, … }`. `url` points to the most recent **snapshot** (cloud-stored, encrypted, gzipped); `messages` is the tail of the message log since that snapshot.
3. New joiner downloads the snapshot, deserialises it into a `VirtualMachine` instance, then replays `messages` to fast-forward to the current `(time, seq)`.
4. The client's view is instantiated with a reference to the now-synced model. The reflector starts streaming `RECV`/`TICK` to it.

The first user to a session has no snapshot; the reflector calls back via SYNC with empty messages, and the client runs the model's `init()` to bootstrap. Subsequent users **do not** call `init()` — their state comes entirely from snapshot + replay.

## Snapshots

Snapshots ([`tutorials/snapshots`](https://docs.multisynq.io/tutorials/snapshots), [`controller.js#analyzeTally`](https://github.com/multisynq/multisynq-client/blob/main/client/teatime/src/controller.js)):

- The reflector periodically issues a `pollForSnapshot` request. Trigger conditions in `controller.js`: `SNAPSHOT_AFTER_CPU = 5000` ms of accumulated simulation CPU, or `DEPIN_SNAPSHOT_AFTER_TEATIME = 5 * 60 * 1000` ms of pseudo-time elapsed on DePIN reflectors. Debounce: `SNAPSHOT_POLL_DEBOUNCE = 5000`.
- Each client serialises its model state to JSON, hashes it with `fast-json-stable-stringify` for canonical ordering, and votes via the **TUTTI** ("all together") protocol with `{ hash, viewId }`.
- The reflector tallies votes and announces the result. The hash grouping is the **cross-client snapshot-equality test** — see [`determinism.md`](determinism.md). One client (the lowest-`viewId` voter for a given hash group) is asked to actually upload the encrypted snapshot to cloud storage; the URL is announced via `SNAP`.

If the tally splits into more than one hash group, `controller.js` logs:

> `Session diverged (#${previous})! Snapshots fall into ${numberOfGroups} groups`

and triggers `diffDivergedSnapshots()` to download two divergent snapshots and JSON-diff them for debugging. This is Croquet's only first-party divergence-detection mechanism. See [`determinism.md`](determinism.md) for what happens after divergence.

## Message types observed on the wire

From the reflector protocol implemented in `controller.js`:

| Direction | Type | Purpose |
|---|---|---|
| Reflector → Client | `SYNC` | Initial handshake; payload includes snapshot URL, message tail, current `(time, seq)`. |
| Reflector → Client | `RECV` | Ordered application message. Payload `[time, seq, encryptedPayload, …]`. |
| Reflector → Client | `TICK` | Heartbeat advancing simulation time. |
| Reflector → Client | `INFO` | Operator-injected information messages. |
| Client → Reflector | `JOIN` | Session join request. |
| Client → Reflector | `SEND` | Model-bound application message (rate-limited; `PAYLOAD_LIMIT_RECOMMENDED = 4 KiB`, `PAYLOAD_LIMIT_MAX = 16 KiB`). |
| Client → Reflector | `TUTTI` | Voted message — used for snapshot agreement, persistence votes, and `#reflected` user events that need divergence detection. |
| Client → Reflector | `SNAP` | Announces uploaded snapshot URL after winning the TUTTI vote. |

All `SEND`/`RECV` payloads are AES-encrypted with a key derived (PBKDF2) from the session password. The reflector cannot read application data; this is what enables Croquet to claim "one of the most private real-time multiplayer solutions available" (README).

## Heartbeat and the `tps` parameter

The default tick rate is 20 ticks/second; sessions can override via `tps` (an integer 1..60 with optional `xMultiplier` for locally-generated intermediate ticks). From `controller.js#getTickAndMultiplier`:

```
const tick = 1000 / Math.max(1/30, Math.min(60, rate));   // 1 tick per 30 s minimum
```

Heartbeat ticks exist *because* simulation time is event-driven — without ticks, a quiescent session's model would freeze. With ticks, animations, decay timers, and scheduled `this.future()` callbacks all advance deterministically without requiring user input.

The synced/unsynced classification (`SYNCED_MIN = 200 ms`, `SYNCED_MAX = 2000 ms`) is *not* part of consensus — it's a local UX signal for whether to render the view at all. A client whose tick stream lags is considered "unsynced" and may pause rendering until it catches up.

## Implications for Myrhiza

1. **TeaTime is one of two viable patterns for deterministic state-apply.** The other is Agoric SwingSet's event-log-replay-with-syscall-transcript. Croquet trades a sequencer dependency for cheaper write-side ordering; Agoric trades replay cost for fully decentralised single-author authority. Myrhiza's `state-apply` should pick deliberately — they are not the same thing wearing different clothes. See [`comparisons.md`](comparisons.md).

2. **A reflector is not a server-of-truth.** If Myrhiza adopts a sequencer (BFT-ordered or otherwise) for `state-apply` event ordering, model that role explicitly: the sequencer orders, peers compute. Mixing the roles is what gets Web2 designs into "the server is the application" trouble.

3. **Snapshot-equality voting is a real divergence detector — but only at snapshot cadence.** Croquet's clients can drift for up to ~5 s of pseudo-time before any divergence is noticed. For Myrhiza this means a `state-apply` purity audit cannot rely solely on periodic state-digest comparison; per-event hashing has to be considered.

4. **End-to-end encryption against the sequencer is achievable and load-bearing for trust.** The reflector being unable to decrypt payloads is what makes a centralised ordering service compatible with peer-owned data. Worth replicating.

5. **Snapshot-restore-then-replay is the canonical join path.** Both Croquet and SwingSet land on this pattern. New peers do not replay genesis-to-now; they load a recent snapshot and replay only the message tail. Myrhiza specs should plan for this from day one.

6. **Heartbeat ticks are required, not optional.** Any deterministic VM whose pseudo-time can advance independently of user input (animation, decay, scheduled work) needs an exogenous tick from the sequencer. Without it, "quiescent but still-progressing" sessions are impossible.

## Sources

- [`@multisynq/client` on npm](https://www.npmjs.com/package/@multisynq/client) — version 1.1.0, Apache-2.0, published 2025-07-24.
- [`multisynq/multisynq-client` GitHub](https://github.com/multisynq/multisynq-client) — 241 stars, Apache-2.0.
- [`client/teatime/src/controller.js`](https://github.com/multisynq/multisynq-client/blob/main/client/teatime/src/controller.js) — reflector protocol, snapshot voting, divergence detection.
- [`client/teatime/src/vm.js`](https://github.com/multisynq/multisynq-client/blob/main/client/teatime/src/vm.js) — VirtualMachine, message queue, seeded RNG.
- [`client/math/math.js`](https://github.com/multisynq/multisynq-client/blob/main/client/math/math.js) — `@stdlib/math`-based deterministic transcendentals; iOS `Math.pow` workaround.
- [Multisynq docs: Real-time Synchronization](https://docs.multisynq.io/essentials/sync)
- [Multisynq docs: Model-View-Synchronizer](https://docs.multisynq.io/tutorials/model-view-synchronizer)
- [Multisynq docs: Snapshots](https://docs.multisynq.io/tutorials/snapshots)
- [Multisynq docs: Sim Time & Future](https://docs.multisynq.io/tutorials/sim-time-future)
- [Multisynq docs: Writing a Multisynq Model](https://docs.multisynq.io/tutorials/writing-multisynq-model)
- [Multisynq API: Session.join](https://multisynq.github.io/multisynq-client/Session.html)
- [Multisynq API: Model](https://multisynq.github.io/multisynq-client/Model.html)
- [Wikipedia: Croquet Project](https://en.wikipedia.org/wiki/Croquet_Project) — origin (2001), Squeak roots, key authors.
- Smith, Kay, Raab, Reed — *Croquet: A Collaboration System Architecture*, C5 2003 (original paper, hosted at vpri.org; HTTPS retrieval intermittent).
