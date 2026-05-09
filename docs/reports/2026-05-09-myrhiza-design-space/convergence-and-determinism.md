**Date:** 2026-05-09
**Status:** active (preparation phase for master-spec brainstorming)
**Subject:** Myrhiza decision space — convergence + state-apply + determinism

## Decision domains in this cluster

1. **Convergence paradigm** — event-log replay vs lockstep vs CRDT merge vs validating DHT vs hybrid.
2. **State-apply discipline** — purity model, helper-set scope, fuel/instruction limits, denied imports, encoding format for `state-digest`, snapshot mechanism, snapshot portability across version upgrades.
3. **Determinism enforcement** — float discipline, allocator/HashMap iteration, instruction count vs wall time, deterministic helper set definition, cross-peer hash gossip.

The CLAUDE.md "Component Profiles" table already locks: `state-apply` is strict-pure, the four-profile split, "pre-check is the same WASM function." What is *not* locked is **how convergence is shaped at the kernel layer above `state-apply`** (paradigm, sync protocol, ordering primitive). That is the high-leverage axis below.

## Domain 1: Convergence paradigm

**What's at stake**: This decides the kernel's job. Does the kernel order events into a total/partial sequence and feed them through `apply` (replay)? Does it hold all peers to a strict ordered tick (lockstep)? Does it merge per-author writes via CRDT algebra and skip ordering entirely? Does it validate per-op at peers holding the relevant shard (DHT)? The choice determines what `state-apply`'s ABI looks like, what the sync wire protocol is, what failure modes exist, and what offline tolerance is achievable.

### Option A: Event-log replay (Agoric-Endo / Willow today)

- **Mechanism**: Append-only log of signed events; canonical ordering (per-author chain + cross-author causal `deps` topo-sorted with deterministic tiebreak); peers replay log through pure `apply(prior, event) -> next` to materialize state. Snapshots are local cache, log is the consensus surface.
  - Agoric variant: single-vat-per-log + transcript-of-`(delivery, syscalls, result)`; replay verifies syscall equivalence (`agoric-endo/determinism.md` §"Mediated I/O").
  - Willow variant: per-author Merkle DAG (multi-author causal), `EventDag::insert` enforces `seq == latest_seq + 1` and `prev == current_head` *before* signature verification; topo-sort is Kahn's + `BTreeSet<EventHash>` lex tiebreak (`willow/state-machine.md` §"Shipped today: the DAG"; `willow/determinism.md` §"Today: how convergence is achieved").
- **Pros for Myrhiza**:
  - Strongest production track record (Agoric mainnet 3.5y, ~23 chain upgrades; Willow 2y) (`agoric-endo/lessons.md` "Validates"; `willow/lessons.md` §"Per-author Merkle DAG").
  - Generic over payload — kernel doesn't know app semantics (`willow/state-machine.md` §"Aspirational PR #636: generalization").
  - Snapshot-as-cache stance survives engine-internal divergence (gcc-9 incident in `agoric-endo/determinism.md` §"Real incidents").
  - Handles offline / intermittent peers gracefully (the buffered + soft-dep-gap pattern in `willow/state-machine.md` §"Out-of-order delivery").
- **Cons for Myrhiza**:
  - Replay cost grows with log length; needs snapshot policy + retention (`agoric-endo/persistence.md` §"Snapshot frequency").
  - Cross-version migration is painful (Agoric `baggage`, vat-upgrade rollback in `agoric-endo/persistence.md` §"Vat upgrade").
  - Per-author chains do not natively address "concurrent semantically-conflicting writes" — apps still need merge logic (the bank-account problem from `crdts/open-problems.md` §3).
  - Multi-author causal DAG is not the same shape as Agoric's single-author transcript; Willow's design is the relevant reference, not SwingSet's, when peers may concurrently author.
- **Sources**: `prior-art/agoric-endo/determinism.md` §§"What 'deterministic' has to mean here", "Mediated I/O", "Real incidents"; `prior-art/agoric-endo/persistence.md` §§"Three-layer persistence", "Vat upgrade"; `prior-art/willow/state-machine.md` §§"Shipped today", "What Myrhiza inherits"; `prior-art/willow/determinism.md` §§"Today: how convergence", "Aspirational PR #636".

### Option B: Lockstep deterministic VM (Croquet/Multisynq)

