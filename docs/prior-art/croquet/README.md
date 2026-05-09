**Date:** 2026-05-09
**Status:** active
**Subject:** Croquet / Multisynq — lockstep deterministic-VM collaboration paradigm; the canonical reference for "all peers run the same compute on identically-ordered messages"

# Croquet / Multisynq prior art

Reference folder for the Croquet lineage — academic Croquet Project (2003) → Croquet Corporation/Labs (2018-2024) → Multisynq Network (2024-present rebrand). 11 files, ~1,350 lines.

This is the *fills the determinism & lockstep gap* folder. Croquet is the canonical reference for **lockstep determinism as a cross-peer convergence paradigm** — different from event-log replay (Agoric SwingSet), different from CRDT merge (Automerge/Yjs/Loro), different from validating DHT (Holochain). The corpus surfaces the mechanics so future Myrhiza spec authors choosing a `state-apply` semantics have lockstep on the table as one of four viable patterns.

## Key facts at a glance

| Field | Value |
|---|---|
| Original paper | Smith, Kay, Raab, Reed — *Croquet: A Collaboration System Architecture* — **C5 2003** (NOT OOPSLA — common citation error). Built on Squeak Smalltalk. |
| Modern stack | JavaScript / WebAssembly. SDK `@multisynq/client` 1.1.0 (npm, **Apache-2.0**). |
| Repo | github.com/multisynq/multisynq-client (241 stars, Apache-2.0) |
| Legacy SDK | `@croquet/croquet` 2.0.4 — earlier versions proprietary; **the 2.0.4 republish on 2025-06-09 carries `Apache-2.0`**, aligning the legacy SDK with the Multisynq open-source rebrand |
| Reflector / Synchronizer | Multisynq DePIN reflector network. **Closed-source binary** (`cdrakep/synqchronizer` Docker image) requiring **Synq Key** issued by Multisynq |
| Croquet network deprecated | 2025-07-30 (legacy hosted reflector); Croquet Labs is "primary provider to Multisynq Network" |
| Tick rate | 20 Hz default (range 1/30 Hz – 60 Hz) |
| Snapshot cadence | every 5 s of CPU time, or every 5 minutes of pseudo-time on DePIN reflectors |
| Funding (Multisynq) | $2.2M seed lead Manifold (Apr 2024) + $350K token sale (Feb 2025) |
| Funding (Croquet Corp) | Founded May 2018; **$2.7M seed Feb 2020** ($2.0M from SIP Global Partners) |
| Founders | David A. Smith (active), Alan Kay (academic co-author 2003, advisor); **Croquet Corporation co-founders include Vanessa Freudenberg (Chief Architect, JS rewrite lead), Aran Lunzer, Yoshiki Ohshima, Brian Upton** |
| Bus-factor signal | **Vanessa Freudenberg (Chief Architect / JS-rewrite lead) died 2025-10-22** — small team, structurally exposed |
| Examples | github.com/multisynq has multiblaster (asteroids), labyrinth (multiplayer strategy/shooter), physics-fountain, vibecoded-submarine, multitug, multicar |

## How to use

Read in this order:

