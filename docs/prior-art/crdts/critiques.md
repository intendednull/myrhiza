**Date:** 2026-05-09
**Status:** active
**Subject:** Third-party criticism of CRDTs and of specific libraries (Automerge / Yjs / Loro). Quoted verbatim where possible. Source URL + author + approximate date for every quote.

This file exists so Myrhiza spec authors can read the strongest arguments *against* the path before committing to a CRDT-based `state-apply`.

## 1. Kleppmann's own pivot: Eg-walker (2024–25)

Martin Kleppmann is the author of Automerge and the most-cited CRDT researcher of the last decade. His 2024 eg-walker paper is the single sharpest critique of CRDTs by a CRDT person.

> "Existing algorithms fall in two categories: Operational Transformation (OT) algorithms are slow to merge files that have diverged substantially due to offline editing; CRDTs are slow to load and consume a lot of memory."
>
> "Compared to existing CRDTs, [Eg-walker] consumes an order of magnitude less memory in the steady state, and loading a document from disk is orders of magnitude faster."

Source: Joseph Gentle and Martin Kleppmann, *Collaborative Text Editing with Eg-walker: Better, Faster, Smaller*, EuroSys 2025. https://arxiv.org/abs/2409.14252 — published Sept 2024, presented April 2025.

Kleppmann's blog post version: https://martin.kleppmann.com/2025/03/30/eg-walker-collaborative-text.html

The takeaway is not "CRDTs are wrong" but "CRDTs as currently implemented impose costs that may not be necessary." Eg-walker is technically a CRDT but replays operations in a way that produces OT-equivalent behavior — a hybrid.

## 2. Joseph Gentle on academic CRDT implementations

