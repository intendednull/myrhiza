**Date:** 2026-05-22
**Status:** active
**Subject:** Sybil-resistance + free-riding-defense literature survey — what Myrhiza can borrow for distributed-maintenance enforcement

# Sybil resistance and free-riding defense in P2P systems

This folder is a multi-paper survey, not a single-project deep-dive. It collects ~20 years of academic and deployed work on the two intertwined problems Willow's `open-problems.md` flags as the **#1 unresolved enforcement question** for Myrhiza:

1. **Sybil resistance** — making it costly for one attacker to look like many peers.
2. **Free-riding defense** — making it costly for one peer to consume the network without contributing maintenance work.

Neither is solved in the general case. Every approach trades one of three properties — *Sybil-resistance*, *permissionlessness*, *cost-freeness* — for the other two. The Holochain folder calls this the "Sybil/permissionless/free trilemma" (`prior-art/holochain/open-problems.md#1-sybil-resistance`); the framing is real and it shapes everything else in this corpus.

**Why this corpus exists.** PR #636's research notes (sibling to the master spec) frame maintenance work as a fourth class of components alongside `state-apply` / `state-propose` / `interaction` / `behavior`: persister, snapshot provider, sync provider, replay buffer. A peer's "participation" is the set of maintenance components it has instantiated plus the capacity it has declared. The hard problem is **enforcement under Sybil**: a custom client that does not run maintenance components, multiplied by spinning up many identities, free-rides on honest participants. Self-reported participation is gameable. Refusal-to-serve-non-participants is the enforcement primitive Willow's note proposes; this corpus catalogs the metrics and Sybil-defenses underneath it.

**The Willow advantage.** Most P2P systems bootstrap social graphs they don't have. Willow has one for free — every invite is a permission edge between two human-attested identities. That graph is exactly the input SybilGuard / SybilLimit / Whanau need. Whether it actually works depends on assumptions those papers spell out and Willow may or may not satisfy. The synthesis lives in [`lessons.md`](lessons.md) §"Validates" #1 and [`taxonomy.md`](taxonomy.md) §"Where Myrhiza sits" — a dedicated `myrhiza-social-graph.md` deep-dive is queued as future work.

## Key facts

| Fact | Value |
|---|---|
| Survey scope | ~10 papers + 4 deployed systems, 2000–2024 |
| Oldest reference | Adar & Huberman, "Free Riding on Gnutella," First Monday Oct 2000 |
| Most-cited foundational paper | EigenTrust (Kamvar / Schlosser / Garcia-Molina, WWW 2003) — ~5,800+ citations |
| Most-cited critique | Alvisi et al., "SoK: The Evolution of Sybil Defense via Social Networks," IEEE S&P 2013 |
| Most-deployed reciprocity scheme | BitTorrent choking algorithm (Cohen, P2PECON 2003); ~25 years of production data |
| Research-grade vs deployed | All Sybil-defense literature is **research-grade**. The deployed answers (BitTorrent, Bitswap, BarterCast) are all *reciprocity*, not *Sybil-defense* |
| Strongest cryptographic answer | Filecoin proofs of replication / spacetime — heavy machinery, blockchain-bound |

## Contents

Each file is independent and can be skimmed standalone.

**Reference**
- [**Taxonomy**](taxonomy.md) — the canonical "tit-for-tat vs reputation vs social-graph vs DHT-responsibility vs PoW/PoS" partition. **Read first if new to the area.**
- [**Algorithms index**](algorithms.md) — one-paragraph summary per algorithm with paper IDs, year, venue, status. Covers Adar & Huberman 2000 (Gnutella free-riding measurement), Tribler BarterCast, Filecoin PoRep/PoSt.

