**Date:** 2026-05-29
**Status:** active
**Subject:** The decision file — what the market-based-control paradigm validates, what to avoid, what to borrow, framed for Myrhiza's reciprocity model. Captures lessons; does not assert Myrhiza decisions.

# Lessons for Myrhiza — market-based control

Synthesis across [`history.md`](history.md), [`spawn-tycoon.md`](spawn-tycoon.md),
[`mirage-bellagio.md`](mirage-bellagio.md), [`markets-overkill.md`](markets-overkill.md),
[`open-problems.md`](open-problems.md). Format: validates / avoid / borrow. These are *lessons
captured from prior art*, not settled Myrhiza decisions — the reciprocity model is still a
brainstorm ([report](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md)), and the
locked/open status of each idea lives there, not here.

## The single most important lesson — the local-crediting escape

The cluster of failures that killed every computational market — **price volatility, price-
discovery latency, bid-authoring UX** ([`open-problems.md`](open-problems.md) #1–#3) — are all
properties of a **live market**: agents bidding, prices clearing, valuations being authored and
discovered in real time. Myrhiza's reciprocity model is **not a live market.** It is a **per-peer
local crediting ledger, off the determinism path** — each peer privately records work done for it
and credits it by *its own* measured/replacement cost, with no auction, no shared price, no bid,
no clearing. That single architectural choice **sidesteps the entire volatility/discovery/bid-UX
cluster** that the paradigm could never solve. Name this explicitly in any spec: *Myrhiza borrows
the* subjective-marginal-value *insight from market-based control while rejecting the* market
*itself.* This is the model's escape, and the failure record in this folder is its strongest
justification.

## Validates

1. **Subjective per-peer pricing is sound — *as a cost metric, locally*.** The paradigm's core
   insight (Sutherland → Miller-Drexler → Spawn) is that resources should be valued at their
   *marginal cost to the holder*, which differs per agent — exactly `value_P = resource_vector ·
   shadow_prices_P`. Using it as a *local crediting metric* (not a global allocation rule) is the
   defensible slice. *Source: [`history.md`](history.md); the optimality limits are in
   [`prior-art/resource-pricing-theory/`](../resource-pricing-theory/).*
2. **Enforcement must be the binding, sole path.** Mirage survived only because it was the *sole*
   path to a scarce binding resource; Bellagio failed competing with a free best-effort default.
   This validates making low standing *bind* at the capability boundary with no free side door —
   which is what Myrhiza's refusal-as-first-class-primitive enables. *Source:
   [`mirage-bellagio.md`](mirage-bellagio.md).*
3. **A decay/"use it or lose it" knob is mandatory, not optional.** Both deployed markets needed a
   savings tax to stop idle scrip hoarding. This validates the report's consumption-relative decay
   as a *required* element, and confirms it is better-targeted than a flat wall-clock tax. *Source:
   [`mirage-bellagio.md`](mirage-bellagio.md).*
4. **Mechanism/policy separation is the right structure.** SHARP built policy-agnostic plumbing
   (tickets/leases, delegation) and let the market be one policy above it. This validates the
   locked decision that reciprocity logic is a *module*, with the kernel providing only the
   capability-gated enforcement mechanism. *Source: [`markets-overkill.md`](markets-overkill.md).*
5. **The Coasean boundary is real and the founders said so.** Miller & Drexler conceded that fine-
   grained pricing has overhead and that "islands of central direction" belong inside a "sea of
   trade." This validates the report's "flat fair-share within a trust domain; price only across
   boundaries" mitigation. *Source: [`open-problems.md`](open-problems.md) #4.*

## Avoid

| Pitfall | Source | Why it killed prior systems |
|---|---|---|
| **Running a live auction / market in the runtime.** | [`spawn-tycoon.md`](spawn-tycoon.md), [`markets-overkill.md`](markets-overkill.md) | Volatility, discovery latency, bid UX. Spawn's own author abandoned it; the price-free design shipped instead. |
| **A shared, circulating virtual currency.** | [`mirage-bellagio.md`](mirage-bellagio.md), [`open-problems.md`](open-problems.md) #5 | Starvation, depletion, hoarding, inflation — needs perpetual monetary policy. A *shared* token is the thing the no-token decision rejects; per-peer private bookkeeping has none of these. |
| **Pricing a non-binding resource.** | [`mirage-bellagio.md`](mirage-bellagio.md) | Bellagio priced PlanetLab CPU that wasn't the scarce constraint; users routed around it. Standing must bite where the resource is actually scarce *to the serving peer*. |
| **Optional/opt-in enforcement.** | [`mirage-bellagio.md`](mirage-bellagio.md) | A market with a free side door is ignored. If a freeloader can still get served by a default path, standing is advisory and useless. |
| **Global market-clearing (WALRAS-style).** | [`markets-overkill.md`](markets-overkill.md) | Requires cross-peer price reconciliation Myrhiza structurally cannot have (determinism boundary). Claiming NUM/Walrasian optimality without its preconditions is the report's soundness-honesty trap (#5). |
| **Pricing at too-fine a grain.** | [`open-problems.md`](open-problems.md) #4 | Below the Coasean threshold, accounting overhead exceeds the benefit. Bitswap *stripped* its byte ledger for this reason. |
| **Requiring users to author bids/valuations.** | [`open-problems.md`](open-problems.md) #3 | The adoption-killing UX. Value must be computed (module recipe × peer prices), never hand-bid. |

## Borrow

1. **Subjective marginal cost as the credit unit.** Take the per-agent valuation insight, drop the
   market that delivers it. Credit work by the recipient's *own* replacement cost (what it would
   cost *me* to reproduce), never the counterparty's self-report — the directional rule. *See
   [`history.md`](history.md), reciprocity report layer 4.*
