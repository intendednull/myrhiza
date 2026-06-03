**Date:** 2026-05-29
**Status:** active
**Subject:** The "markets are overkill" critique — Waldspurger's own pivot from Spawn to price-free lottery/stride scheduling (which shipped), SHARP's mechanism/policy separation, and the declined global-clearing runner-up (Wellman's WALRAS / Clearwater 1996).

# Markets are overkill — the critique the paradigm's own builders made

The strongest case against market-based control was made by the people who built the markets.
This file collects that internal critique: the author of Spawn abandoned it for a price-free
design that then shipped to billions of machines; the substrate layer (SHARP) deliberately
refused to mandate a market; and the global-market-clearing branch (WALRAS) was a coherent,
worked-out alternative that the field declined in favor of *local* pricing. For Myrhiza, this
file is the evidence base for "don't run a live market; do local crediting instead."

## The damning tell — Waldspurger's pivot (the load-bearing fact)

Carl Waldspurger was lead author of **Spawn** (1992), a full Vickrey-auction computational market
([`spawn-tycoon.md`](spawn-tycoon.md)). Two years later, at MIT, he published **lottery
scheduling** (Waldspurger & Weihl, OSDI 1994) and **stride scheduling**, consolidated in his
dissertation *Lottery and Stride Scheduling* (MIT-LCS-TR-667, 1995). These are **price-free
proportional share**:

- Resource rights are **tickets**. A client's share of a resource is its tickets / total tickets.
- Lottery scheduling picks the next holder by a random draw weighted by tickets; stride
  scheduling is the deterministic analogue (same proportional outcome, no randomness).
- A **currency abstraction** lets modules name/share/protect tickets — but it is an *accounting*
  currency, **not a price**. There is no auction, no bid, no price discovery, no clearing.

The same person who built the market chose the price-free design for the *next* system — and it
is the price-free lineage that won. Stride/lottery proportional share is the conceptual ancestor
of the weighted fair-share schedulers that shipped in production operating systems (the Linux
**CFS** and later **EEVDF** lineage are proportional-share-by-weight). The market did not ship;
the price-free alternative did. **When the inventor of the market reaches for a price-free design
the moment he wants something that actually works, that is the strongest possible evidence that
the market machinery was overkill for the job.**

The reciprocity report names this exact tell as its model-challenge #3 and answers it: Myrhiza's
crediting is *per-peer local bookkeeping off the determinism path, not a live auction*, so it
incurs none of the volatility/discovery/bid-UX costs that drove the pivot. The forward pointer
for the *formal* fairness side of this critique — DRF's proof that "price every resource and sum"
can be fairness-inferior to a price-free share rule — is [`prior-art/resource-pricing-theory/`](../resource-pricing-theory/).

## SHARP — mechanism without a mandated market

Fu, Chase, Chun, Schwab, Vahdat — "SHARP: An Architecture for Secure Resource Peering" (SOSP
2003). SHARP is the substrate Mirage and Bellagio built on, and its design choice is itself a
verdict: it **separates mechanism from policy.** SHARP provides cryptographically protected
resource **claims** — split into **tickets** (soft promises) and **leases** (hard grants) — plus
secure delegation and *accountable oversubscription*, so sites can trade or federate resources
"according to local policies." It does **not** mandate a market; its PlanetLab demonstration used
a "decentralized barter economy," but that is one policy among many the mechanism admits. The
lesson Myrhiza should take: build the *enforcement mechanism* (refusal, capability-gated serving)
as policy-agnostic plumbing, and let the *pricing/standing policy* (the reciprocity module) sit
above it — which is exactly the [reciprocity report's locked decision](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md)
that "reciprocity logic is a module, not a kernel built-in."

## The declined runner-up — global market-clearing (WALRAS / Clearwater 1996)

The paradigm had a more ambitious branch: instead of pairwise/auction pricing, compute a **global
general-equilibrium price** that clears all markets at once. Michael Wellman's **WALRAS** (a
"market-oriented programming environment," JAIR 1, 1993; the WALRAS *algorithm* later in
*Computational Economics* 12(1), 1998) did exactly this — a distributed tâtonnement that converges
to Walrasian equilibrium prices across interdependent computational markets. Scott Clearwater's
edited volume **_Market-Based Control: A Paradigm for Distributed Resource Allocation_** (World
Scientific, 1996) is the paradigm's anthology, collecting WALRAS-style global-clearing work
alongside auction and proportional-share approaches.

**Why this is the runner-up Myrhiza declines.** Global market-clearing requires all agents to
share consistent prices and to iterate to convergence — a *cross-peer price reconciliation* step.
Myrhiza's model deliberately forgoes that: each peer sets its **own** subjective shadow prices and
never reconciles them globally (the directional crediting rule). This buys trust-minimality and
determinism-compatibility at the cost of global optimality — the reciprocity report's model-
challenge #5 ("soundness honesty") is precisely the admission that independently-set local prices
are a *locally-valid Lagrangian cost*, not a globally-optimal Walrasian clearing. WALRAS is the
thing Myrhiza is *not* doing, and naming it that way keeps the design honest about what it gives up.

## The combined verdict for Myrhiza

Three independent signals point the same way: (1) the market's own inventor went price-free for
the system that shipped; (2) the substrate layer refused to mandate a market and made
mechanism/policy separable; (3) the global-optimal branch (WALRAS) needs cross-peer price
reconciliation Myrhiza structurally cannot have. The convergent reading: **don't run a live market
inside the runtime.** Use a price-free or local-crediting mechanism for the common case, and
reserve any pricing for the rare cross-trust-boundary exchange (the Coasean boundary —
[`open-problems.md`](open-problems.md) #4). This is the load-bearing escape that [`lessons.md`](lessons.md)
develops.

## Sources

- [Waldspurger & Weihl, "Lottery Scheduling: Flexible Proportional-Share Resource Management," OSDI 1994](https://www.usenix.org/conference/osdi-94/lottery-scheduling-flexible-proportional-share-resource-management); [Waldspurger, "Lottery and Stride Scheduling," MIT-LCS-TR-667, EECS/MIT, Sept 1995](https://publications.csail.mit.edu/lcs/pubs/pdf/MIT-LCS-TR-667.pdf) — price-free ticket/currency mechanism verified from search of primary abstracts.
- [Fu / Chase / Chun / Schwab / Vahdat, "SHARP: An Architecture for Secure Resource Peering," SOSP 2003](https://www.cs.rochester.edu/meetings/sosp2003/papers/p204-fu.pdf) — tickets/leases, mechanism/policy separation, and "decentralized barter economy" demo verified from primary PDF.
- [Wellman, "A Market-Oriented Programming Environment and its Application to Distributed Multicommodity Flow Problems" (WALRAS), JAIR 1, 1993, arXiv cs/9308102](https://arxiv.org/abs/cs/9308102); ["The WALRAS Algorithm: A Convergent Distributed Implementation of General Equilibrium Outcomes," *Computational Economics* 12(1), 1998](https://link.springer.com/article/10.1023/A:1008654125853).
- [Clearwater (ed.), *Market-Based Control: A Paradigm for Distributed Resource Allocation*, World Scientific, 1996, ISBN 9810222548](https://books.google.com/books/about/Market_based_Control.html?id=-nfFSLQ6M74C) — title/publisher verified; year 1996 (one tracker says 1995 — see flag).
- Linux proportional-share lineage (CFS / EEVDF) — *general background, widely documented; treated here as the deployed price-free descendant of stride scheduling, not a load-bearing citation.*
- Cross-references: [`spawn-tycoon.md`](spawn-tycoon.md), [`open-problems.md`](open-problems.md), [`lessons.md`](lessons.md), [`prior-art/resource-pricing-theory/`](../resource-pricing-theory/).

> **Flag — Clearwater year:** Most catalogues (World Scientific, Google Books, Amazon ISBN 9810222548) date the volume **1996**; one Semantic Scholar record shows 1995. The seed's "1996" is the dominant attribution and is used here. Low-impact discrepancy.
