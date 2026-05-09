**Date:** 2026-05-09
**Status:** active
**Subject:** Curated index of papers + talks that anchor Myrhiza's design space — local-first software, ocap, actor model, deterministic VMs, CRDTs, WASM Component Model

# Local-first + foundational references

Index of *external papers and talks* that anchor Myrhiza's design space. Companion to the [`prior-art/`](../prior-art/) deep-dive folders. Where a topic has its own folder, this file points there with a one-line framing; where it doesn't, the canonical source is linked directly.

This file exists so future Myrhiza spec authors don't repeatedly relitigate which papers underpin the design. Read in declared order; skim what you already know.

## How this differs from `prior-art/`

- **`prior-art/<system>/`** = deep-dive on an external project we are learning from. Multi-file folder.
- **`references/<topic>.md`** = single-file curated index of papers and talks anchoring a topic. No deep dive — links + 1-3 sentence framing per item.

This file is the first reference document in the corpus. Add others (e.g. `references/wasm-component-model.md`, `references/capabilities.md`) only when a topic crosses the threshold of "spec authors keep asking the same question."

## 1. The Local-First essay (2019)

**Kleppmann, Wiggins, van Hardenberg, McGranaghan** — *Local-First Software: You Own Your Data, in spite of the Cloud* — Onward! 2019. <https://www.inkandswitch.com/essay/local-first/>

**Why read it.** Defines the design space Myrhiza is in. Articulates seven properties of "local-first software": no spinners, work isn't trapped on one device, network is optional, supports collaboration, longevity, privacy + security by default, ownership. Frames why centralized SaaS is structurally hostile to user agency.

