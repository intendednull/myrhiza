**Date:** 2026-05-22
**Status:** active
**Subject:** Eg-walker / diamond-types-specific terms. Use this when reading the paper or the implementations.

# Glossary

Terms common to general CRDT discussion are covered in [`../crdts/glossary.md`](../crdts/glossary.md); this file lists only eg-walker-specific or diamond-types-specific vocabulary.

## Algorithm terms

**Event graph.** The append-only DAG of all operations ever authored on a document. Each node is an operation; each edge is a "happens-before" relationship via `parent_versions`. The graph is the canonical persistent form in eg-walker — closer in shape to a git commit DAG or Hypercore log than to a CRDT's per-character metadata.

**Walk (verb / noun).** The topological traversal of the event graph during replay or merge. To *walk* the graph is to apply operations in causal order, transforming positions OT-style. The walk is deterministic — all peers produce the same materialised state from the same graph.

**Causal frontier.** The set of operation IDs that are at the "tip" of an event graph — operations no other observed operation depends on. Equivalent to git's set-of-heads, or to a CRDT version vector's max-clock per agent. Used for sync: peers exchange the difference between their frontiers.

**Snapshot.** The materialised document state at a specific causal frontier, plus the frontier itself. An optimisation: replays don't need to start from the empty document if a snapshot is cached. In eg-walker the snapshot is *not* the canonical form — the graph is. The snapshot is implementation-defined.

**Replay.** Computing document state by walking the graph from a known starting point (empty doc or cached snapshot) to a target frontier. Replays are deterministic; same graph → same state.

**Fugue ordering / Fugue tie-break.** The Weidner & Kleppmann 2023 rule for breaking ties between concurrent inserts at the same position. Eg-walker inherits Fugue from the paper of the same name and applies it during walk, not at storage time. Critical for minimising interleaving: concurrent runs of text stay contiguous in the merge.

**Operation transformation (during walk).** The position-adjustment step eg-walker performs as it walks an operation against the current snapshot. Structurally an OT transformation, but applied to operations from the persistent graph rather than to operations from a live stream. The "OT-without-OT's-fragility" move.

**`(agent, sequence_number)` op ID.** Eg-walker's globally unique operation identifier. `agent` is a per-author stable identifier (could be Ed25519 pubkey, could be an interned table index); `sequence_number` is monotonic within the agent. Maps cleanly to Willow's `Event { author, prev, ... }` pattern.

**`parent_versions`.** The set of operation IDs the originating peer's frontier contained when the operation was created. The causal context for the op. Equivalent to a vector-clock observation, but expressed as a literal set rather than a per-agent counter.

## Diamond-types implementation terms

**Oplog.** Diamond-types' columnar storage layout for the operation graph. Each operation field (agent, sequence, parent_versions, type, position, content) is stored in its own column with column-specific encoding (run-length, delta, dictionary). The on-disk format diamond-types ships.

**Content tree.** Diamond-types' B-tree mapping document positions to character runs and their authoring operations. Used during walks to find the current position of any operation's character.

**Packed agent ID.** Diamond-types' interned-small-int representation of `(agent, sequence)` op identifiers. Agents are stored in a per-document table; each op references its agent by table index. Means a single-byte tag suffices for most ops.

**`more_types` branch.** The experimental diamond-types branch extending the algorithm beyond plain text to JSON-style data (maps, lists, counters). Not paper-published; not on crates.io.

**Diamond-types CRDT (legacy).** The Stage-2 (2021-2022) RGA-style fast CRDT diamond-types shipped before the eg-walker algorithm. Referenced in the paper as the `DT-CRDT` comparator. The `1.0.0` crates.io release.

**Diamond-types eg-walker (current).** The Stage-3 (2024+) implementation of the paper's algorithm. In-tree on master; `2.0.0` in `Cargo.toml`; **not published to crates.io**.

## Wrapper / packaging terms

**`diamond-types`.** The Rust crate. Published at `1.0.0` on crates.io (2022-08-25). Master is at `2.0.0` (unpublished).

**`diamond-types-web`.** npm package — WASM bindings for browsers. Latest `1.0.2` (2023-05-15). Wraps the Stage-2 diamond-types CRDT, not the current eg-walker.

**`diamond-types-node`.** npm package — WASM bindings for Node.js. Latest `1.0.2` (2023-05-15). Same Stage-2 wrapping as `-web`.

**`eg-walker-reference`.** The pedagogical TypeScript reference implementation. Conformance-tested against diamond-types. ~200x slower; ~30x fewer LOC. The right starting point if you want to read code.

**`egwalker-paper`.** The reproducibility artefact repository. Contains paper source (LaTeX), benchmarks (Criterion.rs), datasets, comparator implementations.

**`egwalker-from-scratch`.** Joseph Gentle's live-coded tutorial repo. Useful for understanding the algorithm progressively.

## Related-but-distinct terms

**ShareJS.** Joseph Gentle's 2011-era OT library. **Not eg-walker.** Same author; different algorithm; different paradigm.

**ShareDB.** ShareJS's successor (Gentle, 2015-current). **Not eg-walker.** OT-based, server-mediated.

**Diamond-types CRDT vs Diamond-types eg-walker.** The former is the project's pre-paper RGA-style implementation; the latter is the post-paper algorithm. Same project name; very different internal posture.

**Fugue (the paper) vs Fugue (the tie-break rule).** Fugue is both a CRDT algorithm by Weidner & Kleppmann (2023) — used by Loro for text — *and* the tie-break rule eg-walker adopts. Eg-walker uses Fugue *during walks*, not as a persistent structure.

## Sources

- Paper: <https://arxiv.org/abs/2409.14252>
- Diamond-types: <https://github.com/josephg/diamond-types>
- Eg-walker reference: <https://github.com/josephg/eg-walker-reference>
- Fugue paper: <https://arxiv.org/abs/2305.00583>
- Cross-reference: [`../crdts/glossary.md`](../crdts/glossary.md)
