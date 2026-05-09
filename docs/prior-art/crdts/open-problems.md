**Date:** 2026-05-09
**Status:** active
**Subject:** What CRDTs structurally do not solve. Honest list of gaps Myrhiza will hit if it leans on Automerge / Yjs / Loro for `state-apply` convergence without a layer above.

CRDTs guarantee convergence for a fixed schema, fixed authority model, and fixed runtime. Real systems break at least one of those assumptions. This file enumerates the breakages.

## 1. Schema evolution

Adding a field to a `Y.Map` or `LoroDoc` map is fine *if all replicas have the new schema*. If some replicas still run the old schema and the merge model permits last-writer-wins on the field, the new-field writes from upgraded replicas can be observed-but-misinterpreted by old replicas, or writes from old replicas (which lack the field) can clobber the new structure as a logical "absence."

Removing a field is worse: concurrent operations from old-schema replicas keep producing writes for a field the new schema doesn't recognize. None of Automerge / Yjs / Loro have a built-in story here.

State of the art: Ink & Switch's Cambria (https://www.inkandswitch.com/cambria/) — bidirectional lenses, never reached production. The core problem — how to converge across heterogeneous schemas — remains open.

For Myrhiza: any change to a `state-apply` component's input schema is an ABI break unless the kernel mediates translation.

## 2. Authority / authorization

CRDTs converge regardless of *who* made the change. There is no built-in concept of "Alice is allowed to change this field, Bob is not." Add a malicious peer that emits a valid-format CRDT op against your shared doc and the algorithm dutifully merges it.

