**Date:** 2026-05-09
**Status:** active
**Subject:** Croquet/Multisynq vs adjacent convergence systems — CRDTs, Agoric SwingSet, game-engine lockstep, Holochain — and what the comparison says about where Croquet's lockstep-determinism pattern fits in the Myrhiza design space.

This file places Croquet/Multisynq against four neighboring approaches to "many machines, identical state." All five solve a version of the same problem; the differences are in *how the inputs get ordered*, *who is authoritative*, and *what the failure modes look like*. For Croquet's own architecture see `architecture.md` and `determinism.md`; for the programming model see `programming-model.md`; for governance see `governance.md`.

## 1. At-a-glance

| Approach | Who orders inputs | Who is authoritative | Determinism property | Scale shape | Production users | Open source |
|---|---|---|---|---|---|---|
| **Croquet/Multisynq lockstep** | Reflector (single per-session) | All clients (each replicas) | Strict global order; identical VM step on every peer | Reflector is bottleneck per session (~tens of clients) | Multisynq DePIN apps; small | SDK Apache-2.0; **server proprietary** (`governance.md`) |
| **CRDT merge** (Yjs/Automerge/Loro) | No global order; partial-order merge | None — every replica is authoritative for its own writes | Convergence under any delivery order | Unlimited peers; relays optional | Evernote, Affine, JupyterLab RTC, hundreds more (`../crdts/`) | Yes (MIT across Yjs/Automerge/Loro) |
| **Agoric SwingSet** | Cosmos chain BFT consensus | The chain (single canonical history) | Pure-fn vat replay over transcript + heap snapshot | Validator-set bound (10s-100s) | Agoric mainnet contracts (`../agoric-endo/`) | Yes (Apache-2.0) |
| **Game-engine lockstep** (StarCraft, Age of Empires, Factorio) | Designated host or peer-elected | All clients (each replicas) | Strict global order; identical sim step on every peer | Bound by slowest peer; ~2-8 typical | Tens of millions of players over decades | No (game engines proprietary; pattern documented) |
| **Holochain validating-DHT** | No global order; per-entry validation | Each peer for its own source chain; DHT for shared state | Per-entry validation function (WASM zomes) | DHT-scaled (large) | Volla phone hApps; small (`../holochain/`) | Yes (CAL-1.0) |

**Where Croquet sits.** It is *closest* to game-engine lockstep (same mechanism: total-order inputs, deterministic compute) and *next-closest* to Agoric (same property: identical state from identical inputs). It is *furthest* from CRDTs (which deliberately avoid global order) and Holochain (which has no per-session sequencer at all).

## 2. Croquet vs CRDTs

Both systems achieve cross-peer convergence. The shapes are opposites.

| Axis | Croquet/Multisynq lockstep | CRDT merge (Yjs / Automerge / Loro) |
|---|---|---|
| Input order | Single canonical order, established by reflector | No canonical order; ops carry vector-clock / Lamport metadata |
| Divergence | Forbidden — replicas never disagree | Expected — replicas diverge then merge |
| Authority surface | Reflector (one per session) | None at protocol level; relay can be any commodity server |
| Offline behavior | Client cannot make authoritative changes; reflector is single-point-of-availability for writes | Client makes local changes optimistically; merges on reconnect |
| State-space cost | Tiny — only current VM state need be retained | Tombstones, history vectors, per-op metadata accumulate (see `../crdts/comparisons.md` §4) |
| Latency floor | One round trip to reflector for *every* user input | Zero round trips for local input; eventual sync with peers |
| What if a peer is slow? | Whole session waits at the slowest peer's step (or peer is dropped) | Slow peer gets behind, catches up via state-vector diff |

**Trade-off summary.** CRDTs trade authority for availability: any peer can write at any time, divergence is acceptable, eventually-consistent. Croquet trades availability for authority: every write goes through the reflector, divergence is impossible, immediately-consistent. CRDTs scale horizontally (any peer count); Croquet scales per session up to the reflector's throughput.

