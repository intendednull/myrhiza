**Date:** 2026-05-29
**Status:** active
**Subject:** The decision file — what resource-pricing theory validates about `value_P = resource_vector · shadow_prices_P`, what to avoid, what to borrow. Read when specifying the reciprocity `value()` metric.

# Lessons for Myrhiza — resource-pricing theory

Synthesis across [`network-utility-maximization.md`](network-utility-maximization.md), [`dominant-resource-fairness.md`](dominant-resource-fairness.md), [`congestion-pricing.md`](congestion-pricing.md), [`open-problems.md`](open-problems.md). Format: validates / avoid / borrow. These are *lessons captured from the literature*, not Myrhiza decisions — promote real decisions into a `myrhiza-participation-*` spec.

The single throughline, stated once: **use the dot-product only as a cost/credit *metric*, never as the resource-*allocation* / fairness rule.**

## Validates

1. **The dot-product is the NUM route-price — a sound *local* Lagrangian.** Kelly/Maulloo/Tan's optimum rate is `x_r = w_r / Σ_{j∈r} μ_j`; the route price `Σ_{j∈r} μ_j = resource_vector · μ` *is* `value_P(action) = resource_vector(action) · shadow_prices_P`. Shadow prices are Lagrange multipliers — marginal opportunity costs — by LP duality. So pricing an action by its resource recipe dotted with per-resource scarcity is **not ad hoc; it is the canonical congestion-pricing construction.** *Source: [`network-utility-maximization.md`](network-utility-maximization.md).*

2. **Subjective per-peer scarcity prices are coherent — as each peer's own Lagrangian.** A peer's prices being its *own* marginal opportunity costs (fast CPU → compute cheap; scarce disk → persistence dear) is exactly the LP-dual reading. Each peer's local valuation is internally well-posed and produces comparative-advantage gains from trade. *Source: [`network-utility-maximization.md`](network-utility-maximization.md).*

3. **Scarcity-price-as-accountability is a standards-worthy idea.** Briscoe's ConEx took "make a resource's shadow price legible and attributable to the consumer" all the way to IETF Experimental. The instinct behind Myrhiza's credit/debit-by-cost is one serious engineers have pursued. *Source: [`congestion-pricing.md`](congestion-pricing.md).*

4. **Off-determinism, local-only pricing is the *right* home for subjective prices.** NUM prices can only converge via cross-peer congestion feedback; Myrhiza's determinism boundary forbids that feedback on the authoritative path. So subjective real-time prices *must* live in a per-peer, non-authoritative component — which is precisely where the brainstorm put the ledger. The constraint and the design agree. *Source: [`network-utility-maximization.md`](network-utility-maximization.md) §caveat; [`determinism.md`](../../specs/2026-05-09-myrhiza-master-design/determinism.md).*

## Avoid

