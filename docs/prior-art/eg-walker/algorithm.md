**Date:** 2026-05-22
**Status:** active
**Subject:** The eg-walker algorithm — what it actually does, in enough detail to evaluate against Myrhiza's `state-apply` design

# The algorithm

Reference: Gentle & Kleppmann, *Collaborative Text Editing with Eg-walker: Better, Faster, Smaller*, EuroSys 2025 (arXiv:2409.14252, v1 submitted 2024-09-21). Where I cite section numbers below, they are inferred from the paper's structure and verbatim PDF extraction was not available during this research session — re-verify against the published PDF if precision matters. Where I quote without a verbatim flag, treat the wording as paraphrase against the [`eg-walker-reference`](https://github.com/josephg/eg-walker-reference) TypeScript implementation, the paper abstract, and Joseph Gentle's blog posts.

## The data model

An **event graph** is an append-only DAG of operations. Each operation is:

```
Operation {
  id:              (agent_id, sequence_number)
  parent_versions: set of operation ids
  type:            insert | delete
  position:        u32           // logical position at time of authoring
  content:         char          // for insert
}
```

The agent ID is a stable per-author identifier; sequence number monotonically increases within an agent. Together they form a *globally unique* operation ID. `parent_versions` is the causal frontier the originating peer observed when the operation was created — exactly the same shape as Willow's `Event { prev, deps }` (see [`../willow/state-machine.md`](../willow/state-machine.md)).

**Key property:** operations are **never modified** once created. The graph is purely additive. This is structurally the same as the per-author Merkle DAG Willow ships and Myrhiza inherits.

## Replay, not merge

In a typical CRDT (Yjs, Automerge, Loro), each operation carries metadata that determines its position in the merged document — `originLeft`/`originRight` references, RGA-style tombstone links, Fugue parents-of-parents. The merged document is a *function of that metadata*; the metadata is stored at rest.

Eg-walker takes a different posture: **store only the raw operations + causal parents; recompute merged state by walking the graph.** When two replicas merge, they:

1. Exchange the operations they don't have (gossip filtered by causal frontier — same as Hypercore/Iroh/Willow blob sync).
2. Topologically sort the new operations in causal order.
3. Replay each operation against a working state, **transforming its position** (OT-style) against any concurrent operations that have been applied since it was authored.
4. The result is the merged state.

The OT-style transformation step is what makes eg-walker fast: positions only need to be transformed for the operations that are *actually concurrent* with the operation being applied, not against every operation in history.

> "The system replays the oplog from scratch to calculate document state at any version, then updates snapshots to merge newer changes. Operations are processed independently on every peer using a Fugue-based sequencing approach (tie-breaking by agent ID)."
> — `eg-walker-reference` README

## Concurrent-insert tie-breaking: Fugue

When two replicas concurrently insert at the same position (e.g. both replicas observe `"hello world"`, replica A inserts `" wonderful"` at position 5, replica B inserts `" cruel"` at position 5), eg-walker uses **Fugue ordering** (Weidner & Kleppmann 2023, arXiv:2305.00583) to decide which insert wins the leftward slot.

Fugue's contribution: a tie-break rule that **minimises interleaving** — concurrent runs of text stay contiguous in the merged output rather than getting shuffled character-by-character. Without it, RGA-family CRDTs can produce `" w c o r n u d e e l r f u l"` (interleaved) instead of `" wonderful cruel"` or `" cruel wonderful"`.

Eg-walker inherits Fugue's tie-break rule but applies it *during replay*, not by storing tombstone-style Fugue parent references at rest. This is the key encoding win.

See also: [`../crdts/crdt-theory.md`](../crdts/crdt-theory.md) §"Fugue (Weidner & Kleppmann 2023)" — Fugue is also the algorithm Loro uses for text.

## Snapshots

Replaying from scratch every time would be O(n) in operations per query, which is unworkable for any non-trivial document. The implementation caches a **snapshot** of the document state at the current "head" frontier:

- The snapshot is a `(causal_frontier, document_state)` pair.
- New operations that are causally after the snapshot's frontier can be applied incrementally — they don't need full replay.
- When merging concurrent operations (i.e. operations the snapshot's frontier doesn't dominate), eg-walker replays *only the affected subgraph* — operations from the divergence point forward, on both branches.

The snapshot is the load-bearing engineering optimisation. Without it, the algorithm's theoretical replay-on-demand posture would not be practical. With it, steady-state operation is O(1) amortised for non-concurrent inserts and O(branch-divergence) for merges.

## The walk

The "walk" in *event-graph walker* is the topological traversal of the operation DAG during replay or merge. The walk:

