**Date:** 2026-05-09
**Status:** active
**Subject:** Lessons from Croquet/Multisynq prior art for Myrhiza state-apply convergence

# Lessons for Myrhiza

The decision-relevant synthesis. Other files are evidence; this file is what we take away.

Croquet's lockstep paradigm is **NOT the right `state-apply` model for Myrhiza** (P2P-native commitment + scale + offline-tolerance all argue against). But it IS the canonical reference for deterministic-VM **mechanics**: pseudo-time, seeded RNG, snapshot voting, transcendental hardening. The borrow is at the mechanics level, not the architecture level.

Format: validates / avoid / borrow / open questions.

## Validates

Croquet/Multisynq prior art **confirms** these Myrhiza design bets:

- **Cross-replica deterministic compute is achievable in production.** Croquet has run lockstep deterministic JS at non-trivial scale (gaming, collaborative apps, the multiblaster demo) for ~20 years. Myrhiza's `state-apply` purity requirement is not unrealistic — the harder version (deterministic JS in browsers) has been shipped.
- **The Model/View split is the right component-profile shape.** Croquet's deterministic Model + non-deterministic View maps exactly onto Myrhiza's `state-apply` + `interaction` profiles. This is a third independent confirmation (Agoric vats + Holochain zomes + Croquet Models all distinguish deterministic-replicated from non-deterministic-local).
- **Virtual time is mandatory for deterministic VMs.** All three runtimes (Croquet, Agoric, Holochain) replace wall-clock with a virtual clock. Myrhiza `state-apply` must do the same: deterministic-helper-set includes virtual-time.
- **Seeded RNG is mandatory.** All three runtimes seed RNG from the input stream. Myrhiza `state-apply` must do the same: deterministic-helper-set includes seeded RNG keyed off the event being applied.
- **Snapshot equality is the right correctness check.** Croquet's TUTTI vote (periodic cross-replica hash comparison) is how you actually catch determinism drift in practice. The abstract proof of "pure function = same output" is not enough; you need a runtime check.
- **Floating-point determinism in JS is hard but tractable.** Croquet's `@stdlib/math` patch with iOS-Safari workarounds proves it can be done. Wasmtime gives Myrhiza stronger FP determinism by default than JS, but the lesson is: don't assume; verify.

## Avoid

Croquet/Multisynq prior art shows where the **easy mistakes** are:

- **Don't adopt the reflector-dependent architecture.** Multisynq's Synchronizer is a closed-source binary (`cdrakep/synqchronizer` Docker image) requiring a Synq Key issued by Multisynq the company. Even hypothetically open-source, a reflector is a single coordination point that conflicts with P2P-native architecture. Myrhiza must not have this dependency.
- **Don't assume "deterministic JS" is the goal.** Croquet's `@stdlib/math` patch is a workaround for JS's non-determinism, not a feature. Myrhiza targets WASM Component Model — a substrate with stronger FP determinism. Don't import the JS-determinism-patches mindset; verify Wasmtime's behavior directly.
- **Don't ignore drift detection because the math says replicas converge.** Croquet detects drift via TUTTI but **does NOT auto-recover** — the session limps along with divergent state. The lesson is that drift detection is necessary AND that Myrhiza must design recovery (re-sync from snapshot, expel-and-rejoin, halt-and-investigate) — silent divergence is worse than crash.
- **Don't assume "all peers active" topology.** Lockstep requires all peers to be live or absent (with snapshot catchup). Myrhiza's "peers may be intermittently connected" / "peers may be offline for hours/days" model is fundamentally incompatible with strict lockstep.
- **Don't assume scale beyond ~hundreds of peers.** Game-engine lockstep typically caps at ~8-16 players (StarCraft etc.); Croquet/Multisynq pushes higher but still hits ceilings at hundreds. If Myrhiza apps need thousand-peer rooms, lockstep is the wrong shape.
- **Don't bet on Multisynq the company.** Vanessa Freudenberg (longtime engineer) died Oct 2025; team is small (single-digit-to-low-double-digit per Crunchbase signals); $2.2M seed (Apr 2024) + $350K token sale (Feb 2025) is undercapitalized. The pattern is worth studying; the company is not a depend-on dependency.
- **Don't conflate `@multisynq/client` (Apache-2.0) with the full stack.** The SDK is open-source; the Synchronizer that messages flow through is closed-source. Multisynq is "open SDK + closed network."

## Borrow

Specific patterns Myrhiza `state-apply` design should **steal**:

- **Pseudo-time as the only time source for deterministic compute.** Myrhiza's deterministic-helper-set must include virtual-time, derived from the event being applied. `state-apply` components MUST NOT have access to wall-clock.
- **Seeded RNG keyed to event ID.** Myrhiza's deterministic-helper-set must include seeded RNG. All `state-apply` invocations on the same `(prior, event)` produce the same random sequence.
- **Snapshot-equality voting (TUTTI pattern).** Myrhiza should periodically (every N events, or on commit boundaries) compute a deterministic hash of state and verify cross-replica match. Detect drift; design recovery; don't silently let divergent replicas continue.
- **Canonical-stringify hash.** Croquet uses `fast-json-stable-stringify` for cross-replica hash computation. Myrhiza state hashes must use a canonical serialization (the equivalent for whatever Myrhiza state shape is).
- **Code-hash as session-scope.** Croquet sessions are scoped by `(apiKey, appId, name, password, code-hash)`. Two peers with different code can't share a session. Myrhiza app sessions should similarly be code-hash-scoped — peers with different `state-apply` component bytes can't accidentally interleave.
- **"Forbidden APIs in Models" discipline.** Croquet's Model class explicitly disallows wall-clock reads, DOM access, network, file I/O. Myrhiza `state-apply` components must have the same discipline — enforced at the WIT contract level (kernel imports declare what's available; nothing else can be called).
- **Heartbeat ticks for time advance.** Even with no user input, Croquet emits `TICK` messages so simulation can advance. Myrhiza needs to think about this: does `state-apply` ever get invoked just because virtual time advances? If yes, kernel emits time-tick events.

