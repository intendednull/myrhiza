**Date:** 2026-05-29
**Status:** active
**Subject:** Spawn (Vickrey-auction computational economy, 1992) and Tycoon (HP Labs proportional-share-by-bid, ~2004) — mechanisms, currency models, and abandoned status.

# Spawn and Tycoon — the two canonical implemented markets

These are the two most-cited *implemented* computational markets and the systems the reciprocity
brainstorm names. Both ran; neither survived. The pair also brackets the paradigm's own
self-correction: Spawn is a full auction market (1992); Tycoon (~2004) keeps the market vocabulary
but thins the mechanism down to proportional share — already conceding ground to the price-free
critique that [`markets-overkill.md`](markets-overkill.md) develops.

## Spawn (Waldspurger, Hogg, Huberman, Kephart, Stornetta — IEEE TSE 18(2), 1992)

**What it was.** An open, market-based system for harvesting idle cycles across a network of
heterogeneous workstations, used for coarse-grain concurrent applications (the demo workload was
concurrent Monte Carlo simulation). Built at Xerox PARC / Stanford.

**Mechanism — the auction.** Each processor (a *seller*) auctions off CPU time slices; jobs
(*buyers*) bid money for them. **The auctions are sealed-bid, second-price (Vickrey) auctions** —
verified directly from the primary: *"The auctions employed by Spawn are sealed-bid, second-price
auctions"* (TSE p. 105). "Sealed" = bidders cannot see others' bids; "second-price" = the winner
pays the second-highest bid. The paper's stated rationale is the standard Vickrey one: it gives
agents an incentive to bid their true valuation. (This resolves a prior-pass flag — the Vickrey
detail is now confirmed from the primary PDF, not a secondary source.)

**Currency model — hierarchical funding.** Jobs receive an initial money allocation on entry and
spend it on CPU and on communication charges for crossing network links. Concurrent applications
are trees of tasks; **funding is inherited down the tree** — a parent task funds its children,
who bid for their own resources, so an application's total money budget bounds its aggregate
resource consumption. Money is a *spent* resource here (unlike Sutherland's reverting yen).

**What the paper itself flags.** Spawn's own evaluation foregrounds the paradigm's chronic
problems: **price transients and equilibrium dynamics** (prices oscillate before settling),
**fairness** of the resulting distribution, and **scaling** to large systems. These are not
incidental — they are the volatility/discovery cluster that [`open-problems.md`](open-problems.md)
#1–2 names as structurally unsolved.

**Status: research-only, effectively abandoned.** Spawn was a research prototype. It saw no
production deployment, and its own lead author moved to price-free scheduling within two years
([`markets-overkill.md`](markets-overkill.md)). It survives as a citation, not a system.

## Tycoon (Lai, Rasmusson, Adar, Sorkin, Zhang, Huberman — HP Labs, ~2004–05)

**What it was.** A distributed market-based resource allocation system for shared clusters
(Grid, PlanetLab), from HP Labs Palo Alto. Goal: let users *differentiate the value of their
jobs* (which plain proportional share cannot) while keeping acquisition latency low and imposing
*no manual bidding overhead*.

**Mechanism — proportional share by bid, not an auction.** This is the key distinction from
Spawn. Tycoon does **not** clear an auction. Each host allocates its capacity to users **in
proportion to their bids**: user *i*'s share of resource *r* ≈ bid(i,r) / Σ bids on *r*. The
paper calls this a "best-effort" allocation — your share changes continuously as others' bids,
and the set of active applications, change. Locally, Tycoon used PlanetLab's `plkmod`
proportional-share scheduler; it managed **CPU cycles only** (VServer/`plkmod` did not virtualize
other resources). So Tycoon is a market *interface* (bids, currency, value-differentiation) over
a price-free proportional-share *mechanism* — the paradigm already meeting its critics halfway.

**Currency model.** A continuously divisible virtual currency; bids are rates, not lump
payments, and a user spreads its budget across the hosts it wants. The design also sketched
richer allocation functions (it mentions Generalized Vickrey Auctions as an alternative) but the
shipped mechanism was proportional share.

**Status: research-only, effectively abandoned.** Tycoon ran on PlanetLab as a research system
(~2004–05) and was published across an arXiv tech report and a *Multiagent and Grid Systems*
journal article. It did not achieve durable production adoption; HP Labs did not carry it forward
into a product, and it is not in use today.

## Why both matter to Myrhiza

- **Spawn shows the full-market failure surface** — Vickrey auctions, hierarchical money, and
  *yet* transients/fairness/scaling problems plus zero deployment. If Myrhiza ran a live auction,
  it would inherit exactly this surface. The reciprocity model's local-crediting design exists
  precisely to avoid it ([`lessons.md`](lessons.md)).
- **Tycoon shows the convergence point** — when you actually try to ship "value-differentiated
  allocation," you end up at *proportional share by a weight*, which is what price-free schedulers
  already do. The marginal value of the "market" framing over a plain share weight is the open
  question the whole paradigm never answered favorably.
- **Author/affiliation note:** the canonical Tycoon author list (journal version, *MGS* 2005) is
  Lai / Rasmusson / Adar / **Stephen Sorkin** / Li Zhang / Huberman. An earlier arXiv preprint
  (cs/0404013) listed a shorter author set (Lai / Huberman / Fine); the cond.org "Implementation"
  version lists five (no Sorkin). The seed citation's "Sorkin/Zhang" matches the journal version.

## Sources

- [Waldspurger / Hogg / Huberman / Kephart / Stornetta, "Spawn: A Distributed Computational Economy," *IEEE TSE* 18(2):103–117, Feb 1992, DOI 10.1109/32.121753](https://dl.acm.org/doi/10.1109/32.121753) — Vickrey-auction quote and hierarchical-funding model verified from primary PDF.
- [Lai / Rasmusson / Adar / Sorkin / Zhang / Huberman, "Tycoon: an Implementation of a Distributed, Market-based Resource Allocation System," arXiv cs/0412038, 2004; *Multiagent and Grid Systems* 1(3), 2005](https://arxiv.org/abs/cs/0412038) — proportional-share-by-bid mechanism and CPU-only scope verified from primary PDF.
- Cross-references: [`history.md`](history.md), [`markets-overkill.md`](markets-overkill.md), [`mirage-bellagio.md`](mirage-bellagio.md), [`open-problems.md`](open-problems.md).
