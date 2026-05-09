**Date:** 2026-05-09
**Status:** active
**Subject:** chronological lineage of CRDT research and engineering relevant to Automerge, Yjs, and Loro

Two parallel threads run through this history: an academic line (Nancy / Lorraine, KAIST, INRIA, Cambridge) producing the algorithms, and an engineering line (RWTH Aachen i5, Ink & Switch, Loro) productising them. They cross over often.

## 2006 — WOOT

Oster, Urso, Molli, Imine at INRIA / LORIA (Nancy) publish WOOT — *Real time group editors without Operational Transformation*. First text CRDT in the modern sense: each character has a unique id and explicit `(prev, next)` neighbours. Sets the template that RGA, YATA, and Fugue all extend.

## 2009 — Treedoc, Logoot, RGA precursor

ICDCS 2009 is the year the text-CRDT line gets its main competitors:

- Preguiça, Marquès, Shapiro, Letia publish **Treedoc**. Position = path in a dense binary tree.
- Weiss, Urso, Molli publish **Logoot**. Position = list of integers in a dense order; no tombstones.
- Roh, Kim, Lee, Maeng release the **RGA** technical report at KAIST (CS-TR-2009-318). Linked-list-of-chars with s4vector timestamps.

These three propose three different ways to make insert positions globally unique without coordination. The taxonomy of "where does the position id come from" — tree, dense list, lookup table — is set here.

## 2011 — the Shapiro report; "CRDT" as a term

Shapiro, Preguiça, Baquero, Zawirski publish *A Comprehensive Study of Convergent and Commutative Replicated Data Types* as INRIA Research Report RR-7506, with a companion paper at SSS 2011 (*Conflict-free Replicated Data Types*). The taxonomy CvRDT vs CmRDT is formalised; the term "CRDT" enters general use. Same year, Roh, Jeon, Kim, Lee publish the journal version of RGA in JPDC 71(3) — *Replicated abstract data types: Building blocks for collaborative applications*.

This is the field's foundational year. Anything written after 2011 cites the Shapiro report.

## 2014 — Yjs starts

Kevin Jahns, then a CS student and HiWi at the Lehrstuhl für Informatik 5 (i5, Information Systems & Databases) at RWTH Aachen University, starts work on Yjs. Early prototypes use a CmRDT-style framework; the algorithm that becomes YATA is being designed here. Yjs is presented at FOSDEM 2015.

## 2015 — Yjs at ICWE

Nicolaescu, Jahns, Derntl, Klamma publish *Yjs: A Framework for Near Real-Time P2P Shared Editing on Arbitrary Data Types* at ICWE 2015 (Engineering the Web in the Big Data Era, Springer LNCS). First academic publication of Yjs; the algorithm is still pre-YATA in the final form.

## 2016 — YATA

Same authors (Nicolaescu, Jahns, Derntl, Klamma) publish *Near Real-Time Peer-to-Peer Shared Editing on Extensible Data Types* at ACM GROUP 2016 (Sanibel Island, Florida, November 13-16). This is the YATA paper proper. The algorithm extends RGA-style left-origin with an explicit right-origin pointer for tighter conflict resolution. Yjs ships YATA in production from this point.

## 2017 — Automerge starts

Martin Kleppmann (Cambridge) collaborates with Ink & Switch (Peter van Hardenberg, Adam Wiggins, Mark McGranaghan) to build a JavaScript JSON CRDT library based on his earlier research. The result is Automerge. Initial implementation is TypeScript; Ink & Switch becomes the institutional home.

## 2018-2019 — local-first

Automerge gains traction. Hypermerge (Automerge over the Dat networking stack) ships as Ink & Switch's reference networking layer. October 2019, Kleppmann, Wiggins, van Hardenberg, McGranaghan publish *Local-first software: You own your data, in spite of the cloud* at Onward! 2019 (DOI 10.1145/3359591.3359737). The essay coins "local-first" as a category and gives Automerge its product framing. The current Automerge website still traces the project's pedigree to this paper.

## 2020 — Automerge Rust port begins

Ink & Switch funds a 6-week sprint to port Automerge to Rust (`automerge-rs`). Goal: WASM target for the JS bindings, native target for desktop and mobile. Performance is poor in the JS-only version; the Rust core fixes that. This becomes the foundation of Automerge 2.x.

## 2021 — yrs starts; move-tree paper

- Bartosz Sypytkowski (and others) start `y-crdt` / `yrs`, a Rust port of Yjs aiming for binary-protocol compatibility with the JS implementation.
- Kleppmann, Mulligan, Gomes, Beresford publish *A Highly-Available Move Operation for Replicated Trees* in IEEE TPDS 33(7):1711-1724 (October 2021). Solves the concurrent-move-causes-cycle problem with an undo-redo-by-Lamport-order scheme; demos real bugs in Google Drive and Dropbox.
- Late 2021, the Peritext essay drops on inkandswitch.com (Litt, Lim, Kleppmann, van Hardenberg). Rich-text CRDTs become a discoverable category for app builders.

## 2022 — Loro starts; Peritext paper

- July 2022 the `loro-dev/loro` repository is created by Zixuan Chen and collaborators. From the start the design pulls from Automerge (columnar encoding), Yjs (merging algorithm), and Joseph Gentle's diamond-types Event Graph Walker. Initially closed-source, open-sourced 2023.
- November 2022, Litt, Lim, Kleppmann, van Hardenberg publish *Peritext: A CRDT for Collaborative Rich Text Editing* at ACM CSCW (PACMHCI 6 CSCW2 art. 531). Formal version of the 2021 essay.

