**Date:** 2026-05-29
**Status:** active
**Subject:** What none of the token-free reciprocity systems solve — the gaps the leading Myrhiza valuation model inherits.

# Open problems — token-free heterogeneous reciprocity

What no system in this folder cleanly solves for a per-peer, heterogeneous-resource, no-token economy. Each entry: short problem statement, why it matters for Myrhiza's `value_P(action) = resource_vector · shadow_prices_P` model, and which surveyed systems touch it. These are the gaps a `myrhiza-participation-*` spec must accept or close — not decisions Myrhiza has made.

## 1. Exchange-rate derivation across heterogeneous resources

This is **the** open problem, and the reason the folder exists. OurGrid's multi-service work proves heterogeneous reciprocity *marginalizes free riders* but provides **no principled rule for the exchange rate between services** — how many CPU-ms is a byte-hour worth? OurGrid uses local profitability heuristics; Samsara dodges it by forcing same-resource symmetry (so the rate is always 1:1, and specialization dies); GNUnet collapses everything to one scalar priority. None derive a cross-resource rate from physical reality.

**Why it matters:** Myrhiza's proposed answer — each peer's subjective shadow-prices over a module-declared resource vector — is exactly a candidate *derivation* of the rate OurGrid left open. But "each peer picks its own prices" is locally valid only; it is **not** a market-clearing rate, and prices never reconcile across peers (the directional rule deliberately forgoes reconciliation). So the rate is well-defined *per peer* and undefined *globally*. Whether per-peer rates are enough for stable reciprocity is unproven.

**Touched by:** [`ourgrid.md`](ourgrid.md) (names the problem), [`samsara.md`](samsara.md) (the 1:1 degenerate case), [`gnunet.md`](gnunet.md) (scalar collapse).

## 2. Verifying self-measured provider cost

The directional rule values work I *do* by my own *measured* cost. But a self-measured resource vector is a credit-stuffing surface: a peer can over-report what an action cost it. GNUnet sidesteps this by crediting the *requester's* declared priority (capped by held trust), not the provider's cost — i.e. it never trusts a self-measured cost at all. Samsara sidesteps it by making the unit physical and symmetric (a held byte is self-evidently held, and is challenge-verifiable). Neither *verifies a self-reported cost*; they avoid needing one.

**Why it matters:** Myrhiza wants to credit *received* work by the recipient's own replacement cost (so the provider's self-report never enters the credit), which is the GNUnet-shaped escape. But the provider's self-measured cost still drives *its own* serving priorities, and kernel-side metering of *actual* consumption is the only real check. Myrhiza's substrate does not yet meter the needed quantities (no byte counters, fuel consumed-but-not-read — per the brainstorm's verified substrate note). So verification is greenfield instrumentation, not adoption.

**Touched by:** [`gnunet.md`](gnunet.md), [`samsara.md`](samsara.md), and the BOINC/Gridcoin evidence flagged for [`prior-art/sybil-resistance/`](../sybil-resistance/) (self-reported-credit arms race).

## 3. Transitive / multi-hop credit without EigenTrust-style Sybil fragility

A purely bilateral ledger has the "everywhere gap": B's good behavior toward A means nothing to C, so a peer must rebuild standing with every new counterparty. Going transitive lets standing travel — but global transitive trust (EigenTrust) is Sybil-fragile (a Sybil clique vouches for itself). GNUnet's delegation-with-margin is the **only** surveyed mechanism that achieves transitivity *without* global aggregation, and it does so by reducing priority on each forward hop and charging the forwarder. It is promising but unproven at scale, and the margin must be tuned to kill credit-loop minting.

**Why it matters:** Myrhiza parked transitivity as an open fork. GNUnet shows there is a Sybil-safe shape; whether it composes with a *resource vector* (rather than a scalar priority) is unexplored.

**Touched by:** [`gnunet.md`](gnunet.md) (delegation-with-margin), contrast [`prior-art/sybil-resistance/eigentrust.md`](../sybil-resistance/eigentrust.md).

## 4. The firm-vs-market granularity boundary (Coase)

Pricing every inter-component action with a dot-product has real overhead. Below some interaction size, flat fair-share is cheaper and good enough — Samsara's same-unit 1:1 contracts and FairTorrent's plain deficit counter are the cheap regime; full subjective pricing is the expensive regime. Where is the boundary? Coase's firm-vs-market framing says: inside a trust domain, use flat share (the "firm"); across trust boundaries, use prices (the "market"). None of the surveyed systems draw this line explicitly — they each sit entirely on one side.

**Why it matters:** Myrhiza will pay pricing overhead it doesn't need if it prices *within* a trusted module/app the same as *across* untrusted peers. The brainstorm's "flat fair-share within a trust domain, `value_P` only across boundaries" is the intended boundary; no surveyed system validates where to put it.

**Touched by:** [`samsara.md`](samsara.md) §"same-unit crediting needs no valuation function", [`credit-and-scrip.md`](credit-and-scrip.md) (FairTorrent).

## 5. Asymmetric demand — peers that structurally cannot reciprocate in kind

A snapshot/sync provider or a relay-bound mobile peer may consume one resource heavily and produce another, or produce nothing the counterparty wants. Strict same-resource reciprocity (Samsara) excludes them outright; heterogeneous reciprocity (OurGrid) helps only if the resources they *can* offer are wanted by someone. None of the surveyed systems fully solve "a peer with nothing the network currently wants."

**Why it matters:** Myrhiza explicitly has asymmetric roles (the four/five component profiles). The heterogeneous model is *necessary* for them to participate, but not obviously *sufficient*.

**Touched by:** [`ourgrid.md`](ourgrid.md) (multi-service is the partial answer), [`samsara.md`](samsara.md) (the exclusion case).

## 6. Global supply management is unsolved *and unwanted*

Every global-scrip system (Karma, Maze, MojoNation) had to manage a money supply — and every one either reintroduced an authority (Karma's bank-set, Dandelion's server) or failed (MojoNation's inflation). The per-peer model avoids this *by construction*, but the price is that there is no global notion of standing at all — only the per-(peer, module/app) ledgers, plus a hard cross-app `convert()` residue. So "global supply management" is solved by *not having a global supply*, at the cost of leaving cross-app aggregation open.

**Why it matters:** confirms the no-token lock is the right trade, and names what it costs (the cross-app gap), so a spec author does not later reach for a global unit to close that gap and walk back into the Karma/MojoNation failure modes.

**Touched by:** [`credit-and-scrip.md`](credit-and-scrip.md) (all four foils).

## Cross-references

- [`README.md`](README.md), [`lessons.md`](lessons.md), per-system files.
- [`reports/2026-05-29-reciprocity-economy-brainstorm/README.md`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) — "Open forks" and "model challenges" (DRF, Leontief, markets-are-overkill) are the deeper formal versions of #1, #2, and #4 here.
- [`prior-art/sybil-resistance/open-problems.md`](../sybil-resistance/open-problems.md) — the Sybil/whitewashing/collusion side of the same problem.

## Sources

All sources in the per-system files in this folder.