2. **Sutherland's non-consumable-token *contrast*.** Sutherland's yen *reverted* to the bidder
   (priority token, not spent scrip). Myrhiza goes further — no token at all — but the contrast is
   instructive: even the 1968 root avoided a depletable currency. *See [`history.md`](history.md).*
3. **Mirage's two-part anti-hoarding policy, retargeted.** "Profit-sharing for idle users" +
   "savings tax / use it or lose it." Myrhiza's consumption-relative decay is the better-aimed
   version (it taxes taking-without-giving, the exact target). *See [`mirage-bellagio.md`](mirage-bellagio.md).*
4. **SHARP's tickets/leases + delegation as the enforcement-mechanism shape.** Policy-agnostic,
   cryptographically protected claims with delegation — the right shape for capability-gated
   serving under the reciprocity policy. *See [`markets-overkill.md`](markets-overkill.md).*
5. **The Coasean coastline as an explicit spec parameter.** Borrow the *concept* (price across
   trust boundaries, share within them) and make where the line sits a documented design decision,
   not an accident. *See [`open-problems.md`](open-problems.md) #4.*

## Tension with `agoric-endo/`

[`agoric-endo/governance.md` §"Implications for Myrhiza"](../agoric-endo/governance.md) states
flatly that "tokenomics are not a runtime concern" and treats Agoric's BLD/IST half-collapse as a
cautionary tale against baking economic primitives into the runtime. This folder *sharpens* that
into a productive tension: the reciprocity model **does** put an economic primitive (standing /
value) in the runtime layer — but a **token-free, per-peer, non-authoritative** one. The
agoric-endo lesson is "no token / no chain governance debt"; this folder's lesson is "no *live
market* either, but a local-crediting metric is admissible *because* it sheds exactly the
properties — shared currency, global clearing, live pricing — that make economics a runtime
liability." The two are compatible: both reject the token; this folder adds *why a local metric is
the safe residue* and where its boundary sits.

## Cross-references

- [`README.md`](README.md), [`history.md`](history.md), [`open-problems.md`](open-problems.md)
- [`spawn-tycoon.md`](spawn-tycoon.md), [`mirage-bellagio.md`](mirage-bellagio.md), [`markets-overkill.md`](markets-overkill.md)
- [`reports/2026-05-29-reciprocity-economy-brainstorm/`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) — the consumer; locked/open status of every idea lives there.
- [`prior-art/p2p-resource-economics/`](../p2p-resource-economics/) — where the local-crediting shape is *realized* (OurGrid, GNUnet, Samsara).
- [`prior-art/resource-pricing-theory/`](../resource-pricing-theory/) — the formal fairness side (DRF, Kelly NUM).
- [`prior-art/agoric-endo/governance.md`](../agoric-endo/governance.md) — the tokenomics tension above.

## Sources

All sources in the per-file `## Sources` sections of this folder.
