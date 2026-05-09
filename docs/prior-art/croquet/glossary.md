**Date:** 2026-05-09
**Status:** active
**Subject:** Glossary of Croquet/Multisynq terms used across this folder

# Glossary

Croquet/Multisynq-specific vocabulary used across [architecture.md](architecture.md), [determinism.md](determinism.md), [programming-model.md](programming-model.md), [multisynq-platform.md](multisynq-platform.md), and the cross-cutting files.

## Project + organization

- **Croquet Project** — academic effort 2001-~2010 led by David A. Smith, Alan Kay, David P. Reed, Andreas Raab. Original implementation in Squeak Smalltalk. MIT-licensed. Largely dormant by 2010. See [governance.md](governance.md).
- **Croquet Corporation** — commercial revival May 2018; JavaScript rewrite. $2.7M seed funding. Closed-source SDK. See [governance.md](governance.md).
- **Croquet Labs** — sub-brand / operating arm. Currently described as "primary provider to Multisynq Network."
- **Multisynq Network** — current 2024 rebrand. DePIN-positioned. Apache-2.0 SDK. Croquet network deprecated 2025-07-30. See [governance.md](governance.md).
- **C5 2003** — Conference on Creating, Connecting and Collaborating through Computing. Venue of the original 2003 Croquet paper. NOT OOPSLA (common citation error).
- **VPRI** — Viewpoints Research Institute, Alan Kay's nonprofit; one of the original Croquet Project funders.

## Core architecture

- **Model** — replicated, deterministic class. State that all peers compute identically. Subclass `Model`; create via `Model.create(...)`. See [programming-model.md](programming-model.md).
- **View** — per-client, non-deterministic class. UI / DOM / animation. Subclass `View`. View can read Model state; View cannot mutate Model state directly — must publish a message.
- **Synchronizer** (formerly "reflector") — the message-ordering server. Receives messages from clients, assigns sequence numbers, broadcasts back to all clients in canonical order. The Synchronizer NEVER computes application state. See [architecture.md](architecture.md).
- **TeaTime** — the original 2003 collaboration protocol name. Defines simulated pseudo-time, message ordering, and replicated computation. The modern stack inherits the architecture but rebuilt the implementation in JS.
- **Model-View-Synchronizer (MVS)** — Croquet's variant of MVC where the Synchronizer is the third leg.

## Determinism mechanics

- **Pseudo-time / virtual time** — Croquet's "now" is *not* `Date.now()`. It's a virtual clock advanced by Synchronizer-emitted `TICK` messages. Models read time via `this.now()`. See [determinism.md](determinism.md).
- **Tick rate** — frequency of automatic time-advance messages. Default 20 Hz. Range 1/30 Hz – 60 Hz.
- **Seeded RNG** — `this.random()` in Models uses `seedrandom` library keyed to the snapshot ID, so all replicas produce identical random sequences. Verified at `vm.js:481`. See [determinism.md](determinism.md).
- **`@stdlib/math`** — Croquet replaces JS's `Math.sin/cos/pow/exp/log` with deterministic implementations from `@stdlib/math/base/special/*`. The `client/math/math.js` patch contains a documented iOS-Safari `Math.pow` workaround. See [determinism.md](determinism.md).
- **`fast-json-stable-stringify`** — library Croquet uses to produce canonical hashes of Model state for snapshot-equality checks across replicas. See [determinism.md](determinism.md).
- **TUTTI** — Croquet's snapshot-equality voting mechanism. All clients hash their Model state via `fast-json-stable-stringify` and submit the hash to the Synchronizer. If hashes diverge, the Synchronizer logs `Session diverged (#${previous})!` but **does not auto-recover** — the session continues with divergent state. See [determinism.md](determinism.md), [open-problems.md](open-problems.md).

## Programming-model abstractions

