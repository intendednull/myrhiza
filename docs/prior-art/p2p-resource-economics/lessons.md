**Date:** 2026-05-29
**Status:** active
**Subject:** The decision file — what token-free heterogeneous reciprocity prior art validates, what to avoid, what to borrow, for the no-token per-peer reciprocity model. Read when designing the participation/valuation spec.

# Lessons — token-free heterogeneous reciprocity

Synthesis across [`ourgrid.md`](ourgrid.md), [`gnunet.md`](gnunet.md), [`samsara.md`](samsara.md), and [`credit-and-scrip.md`](credit-and-scrip.md). Format: **Validates / Avoid / Borrow**.

**This file captures lessons, not decisions.** Myrhiza has *not* decided to adopt any of this — the consumer is the exploratory brainstorm at [`reports/2026-05-29-reciprocity-economy-brainstorm/`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md), whose `value()` rule is a *leading candidate*, not a spec. Decisions get promoted to `docs/specs/` separately. Read this as "here is what the literature would tell a spec author," not "here is what we are building."

## Validates

1. **Token-free, fully-decentralized, local-per-peer reciprocity is real and was deployed.** OurGrid's Network of Favors ran a production grid on exactly this — local favor balances, no currency, no central ledger. The leading Myrhiza model is not unprecedented; its closest realization shipped. *Source: [`ourgrid.md`](ourgrid.md).*

2. **Heterogeneous reciprocity marginalizes free riders even in the hardest regime.** OurGrid's multi-service extension marginalizes free riders *even when the donor's cost is nearly as large as the recipient's utility* — the worst case for any reciprocity scheme. "Peers swap services they value differently" is empirically robust. *Source: [`ourgrid.md`](ourgrid.md).*

3. **A private, non-converged, per-peer trust scalar is enough to run a real network.** GNUnet has done it for ~20 years (still maintained; 0.27.0 in 2026) with trust kept purely local — "nodes can only trust their own records." This validates keeping Myrhiza's ledger *off* the determinism path, in a per-peer behavior component. *Source: [`gnunet.md`](gnunet.md).*

4. **Non-negativity defeats whitewashing for free.** Both OurGrid (non-negative local reputation) and GNUnet (fresh identity = zero trust, low exchange rate) make a freshly-minted identity gain *nothing*. GNUnet bounds attacker damage at **d ≤ c + ε** — capacity contributed plus harmless excess. The brainstorm's "no negative credit minted for free" rests on the same lever. *Source: [`ourgrid.md`](ourgrid.md), [`gnunet.md`](gnunet.md).*

5. **Replacement-cost crediting is sound and trust-minimal — in one resource.** Samsara prices received storage in *the same resource the recipient gives up* (an incompressible claim), needing no token and no trusted third party. This validates the *form* of Myrhiza's "credit received work at my own replacement cost." *Source: [`samsara.md`](samsara.md).*

6. **"No global token, ever" is empirically well-founded.** Four independent failure modes: Karma needs a bank-set + monetary policy; Maze got wash-traded and whitewashed at scale; Dandelion needs a central server; MojoNation hyperinflated and folded. The no-token lock is supported by evidence, not taste. *Source: [`credit-and-scrip.md`](credit-and-scrip.md).*

## Avoid

