**Date:** 2026-05-22
**Status:** active
**Subject:** Third-party voices on eg-walker / diamond-types. Verbatim quotes. The paper's own honesty about limits.

# Critiques

This file collects external voices — Kleppmann's own pivot framing, Gentle's earlier blog posts that motivated the algorithm, HN reception, and the paper's own §7 limitations. Quoted verbatim where possible; paraphrase flagged.

## 1. The paper's own framing (verbatim from abstract)

> "Collaborative text editing algorithms allow several users to concurrently modify a text file, and automatically merge concurrent edits into a consistent state."
>
> "Compared to existing CRDTs, [Eg-walker] consumes an order of magnitude less memory in the steady state, and loading a document from disk is orders of magnitude faster."
>
> "Compared to OT, merging long-running branches is orders of magnitude faster. In the worst case, the merging performance of Eg-walker is comparable with existing CRDT algorithms."
>
> "Eg-walker can be used everywhere CRDTs are used, including peer-to-peer systems without a central server."

Source: <https://arxiv.org/abs/2409.14252>. The "worst case… comparable" line is important — the paper does not claim eg-walker is universally faster. It claims a memory win at rest and a load-time win; for the merge step itself the floor is the same as YATA/RGA.

## 2. Kleppmann's pivot

The most-cited CRDT researcher of the last decade arguing that *the persistent-CRDT-at-rest pattern is mostly a cache, not load-bearing*.

From Kleppmann's homepage publications listing (paraphrase against <https://martin.kleppmann.com/>):

> "[*Collaborative Text Editing with Eg-walker: Better, Faster, Smaller*] — Joseph Gentle, Martin Kleppmann. EuroSys 2025. Best Artifact Award (Gilles Muller Award)."

The fact that this is from the author of Automerge — and that the paper benchmarks Automerge as a comparator and shows substantial memory wins over it — is itself the critique. Kleppmann did not write a "Automerge was wrong" essay; the paper is the essay.

From [`../crdts/critiques.md` §1](../crdts/critiques.md):

> "The takeaway is not 'CRDTs are wrong' but 'CRDTs as currently implemented impose costs that may not be necessary.' Eg-walker is technically a CRDT but replays operations in a way that produces OT-equivalent behavior — a hybrid."

That synthesis (from the crdts/ folder) is the right framing. Eg-walker is not a rejection of CRDT theory; it's a rejection of the *encoding pattern* CRDT libraries inherited from the academic literature.

## 3. Joseph Gentle's lineage

Gentle's blog history is the second source of critique — most of his older posts critique CRDTs as implemented before he built eg-walker.

### *I was wrong. CRDTs are the future* (Gentle, 2020-09-26)

URL: <https://josephg.com/blog/crdts-are-the-future/>

Gentle (ShareJS author, ex-Google Wave) reverses his earlier OT-defending position after seeing Kleppmann's work. The post is short; the relevant pattern is **someone who shipped a production OT system publicly conceding OT is structurally inferior**.

### *5000x faster CRDTs: An adventure in optimization* (Gentle, 2021-07-31)

URL: <https://josephg.com/blog/crdts-go-brrr/>

Already extensively quoted in [`../crdts/critiques.md` §2](../crdts/critiques.md). Re-quoting the most load-bearing line for this folder:

> "most CRDTs you read about in academic papers are crazy slow"

> "I was reading papers which described the *behaviour* of different systems. And I assumed that meant we knew how the best way to *implement* those systems. And wow, I was super wrong."

> "A slow implementation suggests, but can never prove that every implementation of the system will be slow."

> "[diamond-types is] processing the same editing trace in 56 milliseconds. Thats 0.056 seconds, which is over 5000x faster."

These quotes pre-date the eg-walker paper by ~3 years. They describe diamond-types-as-a-fast-CRDT (Stage 2 per [diamond-types.md](diamond-types.md)). The paper's algorithmic move came later. The blog post is the *motivational* lineage; the paper is the *algorithmic* output.

### *Rewriting Rust* (Gentle, 2024-09-26)

URL: <https://josephg.com/blog/rewriting-rust/>

Coincides with the eg-walker arXiv pre-print (2024-09-21). Not directly about eg-walker; included here because it dates Gentle's active work in the same week as the paper drop, demonstrating engagement.

## 4. The Best Artifact Award (EuroSys 2025)

