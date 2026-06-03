**Date:** 2026-05-29
**Status:** active
**Subject:** Glossary of token-free reciprocity / resource-economics terms used across this folder.

# Glossary

Terms as used in this folder. Where a term is the *analyst's* framing rather than a surveyed system's own vocabulary, that is flagged — academic-attribution hygiene.

**Bank-set.** In Karma, a quorum of nodes that collectively tracks one peer's global balance and certifies transfers atomically, tolerating a fraction of malicious members. The price of making a *global transferable* currency safe; the per-peer model needs none. See [`credit-and-scrip.md`](credit-and-scrip.md).

**Claim (storage claim).** In Samsara, an **incompressible placeholder** of equivalent size that a peer must hold in return for storing data on another. Makes an asymmetric "store my data" request into a symmetric storage contract. The replacement cost, denominated in the resource itself. See [`samsara.md`](samsara.md).

**Comparative advantage.** Ricardian trade-theory term for the gains two parties get by each specializing in what is *relatively* cheap for them and trading. **Analyst's framing, not the surveyed papers' term** — OurGrid frames the same effect as "profitability of exchange" and never uses "comparative advantage." Used here only as a lens. See [`ourgrid.md`](ourgrid.md).

**Damage bound (d ≤ c + ε).** GNUnet's result that the harm an attacker A can do is bounded by A's own capacity `c` plus the network's excess bandwidth `ε`; since ε is by definition harmless, effective damage is bounded by the capacity A genuinely contributes. See [`gnunet.md`](gnunet.md).

**Deficit counter.** FairTorrent's per-peer counter = bytes uploaded − bytes downloaded; a peer always serves the peer it owes most. Single-resource instance of the net-imbalance standing curve. See [`credit-and-scrip.md`](credit-and-scrip.md).

**Excess rule.** GNUnet's pricing rule: serve requests *for free when idle* (spare capacity), and *charge trust only under load*, dropping the lowest-effective-priority requests first. Dissolves the newcomer-bootstrap problem. See [`gnunet.md`](gnunet.md).

**ExtNoF (Extended Network of Favors).** Descriptive name for OurGrid's multi-service NoF — the extension of favor accounting from CPU-only to multiple heterogeneous services (CPU, data transfer, storage). The underlying paper is titled "A Reciprocation-Based Economy for Multiple Services in P2P Grids"; "ExtNoF" is community shorthand and may not appear verbatim in that paper (flagged). See [`ourgrid.md`](ourgrid.md).

**Favor balance.** In OurGrid's NoF, the per-peer local record of (value of favors received from a peer, value of favors given to it), from which a non-negative local reputation is computed. Purely bilateral; nothing global. See [`ourgrid.md`](ourgrid.md).

**Fungible.** Property of a currency where one unit is interchangeable with any other and worth the same to everyone. Global scrip is fungible (and so wash-tradeable); per-peer balances are *non-fungible* (B's standing with A means nothing to C), which is the structural collusion defense. See [`credit-and-scrip.md`](credit-and-scrip.md).

**NoF (Network of Favors).** OurGrid's token-free, fully decentralized reciprocation algorithm: each peer keeps local favor balances and prioritizes serving the peers it owes most. The closest published realization of the leading Myrhiza model. See [`ourgrid.md`](ourgrid.md).

**Optimistic unchoke.** BitTorrent's bolted-on slot that periodically serves a random peer with no history, giving newcomers a way in. GNUnet's excess rule achieves the same effect as a *consequence of pricing* rather than a special case. See [`prior-art/sybil-resistance/bittorrent.md`](../sybil-resistance/bittorrent.md), [`gnunet.md`](gnunet.md).

**Replacement cost.** The cost to a peer of reproducing a piece of work itself — the basis on which the leading model proposes to credit *received* work (never the provider's self-report). Samsara is the literal storage instance: the claim you hold *is* your replacement cost. See [`samsara.md`](samsara.md), [`gnunet.md`](gnunet.md).

**Resource recipe / resource vector.** A module-declared objective description of what an action consumes (CPU-ms, byte-hours, bandwidth), the same everywhere it runs. The objective half of `value_P(action) = resource_vector · shadow_prices_P`. Largely greenfield in Myrhiza's substrate (per the brainstorm). See [`reports/2026-05-29-reciprocity-economy-brainstorm/README.md`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md).

**Scrip.** A non-government, system-internal currency token (Karma's *karma*, MojoNation's *Mojo*, Dandelion's credit). Global scrip is the cautionary anti-pattern this folder catalogs. See [`credit-and-scrip.md`](credit-and-scrip.md).

**Shadow price.** The marginal value (LP-dual) a peer assigns to a unit of a resource given its own scarcity — fast CPU → compute cheap, scarce disk → storage expensive. The subjective half of the leading model; *per peer*, never reconciled globally (so a *locally-valid Lagrangian cost*, not a market-clearing price). Deeper formal treatment is in the `resource-pricing-theory` candidate folder, not here. See [`reports/2026-05-29-reciprocity-economy-brainstorm/README.md`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md).

**Standing / local reputation.** A peer's per-counterparty assessment of how much it owes / is owed, driving serving priority. OurGrid's `r_A(B)`, GNUnet's per-neighbor trust, the brainstorm's smooth standing curve. Always *local and subjective*. See [`ourgrid.md`](ourgrid.md), [`gnunet.md`](gnunet.md).

**Trust (GNUnet sense).** GNUnet's private, per-peer, non-negative scalar currency — earned by serving, spent by requesting, capped on use at `min(requested priority, held trust)`. Not gossiped, not global. See [`gnunet.md`](gnunet.md).

**Wash-trading.** Colluding identities exchanging useless work in a loop to mint balance/scrip, then spending it on real resources elsewhere. The attack global fungible scrip invites (measured in Maze) and per-peer non-fungible balances escape. See [`credit-and-scrip.md`](credit-and-scrip.md).

**Whitewashing.** Abandoning a low-standing identity and starting fresh with a new one. Cheap when identities are free; defeated by non-negative balances (a fresh zero-balance identity is no better off than an honest newcomer) plus social-graph admission. See [`ourgrid.md`](ourgrid.md), [`gnunet.md`](gnunet.md), [`prior-art/sybil-resistance/open-problems.md`](../sybil-resistance/open-problems.md).

## Sources

Term definitions are drawn from the per-system files in this folder; see each cross-referenced file for primary sources.
