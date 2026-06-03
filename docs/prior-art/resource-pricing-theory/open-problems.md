**Date:** 2026-05-29
**Status:** active
**Subject:** What resource-pricing theory does not cleanly solve for a per-peer, no-reconciliation reciprocity economy — the open problems Myrhiza inherits if it specs `value_P = resource_vector · shadow_prices_P`.

# Open problems — resource-pricing theory for local reciprocity

What the theory in this folder does *not* settle for a per-peer, non-authoritative valuation. Each entry: problem statement + why it matters for Myrhiza + canonical sources. These are the questions a `myrhiza-participation-*` spec must answer or explicitly defer.

## 1. Cross-resource commensurability — is collapsing the vector to one scalar even well-posed?

The dot-product `resource_vector · shadow_prices` produces a single scalar. But DRF's premise is that resources are **Leontief / non-substitutable** — a CPU-blocked task gets *zero* value from extra RAM ([`dominant-resource-fairness.md`](dominant-resource-fairness.md)). For such demands, "value" is governed by the *bottleneck* resource, and a fixed price vector that sums over *all* resources misprices any action whose binding resource differs from the price vector's emphasis. There is no theorem that says a scalar collapse preserves the right ordering of actions when demands are bottlenecked.

**What's needed:** decide whether to price against the *binding* resource (a max-over-resources / dominant-resource rule) or to carry the vector and reconcile per-resource, collapsing only at the last moment — and prove (or empirically check) that whichever choice preserves a sensible cost ordering. The brainstorm's mitigation #2 leans toward binding-resource pricing.

**Canonical sources:** [`dominant-resource-fairness.md`](dominant-resource-fairness.md) (Leontief, DRF), [reciprocity brainstorm §"What the leading `value()` must answer"](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) #2.

## 2. Local prices without reconciliation — what is the soundness floor?

NUM's optimality, convergence, and cross-peer consistency are theorems about a *shared* optimization reconciled by congestion feedback ([`network-utility-maximization.md`](network-utility-maximization.md)). Myrhiza's prices are per-peer and *never reconciled* — by determinism-boundary necessity, not oversight. So the prices are a *valid local Lagrangian cost* but **not** dual optimizers of any global problem. The open question: **what, formally, do we still get?** Is "each peer's crediting is internally consistent and incentive-aligned for *that peer*" a strong enough property to carry the reciprocity model, or does cross-peer inconsistency open exploitable arbitrage (peer X values an action high, peer Y values the inverse high, an intermediary farms the gap)?

**What's needed:** a precise statement of the local-soundness property the model actually relies on, and an adversarial check for cross-peer price arbitrage. Until then, label the construction a **trust-minimal heuristic**, never a proven dual or market clearing.

**Canonical sources:** [`network-utility-maximization.md`](network-utility-maximization.md) §"the critical caveat" + §"dual decomposition," [reciprocity brainstorm](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) #5.

## 3. Shadow prices may not exist under friction

Even granting reconciliation, Czichowsky–Muhle-Karbe–Schachermayer show shadow prices can **fail to exist** under arbitrarily small transaction costs ([`network-utility-maximization.md`](network-utility-maximization.md) §"transaction costs"). A reciprocity ledger with directional crediting, replacement-cost caps for non-reproducible work, and decay is *full* of frictions. Whether a globally-coherent shadow price even *exists* for Myrhiza's frictioned exchange is unverified — and the financial-math result is a warning that "of course there's a price" is not safe to assume.

**What's needed:** treat existence of a coherent cross-peer price as an assumption to *test*, not a given. The per-peer-local stance partly dodges this (each peer only needs its *own* prices to exist, which they do), but any future *global* aggregation layer (Open fork #3) re-exposes it.

**Canonical sources:** [`network-utility-maximization.md`](network-utility-maximization.md) §"transaction costs"; [Czichowsky et al., SIAM J. Fin. Math. 2014](https://epubs.siam.org/doi/10.1137/130925864).

## 4. Strategy-proofness loss under dynamic demand

DRF's strategy-proofness is proved for a *static* demand snapshot. Asset Fairness (the dot-product) *is* strategy-proof in DRF's table — but it **fails sharing incentive and resource monotonicity**, and resource-monotonicity failure means *adding* resources can *decrease* a user's allocation, which under dynamic demand becomes an incentive to misreport timing. Myrhiza's demand is highly dynamic (peers join/leave, load spikes). Whether *any* of these one-shot game-theoretic properties survive in a repeated, dynamic, per-peer-priced setting is open — and the financial-math friction result (#3) suggests dynamics make it worse, not better.

**What's needed:** evaluate strategy-proofness in the *repeated dynamic* game, not the static one. This is where the directional crediting rule (value received work by my own replacement cost) earns its keep — it removes the counterparty's ability to game the price they're credited at — but the *timing* and *resource-monotonicity* gaming surfaces remain.

**Canonical sources:** [`dominant-resource-fairness.md`](dominant-resource-fairness.md) (Table 2, resource monotonicity), [reciprocity brainstorm](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) #4.

## 5. Allocation vs. metric — the boundary must be specified

The folder's central lesson is "dot-product as cost/credit *metric*, dominant-resource rule for *allocation*" ([`lessons.md`](lessons.md)). But the *interface* between the two — when a credit balance translates into an allocation/queue-priority decision (Open fork #6, the enforcement side) — is unspecified. A metric and an allocation rule that disagree (the metric says a peer is in deficit; the dominant-resource rule says serve them anyway because they're the bottleneck) need a defined precedence.

**What's needed:** the enforcement-side spec (Open fork #6) must say exactly how the `value_P` credit balance feeds the dominant-resource serving decision — which one wins under conflict.

**Canonical sources:** [`lessons.md`](lessons.md), [reciprocity brainstorm](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) Open fork #6.

## Cross-references

- [`README.md`](README.md), [`lessons.md`](lessons.md), [`network-utility-maximization.md`](network-utility-maximization.md), [`dominant-resource-fairness.md`](dominant-resource-fairness.md)
- [reciprocity brainstorm §"What the leading `value()` must answer"](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md)
- [`prior-art/p2p-resource-economics/open-problems.md`](../p2p-resource-economics/open-problems.md), [`prior-art/market-based-control/`](../market-based-control/)

## Sources

All primary sources in the per-topic files' `## Sources`. Anchors: [Kelly et al. 1998](https://www.statslab.cam.ac.uk/~fpk1/rate.pdf), [Ghodsi et al. NSDI 2011](https://www.usenix.org/legacy/events/nsdi11/tech/full_papers/Ghodsi.pdf), [Czichowsky et al. 2014](https://epubs.siam.org/doi/10.1137/130925864).
