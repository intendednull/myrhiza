**Date:** 2026-05-22
**Status:** active
**Subject:** BitTorrent choking algorithm + PropShare — the deployed-at-scale reciprocity reference. Tit-for-tat works; it doesn't solve Sybil.

# BitTorrent choking and the reciprocity lineage

If any algorithm in this corpus has earned the right to be called "battle-tested," it's BitTorrent's **choking algorithm**. Bram Cohen's 2003 design has been continuously deployed for ~22 years across hundreds of millions of clients and many petabytes per day of data transfer. It is the most-deployed P2P reciprocity scheme by orders of magnitude.

It is also **not a Sybil defense**. Surfacing this clearly is the load-bearing job of this file. BitTorrent is Sybil-*tolerant* per-connection — a fresh Sybil starts at zero credit and has to earn its way up just like any newcomer — but it does not prevent Sybil attacks globally, and there are well-documented attacks (free-riding clients, BitTyrant) that exploit the per-connection model in aggregate.

## The Cohen 2003 paper

- **Citation:** Bram Cohen, "Incentives Build Robustness in BitTorrent," **Proceedings of the 1st Workshop on Economics of Peer-to-Peer Systems (P2PECON 2003)**, Berkeley CA, May 22 2003, pp. 68–72. [PDF (bittorrent.org)](https://bittorrent.org/bittorrentecon.pdf).
- **Author:** Bram Cohen, the original BitTorrent designer. The paper followed the protocol's first release (~2001) and described the incentive layer post-hoc.

The paper makes one design choice (choking) and analyzes its game-theoretic properties under three traffic patterns:

1. **Steady state.** All peers have similar download progress, want similar pieces, and have similar upload bandwidth. Tit-for-tat induces a Pareto-efficient resource allocation.
2. **Startup.** A new peer has nothing to trade. The *optimistic-unchoke* mechanism (below) allows new peers to bootstrap.
3. **Endgame.** A peer has nearly the full file. Standard tit-for-tat would cause peers to stop uploading once they have nothing left to want. The protocol handles this via *endgame mode* (peers continue uploading to peers they previously traded with).

## The choking algorithm

Each peer maintains **4 unchoked slots + 1 optimistic-unchoke slot** for outbound transfer:

- **The 4 unchoked slots** are filled by the 4 peers who uploaded to this peer the fastest in the recent past (default: last 10 seconds). Reassessed every 10 seconds.
- **The optimistic-unchoke slot** rotates randomly every 30 seconds to a peer not currently in the unchoked set. This is the bootstrap mechanism: even peers with no prior history get a chance to receive data, and the optimistic peer can then prove itself by uploading back.

A peer who is "**snubbed**" (an unchoked peer who hasn't received data from us in a minute) retaliates by stopping uploads to that peer until reciprocation resumes.

The combination produces a robust local equilibrium:

- *Free-riders* (peers who never upload) get optimistic-unchoke service only, which is a small fraction of total bandwidth and rotates away quickly. Net cost to honest peers is low.
- *Honest peers* discover mutually-fast partners via the optimistic-unchoke and form persistent reciprocal pairs.
- *Faster-than-mean peers* find each other (because they upload fast to each other → both stay unchoked) and form a high-bandwidth subset.

The whole machinery is fully decentralized — no tracker is in the loop for choking decisions. The tracker only provides peer-discovery.

## Game-theoretic properties

- **Pareto-efficient locally.** Each peer maximizes its own download rate subject to the bandwidth-budget constraint; the protocol's allocation is Pareto-optimal for the peers' local view.
- **Strategy: "be honest" is approximately optimal.** Lying about local state doesn't help much. Hoarding bandwidth without uploading gets you choked.
- **Not globally efficient.** Section 5 of the paper acknowledges that the choking-rank approach is heuristic; better allocations exist in theory. **PropShare (below) showed it's not even close to optimal.**

## PropShare: BitTorrent is an Auction (Levin et al. SIGCOMM 2008)

- **Citation:** Dave Levin, Katrina LaCurts, Neil Spring, Bobby Bhattacharjee, "**BitTorrent is an Auction: Analyzing and Improving BitTorrent's Incentives**," **SIGCOMM 2008**. [Project page](http://www.cs.umd.edu/projects/propshare/). [Paper](https://dl.acm.org/doi/10.1145/1402946.1402987).

The PropShare critique:

- BitTorrent's choking is a **rank-and-cut** allocation: "I serve the top 4 peers; nothing to peers ranked 5+." This wastes bandwidth that could go to slightly-lower-ranked peers as marginal contribution.
- Modeled as an *auction*, BitTorrent's allocation is strategy-mutable: a rational peer can game the rank-cutoff by carefully calibrating upload to specific peers.

The PropShare fix: **proportional-share allocation**. Each peer gets upload bandwidth in proportion to what they contributed. No rank cutoff; smooth allocation. The resulting auction is **strategy-proof** — lying about contributions cannot improve a peer's allocation.

Empirical results (PlanetLab + local cluster + live swarms):

- PropShare clients achieve faster downloads than vanilla BitTorrent.
- The fairness is *better* — high-upload peers get rewarded more reliably; the variance is lower.
- Resistance to **collusion** (groups of peers sharing knowledge to game choking) is meaningfully higher.
- Resistance to **Sybil attacks** is meaningfully higher because per-Sybil contribution must scale linearly with per-Sybil reward.

### Why PropShare never replaced choking

Bram Cohen's choking algorithm was already established and "working well enough." BitTorrent client developers had no commercial incentive to change. The closed-source BitTorrent Inc. clients stuck with choking; the open-source clients (libtorrent, Transmission, qBittorrent) followed suit. By the time PropShare's improvements were quantified, the deployed base was too large to migrate. **Path-dependence beat protocol-improvement.**

This is a generally-applicable lesson — see [`lessons.md`](lessons.md).

## BitTyrant: how to game BitTorrent

- **Citation:** Piatek / Isdal / Anderson / Krishnamurthy / Venkataramani, "Do incentives build robustness in BitTorrent?", **NSDI 2007**. UW.

BitTyrant is a *malicious* BitTorrent client demonstrating that the choking algorithm can be gamed:

- Identifies the *minimum* upload rate to each peer that keeps the peer optimistically unchoking BitTyrant.
- Allocates upload only at that minimum, freeing bandwidth for further downloads.
- Achieves ~70% better download speeds than vanilla BitTorrent at the cost of ~50% upload efficiency for the swarm.

This is **per-client attacks on the per-connection reciprocity**. Not Sybil-multiplication; just smarter strategy within the existing identity-set. The paper's takeaway: choking is incentive-*partial*, not incentive-*compatible*. PropShare's auction framing was the response — strategy-proof allocation by construction.

## What BitTorrent doesn't solve

- **Sybil attacks at scale.** A Sybil army of `k` identities, each acting honest enough to maintain reciprocal links to a small set of honest peers, can extract aggregate resources. The per-connection reciprocity caps the *rate* per attacker-identity, not the *aggregate* over many.
- **Cold-start fairness.** New peers depend on optimistic-unchoke charity to bootstrap. In a swarm with too many newcomers and too few experienced peers, swarms fail.
- **Asymmetric workloads.** If most peers want one specific block, and a Sybil army supplies it, the army extracts useful work without ever deeply reciprocating.
- **Long-running selfishness.** A peer can build a long-running history of "just-enough" cooperation, never being penalized, while contributing far less than its capacity.
- **Whitewashing.** Drop an identity, start fresh. The cost is just the relationship-rebuilding time. BitTorrent assumes peers don't care about identity persistence; this is true for casual users but wrong for clients optimizing for long-term game.

## Private trackers and the layered defense

The dominant "production" answer to BitTorrent's Sybil and free-rider problems has been **private trackers**: closed-membership BitTorrent communities where:

- Identity is invitation-gated. Each user has a stable username.
- The tracker maintains an upload/download ratio per user. Below-threshold users are kicked.
- Cheating clients are detected by tracker-side measurement.

This is a **layered defense** — Sybil resistance via admission control (category 6), in-network behavior measured by the tracker (centralized accounting), per-connection choking still does its job. The combination is robust *because* it doesn't rely on the choking algorithm alone for Sybil resistance.

The lesson generalizes: **deployed-at-scale P2P reciprocity always pairs with an admission-control layer**. The pure reciprocity-only approach (public BitTorrent swarms) tolerates the free-rider tax as a cost of permissionlessness.

## Implications for Myrhiza

1. **Adopt choking-style reciprocity for one-on-one transactions.** Where Myrhiza peers need to exchange bytes (state-apply log sync, snapshot fetch, blob retrieval), per-connection reciprocity is the deployed-at-scale answer. Don't overdesign — Cohen's 4-unchoked + 1-optimistic pattern is good enough.
2. **PropShare's proportional-share is the better default if implementation cost is comparable.** Strategy-proof allocation matters for any client that might be modified or replaced. Modern P2P clients (libtorrent et al.) have gradually shifted toward proportional-share variants — Myrhiza should start there, not at Cohen's 2003 baseline.
3. **The optimistic-unchoke slot is the bootstrap mechanism.** Without it, new peers can't join. A Myrhiza maintenance-enforcement scheme must include an analogous "give newcomers a chance to prove themselves" mechanism — refusing service to all non-participants from the start makes the network unjoinable.
4. **Per-connection reciprocity is one layer; do not skip the admission layer.** Production-grade P2P reciprocity always pairs with admission control. Myrhiza's invite graph is the natural admission layer. Use both; don't pretend choking alone solves Sybil.
5. **Path-dependence matters.** The PropShare lesson — better algorithm, never deployed because the existing one was good-enough — is a warning. Pick a reciprocity algorithm before launch; switching is much harder than picking right.
6. **Snubbing semantics translate cleanly to maintenance refusal.** Cohen's snubbing pattern ("if you stop uploading to me, I stop uploading to you, until reciprocation resumes") is exactly the shape of "if you stop running maintenance components, I refuse to serve your reads, until participation resumes." Same primitive, different unit-of-account.
7. **Endgame mode is real.** Late-stage transfers under tit-for-tat without an endgame-fallback are slow. Myrhiza needs a story for "this peer has all the snapshots they need; what should they do?" — almost certainly *continue serving others until the swarm completes*.

## Sources

- [Cohen, "Incentives Build Robustness in BitTorrent," P2PECON 2003 (bittorrent.org PDF)](https://bittorrent.org/bittorrentecon.pdf).
- [Cohen, "Incentives Build Robustness in BitTorrent" (alternative mirror)](https://www.adrian.idv.hk/2008-11-18-cohen-bt/).
- [Levin / LaCurts / Spring / Bhattacharjee, "BitTorrent is an Auction: Analyzing and Improving BitTorrent's Incentives," SIGCOMM 2008](https://dl.acm.org/doi/10.1145/1402946.1402987) — DOI [10.1145/1402946.1402987](https://dl.acm.org/doi/10.1145/1402946.1402987).
- [PropShare project page at UMD](http://www.cs.umd.edu/projects/propshare/).
- [Piatek et al., "Do incentives build robustness in BitTorrent?" (BitTyrant paper), NSDI 2007](https://www.usenix.org/legacy/event/nsdi07/tech/full_papers/piatek/piatek.pdf).
- [BitTorrent.org protocol specification](https://www.bittorrent.org/beps/bep_0003.html) — the wire-level reference.
- Cross-references: [`taxonomy.md`](taxonomy.md) §1, [`ipfs-bitswap.md`](ipfs-bitswap.md), [Tribler BarterCast in `algorithms.md`](algorithms.md), [`lessons.md`](lessons.md), `prior-art/pears/hyperswarm.md` (a different deployed P2P reciprocity model).
