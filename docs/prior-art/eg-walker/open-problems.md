**Date:** 2026-05-22
**Status:** active
**Subject:** What eg-walker / diamond-types structurally does not solve. Honest list of gaps Myrhiza will hit if it leans on the algorithm/impl for `state-apply` without a layer above.

# Open problems

Eg-walker resolves *some* of the problems [`../crdts/open-problems.md`](../crdts/open-problems.md) lists — memory at rest, load time, long-running offline merge. It does not resolve all of them; in some cases it inherits the same problem in a slightly different form; in some cases it introduces *new* problems.

This file enumerates the breakages honestly. The paper's own §7 (Limitations) is short; this file extends it with Myrhiza-specific gaps the paper doesn't surface.

## 1. Garbage collection at scale (unsolved)

**The problem.** Eg-walker stores operations forever, by design. Every keystroke is one op; an active document accumulates ops monotonically. The snapshot caches the materialised state but does not replace the ops — the ops are needed for any future merge with a replica that has *not* observed them.

The paper acknowledges this in its limitations section (paraphrased into this synthesis): event-graph storage grows linearly with edit history, and garbage collection requires coordination across all replicas that may still hold divergent branches.

**Comparison.** CRDTs have the same problem (per [`../crdts/open-problems.md` §4](../crdts/open-problems.md)). Yjs has explicit GC; Automerge compresses but does not delete; Loro's "shallow snapshot" is destructive. Eg-walker is **not better** here; it inherits the coordinated-GC problem that none of the libraries solve.

**Implication for Myrhiza.** A Myrhiza app using eg-walker semantics for shared text will accumulate ops indefinitely. Any GC story is *outside* eg-walker — the same checkpoint protocol [`../crdts/open-problems.md` §4](../crdts/open-problems.md) describes.

## 2. Authority / Byzantine resistance (unsolved)

**The problem.** Eg-walker's algorithm assumes operations are valid by construction. There is no signature, no peer identity check, no permission predicate. A malicious peer can author any operation with any agent-ID and any `parent_versions` set; the algorithm will dutifully fold it into the graph.

The paper does not discuss authentication; this is intentional — the algorithm is a *merging* algorithm, not an authority layer.

**Comparison.** Same gap as Automerge / Yjs / Loro. Kleppmann's own BFT-CRDT paper (<https://martin.kleppmann.com/papers/bft-crdt-papoc22.pdf>) addresses this conceptually for CRDTs but is not folded into eg-walker.

**Implication for Myrhiza.** This is the **core reason eg-walker alone does not satisfy `state-apply`.** Myrhiza's kernel pre-validates events via the state-apply dry-run, signs them with the author's Ed25519 key, and rejects events that fail the pre-check. Eg-walker has no analogue. Myrhiza wraps eg-walker semantics with an authority layer; eg-walker provides the merge layer.

The structural fit: eg-walker's `(agent, sequence, parent_versions)` op identity maps cleanly onto Willow's `Event { author, prev, deps, payload, sig }` — the eg-walker `agent` is Willow's `author` (Ed25519 public key); the eg-walker `parent_versions` is Willow's `prev` + `deps`. Eg-walker doesn't have `sig` because the algorithm doesn't need it; Myrhiza adds it at the kernel layer.

## 3. Validation / cross-op invariants (unsolved)

**The problem.** The bank-account problem (from [`../crdts/open-problems.md` §3](../crdts/open-problems.md)) applies to eg-walker unchanged. Replica A and replica B both observe balance = 100, both concurrently withdraw 80. Both ops are individually valid; the converged result is balance = -60.

Eg-walker's per-op transformation can detect *position-level* conflicts (two inserts at position 5) and resolve them deterministically; it cannot detect *semantic* conflicts (two withdrawals violating a balance invariant).

**Implication for Myrhiza.** Same as CRDTs. Myrhiza's `state-apply` pre-check runs the apply on a hypothetical post-state and rejects if it produces an invalid state. This is the kernel's job; eg-walker is one possible mechanism for the *merge* step but does not own the *validate* step.

## 4. Schema evolution (unsolved, possibly harder than CRDTs)

**The problem.** Eg-walker stores raw ops. If the document's schema changes (new field, removed field, changed type), old ops in the graph still refer to the *old* schema's positions. Re-walking the graph under the new schema means the algorithm needs to know how to translate old-schema positions to new-schema positions — i.e. an explicit schema-migration step.