## Open questions

Myrhiza spec authors should address with this corpus loaded:

- **Is `state-apply` lockstep-shaped, event-log-shaped, CRDT-merge-shaped, or validating-DHT-shaped?** This corpus says lockstep is the wrong primary shape for Myrhiza but the deterministic-VM mechanics translate. The other three folders ([`../agoric-endo/`](../agoric-endo/), [`../crdts/`](../crdts/), [`../holochain/`](../holochain/)) cover the alternatives.
- **What's Myrhiza's "Synchronizer" analog?** If Myrhiza has any concept of message ordering across peers, who orders? Options: per-app-elected sequencer (lockstep-style), per-event content-hash (CRDT-style), per-app blockchain (Agoric-style), validating-DHT (Holochain-style).
- **Snapshot semantics.** Does Myrhiza's `state-apply` produce snapshots? At what cadence? How are they distributed to peers needing catchup? This is borrowed from Croquet's pattern but the answer is Myrhiza's design problem.
- **Cross-replica drift detection.** What's the Myrhiza analog of TUTTI? Periodic hash comparison? Per-event commit verification? Design choice with deep implications.

## Recommendation matrix for Myrhiza

This is the four-pattern triangulation across all of [`../agoric-endo/`](../agoric-endo/) + [`../crdts/`](../crdts/) + [`../holochain/`](../holochain/) + this folder:

| If Myrhiza's `state-apply` semantics need… | Choose | Reason | Risk |
|---|---|---|---|
| Strongest cross-replica consistency, small group, low latency, all-peers-active | **Lockstep** (Croquet pattern) | Identical state at every epoch; trivial to verify | Reflector dependency; offline-intolerant; scale ceiling |
| Strong consistency with chain ordering, single-author input log per vat | **Event-log replay** (Agoric pattern) | Verifiable; chain-friendly; snapshot+log replay is well-understood | Replay cost; chain dependency |
| Maximum offline tolerance, no coordination required | **CRDT merge** (Automerge/Yjs/Loro pattern) | No coordinator; scales freely; offline-tolerant | Convergence ≠ semantic correctness; authority/validation orthogonal |
| P2P-native, per-entry validation rather than ordering | **Validating DHT** (Holochain pattern) | No coordinator; deterministic validation in WASM zomes | Newer; smaller production track; CRDT-shaped state issues |

For Myrhiza specifically, given the P2P-native + intermittent-peer-tolerant + scale-ambiguous design space:

- **Most likely fit:** validating-DHT-shaped or CRDT-shaped. Lockstep is wrong (reflector); event-log-replay is awkward without a chain.
- **Most-borrowable-mechanics from this folder:** virtual time, seeded RNG, snapshot-equality voting, canonical-stringify hashing, code-hash session scoping, forbidden-APIs-in-Models discipline.

## Recommended posture for the runtime spec

A defensible default given the corpus:

1. **Don't pick lockstep as Myrhiza's primary `state-apply` shape.** Reflector dependency conflicts with P2P-native commitment.
2. **DO borrow the deterministic-VM mechanics.** Pseudo-time, seeded RNG, snapshot voting, canonical hashing, forbidden-API discipline — all directly applicable regardless of which paradigm Myrhiza picks.
3. **Design drift detection from day one.** Croquet's TUTTI is the pattern. Myrhiza needs a way to verify cross-replica state agreement, not just trust the abstract proof.
4. **Decide on the snapshot-vs-replay tradeoff.** Croquet snapshots; Agoric replays from log + checkpoint; CRDTs use neither (state is the merge result). Myrhiza needs to pick.
5. **Treat the "Synchronizer is closed-source binary requiring permissioned key" pattern as an anti-example.** Whatever Myrhiza's coordination primitive is, it must be open-source, self-hostable, and unkeyed.

## Sources

This file synthesizes from sibling files and the cross-paradigm folders. Primary sources cited per sibling:

- [architecture.md](architecture.md), [determinism.md](determinism.md) — Croquet/Multisynq runtime mechanics
- [programming-model.md](programming-model.md), [multisynq-platform.md](multisynq-platform.md) — developer + deployment surface
- [governance.md](governance.md), [comparisons.md](comparisons.md) — three-era lineage + cross-paradigm positioning
- [open-problems.md](open-problems.md), [critiques.md](critiques.md) — gaps + third-party voices
- Cross-paradigm folders: [`../agoric-endo/`](../agoric-endo/), [`../crdts/`](../crdts/), [`../holochain/`](../holochain/)
- 2003 paper: Smith, Kay, Raab, Reed — *Croquet: A Collaboration System Architecture*, C5 2003