The paper won the **Gilles Muller Best Artifact Award at EuroSys 2025** (per Kleppmann's homepage). This is an artefact-evaluation award — the committee verified:

- The paper's claims reproduce against the published benchmarks.
- The benchmarks ([`egwalker-paper` repo](https://github.com/josephg/egwalker-paper)) run end-to-end.
- The published data + code is sufficient for independent reproduction.

This is **the strongest external validation eg-walker has received**. It is not a Best Paper award (the algorithm's theoretical novelty), but it is the highest form of "we checked, the claims hold" the systems community awards. Take it more seriously than the headline numbers because the committee verified the headline numbers.

## 5. HN reception

The eg-walker pre-print was discussed on Hacker News around the arXiv-v1 drop (2024-09-21) and again around the EuroSys presentation (March/April 2025). The most substantive thread is on the Kleppmann blog post; HN ID approximately `43447540` for the March 2025 post (HN search; verification gap — was not able to fetch the thread directly due to rate-limiting).

Recurring themes across the discussion (paraphrased; HN comments are not stable enough to quote without freshly verifying):

- Praise for the benchmarks and the reproducibility artefact.
- Skepticism about whether the algorithm extends beyond text (the limitation [open-problems.md §12](open-problems.md) names).
- Comparison to Replicache / Zero (server-authoritative architectures) as a competing pattern, not a competing algorithm.
- Discussion of whether the snapshot caching is actually the entire optimisation (it is, partially — see [algorithm.md §Snapshots](algorithm.md)).

**Verification gap:** I was not able to fetch HN comment text directly during this research session (rate-limited). The above is from secondary references; treat as paraphrase, not verbatim. If a future polish pass can confirm specific quotes, fold them in.

## 6. Skepticism: what hasn't been validated at scale

Eg-walker is **research-grade-but-shipping**. Things the research community has *not* validated:

- **Multi-million-op documents in production.** The paper benchmarks run on editing traces of "long-running" documents but not on the scale a multi-year shared workspace would reach. No production-grade documents of >10M ops have been published-on.
- **Cross-platform sync at scale.** Diamond-types ships Rust + WASM but no production app uses it for multi-platform sync at meaningful scale. Yjs/Automerge have run-the-gauntlet here; eg-walker has not.
- **Long-running offline merges in practice.** The paper's offline-merge benchmark uses synthetic divergence patterns. Real long-running offline (a month, six months) merges with realistic patterns are not in the published evaluation.
- **Anything beyond text.** The `more_types` branch is experimental; the paper's claims do not extend.

**For Myrhiza:** if eg-walker semantics get adopted, the *first* large-scale production deployment in a Myrhiza app would be a research contribution in itself, not a stress-test against an existing baseline.

## 7. The "no flagship app" honest disclosure

Diamond-types crates.io: 27,041 total / 3,065 recent downloads. Comparable to Loro's posture: shipping, real artefact, but no Linear/Notion/Proton-Docs-scale flagship.

This is **not unique to eg-walker** — it's the local-first / decentralised software community's general state. CRDTs that have a flagship app (Yjs via JupyterLab/Proton Docs) got there over a decade of iteration. Eg-walker is two years post-public-implementation. The maturity gap is age-related, not algorithm-related.

But for Myrhiza's adoption purposes, "research-grade-but-shipping" is the honest framing. Don't pitch eg-walker as production-validated; pitch it as algorithmically-vetted-implementation-needs-hardening.

## 8. The Aaron Boodman / Rocicorp angle

Already extensively covered in [`../crdts/critiques.md` §6](../crdts/critiques.md). The shorter framing for this folder:

Boodman/Rocicorp ship the **server-authoritative** alternative (Replicache → Zero; Reflect was shut down in 2025). The implicit critique of all decentralised-merge work — eg-walker included — is that production apps want a server that can validate writes, and the algorithmic depth of decentralised merge is wasted effort.

Eg-walker doesn't engage with this critique directly because it's a different *kind* of system. For Myrhiza, which is structurally P2P-only, the Boodman critique is on a different axis: it's "you're solving the wrong problem," not "you're solving the right problem badly."

**Note on Gentle-at-Rocicorp claim.** The Myrhiza research brief flagged a possible "Gentle is at Rocicorp" angle to verify. **I was not able to verify this.** Gentle's GitHub organisations are `codeparty`, `share`, `derbyjs`, `ottypes` (not `rocicorp`). The diamond-types README credits Invisible College for funding. Treat the Gentle-at-Rocicorp connection as **unconfirmed**; if it turns out to be correct, a future polish pass should fold it in. (Source: GitHub profile, diamond-types README, npm publish history.)

## 9. The paper's own §7 limitations

Paraphrased from the paper (the §7 limitations section is short; full verbatim would be a few sentences from the PDF, which I could not extract cleanly in this session — re-verify in polish pass):

- **Storage growth.** Event-graph storage grows linearly with edit count; the paper acknowledges this and gestures at compaction work for future research.
- **Single-document focus.** The paper's algorithm is text-only. Generalising to other data shapes is future work.
- **Authentication.** The paper does not address signed operations, peer identity, or Byzantine resistance. This is intentional — the algorithm is a merging algorithm.

**Verification gap:** the §7 verbatim wording could not be cleanly extracted during this research session (PDF text extraction failed locally). A polish pass should pull the exact phrasing from <https://arxiv.org/pdf/2409.14252>.

## 10. What this folder soft-pedalled and shouldn't

To stay honest:

- **Eg-walker is not "the future of CRDTs."** It is one paper, one implementation, one paradigm move. The CRDT libraries it benchmarks against are actively closing the gap (Automerge 3.0 columnar; Loro 1.x; Yjs 14 prerelease). Don't claim eg-walker as a settled win.
- **The "5000x faster" framing is 2021-era.** It doesn't apply to current Automerge 3.0 or Yjs 13.x in steady state. The current paper's headline ("order of magnitude less memory") is more conservative and is what should be cited.
- **The paper benchmarks are 2024 snapshots.** Loro `1.x` and Yjs `14` prerelease post-date the paper. Future comparisons should re-benchmark against current versions.

## Sources

- Paper: <https://arxiv.org/abs/2409.14252>
- Kleppmann homepage publications: <https://martin.kleppmann.com/>
- Gentle, *5000x faster CRDTs*: <https://josephg.com/blog/crdts-go-brrr/>
- Gentle, *I was wrong. CRDTs are the future*: <https://josephg.com/blog/crdts-are-the-future/>
- Gentle, *Rewriting Rust*: <https://josephg.com/blog/rewriting-rust/>
- Automerge 3.0 release: <https://automerge.org/blog/automerge-3/>
- EuroSys 2025: <https://2025.eurosys.org/>
- Cross-reference: [`../crdts/critiques.md`](../crdts/critiques.md), [`../crdts/open-problems.md`](../crdts/open-problems.md)