CRDTs at least have a "merged state" they can transform; eg-walker has only the op log, which is harder to migrate because each op is positionally encoded against the schema at the time of authoring.

**Comparison.** Possibly *harder* than the CRDT case in [`../crdts/open-problems.md` §1](../crdts/open-problems.md) because eg-walker's storage form is *more* tied to the original schema. The Cambria-style schema-lens approach (`https://www.inkandswitch.com/cambria/`) would have to be applied to each op individually during replay.

**Implication for Myrhiza.** State-apply ABI break = ABI break. If a Myrhiza state component changes its event payload format, the kernel needs to either re-serialise the per-author Merkle DAG (expensive) or run a schema-translation layer at apply time (possible but slow). Eg-walker doesn't help here.

## 5. Offline merge cost when full history isn't available (new problem)

**The problem.** A replica that joins the network *fresh* (no prior history) needs the full event graph to materialise the current state. A replica that has been offline for a long time may need many gigabytes of operations to catch up — eg-walker's "load from disk" advantage is for a replica that *already has* the graph; the cold-join case is bounded by the graph size.

CRDTs that ship snapshot-based catchup (Loro's `shallow_snapshot`, Automerge's compact) can deliver a small bootstrap blob; eg-walker as published has no such mechanism — the snapshot is a derived cache, not an authoritative state form.

**Implication for Myrhiza.** A Myrhiza peer that joins a long-running topic still needs the per-author Merkle DAG (this is true today). The eg-walker model doesn't make the cold-join cheaper. Worker-computed snapshots (as Myrhiza already plans — see [`../willow/runtime-vision.md`](../willow/runtime-vision.md)) need to be authoritative-snapshot-shaped, not derived-cache-shaped.

## 6. Partial replication / view-only peers (unsolved)

**The problem.** Eg-walker assumes every replica that wants to *query* the document has the *full* operation graph. There is no story for "I want to read the document but I don't want to store every op." A replica that wants partial replication has to truncate the graph, which means losing the ability to merge with replicas that have ops in the truncated region.

**Comparison.** This is the same partial-replication problem [`../crdts/open-problems.md` §6](../crdts/open-problems.md) acknowledges for CRDTs. Eg-walker doesn't make it better and may make it worse (because the graph is the canonical form, not a derived structure).

**Implication for Myrhiza.** Read-only peers (a phone glancing at a document) want to consume the snapshot, not the full graph. Eg-walker's snapshot-as-cache posture makes this awkward — the snapshot isn't authoritative on its own. Myrhiza will need worker peers that produce authoritative snapshots (signed, content-addressed) for read-only peers.

## 7. Rich text intent preservation (partially solved)

**The problem.** Eg-walker uses Fugue ordering for tie-breaks, which is the strongest published answer to concurrent-insert interleaving. For *insert/delete* on plain text, eg-walker preserves intent at least as well as any production CRDT.

For *rich text* (bold, italic, links, headings) — the Peritext problem (`https://www.inkandswitch.com/peritext/`) — eg-walker as published in the paper does **not** address formatting marks. The paper is plain-text-only.

**Comparison.** Automerge has Peritext-aligned work; Loro implements Peritext for text formatting. Eg-walker does not (yet).

**Implication for Myrhiza.** If a Myrhiza app needs rich text, eg-walker's published form is insufficient. The Peritext-merge logic would have to be layered on top — possibly as an extension of the eg-walker walk, possibly as a separate per-mark merge step.

## 8. Cross-implementation compatibility (unsolved)

**The problem.** The eg-walker paper specifies the algorithm; it does not specify a *wire format*. Two implementations of eg-walker (diamond-types in Rust; eg-walker-reference in TS; the in-paper formal specification) compute the same logical merge result but **do not exchange operations in a defined format**. The reference TS impl is conformance-tested against diamond-types — but the conformance test is "do they produce the same state given the same op set," not "do they read each other's serialised oplogs."

The two npm packages (`diamond-types-web`, `diamond-types-node`) are wrappers over the *same* Rust core, so they share the diamond-types-specific wire format. A future independent eg-walker implementation would not share that format.

