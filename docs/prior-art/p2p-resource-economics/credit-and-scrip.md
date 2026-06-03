**Date:** 2026-05-29
**Status:** active
**Subject:** Global-scrip cautionary foils (Karma, Maze, Dandelion, MojoNation/Mnet) + single-resource fairness points (FairTorrent, Tahoe-LAFS) — why fungible global scrip invites the collusion/hoarding that per-peer vectors escape.

# Credit and scrip — the cautionary foils

The systems here are the **negative space** of the folder. Each tried to give P2P resource sharing a *global, fungible* unit of account — a scrip currency — and each ran into a failure mode that a **per-peer, non-fungible** value vector structurally avoids. They are catalogued so a Myrhiza spec author can point at the specific attacks fungibility invites, rather than re-discovering them.

## Why global fungible scrip is the trap

A per-peer reciprocity ledger (OurGrid, GNUnet, Samsara, and the leading Myrhiza model) records *bilateral* history: what B did for A lives only in A's books and means nothing to C. A **global scrip** instead mints a transferable token that is worth the same to everyone. That single design choice opens three attacks at once:

- **Collusion / wash-trading.** Two or more identities trade useless work in a loop to mint scrip, then spend it on real resources elsewhere. Because the token is fungible and globally honored, the minting site and the spending site need not be the same victim — so no single peer ever sees the imbalance. A per-peer ledger denies this: useless work between colluders only inflates *their own* mutual balance, which buys nothing from a third party.
- **Hoarding / inflation.** A global money supply must be managed. Too much and it inflates (early contributions become worthless); too little and newcomers can never bootstrap. Someone must set monetary policy — which reintroduces a trusted authority.
- **A settlement authority creeps back in.** To stop double-spending of a transferable token you need consensus on balances — a bank, a quorum, or a blockchain. The per-peer model needs none because nothing transfers.

## Karma (P2PEcon 2003) — scrip with a bank-set quorum

