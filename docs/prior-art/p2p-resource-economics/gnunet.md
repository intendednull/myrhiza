**Date:** 2026-05-29
**Status:** active
**Subject:** GNUnet's excess-based economic model + GAP trust accounting — the closest precedent for "value work I do by my own provider cost."

# GNUnet — the excess-based economic model

The gap analysis flagged GNUnet as the **single highest-value missing system** in the corpus, and it earns that. Grothoff's excess-based model is a near 1:1 match for the *provider half* of Myrhiza's directional crediting rule — "value the work I do by my own (provider) cost" — combined with a structural Sybil/whitewashing immunity that the brainstorm wants but had no published precedent for.

## Citation and status

- Christian Grothoff, "An Excess-Based Economic Model for Resource Allocation in Peer-to-Peer Networks," *Wirtschaftsinformatik* 45(3), June 2003, pp. 285–292 (DOI 10.1007/BF03254946). Written while Grothoff was at Purdue University; the model is the resource-allocation layer of GNUnet, the GNU anonymous P2P framework. Verified against the paper PDF at grothoff.org.
- The resource-accounting is realized in **GAP** (Bennett & Grothoff, "gap — practical anonymous networking," PET 2003) — the anonymity routing protocol whose request-priority field carries the trust accounting described below.
- **Maintenance status: actively maintained.** GNUnet 0.27.0 was released March 2026 (0.24.0 March 2025, 0.25.0 Sept 2025, 0.26.0 Nov 2025). It remains pre-1.0 with documented rough edges (the TRANSPORT subsystem has known major issues; gnunet-gtk was retired at 0.25.0). The *economic model* is a stable, long-standing part of the design. Verified via gnunet.org news.

## The model

GNUnet uses **trust**, not money, as its currency, and there are **no trusted entities** — every node is an equal peer. The accounting is:

- **Private and per-peer.** Each node maintains a trust balance *for each neighbor*, visible only to itself. "Nodes can only trust their own records." There is no global ledger, no gossiped reputation, no bank. This is the property that makes it converge-free and side-effect-free — exactly why Myrhiza can keep its ledger off the determinism path.
- **Non-negative.** Trust is bounded below at zero.
- **Earned by serving, spent by requesting.** A request carries a **priority** — the amount of trust the sender is willing to spend. If a node S sends a request to R with priority 10 and R answers, S increases its trust in R by 10. Symmetrically, R only honors priority up to the trust it already holds in S: the effective priority is `min(requested_priority, trust R has in S)`. Serving earns; requesting spends.

### The excess rule (the part Myrhiza wants)

The model is "excess-based" in a precise sense: **a node serves requests for free when it has spare capacity, and only charges trust when it is under load.**

- When R is idle, it answers and may charge little or nothing — capacity that would otherwise go unused is given away.
- When R is busy, it must drop some requests; it drops the ones with the **lowest effective priority first**. Trust only becomes scarce, and therefore only gets spent, under contention.

This dissolves the newcomer/bootstrap problem that plagues strict tit-for-tat: a fresh node with zero trust can still be served whenever the network is idle, and uses that free service to do work and earn trust. It is the same shape as BitTorrent's optimistic unchoke, but falls out of the pricing rule instead of being a bolted-on slot. The brainstorm's "standing is a smooth gradient, priority capped by earned trust" is GNUnet's rule restated.

### Structural Sybil / whitewashing immunity

Because trust is earned only by *actually serving* and is private per-peer:

- **A fresh identity gains nothing.** A new node starts at zero trust everywhere and is served only out of others' excess capacity — exactly what any newcomer gets. Minting N identities yields N zero-trust accounts, not N times the resources. Grothoff handles persistent identity-cyclers by giving new identities a **low exchange rate** for their currency, so they must issue many more requests to get the same service.
- **Bounded damage.** The paper bounds the damage `d` an attacker A can inflict: it is A's own capacity `c` plus the network's excess bandwidth `ε` — i.e. **d ≤ c + ε**. Since ε is by definition traffic that does not degrade performance, the *effective* damage is bounded by the capacity A genuinely contributes. An attacker cannot harm the network beyond the resources it actually brings.

### Transitivity by delegation, with a margin