1. **[architecture.md](architecture.md)** — Model/View/Synchronizer split, reflector-as-sequencer, wire protocol (`SYNC`/`RECV`/`TICK`/`TUTTI`/`SNAP` verified from `controller.js`), session lifecycle, snapshot voting.
2. **[determinism.md](determinism.md)** — *the load-bearing file*. Pseudo-time, seeded `seedrandom` PRNG (verified at `vm.js:481`), patched `Math` dispatch through `@stdlib/math` with documented iOS-Safari `pow()` workaround, `fast-json-stable-stringify`-based TUTTI snapshot-equality vote, `Session diverged (#${previous})!` detection-without-recovery posture.
3. **[programming-model.md](programming-model.md)** — Model/View classes, `Model.create(...)`, publish/subscribe API, `this.future(ms).callback()` virtual-time scheduling, the Prime Directive serialization rules, walked-through `multiblaster` example, what's forbidden in Models.
4. **[multisynq-platform.md](multisynq-platform.md)** — Multisynq Network as a service: DePIN reflector model, free API keys via multisynq.io/coder, Synq Key requirement for operating a Synchronizer, three SDK distribution paths.
5. **[governance.md](governance.md)** — three-era lineage. Era 1 (academic 2001-~2010, MIT-licensed Squeak SDK 1.0 published 2009-12-24), Era 2 (Croquet Corporation founded May 2018, $2.7M seed), Era 3 (Multisynq rebrand 2024). License posture transition from proprietary to Apache-2.0 (mostly).
6. **[comparisons.md](comparisons.md)** — vs CRDTs ([../crdts/](../crdts/)), vs Agoric SwingSet ([../agoric-endo/](../agoric-endo/)), vs game-engine lockstep (StarCraft/AoE/Factorio/Glenn Fiedler's lockstep articles), vs Holochain ([../holochain/](../holochain/)).
7. **[critiques.md](critiques.md)** — third-party voices. HN/Register thread quotes (doctorpangloss, Animats/John Nagle, avallach, Rodeoclash, tamimio); closed-reflector concerns (Tom7, karlkarl); Krestianstvo's "it remains a server"; Fiedler/Dawson on lockstep+FP determinism.
8. **[open-problems.md](open-problems.md)** — 11 structural gaps: reflector SPOF, latency floor, FP determinism in JS transcendentals (mitigated via `@stdlib/math` but bounded), snapshot bloat without GC, late-joiner cost, no schema migration, Byzantine peers, ~hundreds-scale ceiling, no offline-first, no WASM Component Model artifact.
9. **[lessons.md](lessons.md)** — *the decision file*. Validates / avoid / borrow + the four-pattern recommendation matrix.
10. **[glossary.md](glossary.md)** — Croquet/Multisynq-specific terms.

If you only have time for two files: read **lessons.md** + **determinism.md**.

## Why this folder exists

Myrhiza's `state-apply` purity requirement has four viable cross-peer convergence paradigms in production prior art:

| Paradigm | Reference | Coordination shape | Strength | Weakness |
|---|---|---|---|---|
| **Lockstep deterministic VM** | **This folder (Croquet/Multisynq)** | Reflector-ordered global message stream | Strongest consistency; identical state across replicas at every epoch | Reflector dependency; scale ceiling at ~hundreds; offline-tolerance poor |
| **Event-log replay** | [`../agoric-endo/`](../agoric-endo/) | Single canonical input log | Strong consistency; verifiable replay; chain-friendly | Chain dependency; replay cost on cold start |
| **CRDT merge** | [`../crdts/`](../crdts/) | No global ordering required | No coordination; offline-tolerant; scale-free | Convergence ≠ semantic correctness; authority/validation orthogonal |
| **Validating DHT** | [`../holochain/`](../holochain/) | Per-entry deterministic validation in WASM zomes | P2P-native; no coordinator | Newer; smaller production track |

This folder studies **paradigm 1**. The corpus completes the determinism survey.

## What Croquet teaches that no other folder in this corpus does

- **Pseudo-time as the only time source for deterministic compute.** All time-reads in Models go through a virtual clock advanced by reflector messages, not wall-clock. The pattern generalizes: any deterministic VM needs an explicit virtual-time abstraction.
- **Seeded RNG bound to the message stream.** `this.random()` uses `seedrandom` keyed off snapshot ID, so all replicas produce the same random values. Generalizes to: deterministic-helper-set in Myrhiza must include seeded RNG.
- **Snapshot-equality voting (TUTTI) as drift detection.** Periodic cross-replica hash comparison via `fast-json-stable-stringify` proves convergence — or detects divergence. Generalizes to: any deterministic-VM Myrhiza ships should include a drift-detection mechanism, not just trust the abstract proof.
- **`@stdlib/math` for floating-point determinism.** Croquet replaces `Math.sin/cos/pow` etc. with deterministic implementations from `@stdlib/math`, including a documented iOS-Safari `Math.pow` workaround. Wasmtime gives stronger FP determinism by default than JS, but Croquet's experience is the reference for "what goes wrong if you don't think about this."
- **DePIN reflector economics.** Multisynq's experiment in token-incentivized reflector-operator participation is a real-world data point for "decentralized message-ordering infrastructure" — useful negative reference if Myrhiza prefers fully P2P (no central reflector).

## Honest assessment for Myrhiza

Lockstep is **not the right paradigm for Myrhiza** as the headline `state-apply` semantics, for three reasons:

1. **Reflector dependency conflicts with P2P-native commitment.** Multisynq requires a Synq Key from Multisynq to operate a reflector — gatekept infrastructure. Even a hypothetical fully-open-source reflector would be a coordination point Myrhiza's design avoids.
2. **Scale ceiling.** Lockstep collapses at thousands-of-peers; Myrhiza apps may exceed this.
3. **Offline tolerance.** Lockstep requires all peers to be live or absent (with snapshot catchup). Myrhiza's "peers may be intermittently connected" model is a poor fit.

But lockstep **is** the right reference for deterministic-VM mechanics: pseudo-time, seeded RNG, snapshot voting, `@stdlib/math`-style transcendental hardening. If Myrhiza's `state-apply` ABI ends up looking like "deterministic WASM compute with virtual-time + seeded RNG + snapshot-equality verification," Croquet is the canonical prior art.

## Framing disclosure

These docs are written from the **Myrhiza-as-deterministic-state-apply-runtime** stance — the "Implications for Myrhiza" sub-sections frame Croquet's lockstep choices through that lens. Croquet's design point is *not* what Myrhiza is doing (Myrhiza is P2P, lockstep needs a reflector), but the determinism mechanics translate cleanly. A reader auditing whether *deterministic state-apply itself* is the right primitive should weigh the [open-problems.md](open-problems.md) and [critiques.md](critiques.md) carefully — the Multisynq team's experience is the strongest empirical evidence for what works and what doesn't in deterministic JS runtimes.

The corpus also reads through the comparison lens with the other three convergence-paradigm folders ([`../agoric-endo/`](../agoric-endo/), [`../crdts/`](../crdts/), [`../holochain/`](../holochain/)) — all are alternatives to lockstep. Decisions about Myrhiza's `state-apply` semantics should triangulate across all four.

## Sources

Per-file `## Sources` sections list URLs cited in that file. Aggregate top-level sources:

- 2003 paper: <http://www.vpri.org/pdf/tr2003001_croq_collab.pdf> (cert-mismatch when scripted-fetched; archive copy via vpri.org)
- Multisynq SDK: <https://github.com/multisynq/multisynq-client>
- Multisynq website: <https://multisynq.io>
- Croquet SDK (legacy): <https://github.com/croquet/croquet>, <https://www.npmjs.com/package/@croquet/croquet>
- React bindings: <https://github.com/multisynq/react-together>
- Wikipedia: <https://en.wikipedia.org/wiki/Croquet_Project>
- David A. Smith on X: <https://x.com/gocroquet>
