**Date:** 2026-05-29
**Status:** active
**Subject:** Resource-pricing theory — the formal foundation for `value_P(action) = resource_vector · shadow_prices_P`, and its single most load-bearing challenge.

# Resource-pricing theory — shadow prices, dual optimality, and the fairness limit

This folder is a focused literature survey, not a single-project deep-dive. It exists to answer one question for a future `myrhiza-participation-*` spec author: **is the dot-product valuation the reciprocity brainstorm landed on — `value_P(action) = resource_vector(action) · shadow_prices_P` — theoretically sound, and where does it break?**

The honest answer is *two-sided*, and this folder carries both halves deliberately:

- **(a) The foundation that makes the dot-product sound — *as a local construction*.** Kelly/Maulloo/Tan's network-utility-maximization (NUM) framework gives the exact identity: a route's price is the **sum of per-link shadow prices** along the route, which *is* `resource_vector · shadow_prices`. Shadow prices are Lagrange multipliers — opportunity costs — by LP duality. This is rigorous. See [`network-utility-maximization.md`](network-utility-maximization.md).
- **(b) The single most load-bearing challenge.** Dominant Resource Fairness (DRF) proves, with a worked counterexample (*Asset Fairness*), that "price every resource and sum" — structurally the same dot-product *with a shared price vector* — can **violate the sharing-incentive property**: a user ends up worse off than under a static equal split. DRF achieves fairness over the resource *vector* with **no prices** and shipped at scale. See [`dominant-resource-fairness.md`](dominant-resource-fairness.md).

The reconciliation — and the load-bearing takeaway — is in [`lessons.md`](lessons.md): **use the dot-product only as a cost/credit *metric*, never as the allocation/fairness rule.** The consumer that needs this is the [reciprocity-economy brainstorm](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md), specifically its §"What the leading `value()` must answer."

## Key facts

| Fact | Value |
|---|---|
| Survey scope | 4 primary works + supporting theory, 1998–2018 |
| Foundational paper | Kelly / Maulloo / Tan, "Rate control for communication networks," *J. Oper. Res. Soc.* **49**(3):237–252, 1998 |
| Load-bearing challenge | Ghodsi et al., "Dominant Resource Fairness," NSDI 2011 — the *Asset Fairness* counterexample |
| The core identity | route price = Σ per-link shadow prices = `resource_vector · shadow_prices`; shadow price = Lagrange multiplier = opportunity cost (LP duality) |
| The core caveat | NUM optimality holds **only** when prices are dual optimizers of a *shared* optimization reconciled via congestion feedback. Purely-local prices are a *valid Lagrangian cost* but a **heuristic**, not a proven dual. |
| Deployed shadow-pricing | DRF (no prices) shipped in Mesos/YARN at scale. Congestion-pricing-as-accountability (ConEx) is **IETF Experimental** — trialed, not production. |
| Verification status | Kelly identity + DRF authors/venue/numbers read off **primary PDFs** (statslab.cam.ac.uk, usenix.org). See per-file `## Sources`. |

## Contents

Each file is independent and ends with `## Sources`.

**Foundation**
- [**Network Utility Maximization**](network-utility-maximization.md) — Kelly/Maulloo/Tan. The route-price = Σ-shadow-prices identity that legitimizes the dot-product; LP duality; dual decomposition; and the critical "local prices forfeit global optimality" caveat. **Read first.**

**Challenge (load-bearing)**
- [**Dominant Resource Fairness**](dominant-resource-fairness.md) — Ghodsi et al. (NSDI 2011) + DRFH (Wang/Liang/Li). The *Asset Fairness* counterexample; the four fairness properties; Leontief non-substitutability. **The reason the dot-product cannot be the allocation rule.**

**Deployment reality**
- [**Congestion pricing**](congestion-pricing.md) — Briscoe's ConEx / re-ECN / re-feedback. The closest thing to deployed shadow-pricing-as-accountability — and its honest status (Experimental, not production).

**Synthesis**
- [**Open problems**](open-problems.md) — commensurability, local-prices-without-reconciliation soundness, strategy-proofness loss under dynamic demand.
- [**Lessons**](lessons.md) — **the consult-when-designing file.** Validates / Avoid / Borrow, framed for the reciprocity model. Cross-links the brainstorm.
- [**Glossary**](glossary.md) — shadow price, Lagrange multiplier/dual, proportional fairness, dominant resource, sharing-incentive, strategy-proofness, envy-freeness, Leontief utility, NUM.

## Reading order

1. [`network-utility-maximization.md`](network-utility-maximization.md) — why the dot-product is a sound *local Lagrangian*.
2. [`dominant-resource-fairness.md`](dominant-resource-fairness.md) — why it must not be the *allocation* rule.
3. [`lessons.md`](lessons.md) — the reconciliation (metric, not allocation).
4. [`open-problems.md`](open-problems.md), [`congestion-pricing.md`](congestion-pricing.md), [`glossary.md`](glossary.md) — depth as needed.

## Glossary stub

