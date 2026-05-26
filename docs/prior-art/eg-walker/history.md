**Date:** 2026-05-22
**Status:** active
**Subject:** Chronological lineage of eg-walker / diamond-types — Joseph Gentle (ShareJS → CRDTs go brrr → diamond-types → eg-walker paper) and Martin Kleppmann (Automerge → BFT-CRDT → eg-walker)

# History

Eg-walker emerged from two distinct lineages converging in 2024: Joseph Gentle's OT-to-CRDT-implementation engineering work, and Martin Kleppmann's CRDT-theory pivot away from per-character metadata. This file traces both.

## Gentle's lineage: OT → fast CRDT → eg-walker

### 2009-2010: Google Wave era

Joseph Gentle worked on **Google Wave**, Google's first attempt at large-scale collaborative editing. Wave used Operational Transformation (OT) — at the time the only published algorithm that could merge concurrent edits without a CRDT. The OT implementation was central to Wave; Wave's eventual cancellation (2010) is sometimes cited as evidence OT-at-scale is hard.

Gentle's later blog posts ([crdts-go-brrr](https://josephg.com/blog/crdts-go-brrr/)) reference Wave by name:

> "the algorithm we used for Google Wave. The algorithm which - hang on - I knew for a fact didn't take 3 seconds to process large paste events."

### 2011: ShareJS

After Wave, Gentle wrote **ShareJS** — an OT-based JavaScript library for collaborative editing in any web app. ShareJS is the direct ancestor of:

- **ShareDB** — Gentle's "ShareJS is now ShareDB" rewrite, currently at v5.2.2 (2025), 6.5k stars. The ShareJS repo (<https://github.com/share/ShareJS>) has a deprecation notice pointing to ShareDB.
- **DerbyJS** — a web framework built on top of ShareJS (Gentle is a `derbyjs` org member).
- **OTtypes** (`ottypes/json1`, etc.) — typed OT operations Gentle maintains.

This is **the most relevant production OT lineage in open-source**. Gentle shipped OT at scale (Wave) and then re-shipped OT in open form (ShareJS / ShareDB) for a decade.

### 2020-09-26: "I was wrong. CRDTs are the future"

URL: <https://josephg.com/blog/crdts-are-the-future/>

Gentle reverses his OT-defending position. The post acknowledges Kleppmann/Automerge's argument and signals an intent to engage with CRDTs seriously.

### 2021-07-31: "5000x faster CRDTs" + diamond-types `0.1.0`

URL: <https://josephg.com/blog/crdts-go-brrr/>

