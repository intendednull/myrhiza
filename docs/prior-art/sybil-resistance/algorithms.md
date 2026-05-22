**Date:** 2026-05-22
**Status:** active
**Subject:** Per-algorithm index — short summaries with paper IDs, year, venue, deployment status

# Algorithms index

One-paragraph summary per algorithm, ordered by category (see [`taxonomy.md`](taxonomy.md)). Each row links to the deeper per-paper file when one exists. Use this file to find an algorithm quickly; use the per-paper file for primary-source depth.

## Tit-for-tat / pairwise reciprocity

### BitTorrent choking algorithm

- **Paper:** Bram Cohen, "Incentives Build Robustness in BitTorrent," **P2PECON 2003** (1st Workshop on Economics of Peer-to-Peer Systems, Berkeley), pp. 68–72.
- **Mechanism:** Each peer maintains 4 *unchoked* upload slots filled by the 4 peers who uploaded fastest to it in the last 10s. A 5th *optimistic-unchoke* slot rotates every 30s to discover new partners. Peers without reciprocating downloads get *choked* (no upload). Snubbed peers retaliate by stopping uploads.
- **Sybil property:** Per-connection Sybil-tolerant; new Sybils start at zero credit.
- **Deployment:** ~25 years of production. The most widely-deployed P2P reciprocity scheme. See [`bittorrent.md`](bittorrent.md).
- **Weakness:** Cohen's choking algorithm provably leaves bandwidth on the table because it ignores capacity asymmetry. PropShare (below) is the auction-theoretic improvement.

### PropShare

- **Paper:** Levin / LaCurts / Spring / Bhattacharjee, "BitTorrent is an Auction: Analyzing and Improving BitTorrent's Incentives," **SIGCOMM 2008**.
- **Mechanism:** Replace choking's rank-and-cut policy with a proportional-share auction — give upload bandwidth to each peer in proportion to that peer's contributed bandwidth. Strategy-proof: lying about contributions doesn't help.
- **Sybil property:** Like BitTorrent — Sybil-tolerant per connection, more robust against collusion than vanilla choking.
- **Deployment:** Never widely deployed; vanilla BitTorrent remained dominant. Empirically faster downloads in lab and PlanetLab settings. See [`bittorrent.md`](bittorrent.md).

### IPFS Bitswap ledger

