**Date:** 2026-05-09
**Status:** active
**Subject:** algorithmic foundations underlying Automerge, Yjs, and Loro — what Myrhiza needs from CRDT theory before picking an algorithm

## 1. What is a CRDT

A Conflict-free Replicated Data Type is a data structure replicated across peers where concurrent updates merge deterministically without coordination. Two formal flavours, defined in Shapiro et al. 2011:

- **CvRDT (state-based / convergent)** — replicas exchange full state; merge is a join on a join-semilattice (idempotent, commutative, associative).
- **CmRDT (operation-based / commutative)** — replicas exchange operations; the delivery layer guarantees causal broadcast and exactly-once delivery, and concurrent ops must commute.

Strong Eventual Consistency (SEC) is the convergence target: any two replicas that have observed the same set of updates are in equivalent states. For Myrhiza this is exactly the property `state-apply` must give us — a pure function of `(prior state, event)` whose output depends only on the causal set of events seen, not the order they arrived.

## 2. The text-CRDT problem

Counters and grow-only sets are easy because their concurrent operations naturally commute. Sequences (text, lists) are hard: position is relative, two peers can insert "between A and B" simultaneously, and any total order over those inserts has to be invented from local information alone.

Three properties the literature aims for:

- **Convergence** — same state from same op set.
- **Intent preservation** — what the user meant locally survives the merge (e.g., a word stays a word, a paragraph break stays a paragraph break).
- **No interleaving** — concurrent runs of contiguous text from different peers do not get shuffled together.

Convergence is mechanical. The other two are where algorithms diverge.

## 3. The text-CRDT family tree

- **Treedoc (Preguiça et al. 2009, ICDCS)** — positions are paths in a dense binary tree. Concurrent inserts at the same spot fork the tree.
- **Logoot (Weiss, Urso, Molli 2009, ICDCS; Logoot-Undo 2010, IEEE TPDS)** — positions are dense ordered identifiers (lists of integers). No tombstones.
- **WOOT (Oster, Urso, Molli, Imine 2006)** — first proper text CRDT; each char carries `(prev_id, next_id)` and a tombstone on delete.
- **RGA (Roh, Jeon, Kim, Lee 2011, JPDC; earlier KAIST tech report 2009)** — Replicated Growable Array. Linked-list-of-chars with s4vector timestamps; remote ops in O(1) via a hash index.
- **YATA (Nicolaescu, Jahns, Derntl, Klamma 2016, GROUP)** — RGA-shaped but conflict resolution uses both left and right origin pointers; backs Yjs.
- **Fugue (Weidner & Kleppmann 2023, arXiv 2305.00583)** — proves and achieves *maximal non-interleaving*; backs Loro and shapes recent Yjs work.
- **Eg-walker (Gentle, Kleppmann 2024, arXiv 2409.14252; EuroSys 2025)** — non-CRDT approach: an event graph plus an OT-style replay walker.

Library mapping: Automerge uses RGA; Yjs uses YATA; Loro uses Fugue (text) plus a movable tree.

## 4. The move-tree problem

A naive "move node X under parent Y" CRDT is broken: two peers can move A under B and B under A concurrently, producing a cycle, or both move the same node concurrently and lose one move. Kleppmann, Mulligan, Gomes, Beresford 2021 ("A Highly-Available Move Operation for Replicated Trees", IEEE TPDS 33(7)) solves this by making each move op carry the *prior parent* of the moved node and reordering operations by Lamport timestamp on convergence — undoing and redoing moves locally so the final tree is the same on every replica. Loro's movable tree implements this; Automerge's tree is built on the same primitives. Real bugs in Google Drive and Dropbox motivated the paper.

For Myrhiza this matters because filesystem-shaped state (folders, threads, channels, scenes) wants a tree, and a non-converging tree is a determinism violation.

## 5. Rich text — Peritext

Plain-text CRDTs handle characters; rich text adds formatting spans (bold, italic, link). Peritext (Litt, Lim, Kleppmann, van Hardenberg 2022, PACMHCI 6 CSCW2 art. 531; Ink & Switch essay 2021) stores formatting as separate operations that *reference stable character IDs* at the start and end of each span, then derives the final formatted view deterministically. Concurrent "bold this word" and "extend the word by typing" merge intuitively. Loro's `crdt-richtext` implements Peritext on top of Fugue.

## 6. The interleaving anomaly

Concrete case. Document is `Hello`. Alice inserts ` world` at position 5. Bob concurrently inserts ` everyone` at position 5. With Logoot or Treedoc the dense identifier space can pick fresh IDs that happen to fall in the middle of either run, producing ` weverlydone` or similar. With RGA/YATA this is mostly avoided in the forward direction but can fail at left-edge inserts. Fugue formalises *maximal non-interleaving*: for any two concurrent contiguous runs, the merged result places all of one run before all of the other. FugueMax is proved to satisfy it; ordinary CRDTs (RGA, YATA, Logoot, Treedoc) do not.

This is purely a UX correctness property — the algorithms still converge — but a bad merge there is the difference between "looks like a sane edit conflict" and "looks like the document was put through a blender".

## 7. State vectors and version vectors