Byzantine-fault-tolerant CRDTs exist as research (Kleppmann's BFT-CRDT paper, https://martin.kleppmann.com/papers/bft-crdt-papoc22.pdf) but no mainstream lib ships authority enforcement in the merge function.

For Myrhiza: this is the core reason CRDTs alone don't satisfy `state-apply`. The kernel must validate authority *before* an event is admitted to the convergent log. Convergence does not imply legitimacy.

## 3. Validation / invariants across concurrent operations

The bank-account problem: replica A and replica B both observe balance = $100, both concurrently withdraw $80. Both ops are individually valid; the converged result is balance = $-60. CRDTs cannot reject this — the merge function does not get to refuse.

This generalizes to any cross-row invariant ("at most one row has `is_primary = true`," "sum of allocations ≤ total," referential integrity). CRDTs converge, then violate.

Workaround in practice: model the constrained quantity as a reservation/escrow data structure that *is* a CRDT (e.g. bounded counter with reservation tokens). This works for some invariants and not others. None of the three libs ship pre-built bounded-invariant types.

## 4. Garbage collection of tombstones

Per-library state:

- **Yjs:** can merge adjacent struct tombstones and discard content of deleted items, but cannot remove tombstone metadata while preserving causal order. GC must be explicitly enabled and is "unsafe" if old replicas may reappear with concurrent ops referring to GC'd structs.
- **Automerge:** retains full op history by default. 3.0 columnar encoding compresses but does not delete. Compaction stories are external.
- **Loro:** "shallow snapshot" export drops history but is destructive — replicas using it can no longer merge with replicas holding the dropped history.

In all three, **document size grows monotonically without coordinated GC.** "Coordinated" is the load-bearing word: GC requires consensus on a checkpoint, which is consensus, which is the thing CRDTs are supposed to avoid.

For Myrhiza: long-lived shared state needs a checkpoint protocol on top of the CRDT. Not solved at the lib layer.

## 5. Schema migration of on-disk bytes

Distinct from §1. Even if the logical schema doesn't change, library upgrades can change the encoding. Automerge has rewritten its wire format twice (1.x → 2.0 → 3.0). Yjs has been more conservative but added a v2 update format. Loro promises stability from 1.0 but has no track record.

Myrhiza's persistence layer cannot assume that byte-on-disk from version N is readable by version N+1 of the lib. Either pin the lib version forever, or own the migration step.

## 6. Long-running collaboration with intermittent peers

Peers offline for months: state vectors blow up (one entry per ever-active actor), bloom filter false-positive rates rise, sync delta computation gets expensive. None of the three libs have a "graceful peer eviction" story; once a peer ID exists in the causal history, it's there.

Workaround: actor-id rotation with a coordination layer. Adds complexity Myrhiza must own.

## 7. Rich text intent preservation

Concurrent boldface + delete: Alice bolds characters 5–10, Bob deletes characters 7–15. The merged result has surviving characters 5–6 bold, characters 7–15 gone. What about character 16+? Was the bold range "the word" or "those exact characters"?

Peritext (https://www.inkandswitch.com/peritext/) is the research-grade solution. Loro's rich-text component cites Peritext influence but a full reference implementation is rare. Yjs's formatting attributes use a simpler model that can produce intent violations on certain concurrent edits. Automerge has Peritext-aligned work but limited.

Concurrent insertion at the same anchor remains the canonical "interleaving" failure mode. Fugue (Loro) is the strongest lib-level answer to date. Even Fugue does not solve all rich-text intent cases.

## 8. Concurrent move of tree nodes

Only Loro implements the Kleppmann 2021 move-op CRDT (https://martin.kleppmann.com/papers/move-op.pdf). Automerge has limited move support; Yjs has no native move (apps simulate via delete + reinsert, losing identity).

For Myrhiza state with tree shape (file system, hierarchical doc, scene graph), this is decisive: Loro is the only lib that handles concurrent "move A under B" + "move B under A" without producing a cycle or losing a subtree.

## 9. Cross-library interop

A Yjs replica cannot merge with an Automerge replica. Different on-wire formats, different op identities, different conflict-resolution rules. There is no neutral "CRDT interchange format." Even within a single algorithm family (RGA), implementation byte formats differ.

Implication for Myrhiza: if a Myrhiza app picks a CRDT lib, the choice is sticky. Multi-app federation cannot rely on cross-lib convergence at the byte layer.

## 10. WASM Component Model integration

None of Automerge / Yjs / Loro ship as a Component Model `.wasm` artifact with a WIT interface. All three publish raw modules consumed via wasm-bindgen (Rust libs) or hand-written shims (Yjs).

For Myrhiza this means:

- Wrapping the lib as a Component is Myrhiza's job.
- Determinism boundaries inside the lib (struct ID generation, GC timing, columnar block boundaries) are not introspectable by the kernel — they're inside the wrapped module.
- Cross-Component sharing of CRDT state must go through serialized export/import, not shared memory.

This is not a deficiency of the libs — Component Model is young — but it's a real cost Myrhiza must absorb.

## 11. Determinism of internal representation

CRDTs guarantee identical *logical* document state given the same op set. They do *not* guarantee identical *internal byte representation*. Two replicas merging the same ops may have different in-memory struct orderings, different GC timings, different columnar block boundaries.

For `state-apply`, if the kernel needs to hash converged state for content-addressing or proof, it must canonicalize externally. The CRDT lib cannot be the source of truth for byte-equal state.

## Sources

- Cambria: https://www.inkandswitch.com/cambria/
- Cambria + Automerge integration paper: https://dl.acm.org/doi/pdf/10.1145/3447865.3457963
- BFT-CRDT (Kleppmann): https://martin.kleppmann.com/papers/bft-crdt-papoc22.pdf
- Tree-move CRDT (Kleppmann et al. 2021): https://martin.kleppmann.com/papers/move-op.pdf
- Peritext: https://www.inkandswitch.com/peritext/
- Fugue (Weidner & Kleppmann): https://arxiv.org/abs/2305.00583
- Yjs `INTERNALS.md` GC discussion: https://github.com/yjs/yjs/blob/main/INTERNALS.md
- Automerge 3.0 (compaction): https://automerge.org/blog/automerge-3/
- Loro version_deep_dive: https://loro.dev/docs/advanced/version_deep_dive
- Live & Local Schema Change paper: https://arxiv.org/pdf/2309.11406