- **Spec:** [Bitswap Protocol](https://specs.ipfs.tech/bitswap-protocol/), Protocol Labs, ongoing development under the [Boxo](https://github.com/ipfs/boxo) project (2023–present).
- **Mechanism:** Each peer maintains a *ledger* of bytes-sent and bytes-received per other peer. The ledger informs a decision engine that picks who to serve when bandwidth is contended. The IPFS white paper proposed credit-based serving; deployed go-bitswap is closer to "best-effort with peer-scoring." The original white-paper economic strategy was largely **not implemented** in production go-bitswap; the ledger as deployed is more informational than enforced.
- **Sybil property:** Sybil-tolerant per connection. Globally exploitable.
- **Deployment:** IPFS-wide; one of the few production deployments of a P2P ledger reciprocity model. Recent (2024–2025) work: Bitswap Broadcast Reduction cut broadcast bandwidth 50–95% by tracking which peers actually respond. HTTP retrieval shipped alongside Bitswap as an alternative path. See [`ipfs-bitswap.md`](ipfs-bitswap.md).

### Hyperswarm reciprocity

- **System:** Hyperswarm (Holepunch / Pears stack); see `prior-art/pears/hyperswarm.md`.
- **Mechanism:** Implicit — peers serve each other based on swarm membership and topic interest, no formal ledger. Cohort-style. Closer to BitTorrent's swarm model than Bitswap's per-block accounting.
- **Sybil property:** No explicit Sybil defense at the swarm layer. Topic discovery uses Hyperdht (DHT lookup) which inherits the same Sybil weaknesses as any open DHT.
- **Deployment:** Production in Keet; low-tens-of-thousands MAU (see `prior-art/pears/`).

## Reputation systems

### EigenTrust

- **Paper:** Kamvar / Schlosser / Garcia-Molina, "The EigenTrust Algorithm for Reputation Management in P2P Networks," **WWW 2003**, pp. 640–651, DOI 10.1145/775152.775242.
- **Mechanism:** Each peer rates other peers (local trust). The global trust vector is the principal eigenvector of the normalized local-trust matrix — equivalently, the limit of the Markov-chain random walk where the transition probabilities are normalized local trusts. Computed via distributed power iteration over the network.
- **Sybil property:** *Vulnerable.* Sybils can mutually rate each other highly. Mitigation: pre-trusted seed peers anchor the eigenvector. Doesn't fully solve.
- **Citation count:** ~12,000+ (canonical reference for "reputation system in P2P").
- **Deployment:** No production deployment of EigenTrust itself. Variants and inspirations live on. See [`eigentrust.md`](eigentrust.md).

### PowerTrust

- **Paper:** Zhou & Hwang, "PowerTrust: A Robust and Scalable Reputation System for Trusted Peer-to-Peer Computing," IEEE TPDS 2007.
- **Mechanism:** EigenTrust variant using power-law node distribution to make convergence faster and use "power nodes" as reputation aggregators.
- **Sybil property:** Same weaknesses as EigenTrust; the "power node" set is itself a Sybil target.
- **Deployment:** Research-grade. See [`eigentrust.md`](eigentrust.md) for context.

### BarterCast

- **Paper:** Meulpolder / D'Acunto / Pouwelse, "BarterCast: A practical approach to prevent lazy freeriding in P2P networks," **HotP2P 2009** (6th International Workshop on Hot Topics in P2P Systems).
- **Mechanism:** Each peer maintains a local view of bandwidth exchanges (direct + gossiped-indirect). Reputation is a max-flow computation over the local-view graph: "how much bandwidth has flowed *to* this peer from sources I trust?" The peer requesting a download is preferentially served by peers who have a positive max-flow path from a trusted source.
- **Sybil property:** Sybils with no real bandwidth contribution show zero max-flow from real sources. Sybils that *do* contribute aren't free identities anymore.
- **Deployment:** Deployed in Tribler (open-source P2P client + research project from TU Delft, active 2008–present). Thousands-of-users scale. See [`tribler-bartercast.md`](tribler-bartercast.md).

## Social-graph Sybil defenses

### SybilGuard

- **Paper:** Yu / Kaminsky / Gibbons / Flaxman, "SybilGuard: Defending against Sybil Attacks via Social Networks," **SIGCOMM 2006**, pp. 267–278. Lead author Haifeng Yu (Intel Research Pittsburgh + CMU at time of writing).
- **Mechanism:** Each honest node performs O(√n log n) random walks on the social graph. Two nodes are considered "trusted neighbors" if their random walks intersect at a *shared edge* in a specific way (the "verifiable random walk" property). Sybils have only O(g) attack edges, so their random walks rarely cross into the honest region.
- **Sybil property:** Bounds the number of accepted Sybils to O(√n log n) per attack edge.
- **Deployment:** Never deployed; research-grade. See [`sybilguard-sybillimit.md`](sybilguard-sybillimit.md).
- **Critique:** Superseded by SybilLimit (same authors, 2008). The SybilGuard bound is "too many Sybils per attack edge"; SybilLimit tightens this.

### SybilLimit

- **Paper:** Yu / Gibbons / Kaminsky / Xiao, "SybilLimit: A Near-Optimal Social Network Defense against Sybil Attacks," **IEEE S&P 2008**, pp. 3–17.
- **Mechanism:** Improvement over SybilGuard. Uses *short* random walks (~O(log n)) and accepts a verifier-edge match instead of full path intersection. Tightens the per-attack-edge bound from "many Sybils" to **O(log n) Sybils per attack edge**.
- **Sybil property:** Tolerates O(n/log n) attack edges total before degrading.
- **Deployment:** Never deployed; research-grade. Most-cited social-graph Sybil defense paper. See [`sybilguard-sybillimit.md`](sybilguard-sybillimit.md).
- **Critique:** Alvisi et al. 2013 (SoK paper) showed SybilLimit (and all social-graph Sybil defenses) effectively detects community structure, not Sybils specifically — Sybils embedded in a tight community defeat the algorithm.

### SybilInfer

- **Paper:** Danezis & Mittal, "SybilInfer: Detecting Sybil Nodes using Social Networks," **NDSS 2009**.
- **Mechanism:** Bayesian-inference variant. Posterior over "which nodes are Sybils" given the observed social graph; sampling-based inference (MCMC).
- **Sybil property:** Stronger when the social graph has a clear "honest region" plus "Sybil region" with few crossings; degrades under high attack-edge counts.
- **Deployment:** Never deployed; research-grade. See [`sybilguard-sybillimit.md`](sybilguard-sybillimit.md) §SybilInfer.

### SumUp

- **Paper:** Tran / Min / Li / Subramanian, "Sybil-resilient Online Content Voting," **NSDI 2009**.
- **Mechanism:** Distinct from SybilLimit-family — used for *vote aggregation*, not membership. Computes an envelope around the honest region using attack-edge bounds; each Sybil region collectively contributes at most one vote per attack edge.
- **Sybil property:** Vote influence per attack edge is bounded, not Sybil count itself.

### Whanau

- **Paper:** Lesniewski-Laas & Kaashoek, "Whanau: A Sybil-proof Distributed Hash Table," **NSDI 2010**. MIT PDOS group.
- **Mechanism:** A DHT routing protocol where each node's O(√n log n) routing-table entries come from random walks on the social graph. Sybils only get into the routing table via attack edges, so any single lookup almost-always traverses through honest nodes. Lookups remain Sybil-resistant up to O(n/log n) attack edges.
- **Sybil property:** Same regime as SybilLimit but applied to *DHT routing* instead of *node acceptance*.
- **Deployment:** Research-grade reference implementation; no production deployment. See [`whanau.md`](whanau.md).
- **Strength:** Uniquely "Sybil-proof DHT" — most DHTs (Kademlia, Chord, mainline) are Sybil-vulnerable by construction.

## Byzantine + Altruistic + Rational research lineage

### BAR Fault Tolerance (the model)

- **Paper:** Aiyer / Alvisi / Clement / Dahlin / Martin / Porth, "BAR Fault Tolerance for Cooperative Services," **SOSP 2005**, pp. 45–58, DOI 10.1145/1095810.1095816. UT Austin LASR (Laboratory for Advanced Systems Research).
- **Mechanism:** Frames cooperative-service correctness under three node classes: **Byzantine** (arbitrarily malicious), **Altruistic** (follows protocol faithfully), **Rational** (self-interested; deviates if doing so improves utility). Provides a state-machine-replication protocol that tolerates a mix of all three classes.
- **Significance:** Established BAR as the *right* model for P2P cooperative protocols. The frame still dominates incentive-aware-protocol literature 20 years later. See [`bar-gossip.md`](bar-gossip.md).
- **Deployment:** Research-grade implementation. The model framework is the deliverable, not the artifact.

### BAR Gossip

- **Paper:** Li / Clement / Wong / Napper / Roy / Alvisi / Dahlin, "BAR Gossip," **OSDI 2006**. Same UT Austin LASR group.
- **Mechanism:** P2P data streaming under BAR. Uses *verifiable pseudo-random* partner selection so rational peers cannot game who they gossip with. Combines balanced-exchange gossip + optimistic-push for live streaming guarantees.
- **Significance:** First P2P streaming system with provable BAR-tolerance.
- **Deployment:** Research prototype only; never deployed at scale. See [`bar-gossip.md`](bar-gossip.md).

### FlightPath

- **Paper:** Li / Clement / Marchetti / Kapritsos / Robison / Alvisi / Dahlin, "FlightPath: Obedience vs. Choice in Cooperative Services," **OSDI 2008**. Same lineage.
- **Mechanism:** BAR-streaming refinement that allows rational nodes some *latitude* in protocol choices (e.g. who to peer with) while still bounding the harm those choices can cause. "Obedience vs choice" is the design knob.
- **Significance:** Showed BAR protocols can leave room for rational-node freedom without losing guarantees.
- **Deployment:** Research prototype only.

## Cryptographic proofs

### Filecoin Proof of Replication (PoRep)

- **Paper:** Benet & Dalrymple et al., "Proof of Replication" technical report, Protocol Labs, 2017; subsequent academic publications and engineering iterations through 2026.
- **Mechanism:** A storage provider seals a *unique copy* of a sector (data + per-provider randomness, slow-VDE-encoded). The sealing is provably slow and uniquely tied to that provider's identity. A SNARK over the sealed sector + commitments allows fast verification.
- **Sybil property:** Cryptographically Sybil-resistant — each Sybil identity would have to *separately* seal their own sectors, paying the full computational + storage cost.
- **Deployment:** Filecoin mainnet, October 2020 onwards; ongoing protocol revisions. See [`filecoin-post.md`](filecoin-post.md).

### Filecoin Proof of Spacetime (PoSt)

- **Mechanism:** After PoRep, the provider must periodically prove they still hold the sealed copy. The proof is challenge-response over the sealed sector; passing the challenge with high probability requires actually holding the data.
- **Sybil property:** Combined with PoRep, gives ongoing cryptographic Sybil-resistance for *storage* specifically.
- **Deployment:** Filecoin mainnet. See [`filecoin-post.md`](filecoin-post.md).

## Measurement / empirical-baseline references

### Free-riding on Gnutella

- **Paper:** Adar & Huberman, "Free Riding on Gnutella," ***First Monday* 5(10), October 2000.**
- **Result:** 24-hour Gnutella measurement (August 2000): ~70% of users shared *zero* files; ~50% of all query responses came from the top 1% of sharers; ~25% of users provided ~99% of files.
- **Significance:** Established the canonical empirical baseline for "free-riding is the default user behavior in permissionless P2P." Predates everything else in this corpus.
- See [`adar-huberman.md`](adar-huberman.md).

## Survey papers / SoKs

### Sybil defense survey (Yu 2011)

- **Paper:** Yu, "Sybil defenses via social networks: a tutorial and survey," **SIGACT News** 42(3), 2011.
- **Significance:** Earlier survey by the SybilGuard / SybilLimit lead author. Useful as a category map.

### SoK: Evolution of Sybil Defense via Social Networks (Alvisi 2013)

- **Paper:** Alvisi / Clement / Epasto / Lattanzi / Panconesi, "SoK: The Evolution of Sybil Defense via Social Networks," **IEEE S&P 2013**.
- **Significance:** The most useful single critique of the social-graph Sybil-defense category. Argues that *all* social-graph Sybil defenses (SybilGuard, SybilLimit, SybilInfer, SumUp, SybilRank, Gatekeeper) effectively perform *community detection* — they identify the well-connected honest region and reject everything else. A Sybil region that *is* a tight community defeats them all.
- **Implication for Myrhiza:** If the Willow invite graph has substructure (multiple disjoint social cliques), social-graph defenses will partition them unpredictably. Read this paper before committing to a social-graph approach.

## Sources

All citations under per-algorithm headings above; primary URLs listed in [`README.md`](README.md) Sources section. Per-paper deep-dive sources in each paper's own file.
