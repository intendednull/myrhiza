**Date:** 2026-05-29
**Status:** active
**Subject:** OurGrid's Network of Favors + the multi-service extension — the closest published realization of token-free, per-peer, heterogeneous-resource reciprocity.

# OurGrid — the Network of Favors and its multi-service extension

This is the **priority-1 file** in the folder. Of every system surveyed here, OurGrid's Network of Favors (NoF) is the *closest published realization* of the reciprocity model the brainstorm report ([`reports/2026-05-29-reciprocity-economy-brainstorm/`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md)) calls leading: token-free, fully decentralized, with **local per-peer favor balances** and no global currency. The multi-service extension goes one step further and is the part that matters most — peers swap services they value differently, which is `value_P(action) = resource_vector · shadow_prices_P` in all but name.

## What OurGrid is

OurGrid is a peer-to-peer desktop grid for **Bag-of-Tasks** applications (independent parallel tasks), developed mainly at the Universidade Federal de Campina Grande (UFCG), Brazil. It ran a production community grid from December 2004 onward — this matters, because the NoF results below are partly empirical, not purely simulated. The project's site still advertises releases (4.4.0), though the bulk of the research output and live deployment dates to the mid-to-late 2000s; treat "still maintained" as plausible-but-stale (unverified — site claim only).

## The Network of Favors (NoF) — single service

From Andrade / Brasileiro / Cirne / Mowbray, "Discouraging Free Riding in a Peer-to-Peer CPU-Sharing Grid" (HPDC 2004):

- Donating a resource (a CPU slot for a task) is a **favor**. Each peer keeps a **local record** of the total value of favors it has given to, and received from, each other peer. Nothing is global; nothing is a token.
- Peer A computes a **local reputation** `r_A(B)` from just two numbers: the value of favors A received from B and the value of favors B received from A. When A must choose whom to serve, it prioritizes the peers it owes the most.
- The reputation function is **non-negative** — a peer that received more than it gave does not go negative and so cannot be out-prioritized by a fresh whitewashed identity that simply has a zero balance. This is the paper's explicit defense against "malicious ID-changing."
- State cost is tiny: a peer only tracks counterparties with nonzero local reputation.

The paper's analytic result: under a model where peers switch strategy toward whatever is in their interest, a non-negative local-reputation scheme drives the community **toward a state with no free riders** — free riding stops paying once collaborators prioritize each other. The conclusion notes that adding a **sublinear history term** further improves the ability to marginalize free riders. This is the empirical/analytic "free-rider-marginalization" result the brainstorm leans on, and it holds *even when the donor's cost is close to the recipient's utility* (made explicit in the multi-service paper below).

## The multi-service extension — heterogeneous resources

From Mowbray / Brasileiro / Andrade / Santana / Cirne, "A Reciprocation-Based Economy for Multiple Services in Peer-to-Peer Grids" (IEEE P2P 2006, pp. 193–202):

This is the load-bearing paper for Myrhiza. OurGrid deployment experience showed users wanted incentives not just for CPU but for **data transfer and storage** too — multiple, non-interchangeable services. The extension:

- Keeps the local, per-peer, reliable-information-only favor accounting.
- Adds the question single-service NoF never faced: *should peer A even bother exchanging a given service with B?* A peer assesses whether a trade is **profitable** to it — i.e. whether the service it gives up is cheaper to it than the service it gets is valuable to it. Peers differ in what is cheap vs scarce for them, so trades that look uneven in raw units are still mutually beneficial.
- Headline result (simulation): the mechanism **marginalizes free riders even when the cost to a peer of donating a service is nearly as large as the utility it gains by receiving one** — the hardest regime for any reciprocity scheme.

This "peers value services differently, so they swap cheap-for-them for scarce-for-them" structure is exactly Myrhiza's subjective-shadow-prices idea. **The papers do not call this "comparative advantage"** — that is the *analyst's* framing (and the term appears zero times in the HPDC 2004 paper; the multi-service paper frames it as profitability of exchange, not Ricardian trade theory). Flagging this so a spec author doesn't attribute the economics vocabulary to OurGrid's authors.

