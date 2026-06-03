**Date:** 2026-05-29
**Status:** active
**Subject:** Market-based-control / computational-economy paradigm survey — the lineage `value_P = resource_vector · shadow_prices_P` descends from, and its 50-year record of mostly-failing to deploy.

# Market-based control — the computational-economy paradigm and its deployment record

This folder is a multi-paper survey, not a single-project deep-dive. It documents the
**market-based control** paradigm: the recurring idea, alive since 1968, that distributed
computational resources should be allocated by a *market* — agents bid currency for CPU,
bandwidth, and storage, and prices coordinate supply and demand the way they do in human
economies. Myrhiza's reciprocity-economy brainstorm landed on a subjective per-peer pricing
rule, `value_P(action) = resource_vector(action) · shadow_prices_P` ([reciprocity report
§"The model so far" layer 4](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md)).
That rule is a direct descendant of this paradigm. This folder traces the descent — **and,
just as importantly, the failures.**

**The honest throughline.** Market-based resource allocation has been invented, re-invented,
implemented, and published roughly once a decade since Sutherland's 1968 PDP-1 auction. Almost
none of it deployed and stuck. The systems that *shipped* and *survived* at scale — Linux's
proportional-share schedulers, BitTorrent's choking, IPFS Bitswap — are **price-free**. The
corpus value here is therefore the **failure analysis** as much as the mechanisms: *why* markets
keep losing to simpler share-based or reciprocity-based schemes, and what narrow conditions let
the rare market deployment (Mirage) survive. Read this folder as a cautionary record, not a
menu of mechanisms to adopt.

**Boundary with `agoric-endo/`.** This is **not** an extension of [`prior-art/agoric-endo/`](../agoric-endo/).
That folder documents *Agoric the company* — the SES/Endo smart-contract stack and its BLD/IST
tokenomics. This folder documents the *market-based-control paradigm* the **name** "Agoric"
traces to: Miller & Drexler's 1988 "Agoric Open Systems" papers (Greek *agora*, marketplace).
The two share personnel and intellectual lineage — Mark Miller and the AMIX/E crew appear in
both — but they are distinct subjects. We cross-reference [`agoric-endo/history.md`](../agoric-endo/history.md)
for the shared lineage and note a productive tension with [`agoric-endo/governance.md` §"Implications"](../agoric-endo/governance.md)
("tokenomics are not a runtime concern"); we do not restate that folder's content.

## Key facts

| Fact | Value |
|---|---|
| Survey scope | ~8 systems / papers, 1968–2009, plus modern price-free contrast |
| Deep root | Sutherland, "A Futures Market in Computer Time," *CACM* 11(6), June 1968 (Harvard PDP-1) |
| Paradigm-naming paper | Miller & Drexler, "Markets and Computation: Agoric Open Systems," in *The Ecology of Computation* (ed. Huberman), North-Holland, 1988 |
| Canonical implemented market | Spawn — Waldspurger/Hogg/Huberman/Kephart/Stornetta, *IEEE TSE* 18(2), 1992 (sealed-bid second-price / Vickrey auctions) |
| The damning tell | Waldspurger built Spawn (market, 1992), then abandoned it for **price-free** lottery/stride scheduling (1994–95); the price-free design shipped (Linux CFS/EEVDF lineage), the market did not |
| Lone deployment success | Mirage (sensornet testbed, 2005) — survived *only* as the mandatory sole path to a scarce binding resource |
| Matched-pair failure | Bellagio (PlanetLab) — opt-in market competing with a free best-effort default, pricing a non-binding resource; low adoption |
| Both Mirage/Bellagio needed | a savings-tax / currency decay ("use it or lose it") to stop idle scrip hoarding |
| Research-only vs deployed | Every market system here is **research-only or abandoned.** No general-purpose computational market is in production |

## Contents

Each file is independent and ends with `## Sources`. Reading order below.

**Reference / narrative**
- [**history.md**](history.md) — the chronological lineage, one paragraph per milestone: Sutherland 1968 → Miller-Drexler 1988 → Spawn 1992 → Tycoon ~2004 → the testbed deployments. **Read first for orientation.**
- [**spawn-tycoon.md**](spawn-tycoon.md) — the two canonical implemented markets: Spawn (Vickrey-auction computational economy) and Tycoon (HP Labs proportional-share-by-bid). Mechanisms, currency models, and STATUS (both research-only / abandoned).

**The deployment-survival evidence**
- [**mirage-bellagio.md**](mirage-bellagio.md) — **the most directly actionable file.** The two adoption post-mortems: Mirage (succeeded — mandatory sole path to a scarce binding resource) vs Bellagio (failed — opt-in vs free best-effort, non-binding resource), and why both needed a savings-tax/decay.
- [**markets-overkill.md**](markets-overkill.md) — the "markets are overkill" critique. Waldspurger's own pivot from Spawn to price-free scheduling; SHARP's mechanism/policy separation; Clearwater's *Market-Based Control* (1996) and Wellman's WALRAS (the declined global-clearing runner-up to local pricing).

**Synthesis**
- [**open-problems.md**](open-problems.md) — what the paradigm structurally never solved (price volatility, discovery latency, bid-authoring UX, the Coase firm-vs-market boundary Miller-Drexler themselves conceded).
- [**lessons.md**](lessons.md) — **the consult-this-when-designing file.** Validates / Avoid / Borrow, framed for the reciprocity model. Lands the escape hatch: a per-peer *local crediting ledger* (not a live auction) sidesteps the cluster of failures that killed these systems.
- [**glossary.md**](glossary.md) — terms used across the folder.