## 2023 — Fugue; Automerge 2.0

- Automerge 2.0 ships in January 2023 — first release with the Rust core and JS/WASM wrappers as the production path.
- April 2023, Weidner & Kleppmann publish *The Art of the Fugue: Minimizing Interleaving in Collaborative Text Editing* (arXiv:2305.00583). FugueMax proved to satisfy maximal non-interleaving. Loro adopts Fugue for text; Yjs follow-up work integrates Fugue ideas.
- November 2023, Loro is open-sourced.

## 2024 — Eg-walker

September 2024, Gentle and Kleppmann publish *Collaborative Text Editing with Eg-walker: Better, Faster, Smaller* (arXiv:2409.14252). Argument: a non-CRDT replicated data structure (event graph + OT-style replay) can match CRDT correctness while using order-of-magnitude less steady-state memory and loading orders of magnitude faster from disk. Diamond-types is the reference implementation. Kleppmann explicitly frames this as "maybe pure CRDT was not the destination". Published EuroSys 2025.

## 2025-2026 — current state

- **Automerge 3.x stable** with the Rust core; default in production deployments.
- **Yjs v14 RC** lineage (npm `next` 14.0.0-8, `beta` 14.0.0-16; GitHub release tags including `v14.0.0-rc.13` 2026-04-14 have no matching npm publish); YATA + Fugue-flavoured improvements; yrs maintained in parallel for non-JS hosts.
- **Loro 1.x stable** as of 2025; movable tree, Peritext-on-Fugue rich text, event-graph-walker-influenced encoding.
- The discourse has split: "classical CRDT" (Automerge, Yjs in YATA mode) vs "event-graph hybrid" (Loro, diamond-types, Eg-walker). Both converge; the difference is engineering tradeoffs.

## Reading order for Myrhiza

If you are designing `state-apply` convergence semantics, read in this order:

1. Shapiro et al. 2011 INRIA RR-7506 — the vocabulary. Sections 1-3 only; the catalogue chapter is reference.
2. The local-first essay (Kleppmann et al. 2019) — frames why we are doing this at all.
3. Kleppmann et al. 2021 *Move Operation for Replicated Trees* — short, surgical, shows what "deterministic merge" looks like under pressure. Anything we build with tree-shaped state must absorb this.
4. Nicolaescu et al. 2016 *YATA* — the simplest production-grade text CRDT to read end-to-end.
5. Litt et al. 2022 *Peritext* — only if rich text matters for the relevant `state-apply`.
6. Weidner & Kleppmann 2023 *Fugue* — the interleaving correctness story; understand maximal non-interleaving.
7. Gentle & Kleppmann 2024 *Eg-walker* — for the "is the in-memory CRDT actually load-bearing or just a cache" question that affects how Myrhiza's kernel materialises state from the event log.

Skip on first pass: Treedoc, Logoot, WOOT — historical; their failure modes are the motivation for everything later but you do not need the algorithms.

## Sources

- Oster et al. 2006 — *Real time group editors without OT* (WOOT). https://inria.hal.science/inria-00071240
- Preguiça et al. 2009 — *Treedoc*. ICDCS 2009. https://inria.hal.science/inria-00445975
- Weiss et al. 2009 — *Logoot*. ICDCS 2009. https://inria.hal.science/inria-00432368
- Roh et al. 2011 — *Replicated abstract data types* (RGA). JPDC 71(3). http://csl.skku.edu/papers/jpdc11.pdf
- Shapiro et al. 2011 — *A Comprehensive Study of CRDTs*. INRIA RR-7506. https://inria.hal.science/inria-00555588
- Nicolaescu, Jahns, Derntl, Klamma 2015 — *Yjs: A Framework for Near Real-Time P2P Shared Editing*. ICWE 2015. https://link.springer.com/chapter/10.1007/978-3-319-19890-3_55
- Nicolaescu et al. 2016 — *Near Real-Time P2P Shared Editing on Extensible Data Types* (YATA). ACM GROUP 2016. https://dl.acm.org/doi/10.1145/2957276.2957310
- Kleppmann, Wiggins, van Hardenberg, McGranaghan 2019 — *Local-first software*. Onward! 2019. https://www.inkandswitch.com/essay/local-first/
- Kleppmann, Mulligan, Gomes, Beresford 2021 — *A Highly-Available Move Operation for Replicated Trees*. IEEE TPDS 33(7). https://martin.kleppmann.com/papers/move-op.pdf
- Litt, Lim, Kleppmann, van Hardenberg 2022 — *Peritext*. PACMHCI 6 CSCW2 art. 531. https://www.inkandswitch.com/peritext/
- Weidner & Kleppmann 2023 — *The Art of the Fugue*. arXiv:2305.00583. https://arxiv.org/abs/2305.00583
- Gentle, Kleppmann 2024 — *Eg-walker*. arXiv:2409.14252. https://arxiv.org/abs/2409.14252
- Yjs repo — https://github.com/yjs/yjs
- Automerge repo + 2.0 announcement — https://automerge.org/blog/automerge-2/
- Loro repo + about page — https://github.com/loro-dev/loro , https://www.loro.dev/about