**Comparison.** This is *the same* cross-library interop problem [`../crdts/open-problems.md` §9](../crdts/open-problems.md) names for CRDTs: even within a single algorithm family, no neutral interchange format exists.

**Implication for Myrhiza.** If multiple Myrhiza apps want to share state-apply implementations across language boundaries, the wire format has to be Myrhiza-specified (probably in a WIT contract), not eg-walker-specified. Eg-walker tells you the merge semantics; it does not tell you the bytes.

## 9. WASM Component Model integration (unsolved)

**The problem.** Diamond-types ships as raw wasm-bindgen modules. The npm packages (`diamond-types-web`, `diamond-types-node`) are 1.0.2 from 2023-05-15 and consume the old (pre-eg-walker-paper) diamond-types core. There is no Component Model artefact, no WIT contract, no Myrhiza-shaped host-import binding.

**Comparison.** Same gap as Automerge / Yjs / Loro per [`../crdts/open-problems.md` §10](../crdts/open-problems.md). Component Model is young.

**Implication for Myrhiza.** Adopting eg-walker semantics means writing the WIT and the host-binding ourselves. The Rust core can be wrapped via `cargo component`, but the algorithm's host-import surface (allocators, time, etc.) has to be mapped to Myrhiza's `state-apply` deterministic-helper-set explicitly.

## 10. Determinism of internal representation (same as CRDTs)

**The problem.** Eg-walker guarantees identical *logical* document state given the same op set. It does *not* guarantee identical *internal snapshot byte representation*. Two replicas merging the same ops may have different snapshot byte layouts.

**Comparison.** Same caveat as [`../crdts/open-problems.md` §11](../crdts/open-problems.md). The op graph is canonical; the snapshot is implementation-defined.

**Implication for Myrhiza.** Myrhiza state components need a `state-digest()` export that canonicalises externally — the eg-walker snapshot can't be the source of truth for content-addressed state. This matches the existing [`willow/runtime-vision.md`](../willow/runtime-vision.md) §"Cross-peer convergence" commitment.

## 11. Stewardship / bus factor (real)

**The problem.** Joseph Gentle is the sole maintainer of diamond-types. The published `crates.io` version is three years stale. Funding flows via the Invisible College (one institutional source).

**Comparison.** Comparable to Yjs's Kevin-Jahns risk and Loro's Zixuan-Chen risk per [`../crdts/governance.md`](../crdts/governance.md). Healthier than zero-stewardship; less healthy than Automerge (Ink & Switch backing).

**Implication for Myrhiza.** If Myrhiza vendors diamond-types, the bus-factor is real. Forking is plausible (ISC license permits it; the algorithm is fully published). The pedagogical TS implementation (`eg-walker-reference`, 200 LOC) provides a fallback path: a Myrhiza-owned Rust re-implementation of the paper's algorithm is a manageable scope.

## 12. The algorithm is text-specific (real)

**The problem.** The paper covers plain text. The `more_types` branch of diamond-types is experimental work to extend to JSON-style data; this is not paper-published.

For Myrhiza state components that aren't text-shaped (counters, maps, sets, trees), eg-walker as published is **inapplicable**.

**Comparison.** Automerge, Yjs, Loro all cover multiple data shapes. Eg-walker is more like a single-purpose algorithm — text is the entire scope.

**Implication for Myrhiza.** For non-text apps, eg-walker is the *paradigm reference*, not the *algorithm to use*. The event-log-replay shape generalises; the position-transformation specifics don't.

## Sources

- Paper (limitations section): <https://arxiv.org/abs/2409.14252>
- Diamond-types README ("WIP", `more_types` branch): <https://github.com/josephg/diamond-types>
- `eg-walker-reference` README (conformance test, ~200x slower): <https://github.com/josephg/eg-walker-reference>
- Cross-reference: [`../crdts/open-problems.md`](../crdts/open-problems.md)
- BFT-CRDT (Kleppmann, the authority side): <https://martin.kleppmann.com/papers/bft-crdt-papoc22.pdf>
- Cambria (schema evolution): <https://www.inkandswitch.com/cambria/>
- Peritext (rich text): <https://www.inkandswitch.com/peritext/>
- Move-op tree CRDT (Kleppmann 2021): <https://martin.kleppmann.com/papers/move-op.pdf>