## How to use this prior-art doc

Designing the Myrhiza participation/reciprocity model or any feature that prices resources?
Start with [`history.md`](history.md) for the lineage, then [`mirage-bellagio.md`](mirage-bellagio.md)
for the deployment-survival conditions, then [`lessons.md`](lessons.md) for action-oriented
synthesis. Drop into [`spawn-tycoon.md`](spawn-tycoon.md) / [`markets-overkill.md`](markets-overkill.md)
for mechanism detail.

**Framing disclosure.** This corpus is written from Myrhiza's stance, which is **not neutral**.
Myrhiza has committed to *no global token, ever* and to a *per-peer, non-authoritative,
local-crediting* model that lives off the determinism path (see the [reciprocity report's
locked decisions](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md)). That
commitment is a *bet against* most of the paradigm documented here: against live auctions,
against global market-clearing (WALRAS), against a shared currency that can be hoarded,
inflated, or starved. So this folder reads the literature **looking for evidence that the local-
crediting escape is sound** — it foregrounds the failure modes a live market incurs and the
narrow conditions a deployed market needs, and it backgrounds the optimality theory (NUM,
general equilibrium) that local subjective pricing deliberately forgoes. A reader asking "should
Myrhiza run a real market after all?" should weigh the corpus accordingly: it is a *learn-why-
markets-fail-into-a-local-crediting-Myrhiza* artifact, not an even-handed market-design textbook.
The optimality/fairness side of the same question lives in the sibling folder
[`prior-art/resource-pricing-theory/`](../resource-pricing-theory/) (Kelly NUM, DRF); the
token-free heterogeneous-reciprocity systems that actually *realize* the local-crediting shape
live in [`prior-art/p2p-resource-economics/`](../p2p-resource-economics/).

## Cross-links

- [`reports/2026-05-29-reciprocity-economy-brainstorm/`](../../reports/2026-05-29-reciprocity-economy-brainstorm/) — the consumer; `value_P = resource_vector · shadow_prices_P` is the descendant idea, and challenge #3 ("markets are overkill") is exactly this folder's throughline.
- [`prior-art/agoric-endo/history.md`](../agoric-endo/history.md) — shared Miller-Drexler / AMIX / E lineage (the company, not the paradigm).
- [`prior-art/agoric-endo/governance.md`](../agoric-endo/governance.md) §"Implications for Myrhiza" — "tokenomics are not a runtime concern"; the productive tension this folder sharpens.
- [`prior-art/p2p-resource-economics/`](../p2p-resource-economics/) — token-free heterogeneous reciprocity (OurGrid, GNUnet, Samsara); where the leading Myrhiza model actually lives.
- [`prior-art/resource-pricing-theory/`](../resource-pricing-theory/) — Kelly NUM, DRF, the formal fairness critique of "price every resource and sum."
- [`prior-art/sybil-resistance/`](../sybil-resistance/) — `taxonomy.md` Category 4 (PoW/PoS/token, rejected) and the reciprocity-beats-reputation lesson.

## Sources

Full per-claim citations live in the per-file `## Sources` sections. Primary roots:

- [Sutherland, "A Futures Market in Computer Time," *CACM* 11(6):449–451, June 1968, DOI 10.1145/363347.363396](https://dl.acm.org/doi/10.1145/363347.363396)
- [Miller & Drexler, "Markets and Computation: Agoric Open Systems," in *The Ecology of Computation* (ed. B. Huberman), North-Holland, 1988](https://papers.agoric.com/papers/markets-and-computation-agoric-open-systems/full-text/)
- [Waldspurger / Hogg / Huberman / Kephart / Stornetta, "Spawn: A Distributed Computational Economy," *IEEE TSE* 18(2):103–117, Feb 1992, DOI 10.1109/32.121753](https://dl.acm.org/doi/10.1109/32.121753)
- [Lai / Rasmusson / Adar / Sorkin / Zhang / Huberman, "Tycoon: an Implementation of a Distributed, Market-based Resource Allocation System," HP Labs / arXiv cs/0412038, 2004–05](https://arxiv.org/abs/cs/0412038)
- [Chun et al., "Mirage: A Microeconomic Resource Allocation System for Sensornet Testbeds," IEEE EmNetS-II, 2005](https://cseweb.ucsd.edu/~aauyoung/papers/mirage-emnets05.pdf)
- [AuYoung / Chun / Snoeren / Vahdat, "Resource Allocation in Federated Distributed Computing Infrastructures" (Bellagio), OASIS 2004](https://cseweb.ucsd.edu/~aauyoung/papers/bellagio-oasis04.pdf)
- [Shneidman / Ng / Parkes / AuYoung / Snoeren / Vahdat / Chun, "Why Markets Could (But Don't Currently) Solve Resource Allocation Problems in Systems," HotOS X, 2005](https://www.usenix.org/legacyurl/hotos-x-151-technical-paper-25)
- [Waldspurger & Weihl, "Lottery Scheduling: Flexible Proportional-Share Resource Management," OSDI 1994](https://www.usenix.org/conference/osdi-94/lottery-scheduling-flexible-proportional-share-resource-management)
- [Fu / Chase / Chun / Schwab / Vahdat, "SHARP: An Architecture for Secure Resource Peering," SOSP 2003](https://www.cs.rochester.edu/meetings/sosp2003/papers/p204-fu.pdf)
- [Clearwater (ed.), *Market-Based Control: A Paradigm for Distributed Resource Allocation*, World Scientific, 1996, ISBN 9810222548](https://books.google.com/books/about/Market_based_Control.html?id=-nfFSLQ6M74C)