Trust is *not* transitive by default (A trusting B does not make A trust C). Transitivity is achieved by **delegation with a strict margin**: when B forwards A's priority-10 request to C, B forwards it at a *reduced* priority (the paper's figure shows priority 10 in, priority 5 out) and reduces its own trust in A accordingly. The margin is what stops the obvious credit-loop attack (route trust around a cycle to mint it). This is the cleanest published precedent for the brainstorm's parked "transitive/multi-hop credit" fork — and it gets there *without* EigenTrust-style global eigenvector computation, which is the Sybil-fragile approach Myrhiza wants to avoid (see [`prior-art/sybil-resistance/eigentrust.md`](../sybil-resistance/eigentrust.md)).

## Honest limits

- GNUnet trust is a **single scalar per peer**, not a resource vector. It prices "a request" by a priority number, not by CPU-ms / byte-hours / bandwidth. The *heterogeneous* half of Myrhiza's model is OurGrid's contribution, not GNUnet's.
- The crediting is **symmetric in unit** (you credit R by the priority *you* declared), which is trust-minimal on the requester side but means the provider's actual cost never enters. Myrhiza's directional rule ("value work I receive by my own replacement cost") is the dual choice and is *not* what GNUnet does — GNUnet credits by requester-declared priority, capped by held trust.
- The model was designed for an **anonymous file-sharing** workload. Kügler's PET 2003 analysis ("An Analysis of GNUnet and the Implications for Anonymous, Censorship-Resistant Networks," LNCS 2760, pp. 161–176) found the *performance/anonymity* features (not the economic model per se) could be exploited to deanonymize downloaders — a caution that excess-based optimizations can leak information, relevant if Myrhiza ties serving behavior to observable timing.

## Implications for Myrhiza (framing-disclosed — see [`README.md`](README.md))

1. **The excess rule is the bootstrap answer.** "Serve free when idle, charge only under load" gives newcomers a way in *and* keeps free-riding cheap to tolerate — adopt the shape. See [`lessons.md`](lessons.md) Borrow.
2. **Private per-peer trust validates the off-determinism-path ledger.** GNUnet proves a purely local, non-converged trust scalar is enough to run a real network. Myrhiza's behavior-component ledger is the same architectural choice.
3. **d ≤ c + ε is the Sybil bound to aim for.** A reciprocity rule where a fresh identity gains nothing and damage is bounded by contributed capacity is the target property. Myrhiza gets there via non-negativity + social-graph-scaled grace buffers; GNUnet gets there via the excess rule. Compare both.
4. **Delegation-with-margin over EigenTrust.** If Myrhiza ever wants transitive credit, GNUnet's margin-on-forward is the Sybil-safe pattern; EigenTrust's global aggregation is the fragile one.

## Sources

- [Grothoff, "An Excess-Based Economic Model for Resource Allocation in Peer-to-Peer Networks," Wirtschaftsinformatik 45(3), 2003, pp. 285–292](https://grothoff.org/christian/ebe.pdf) — verified against PDF: trust private/per-peer/non-negative, priority = `min(requested, held trust)`, excess rule, damage bound d ≤ c + ε, delegation-with-margin, fresh-identity low exchange rate.
- [Springer record, DOI 10.1007/BF03254946](https://link.springer.com/article/10.1007/BF03254946) — venue, volume, pages.
- [Bennett & Grothoff, "gap — practical anonymous networking," PET 2003](https://grothoff.org/christian/aff.pdf) — the GAP protocol carrying the priority field.
- [Kügler, "An Analysis of GNUnet and the Implications for Anonymous, Censorship-Resistant Networks," PET 2003, LNCS 2760, pp. 161–176](https://www.freehaven.net/anonbib/cache/kugler:pet2003.pdf) — deanonymization analysis (targets anonymity, not the economic model directly).
- [GNUnet release news (0.27.0, March 2026)](https://lists.gnu.org/archive/html/info-gnu/2026-03/msg00007.html), [GNUnet docs](https://docs.gnunet.org/master/about.html) — maintenance status.
- Cross-references: [`README.md`](README.md), [`open-problems.md`](open-problems.md) §2–3, [`lessons.md`](lessons.md), [`prior-art/sybil-resistance/eigentrust.md`](../sybil-resistance/eigentrust.md), [`reports/2026-05-29-reciprocity-economy-brainstorm/README.md`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md).