- **`Model.create(...)`** — the only way to instantiate a Model. Direct `new` is forbidden because the framework needs to assign deterministic IDs and register the instance in the snapshot-able tree.
- **`this.future(ms).method()`** — schedule a future invocation of `method` on this Model after `ms` of *virtual* time. The deterministic-runtime equivalent of `setTimeout`. See [programming-model.md](programming-model.md).
- **`this.publish(scope, event, data)`** — publish a message. Messages flow Model→Model and Model→View immediately; View→Model goes through the Synchronizer first.
- **`this.subscribe(scope, event, handler)`** — register a handler for a `(scope, event)` pair.
- **Prime Directive** — Croquet's serialization rules. Model state must be JSON-serializable; non-serializable types (functions, DOM nodes, sockets) cannot live in Model. See [programming-model.md](programming-model.md).
- **`Session.join({apiKey, appId, name, password, model, view})`** — entry point. Returns a Session that runs the deterministic runtime. The `(apiKey, appId, name, password, code-hash)` tuple identifies the session; clients with mismatched tuples can't join.
- **`wellKnownModel`** — a registered Model accessible by name across the session, even from Views. The "global" Model.

## Wire protocol

- **`SYNC`** — initial synchronization message; client receives current snapshot.
- **`RECV`** — client receives a message in the deterministic order assigned by the Synchronizer.
- **`TICK`** — periodic time-advance message. Even with no user input, ticks advance pseudo-time.
- **`TUTTI`** — snapshot-equality vote initiation.
- **`SNAP`** — snapshot upload/download.

## Multisynq platform

- **Synq Key** — credential issued by Multisynq the company, required to operate a Synchronizer node. Permissioned operation. See [multisynq-platform.md](multisynq-platform.md), [governance.md](governance.md).
- **DePIN** — Decentralized Physical Infrastructure Network. Multisynq's framing for the reflector-operator network. Operators run nodes for token rewards.
- **`synchronizer-cli`** — Apache-2.0 CLI tool to operate a Synchronizer node. See [multisynq-platform.md](multisynq-platform.md).
- **API key** (free) — issued via multisynq.io/coder. Lets developers authenticate against the Multisynq Network. Distinct from a Synq Key (operator credential).
- **Code hash** — part of the session-scope tuple. Two clients with different bundled code can't share a session. Prevents accidental cross-version interference.

## Cross-substrate (for comparison with neighbor folders)

- **CRDT** ([crdts](../crdts/)) — alternative cross-peer convergence approach using merge functions instead of message ordering. Different consistency-vs-coordination trade-off. See [comparisons.md](comparisons.md).
- **Vat** ([Agoric / Endo](../agoric-endo/)) — Agoric's deterministic-VM unit. Single-machine deterministic with cross-machine consensus on input log. Closer to Croquet than CRDTs, but uses chain ordering instead of reflector ordering. See [`../agoric-endo/vat-model.md`](../agoric-endo/vat-model.md), [comparisons.md](comparisons.md).
- **state-apply** (Myrhiza) — pure WASM Component Model function `(prior_state, event) → next_state`. Croquet's Model class is the JS-runtime analog of this concept. The Model/View split maps directly to Myrhiza's `state-apply` / `interaction` component profiles.
- **Source chain** ([Holochain](../holochain/)) — Holochain's per-agent append-only signed log. Different paradigm: validating DHT instead of lockstep. See [comparisons.md](comparisons.md).
- **Lockstep** (game-engine) — StarCraft, Age of Empires, Factorio all use the same lockstep-determinism pattern as Croquet. The lineage runs through the game-engine community in parallel. See [comparisons.md](comparisons.md).

## Sources

- Multisynq docs: <https://multisynq.io/docs>
- Multisynq SDK source: <https://github.com/multisynq/multisynq-client>
- Croquet 2003 paper: *Croquet: A Collaboration System Architecture* (Smith, Kay, Raab, Reed, C5 2003)
- Wikipedia: <https://en.wikipedia.org/wiki/Croquet_Project>
- See per-file `## Sources` sections.
