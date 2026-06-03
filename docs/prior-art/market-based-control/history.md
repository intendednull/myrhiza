**Date:** 2026-05-29
**Status:** active
**Subject:** Chronological lineage of market-based control — Sutherland 1968 → Miller-Drexler 1988 → Spawn 1992 → Tycoon ~2004 → the testbed deployments.

# History — the recurring idea, 1968 to 2009

The market-based-control paradigm has been independently re-invented roughly once per decade.
The arc below is not a story of cumulative progress toward deployment; it is a story of the
same idea proposed, implemented, and then *not* sticking — with the price-free alternatives
quietly winning each time. Read this alongside [`markets-overkill.md`](markets-overkill.md),
which makes the "the alternative shipped instead" point explicit.

## 1968 — Sutherland's futures market (the deep root)

I. E. Sutherland, then at Harvard's Aiken Computation Laboratory, ran a **continuous auction
for time on the PDP-1**. Users wrote bids (in an artificial currency called "yen") on a paper
scheduling sheet; a higher bid in yen-per-hour preempted a lower one; the schedule froze each
morning for the next day. The load-bearing design choice: **yen are non-consumable** — they
"revert to the bidder as soon as he consumes the computer time for which he bid," so a user's
purchasing power does not decrease through use. This eliminated the "feast and famine" of
monthly allocations (the computer sitting idle at month's end because everyone exhausted their
quota). Different users got different yen budgets by project importance (faculty 10, casual
student 1). This is the deep root of every later computational market: bidding currency as a
*priority* token, not a *consumed* resource — a distinction Myrhiza's no-token stance should
note, because it is the opposite of a spent-scrip economy. (CACM 11(6):449–451, June 1968.)

## 1988 — Miller & Drexler name the paradigm ("Agoric Open Systems")

Mark S. Miller and K. Eric Drexler's three-part contribution to *The Ecology of Computation*
(ed. Bernardo Huberman, North-Holland) — "Markets and Computation: Agoric Open Systems,"
"Comparative Ecology," and "Incentive Engineering" — gave the paradigm its name (Greek *agora*,
marketplace) and its manifesto. The thesis: the price system that coordinates human economies
can coordinate computation in large open systems, combining local decisions by diverse parties
into globally effective behavior. Crucially for Myrhiza, the *same papers* concede the limits:
markets carry transaction overhead, and "for small enough objects and transactions, the cost of
accounting and negotiations will overwhelm any advantages" — so "computational markets will
consist of islands of central direction in a sea of trade." This is the Coasean firm-vs-market
boundary, conceded by the paradigm's own founders (see [`open-problems.md`](open-problems.md)
#4). These authors are the same lineage documented in [`agoric-endo/history.md`](../agoric-endo/history.md)
(AMIX → E → Caja → SES) — but the 1988 paradigm is distinct from the 2018 company.

## 1992 — Spawn: the paradigm implemented

Carl Waldspurger, Tad Hogg, Bernardo Huberman, Jeffrey Kephart, and W. Scott Stornetta (Xerox
PARC / Stanford) built **Spawn**, a working distributed computational economy harvesting idle
workstation cycles. Jobs held money, bought CPU slices via **sealed-bid second-price (Vickrey)
auctions** run by each processor, and funding flowed hierarchically down a tree of subtasks.
Spawn is the canonical *implemented* market and the reference the reciprocity brainstorm cites.
Its own paper documents the paradigm's chronic ailments: price transients, equilibrium dynamics,
and fairness questions. See [`spawn-tycoon.md`](spawn-tycoon.md). (IEEE TSE 18(2), 1992.)

## 1994–95 — the pivot: Waldspurger abandons the market

