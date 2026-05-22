**Date:** 2026-05-22
**Status:** active
**Subject:** BAR Fault Tolerance + BAR Gossip + FlightPath — the Byzantine/Altruistic/Rational research lineage from UT Austin LASR

# BAR Gossip and the BAR research lineage

The most academically respectable framing of "honest peers + selfish peers + actively-malicious peers" comes from a single research group: UT Austin's Laboratory for Advanced Systems Research (LASR), led by Lorenzo Alvisi and Mike Dahlin, in a four-paper sequence between 2005 and 2008. The framework is the **BAR model** — Byzantine / Altruistic / Rational — and it's the right vocabulary for talking about Myrhiza's distributed-maintenance enforcement.

The implementations are research-grade. None of these systems shipped at scale. The **vocabulary**, **the proof techniques**, and **the protocol design patterns** are the durable contribution.

## The BAR model

Three node classes:

- **Byzantine** — may deviate arbitrarily from the protocol. Adversary-controlled or buggy. The classical BFT enemy.
- **Altruistic** — follows the protocol faithfully, regardless of self-interest. The classical reliable participant.
- **Rational** — self-interested but boundedly so. Will deviate from the protocol if and only if doing so improves the rational node's *utility* (where utility is defined by the protocol designer and embedded in the threat model).

The key insight: **most real-world P2P misbehavior is rational, not Byzantine**. Free-riders don't want to crash the network — they want to consume it without contributing. Classical BFT protocols overshoot by assuming the worst about every misbehaving node; BAR protocols give rational nodes a *clear incentive to behave* and only need full Byzantine machinery against the (usually small) Byzantine set.

This is exactly the lens Willow's `research-notes-distributed-maintenance.md` needs:

- Byzantine = peers running modified clients that emit invalid events / refuse to serve / actively poison data.
- Altruistic = honest reference-client peers.
- **Rational = peers running modified clients to skip maintenance work while still consuming the network** — the dominant threat per the Willow note.

Protocols robust against the rational set are the goal. Anti-free-rider design is anti-rational-deviation design.

## Paper 1: BAR Fault Tolerance for Cooperative Services (SOSP 2005)