The seminal optimisation post. Diamond-types `0.1.0` published to crates.io on **2021-07-26** (per [crates.io API](https://crates.io/api/v1/crates/diamond-types)) — five days before the blog post. The post demonstrates the optimisation through diamond-types' implementation.

At this stage, diamond-types is an **optimised RGA-style CRDT**, not the eg-walker algorithm. The blog post's "5000x faster" claim is *against Automerge 2.x*. Yjs is also referenced and credited:

> "Kevin's list representation + insertion approach I describe here makes everything so much faster and simpler."

Gentle credits Kevin Jahns (Yjs) and Martin Kleppmann (Automerge) as foundational.

### 2022-08-25: diamond-types `1.0.0` on crates.io

Per [crates.io API](https://crates.io/api/v1/crates/diamond-types) — the latest and last published version. License changed from `ISC OR Apache-2.0` (in 0.1.0) to `ISC` only. The published artefact is **frozen at 2022-08-25**; all subsequent work happens on master without re-publishing.

### 2022-03-28: First npm wrapper

`diamond-types-web` and `diamond-types-node` both published `0.1.0` on 2022-03-28. Both reach `1.0.2` on 2023-05-15 and have not been republished since. The npm wrappers consume the *pre-eg-walker-paper* version of diamond-types.

### 2024-09-21: arXiv:2409.14252 v1

The eg-walker paper drops on arXiv. Title: *Collaborative Text Editing with Eg-walker: Better, Faster, Smaller*. Authors: Joseph Gentle, Martin Kleppmann. The algorithm is publicly described for the first time.

Diamond-types' master branch transitions toward implementing the paper's algorithm; the in-tree code becomes "diamond-types 2.0.0" (Cargo.toml version on master) but is **not republished to crates.io**.

### 2024-11-30: `egwalker-paper` repo last updated

The paper's reproducibility artefact ([`josephg/egwalker-paper`](https://github.com/josephg/egwalker-paper), 239 commits, Rust) is finalised for the conference submission. Includes:

- Paper source in LaTeX + earlier Typst draft
- Criterion.rs-based benchmarks
- Seven editing traces in `datasets/raw/`
- Comparator implementations: optimised diamond-types, legacy diamond-types CRDT, Automerge, Yjs, Yrs, OT reference

### 2025-03-30 (approx): Kleppmann's blog post

Kleppmann publishes the eg-walker blog post on his personal site. The URL slug used to be `2025/03/30/eg-walker-collaborative-text.html` but I could not verify this URL is currently live during research (got 404 on direct fetch — may have been renamed or relocated). The post is referenced by [`../crdts/critiques.md`](../crdts/critiques.md) §1.

### 2025-03/04: EuroSys 2025 presentation

EuroSys 2025 held in Rotterdam, co-located with ASPLOS 2025. The eg-walker paper is presented and wins the **Gilles Muller Best Artifact Award** (per Kleppmann's homepage publications listing).

### 2025-05-27: `eg-walker-reference` updated

The pedagogical TypeScript implementation ([`josephg/eg-walker-reference`](https://github.com/josephg/eg-walker-reference)) is updated. 174 stars, designed for conformance testing against diamond-types. Acts as the readable companion to the paper.

### 2025-2026: ongoing

Diamond-types' master continues to evolve; the `more_types` branch explores JSON/list/map extensions; `eg-walker-reference` continues conformance testing. **No `crates.io` re-publish.** The community has begun spinning up alternative implementations: Go ports (`go-eg-walker`, `kevinxiao27/eg-walker`), MoonBit ports (`mizchi/converge`), more TypeScript ports.

## Kleppmann's lineage: Automerge → BFT-CRDT → eg-walker

### 2017-2024: Automerge

Martin Kleppmann co-founded Automerge as the canonical "Local-First" CRDT library. The 2019 *Local-First Software* essay (Kleppmann/Wiggins/van Hardenberg/McGranaghan, <https://www.inkandswitch.com/local-first/>) framed CRDTs as the *enabling substrate* for local-first apps.

### 2021-09: BFT-CRDT paper

Kleppmann, *Making CRDTs Byzantine Fault Tolerant* (PaPoC 2022, <https://martin.kleppmann.com/papers/bft-crdt-papoc22.pdf>). Acknowledges that mainstream CRDTs have no authority enforcement and explores adding it. This is the authority-side parallel work; eg-walker does not fold it in.

### 2021: Move-op tree CRDT

Kleppmann/Mulligan/Gomes/Beresford, *A Highly-Available Move Operation for Replicated Trees* (<https://martin.kleppmann.com/papers/move-op.pdf>). Loro implements this algorithm. The "what merge correctness looks like under stress" template.

### 2022: Peritext

Litt/Lim/Kleppmann/van Hardenberg, *Peritext* (<https://www.inkandswitch.com/peritext/>). The rich-text-intent-preservation move; precedes Fugue. Eg-walker does not address rich text.

### 2023-05: Fugue

Weidner & Kleppmann, *The Art of the Fugue* (<https://arxiv.org/abs/2305.00583>). The maximal-non-interleaving tie-break that eg-walker adopts for concurrent inserts. Without Fugue, eg-walker's tie-break would be RGA-style and produce interleaving artefacts.

### 2024-09: Eg-walker paper

The Gentle-Kleppmann collaboration. From Kleppmann's perspective, the paper is the culmination of: *CRDT theory is sound; the storage form CRDTs adopted is wasteful; here's an algorithm that keeps the theory and replaces the storage*.

## Funding lineage

Diamond-types' README states the project is funded by the **Invisible College** — a research/funding entity whose connection to Joseph Gentle's other work I was not able to fully characterise during this research session. The Invisible College's website (`invisible.college`) returned no extractable content; treat the funding source as named-but-not-deep-investigated.

Kleppmann is a tenured Associate Professor at the University of Cambridge; his eg-walker work is part of his academic research portfolio.

There is **no Rocicorp / Replicache funding involvement** that I could verify; the user-facing brief flagged this as a possible angle, and the verification turned up no evidence (Gentle's GitHub organisations are codeparty, share, derbyjs, ottypes — not rocicorp). If a future session can connect Gentle to Rocicorp definitively, this file should be updated.

## Why the lineage matters for Myrhiza

The historical context tells you what eg-walker is and isn't:

- **It's not a "from-scratch new algorithm."** It's the convergence of two long lineages: Gentle's *make-CRDTs-fast* engineering, and Kleppmann's *CRDT-theory-is-fine-but-storage-isn't* theoretical work.
- **It's not academic-only.** Diamond-types ships. The TypeScript reference ships. The benchmarks are reproducible (Best Artifact Award).
- **It's not production-hardened.** No flagship app. Diamond-types crates.io is three years stale. The ecosystem is one Rust crate, one TS reference, a few experimental ports.

For Myrhiza, this maps cleanly: the algorithm is solid enough to learn from, the implementation is too thin to vendor as-is. Borrow the algorithm; consider re-implementing or contributing to diamond-types if the team needs production-quality bindings.

## Sources

- diamond-types crates.io history: <https://crates.io/api/v1/crates/diamond-types>
- npm registry for `diamond-types-web` / `-node`: <https://registry.npmjs.org/diamond-types-web>, <https://registry.npmjs.org/diamond-types-node>
- ShareJS repo: <https://github.com/share/ShareJS>
- ShareDB repo: <https://github.com/share/sharedb>
- Joseph Gentle's blog: <https://josephg.com/blog/>
- Kleppmann homepage publications: <https://martin.kleppmann.com/>
- arXiv paper: <https://arxiv.org/abs/2409.14252>
- EuroSys 2025: <https://2025.eurosys.org/>
- Eg-walker paper artefacts repo: <https://github.com/josephg/egwalker-paper>
- Eg-walker reference repo: <https://github.com/josephg/eg-walker-reference>
- Automerge: <https://automerge.org>
- BFT-CRDT paper: <https://martin.kleppmann.com/papers/bft-crdt-papoc22.pdf>
- Local-First Software essay: <https://www.inkandswitch.com/local-first/>
- Fugue paper: <https://arxiv.org/abs/2305.00583>