| Pitfall | Source | Mitigation |
|---|---|---|
| **Using the dot-product as the allocation / fairness rule.** Asset Fairness *is* `resource_vector · price_vector` as an allocation rule, and DRF's **Theorem 1 proves it violates sharing incentive** — a user ends up worse off than under a static equal split. The one property a reciprocity economy most needs is the one priced summation breaks. | [`dominant-resource-fairness.md`](dominant-resource-fairness.md) | Confine the dot-product to crediting the per-peer ledger. Govern *what gets served* with a DRF/DRFH dominant-resource max-min rule. |
| **Claiming NUM's optimality/convergence for local prices.** Independently-set, unreconciled prices are *not* dual optimizers of any shared problem. Borrowing NUM's rigor without its congestion-feedback precondition is over-claiming. | [`network-utility-maximization.md`](network-utility-maximization.md) | Label the local construction a **trust-minimal heuristic** — a valid local Lagrangian, not a proven dual or market clearing. State this in the spec. |
| **Collapsing the resource vector to a scalar before the bottleneck is known.** Leontief/non-substitutable demands mean a fixed price vector misprices bottlenecked actions; a CPU-blocked job gains nothing from RAM. Clouds bundle prices for this reason. | [`dominant-resource-fairness.md`](dominant-resource-fairness.md) | Price against the *binding* resource (max-over-resources) or carry the vector and collapse last. |
| **Assuming a globally-coherent shadow price exists.** Under transaction-cost friction (and a reciprocity ledger is full of it), shadow prices can fail to exist *at all* (Czichowsky–Muhle-Karbe–Schachermayer). | [`network-utility-maximization.md`](network-utility-maximization.md) §transaction costs | Keep prices per-peer-local (each peer's own prices trivially exist). Treat any future *global* aggregation as re-exposing this and verify existence there. |
| **Citing ConEx as "deployed."** It is IETF **Experimental** — trialed, never production. A prior pass over-claimed this. | [`congestion-pricing.md`](congestion-pricing.md) | Say "standardized as Experimental and trialed, not production." Its non-deployment is itself a lesson about cross-trust-boundary coordination cost. |

## Borrow

1. **The NUM route-price construction, as the credit/debit metric.** Adopt `value_P(action) = resource_vector(action) · shadow_prices_P` directly for ledger crediting — it is the canonical form, and naming it as NUM's route-price gives the spec a rigorous pedigree for the *metric*. *See [`network-utility-maximization.md`](network-utility-maximization.md).*

2. **DRF/DRFH dominant-resource bottleneck rule, for allocation.** When deciding what to serve under contention (the enforcement side, Open fork #6), use dominant-resource max-min — it satisfies sharing incentive, strategy-proofness, envy-freeness, Pareto efficiency over the resource *vector* with no prices, and **shipped at scale (Mesos/YARN)**. DRFH is the closer analogue since peers are heterogeneous servers. *See [`dominant-resource-fairness.md`](dominant-resource-fairness.md).*

3. **Honest "heuristic, not proven dual" labeling.** The most important thing to borrow is *epistemic discipline*: present the local dot-product as a trust-minimal heuristic with a sound local-Lagrangian pedigree, and be explicit that it forfeits NUM's global guarantees by forgoing reconciliation. This is the difference between a defensible spec and an over-claim. *See [`network-utility-maximization.md`](network-utility-maximization.md), [reciprocity brainstorm](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) #5.*

4. **ConEx's metric-placement discipline.** Measure the accountable price where the accountable party cannot forge it (congestion-volume measured by the network, re-inserted verifiably). Myrhiza's analogue: credit *received* work by the recipient's *own* measured replacement cost, never the sender's self-report. *See [`congestion-pricing.md`](congestion-pricing.md), [`prior-art/sybil-resistance/`](../sybil-resistance/).*

## The single most important lesson

**The dot-product is the right *metric* and the wrong *allocator*.** It is literally NUM's route-price (sound, as a local Lagrangian) — so credit/debit the per-peer ledger with it, label it a trust-minimal heuristic, and **never** let it decide allocation. For allocation, use a DRF-style dominant-resource rule, which provably preserves the sharing incentive that priced summation provably breaks, and which shipped at scale. Everything else in this folder is support for that one sentence.

## Cross-references

- [`README.md`](README.md), [`network-utility-maximization.md`](network-utility-maximization.md), [`dominant-resource-fairness.md`](dominant-resource-fairness.md), [`congestion-pricing.md`](congestion-pricing.md), [`open-problems.md`](open-problems.md), [`glossary.md`](glossary.md)
- [reciprocity brainstorm §"What the leading `value()` must answer"](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) — this folder supplies its citations #1, #2, #5.
- [`prior-art/p2p-resource-economics/lessons.md`](../p2p-resource-economics/lessons.md) — where the model lives; [`prior-art/market-based-control/`](../market-based-control/) — the "markets are overkill" lineage; [`prior-art/sybil-resistance/`](../sybil-resistance/) — self-reported-cost verification.

## Sources

All primary sources in the per-topic files' `## Sources`.