- **Citation:** Aiyer / Alvisi / Clement / Dahlin / Martin / Porth, *Proceedings of the 20th ACM Symposium on Operating Systems Principles (SOSP 2005)*, Brighton UK, October 23–26, 2005, pp. 45–58. [DOI 10.1145/1095810.1095816](https://dl.acm.org/doi/10.1145/1095810.1095816). [PDF (Cornell mirror)](https://www.cs.cornell.edu/lorenzo/papers/sosp05.pdf).
- **Contribution:** Introduces the BAR model. Provides a state-machine-replication protocol that tolerates a mix of all three node classes, with rigorous game-theoretic analysis of the rational set's incentives.
- **Application case study:** **Cooperative backup** — peers storing copies of each other's encrypted backups. A canonical "Byzantine adversaries can corrupt; rational adversaries free-ride; altruists carry both" workload.

The protocol design hinges on three primitives:

1. **Public proofs of action.** Every protocol step a peer takes is signed and observable, so rational deviation is detectable.
2. **Penance protocols.** Penalties for detected misbehavior are encoded as required-extra-work, paid back to the network rather than burned. Makes "skip work" strictly worse than "do work" *for a rational node*.
3. **Symmetric obligations.** All peers do the same amount of work; nobody is a coordinator. Removes the most-tempting deviation target.

The paper proves that the protocol is **incentive-compatible** under standard rational-actor assumptions — a rational node strictly prefers following the protocol to any unilateral deviation.

### Limitations

- The protocol's per-operation cost is high (~3× a non-BAR equivalent). Acceptable for low-traffic services like cooperative backup; not obviously acceptable for high-traffic data sync.
- The "rational" model assumes nodes optimize a *known* utility function. Real rational nodes have hidden state (e.g. they might value privacy in ways the protocol doesn't model). The model is rigorous; the threat model may be incomplete.
- Byzantine fraction bounded conventionally (n ≥ 3f + 1 for f Byzantine nodes).

## Paper 2: BAR Gossip (OSDI 2006)

- **Citation:** Li / Clement / Wong / Napper / Roy / Alvisi / Dahlin, *Proceedings of the 7th USENIX Symposium on Operating Systems Design and Implementation (OSDI 2006)*. [USENIX page](https://www.usenix.org/conference/osdi-06/bar-gossip).
- **Contribution:** First P2P **live streaming** protocol with BAR tolerance.
- **Application:** Live video / audio streaming over P2P, where missed deadlines = perceptible degradation. ~99% of broadcast packets delivered to all altruistic + rational nodes within latency budget even when 40% of nodes are rational.

The streaming workload is harder than backup because:

- Real-time deadlines. Penance protocols that "make the cheater pay later" don't help if the cheating *now* causes a missed frame.
- Asymmetric importance — early packets in a streaming epoch enable subsequent decoding, so a rational node attacking *only* early packets disproportionately harms.

BAR Gossip's two-part design:

1. **Balanced exchange.** Peers in a gossip round exchange equal-volume payloads, atomically — neither side can extract more bytes than they contribute. Implemented as a cryptographic exchange protocol: peer A reveals a hash, B sends payload encrypted under that hash, A reveals the preimage, B can now decrypt; if A defects (never reveals), neither side gets the payload, so A has no rational reason to defect.
2. **Optimistic push.** A small fraction of bandwidth is "tip" — peers push to random partners without expecting reciprocation. Bootstrap mechanism for new peers and patch mechanism for missing packets.

The **verifiable pseudo-random partner selection** is the load-bearing insight: each peer's gossip partner for round t is deterministically computed from a public seed + the peer's identity. Rational peers cannot game who they gossip with to favor friends or starve enemies.

### Significance for Myrhiza

The two-party exchange protocol is the cleanest construction in this corpus for "you serve me iff I serve you, on this specific transaction." If Myrhiza ever needs *cryptographic* per-event reciprocity (rather than statistical / over-time accounting), BAR Gossip's exchange is the reference. Sync of a Willow state-apply log between two peers is potentially this shape.

## Paper 3: FlightPath: Obedience vs. Choice (OSDI 2008)

- **Citation:** Li / Clement / Marchetti / Kapritsos / Robison / Alvisi / Dahlin, *Proceedings of OSDI 2008*, San Diego CA, December 2008.
- **Contribution:** Relaxes BAR Gossip's strict-obedience model. Lets rational nodes have *some* discretion (e.g. preferring certain partners) without losing protocol guarantees.

The motivation: BAR Gossip's "verifiable pseudo-random partner selection" leaves rational nodes with zero choice. This makes the protocol mathematically clean but ergonomically rigid — a rational node with extra bandwidth has no way to "donate" it productively without violating protocol. FlightPath introduces *latitude*: rational nodes can choose among a *set* of acceptable behaviors, and the protocol bounds the harm any single choice can cause.

The technical mechanism is a *latitude-bounded protocol*: instead of "do X exactly," the spec says "do X, or X′, or X′′; choose whichever optimizes your utility within these constraints." Each X-variant is independently game-theoretically analyzed.

### Significance for Myrhiza

The model is suggestive for Myrhiza's deployment-role abstraction (PR #636: "behavior components run in operator-controlled instances"). Operators *will* want latitude — which peers to peer with, when to serve, how to allocate scarce upload. The strict-obedience BAR Gossip model assumes none of this. FlightPath's framing — "give them latitude but bound it" — is the right shape, even if FlightPath's specific construction is too narrow.

## Paper 4: BAR Primer (book chapter / tutorial, ~2010)

- **Citation:** Clement / Dahlin / Alvisi, "BAR Primer" — pedagogical paper summarizing the BAR model and the SOSP'05 / OSDI'06 / OSDI'08 trilogy.
- **Use:** Read first if approaching BAR cold. Not a research contribution per se; a clean introduction to the model.

## What BAR doesn't solve

- **Sybil resistance.** BAR assumes *identity* is given. The BAR model says "given these N nodes with these identities, here's how to make the protocol robust to their misbehavior" — it does *not* prevent the same human from controlling many BAR nodes. Sybil-defense must come from elsewhere.
- **Bootstrap.** BAR protocols assume the node set is known and stable enough for the analysis to apply. New nodes joining mid-protocol need an admission mechanism BAR doesn't provide.
- **Utility specification.** The "rational" set's utility function is part of the spec, not a given. Specifying it correctly for a real workload (Willow state-apply log sync? maintenance-component contribution?) is non-trivial and is itself an open research problem.

The BAR papers explicitly stack with a separate Sybil defense — e.g. the SOSP'05 paper assumes an external membership service. Myrhiza's permission/invite graph is a candidate for that external service. The BAR model would then operate over the SybilLimit-accepted honest region.

## Deployment status

| System | Status |
|---|---|
| BAR FT (SOSP'05) cooperative backup | Research prototype; never operationalized. |
| BAR Gossip streaming | Research prototype; never operationalized. UT Austin internal use only. |
| FlightPath | Research prototype; never operationalized. |
| BAR model (the vocabulary) | Adopted in research literature; ~1,500 citations across the trilogy. Still active framing in 2025+ papers on incentive-aware distributed protocols. |

The LASR group at UT Austin remained productive into the mid-2010s; Lorenzo Alvisi moved to Cornell in 2015 ([Cornell publications page](https://www.cs.cornell.edu/lorenzo/publications.html)). Allen Clement went to Google. Mike Dahlin is now at UT Austin Dell Medical School. The BAR-specific research line is dormant but the model lives on.

## Implications for Myrhiza

1. **Adopt the BAR vocabulary.** "Byzantine / Altruistic / Rational" is sharper than "honest / malicious / free-rider." Myrhiza specs should use the BAR partition when describing threat models for maintenance enforcement. The Willow distributed-maintenance note already approaches this framing implicitly — adopt the language explicitly.
2. **The right enemy is rational, not Byzantine.** Most peers running modified clients to skip maintenance work are rational nodes trying to consume the network cheaply, not malicious adversaries trying to break it. Protocol design should target rational deviation first; Byzantine resistance comes after.
3. **Penance protocols are an under-used pattern.** Instead of binary "in / out," Myrhiza could use *graduated participation* — peers who fall behind their declared maintenance capacity pay back via extra work before regaining full service. The pattern is well-developed in the BAR literature but barely deployed.
4. **Verifiable pseudo-random selection is the right pattern for peer-pairing.** If Myrhiza needs to pair peers for sync, broadcast, or any per-round-action, deterministic selection from a public seed prevents rational peers from cherry-picking partners. The Whanau DHT routing uses essentially this pattern. See [`whanau.md`](whanau.md).
5. **BAR protocols compose with social-graph Sybil defense.** The natural Myrhiza stack: SybilLimit over invite graph → admission control; BAR-style protocol over admitted nodes → in-network enforcement. The composition is novel; the pieces are not.
6. **Don't expect ready code.** The BAR literature is research-grade. No production-quality libraries; no maintained reference implementations. Myrhiza would be re-implementing from papers if it adopts BAR primitives directly. Budget the time.

## Sources

- [Aiyer / Alvisi / Clement / Dahlin / Martin / Porth, "BAR Fault Tolerance for Cooperative Services," SOSP 2005](https://www.cs.cornell.edu/lorenzo/papers/sosp05.pdf) — DOI [10.1145/1095810.1095816](https://dl.acm.org/doi/10.1145/1095810.1095816). Pages 45–58.
- [Li et al., "BAR Gossip," OSDI 2006](https://www.usenix.org/conference/osdi-06/bar-gossip) — USENIX OSDI proceedings.
- [Li et al., "FlightPath: Obedience vs. Choice in Cooperative Services," OSDI 2008](https://www.cs.utexas.edu/~lorenzo/papers/flightpath.pdf).
- [BAR Gossip extended paper, April 2006 tech report](https://www.cs.utexas.edu/~dahlin/papers/bar-gossip-apr-2006.pdf).
- [UT Austin BAR project page](https://www.cs.utexas.edu/~lorenzo/bar.html) — historical project landing page.
- [Lorenzo Alvisi's publications](https://www.cs.cornell.edu/lorenzo/publications.html) — current canonical bibliography for all four BAR papers.
- Cross-references: [`taxonomy.md`](taxonomy.md), [`open-problems.md`](open-problems.md), [`lessons.md`](lessons.md), `prior-art/willow/open-problems.md`.
