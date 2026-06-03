**Date:** 2026-05-29
**Status:** active
**Subject:** Kelly/Maulloo/Tan NUM — the route-price = Σ-shadow-prices identity that makes the dot-product a sound *local* Lagrangian, and the caveat that global optimality needs cross-peer reconciliation.

# Network Utility Maximization — the foundation, and its precondition

## The paper

Kelly, Maulloo & Tan, **"Rate control for communication networks: shadow prices, proportional fairness and stability,"** *Journal of the Operational Research Society* **49**(3):237–252, March 1998. (Authors, title, venue, volume/issue/pages verified off the primary PDF at statslab.cam.ac.uk.) This is the founding paper of **Network Utility Maximization (NUM)** — the framework that recasts congestion control as a distributed solution to a constrained optimization, and the canonical source for "shadow price = Lagrange multiplier = opportunity cost."

## The setup

A network is a set `J` of resources (links), each with capacity `C_j`. A *route* `r` is a subset of links; the `0/1` matrix `A` records which links each route uses (`A_{jr}=1` iff route `r` uses link `j`). Each user sends at rate `x_r` over its route and derives utility `U_r(x_r)` (increasing, strictly concave). The **SYSTEM problem** maximizes aggregate utility `Σ_r U_r(x_r)` subject to the capacity constraints `Ax ≤ C`.

Because the network does not know each user's utility `U_r`, Kelly *decomposes* SYSTEM into:

- **USER_r** — given a per-unit price, each user chooses how much to pay (`w_r`);
- **NETWORK(A, C; w)** — given the willingness-to-pay vector `w`, the network maximizes `Σ_r w_r log x_r` subject to `Ax ≤ C`.

The decomposition is the load-bearing move: it lets a decentralized system reach the global optimum **without the network ever knowing the utilities** — *provided the two sub-problems are coupled by a price signal*.

## The identity — this is the dot-product

Form the Lagrangian for NETWORK:

> `L(x, z, μ) = Σ_r w_r log(x_r) + μᵀ(C − Ax − z)`

where `z ≥ 0` are slack variables and **`μ` is the vector of Lagrange multipliers, which Kelly names "shadow prices" in the same breath.** Setting `∂L/∂x_r = 0` gives the unique primal optimum (Kelly's equation (3)):

> `x_r = w_r / ( Σ_{j ∈ r} μ_j )`

The denominator `Σ_{j ∈ r} μ_j` is **the sum of the per-link shadow prices along route `r`**. That sum *is* the price of the route. Written as a dot product: if `a_r` is route `r`'s incidence vector over links (its *resource vector* — which links it consumes), then

> route price of `r`  =  `a_r · μ`  =  `Σ_{j} a_{jr} μ_j`  =  `Σ_{j ∈ r} μ_j`.

This is **exactly the structural form of `value_P(action) = resource_vector(action) · shadow_prices_P`.** Kelly's primal dynamics (his equations (5)–(6)) confirm the reading: the network adjusts each rate so as to "equalise the aggregate cost of this flow, `x_r · Σ_{j∈r} μ_j`, with a target value `w_r`." The cost of using a route is the dot product of *what it consumes* (the per-link usage vector) with *what each unit is worth* (the per-link shadow price). The reciprocity model's dot-product is not an analogy to NUM — it is the same object.

## Why shadow price = opportunity cost (LP duality)

A Lagrange multiplier on a binding capacity constraint is, by the envelope theorem / LP duality, the **marginal increase in the optimal objective per unit of relaxed capacity** on that link. Equivalently: the *opportunity cost* of the link's last unit of capacity — what the system forgoes by spending capacity here rather than on the next-best use. A scarce link (binding constraint) carries a high `μ_j`; a slack link carries `μ_j = 0` (complementary slackness: `μᵀ(C − Ax) = 0`, Kelly's relation (4)). This is precisely the intuition the reciprocity model wants: *price a resource by how scarce it is to me.*

## The critical caveat — soundness needs reconciliation

Here is the precondition a spec author must not lose. NUM's good properties — **convergence to the system optimum, consistency across all participants, and global Pareto-optimality** — are theorems about a *specific* situation:

1. **The prices `μ` are the dual optimizers of a *single, shared* optimization** (SYSTEM). There is one `μ_j` per link, seen by everyone routing through it.
2. **Those prices are reached by congestion feedback.** Kelly's whole stability result is that the *dual algorithm* — where each resource raises `μ_j` as its measured load `Σ_{s: j∈s} x_s` rises, and users back off in response — converges to the dual optimum. The feedback loop *is* what reconciles every participant onto the same price.