Vishnumurthy, **Chandrakumar**, and Sirer (Cornell), "KARMA: A Secure Economic Framework for Peer-to-Peer Resource Sharing." (Note: the seed citation's "Chakravarty" is wrong — the middle author is **Sangeeth Chandrakumar**; corrected against the PDF.)

- Each peer has a **single global scalar**, its *karma*, representing its standing in the whole system. Karma goes up when it contributes, down when it consumes.
- Because the balance is global and transferable, Karma needs a **bank-set**: a quorum of nodes that tracks each peer's karma and certifies transfers atomically, tolerating a fraction of malicious bank-set members. It also applies **periodic inflation/deflation corrections** to the total karma in circulation — i.e. explicit monetary policy.
- This is the canonical illustration of the trap: to make a global currency safe, Karma reintroduces a distributed bank and a central-bank-style supply correction. Myrhiza's "no global token, ever" lock is a direct rejection of this architecture.

## Maze — measured collusion and whitewashing in the wild

Maze was a large deployed (China-based academic) P2P file-sharing system with a points-based incentive scheme — a soft scrip. Two measurement studies make it valuable evidence:

- Free-riding study (2005): even with incentive points, selfish behavior persisted (correlated with short online time, not with bandwidth or NAT).
- Collusion study, Lian et al., "An Empirical Study of Collusion Behavior in the Maze P2P File-Sharing System" (ICDCS 2007): with a **non-net-zero** point policy, users colluded in patterns resembling web-spam link farms — repeated fake transfers to pump points — and **whitewashed** accounts (abandon a low-point identity, start fresh) as a Sybil variant. The authors propose an *upload-entropy* defense. Maze is the empirical receipt for "fungible-ish scrip gets wash-traded at scale."

## Dandelion (USENIX ATC 2007) — robust incentives, but server-mediated

Sirivianos, Park, Yang, Jarecki (UC Irvine), "Dandelion: Cooperative Content Distribution with Robust Incentives."

- Dandelion gives **provably non-manipulable** virtual-currency credit for uploads via *strict fair exchange*: a client cannot get content without paying credit, nor earn credit for uploads it did not perform.
- The catch, and the reason it is a foil rather than a model: it achieves robustness with a **central server** (the content provider) that issues and clears credit. It buys non-manipulability with centralization — exactly the trade Myrhiza will not make. It is the proof that *if* you accept a trusted clearer, scrip can be made sound; the whole point of the per-peer model is to avoid needing one.

## MojoNation / Mnet — digital-cash-for-resources, abandoned

MojoNation paid users a micropayment currency ("Mojo") for contributing bandwidth and storage; it was an early-2000s commercial attempt at a literal resource market. It **failed and was abandoned** — the company ran out of money in early 2002, and the codebase became the noncommercial Mnet (Mojo's anti-inflation story was inadequate and the currency hyperinflated). Two of its alumni are load-bearing for the wider lineage: Bram Cohen left to build BitTorrent (pure per-connection reciprocity, no currency — see [`prior-art/sybil-resistance/bittorrent.md`](../sybil-resistance/bittorrent.md)) and Zooko Wilcox-O'Hearn went on to Mnet and later Tahoe-LAFS. The lesson the *successors* drew: drop the global currency. (Some specifics — exact dates, the inflation mechanism — are from secondary sources; treat as directionally correct, details unverified.)

## Single-resource fairness points (not foils — useful references)

Two token-free systems sit alongside the foils as positive single-resource fairness references:

- **FairTorrent** (Sherman, Nieh, Stein, Columbia; CoNEXT 2009): a **deficit-based** distributed algorithm. Each peer keeps a deficit counter per peer = bytes uploaded − bytes downloaded, and always serves the peer it owes most. No currency, no central control, no rate prediction, resilient to free-riders and strategic peers. It is the cleanest *single-resource* (bytes) instance of "serve whom you owe most" — the same shape as the brainstorm's net-imbalance standing curve, restricted to one resource.
- **Tahoe-LAFS friendnet quota**: in a "friendnet" (a group of friends sharing storage with no central admin or payment), each storage server tracks *per-account* usage and can refuse new leases once an account exceeds quota. Token-free, per-relationship, refusal-as-enforcement — a deployed instance of exactly the enforcement primitive Myrhiza's kernel would provide, in the single resource of storage. (Tahoe-LAFS is by Zooko et al. — the MojoNation lineage that *learned* to drop the currency.)

## Implications for Myrhiza (framing-disclosed — see [`README.md`](README.md))

1. **"No global token, ever" is empirically well-founded.** Karma needs a bank-set, Maze got wash-traded, Dandelion needs a central server, MojoNation hyperinflated. The lock in the brainstorm is supported by four independent failure modes, not just taste. See [`lessons.md`](lessons.md) Avoid.
2. **Non-fungibility is the collusion defense.** Per-peer, non-transferable balances mean colluders can only inflate their *mutual* books, which buys nothing externally. State this as the structural reason Myrhiza escapes the Maze attack.
3. **Deficit-counter / serve-whom-you-owe-most is the deployed primitive** (FairTorrent) — borrow the shape; the leading model generalizes it from bytes to a resource vector.
4. **Refusal-as-enforcement is deployed in friendnets** (Tahoe-LAFS) — Myrhiza's kernel-mediated refusal is the same primitive made first-class. See [`reports/2026-05-29-reciprocity-economy-brainstorm/README.md`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) Open fork #6.

## Sources

- [Vishnumurthy / Chandrakumar / Sirer, "KARMA: A Secure Economic Framework for Peer-to-Peer Resource Sharing," P2PEcon 2003](https://www.cs.cornell.edu/people/egs/papers/karma.pdf) — verified against PDF: global scalar karma, bank-set quorum, periodic inflation/deflation corrections, atomic transfers. **Author name corrected: Chandrakumar, not Chakravarty.**
- [Lian et al., "An Empirical Study of Collusion Behavior in the Maze P2P File-Sharing System," ICDCS 2007](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/icdcs07-1.pdf) — collusion patterns, non-net-zero points, upload-entropy defense.
- [Yang et al., "An Empirical Study of Free-Riding Behavior in the Maze P2P File-Sharing System," IPTPS 2005](https://link.springer.com/chapter/10.1007/11558989_17) — free-riding measurement.
- [Sirivianos / Park / Yang / Jarecki, "Dandelion: Cooperative Content Distribution with Robust Incentives," USENIX ATC 2007](https://www.usenix.org/legacy/event/usenix07/tech/full_papers/sirivianos/sirivianos.pdf) — verified against PDF: server-issued virtual currency, strict fair exchange, provably non-manipulable.
- [Mnet / MojoNation overview (HandWiki)](https://handwiki.org/wiki/Software:Mnet_(peer-to-peer_network)) and [The Mojo Nation Story](https://www.financialcryptography.com/mt/archives/000572.html) — history, abandonment, inflation (secondary; details unverified).
- [Sherman / Nieh / Stein, "FairTorrent: Bringing Fairness to Peer-to-Peer Systems," CoNEXT 2009 (Rome)](http://conferences.sigcomm.org/co-next/2009/papers/Sherman.pdf) — deficit-counter algorithm; Sherman & Nieh (Columbia), Stein. Venue/authors verified.
- [Tahoe-LAFS QuotaManagement / AccountingDesign wiki](https://tahoe-lafs.org/trac/tahoe-lafs/wiki/QuotaManagement) — friendnet per-account quota and lease refusal.
- Cross-references: [`README.md`](README.md), [`lessons.md`](lessons.md), [`prior-art/sybil-resistance/bittorrent.md`](../sybil-resistance/bittorrent.md), [`prior-art/sybil-resistance/ipfs-bitswap.md`](../sybil-resistance/ipfs-bitswap.md), [`reports/2026-05-29-reciprocity-economy-brainstorm/README.md`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md).