**Where it falls short.** The essay's CRDT-as-substrate framing is now contested by Kleppmann's own [Eg-walker work (2024)](#42-eg-walker-non-crdt-replicated-data) — replicated data structures don't *have* to be CRDTs. Read the local-first essay first, then the Eg-walker paper, to track the evolution.

**Cross-reference.** [`../prior-art/crdts/`](../prior-art/crdts/), [`../prior-art/agoric-endo/`](../prior-art/agoric-endo/), [`../prior-art/holochain/`](../prior-art/holochain/) — all three are local-first runtimes.

## 2. Object capabilities & the ocap lineage

### 2.1 Mark Miller's thesis (2006)

**Mark Samuel Miller** — *Robust Composition: Towards a Unified Approach to Access Control and Concurrency Control* — PhD dissertation, Johns Hopkins University, May 2006. Advisor: Jonathan S. Shapiro. <http://www.erights.org/talks/thesis/markm-thesis.pdf>

**Why read it.** The canonical text on object-capability discipline. Introduces the "ocap" terminology, defines E as a working ocap language, describes CapDesk as a virus-safe desktop. The thesis that informed all subsequent ocap work — Spritely Goblins, Agoric Endo, Cap'n Proto.

**Where it falls short.** Long (377 pages); the first 6 chapters are the canonical reading; the rest is implementation-of-E details that Spritely + Agoric have superseded.

**Cross-reference.** [`../prior-art/spritely-ocapn/`](../prior-art/spritely-ocapn/), [`../prior-art/agoric-endo/`](../prior-art/agoric-endo/) — both are direct descendants.

### 2.2 Hewitt actor model (1973)

**Hewitt, Bishop, Steiger** — *A Universal Modular ACTOR Formalism for Artificial Intelligence* — IJCAI 1973.

**Why read it.** Foundational actor model. Concurrency primitive that informs Erlang, Akka, JavaScript event loops, and (indirectly) the deterministic-replay actor patterns in Croquet/Multisynq, Agoric SwingSet, and Spritely Goblins.

**Where it falls short.** Period-specific notation; the conceptual content is what matters, not the formalism details. Skim.

## 3. Distributed-systems primitives

### 3.1 Lamport, time and ordering (1978)

**Leslie Lamport** — *Time, Clocks, and the Ordering of Events in a Distributed System* — CACM 1978. <https://lamport.azurewebsites.net/pubs/time-clocks.pdf>

**Why read it.** The Lamport timestamp. The vector-clock/version-vector lineage. Every modern P2P runtime — CRDTs (Yjs's `clientID`/`clock`, Automerge's actor IDs), Hypercore (signed log + Lamport), Holochain (per-agent source chain) — uses primitives traceable to this paper.

### 3.2 Shapiro et al. CRDT survey (2011)

**Shapiro, Preguiça, Baquero, Zawirski** — *A Comprehensive Study of Convergent and Commutative Replicated Data Types* — INRIA RR-7506. <https://hal.inria.fr/inria-00555588>

**Why read it.** The CRDT canon. Defines CvRDT vs CmRDT taxonomy; surveys all the foundational types (counters, sets, maps, lists). Coined "CRDT" as a term.

**Cross-reference.** [`../prior-art/crdts/crdt-theory.md`](../prior-art/crdts/crdt-theory.md) — the family-tree synthesis built on this paper.

## 4. Convergence-paradigm anchor papers

Myrhiza's `state-apply` purity requirement has four viable cross-peer convergence paradigms (see [`../prior-art/croquet/lessons.md`](../prior-art/croquet/lessons.md) for the four-pattern matrix). One canonical paper per paradigm:

### 4.1 Lockstep deterministic VM (Croquet)

**Smith, Kay, Raab, Reed** — *Croquet: A Collaboration System Architecture* — C5 2003. (Common citation error: this was C5 2003, not OOPSLA.)

**Why read it.** Original presentation of TeaTime — the deterministic-VM-with-reflector-ordering paradigm. The single best reference for "all peers run the same compute on identically-ordered messages."

**Cross-reference.** [`../prior-art/croquet/`](../prior-art/croquet/) — the modern Multisynq stack, which descends from this paper.

### 4.2 Eg-walker (non-CRDT replicated data)

**Gentle, Kleppmann** — *Collaborative Text Editing with Eg-walker: Better, Faster, Smaller* — arXiv:2409.14252, EuroSys 2025. <https://arxiv.org/abs/2409.14252>

**Why read it.** Kleppmann's argument that *replicated data structures need not be CRDTs* — that an event-graph + OT-style replay can outperform CRDTs while preserving correctness. This paper recasts the entire local-first design space.

**Cross-reference.** [`../prior-art/crdts/critiques.md`](../prior-art/crdts/critiques.md) — quotes the Eg-walker pivot as the single most important critique of CRDT orthodoxy.

### 4.3 Move-tree (Kleppmann et al. 2021)

**Kleppmann, Mulligan, Gomes, Beresford** — *A Highly-Available Move Operation for Replicated Trees* — IEEE TPDS 33(7), 2022. <https://martin.kleppmann.com/papers/move-op.pdf>

**Why read it.** Short, surgical demonstration of "what 'deterministic merge' looks like under pressure." Solves the cycle-and-loss problem of naive concurrent move operations on trees. Isabelle/HOL-verified. Anything Myrhiza ships with tree-shaped state must absorb this.

**Cross-reference.** [`../prior-art/crdts/loro.md`](../prior-art/crdts/loro.md) — Loro implements this algorithm.

### 4.4 YATA (the simplest production text CRDT)

**Nicolaescu, Jahns, Derntl, Klamma** — *Near Real-Time Peer-to-Peer Shared Editing on Extensible Data Types* — ACM GROUP 2016. <https://dl.acm.org/doi/10.1145/2957276.2957310>

**Why read it.** YATA — the algorithm behind Yjs. The simplest production-grade text CRDT that's readable end-to-end in one sitting. If you want to understand how text CRDTs work *in practice*, this is the paper.

**Cross-reference.** [`../prior-art/crdts/yjs.md`](../prior-art/crdts/yjs.md).

### 4.5 Peritext (rich text CRDT)

**Litt, Lim, Kleppmann, van Hardenberg** — *Peritext: A CRDT for Collaborative Rich Text Editing* — Ink & Switch essay 2021; PACMHCI 6 CSCW2 art. 531 (formal version, 2022). <https://www.inkandswitch.com/peritext/>

**Why read it.** The only published correct algorithm for rich-text formatting (bold, italic, links) that preserves formatting intent across concurrent edits. If any Myrhiza app surfaces rich text, this is the algorithm.

**Cross-reference.** [`../prior-art/crdts/automerge.md`](../prior-art/crdts/automerge.md), [`../prior-art/crdts/loro.md`](../prior-art/crdts/loro.md).

### 4.6 Fugue (maximal non-interleaving)

**Weidner, Kleppmann** — *The Art of the Fugue: Minimizing Interleaving in Collaborative Text Editing* — arXiv:2305.00583, 2023. <https://arxiv.org/abs/2305.00583>

**Why read it.** The interleaving correctness story. Proves that FugueMax achieves *maximal non-interleaving* — concurrent text-runs stay contiguous in the merged output. Loro adopts Fugue for text.

## 5. Group key agreement

### 5.1 RFC 9420 — MLS Protocol

**Barnes, Beurdouche, Robert, Millican, Omara, Cohn-Gordon** — *The Messaging Layer Security (MLS) Protocol* — RFC 9420, July 2023. <https://www.rfc-editor.org/rfc/rfc9420.html>

**Why read it.** IETF Standards Track. Asynchronous group key agreement with forward secrecy and post-compromise security. The reference cryptographic primitive if Myrhiza grows multi-party room-shaped capabilities.

**Cross-reference.** [`../prior-art/mls/`](../prior-art/mls/) — full deep dive.

### 5.2 ETK 2025 finding (load-bearing critique)

**Cremers, Günther, Wallez, Zhao** — *ETK: External-Operations TreeKEM and the Security of MLS in RFC 9420* — IACR ePrint 2025/229. <https://eprint.iacr.org/2025/229.pdf>

**Why read it.** Proves MLS *fails* the FCGKA security goal it claims when used with EUF-CMA-only signatures (e.g. ECDSA). Published-RFC-level finding. If Myrhiza adopts MLS, use Ed25519 (which is SUF-CMA), not ECDSA.

## 6. WASM Component Model lineage

The Component Model has its own folder; one canonical paper-shaped reference here:

### 6.1 Lin Clark Component Model talks

**Lin Clark** — *The Wasm Component Model* — Lin Clark's blog series at [bytecodealliance.org](https://bytecodealliance.org/articles/wasi-preview-2-now-available-in-wasmtime).

**Why read it.** Lin Clark is the most accessible explainer of the Component Model's design rationale. Her cartoon-illustrated essays are the canonical introduction.

**Cross-reference.** [`../prior-art/wasm-component-model/`](../prior-art/wasm-component-model/) — full deep dive on the substrate.

## 7. Talks worth watching

These are linked, not deep-dived. Watch when you have an hour.

### 7.1 Christopher Lemmer Webber on OCapN

Multiple talks at Strange Loop, Spritely-led events, and Pivot Tracker. Search "Christopher Lemmer Webber OCapN" on YouTube. Treats the cross-implementation distributed object-capability protocol — what Spritely + Agoric + MetaMask + Cap'n Proto are converging on.

**Cross-reference.** [`../prior-art/spritely-ocapn/`](../prior-art/spritely-ocapn/).

### 7.2 Mathias Buus on Hypercore

Multiple talks at Dat Conference, Holepunch events. Search "Mathias Buus Hypercore" on YouTube. The most accessible introduction to append-only signed log architecture for P2P apps.

**Cross-reference.** [`../prior-art/pears/hypercore-stack.md`](../prior-art/pears/hypercore-stack.md).

### 7.3 David A. Smith on Multisynq / Croquet

David A. Smith's Discord and X presence (`@gocroquet`) hosts demos and talks on the lockstep deterministic VM paradigm. Search "David A Smith Multisynq" on YouTube.

**Cross-reference.** [`../prior-art/croquet/`](../prior-art/croquet/).

### 7.4 Glenn Fiedler on game-engine lockstep

**Glenn Fiedler** — *Networked Physics* and *Deterministic Lockstep* essays at <https://gafferongames.com>.

**Why watch / read.** Game-dev has decades of experience with lockstep determinism — most published critiques of the paradigm originate here. Glenn Fiedler's essays are the canonical accessible source.

**Cross-reference.** [`../prior-art/croquet/critiques.md`](../prior-art/croquet/critiques.md) §5 quotes Fiedler.

## 8. What's deliberately not in this index

- **Implementation tutorials** (e.g. "how to write a Yjs provider"). Those live in upstream docs; Myrhiza's spec authors don't need a curated link.
- **Comparison articles by random bloggers.** Quote in [`../prior-art/<system>/critiques.md`](../prior-art/) when substantive; don't elevate to top-level reference.
- **Pre-2010 academic CRDT lineage** (Treedoc, Logoot, WOOT). [`../prior-art/crdts/crdt-theory.md`](../prior-art/crdts/crdt-theory.md) covers these in depth; only the Shapiro 2011 survey is general enough to belong here.
- **WASM substrate papers** — Andreas Rossberg's WebAssembly papers, etc. The Component Model folder ([`../prior-art/wasm-component-model/`](../prior-art/wasm-component-model/)) covers the substrate; this index would only duplicate.

## 9. Reading order for a new Myrhiza spec author

If you've never read any of this and you're about to write a Myrhiza spec, read in this order:

1. **§1 — Local-First essay (Kleppmann et al. 2019)** — frames why we are doing this.
2. **§3.1 — Lamport 1978** — the time/ordering primitives all P2P protocols use.
3. **§3.2 — Shapiro 2011 CRDT survey** — the vocabulary for talking about replicated data.
4. **§2.1 — Mark Miller thesis chapters 1-6** — ocap discipline. Skip the implementation details.
5. **§4.1 — Smith, Kay et al. 2003** — lockstep determinism paradigm.
6. **§4.3 — Kleppmann 2021 move-tree** — short, surgical, what deterministic merge actually looks like.
7. **§4.2 — Gentle & Kleppmann 2024 Eg-walker** — the "is in-memory CRDT actually load-bearing or just a cache" question.
8. **§5.1 — RFC 9420 MLS** *only if* designing group capabilities.
9. **§6.1 — Lin Clark Component Model talks** — the WASM substrate.
10. **§7 talks** — when you want narrative + intuition.

Skip on first pass: §3.2 details if you've read CRDTs deep dive; §5 if not designing group caps; §7.4 if not weighing lockstep vs alternatives.

## Sources

All URLs cited inline above. This file is itself an index, not a synthesis — sources live next to claims.
