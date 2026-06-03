**Date:** 2026-05-29
**Status:** active
**Subject:** Glossary for resource-pricing theory — shadow price, Lagrange multiplier/dual, proportional fairness, dominant resource, sharing-incentive, strategy-proofness, envy-freeness, Leontief utility, NUM.

# Glossary — resource-pricing theory

Terms used across this folder. Each entry: definition + where it bites for Myrhiza's `value_P = resource_vector · shadow_prices_P`.

**NUM (Network Utility Maximization).** The framework (Kelly/Maulloo/Tan 1998) that recasts resource sharing as maximizing aggregate user utility subject to capacity constraints, solved distributedly via price signals. The origin of "shadow price = Lagrange multiplier" in networking. *Myrhiza:* the dot-product valuation is NUM's route-price. See [`network-utility-maximization.md`](network-utility-maximization.md).

**Shadow price.** The marginal value of relaxing a constraint by one unit — the *opportunity cost* of a scarce resource's last unit. On a binding capacity constraint it is positive; on a slack one it is zero (complementary slackness). *Myrhiza:* a peer's `shadow_prices_P` are its own marginal opportunity costs per resource (scarce disk → high price). Locally well-defined; globally unreconciled.

**Lagrange multiplier / dual.** In constrained optimization, the multiplier `μ` attached to a constraint in the Lagrangian; at the optimum it equals the shadow price (envelope theorem). The **dual problem** optimizes over these multipliers. **LP duality** is the linear-programming case where primal optimum = dual optimum. *Myrhiza:* shadow_prices being Lagrange multipliers is *why* the dot-product is principled — but only as the dual of an actual shared optimization. Unreconciled local prices are not dual optimizers of anything global.

**Opportunity cost.** The value of the best alternative forgone by committing a resource to a given use. The economic content of a shadow price: a scarce link's `μ_j` is what the system gives up elsewhere by spending its last unit of capacity here. *Myrhiza:* the intuition behind "price a resource by how scarce it is to *me*" — a peer's price for a resource is what *it* forgoes by spending that resource serving someone else.

**Complementary slackness.** The LP-duality condition `μᵀ(C − Ax) = 0`: a constraint's shadow price is nonzero *only* if the constraint binds (the resource is saturated); slack resources price at zero. *Myrhiza:* a peer should charge for a resource only when that resource is actually scarce *to it* — abundant resources are free to give, which is what produces comparative-advantage gains from trade.

**Weighted DRF.** DRF generalized so each user `i` carries a weight vector `W_i`; the dominant share becomes `max_j {u_{i,j}/w_{i,j}}`. Lets the policy favor users who contributed more resources or run more important jobs. *Myrhiza:* the hook for letting a peer's *prior contribution* (its standing) bias the dominant-resource serving decision — the natural bridge between the credit metric and the allocation rule.

**Proportional fairness.** An allocation `x` is proportionally fair if, for any feasible alternative `x*`, the aggregate of proportional changes `Σ_r (x*_r − x_r)/x_r ≤ 0`. Equivalent to maximizing `Σ_r log(x_r)`; it is the Nash bargaining solution. The fairness criterion NUM's prices implement. *Myrhiza:* the fairness notion that *priced* NUM achieves — contrast DRF's dominant-resource max-min, which needs no prices.

**Dominant resource.** For a user, the resource it holds the largest *share* (fraction of total) of. A CPU-bound job's dominant resource is CPU; an I/O-bound job's is bandwidth. *Myrhiza:* DRF equalizes dominant *shares*, not priced sums — the bottleneck-aware allocation rule to pair with the dot-product metric. See [`dominant-resource-fairness.md`](dominant-resource-fairness.md).

**Sharing-incentive.** A fairness property: no user is worse off under the shared policy than under a static equal `1/n` partition. *Myrhiza:* the property that **priced summation (Asset Fairness) provably violates** (DRF Theorem 1) and DRF satisfies — the single most load-bearing reason the dot-product must not be the allocator.

**Strategy-proofness.** A user cannot benefit by *lying about its resource demands* (incentive compatibility). *Myrhiza:* Asset Fairness has it statically; the question is whether it survives dynamic, repeated, per-peer-priced play (open problem #4). Directional crediting (value received work by my own replacement cost) removes the counterparty's lever on its credited price.

**Envy-freeness.** No user prefers another user's allocation to its own. *Myrhiza:* satisfied by DRF, Asset Fairness, and CEEI alike — not the discriminating property; sharing-incentive is.

**Leontief utility (fixed-proportions / non-substitutability).** Utility from resources consumed in a *fixed recipe*; extra of a non-bottleneck resource has zero marginal value (a CPU-blocked task gains nothing from more RAM). The demand model DRF assumes and clouds bundle prices around. *Myrhiza:* the reason a *fixed* price vector misprices bottlenecked actions — collapsing the resource vector to a scalar before the binding resource is known loses information. See [`dominant-resource-fairness.md`](dominant-resource-fairness.md).

**Pareto efficiency.** No user's allocation can be increased without decreasing another's. *Myrhiza:* satisfied by DRF and by priced allocators; necessary but not sufficient — does not by itself imply fairness.

**Congestion-volume.** ConEx's accountability metric: the volume of bytes dropped or ECN-marked over a period — an attributable, network-measured shadow-price signal. *Myrhiza:* the model for placing a price where the accountable party cannot forge it. See [`congestion-pricing.md`](congestion-pricing.md).

**CEEI (Competitive Equilibrium from Equal Incomes).** Microeconomics' preferred fair-division mechanism: each user gets `1/n` of every resource, then trades in a perfect market; the Nash-bargaining outcome. Envy-free and Pareto-efficient but **not strategy-proof** (DRF §6.1.2). *Myrhiza:* the "market clearing" baseline DRF rejects for gameability — a caution against assuming a market-equilibrium price is safe.

**Asset Fairness.** DRF's priced-summation baseline: assign each resource a price, equalize aggregate value `Σ_j price_j · share_{i,j}` across users. *Structurally identical to `resource_vector · shadow_prices` used as an allocation rule.* Violates sharing-incentive (Theorem 1). *Myrhiza:* the named form of the dot-product-as-allocator anti-pattern.

**Tâtonnement / Walrasian auctioneer.** The idealized iterative price-adjustment process by which a market is imagined to reach equilibrium (raise prices on over-demanded goods, lower on under-demanded). Kelly notes his congestion algorithms are a real embodiment of this otherwise "implausible" construct. *Myrhiza:* out of scope by design — Myrhiza runs no live auction; see [`prior-art/market-based-control/`](../market-based-control/).

## Cross-references

- [`README.md`](README.md), [`network-utility-maximization.md`](network-utility-maximization.md), [`dominant-resource-fairness.md`](dominant-resource-fairness.md), [`congestion-pricing.md`](congestion-pricing.md), [`lessons.md`](lessons.md), [`open-problems.md`](open-problems.md)

## Sources

Definitions read from the primary sources cited in [`network-utility-maximization.md`](network-utility-maximization.md) and [`dominant-resource-fairness.md`](dominant-resource-fairness.md): [Kelly et al. 1998](https://www.statslab.cam.ac.uk/~fpk1/rate.pdf), [Ghodsi et al. NSDI 2011](https://www.usenix.org/legacy/events/nsdi11/tech/full_papers/Ghodsi.pdf).
