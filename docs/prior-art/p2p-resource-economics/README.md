**Date:** 2026-05-29
**Status:** active
**Subject:** Token-free, heterogeneous, per-peer P2P resource economics — the closest real precedents for a no-token reciprocity economy with per-peer subjective valuation.

# P2P resource economics — token-free, heterogeneous, per-peer reciprocity

This folder is a multi-system survey, not a single-project deep-dive. It collects the published systems that come closest to the reciprocity model the Myrhiza brainstorm ([`reports/2026-05-29-reciprocity-economy-brainstorm/`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md)) calls leading:

> `value_P(action) = resource_vector(action) · shadow_prices_P`

— a module declares the objective resource recipe (CPU-ms, byte-hours, bandwidth); each peer applies its own subjective shadow-prices; work I *do* is valued by my measured cost, work I *receive* by my own replacement cost. Per-peer, local, **no global token**, with the ledger kept off the determinism path.

That model is a *synthesis*, and its halves exist separately in the wild. This folder maps where:

- **OurGrid's Network of Favors** — the heterogeneous, per-peer, token-free reciprocity half (and the free-rider-marginalization result). The closest published realization. **Priority-1.**
- **GNUnet's excess-based model** — the "value work I do by my own provider cost" half, plus a structural Sybil bound and a Sybil-safe transitivity mechanism. **The single highest-value system the corpus was missing.**
- **Samsara** — the replacement-cost-crediting half, in one resource, and the symmetric floor where specialization dies.
- **The scrip systems** (Karma, Maze, Dandelion, MojoNation) — the cautionary foils: why a global fungible token is the wrong turn.

**Why this folder exists.** The brainstorm's gap analysis flagged that the corpus had *no* folder where the leading valuation model's actual precedents live — OurGrid, GNUnet, and Samsara were all absent. This folder fills that gap. It is the resource-*economics* companion to [`prior-art/sybil-resistance/`](../sybil-resistance/), which covers the orthogonal Sybil/free-riding-*defense* literature.

## Key facts

| Fact | Value |
|---|---|
| Survey scope | 6 token-free reciprocity systems + 2 single-resource fairness points, 2003–2009 |
| Closest realization of the leading model | OurGrid Network of Favors + multi-service extension (HPDC 2004 / IEEE P2P 2006) |
| Highest-value previously-missing system | GNUnet excess-based model (Grothoff, Wirtschaftsinformatik 2003) — still actively maintained (0.27.0, 2026) |
| Replacement-cost precedent | Samsara (Cox & Noble, SOSP 2003) — storage-for-storage, ~100% overhead |
| Global-scrip failure modes catalogued | bank-set + monetary policy (Karma); wash-trading + whitewashing (Maze); central server (Dandelion); hyperinflation + abandonment (MojoNation) |
| GNUnet attacker-damage bound | d ≤ c + ε (contributed capacity plus harmless excess) |
| Deployed token-free single-resource fairness | FairTorrent deficit counter (CoNEXT 2009); Tahoe-LAFS friendnet quota |
| The unsolved core | cross-resource exchange-rate derivation — OurGrid names it, nobody closes it (see `open-problems.md` §1) |
| "Comparative advantage" | the analyst's framing; OurGrid's papers say "profitability of exchange" and never use the term |

## Contents

Each file is independent and can be skimmed standalone; each ends with `## Sources`.

**Per-system files**
- [**ourgrid.md**](ourgrid.md) — **priority-1.** Network of Favors + multi-service extension. Token-free, decentralized, local per-peer favor balances over heterogeneous services. The closest published realization; names the exchange-rate open problem.
- [**gnunet.md**](gnunet.md) — excess-based economic model + GAP trust accounting. Private non-negative per-peer trust, the excess rule, Sybil bound d ≤ c + ε, delegation-with-margin transitivity. The provider-cost half.
- [**samsara.md**](samsara.md) — fairness by forced symmetric storage claims. The replacement-cost precedent, and the anti-comparative-advantage degenerate case.
- [**credit-and-scrip.md**](credit-and-scrip.md) — the global-scrip cautionary foils (Karma, Maze, Dandelion, MojoNation) + single-resource fairness points (FairTorrent, Tahoe-LAFS).

**Synthesis**
- [**open-problems.md**](open-problems.md) — what none of these solve: exchange-rate derivation, verifying self-measured cost, Sybil-safe transitive credit, the firm-vs-market granularity boundary, asymmetric demand.
- [**lessons.md**](lessons.md) — **the consult-this-when-designing file.** Validates / Avoid / Borrow, framed for the leading model as consumer. Captures lessons, not decisions.
- [**glossary.md**](glossary.md) — NoF, ExtNoF, excess rule, replacement cost, scrip, favor balance, shadow price, wash-trading, etc.

## Canonical reading order