**Per-algorithm / per-system files**
- [**BAR Gossip + BAR FT + FlightPath**](bar-gossip.md) — Byzantine / Altruistic / Rational research lineage from UT Austin's LASR group (Aiyer/Alvisi/Clement/Dahlin et al., 2005–2008). The right academic frame; never deployed at scale.
- [**EigenTrust and reputation systems**](eigentrust.md) — global reputation via gossip eigenvector. Sybil-vulnerable by design; foundational reference for the reputation-system family (PowerTrust, PeerTrust, FuzzyTrust).
- [**SybilGuard + SybilLimit**](sybilguard-sybillimit.md) — social-graph Sybil defenses (Yu et al., SIGCOMM 2006 + IEEE S&P 2008). The papers most directly relevant if Myrhiza uses its permission/invite graph as the social-graph input. Includes the Alvisi 2013 SoK critique.
- [**Whanau**](whanau.md) — Sybil-proof DHT routing (Lesniewski-Laas / Kaashoek, NSDI 2010). Uses a social graph to build the DHT routing tables themselves.
- [**BitTorrent choking + PropShare**](bittorrent.md) — Cohen's tit-for-tat (P2PECON 2003) and the auction-theoretic improvement (Levin et al., SIGCOMM 2008). The deployed-at-scale reference point. Per-connection reciprocity, *not* Sybil defense.
- [**IPFS Bitswap**](ipfs-bitswap.md) — ledger-based reciprocity for block exchange. Closer in spirit to BitTorrent than to a reputation system; the ledger as deployed is mostly informational, not enforced.
- [**Verifying self-reported cost**](self-reported-cost-verification.md) — BOINC / Folding@home / Gridcoin: how volunteer-computing systems score (and get gamed on) *self-measured* contribution. The home of the credit-stuffing problem the reciprocity-economy brainstorm must solve — verify, bound, and sign-inside-the-event a self-attested cost. Cross-links the brainstorm report.

**Synthesis**
- [**Open problems**](open-problems.md) — what none of the above structurally solves; the gaps Myrhiza must accept or work around.
- [**Lessons for Myrhiza**](lessons.md) — **the consult-this-when-designing file.** Validates / avoid / borrow. The "permission-graph-as-Sybil-input" thesis lands in §"Validates" #1 + [`taxonomy.md`](taxonomy.md) §"Where Myrhiza sits" (a dedicated `myrhiza-social-graph.md` deep-dive is queued as future work).

## How to use this prior-art doc

Designing a Myrhiza feature with overlap to maintenance enforcement or Sybil resistance? Start with [`taxonomy.md`](taxonomy.md) for orientation, then [`lessons.md`](lessons.md) for action-oriented synthesis. Drop into per-paper files for primary-source depth when a specific algorithm becomes a candidate.

**Framing disclosure.** This corpus is written from a Myrhiza-runtime stance — most "Implications for Myrhiza" sub-sections frame each algorithm through the lens of *"does this help enforce distributed maintenance under Sybil, given that Myrhiza already has a permission/invite trust graph?"* That framing biases the reading: it foregrounds social-graph approaches (because Myrhiza has the input) and backgrounds approaches that require independent cost imposition (PoW, PoS) or central authorities (CAPTCHAs, Sybil-detection services). A future reader asking *"should Myrhiza use the permission graph at all?"* should weigh the corpus accordingly — it is a learn-from-the-literature-into-graph-aware-Myrhiza artifact, not a neutral catalog.

**Honest framing on the literature itself.** The Sybil-defense literature is *research-grade*. SybilGuard / SybilLimit / Whanau have never been deployed at scale. The deployed reciprocity schemes (BitTorrent, Bitswap, BarterCast) are Sybil-tolerant per-connection but globally exploitable — they do not solve Sybil, they just make any single connection economically uninteresting to attack. Filecoin's PoSt is heavy machinery designed around a blockchain settlement layer Myrhiza doesn't have. **There is no off-the-shelf answer Myrhiza can adopt unchanged.** What this corpus gives a Myrhiza spec author is a vocabulary, a set of known-broken approaches to avoid, and a small set of design primitives worth adapting.

## Cross-links