- **Mechanism**: A reflector assigns every message a `(time, seq)` total order; every replica receives the same envelope sequence + heartbeat ticks; every VM is a pure function of `(snapshot, ordered message stream)`. Hash-vote (TUTTI) periodically detects drift (`croquet/determinism.md` §"The lockstep paradigm", §"What snapshots prove").
- **Pros for Myrhiza**:
  - Strongest cross-replica consistency at every tick (no eventual-consistency window).
  - Pseudo-time + seeded RNG primitives are mature (`croquet/determinism.md` §§"Simulated pseudo-time", "Seeded RNG").
  - Snapshot-equality voting is a borrowable mechanic regardless of paradigm choice.
- **Cons for Myrhiza**:
  - **Reflector dependency conflicts with P2P-native framing.** Multisynq's Synchronizer is a closed-source binary requiring a Synq Key (`croquet/lessons.md` §"Avoid"). Even if open-sourced, a single coordination point breaks peer symmetry.
  - **Offline-intolerant.** Lockstep requires all peers live or absent-with-snapshot-catchup; doesn't fit Myrhiza's "peers offline for hours/days" assumption.
  - **Scale ceiling.** Game-engine lockstep caps at ~hundreds of peers (`croquet/lessons.md` §"Avoid").
  - **No automatic divergence recovery.** TUTTI detects, logs, debugs — does not heal (`croquet/determinism.md` §"What snapshots prove").
- **Sources**: `prior-art/croquet/determinism.md` §§"The lockstep paradigm", "What snapshots prove", "What Croquet does not solve"; `prior-art/croquet/lessons.md` §§"Avoid", "Recommendation matrix"; `prior-art/croquet/programming-model.md` §"The two-class model".
- **Closest precedent in this corpus**: Croquet itself; **runner-up**: Agoric's "in consensus mode the chain itself acts as a serializer" pattern, which is lockstep-shaped at chain-block granularity rather than per-message.

### Option C: CRDT merge (Automerge/Yjs/Loro)

- **Mechanism**: Per-replica state mutated by ops; merge is commutative/associative/idempotent so peers reach the same logical state regardless of op-receipt order. No coordinator, no global ordering.
- **Pros for Myrhiza**:
  - Maximum offline tolerance — no coordinator required (`crdts/lessons.md` §"Validates").
  - Three production libraries demonstrate convergence at scale.
  - Per-container-type algorithm choice (Loro) lets apps pick text/list/tree merge semantics.
- **Cons for Myrhiza**:
  - **No authority enforcement.** Merge converges regardless of *who* made the change (`crdts/open-problems.md` §2). Myrhiza's `state-apply` *must* gate on author permission — CRDT merge alone cannot do that.
  - **No invariant enforcement.** Bank-account-must-stay-positive cannot be expressed in merge (`crdts/open-problems.md` §3).
  - **Logical convergence ≠ byte-identical state.** None of the three libraries guarantee identical internal representation; cross-peer state hashing requires canonicalization (`crdts/comparisons.md` §9).
  - **No schema evolution.** `crdts/open-problems.md` §1 — Cambria is research, not production.
  - **No tombstone GC without coordination.** Doc grows monotonically (`crdts/open-problems.md` §4).
  - **No Component Model packaging.** All three ship raw modules + bindgen (`crdts/comparisons.md` §6).
  - Library choice is sticky: Yjs ↔ Automerge ↔ Loro are wire-incompatible (`crdts/open-problems.md` §9).
- **Sources**: `prior-art/crdts/lessons.md` §§"Avoid", "Recommendation matrix"; `prior-art/crdts/comparisons.md` §§"Algorithm comparison", "Determinism reality", "WASM compilability"; `prior-art/crdts/open-problems.md` §§1–11.
- **Closest precedent**: Automerge (Rust, Ink & Switch, healthiest stewardship); **runner-up**: Loro (Fugue text + Move-tree, Rust-native, but bus-factor 1).

### Option D: Validating DHT (Holochain)

