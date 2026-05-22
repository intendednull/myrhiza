**Date:** 2026-05-22
**Status:** active
**Subject:** Canonical taxonomy — the design space of Sybil-resistance and free-riding defense

# Taxonomy

Every approach in this corpus falls into one (or a small composition) of six categories. The categories are not a partition of *the universe of possible defenses* — Sybil resistance is open-ended — but they are a useful partition of *what people have actually tried and published on*. Use this file as the orientation map before diving into per-paper files.

## The trilemma

The Holochain folder (`prior-art/holochain/open-problems.md#2-free-rider-participation`) frames the trade-off Sybil work is fundamentally bounded by:

- **Sybil-resistance** — one attacker cannot trivially appear as many peers.
- **Permissionlessness** — anyone can join without a gatekeeper.
- **Cost-freeness** — joining and participating are economically free.

You can pick at most two. Pick (resistance + permissionless) and you must impose a cost — PoW, PoS, proof-of-personhood ceremony. Pick (resistance + free) and you must have a gatekeeper — an invite graph, an allow-list, a trust authority. Pick (permissionless + free) and you have Gnutella circa 2000 — open, free, and Sybil-collapsing.

Myrhiza's permission/invite graph is a *gatekeeper-style* primitive — Willow takes the (resistance + free, sacrificing permissionlessness) corner of the trilemma. This is by design and is the structural advantage [`lessons.md` §"Validates" #1 + `taxonomy.md` §"Where Myrhiza sits"](lessons.md) builds on.

## Six approach categories

### 1. Tit-for-tat / pairwise reciprocity

**What it does.** Per-pair accounting: I serve you in proportion to how much you've served me. No global view; no reputation gossip. Each peer maintains its own ledger of bilateral interactions.

**Sybil property.** *Sybil-tolerant per-connection, not Sybil-resistant globally.* Creating new identities doesn't help the attacker — a fresh Sybil starts with zero credit and has to earn its way up the same as any newcomer. But many Sybils acting in parallel *can* extract aggregate resources from a sparsely-connected swarm, because each individual connection is below the threshold where the honest peer would notice.

**Cost.** None to participate honestly; impossible to defect persistently without local detection.

**Canonical references.** BitTorrent choking (Cohen 2003), PropShare (Levin et al. 2008), IPFS Bitswap ledger, Hyperswarm reciprocity. See [`bittorrent.md`](bittorrent.md), [`ipfs-bitswap.md`](ipfs-bitswap.md).

**Strength.** Deployed at scale for ~25 years. Empirically robust against routine free-riders and Sybil-flood attacks because the cost of attacking is paid in time-to-establish each connection.

