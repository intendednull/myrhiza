**Date:** 2026-05-29
**Status:** active
**Subject:** Glossary of market-based-control terms used across this folder.

# Glossary

Terms as used in this folder. Where a term collides with a different meaning in `agoric-endo/`
or `resource-pricing-theory/`, the collision is noted.

- **Agoric system** — Miller & Drexler's 1988 term (from Greek *agora*, marketplace) for a
  computational system that coordinates via market mechanisms — agents, prices, trade. **Note the
  collision:** in [`agoric-endo/`](../agoric-endo/), "Agoric" means the *company* and its
  SES/Endo/SwingSet stack. This folder always means the *paradigm*.

- **Market-based control** — the umbrella name (Clearwater 1996) for allocating distributed
  computational resources by a market: bidding, pricing, clearing. The paradigm this folder
  surveys.

- **Computational economy** — a running system in which software agents hold currency and buy/sell
  compute resources (e.g. Spawn). Roughly synonymous with "agoric system" / "market-based control"
  at the implementation level.

- **Vickrey auction (sealed-bid second-price)** — an auction where bids are hidden and the winner
  pays the *second*-highest bid. Incentive-compatible: bidders' best strategy is to bid their true
  valuation. The mechanism Spawn used.

- **Combinatorial auction** — an auction where bidders bid on *bundles* of items (e.g. "32 motes
  for 8 hours") rather than single items, and the auctioneer picks the revenue-maximizing set of
  winning bundles. Used by Mirage and Bellagio. Expressive but raises the bid-authoring burden.

- **Proportional share** — allocating a resource to each client in proportion to a weight (tickets,
  or a bid amount). **Price-free** in the lottery/stride sense (weight = tickets); **market-flavored**
  in Tycoon (weight = bid). The deployed, surviving family.

- **Lottery / stride scheduling** — Waldspurger & Weihl's price-free proportional-share schedulers.
  Tickets represent resource rights; lottery draws randomly weighted by tickets, stride is the
  deterministic equivalent. Conceptual ancestor of Linux CFS/EEVDF.

- **Shadow price** — the marginal value of relaxing a constrained resource by one unit; in this
  folder, a peer's *subjective* per-resource scarcity weight. In
  [`resource-pricing-theory/`](../resource-pricing-theory/) the same term is the LP-dual / Lagrange
  multiplier in Kelly's NUM. Myrhiza's `shadow_prices_P` are *locally set, not globally reconciled*.

- **Tâtonnement** — the iterative price-adjustment process (raise prices on over-demanded goods,
  lower on under-demanded) by which a market gropes toward equilibrium. WALRAS implements a
  distributed tâtonnement. The cross-peer reconciliation Myrhiza forgoes.

- **Walrasian / general equilibrium** — a single price vector that clears *all* interdependent
  markets simultaneously (supply = demand everywhere). Wellman's WALRAS computes it distributedly.
  The global-optimal runner-up Myrhiza declines.

- **Tickets / leases (SHARP)** — SHARP's two-part resource claim: a **ticket** is a soft promise
  (may be oversubscribed), a **lease** is a hard, time-bounded grant. Policy-agnostic plumbing for
  resource peering.

- **Virtual currency / scrip** — a closed, non-cash currency internal to a system (Sutherland's
  "yen," Mirage/Bellagio's auction currency). Prone to starvation, depletion, hoarding, inflation —
  needs monetary policy. Myrhiza rejects a *shared* one.

- **Savings tax / "use it or lose it"** — a decay applied to accumulated currency to stop idle
  users hoarding scrip. Mirage's anti-hoarding mechanism; the deployed precedent for Myrhiza's
  consumption-relative decay.

- **Best-effort default** — a free, un-priced fallback allocation (PlanetLab's proportional share).
  A market that competes *against* a best-effort default tends to be ignored (Bellagio's failure);
  a market that is the *sole* path has no such competition (Mirage's success).

- **Binding resource** — a resource that is actually the scarce constraint at decision time
  (Mirage's physical motes). Pricing a *non-binding* resource gives users no reason to pay
  (Bellagio's mistake).

- **Coasean firm-vs-market boundary** — Ronald Coase's question of when to coordinate by market
  (price everything) vs by "firm" (direct/central coordination), set by transaction-cost overhead.
  Miller & Drexler conceded it: below some granularity, pricing's overhead exceeds its benefit, so
  "islands of central direction" sit in a "sea of trade."

- **Market plumbing** — the infrastructure a deployed market needs (authentication, isolation,
  currency management, bidding interface, clearing). The HotOS 2005 retrospective identifies its
  cost and immaturity as a barrier to adoption.

## Sources

Definitions synthesized from the per-file sources in this folder. See [`history.md`](history.md),
[`spawn-tycoon.md`](spawn-tycoon.md), [`mirage-bellagio.md`](mirage-bellagio.md),
[`markets-overkill.md`](markets-overkill.md), and [`open-problems.md`](open-problems.md).
