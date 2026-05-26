**Date:** 2026-05-22
**Status:** active
**Subject:** Lessons for Myrhiza — what eg-walker validates, what to avoid, what to borrow. The decision file.

# Lessons

This is the synthesis file — read it when designing a Myrhiza spec that touches `state-apply` convergence, event-log replay, or shared-document state. Three sections: **validates**, **avoid**, **borrow**.

## Validates

Things Myrhiza is already doing that eg-walker independently arrived at.

### V1. Event-log-replay as the convergence paradigm

Eg-walker's core move is **persist the event graph; compute state on demand by walking it**. This is structurally identical to Myrhiza's `state-apply` design:

- Myrhiza: per-author Merkle DAG of signed events; `state-apply` is a deterministic pure fn of `(prior state, event)`; state is the *result of folding* `state-apply` over events.
- Eg-walker: append-only event graph of operations; replay is a deterministic walk; state is the *result of walking* the graph.

The paradigm match is exact. **Eg-walker is the most authoritative published validation that the Myrhiza `state-apply` paradigm is sound** — Kleppmann (author of Automerge, originator of much CRDT canon) is endorsing exactly this shape over the persistent-CRDT-at-rest alternative.

Cite this when defending the `state-apply` paradigm against "why not CRDTs?" pushback. The answer is: *the most prominent CRDT researcher's own next paper does what we're already doing.*

### V2. Strict determinism on apply

Eg-walker's algorithm publishes **bit-identical convergence** as a correctness claim. The paper backs this with a conformance test between diamond-types (Rust, optimised) and eg-walker-reference (TypeScript, naive) — two independent implementations that produce identical merged state given the same op set.

This validates Myrhiza's `state-apply` strict-determinism requirement (CLAUDE.md): same `(prior state, event)` → same post-state, byte-equal across peers. Eg-walker proves the requirement is achievable in a real algorithm, not just a theoretical aspiration.

### V3. Snapshots as caches, not source of truth

Eg-walker's snapshot is an **internal cache** of the materialised state at a frontier. The source of truth is the graph. If the snapshot is lost, it can be reconstructed by walking the graph from empty.

Myrhiza's worker-computed snapshots (per [`../willow/runtime-vision.md`](../willow/runtime-vision.md) §"Apps as bundles") should follow the same posture: snapshots accelerate cold-join; the per-author Merkle DAG is the truth.

### V4. Causal frontier for sync

Eg-walker's sync model is: announce your causal frontier; receive ops you don't have. Same as Hypercore, same as Willow, same as Myrhiza's planned model. **Cross-paradigm convergence** — three independent local-first projects landed on the same sync shape, despite different data models (eg-walker text, Hypercore append-only-log, Willow/Myrhiza Merkle DAG).

This validates Myrhiza's commitment to **no state-vector layer** above the event log. Don't introduce one; the graph + frontier is sufficient.

### V5. Reference implementation as conformance test

Eg-walker-reference (TypeScript, ~200x slower, ~30x fewer LOC) is a working pedagogical implementation that conforms to diamond-types' merge behaviour. The conformance test is the algorithm's public correctness statement.

**Borrow the discipline:** any Myrhiza state-apply component spec should ship with both a reference (naive, slow, readable) implementation and an optimised one, with a conformance test between them. The reference impl is the algorithm's documentation.

## Avoid

Things eg-walker / diamond-types do or assume that Myrhiza should not lift.

### A1. Vendoring `crates.io = "diamond-types"` directly

The published `crates.io` crate is **`1.0.0` from 2022-08-25** — three years stale relative to the paper's algorithm. The master branch carries `2.0.0` in `Cargo.toml` but has not been published. `cargo add diamond-types` today gives you a pre-eg-walker-paper RGA-style CRDT, not the algorithm the paper describes.

If Myrhiza ever consumes diamond-types, the options are:

1. **Git revision pin** — loses semver, gains current code. Acceptable for an experiment, risky for production.
2. **Fork** — Myrhiza owns the maintenance. Plausible because the algorithm is fully published; the implementation is one Rust crate.
3. **Re-implement from `eg-walker-reference`** — port the 200-LOC TypeScript reference to Rust as a Myrhiza state component. Accept the ~200x perf hit until it matters. Cheapest path to an experiment.

Don't lift `crates.io` directly. The dependency posture is too weak.

### A2. Treating eg-walker as a "general-purpose deterministic merge runtime"