**For Myrhiza.** A `state-apply` component built on Croquet-style lockstep needs a sequencer. A `state-apply` built on CRDT merge needs a merge function and is sequencer-free. The choice is not technology — it is whether the application's correctness model tolerates merge resolution. See `../crdts/comparisons.md` for the per-library tradeoffs and `../crdts/lessons.md` §"convergence-only is not enough" for the distillation.

## 3. Croquet vs Agoric SwingSet

Both systems compute identical state from identical inputs. The difference is *who orders the inputs*.

| Axis | Croquet/Multisynq | Agoric SwingSet |
|---|---|---|
| Sequencer | Reflector (per-session, operated by Croquet Labs / Multisynq Network) | Cosmos chain BFT consensus (validator set) |
| Replicas | All connected clients | All chain validators (off-chain "solo" Endo also exists) |
| Replay unit | VM tick (frame) + ordered messages | Vat transcript + heap snapshot (`../agoric-endo/persistence.md`) |
| Failure model | Reflector down → session frozen | Chain halt → all contracts paused; recovery via consensus |
| Crash recovery | Client reconnects; replays from last snapshot + reflector tail | Vat restored from snapshot + transcript (deterministic replay) |
| Cross-machine consensus | None — there's no Byzantine-tolerance; reflector is trusted | BFT consensus over input log |
| Determinism enforcement | Loose — discipline-based (forbid Date.now, Math.random in Models) | Strict — XS engine with no JIT, SES-frozen primordials, Hardened JS |
| Authority granularity | Per session (separate reflectors per app instance) | Global per chain |

**Critical similarity.** Both treat the *input log* as the canonical history of the world, and treat *application state* as a function of the log. Crash → replay. Add a peer → replay. Verify → replay. The pattern is identical; only the log's authority differs.

**Critical difference.** Agoric is **Byzantine-fault-tolerant** by construction (chain consensus); Croquet is **trusting** (clients trust the reflector to be honest about message order). If the reflector lies about order, all clients converge to the lie. If a chain validator lies about order, the BFT consensus rejects it — at the cost of a large validator set, a token, and chain-style throughput limits.

**For Myrhiza.** Agoric demonstrates that vat-replay-from-transcript is a viable production pattern for deterministic compute. Croquet demonstrates that reflector-ordered-input is a viable pattern for real-time multiuser. Myrhiza could in principle adopt either:
- **Agoric-shaped:** canonical event log somewhere (chain, leader, signed-by-quorum), all peers replay. State-apply is `(prior state, event) → next state`. Latency = log-confirmation latency.
- **Croquet-shaped:** real-time sequencer per session, all peers run lockstep. State-apply is `(prior state, message) → next state`. Latency = round-trip to sequencer.

Both fit Myrhiza's "pure function of (prior state, event) plus deterministic helpers" rule. The difference is the input-ordering substrate. See `../agoric-endo/comparisons.md` for Agoric's own comparison set, including its position vs CosmWasm and EVM.

## 4. Croquet vs game-engine lockstep (StarCraft, Age of Empires, Factorio)

This is the closest technical sibling. The Croquet 2003 paper and Mark Terrano / Paul Bettner's *1500 Archers on a 28.8: Network Programming in Age of Empires and Beyond* (2001) describe **the same mechanism**: each client runs an identical deterministic simulation, and only player commands cross the network.