Two years after Spawn, Waldspurger (with William Weihl, at MIT) published **lottery scheduling**
(OSDI 1994) and **stride scheduling**, then his MIT dissertation (MIT-LCS-TR-667, 1995). These
are **price-free** proportional-share schemes: resource rights are "tickets," allocation is
proportional to ticket holdings, no auction, no price discovery, no bidding. The *same author
who built the market chose the price-free design next* — and it is the price-free lineage
(through Linux's CFS and EEVDF) that shipped to billions of machines, while Spawn did not. This
pivot is the single most informative datum in the corpus; [`markets-overkill.md`](markets-overkill.md)
develops it.

## 2003 — SHARP: mechanism without mandated market

Fu, Chase, Chun, Schwab, and Vahdat's **SHARP** (SOSP 2003) is the substrate layer: cryptographic
**tickets and leases** for secure resource peering across sites, with delegation and accountable
oversubscription. SHARP deliberately **separates mechanism from policy** — it provides the
plumbing for trading resources but does not mandate a market; its PlanetLab demo used a
"decentralized barter economy." Mirage and Bellagio were built *on* SHARP-style claims. See
[`markets-overkill.md`](markets-overkill.md). (SOSP 2003.)

## ~2004–05 — Tycoon: the market thinned to proportional share

HP Labs' **Tycoon** (Lai, Rasmusson, Adar, Sorkin, Zhang, Huberman) kept the "differentiate the
value of your jobs" goal but discarded the auction: each user's share of a host equals its bid
divided by the total bids on that host — **best-effort proportional share by bid**, with no
clearing auction and "no manual bidding overhead." Tycoon ran on PlanetLab managing CPU only.
It is the paradigm meeting the price-free critique halfway: a market in name, proportional share
in mechanism. It did not achieve durable production use. See [`spawn-tycoon.md`](spawn-tycoon.md).

## 2004–05 — Bellagio and Mirage: the deployment experiments (and post-mortems)

The same UCSD/Intel/Harvard group ran two real markets and wrote the honest retrospective.
**Bellagio** (AuYoung/Chun/Snoeren/Vahdat, OASIS 2004) put a combinatorial-auction virtual-
currency market on **PlanetLab** — which already had a free best-effort proportional share, so
the priced resource was non-binding and the market was opt-in; adoption was low. **Mirage**
(Chun et al., IEEE EmNetS-II 2005) put a repeated combinatorial auction on a 148-mote sensornet
testbed and ran daily for ~4 months — and *succeeded*, because it was the **sole means** of
getting physical access to a genuinely scarce binding resource. The "Why Markets Could (But
Don't Currently)..." paper (HotOS 2005) and a 2009 Wiley retrospective distilled the lessons.
This matched pair is the corpus's actionable core: [`mirage-bellagio.md`](mirage-bellagio.md).

## After 2005 — the paradigm goes quiet; price-free ships

No general-purpose computational market reached durable production after the PlanetLab era. The
schemes that *did* ship and survive are price-free or pure-reciprocity: Linux CFS/EEVDF
(proportional share), BitTorrent choking, IPFS Bitswap. The cloud era priced resources, but via
fixed-rate billing and reservation/spot tiers — not the agent-bidding internal markets this
paradigm envisioned. The lineage's living descendants are conceptual (the reciprocity
brainstorm's `value_P` rule) rather than deployed systems.

## Sources

- [Sutherland, "A Futures Market in Computer Time," *CACM* 11(6):449–451, June 1968](https://dl.acm.org/doi/10.1145/363347.363396) — verified from primary PDF (Harvard PDP-1, Aiken Lab; "yen" non-consumable currency).
- [Miller & Drexler, "Markets and Computation: Agoric Open Systems," in *The Ecology of Computation* (ed. Huberman), North-Holland, 1988](https://papers.agoric.com/papers/markets-and-computation-agoric-open-systems/full-text/) — Coasean "islands of central direction in a sea of trade" verified from agoric.com full-text.
- [Waldspurger / Hogg / Huberman / Kephart / Stornetta, "Spawn: A Distributed Computational Economy," *IEEE TSE* 18(2):103–117, 1992](https://dl.acm.org/doi/10.1109/32.121753) — Vickrey-auction detail verified from primary PDF.
- [Waldspurger & Weihl, "Lottery Scheduling," OSDI 1994](https://www.usenix.org/conference/osdi-94/lottery-scheduling-flexible-proportional-share-resource-management); [Waldspurger, "Lottery and Stride Scheduling," MIT-LCS-TR-667, 1995](https://publications.csail.mit.edu/lcs/pubs/pdf/MIT-LCS-TR-667.pdf).
- [Fu / Chase / Chun / Schwab / Vahdat, "SHARP," SOSP 2003](https://www.cs.rochester.edu/meetings/sosp2003/papers/p204-fu.pdf) — verified from primary PDF.
- [Lai et al., "Tycoon," HP Labs / arXiv cs/0412038](https://arxiv.org/abs/cs/0412038) — verified from primary PDF.
- [AuYoung et al., "Bellagio," OASIS 2004](https://cseweb.ucsd.edu/~aauyoung/papers/bellagio-oasis04.pdf); [Chun et al., "Mirage," EmNetS-II 2005](https://cseweb.ucsd.edu/~aauyoung/papers/mirage-emnets05.pdf); [Shneidman et al., HotOS 2005](https://www.usenix.org/legacyurl/hotos-x-151-technical-paper-25) — all verified from primary PDFs.
- Cross-references: [`agoric-endo/history.md`](../agoric-endo/history.md), [`spawn-tycoon.md`](spawn-tycoon.md), [`mirage-bellagio.md`](mirage-bellagio.md), [`markets-overkill.md`](markets-overkill.md), [`open-problems.md`](open-problems.md).
