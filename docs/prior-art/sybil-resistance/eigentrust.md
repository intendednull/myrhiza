**Date:** 2026-05-22
**Status:** active
**Subject:** EigenTrust + reputation-system family — the canonical reputation algorithm, its Sybil-vulnerability, and why "reputation systems are fragile" is a real critique

# EigenTrust and reputation systems

EigenTrust (Kamvar / Schlosser / Garcia-Molina, WWW 2003) is the **canonical reference** for "reputation system in a P2P network." It's also the canonical *bad* example of what happens when you try to bolt a reputation system onto an open P2P network without solving Sybil first. The paper is honest about the limitation. The corpus that grew around it (PowerTrust, PeerTrust, FuzzyTrust, ServiceTrust, etc.) is mostly variations on the same theme with the same fundamental weakness.

This file exists to **surface that critique honestly** before any Myrhiza spec author considers a reputation-system approach.

## The EigenTrust algorithm

- **Citation:** Sepandar D. Kamvar, Mario T. Schlosser, Hector Garcia-Molina, "The EigenTrust Algorithm for Reputation Management in P2P Networks," **WWW 2003** (12th International Conference on World Wide Web), Budapest, May 20–24, 2003, pp. 640–651. [DOI 10.1145/775152.775242](https://dl.acm.org/doi/10.1145/775152.775242). [PDF](https://nlp.stanford.edu/pubs/eigentrust.pdf).

The algorithm:

1. **Local trust.** Each peer i records, after every interaction with peer j, whether the interaction was "satisfactory" (s_ij = +1) or "unsatisfactory" (s_ij = −1). Local trust c_ij = max(s_ij, 0) / Σ_k max(s_ik, 0) — peer i's normalized opinion of j.
2. **Global trust.** Let C be the matrix of local trusts c_ij. The global trust vector **t** is the principal eigenvector of C^T — equivalently, t = lim_{k→∞} (C^T)^k * e for any initial distribution e. The interpretation: **t_j is the probability that a random walk starting from any peer, where transitions are weighted by trust, ends up at peer j.**
3. **Distributed computation.** Each peer holds its local trust row c_i*. To compute global trust, peers run a *distributed power iteration*: in each round, peer j updates its trust score by querying its in-neighbors for their current scores, weighted by the local trusts they assigned j. Converges in O(log n) rounds for well-connected graphs.
4. **Pre-trusted peers.** To anchor the eigenvector, EigenTrust posits a small set of *pre-trusted peers* whose trust is normalized to 1 by external means. Without this, the algorithm is *unanchored* — the eigenvector exists, but Sybils can mutually rate each other to dominate it.

## What it gets right

- **A principled formulation.** Reputation as a fixed-point of a linear operator on the trust matrix. Maps cleanly onto well-understood theory (Markov chains, PageRank — EigenTrust is essentially PageRank applied to peer-trust matrices).
- **Distributed computation.** No central authority needed once peer roles are stable.
- **Reasonable convergence.** Power iteration converges quickly for well-conditioned matrices.

The paper is rigorous and clearly written. The implementation is plausible. **The problem is not the algorithm — the problem is the threat model the algorithm operates under.**

## The Sybil vulnerability

EigenTrust without pre-trusted peers is wildly Sybil-vulnerable. Consider a single attacker controlling k Sybil identities:

- Each Sybil rates the other (k−1) Sybils with c = 1/(k−1). The Sybil sub-matrix is doubly-stochastic (every row sums to 1).
- The Sybil region is a closed sub-chain — once the random-walk enters, it stays. The Sybil region attracts probability mass disproportionate to its edges into the honest region.
- For graphs where Sybils have *any* incoming edges from the honest region, the Sybil region can be made to dominate the eigenvector with surprisingly few attack edges (~log n suffices empirically).

The pre-trusted-peer mechanism *partially* mitigates this — pre-trusted peers act as a "teleport" source in the random walk, biasing mass toward the honest region. But pre-trusted peers are themselves a centralization vector and a Sybil target. **A complete Sybil defense must come from outside the EigenTrust algorithm.**

The paper acknowledges this (§5.2):

> "We assume that pre-trusted peers are non-malicious. … In practice, a small set of pre-trusted peers can be chosen [by the network operator] ..."

In a permissionless P2P system, "a small set of pre-trusted peers" *is* a centralization. The decentralized framing is partly aspirational.

## The reputation-system family

EigenTrust inspired ~20 years of derivatives. Common patterns:

- **PowerTrust** (Zhou & Hwang, IEEE TPDS 2007). EigenTrust variant using power-law node distribution; uses "power nodes" as reputation aggregators. Same Sybil weakness; the power-node set is a Sybil target.
- **PeerTrust** (Xiong & Liu, IEEE TKDE 2004). Adds *transaction context* — reputation weighted by the size of the transaction. Reduces "score-pumping" via many small fake transactions; doesn't solve Sybil.
- **FuzzyTrust** (Song et al., IEEE/ACM TON 2005). Adds fuzzy-logic aggregation of multiple trust signals. Engineering refinement; no Sybil improvement.
- **ServiceTrust** (Su et al., IEEE TPDS 2011) and many others — each contributes algorithmic refinements; none solve Sybil at the algorithm layer.

The pattern is clear: **without a Sybil defense underneath, reputation systems can be tuned but not fixed.**

## The fragility critique

Two classes of attacks dominate the literature:

### Sybil + collusion

Already covered — k Sybils mutually rate each other to dominate the eigenvector.

### Whitewashing

A peer accumulates negative reputation, *drops the identity*, and rejoins with a fresh identity. Reputation systems that allow free identity creation cannot defend against whitewashing — the cost of a fresh identity is zero, so a rational attacker drops bad identities as soon as the cost of operating under them exceeds the cost of starting fresh.

The defense: **make identity creation costly**. Either via Sybil-defense (limit identity multiplication) or via initialization-cost (newcomers start at a *low* trust score and must build up). EigenTrust does the second imperfectly: new peers start with the uniform pre-trusted distribution, which is enough to participate but not enough to abuse.

### Camouflage / score-pumping

A long-running attacker behaves honestly to build reputation, then defects late. Counter-defenses are slow to react because reputation systems trade reactivity for stability. The "burn reputation suddenly" attack is well-known in mature literature.

### The Alvisi 2013 SoK on reputation

The 2013 IEEE S&P SoK paper ([Alvisi et al.](https://oaklandsok.github.io/papers/alvisi2013.pdf)) is primarily about social-graph defenses but contains a sharp aside on reputation systems:

> "Reputation systems collapse the rich structure of the social graph into a scalar per identity. This loses exactly the information — community structure, attack-edge topology — that is needed to defend against Sybils. The pre-trusted-peers crutch is a tacit admission that reputation cannot stand alone."

The corpus's takeaway: **reputation systems are an information-aggregation tool, not a Sybil-defense tool. They tell you *who is trusted by whom*; they do not tell you *who is a real person*.**

## Where reputation actually works

The honest assessment: **reputation systems work in semi-closed networks with out-of-band Sybil control.**

- eBay's seller-feedback system works because identity is gated by a credit-card / phone-number admission and reputation is reset on whitewashing only at the cost of those bindings.
- Stack Overflow's reputation works because identities are gated by email + community moderation.
- Tor's relay-flag system works because operators are gated by long-running uptime measurement + community vetting.

In each case, Sybil resistance is *external* to the reputation algorithm. The reputation algorithm operates over a Sybil-controlled population and aggregates useful information.

## Implications for Myrhiza

1. **Do not deploy a reputation system without a Sybil defense underneath.** EigenTrust-family algorithms in an open P2P network produce metrics; they do not produce Sybil resistance. Build the Sybil defense first; *then* a reputation layer can usefully aggregate behavioral information.
2. **If Myrhiza adopts a reputation layer, the natural shape is:** SybilLimit over the invite graph → bounded honest region → EigenTrust-style aggregation *over that region* → useful per-peer maintenance-contribution score. The composition is the standard "layered defenses" pattern from [`taxonomy.md`](taxonomy.md).
3. **Whitewashing is a real risk even for Myrhiza.** The invite graph slows whitewashing (you need an invite to re-join) but doesn't prevent it (sympathetic existing peers can re-invite). Design with the assumption that an attacker can re-enter with a fresh identity at some cost; don't assume permanent banishment.
4. **The pre-trusted-peers crutch translates oddly to Myrhiza.** Pre-trust in EigenTrust is essentially "the network operator vouches for these peers." Myrhiza has no single operator — but the *root inviters* of each invite tree are structurally analogous. Use that lineage explicitly if a reputation layer is added.
5. **Don't soft-pedal the critique.** The "reputation systems are fragile" line is a real critique; spec drafts that propose a reputation system without addressing whitewashing + Sybil + camouflage attacks have done insufficient threat modeling. Be honest in lessons.md.

## Sources

- [Kamvar / Schlosser / Garcia-Molina, "The EigenTrust Algorithm for Reputation Management in P2P Networks," WWW 2003](https://nlp.stanford.edu/pubs/eigentrust.pdf) — [DOI 10.1145/775152.775242](https://dl.acm.org/doi/10.1145/775152.775242), pp. 640–651.
- [Stanford NLP group page for EigenTrust](https://nlp.stanford.edu/pubs/eigentrust.pdf).
- [Xiong & Liu, "PeerTrust: Supporting Reputation-Based Trust for Peer-to-Peer Electronic Communities," IEEE TKDE, 2004](https://ieeexplore.ieee.org/document/1318566).
- [Zhou & Hwang, "PowerTrust," IEEE TPDS, 2007](https://ieeexplore.ieee.org/document/4287437).
- [Alvisi et al., "SoK: The Evolution of Sybil Defense via Social Networks," IEEE S&P 2013](https://oaklandsok.github.io/papers/alvisi2013.pdf) — §6 on reputation systems.
- Cross-references: [`taxonomy.md`](taxonomy.md), [Tribler BarterCast in `algorithms.md`](algorithms.md) (deployed reputation system), [`lessons.md`](lessons.md), [`lessons.md` §"Validates" #1 + `taxonomy.md` §"Where Myrhiza sits"](lessons.md).