Tracking causal context is universal. A *version vector* maps `peer_id -> highest_seq_seen`. For two replicas to know what to send each other, exchange version vectors and ship the delta. Yjs/YATA, Automerge, and Loro all use this shape, with library-specific encodings (Automerge's columnar binary, Yjs's varint stream, Loro's RLE-compressed). The vector is also how `state-apply` should know whether an event is causally ready — Myrhiza's kernel must enforce causal delivery before invoking `state-apply`, otherwise convergence breaks.

## 8. CAP framing

CRDTs sit on the AP side of CAP: available and partition-tolerant, eventually consistent. What you give up is *invariants that need consensus*: uniqueness (no two users pick the same username), conservation (account balance never goes negative), referential integrity in the strict sense. Anything that is monotonic, append-only, or can be reframed as a non-conflict is fine; anything that needs a global "no" answer is not. Myrhiza's authority model (some events need pre-check by `state-apply` in dry-run) is a deliberate retreat from pure CRDT toward a per-namespace authority — get the AP defaults, opt into CP where invariants demand it.

## 9. Algorithm comparison

| Algorithm | Interleaving | Op size | Doc size growth | GC support | Intent | Used by |
|---|---|---|---|---|---|---|
| Treedoc | weak (depth-based) | path id | tombstones, rebalancing hard | partial | weak | none in survey |
| Logoot | weak | dense id (grows) | unbounded id growth | yes (no tombstones) | weak | research |
| WOOT | medium | char + 2 ids | tombstones forever | hard | medium | research |
| RGA | medium-good | char + s4vector | tombstones, GC viable | with causal stability | good | Automerge |
| YATA | good (forward) | char + 2 origins | tombstones, GC viable | with causal stability | good | Yjs |
| Fugue / FugueMax | maximal | char + 2 origins | tombstones, GC viable | with causal stability | best | Loro, Yjs (recent) |
| Eg-walker | maximal (replay) | event in graph | event log only | log compaction | best | diamond-types, Loro (event-graph) |

"GC" here means: can the algorithm forget tombstones once the network has caught up? RGA-family yes, given causal stability across all peers. Treedoc/WOOT structurally hard.

## 10. Eg-walker — the "non-CRDT" turn

Kleppmann's Eg-walker (Gentle & Kleppmann 2024, EuroSys 2025) keeps the event log as the canonical store, but does not maintain a persistent CRDT structure in memory. To merge, it walks the event graph and replays events through an OT-style transformer; the working state is the document text, not a tagged-character tree. Claimed wins:

- Steady-state memory close to plaintext — no tombstones.
- Document load is reading text + log header, not reconstructing a CRDT.
- Long-divergent merges faster than OT.

Catch: it is still a peer-to-peer-capable scheme (the event graph is the wire format), but its merge cost scales with the size of the divergent region, not with op count overall — so very-long-running offline branches still pay. And it is newer; the "is this provably convergent in all cases" literature is still settling. Loro has incorporated event-graph walker ideas; it is the live alternative to "everything is a tagged-character CRDT".

For Myrhiza's `state-apply`, Eg-walker is interesting because it suggests the *log* is the real ground truth and the in-memory state is a derivable cache — which matches the "state is a function of the event history" framing we already need.

## Sources

- Shapiro, Preguiça, Baquero, Zawirski 2011 — *A Comprehensive Study of Convergent and Commutative Replicated Data Types*. INRIA RR-7506. https://inria.hal.science/inria-00555588
- Shapiro, Preguiça, Baquero, Zawirski 2011 — *Conflict-free Replicated Data Types*. SSS 2011. https://www.lip6.fr/Marc.Shapiro/papers/2011/CRDTs_SSS-2011.pdf
- Preguiça, Marquès, Shapiro, Letia 2009 — *A Commutative Replicated Data Type for Cooperative Editing*. ICDCS 2009. https://inria.hal.science/inria-00445975
- Weiss, Urso, Molli 2009 — *Logoot: A Scalable Optimistic Replication Algorithm for Collaborative Editing on P2P Networks*. ICDCS 2009. https://inria.hal.science/inria-00432368
- Weiss, Urso, Molli 2010 — *Logoot-Undo*. IEEE TPDS.
- Oster, Urso, Molli, Imine 2006 — *Real time group editors without Operational Transformation* (WOOT). INRIA RR-5580. https://inria.hal.science/inria-00071240
- Roh, Jeon, Kim, Lee 2011 — *Replicated abstract data types: Building blocks for collaborative applications*. JPDC 71(3):354-368. http://csl.skku.edu/papers/jpdc11.pdf
- Nicolaescu, Jahns, Derntl, Klamma 2016 — *Near Real-Time Peer-to-Peer Shared Editing on Extensible Data Types*. ACM GROUP 2016. https://dl.acm.org/doi/10.1145/2957276.2957310
- Weidner & Kleppmann 2023 — *The Art of the Fugue: Minimizing Interleaving in Collaborative Text Editing*. arXiv:2305.00583. https://arxiv.org/abs/2305.00583
- Gentle, Kleppmann 2024 — *Collaborative Text Editing with Eg-walker: Better, Faster, Smaller*. arXiv:2409.14252. EuroSys 2025. https://arxiv.org/abs/2409.14252
- Kleppmann, Mulligan, Gomes, Beresford 2021 — *A Highly-Available Move Operation for Replicated Trees*. IEEE TPDS 33(7):1711-1724. https://martin.kleppmann.com/papers/move-op.pdf
- Litt, Lim, Kleppmann, van Hardenberg 2022 — *Peritext: A CRDT for Collaborative Rich Text Editing*. PACMHCI 6 CSCW2 art. 531. https://www.inkandswitch.com/peritext/