Full definitions in [`glossary.md`](glossary.md). The three terms a reader cannot skip: **shadow price** (= Lagrange multiplier = marginal opportunity cost of a binding constraint); **dominant resource** (the resource a user demands the largest *share* of — DRF equalizes shares of *this*, not a priced sum); **sharing-incentive** (no user is worse off than under an equal static partition — the property the dot-product can violate).

## How to use this prior-art doc

Designing the `value()` metric for Myrhiza's reciprocity economy? Read [`network-utility-maximization.md`](network-utility-maximization.md) to understand *why* the dot-product is principled, then [`dominant-resource-fairness.md`](dominant-resource-fairness.md) to understand *where it stops being principled*, then [`lessons.md`](lessons.md) for the metric-not-allocation reconciliation.

**Framing disclosure.** This corpus is written from Myrhiza's **local-subjective-pricing** stance, not a neutral one. Myrhiza's reciprocity ledger is per-peer, non-authoritative, and off the determinism path (see [`determinism.md`](../../specs/2026-05-09-myrhiza-master-design/determinism.md)) — by construction it **cannot** run the cross-peer price reconciliation that NUM's global-optimality proof requires. So this folder reads the literature through the lens *"each peer sets its own shadow prices from its own scarcity, with no reconciliation — what does that buy, and what does it cost?"* That framing foregrounds the duality/Lagrangian foundation (because a single peer's Lagrangian is well-defined) and foregrounds the DRF critique (because it bounds how far a *shared* price vector generalizes — and Myrhiza's are not even shared). It backgrounds the live-market-clearing literature (auctions, tâtonnement convergence) as out-of-scope, because Myrhiza deliberately is not a live market (see [`prior-art/market-based-control/`](../market-based-control/) for that lineage and its deployment-failure evidence). A reader asking *"should Myrhiza price resources at all, vs. flat fair-share?"* should weigh the corpus accordingly: it is a *learn-the-soundness-and-the-limits* artifact for a model already chosen as the leading candidate, not a neutral case for pricing.

**Honest framing on the theory itself.** NUM is rigorous mathematics, but its soundness guarantees (convergence, cross-peer consistency, global optimality) are *theorems about a shared optimization with congestion feedback*. Strip the feedback — as Myrhiza's local model does — and you keep a *valid local Lagrangian cost* but lose the global guarantees. DRF is deployed at scale and shipped *without* prices, which is itself the strongest evidence that priced summation is not the only — or the fairest — way to allocate a resource vector. The right posture for a spec author: borrow NUM's *vocabulary* and its *local construction*, label the local version a **trust-minimal heuristic** (not a proven market clearing), and never let the dot-product govern allocation.

## Cross-links

- [`reports/2026-05-29-reciprocity-economy-brainstorm/README.md`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) §"What the leading `value()` must answer" — **the consumer.** This folder supplies its citations #1 (DRF), #2 (Leontief), #5 (soundness honesty).
- [`prior-art/p2p-resource-economics/`](../p2p-resource-economics/) — where the *model* lives (OurGrid, GNUnet, Samsara). This folder is the *theory under* it.
- [`prior-art/market-based-control/`](../market-based-control/) — the computational-economy paradigm `value_P` descends from + its deployment-failure evidence (the "markets are overkill" lineage).
- [`prior-art/sybil-resistance/`](../sybil-resistance/) — the orthogonal admission layer; self-reported-cost verification (BOINC) lives there.
- [`specs/2026-05-09-myrhiza-master-design/determinism.md`](../../specs/2026-05-09-myrhiza-master-design/determinism.md) — why the ledger (and its prices) are per-peer and non-authoritative.

## Sources

Full citations in each per-topic file's `## Sources`. Anchor primaries:

- [Kelly, Maulloo & Tan, "Rate control for communication networks: shadow prices, proportional fairness and stability," *J. Oper. Res. Soc.* 49(3):237–252, 1998](https://www.statslab.cam.ac.uk/~fpk1/rate.pdf) — read off primary PDF.
- [Ghodsi, Zaharia, Hindman, Konwinski, Shenker & Stoica, "Dominant Resource Fairness: Fair Allocation of Multiple Resource Types," NSDI 2011](https://www.usenix.org/legacy/events/nsdi11/tech/full_papers/Ghodsi.pdf) — read off primary PDF.
- [Wang, Liang & Li, "Multi-Resource Fair Allocation in Heterogeneous Cloud Computing Systems," *IEEE TPDS* 26(10):2822–2835, 2015](https://www.comm.utoronto.ca/~liang/publications/TPDS_DRFH.pdf).
- [Briscoe et al., "Congestion Exposure (ConEx) Concepts, Abstract Mechanism, and Requirements," RFC 7713 (2015)](https://www.rfc-editor.org/rfc/rfc7713.html); [RFC 7837, IPv6 Destination Option for ConEx (Experimental, 2016)](https://www.rfc-editor.org/rfc/rfc7837.html).
- [Czichowsky, Muhle-Karbe & Schachermayer, "Transaction Costs, Shadow Prices, and Duality in Discrete Time," *SIAM J. Financial Math.* 5(1):258–278, 2014](https://epubs.siam.org/doi/10.1137/130925864).