- `prior-art/willow/open-problems.md` §12–69 — the canonical statement of the Myrhiza-side problem.
- `prior-art/holochain/open-problems.md` §1–2 — the Sybil/free-rider trilemma framing and Holochain's honest "we don't solve this" framing.
- `prior-art/iroh/` — transport-layer reality Myrhiza inherits; relays-as-bridge model has Sybil implications.
- `prior-art/pears/` — Hyperswarm's pure-reciprocity model and its limits.
- `prior-art/agoric-endo/` — capability-mediated-participation as a comparator approach.
- `prior-art/libp2p/gossipsub.md` — peer-scoring in deployed gossipsub.

## Sources

- [Adar & Huberman, "Free Riding on Gnutella," *First Monday* 5(10), Oct 2000](https://firstmonday.org/ojs/index.php/fm/article/view/792)
- [Cohen, "Incentives Build Robustness in BitTorrent," P2PECON 2003](https://bittorrent.org/bittorrentecon.pdf)
- [Kamvar / Schlosser / Garcia-Molina, "The EigenTrust Algorithm for Reputation Management in P2P Networks," WWW 2003, pp. 640–651, DOI 10.1145/775152.775242](https://nlp.stanford.edu/pubs/eigentrust.pdf)
- [Aiyer / Alvisi / Clement / Dahlin / Martin / Porth, "BAR Fault Tolerance for Cooperative Services," SOSP 2005, pp. 45–58, DOI 10.1145/1095810.1095816](https://www.cs.cornell.edu/lorenzo/papers/sosp05.pdf)
- [Li / Clement / Wong / Napper / Roy / Alvisi / Dahlin, "BAR Gossip," OSDI 2006](https://www.usenix.org/conference/osdi-06/bar-gossip)
- [Yu / Kaminsky / Gibbons / Flaxman, "SybilGuard: Defending against Sybil Attacks via Social Networks," SIGCOMM 2006, pp. 267–278](http://david.choffnes.com/classes/cs4700sp14/papers/sybilguard.pdf)
- [Yu / Gibbons / Kaminsky / Xiao, "SybilLimit: A Near-Optimal Social Network Defense against Sybil Attacks," IEEE S&P 2008, pp. 3–17](https://www.comp.nus.edu.sg/~yuhf/sybillimit-tr.pdf)
- [Levin / LaCurts / Spring / Bhattacharjee, "BitTorrent is an Auction: Analyzing and Improving BitTorrent's Incentives," SIGCOMM 2008](http://www.cs.umd.edu/projects/propshare/)
- [Li / Clement / Marchetti / Kapritsos / Robison / Alvisi / Dahlin, "FlightPath: Obedience vs. Choice in Cooperative Services," OSDI 2008](https://www.cs.utexas.edu/~lorenzo/papers/flightpath.pdf)
- [Meulpolder / D'Acunto / Pouwelse, "BarterCast: A practical approach to prevent lazy freeriding in P2P networks," HotP2P 2009](https://www.researchgate.net/publication/228871839_BarterCast_A_practical_approach_to_prevent_lazy_freeriding_in_P2P_networks)
- [Lesniewski-Laas / Kaashoek, "Whanau: A Sybil-proof Distributed Hash Table," NSDI 2010](https://pdos.csail.mit.edu/papers/whanau-nsdi10.pdf)
- [Alvisi / Clement / Epasto / Lattanzi / Panconesi, "SoK: The Evolution of Sybil Defense via Social Networks," IEEE S&P 2013](https://oaklandsok.github.io/papers/alvisi2013.pdf)
- [Protocol Labs, "Filecoin: Proof-of-Replication, Power-Fault-Tolerance and Research Roadmap"](https://www.protocol.ai/blog/filecoin-proof-of-replication-power-fault-tolerance-research-roadmap/)
- [Benet / Dalrymple, "Proof of Replication," Filecoin Technical Report](https://filecoin.io/proof-of-replication.pdf)
- [IPFS Bitswap Protocol spec](https://specs.ipfs.tech/bitswap-protocol/)
- Cross-references: `prior-art/willow/open-problems.md`, `prior-art/holochain/open-problems.md`, `prior-art/iroh/`, `prior-art/pears/`, `prior-art/agoric-endo/`, `prior-art/libp2p/`.