The paper covers **plain text**. The `more_types` branch is experimental for JSON. For Myrhiza state components that aren't text-shaped (counters, maps, sets, trees, custom domain types), eg-walker as published is *inapplicable*.

The paradigm (event-log replay) generalises; the position-transformation specifics don't. Don't pitch eg-walker as the substrate for arbitrary state components — it's the *algorithm reference* for text-shaped state, and the *paradigm reference* for everything else.

### A3. Assuming the snapshot is byte-equal across implementations

The eg-walker graph is canonical (bit-equal across peers when the op set matches). The snapshot is **implementation-defined**. Two implementations of eg-walker can produce different snapshot byte layouts even when they agree on the logical document.

For Myrhiza's content-addressed state (state-digest export per [`../willow/runtime-vision.md`](../willow/runtime-vision.md) §"Cross-peer convergence"), the **digest must be canonicalised externally**, not lifted from the snapshot. Same caveat as for any CRDT lib; cite [`../crdts/open-problems.md` §11](../crdts/open-problems.md).

### A4. Trusting the "5000x faster" framing

Gentle's 2021 "5000x faster than Automerge" benchmark is from a specific 2021 trace against Automerge 2.x. **Automerge 3.0 closed much of the gap**; Yjs/Loro never had the gap. The eg-walker paper's *current* claim is "order of magnitude less memory in steady state, orders of magnitude faster to load" — more conservative and more accurate.

Don't repeat the 2021 number. Use the paper's framing.

### A5. Assuming Byzantine-safety

Eg-walker has **no signature, no peer identity check, no permission predicate**. The algorithm trusts every op in the graph. Myrhiza's threat model — apps cannot be trusted with kernel state; the kernel pre-checks events via state-apply dry-run before signing — requires an authority layer that eg-walker doesn't provide.

**Eg-walker is the merge layer; the authority layer is Myrhiza's job.** Cite Kleppmann's BFT-CRDT paper as the conceptual reference, but don't expect either eg-walker or BFT-CRDT to provide a turnkey authority story for state-apply.

### A6. Skipping the encoding work

Most of eg-walker's performance comes from diamond-types' **columnar oplog + B-tree content tree + packed agent IDs**. The algorithm without the encoding is the eg-walker-reference TypeScript implementation: correct but ~200x slower.

If Myrhiza adopts eg-walker semantics, the encoding work is non-trivial and load-bearing. Don't ship the algorithm without budget for the encoding. Or: ship the slow version and accept the perf hit until a Myrhiza app needs the optimised form.

## Borrow

Concrete techniques to lift directly.

### B1. Op identity as `(agent, sequence, parent_versions)`

Eg-walker's op identity is **`(agent, sequence_number, parent_versions: set of op_ids)`**. Maps cleanly to Willow's `Event { author, prev, deps, ... }` and to Myrhiza's per-author Merkle DAG event identity.

The borrow: when designing the Myrhiza event payload shape for state components, **make `parent_versions` an explicit field** — not just an implicit "depend on parent in author's log." Cross-author causal dependencies are valuable for offline merging and for resolving distributed contention; eg-walker shows they don't cost much storage when columnar-encoded.