| Pitfall | Source | Why per-peer vectors escape it |
|---|---|---|
| **Global fungible scrip.** A transferable, globally-honored unit invites wash-trading (mint at colluder, spend at victim), hoarding/inflation, and a settlement authority. | [`credit-and-scrip.md`](credit-and-scrip.md) (Karma, Maze, MojoNation) | Per-peer non-transferable balances mean colluders only inflate their *mutual* books, which buys nothing from a third party. |
| **A bank-set / quorum / central clearer to make credit safe.** Karma's safety *requires* a bank-set + inflation corrections; Dandelion's robustness *requires* a central server. Soundness bought with centralization. | [`credit-and-scrip.md`](credit-and-scrip.md) | Nothing transfers, so nothing needs clearing; no double-spend, no bank. |
| **Trusting a self-reported provider cost as the credit.** A self-measured resource vector is a credit-stuffing surface. | [`gnunet.md`](gnunet.md), [`open-problems.md`](open-problems.md) §2 | Credit *received* work by the recipient's own replacement cost (GNUnet credits requester-declared priority capped by trust) — the provider's number never enters the credit. |
| **Forced same-resource symmetry.** Samsara's 1:1 storage-for-storage is clean but *destroys specialization* — abundant-disk and abundant-CPU peers cannot make a beneficial uneven trade. | [`samsara.md`](samsara.md) | The whole value of a resource vector + subjective prices is to leave this regime. Don't collapse to one resource except inside a trust domain. |
| **Global eigenvector reputation for transitivity.** EigenTrust-style aggregation is Sybil-fragile (cliques vouch for themselves). | [`open-problems.md`](open-problems.md) §3, [`prior-art/sybil-resistance/eigentrust.md`](../sybil-resistance/eigentrust.md) | Prefer GNUnet's delegation-with-margin (reduce priority per hop, charge the forwarder) if transitivity is ever wanted. |
| **Pricing everything, everywhere.** Dot-product per action has overhead that flat fair-share avoids below a threshold. | [`open-problems.md`](open-problems.md) §4 | Flat fair-share *within* a trust domain (the "firm"); `value_P` only *across* trust boundaries (the "market"). |
| **Claiming OurGrid/Samsara already solved the exchange rate.** OurGrid explicitly leaves cross-service exchange-rate derivation open; Samsara only handles 1:1 same-resource. | [`ourgrid.md`](ourgrid.md), [`open-problems.md`](open-problems.md) §1 | The exchange-rate rule is Myrhiza's *contribution*, not adopted prior art — don't overclaim. |

## Borrow

1. **OurGrid's local-reputation-from-two-numbers.** Each peer computes standing for a counterparty from just (favors received, favors given), non-negative, tracking only nonzero-balance counterparties. Tiny state, deployed. The base shape of the per-peer ledger. *See [`ourgrid.md`](ourgrid.md).*

2. **GNUnet's excess rule.** Serve free when idle, charge (spend trust) only under load, drop lowest-effective-priority requests first. This *is* the newcomer-bootstrap answer (free service from excess capacity) and keeps free-riding cheap to tolerate — without a bolted-on optimistic-unchoke slot. *See [`gnunet.md`](gnunet.md).*

3. **GNUnet's `min(requested, held-trust)` cap + delegation-with-margin.** Effective priority capped by earned trust is the brainstorm's "priority capped by earned trust" exactly. Delegation-with-margin is the Sybil-safe path to transitivity if that fork is ever resolved. *See [`gnunet.md`](gnunet.md).*

4. **Samsara's same-unit crediting + challenge-response.** When two peers exchange the *same* resource, the unit *is* the value — no valuation function needed (use this inside trust domains). Periodic challenge-to-prove-possession is a cheap token-free verification primitive. *See [`samsara.md`](samsara.md).*

5. **FairTorrent's deficit counter / serve-whom-you-owe-most.** The cleanest deployed single-resource instance of the net-imbalance standing curve. Generalize from bytes to the resource vector. *See [`credit-and-scrip.md`](credit-and-scrip.md).*

6. **Tahoe-LAFS friendnet refusal-as-enforcement.** Per-account quota with lease refusal once over budget — deployed, token-free, per-relationship. The brainstorm's kernel-mediated refusal (Open fork #6) made first-class. *See [`credit-and-scrip.md`](credit-and-scrip.md).*

## The single most important lesson

**The leading model is the asymmetric, heterogeneous generalization of three deployed token-free systems — and the one part none of them solved is the cross-resource exchange rate, which is exactly the part Myrhiza proposes to add.** OurGrid gives the heterogeneous reciprocity and the free-rider-marginalization result; GNUnet gives the excess rule, the non-negative-trust Sybil bound (d ≤ c + ε), and delegation-with-margin; Samsara gives replacement-cost crediting in one resource and marks the symmetric floor where specialization dies. The scrip systems give the four-way proof that a global token is the wrong turn. The composition — resource vector × subjective shadow-prices, credited at the recipient's replacement cost, off the determinism path, gated by a social graph and a capability kernel — is novel and unproven; every *piece* has prior art and every piece's *limits* are documented here.

## Cross-references

- [`README.md`](README.md), [`open-problems.md`](open-problems.md), [`glossary.md`](glossary.md), per-system files.
- [`reports/2026-05-29-reciprocity-economy-brainstorm/README.md`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) — the consumer; "Locked decisions", "Open forks", "model challenges".
- [`prior-art/sybil-resistance/lessons.md`](../sybil-resistance/lessons.md) — the Sybil/reciprocity decision file this complements (reciprocity-beats-reputation, social-graph-as-admission).

## Sources

All sources in the per-system files in this folder.