- **Mechanism**: Each agent has a per-agent hash-linked source chain; every commit fans out into multiple op types (`StoreEntry`, `RegisterAgentActivity`, etc.) each routed to the DHT peers responsible for that basis hash; those peers run a deterministic `validate()` callback in WASM; valid → integrate + signed receipt back to author + gossip. Invalid → warrant published and gossiped, peers block the author.
- **Pros for Myrhiza**:
  - **Peer-symmetric, no global consensus** — proven at thousand-node scale (`holochain/lessons.md` §"Validates").
  - **HDI/HDK split** is the closest analog to Myrhiza's strict-vs-loose profile distinction; validates the model's necessity (`holochain/determinism.md` §"What's enforced", §"HDI vs HDK host functions").
  - **`must_get_*` with unresolved-dependency retry** is a clean primitive for inductive validation across partial knowledge (`holochain/determinism.md` §"The must_get_* family").
  - **Countersigning** is the right shape for atomic-multi-party events (`holochain/determinism.md` §"Countersigning protocol mechanics").
  - **Membrane proofs** are pluggable Sybil gating without consensus (`holochain/lessons.md` §"Borrow" #6).
- **Cons for Myrhiza**:
  - **Sharding ("partial arcs") is not load-tested after 6+ years** (`holochain/lessons.md` §"Avoid").
  - **DNA hash = network identity** ties any data-model bugfix to a forced-fork. (`holochain/lessons.md` §"Avoid").
  - **No built-in concept of group** — every Holochain app has reinvented group abstraction badly (`holochain/lessons.md` §"Avoid").
  - **HDK breakage every minor release** — custom WASM ABI (which Myrhiza's WIT-typed approach explicitly avoids).
  - **CRDT-shaped state issues persist** — concurrent-write convergence is still per-app-design.
- **Sources**: `prior-art/holochain/lessons.md` §§"Validates", "Avoid", "Borrow"; `prior-art/holochain/determinism.md` §§"What's enforced", "HDI vs HDK", "must_get_* family", "Countersigning".

### Option E: Hybrid — event-log replay with CRDT-typed payloads (or DHT-routed log)

- **Mechanism**: Kernel owns event-log-replay shape (Willow today). Apps choose payload semantics — including CRDT-style per-op-merge if their state shape demands it. Cross-peer convergence checked via app-exported `state-digest()` regardless of internal merge model. Optionally route ops to "interest-set" peers (DHT-flavoured) rather than full broadcast.
- **Pros for Myrhiza**:
  - **Already the direction PR #636 commits to.** `EventDag<P>` generic over opaque payload; `state-apply` is the merge function; apps can implement CRDT semantics inside `apply` if they want.
  - Inherits Willow's structural-equivocation prevention while letting apps pick merge model.
  - Validates against the CRDT corpus's "convergence is achievable" claim while letting authority sit *outside* merge (kernel-side, in pre-check).
  - The Eg-walker insight (`crdts/lessons.md` §"Borrow") supports this: deterministic-replicated-data-structure with merge semantics is the real primitive; CRDTs are one way to build it from the change DAG.
- **Cons for Myrhiza**:
  - The kernel is more opinionated than CRDT-pure; less suited to "pure CRDT app" if such a thing matters.
  - Sharding/DHT-routing is not in Willow today (full broadcast); adding it later is a big surface change.
  - More to spec — kernel must define the helper set, snapshot policy, fuel discipline, *and* leave room for app-defined merge.
- **Closest precedent**: Willow itself (proto-Myrhiza, PR #636); **runner-up**: SwingSet (single-author log) extended with Holochain's per-op-fan-out for shard-routed validation.

**Willow's current position**: Per-author Merkle DAG with topological-sort-by-`EventHash`-lex-tiebreak; `apply_event` is pure Rust today; PR #636 commits to (E) with a generic `EventDag<P>` and the deterministic-helper-set bound to `state-apply` as a WASM component (`willow/runtime-vision.md` §§"Kernel responsibilities", "The four component profiles", "Cross-peer convergence").

**Myrhiza re-evaluation question**: Is the kernel's job *event-log ordering and replay* (E, inheriting Willow), *peer-symmetric validation routing* (D, Holochain-shaped), or *something simpler that shifts more onto app-defined merge* (C-flavoured)? PR #636 picked (E) without an explicit alternatives section. Myrhiza's master spec must.

**Open questions / things to surface during brainstorming**:
- Is the per-author chain integrity rule (`seq == latest+1` ∧ `prev == head`) a kernel-level invariant or an app-pluggable one? Willow ships it kernel-side; Holochain ships it as the source-chain primitive.
- Does Myrhiza commit to single-DAG-per-topic, or does it follow Holochain's "one logical write fans into N op types" pattern? The latter is necessary if sharding/DHT-routing is ever in scope.
- Is `deps: Vec<EventHash>` (cross-author causal heads) part of the kernel envelope, or app-supplied payload metadata? (Willow has it kernel-side capped at 64.)
- Is `timestamp_hint_ms` signed-but-not-used-for-ordering (Willow today) or raised to a deterministic helper input via `host.now-hlc-from-event` (PR #636 forward-looking)?
- What happens when two peers disagree on `state-digest()` hash? Halt-and-investigate (Agoric chain-halt), log-and-continue (Croquet TUTTI), expel-via-warrant (Holochain), or replay-from-log-and-overwrite (SwingSet snapshot recovery)?
- Sharding: every-peer-holds-everything (Willow today, simpler), or arc-based responsibility (Holochain, scales but unproven)?

## Domain 2: State-apply discipline

**What's at stake**: Given the convergence paradigm, what is the contract apps must satisfy in their `state-apply` component? This binds the WIT, the fuel/limit policy, the encoding format, the snapshot mechanism, and the upgrade story. Misspecifying any of these creates correctness bugs that surface as cross-peer divergence months after deployment.

### Option A: Strict purity, kernel-deterministic-helper-set, instruction-count fuel, app `state-digest()`, transcript-as-canonical (PR #636 / Willow forward-looking)

- **Mechanism**:
  - `state-apply` imports only the deterministic helper set: `host.verify-signature`, `host.verify-payload-mac`, `host.hash`, `host.install-key (no-return)`, `host.now-hlc-from-event`, `host.log` (`willow/determinism.md` §"Aspirational (PR #636)"; willow PR diff line 80, 200).
  - Fuel = WASM instruction count, not wall time. Cap per event; out-of-fuel terminates uniformly across peers (`willow/determinism.md` §"Aspirational"; willow PR diff lines 240-241).
  - Floats: WASM spec pins them, but PR #636 recommends banning v1 anyway "to avoid review pain."
  - Cross-peer convergence: app exports `state-digest()` returning canonical bytes (sorted-collection postcard/bincode); kernel hashes the result (PR #636; `willow/determinism.md` §"state-digest, not memory-hash").
  - Snapshot: local cache only, never consensus (Agoric `#5227` precedent in `agoric-endo/determinism.md` §"What's outside the determinism boundary").
- **Pros for Myrhiza**:
  - Determinism by *spec*, not by *engine* — any conforming WASM impl produces identical `state-digest()` outputs (`agoric-endo/lessons.md` §"Avoid: single-engine lock-in").
  - Snapshot-byte divergence is structurally a leading indicator, not a correctness bug — gcc-9 cannot halt the network (`agoric-endo/determinism.md` §"gcc-9 XS heap divergence").
  - Pre-check-equals-apply property holds across the WASM boundary (same fuel, same imports — `willow/runtime-vision.md` §"Pre-check equals apply").
- **Cons for Myrhiza**:
  - Apps must implement `state-digest()` correctly — a footgun (forgetting to sort, using `HashMap`, including non-deterministic `wall-clock-derived` field).
  - Fuel cost changes are themselves consensus-breaking; any opcode-cost tuning post-launch breaks every existing transcript replay (`agoric-endo/determinism.md` §"Determinism gotchas" #5).
  - Helper-set membership is forever; adding a new helper is an ABI break.
- **Sources**: PR #636 §"Determinism, in detail" (lines 200-260 of diff); `prior-art/willow/determinism.md` §"Aspirational"; `prior-art/agoric-endo/determinism.md` §"Implications for Myrhiza".

### Option B: Strict purity + raw-memory snapshot hash (lockstep style)

- **Mechanism**: As (A) but cross-peer convergence verified via raw WASM linear-memory hash, not `state-digest()`. Croquet's TUTTI vote on `fast-json-stable-stringify` of model state is the closest analog (`croquet/determinism.md` §"What snapshots prove").
- **Pros**: No app-level canonicalization burden; check is mechanical.
- **Cons**:
  - **Diverges trivially across peers** due to allocator behavior, struct field padding, `HashMap` iteration order (PR #636 explicit warning, `willow/determinism.md` §"Aspirational"; `agoric-endo/determinism.md` §"gcc-9 XS heap divergence" — this is precisely the failure mode `state-digest()` avoids).
  - Forces single-engine lock-in or extreme engine pinning (Agoric XS-only is the precedent).
- **Sources**: `prior-art/croquet/determinism.md` §"What snapshots prove"; `prior-art/agoric-endo/determinism.md` §"Real incidents".

### Option C: CRDT-library-as-state (no app-defined apply)

- **Mechanism**: The kernel embeds Automerge / Yjs / Loro and apps express state as the library's document type. Apply = library merge.
- **Pros**: Library-provided merge semantics, mature for some shapes.
- **Cons**:
  - None of the three ship as Component Model artifacts (`crdts/comparisons.md` §6).
  - Authority enforcement still must live outside the merge (`crdts/open-problems.md` §2) — so this collapses back into "library is one app-side option, not a kernel primitive."
  - Schema evolution unsolved (`crdts/open-problems.md` §1).
  - Library choice is sticky and wire-incompatible across libraries.
- **Recommendation from corpus**: `crdts/lessons.md` §"Recommended posture" #1: **don't bake a CRDT into the kernel.** Apps may use one inside `state-apply`, but it's their choice.

### Helper-set scope question (within (A))

- **Locked**: `host.log` is non-state-affecting, allowed.
- **Locked**: Wall-clock, randomness, network, FS, env, threads — denied (CLAUDE.md "state-apply is strict purity").
- **Open**: HLC extraction (`host.now-hlc-from-event`) — Willow-shipped doesn't use HLC for ordering, only stores it as derived state; PR #636 lists it as a helper. Picking either is defensible; mixing is bug-prone (`willow/determinism.md` §"Today: HLC's actual role").
- **Open**: `host.install-key` returns no value — encodes "this peer's decryptability is *not* visible to apply" (PR #636). Should this discipline generalize to all capability-related helpers in apply (e.g. "key-handle exists" yes, "what the key decrypts to" no)?
- **Open**: Hash function — blake3? sha256? Both? Spec-pin or app-choice?

### Snapshot mechanism (within (A))

- **Three-layer persistence pattern locked-in across all four paradigms** (`agoric-endo/persistence.md` §"Three-layer persistence"): event-log (consensus), per-component KV (durable), heap snapshot (cache-only). Worth treating as already-decided-by-precedent.
- **Open**: Snapshot frequency policy — every N events, idle-driven, kernel-forced at upgrade? (Agoric default ~200 deliveries; Croquet 5s of CPU or 5min of pseudo-time.)
- **Open**: Snapshot portability across version upgrades — Agoric's `baggage` convention (durable Map keyed by string, v2 reattaches from baggage; everything else wiped). Does Myrhiza inherit this verbatim or specify something component-model-typed?
- **Open**: Null-upgrade smoke test as a kernel-level discipline (`agoric-endo/persistence.md` §""Null upgrade"" — should be in spec, not just a recommended practice.

**Open questions / things to surface during brainstorming**:
- What exactly is in the deterministic helper set? Spell out the WIT.
- Does Myrhiza specify a per-event fuel hard cap, or per-block cumulative budget, or both? Agoric uses both (per-crank `1e8` + per-vat Meter reservoirs).
- How is fuel-cost-versioning handled across runtime upgrades? (Agoric pain point: any opcode-cost change re-replays transcripts differently.)
- Does Myrhiza require an in-memory test-runtime double (Willow `MemNetwork` analog) to enforce determinism in CI? `willow/lessons.md` §"Validates: Network trait + MemNetwork" treats this as prerequisite for not-calcifying-on-real-iroh-only.
- What is the `baggage`-equivalent at upgrade? Does it need to be Component-Model-typed (resource handle to a durable map) or is a string-keyed primitive map sufficient?
- Is there a "pre-check fuel budget" separate from "apply fuel budget"? (PR #636 lines 652-654 flags this as open.)

## Domain 3: Determinism enforcement

**What's at stake**: Even if the spec says `state-apply` is pure, *enforcement* is the engineering. Every opcode that varies across CPU/OS/compiler/runtime-version is a divergence risk. The corpus has multiple production-incident war stories proving that informal enforcement fails.

### Option A: Spec-by-WIT-imports (Component Model native)

- **Mechanism**: `state-apply` declares its import set in WIT. The deterministic helper set is the *only* thing it can import. Components importing wall-clock or random fail at link, not at runtime (`agoric-endo/determinism.md` §"Implications for Myrhiza" #6 "SES `lockdown()` analogue at WIT level"; `holochain/determinism.md` §"Implications for Myrhiza" #1).
- **Pros for Myrhiza**: Statically checkable; doesn't depend on host correctness; same proof regardless of which WASM engine runs.
- **Cons**: Doesn't catch *internal* non-determinism (HashMap iteration inside the component, allocator-dependent struct layouts that escape via `state-digest()` if app forgets to sort).

### Option B: Engine-pin + ablation (Agoric/SwingSet style)

- **Mechanism**: Pin one WASM engine (e.g. Wasmtime in fuel-mode with `epoch_interruption` disabled, SIMD off, threads off, `wasm32-unknown-unknown` only). Disable non-deterministic codepaths via engine config. Vendor + version-pin.
- **Pros**: Tightest control; precedent for high-stakes deployments.
- **Cons**: **Determinism by *engine* not by *spec*** — the anti-pattern from `agoric-endo/lessons.md` §"Avoid: single-engine lock-in." Locks Myrhiza to one runtime, breaks the "any conforming WASM engine" promise.

### Option C: Explicit float discipline (ban / spec-pin / sandbox)

- **Three sub-options**:
  - **C1 — Ban floats in `state-apply`.** PR #636 lines 242-243: "Spec-deterministic floats (the WASM spec pins these), with a strong recommendation to ban v1 to avoid review pain."
  - **C2 — Spec-pin to WASM canonical-NaN behavior.** Wasmtime has a canonical-NaN mode; this gives bit-identical floats within the WASM spec but not for transcendentals (libm).
  - **C3 — Replace transcendentals with deterministic libm.** Croquet's `@stdlib/math` + iOS-Safari `pow` workaround is the precedent (`croquet/determinism.md` §"Floating-point determinism — the actual engineering"). For WASM, this means apps bundle a deterministic libm in their state component.
- **Recommendation from corpus**: C1 for v1, with the option to relax to C2 if a real app needs floats (`willow/determinism.md` §"Re-evaluates"). Agoric removed `WebAssembly` from vat surfaces partly because of float concerns (`agoric-endo/determinism.md` §"SES lockdown()").

### Option D: HashMap / allocator iteration discipline

- **Locked-in by precedent**: Willow uses `BTreeMap`/`BTreeSet` for everything serialized; `HashMap` only on `#[serde(skip)]` indices that are never iterated in apply paths (`willow/determinism.md` §"Today: idempotency hazards"). Kernel's `EventDag` uses `HashMap` internally but `topological_sort` re-orders into `BTreeSet` before iteration.
- **Open**: Is this enforced by lint, by spec-prose, by audit, by runtime check, or by `state-digest()` failure to converge? Willow today is "by audit + apply-paths-don't-iterate-HashMap"; Myrhiza spec must pick a stronger enforcement story since `state-apply` is third-party WASM, not first-party Rust.

### Option E: Instruction count vs wall time (fuel discipline)

- **Locked-in across all paradigms**: instruction count (Agoric computrons, WASM fuel) — *not* wall time. Wall time diverges across hardware; instruction count is bytecode-deterministic. (`agoric-endo/determinism.md` §"Metering: deterministic CPU accounting"; PR #636 lines 240-241.)
- **Open**: What is the per-event fuel cap? Agoric picked `1e8` "by feel and rarely adjusted." Myrhiza needs an empirical basis.
- **Open**: How is fuel-cost-table versioning handled? (Agoric's pain: tuning costs after launch is consensus-breaking.)

### Option F: Cross-peer state-digest gossip / drift detection

- **Mechanism**: After every event (or every N events), peers exchange `state-digest()` hashes. Mismatch → action: halt (Agoric), log+continue (Croquet TUTTI), warrant-and-block (Holochain), replay-from-log (SwingSet snapshot recovery).
- **Locked-in (by Myrhiza CLAUDE.md + PR #636)**: The check exists. The kernel hashes the app's `state-digest()` and gossips it.
- **Open**: Cadence (every event vs every N events vs idle vs commit-boundary).
- **Open**: Response policy. Worth borrowing Croquet's `diffDivergedSnapshots` debug surface (`croquet/determinism.md` §"What snapshots prove" — diffs two divergent JSON-shapes for human review).

**Open questions / things to surface during brainstorming**:
- WIT import set for `state-apply` — is it spec-frozen, or extendable per ABI version?
- Float discipline v1 — ban, canonical-NaN-only, or full IEEE? (PR #636 leans ban, doesn't commit.)
- Fuel cap default + how it's tuned. Per-event or per-block?
- Cross-peer state-digest gossip cadence.
- Divergence response policy. Halt? Log? Warrant? Replay-from-log?
- Replay-equivalence test surface — `(genesis-to-current)` vs `(snapshot-restart-to-current)` on different OS / arch / compiler. Agoric only caught `#4911` because they had this matrix in CI (`agoric-endo/determinism.md` §"Determinism gotchas" #3).
- Kernel input validation as consensus safety — every kernel→`state-apply` boundary needs typed input validation (`agoric-endo/determinism.md` §"the 'banana' halt"; `agoric-endo/persistence.md` §"Failure modes").
- Is there an equivalent of `bringOutYourDead` for capability-handle GC? (`agoric-endo/persistence.md` §"Distributed GC and finalization".)

## Cross-domain interactions

- **Convergence paradigm constrains state-apply ABI.** Pure event-log-replay (E or A) wants `apply(prior, event) -> next` as the entry point. Lockstep wants `tick(state, message-batch) -> next` plus a snapshot-equality vote. CRDT-merge wants `merge(state, peer-state) -> state`. Validating-DHT wants `validate(op) -> {Valid|Invalid|UnresolvedDependencies}` plus a separate apply. **PR #636's `apply` shape implicitly assumes (E).** If Myrhiza re-evaluates paradigm, the ABI rebases.

- **Snapshot mechanism depends on convergence paradigm.** Replay-shape paradigms (A/E) have a clear "log canonical, snapshot cache" answer. CRDT paradigm (C) has no snapshot — state *is* the merge result. Validating-DHT (D) has per-op signed receipts as the closest equivalent. The Agoric "snapshot-as-cache, transcript-as-source-of-truth" only works in replay shapes.

- **Determinism enforcement depends on what `state-apply` is allowed to do.** If the kernel mediates ordering (E), `state-apply` doesn't need its own ordering primitives. If state-apply is a CRDT merge (C), it needs deterministic merge primitives — but those are inside the WASM bundle, opaque to the kernel.

- **Authority validation polarity changes by paradigm.** In (A/E), kernel can pre-check before signing because order-is-decided-here. In (C), there's no pre-check — every replica sees ops in different order and must converge anyway. In (D), pre-check happens on a different peer (the validation authority for that op). **Myrhiza's "pre-check is the same WASM function as apply" only makes sense in (A/E)** — that locked-in discipline implicitly commits to event-log-replay-shape.

- **Float and HashMap discipline is independent of paradigm but matters more in some.** Lockstep + raw-memory-hash (B) is most fragile (single allocator difference halts session). Hybrid + `state-digest()` (E) is most robust — engine-internal divergence is invisible to convergence check.

- **Fuel-cost versioning is harder under "apply runs replay" semantics.** Any change to opcode-fuel-cost re-replays old transcripts at different totals. CRDT merge sidesteps this (no cumulative fuel ledger). Validating-DHT sidesteps this (per-op fuel, no replay). Replay paradigms must spec a fuel-cost epoch / version table.

## Brainstorming question list (sorted by decision-criticality)

1. **What convergence paradigm does the Myrhiza kernel commit to: event-log replay (E, inheriting Willow PR #636), validating DHT (D, Holochain-shaped), CRDT merge (C, kernel-CRDT-as-state), or hybrid?** Highest leverage. Determines ABI shape, sync protocol, sharding model, offline tolerance, failure-mode response. PR #636 picked (E) without a written alternatives section. Master spec must.

2. **What is the deterministic helper set for `state-apply`, exactly?** PR #636 lists six imports (`verify-signature`, `verify-payload-mac`, `hash`, `install-key`, `now-hlc-from-event`, `log`). Each one is forever once published. Audit each, name the runner-up, justify inclusion.

3. **What is the cross-peer divergence response policy?** Halt-and-investigate (Agoric), log-and-continue (Croquet), warrant-and-block (Holochain), or replay-from-log-and-overwrite (SwingSet snapshot recovery)? Different paradigms naturally pick different defaults; Myrhiza must articulate one.

4. **Float discipline in `state-apply` v1: ban, canonical-NaN-pin, or audit?** PR #636 leans ban without committing. Cheapest now is ban; relaxing later is straightforward. Auditing every component for float behavior is not.

5. **Snapshot portability across version upgrades — what is Myrhiza's `baggage` analog?** Agoric has 7 years of pain making `baggage` work. CRDT libs don't solve this at all. Component Model has resource handles; spec must commit to whether durable-state-bridge is a primitive map (string-keyed), a typed resource handle, or per-component contract.

6. **HLC: kernel primitive every event carries, or one input among many that the kernel passes via the event payload?** Willow today does the latter; PR #636 forward-looking proposes a `host.now-hlc-from-event` helper. Worth picking a side and not letting both coexist.

7. **Sharding model: full broadcast (Willow today) or arc-based responsibility (Holochain)?** Locks the network protocol. Holochain has 6+ years of unfinished sharding; Willow has full broadcast that works but doesn't scale past ~peer-set-of-thousands.

8. **Per-event fuel cap default + cost-table versioning policy.** Agoric picked `1e8` empirically and ate the cost-tuning pain. Myrhiza needs both a default and a policy for changing it.

9. **Encoding format for canonical `state-digest()` — postcard, bincode, CBOR, SCALE, custom?** PR #636 says "postcard with sorted collections is the existing-codebase precedent" — but shipped Willow uses bincode. Sorted-collection discipline is the load-bearing piece; the format choice is secondary but should be picked deliberately.

10. **`bringOutYourDead`-style distributed GC for capability handles — needed or subsumed by Component Model resource lifetimes?** Agoric needs it because JS GC is non-deterministic. WASM Component Model has explicit resource lifetimes — does the same problem exist?

11. **Does Myrhiza spec a single canonical WASM runtime (engine-pin, like Agoric's XS), or commit to "any conforming engine"?** Determinism-by-spec is the stronger claim; engine-pin is the easier engineering. Picking either is defensible; mixing is footgun.

12. **Replay-equivalence CI matrix — what's the test surface?** Genesis-to-current vs snapshot-restart-to-current, on different OS / arch / compiler. Cheap to add now; expensive to retrofit after a divergence incident in production.

## Sources

- `/mnt/storage/projects/myrhiza/CLAUDE.md` — Component Profiles section (locked: 4-profile split, `state-apply` strict purity, pre-check-equals-apply, capabilities-only).
- `/tmp/willow-pr-636.diff` lines 80, 200, 240-260, 380-381, 506, 552, 618-654 — deterministic-helper-set, fuel discipline, `state-digest`, child-spec list.
- `/mnt/storage/projects/myrhiza/docs/prior-art/agoric-endo/lessons.md` — Validates / Avoid / Borrow tables; XS engine choice; transcript-vs-snapshot consensus.
- `/mnt/storage/projects/myrhiza/docs/prior-art/agoric-endo/determinism.md` §§"What 'deterministic' has to mean here", "XS: the engine choice", "SES lockdown()", "Mediated I/O", "Metering", "GC determinism", "Real incidents", "Implications for Myrhiza".
- `/mnt/storage/projects/myrhiza/docs/prior-art/agoric-endo/persistence.md` §§"Three-layer persistence", "Spans and incarnations", "Snapshot frequency", "The swing-store", "Vat upgrade", "Distributed GC", "Failure modes", "Implications for Myrhiza".
- `/mnt/storage/projects/myrhiza/docs/prior-art/croquet/lessons.md` §§"Validates", "Avoid", "Borrow", "Recommendation matrix".
- `/mnt/storage/projects/myrhiza/docs/prior-art/croquet/determinism.md` §§"The lockstep paradigm", "Simulated pseudo-time", "The determinism contract", "Seeded RNG", "Floating-point determinism", "What snapshots prove", "What Croquet does not solve", "Comparison to Myrhiza".
- `/mnt/storage/projects/myrhiza/docs/prior-art/croquet/programming-model.md` §§"The two-class model", "Future calls", "Serialization & the Prime Directive", "What the developer cannot do", "Implications for Myrhiza".
- `/mnt/storage/projects/myrhiza/docs/prior-art/crdts/lessons.md` §§"Validates", "Avoid", "Borrow", "Recommendation matrix", "Recommended posture for the runtime spec".
- `/mnt/storage/projects/myrhiza/docs/prior-art/crdts/comparisons.md` §§"At-a-glance", "Algorithm comparison", "Sync protocol", "WASM compilability", "Determinism reality".
- `/mnt/storage/projects/myrhiza/docs/prior-art/crdts/open-problems.md` §§1 Schema evolution, 2 Authority, 3 Validation/invariants, 4 Tombstone GC, 5 Schema migration of bytes, 6 Long-running collaboration, 7 Rich text intent, 8 Concurrent move, 9 Cross-library interop, 10 Component Model, 11 Determinism of internal representation.
- `/mnt/storage/projects/myrhiza/docs/prior-art/holochain/lessons.md` §§"Validates", "Avoid", "Borrow".
- `/mnt/storage/projects/myrhiza/docs/prior-art/holochain/determinism.md` §§"What's enforced", "What isn't enforced", "The warrant response", "Implications for Myrhiza", "HDI vs HDK host functions", "The must_get_* family", "Inductive validation", "Validation receipts", "Countersigning protocol mechanics", "Genesis self-check", "Error model".
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/state-machine.md` §§"Shipped today: the DAG", "Shipped today: materialization", "Shipped today: out-of-order delivery", "Convergence property", "Aspirational (PR #636): generalization", "What Myrhiza inherits".
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/determinism.md` §§"Today: how convergence is achieved", "Today: HLC's actual role", "Today: idempotency hazards", "Aspirational (PR #636): determinism across the WASM boundary", "state-digest, not memory-hash", "What Myrhiza inherits".
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/runtime-vision.md` §§"Kernel responsibilities", "The four component profiles", "Apps as bundles of components", "Pre-check equals apply", "Cross-peer convergence", "Crypto and key custody", "Worker trust shifts", "What stays the same", "What's still open for Myrhiza".
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/lessons.md` §§"Validates" entries on per-author Merkle DAG, iroh, Ed25519, Network/MemNetwork, actor-only state, pre-check-equals-apply, HeadsSummary, dual-target compilation, bincode-with-sorted-collections.
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/authority.md` §§"Single-authority discipline", "The pre-check-equals-apply mechanic", "Permission tiers", "Owner-rooted-with-governance".
- `/mnt/storage/projects/myrhiza/docs/references/local-first.md` (anchor index — Lamport time, Shapiro CRDT survey, Croquet C5 2003 paper).