The follow-on study, Coêlho / Maciel Jr. / Figueiredo / Maia / Brasileiro, "On the Impact of Choice in Multi-Service P2P Grids" (BDIM 2008, pp. 98–101), measures how a peer's *received* utility depends on **which** services it chooses to offer — i.e. given a fixed capacity budget, the choice of what to put on the market changes the return. That is OurGrid's own statement of its open problem (below).

## The open problem OurGrid names (and Myrhiza inherits)

The multi-service work surfaces the exact gap the leading Myrhiza model must close: with heterogeneous services there is no published rule for **how a peer derives the exchange rate between services**, nor for **which services it should offer given a limited budget**. OurGrid handles this with simple local profitability heuristics; it does not derive shadow prices from hardware reality. Myrhiza's "resource recipe × subjective shadow-prices, credited at the recipient's own replacement cost" is a *candidate answer* to OurGrid's open problem — which is why this folder exists. See [`open-problems.md`](open-problems.md) §1.

## Implications for Myrhiza (framing-disclosed — see [`README.md`](README.md))

1. **Existence proof.** Token-free, fully-decentralized, local-per-peer favor accounting *works* and was *deployed* — the leading model is not science fiction. Borrow the local-reputation-from-two-numbers shape directly.
2. **Non-negativity is the whitewashing defense.** NoF's non-negative balance is the same lever the brainstorm's "smooth standing curve, no negative credit minted for free" relies on. Validated prior art. See [`lessons.md`](lessons.md).
3. **Heterogeneity is the whole game, and it is unsolved at the exchange-rate level.** OurGrid proves heterogeneous reciprocity marginalizes free riders; it does *not* prove how to price across services. Myrhiza's contribution would be precisely that pricing rule — don't claim OurGrid already solved it.
4. **OurGrid never had a social graph or a capability kernel.** Its Sybil story is purely the non-negative balance; it has no admission layer. Myrhiza's invite graph (admission) + kernel refusal-to-serve (enforcement) are additions OurGrid lacked, not things it validates.

## Sources

- [Andrade / Brasileiro / Cirne / Mowbray, "Discouraging Free Riding in a Peer-to-Peer CPU-Sharing Grid," HPDC 2004, DOI 10.1109/HPDC.2004.9](https://hpdc.sci.utah.edu/2004/papers/32.pdf) — authors: Nazareno Andrade, Francisco Brasileiro, Walfredo Cirne (UFCG), Miranda Mowbray (HP Labs Bristol). Verified against the PDF: NoF mechanism, non-negative local reputation, free-rider marginalization, sublinear history term. "Comparative advantage" appears 0 times.
- [Mowbray / Brasileiro / Andrade / Santana / Cirne, "A Reciprocation-Based Economy for Multiple Services in Peer-to-Peer Grids," IEEE P2P 2006, pp. 193–202, DOI 10.1109/P2P.2006.3](https://ieeexplore.ieee.org/document/1698610/) — venue + author list verified via dblp. Body verified via abstract/secondary (IEEE PDF behind paywall; full-text claims marked from abstract).
- [Coêlho / Maciel Jr. / Figueiredo / Maia / Brasileiro, "On the Impact of Choice in Multi-Service P2P Grids," BDIM 2008, pp. 98–101](https://dblp.org/db/conf/bdim/bdim2008.html) — venue, authors, pages verified via dblp.
- [OurGrid project site](https://ourgrid.org/) and [OurGrid on Wikipedia](https://en.wikipedia.org/wiki/OurGrid) — deployment history and status (status claim unverified beyond site).
- Cross-references: [`README.md`](README.md), [`open-problems.md`](open-problems.md) §1, [`lessons.md`](lessons.md), [`reports/2026-05-29-reciprocity-economy-brainstorm/README.md`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md), [`prior-art/sybil-resistance/taxonomy.md`](../sybil-resistance/taxonomy.md) §1.