Already on the Myrhiza roadmap per [`../willow/state-machine.md`](../willow/state-machine.md) (Willow's `prev`/`deps` field). Eg-walker validates the design choice.

### B2. Topological-sort + walk for state materialisation

Eg-walker's walk is a deterministic topological traversal of the op graph. The sort criterion is `(parent_set, agent_id)` — parents-dominated first, ties broken by agent ID.

The borrow: a Myrhiza state component's "apply many events" path (e.g. cold-join replay) should use exactly this walk discipline. Deterministic sort → deterministic state. Don't iterate events in receipt order; iterate in topological order. The eg-walker-reference TypeScript code (specifically `causal-graph.ts` + `index.ts`) is the ~200-LOC reference for how to write this.

### B3. Fugue tie-break for concurrent inserts

For Myrhiza state components managing **ordered sequences** (text, lists, ordered collections), the concurrent-insert tie-break should use **Fugue ordering** — agent-ID tie-break that minimises interleaving. This is the current research-grade best answer; Loro adopts it; eg-walker adopts it.

The borrow: codify Fugue as the canonical tie-break in Myrhiza's "ordered collection" state-component contract. Don't invent a new tie-break; don't accept RGA-style tie-break that produces interleaving artefacts.

### B4. Columnar oplog encoding for snapshots

Diamond-types' columnar oplog (each operation field in its own column, with column-specific encoding) is the **directly portable encoding technique** independent of the eg-walker algorithm. Achieves ~1-10 bytes/op for op-log storage.

The borrow: when Myrhiza needs an on-disk format for a state component's event log (especially for snapshot transfer between peers, or for worker-computed checkpoints), columnar-encode by field. This is the same encoding Automerge 3.0 uses; it's the local-first community's settled answer.

### B5. Snapshot = state + frontier (not state + version vector)

Eg-walker's snapshot pairs the materialised state with the **causal frontier** (set of op IDs at the tip). Not a version vector (per-agent max counter). The frontier is the minimal set; it's what sync uses; it's what cold-join needs.

The borrow: when Myrhiza defines the worker-computed snapshot format, use `(materialised_state_digest, causal_frontier_op_id_set)` as the carry — not a version vector. The frontier is what other peers need to determine "what ops should I send you to bring you to this snapshot."

### B6. Reference + optimised implementation pair

The eg-walker-reference (TS, 200x slower, 30x fewer LOC) + diamond-types (Rust, optimised) pair, with conformance testing between them, is the gold-standard model for publishing a deterministic-merge algorithm. The reference impl is the algorithm's documentation; the optimised impl is the production target; the conformance test is the spec.

The borrow: for each Myrhiza state-component algorithm (eg-walker-shape text, future tree-shape, future map-shape), ship a reference + an optimised implementation + a conformance test. The reference impl can live in `docs/prior-art/eg-walker/examples/` or in the spec itself; the optimised impl is the production code; the conformance test is the spec contract.

### B7. Best-Artifact-Award-grade artefact discipline

The eg-walker paper's reproducibility artefact ([`egwalker-paper`](https://github.com/josephg/egwalker-paper)) won the Gilles Muller Best Artifact Award at EuroSys 2025. It contains: paper source, benchmarks, comparator implementations, datasets, results, charts. Anyone can rerun the benchmarks end-to-end.

The borrow (process, not code): when Myrhiza publishes a state-component algorithm spec, ship an artefact in the same shape. Spec source + benchmarks + comparator impls + datasets + results. This is what makes a research-grade-but-shipping claim **defensible** rather than aspirational.

### B8. Honest "research-grade-but-shipping" framing

Diamond-types is shipping (crates.io, npm, 1,800 stars, paper-validated) but **not production-hardened** (no flagship app, three-year-stale published crate, one steward). The README is honest about this ("WIP," "Cargo package is quite out of date").

The borrow (rhetorical): when Myrhiza ships something that's algorithmically vetted but not yet production-flagship-validated, **adopt eg-walker's framing**. Don't soft-pedal the gap. The "research-grade-but-shipping" label is the right framing for any new Myrhiza state-component primitive before it has a flagship app.

## How this lands in Myrhiza specs

When designing a Myrhiza spec that touches `state-apply` convergence, ask:

1. **Does the algorithm produce bit-identical convergence?** (cite V2)
2. **Is the persistent form an event log, not a CRDT structure?** (cite V1)
3. **Are snapshots derived caches, not source-of-truth?** (cite V3, A3)
4. **Is the sync model causal-frontier-based, not state-vector-based?** (cite V4)
5. **Does the algorithm have a reference + optimised implementation pair?** (cite B6)
6. **Is the encoding columnar?** (cite B4)
7. **Have we resisted vendoring `crates.io = "diamond-types"` directly?** (cite A1)
8. **Have we kept the authority layer separate from the merge layer?** (cite A5)

For text-shaped state components specifically:

- **Is Fugue the tie-break?** (cite B3)
- **Is the `(agent, sequence, parent_versions)` op identity explicit?** (cite B1)

If the answer to all of these is yes, the spec is aligned with eg-walker's lessons. If any answer is no, surface it as a deliberate decision in the spec body, with the rationale.

## Sources

- All cross-references inline. Synthesis from sibling files in this folder + cross-folder pointers.
- Algorithm: [algorithm.md](algorithm.md)
- Diamond-types: [diamond-types.md](diamond-types.md)
- Open problems: [open-problems.md](open-problems.md)
- Cross-folder: [`../crdts/lessons.md`](../crdts/lessons.md), [`../willow/runtime-vision.md`](../willow/runtime-vision.md)
- Paper: <https://arxiv.org/abs/2409.14252>
