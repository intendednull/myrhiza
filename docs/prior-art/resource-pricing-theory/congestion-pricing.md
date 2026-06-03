**Date:** 2026-05-29
**Status:** active
**Subject:** Briscoe's ConEx / re-ECN / re-feedback — congestion (shadow) pricing as real-world accountability. The closest thing to deployed shadow-pricing-as-accountability — but IETF Experimental, trialed, NOT production.

# Congestion pricing — shadow prices as accountability, and its honest status

## What it is

Bob Briscoe's line of work — **re-feedback → re-ECN → ConEx (Congestion Exposure)** — is the most serious attempt to turn Kelly-style congestion shadow prices into a *real-world accountability mechanism*. The premise inherits directly from NUM ([`network-utility-maximization.md`](network-utility-maximization.md)): congestion at a link *is* its shadow price — the marginal cost the link imposes on everyone when it is the bottleneck. If that price could be made **visible to every node on the path and attributed to the party causing it**, the network could hold the *upstream sender* accountable for the congestion it inflicts, rather than relying on voluntary TCP back-off.

This is conceptually the same shape Myrhiza's reciprocity model reaches for: make a resource's scarcity-cost *legible* and *attributable*, so the party consuming the scarce resource can be charged/deprioritized. ConEx is the closest the networking world has come to deploying that idea.

## How it works (briefly)

- **ECN** (Explicit Congestion Notification) already lets a congested router *mark* packets instead of dropping them; the receiver echoes the marks back to the sender. So congestion is measured downstream, but only the two endpoints learn the total.
- **re-ECN / re-feedback** has the *sender* re-insert that congestion information back into the forward path, in-band, so that **every node along the path** — not just the endpoints — can see the total downstream congestion a flow is about to cause. This re-inserted signal is the accountability hook: a network operator at any trust boundary can meter "congestion-volume" (bytes dropped or ECN-marked over time) and hold the upstream party responsible.
- **ConEx** is the IETF productization of this: an abstract mechanism (RFC 7713) plus a concrete IPv6 encoding (RFC 7837) for carrying the re-inserted congestion marking.

The accountability metric — *congestion-volume* — is a direct, attributable shadow-price signal. It is the operational answer to "who should pay for scarcity," computed from real congestion feedback (i.e. it *has* the reconciliation NUM requires; it is not a purely-local price).

### The trust-boundary mechanic

The reason re-feedback matters is *where* it makes the price visible. At any **trust boundary** in the internetwork (e.g. between an access ISP and a transit provider), the downstream party can meter the re-inserted congestion signal and bill or police the *upstream* party for the congestion it is about to cause. The accountability is thus pushed to the boundary between two distrusting parties — exactly the place Myrhiza's reciprocity model also operates (a peer crediting work *across* a trust boundary to another peer). The structural parallel is tight: both want a scarcity price that survives a trust boundary and cannot be unilaterally understated by the party being charged. The difference is that ConEx achieves cross-boundary *agreement* via path-wide feedback (the reconciliation), whereas Myrhiza deliberately keeps the price one-sided and local (the recipient's own replacement cost), trading global agreement for trust-minimality.

## Status — be precise

A prior research pass over-claimed that ConEx was "deployed." **It was not, and is not, in production.** The accurate status:

- **RFC 7713** — "Congestion Exposure (ConEx) Concepts, Abstract Mechanism, and Requirements" (Mathis & Briscoe, 2015). *Informational.* The conceptual/architecture document.
- **RFC 7837** — "IPv6 Destination Option for Congestion Exposure (ConEx)" (Krishnan, Kuehlewind, Briscoe & Ralli Ucendo, May 2016). **Experimental Protocol.** Its own text: "published for examination, experimental implementation, and evaluation." (Earlier concepts/use-cases doc: RFC 6789, *Informational*, 2012.)

So the honest one-liner: **ConEx is IETF *Experimental* — standardized as an experiment and trialed, but never production-deployed at internet scale.** The IETF ConEx working group concluded without production adoption. Calling it "deployed" is wrong; calling it "the closest thing to deployed shadow-pricing-as-accountability that reached a standards body" is right.

## Why it matters for Myrhiza

- **Validation by analogy.** ConEx confirms the *idea* is taken seriously by serious engineers: making a scarcity (shadow) price *legible and attributable* is a coherent accountability primitive, and a standards body specified it. Myrhiza's "credit/debit by resource cost" is in the same family.
- **A cautionary tale on deployment.** Even with IETF backing, a multi-decade research pedigree (Kelly's NUM → Briscoe's re-feedback → ConEx), and a clean theory, **congestion-pricing-as-accountability did not cross into production.** The barriers were incremental-deployment and incentive-alignment across trust boundaries — every router on the path has to honor the signal, and no single operator benefits from being first. This is the *exact* class of barrier the brainstorm's "markets are overkill" critique names (see [`prior-art/market-based-control/`](../market-based-control/) for the computational-economy version). Myrhiza sidesteps it by keeping pricing **local and per-peer, off the determinism path** — no cross-operator coordination required — but that is the very choice that forfeits NUM's global optimality (see [`network-utility-maximization.md`](network-utility-maximization.md) §"the critical caveat").
- **The metric design lesson.** ConEx's *congestion-volume* metric is attributable because it is measured by the network and re-inserted by the sender in a verifiable way. Myrhiza's analogue — crediting *received* work by the recipient's own measured replacement cost, never the sender's self-report — is the same instinct: put the metric where it cannot be unilaterally inflated. (See [`prior-art/sybil-resistance/`](../sybil-resistance/) for self-reported-cost verification; the directional crediting rule in the brainstorm.)

## Implications for Myrhiza

- Cite ConEx as **prior art that the scarcity-price-as-accountability idea is sound and standards-worthy** — but always with the *Experimental, not production* qualifier.
- Treat ConEx's non-deployment as **evidence for keeping Myrhiza's pricing local**: the thing that blocked ConEx (cross-trust-boundary coordination) is the thing Myrhiza's per-peer, non-authoritative ledger avoids by design.
- Borrow the *metric-placement* discipline: an accountability price must be measured where the accountable party cannot forge it.

## Sources

- [RFC 7713 — "Congestion Exposure (ConEx) Concepts, Abstract Mechanism, and Requirements," Mathis & Briscoe, 2015](https://www.rfc-editor.org/rfc/rfc7713.html) (Informational).
- [RFC 7837 — "IPv6 Destination Option for Congestion Exposure (ConEx)," 2016](https://www.rfc-editor.org/rfc/rfc7837.html) (**Experimental**).
- [RFC 6789 — "Congestion Exposure (ConEx) Concepts and Use Cases," 2012](https://www.rfc-editor.org/rfc/rfc6789.html) (Informational).
- Briscoe, "Re-feedback: Freedom with Accountability for Causing Congestion in a Connectionless Internetwork," PhD thesis, UCL, 2009 (re-ECN / re-feedback origin; citation by title — not re-verified off PDF, confidence medium).
- Cross-refs: [`network-utility-maximization.md`](network-utility-maximization.md), [`prior-art/market-based-control/`](../market-based-control/), [`prior-art/sybil-resistance/`](../sybil-resistance/).