| Axis | Croquet/Multisynq | RTS lockstep (AoE, StarCraft, Factorio) |
|---|---|---|
| Mechanism | Identical VM tick + ordered messages | Identical sim tick + ordered commands |
| Tick rate | App-defined (often display-rate) | Game-defined turn timer (AoE: ~5-10 turns/sec, with tunable speed) |
| Ordering authority | Reflector (Croquet Labs / Multisynq) | Designated host or peer-elected (typically host's machine) |
| Sync barrier | All peers wait for reflector message before advancing tick | All peers wait for slowest peer's commands at turn boundary |
| Peer count | Tens (limited by reflector fan-out) | 2-8 typical; 16 max in some titles |
| Determinism enforcement | JS-discipline: forbid `Date.now()`, `Math.random()` (Croquet provides `Math.random` replacement); no DOM access in Models | Same compiler, same machine code on every client; care with float ops |
| Programming surface | Model/View framework (explicit split) | Game engine sim/render split (implicit in engine architecture) |
| Floating-point hazard | JS Number is IEEE-754 binary64 across all browsers — but transcendentals (`sin`, `cos`) can drift between V8/SpiderMonkey/JSC | Famously fixed-point in many RTS titles to dodge this; or strict compiler/instruction-set lock |
| Cheat resistance | Cryptographic per-session passwords; reflector cannot be silently subverted by one client | Lockstep is *cheater-friendly* (every client sees full state); commercial games add anti-cheat layers separately |

**Shared lessons.** The lockstep pattern has been production-validated for ~30 years across millions of players. Its scaling ceiling is real (the Age of Empires team explicitly designed for "300 commands per minute per player" on 28.8 modems). Determinism bugs are nasty (one machine drifts from the rest mid-game) and historically required obsessive engineering — Factorio in particular has a [public determinism testbench](https://factorio.com/blog/post/fff-176) that desyncs ship as bugs.

**What Croquet adds beyond game lockstep.**
1. A *framework* abstraction (Model/View) so app developers don't reimplement the lockstep loop themselves.
2. *Persistence* — the VM state can snapshot and a new joiner replays from snapshot + tail rather than from genesis (game lockstep typically replays the whole match from turn zero).
3. *Web target* — runs in browsers, no install. The game-lockstep tradition was native binaries.

**What game lockstep handles better.**
1. *Cheat tolerance is irrelevant in single-binary scenarios.* Croquet apps in browsers face JS-VM-introspection threats game binaries don't.
2. *Floating-point determinism* is a solved problem in the AAA RTS lineage (fixed-point math, instruction lock-in). Croquet's "use our Math.random" approach is a thinner shim.
3. *Backpressure when peers fall behind* is well-understood in RTS (slow-peer drops, host migration). Croquet's reflector centralizes this but its handling is less battle-tested at scale.

**For Myrhiza.** If Myrhiza adopts lockstep for any `state-apply` profile, the RTS lineage is the more mature playbook than Croquet specifically. Read *1500 Archers* and the Factorio determinism posts before designing the lockstep variant. Croquet is the same idea wrapped in a JS framework; the underlying pattern is older and better-documented in game-dev literature.

## 5. Croquet vs Holochain

Different paradigm. Holochain is a **validating distributed hash table**: each peer maintains a personal source chain (per-author signed event log) and the network is a DHT where entries are validated by other peers running WASM zomes against published validation rules. There is no per-session sequencer; there is no shared VM; there is no global event order. See `../holochain/architecture.md` for the full picture.

| Axis | Croquet/Multisynq | Holochain |
|---|---|---|
| State authority | Single shared VM, one per session, deterministic | Personal source chain per author + validating DHT |
| Convergence mechanism | Lockstep replay | Per-entry validation + eventual DHT consistency |
| Group identity | Per session (reflector-bound) | Per network (DNA-hash-bound — see `../holochain/networking.md`) |
| Real-time guarantees | Yes — every client sees same state every tick | No — DHT propagation is eventual |
| Offline writes | No (reflector required) | Yes (write to source chain locally; publish on reconnect) |
| WASM use | None in Croquet legacy; Multisynq SDK is JS | Validation zomes are WASM (`../holochain/architecture.md`) |
| Scaling shape | Per-session reflector | Network-wide DHT (large group friendly) |

**Why include Holochain in this comparison.** Both are post-2010 attempts at "shared-state-without-cloud-server," both have small commercial ecosystems, both face the same explanation-burden for new developers ("how is this not just a server?"). The implementations could not be more different. Holochain's design accepts that you cannot have real-time state across a permissionless DHT and instead optimizes for offline / asynchronous workflows. Croquet does the inverse: optimize for real-time, accept the per-session reflector cost.

**For Myrhiza.** Both data points matter. Myrhiza's `state-apply` profile is supposed to be a pure function of (prior state, event) — that constraint admits Croquet's lockstep, Agoric's transcript replay, *and* Holochain's per-entry validation as concrete realizations. The choice between them is a choice about the *coordination shape*, not the determinism story.

## 6. Croquet vs Myrhiza

Myrhiza's state-apply profile is, abstractly, *some* function of (prior state, event) plus a deterministic helper set. The pattern that satisfies this could be:

| Pattern | Sequencer? | Authority shape | Reference |
|---|---|---|---|
| Lockstep (Croquet pattern) | Yes — per-session | Sequencer is trusted | This folder, esp. `architecture.md` |
| Event-log replay (Agoric pattern) | Yes — global log (chain or leader) | Log authority (BFT consensus, signed quorum, etc.) | `../agoric-endo/persistence.md` |
| CRDT merge (Automerge/Yjs pattern) | No | Per-replica | `../crdts/comparisons.md` |
| Validating-DHT (Holochain pattern) | No (per-entry validation) | Per-author + DHT | `../holochain/architecture.md` |

The Myrhiza spec (under `docs/specs/`) has not committed to one of these as of May 2026. This prior-art folder exists because the choice should be informed by what each pattern produces *in practice* — including the failure modes, the cost of the coordination shape, and the operational dependencies.

## 7. When is lockstep the right choice for Myrhiza?

**Lockstep wins when:**
- Group size is small (≤ ~20 active participants per session).
- All-peers-active is the assumed case (presence is essential to the app: live game, shared 3D space, real-time co-edit with cursors).
- Latency must be uniform across peers (no peer can be ahead of others).
- The application is naturally session-scoped (a meeting, a match, a doc-edit-with-presence) rather than long-lived global state.
- A trusted (or trustless-with-additional-machinery) sequencer is acceptable.

**Lockstep loses when:**
- Group size is large (reflector throughput is bottleneck; one slow peer blocks all).
- Peers are intermittent (long absences are hard to replay; snapshots help but don't eliminate it).
- The application is asynchronous-first (email-shaped, document-shaped, eventually-consistent-shaped).
- The deployment cannot afford a centralized sequencer per session (Myrhiza's P2P thesis specifically resists this).

For Myrhiza specifically, the **P2P-without-central-operator constraint** points away from Croquet's lockstep-with-reflector toward either CRDT merge (no sequencer) or peer-elected lockstep (sequencer rotates among peers; closer to game-engine RTS than Croquet). A direct copy of Multisynq's reflector-as-service model contradicts Myrhiza's design intent — the *pattern* is reusable, the *operator topology* is not.

## 8. Sources

- [Croquet: A Collaboration System Architecture (Smith/Kay/Raab/Reed, C5 2003)](https://www.semanticscholar.org/paper/Croquet-a-collaboration-system-architecture-Smith-Kay/8d3efe9a144a574002bd1f452c3adcf46fa915e2)
- [1500 Archers on a 28.8: Network Programming in Age of Empires and Beyond — Terrano & Bettner](https://www.gamedeveloper.com/programming/1500-archers-on-a-28-8-network-programming-in-age-of-empires-and-beyond)
- [Lockstep protocol — Wikipedia](https://en.wikipedia.org/wiki/Lockstep_protocol)
- [Factorio Friday Facts #176 — determinism](https://factorio.com/blog/post/fff-176)
- [Multisynq — The Real-Time Application Layer of the Internet](https://multisynq.io/)
- [@multisynq/client — npm](https://www.npmjs.com/package/@multisynq/client)
- [github.com/multisynq/multisynq-client](https://github.com/multisynq/multisynq-client)
- `../crdts/comparisons.md` — Yjs / Automerge / Loro head-to-head
- `../crdts/crdt-theory.md` — CRDT taxonomy and convergence proofs
- `../agoric-endo/comparisons.md` — Agoric vs Spritely / Cap'n Proto / EVM / CosmWasm / Iroh / SES
- `../agoric-endo/persistence.md` — vat transcript + heap snapshot replay model
- `../holochain/architecture.md` — validating-DHT model
- `../holochain/comparisons.md` — Holochain vs neighboring systems
- `architecture.md`, `determinism.md`, `programming-model.md`, `multisynq-platform.md`, `governance.md`, `lessons.md` — sibling Croquet refs