Gentle (author of ShareJS, ex-Google Wave) has a long blog history. From *CRDTs go brrr* (https://josephg.com/blog/crdts-go-brrr/, 2021):

> "most CRDTs you read about in academic papers are crazy slow"

> "I was reading papers which described the *behaviour* of different systems. And I assumed that meant we knew how the best way to *implement* those systems. And wow, I was super wrong."

> "Automerge treats each inserted character as a separate item." [Critiquing the encoding overhead.]

> "A slow implementation suggests, but can never prove that every implementation of the system will be slow."

In *I was wrong. CRDTs are the future* (https://josephg.com/blog/crdts-are-the-future/, 2020), Gentle reverses an earlier dismissive position after seeing Kleppmann's optimized approach. Both posts read together = "the algorithms work; the typical implementations leave 100x on the table."

## 3. Marijn Haverbeke (ProseMirror, CodeMirror) on CRDTs vs OT

From discussion threads on `discuss.prosemirror.net` and `marijnhaverbeke.nl`:

Haverbeke describes OT as **"such a hack"** — paraphrased: when document structure is more complicated than plain text and changes are more complicated than insert/delete, defining a convergent transformation function gets hard, and you end up storing tombstones for deletion locations to guarantee convergence anyway. (Source: https://marijnhaverbeke.nl/blog/collaborative-editing-cm.html)

On CRDTs (paraphrased from his discuss.prosemirror.net posts): CRDTs provide a way for developers to *reason about* convergence, contrary to OT. The main problem with OT is that it's hard to reason about — many practical advantages but, being a hack, doesn't provide a mental framework you can confidently apply in different situations.

Quote on CodeMirror's choice — Jahns reports Haverbeke's position as: CRDTs impose **"a too significant overhead"** for CodeMirror. (Cited by Jahns at https://blog.kevinjahns.de/are-crdts-suitable-for-shared-editing — paraphrased reply Jahns is responding to.)

## 4. Raph Levien (xi-editor) — "CRDT is not pulling its considerable weight"

From `news.ycombinator.com/item?id=24176455` ("Are CRDTs suitable for shared editing?", 2020):

> "the CRDT constrains the data model considerably... you always have to design those with the CRDT in mind (ie, everything still has to be a monotonic semi-lattice)."
>
> "CRDT merges aren't a good fit for the problems a code editor is trying to solve, particularly when the 'collaborators' are automated processes such as language servers." — raphlinus

The xi-editor project ultimately backed out of its CRDT-based plan. Jahns acknowledges Xi's published conclusion: **"CRDT is not pulling its (considerable) weight."**

## 5. Zoho Writer team (lewisjoe) on CRDTs for structured docs

Same HN thread (https://news.ycombinator.com/item?id=24176455):

> "Memory issues with tombstones. Marking as deletion has a cost of maintaining them throughout the session." — lewisjoe

> "when it comes to schematic JSON that has semantic value CRDT was an added overhead" — lewisjoe, citing collaborative HTML table operations as the failure case.

> "concurrent conflicts in such cases are notoriously hard to converge without contextual special handling" — lewisjoe (https://news.ycombinator.com/item?id=41099901, 2024)

## 6. Aaron Boodman (Replicache, Zero) on server-authority vs decentralized

Boodman's framing on X (https://x.com/aboodman/status/1843045692736204802, Oct 2024):

> "The second dimension to consider is server-authority vs what I will call 'decentralized' here. Examples of server-authority systems are replicache, zero, powersync, electric, convex, instant, triplit, even firebase ... Examples of decentralized systems are yjs, automerge, [loro]"

Replicache is explicitly server-authority. Boodman has not published a single "we rejected CRDTs" essay, but the entire Replicache architecture is the implicit critique: he believes a centralized authority that can validate writes is necessary for most apps, and CRDT-style decentralized convergence is the wrong default. The Replicache design (server is source of truth; clients sync deltas) is incompatible with a pure CRDT model.

For Myrhiza this is the most important external perspective: Boodman is shipping the centralized-authority-with-local-sync alternative at production scale (Linear, etc.). His implicit critique is that the constraint Myrhiza imposes ("apps cannot touch authority directly; the kernel mediates") rhymes with Replicache's centralized validation more than with Automerge's decentralized convergence.

## 7. HN thread on Automerge schema migration (jitl, 2023)

From https://news.ycombinator.com/item?id=38193640:

> "Migrations on CRDTs are challenging, so it's important to 'get it right' at the beginning."

> "However this looks so easy I worry about apps building with too little thought about long term data modeling."

> "If you do a naive migration and change the type of the field in-place, how do you handle updates from old peers doing a LWW set on bio, when now the data type you expect is a Peritext delta?" — jitl

This is the cleanest articulation of the schema-evolution problem (see `open-problems.md` §1).

## 8. Kevin Jahns (Yjs author) admitting the trade-off

From https://blog.kevinjahns.de/are-crdts-suitable-for-shared-editing (2021–22), Jahns defends Yjs but concedes the worst-case:

Worst case: "in the absolutely worst-case scenario" of one million right-to-left insertions, Yjs uses 112 MB and parses in <400 ms. He argues this scenario "doesn't occur naturally." (Paraphrased)

> "It is basically impossible for a human to write a document that Yjs can't handle." (Paraphrased — his summary line.)

This is honest: the bound exists, it's not catastrophic for real users, but it is real.

## 9. Jahns vs Loro (benchmark methodology dispute)

From https://discuss.yjs.dev/t/yjs-vs-loro-new-crdt-lib/2567:

> "I have a bit of a problem with the Loro CRDT as their benchmarks are not reproducible. They don't even publish the source code for the benchmarks." — Jahns

> "The size of the Loro bundle is over 1MB in size, which needs to be base64 encoded if you ship it to the browser (+30% overhead)." — Jahns

> Disabling Yjs garbage collection in comparisons is "unfair, and misleading to the user." — Jahns

Loro author (zxch3n) acknowledged in the same thread that some performance regression between Loro versions came from "breaking changes" prioritizing future-extensibility over throughput.

## 10. Per-library criticism summary

**Yjs single-maintainer concern.** Jahns is the entire critical path. He funds work via support contracts (acknowledged on the Yjs site). The `yrs` Rust port has separate maintainers (Bartosz Sypytkowski et al.) but is not the canonical implementation. No published succession plan. **This is the single most-cited reason production teams cite for not adopting Yjs.** (Sources: various HN threads; Jahns's own funding model on yjs.dev.)

**Automerge document-size growth.** Pre-3.0: gigabytes-of-memory complaints in real apps. The 3.0 release acknowledges this directly — "We've cut memory usage by over 10x in some cases" (https://automerge.org/blog/automerge-3/). Per-character overhead in the encoded format dropped from ~240 bytes/char (early 2.x) to <1 byte/char (3.0).

**Loro maturity.** No production deployments at scale documented. Pre-1.0 had repeated breaking encoding changes. Velt's "Best CRDT Libraries 2025" guide (https://velt.dev/blog/best-crdt-libraries-real-time-data-sync) writes: Loro **"delivers strong performance but requires substantial development work and isn't production-ready."** Take with caveat that Velt is a competitor; nonetheless the production-readiness claim is repeated by independent sources.

## 11. The Figma counter-example

Figma did not adopt a third-party CRDT lib. From their engineering blog (https://www.figma.com/blog/how-figmas-multiplayer-technology-works/):

> "When Figma first started building multiplayer functionality, [we] decided to develop [our] own solution rather than use operational transforms (OTs)... As a startup valuing the ability to ship features quickly, OTs were unnecessarily complex for [our] problem space, so [we] built a custom multiplayer system that's simpler and easier to implement."

They use a "last-writer-wins register" — the simplest CRDT — backed by an authoritative server. **The critique embedded in this choice:** for most apps, you don't need RGA / YATA / Fugue. You need LWW + a server. The complex CRDT machinery exists to handle decentralized peer-to-peer; if you have a server, much of the algorithmic depth is wasted.

Figma's stack is closer to what Myrhiza's `state-apply` needs (authoritative validation, simple convergence) than Automerge/Yjs/Loro's stack.

## 12. Ink & Switch's own "what we got wrong"

Ink & Switch is the institution behind Automerge. They have not published a single "we got CRDTs wrong" essay, but several pieces are honest about open problems:

- *Local-first software* (https://www.inkandswitch.com/local-first/, 2019): names CRDTs as enabling but lists schema migration, partial replication, and access control as unsolved.
- *Cambria* (https://www.inkandswitch.com/cambria/, 2020): explicitly addresses schema evolution as an open problem CRDTs do not solve. The fact that Cambria exists is the admission.
- *Peritext* (https://www.inkandswitch.com/peritext/, 2022): admits existing rich-text CRDTs do not preserve user intent in important cases. The fact that Peritext exists is the admission.

## Sources

- Eg-walker paper: https://arxiv.org/abs/2409.14252
- Kleppmann eg-walker blog: https://martin.kleppmann.com/2025/03/30/eg-walker-collaborative-text.html
- Joseph Gentle, *CRDTs go brrr*: https://josephg.com/blog/crdts-go-brrr/
- Joseph Gentle, *I was wrong. CRDTs are the future*: https://josephg.com/blog/crdts-are-the-future/
- Marijn Haverbeke on CodeMirror collab: https://marijnhaverbeke.nl/blog/collaborative-editing-cm.html
- HN: Are CRDTs suitable for shared editing?: https://news.ycombinator.com/item?id=24176455
- HN: Movable tree CRDTs and Loro's implementation: https://news.ycombinator.com/item?id=41099901
- HN: Loro launch: https://news.ycombinator.com/item?id=38248900
- HN: Automerge-Repo: https://news.ycombinator.com/item?id=38193640
- Aaron Boodman on X: https://x.com/aboodman/status/1843045692736204802
- Replicache: https://replicache.dev/
- Kevin Jahns, Are CRDTs suitable for shared editing?: https://blog.kevinjahns.de/are-crdts-suitable-for-shared-editing
- Yjs vs Loro forum thread: https://discuss.yjs.dev/t/yjs-vs-loro-new-crdt-lib/2567
- Automerge 3.0 release: https://automerge.org/blog/automerge-3/
- Velt CRDT guide: https://velt.dev/blog/best-crdt-libraries-real-time-data-sync
- Figma multiplayer blog: https://www.figma.com/blog/how-figmas-multiplayer-technology-works/
- Ink & Switch local-first: https://www.inkandswitch.com/local-first/
- Ink & Switch Cambria: https://www.inkandswitch.com/cambria/
- Ink & Switch Peritext: https://www.inkandswitch.com/peritext/
- BFT-CRDT (Kleppmann): https://martin.kleppmann.com/papers/bft-crdt-papoc22.pdf