Strip either condition and the guarantees go with them. **A peer that sets its prices purely from its *own* local scarcity, with no cross-peer price exchange or congestion feedback, computes a perfectly valid Lagrangian cost *for its own constrained problem* — but it has not solved any shared optimization.** Its prices are not the dual optimizers of anything global. The resulting valuation is a **heuristic**: locally coherent, cross-peer *inconsistent* by construction, and carrying none of NUM's optimality. This is the soundness-honesty point the brainstorm flagged ([report §"What the leading `value()` must answer"](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) #5). Myrhiza's prices *must* be labeled a trust-minimal local Lagrangian, **not** a proven dual or a market clearing — the model's "value by MY OWN replacement cost" forgoes the reconciliation that would make it a dual, deliberately, for trust-minimality.

## Dual decomposition — what reconciliation costs

NUM's modern descendant, **dual decomposition** (Palomar & Chiang's "Layering as Optimization Decomposition," 2006, surveying the lineage), makes the price-exchange requirement explicit: to solve a *coupled* multi-agent problem by per-agent subproblems, the agents must exchange (and typically *average* / sub-gradient-update) the prices on the shared constraints until they agree. No exchange → no agreement → no global optimum. For Myrhiza this is the formal statement of *why* per-peer prices cannot converge: there is no price-exchange channel on the determinism path, and adding one would re-introduce the global ledger the model rejects.

## Shadow prices may fail to exist at all — transaction costs

A second, sharper limit: shadow prices are not even guaranteed to *exist* once the frictionless assumption breaks. **Czichowsky, Muhle-Karbe & Schachermayer ("Transaction Costs, Shadow Prices, and Duality in Discrete Time," *SIAM J. Financial Math.* 5(1):258–278, 2014; arXiv:1205.4643)** give an explicit counterexample: for a log-investor in an *arbitrage-free market with bounded prices and arbitrarily small proportional transaction costs*, a shadow price (a frictionless "least favorable" price reproducing the optimum) **may fail to exist globally**. Their companion result ("Shadow prices for continuous processes," Czichowsky & Schachermayer, arXiv:1408.6065) shows existence needs continuity + "no unbounded profit with bounded risk," and a counterexample shows these cannot be relaxed. The lesson transfers: **any friction on the exchange — and a per-peer reciprocity ledger with directional, non-reproducible crediting is shot through with friction — can void the clean "there is a single consistent shadow price" assumption the frictionless NUM identity rests on.** Treat the existence of a coherent global shadow price as an *assumption to be checked*, not a given.

## Implications for Myrhiza

- The dot-product is **principled as a local construction**: it is literally NUM's route-price. Borrow the vocabulary and the form with confidence.
- It is **not principled as a global market**: independently-set local prices are not dual optimizers; do not claim NUM's optimality/convergence. Label it a heuristic.
- The reconciliation NUM needs (congestion feedback / price averaging) is exactly what Myrhiza's non-authoritative, off-determinism ledger *cannot* provide — which is *why* subjective real-time pricing is even possible here, and *why* it can only ever be local. (See [`determinism.md`](../../specs/2026-05-09-myrhiza-master-design/determinism.md).)
- Transaction-cost friction can make a globally-consistent shadow price *non-existent*, not merely unreconciled — a second reason not to over-claim.

## Sources

- [Kelly, Maulloo & Tan, "Rate control for communication networks: shadow prices, proportional fairness and stability," *J. Oper. Res. Soc.* 49(3):237–252, 1998](https://www.statslab.cam.ac.uk/~fpk1/rate.pdf) — primary PDF; the Lagrangian, equations (3)–(6), and the SYSTEM/USER/NETWORK/DUAL decomposition quoted above are read off it.
- [Palomar & Chiang, "A Tutorial on Decomposition Methods for Network Utility Maximization," *IEEE JSAC* 24(8):1439–1451, 2006](https://www.princeton.edu/~chiangm/decomposition.pdf) — dual-decomposition / price-exchange requirement (citation by title; not re-verified off PDF — confidence medium).
- [Czichowsky, Muhle-Karbe & Schachermayer, "Transaction Costs, Shadow Prices, and Duality in Discrete Time," *SIAM J. Financial Math.* 5(1):258–278, 2014](https://epubs.siam.org/doi/10.1137/130925864) (preprint [arXiv:1205.4643](https://arxiv.org/abs/1205.4643)) — non-existence counterexample.
- [Czichowsky & Schachermayer, "Shadow prices for continuous processes," arXiv:1408.6065, 2014/2015](https://arxiv.org/abs/1408.6065) — existence conditions + counterexample.
- Cross-refs: [`dominant-resource-fairness.md`](dominant-resource-fairness.md), [`open-problems.md`](open-problems.md), [`lessons.md`](lessons.md), [`glossary.md`](glossary.md).