**Weakness.** Asymmetric workloads break it. If most peers want one specific block, and a Sybil-army provides that block once each, the army extracts useful work without ever needing to reciprocate at depth. Also: cold-start (new honest peers can't bootstrap without altruism) and unequal-bandwidth (a peer with no upload capacity is indistinguishable from a free-rider).

### 2. Reputation systems / global trust scores

**What it does.** Each peer has a scalar reputation, computed (and gossiped) from observed interaction outcomes. Peers preferentially serve high-reputation peers. Convergence is typically a power iteration over a peer-by-peer trust matrix.

**Sybil property.** *Fragile under Sybil.* The whole graph can be tampered with by Sybil identities reporting friendly opinions of each other. EigenTrust's normalization helps; the cited critique (Alvisi et al. 2013) shows it does not help enough.

**Cost.** Gossip overhead. Convergence latency (~minutes to hours). State drift across peers (each peer's score is approximate).

**Canonical references.** EigenTrust (Kamvar et al. 2003), PowerTrust, PeerTrust, BarterCast. See [`eigentrust.md`](eigentrust.md), [Tribler BarterCast in `algorithms.md`](algorithms.md).

**Strength.** Reputation captures long-running history that pairwise reciprocity discards. Lets a honest newcomer reach a useful subset of high-trust peers quickly.

**Weakness.** Sybil-collusion, score-pumping, whitewashing (drop low-score identity, start fresh), and the fundamental problem that *reputation systems aggregate reports from peers who themselves may be malicious*. The "reputation systems are fragile" critique is real — surface it honestly in lessons.md.

### 3. Social-graph Sybil defenses

**What it does.** Use the social graph (human-attested trust edges between identities) as an input to a Sybil-detection algorithm. The defining assumption: it is hard for an attacker to obtain many edges into the honest region of the graph, even though they can grow the Sybil region arbitrarily. The mathematical bound is **attack edges** — the number of edges crossing from honest to Sybil — vs total edges.

**Sybil property.** *Resists Sybil up to a per-honest-node attack-edge budget.* SybilLimit tolerates O(√n log n) attack edges across a graph of n honest nodes; Whanau tolerates O(n/log n). Beyond that, the defenses degrade.

**Cost.** O(√n log n) state per node (SybilLimit) or O(√n log n) routing-table entries (Whanau); both are sub-linear, which is the point. Mixing-time assumptions (the graph must be *fast-mixing*, conductance ≥ Ω(1/log n)).

**Canonical references.** SybilGuard (Yu et al. 2006), SybilLimit (Yu et al. 2008), Whanau (Lesniewski-Laas & Kaashoek 2010), SybilInfer, SumUp. See [`sybilguard-sybillimit.md`](sybilguard-sybillimit.md), [`whanau.md`](whanau.md).

**Strength.** *The only category that resists Sybil without requiring proof-of-work, proof-of-personhood, or a central authority.* Exactly the shape Willow's invite graph could feed into.

**Weakness.** (a) Assumes the social graph is fast-mixing — many real social graphs are not, particularly small / new / clique-y ones. (b) Targeted attack-edge cultivation — if an attacker can get even ~tens of trust edges into the honest region (in real social networks, this is not hard via social engineering), the defenses break. (c) The honest region must be one connected fast-mixing component; partitions defeat the algorithm. (d) The Alvisi 2013 SoK survey concluded that *all* social-graph Sybil defenses essentially detect *community structure*, not Sybil specifically — Sybils that look like a tight community pass.

### 4. DHT-responsibility / sharded validation

**What it does.** Every peer is *responsible* for validating a specific shard of the data. A Sybil is no help if it doesn't actually run the validation work — and the validation work is verifiable by other peers in the same shard. Closely tied to gossip-validation patterns and "every node holds and validates its slice" architectures.

**Sybil property.** *Sybils get assigned to shards by hash of identity; an attacker can pre-compute Sybil identities to target a specific shard (a "shard-targeting" attack) but this is detectable as anomalous concentration.* Not full resistance; a shrunk form of it.

**Cost.** Per-shard storage + validation overhead per peer. Cannot bootstrap before peer count is large enough for sharding to be meaningful.

**Canonical references.** Holochain's validation-receipt + warrant system (cross-ref: `prior-art/holochain/determinism.md` validation receipts, `prior-art/holochain/identity.md` warrants). Closest neighbor in this corpus rather than a standalone paper. See also Filecoin sealed-sector commitments as a heavier variant.

**Strength.** Cleanly composes with Myrhiza's per-event-validation model. Maps naturally onto the maintenance-component-as-fourth-profile framing.

**Weakness.** Sharding completion is hard — Holochain's open problem #5 (`prior-art/holochain/open-problems.md`) is "sharding has been 6+ years away." Partial-arc topology is research-grade. And the validation work being checkable doesn't directly answer "did the peer also store the data" — that's a different verification.

### 5. Cryptographic proofs (PoRep, PoSt, PoW, PoS)

**What it does.** Make the contribution *cryptographically verifiable*. Filecoin's PoRep proves a peer sealed a unique copy of a dataset; PoSt proves they're still storing it. PoW makes computation the rate-limit; PoS uses stake. All four collapse Sybil because they make the gate the cost-resource, not identity.

**Sybil property.** *Cryptographically Sybil-resistant by construction.* A million Sybils with no stake have no influence.

**Cost.** Significant — heavyweight cryptography, settlement-layer dependency, real economic cost imposed on every participant. A bad fit for a free-to-use consumer-mobile P2P app.

**Canonical references.** Filecoin PoRep + PoSt (Benet & Dalrymple, 2017 onwards). See [Filecoin PoRep/PoSt in `algorithms.md`](algorithms.md). Bitcoin / Ethereum PoW / PoS are out of scope for this corpus but are the same shape mathematically.

**Strength.** *The only category with formal Sybil-resistance proofs.*

**Weakness.** All require a settlement layer (a blockchain or a centralized verifier). All impose a real economic cost on participants. None fit Myrhiza's consumer-mobile P2P-runtime profile without significant adaptation.

### 6. Identity gating / proof-of-personhood / out-of-band attestation

**What it does.** A trusted authority (or a federation, or a ceremony) attests "this human is unique." Worldcoin (iris-scan), Proof-of-Humanity (video + endorsement chain), CAPTCHAs, government ID, phone-number verification. Often pluggable as a one-time gate at join time.

**Sybil property.** *Strong if the attestor is uncompromised and the binding from human to key is sound.* Weak if either fails.

**Cost.** Real-world friction. Privacy cost. Coverage gaps (people without phones / IDs / iris-scan-eligible irises).

**Canonical references.** Outside this corpus's deep-dive scope; mentioned in [`open-problems.md`](open-problems.md) for completeness. Holochain's membrane proofs (`prior-art/holochain/`) are a generic infrastructure for plugging this category in app-by-app.

**Strength.** Composes with everything else. Highest absolute Sybil resistance available.

**Weakness.** Requires a trusted attestor, contradicting decentralization. Privacy cost is real. Operational fragility (lost ID = lost participation).

## Composition

The dominant pattern in deployed systems is **layered composition**: one category for newcomer admission, a second for ongoing accountability, a third for misbehavior detection. Example layerings:

- **BitTorrent in the wild.** Choking (category 1) + private-tracker membership lists (category 6 lite) + community reputation forums (category 2 informal).
- **Holochain DHT.** Membrane proofs (category 6) at admission + DHT-responsibility validation (category 4) at runtime + warrants (category 2 lite, evidence-based) for misbehavior.
- **Filecoin storage market.** PoRep + PoSt (category 5) at the contract level + market-rate reputation among storage providers (category 2 informal).
- **Tor relay network.** Centralized directory + flag-assignment based on observed uptime (category 6 + category 2 hybrid) + relay-flag-as-trust-token.

**The corpus's lesson for Myrhiza.** No single category is sufficient. The folder's per-algorithm files describe individual primitives; [`lessons.md`](lessons.md) describes the composition Myrhiza is likely to need.

## Where Myrhiza sits

Willow's permission/invite graph is the **category-3 input** (social-graph defense) we already have for free. The maintenance-component-as-fourth-profile model is a **category-4 setup** (DHT-responsibility). Combined, the natural composition is:

1. **Admission:** invite graph as the social-graph input → SybilLimit-style honest-region verification.
2. **Ongoing accountability:** maintenance-component-as-declared-capacity per peer; refusal-to-serve-non-participants as the enforcement primitive.
3. **Misbehavior:** Holochain-warrant-style signed evidence of detected misbehavior, gossip-distributed.

This composition is developed in [`lessons.md` §"Validates" #1 + `taxonomy.md` §"Where Myrhiza sits"](lessons.md). The component pieces have research-grade or deployment-grade prior art; the composition is novel and unproven. Be honest about both.

## Sources

- [Alvisi / Clement / Epasto / Lattanzi / Panconesi, "SoK: The Evolution of Sybil Defense via Social Networks," IEEE S&P 2013](https://oaklandsok.github.io/papers/alvisi2013.pdf) — the most useful single survey of the social-graph category; explicitly states the "community detection ≠ Sybil detection" limitation.
- `prior-art/holochain/open-problems.md` §1–2 — the trilemma framing.
- `prior-art/willow/open-problems.md` §12–69 — the Myrhiza-side statement of the problem.
- All per-paper files in this folder for category-specific sources.