1. [`ourgrid.md`](ourgrid.md) — the closest realization; orient on what "token-free heterogeneous reciprocity" looks like when deployed.
2. [`gnunet.md`](gnunet.md) — the excess rule and the Sybil bound; the provider-cost half.
3. [`samsara.md`](samsara.md) — replacement-cost crediting and the symmetric floor.
4. [`credit-and-scrip.md`](credit-and-scrip.md) — why the global-token alternative fails.
5. [`open-problems.md`](open-problems.md) → [`lessons.md`](lessons.md) — the gaps, then the action-oriented synthesis.

## Glossary stub

Full definitions in [`glossary.md`](glossary.md). The load-bearing few:

- **NoF / Network of Favors** — OurGrid's token-free local-favor-balance reciprocation algorithm.
- **Excess rule** — GNUnet: serve free when idle, charge trust only under load.
- **Replacement cost** — the cost to reproduce received work yourself; the basis for crediting it (Samsara's claim is the literal instance).
- **Scrip** — a system-internal currency token (Karma's *karma*, MojoNation's *Mojo*); global fungible scrip is the anti-pattern.
- **Favor balance** — OurGrid's bilateral per-peer record of favors given vs received.
- **Shadow price** — a peer's subjective marginal value for a resource given its own scarcity.

## How to use this prior-art doc

Designing or specifying Myrhiza's participation/valuation economy? Start with the reading order above, then [`lessons.md`](lessons.md) for the Validates / Avoid / Borrow synthesis, dropping into per-system files for primary-source depth when a specific mechanism becomes a candidate. The consumer this folder was built for is the brainstorm report — cross-link back to [`reports/2026-05-29-reciprocity-economy-brainstorm/`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) for the model, its locked decisions, and its open forks.

**Framing disclosure.** This corpus is written from **Myrhiza's no-token, per-peer-reciprocity stance** — it is *not* a neutral catalog of P2P economics. Specifically: the leading `value_P = resource_vector · shadow_prices_P` model is treated as the *target*, and each system is read through "does this realize, validate, or warn against a piece of that model?" That biases the reading. It **foregrounds** token-free local reciprocity (OurGrid, GNUnet, Samsara) and **backgrounds or files-as-foils** every market/currency approach (Karma, MojoNation, Dandelion) and the entire computational-market paradigm (priced as a *separate* candidate folder, `market-based-control`, not here). "Comparative advantage" is the analyst's lens, not the papers' term. A reader asking the prior question — *"should Myrhiza have a reciprocity economy at all, or use share-based fairness / a market / nothing?"* — should weigh the corpus accordingly: it is a learn-the-token-free-precedents-for-a-chosen-direction artifact, and the direction is the brainstorm's, not a settled Myrhiza decision.

## Cross-links

- [`reports/2026-05-29-reciprocity-economy-brainstorm/README.md`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) — the consumer; the model, locked decisions, open forks, and "model challenges."
- [`prior-art/sybil-resistance/`](../sybil-resistance/) — the orthogonal Sybil/free-riding-defense survey (`taxonomy.md` §1 places this folder's systems in Category 1; `lessons.md` for the reciprocity-beats-reputation and social-graph-admission framing; `bittorrent.md` and `ipfs-bitswap.md` for the deployed single-resource reciprocity references).
- [`specs/2026-05-09-myrhiza-master-design/maintenance.md`](../../specs/2026-05-09-myrhiza-master-design/maintenance.md) §12 — the `myrhiza-participation-*` framework this would feed.
- [`specs/2026-05-09-myrhiza-master-design/determinism.md`](../../specs/2026-05-09-myrhiza-master-design/determinism.md) — why the ledger is non-authoritative / off the determinism path.
- Sibling candidate folders flagged but not built here: `market-based-control` (the computational-market paradigm `value_P` descends from) and `resource-pricing-theory` (Kelly NUM / DRF / the soundness critique).

## Sources

Primary sources live in each per-system file's `## Sources`. The anchor citations:

- [Andrade / Brasileiro / Cirne / Mowbray, "Discouraging Free Riding in a Peer-to-Peer CPU-Sharing Grid," HPDC 2004](https://hpdc.sci.utah.edu/2004/papers/32.pdf)
- [Mowbray / Brasileiro / Andrade / Santana / Cirne, "A Reciprocation-Based Economy for Multiple Services in Peer-to-Peer Grids," IEEE P2P 2006](https://ieeexplore.ieee.org/document/1698610/)
- [Grothoff, "An Excess-Based Economic Model for Resource Allocation in Peer-to-Peer Networks," Wirtschaftsinformatik 2003](https://grothoff.org/christian/ebe.pdf)
- [Cox & Noble, "Samsara: Honor Among Thieves in Peer-to-Peer Storage," SOSP 2003](https://www.cs.rochester.edu/meetings/sosp2003/papers/p135-cox.pdf)
- [Vishnumurthy / Chandrakumar / Sirer, "KARMA," P2PEcon 2003](https://www.cs.cornell.edu/people/egs/papers/karma.pdf)
- [Sirivianos / Park / Yang / Jarecki, "Dandelion," USENIX ATC 2007](https://www.usenix.org/legacy/event/usenix07/tech/full_papers/sirivianos/sirivianos.pdf)