1. Identifies the set of operations to apply (the difference between the snapshot's frontier and the target frontier).
2. Sorts them in a deterministic topological order (typically by `(parent_set_minimal, agent_id)` to ensure all peers walk in the same sequence — convergence depends on this).
3. For each operation, computes its **transformed position** by counting concurrent inserts and deletes between the operation's `parent_versions` frontier and the current walk frontier that affect positions ≤ the operation's `position`.
4. Applies the transformed operation to the working snapshot.

This is structurally an **OT algorithm** (Operational Transformation) — but one that operates on the persistent event graph as ground truth rather than on a live stream of operations against a fragile working state. That sidesteps OT's classical problem: OT is hard because the *transformation* must be applied identically by every peer in real-time without missing operations, and concurrent transformations can compose incorrectly. Eg-walker dodges this by transforming during *replay* — the input is the persistent graph, which all peers agree on once they've gossiped.

## Determinism guarantees

The paper claims **bit-identical convergence** when the same set of operations is applied. The convergence proof rests on:

1. The topological sort is deterministic (parent-set comparison + agent-ID tie-break).
2. The Fugue tie-break for concurrent inserts is deterministic.
3. Position transformation is deterministic given the walk order.

This matches Myrhiza's `state-apply` strict-determinism requirement (CLAUDE.md, "state-apply" profile). Eg-walker is one of the very few non-trivial replicated algorithms that publishes a deterministic bit-identical claim *and* ships a reference implementation conformance-tested against it.

**Caveat:** the *internal representation* of the snapshot is implementation-defined. Two implementations of eg-walker may have different snapshot byte layouts even when they agree on the logical document. This is the same caveat as [`../crdts/open-problems.md` §11](../crdts/open-problems.md) — for Myrhiza's content-addressable state, canonicalise externally.

## Storage format

The paper distinguishes the **algorithm** (eg-walker) from the **on-disk format** (diamond-types' columnar encoding). The reference TS implementation stores operations as plain JS objects; diamond-types packs them into:

- A columnar **oplog** (each operation field stored in its own column for compression).
- A B-tree-backed **content tree** mapping document positions to character runs and their authoring operations.
- **Packed agent IDs** — agents are interned into a small-int table so the per-op overhead is `~6-12 bytes` after run-length encoding, vs Automerge's `~240 bytes/op` in 2.x and `<1 byte/char` in 3.0 (per `automerge.org/blog/automerge-3/`).

The paper's headline performance numbers come from the diamond-types format, not the algorithm in isolation. The reference TS implementation is **~200x slower** because it skips the encoding work entirely. Future eg-walker implementations may pick different storage layouts.

## What's not in the algorithm

Eg-walker, as specified in the paper, covers **plain text**. It does not, in the published 2025 form, cover:

- **Tree structures** with concurrent move (the Kleppmann 2021 move-op problem — see [`../crdts/loro.md`](../crdts/loro.md)).
- **Rich text** intent preservation (the Peritext problem).
- **Maps / objects** with concurrent field writes.
- **Sets** with add/remove.
- **Counters**.

The `diamond-types` repo's `more_types` branch is explicit experimental work to extend the encoding to JSON-style data types; eg-walker the *algorithm* is text-only as of the paper. For non-text data, the algorithm shape would need extension; the storage techniques (columnar oplog, B-tree content tree) plausibly transfer but are not proven by the paper.

## The "is it a CRDT" question

Whether eg-walker qualifies as a CRDT depends on what you mean:

- **Yes**, in the formal sense: the algorithm is a deterministic pure function of the set of operations, so the merged state is convergent (same op set → same state on every replica). That is the CRDT definition.
- **No**, in the colloquial sense: it does not maintain CRDT-shaped metadata at rest, does not look like the RGA/YATA/Fugue persistent structures the term implies in editor codebases.

The paper's framing is "the storage form of a CRDT is more flexible than commonly assumed." Kleppmann's blog post (March 2025) is less hedged: it positions eg-walker as a *replacement* for the persistent-CRDT-at-rest pattern. For Myrhiza, the practical distinction is more interesting than the taxonomic one: eg-walker's storage form is an **event log**, exactly the shape Myrhiza's per-author Merkle DAG already provides.

## Implications for Myrhiza `state-apply`

The algorithm shape lines up with Myrhiza's `state-apply` profile:

- **Input is `(prior state, event)`**, not `(prior state, op + CRDT metadata)`. Eg-walker's replay model treats the event graph as ground truth and computes state on demand; Myrhiza treats the per-author Merkle DAG as ground truth and computes state via `state-apply`. Same posture.
- **The walk is the apply.** Eg-walker's "topologically sort then walk" is exactly what a Myrhiza state component's `apply` function does over incoming events.
- **Snapshots are an optimisation, not part of the contract.** Eg-walker's snapshot is an internal cache; replay is the source of truth. Myrhiza can adopt the same posture: state digests are derived from `apply`, not from a persistent state structure.
- **Determinism guarantees match.** Eg-walker publishes bit-identical convergence; Myrhiza requires bit-identical convergence on `state-apply`.

Where the alignment breaks:

- **Eg-walker has no signature, no authority predicate, no event-admission gate.** The algorithm assumes operations are *valid by construction*. Myrhiza's kernel pre-checks via `state-apply` in dry-run mode before signing; eg-walker has no analogue. (See [open-problems.md](open-problems.md) §2.)
- **Eg-walker has no notion of capability** or kernel-mediated host imports. The algorithm runs in a single process; Myrhiza wraps state-apply in a WASM Component with a deterministic helper set. Eg-walker would have to be re-expressed in Myrhiza's component-import shape.
- **Eg-walker is text-specific.** Myrhiza state components are payload-bytes-opaque. The algorithm's transferability beyond text is an open question.

The strongest concrete borrow: **diamond-types' columnar oplog encoding** is a directly portable idea for Myrhiza's snapshot/replay layer. The B-tree content tree is also portable, though more dependent on the data shape.

## Sources

- Paper: <https://arxiv.org/abs/2409.14252> (algorithm + storage + benchmarks; section numbers inferred, verify against PDF)
- ACM version: <https://doi.org/10.1145/3689031.3696076>
- TS reference implementation: <https://github.com/josephg/eg-walker-reference>
- Diamond-types Rust impl: <https://github.com/josephg/diamond-types>
- Fugue paper: <https://arxiv.org/abs/2305.00583>
- Kleppmann's blog overview: <https://martin.kleppmann.com/> (publications list — March 2025 eg-walker post)
- Joseph Gentle's "5000x faster CRDTs" 2021 post: <https://josephg.com/blog/crdts-go-brrr/>
