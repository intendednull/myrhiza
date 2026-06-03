**Date:** 2026-05-22
**Status:** active
**Subject:** The decision file — what Sybil-resistance and distributed-maintenance prior art validates, what to avoid, what to borrow. Read this when designing Myrhiza's participation-enforcement and maintenance-incentive specs.

# Lessons for Myrhiza — Sybil resistance + distributed maintenance

Synthesis across [`bar-gossip.md`](bar-gossip.md), [`eigentrust.md`](eigentrust.md), [`sybilguard-sybillimit.md`](sybilguard-sybillimit.md), [`whanau.md`](whanau.md), [`bittorrent.md`](bittorrent.md), [`ipfs-bitswap.md`](ipfs-bitswap.md), [`self-reported-cost-verification.md`](self-reported-cost-verification.md), [`algorithms.md`](algorithms.md), [`taxonomy.md`](taxonomy.md). Format: validates / avoid / borrow.

## Validates

1. **The permission/invite trust graph is Myrhiza's structural advantage.** Most P2P systems must bootstrap a social graph (BitTorrent has none; Bitswap has none; pure DHTs have none). Myrhiza inherits one for free from its capability model — every cap-grant is an edge. SybilGuard / SybilLimit assume sparse attack edges between honest social graph and Sybil region; Myrhiza's permission graph satisfies this by construction (only invited peers participate). *Source: [`sybilguard-sybillimit.md`](sybilguard-sybillimit.md), [`prior-art/willow/open-problems.md:12-69`](../willow/open-problems.md).*

2. **Reciprocity beats reputation in practice.** BitTorrent's choking algorithm (Cohen 2003) — pairwise tit-for-tat with optimistic unchoke — is the most successful deployed P2P incentive scheme in 25 years. EigenTrust-style global reputation has zero deployments at comparable scale. The lesson: per-connection, locally-observable, immediate-feedback schemes work; global, trust-aggregated, asynchronous schemes don't. *Source: [`bittorrent.md`](bittorrent.md), [`eigentrust.md`](eigentrust.md).*

3. **Sybil-defense is research-grade; reciprocity is deployed.** All Sybil-defense literature (SybilGuard, SybilLimit, Whanau, EigenTrust) is research-grade. Every deployed answer (BitTorrent choking, Bitswap, BarterCast) is *reciprocity*, not Sybil defense. The lesson: don't bet Myrhiza v1 on un-deployed Sybil-defense schemes. Use deployed reciprocity primitives + the permission graph as the Sybil signal. *Source: [`taxonomy.md`](taxonomy.md).*

4. **Maintenance as a fourth component profile (state-apply / state-propose / interaction / behavior + maintenance) is structurally sound.** Holochain treats maintenance (DHT responsibility validation) as kernel-mediated, not app-mediated. The Willow PR #636 proposal extends this — maintenance components run with declared capacity and a refusal-to-serve-non-participants primitive. This is consistent with the deployed-reciprocity lesson: enforcement happens *locally*, per-connection, immediate. *Source: [`prior-art/willow/open-problems.md:12-69`](../willow/open-problems.md), [`prior-art/holochain/lessons.md`](../holochain/lessons.md).*

5. **Free-riding is the dominant historical failure mode.** Adar & Huberman (2000) found ~70% of Gnutella users contributed nothing. Every subsequent P2P system has had to address this. Designing Myrhiza assuming honest peers (no enforcement) replays the Gnutella failure. The lesson: ship enforcement primitives from v1, even if simple (BitTorrent-shaped). *Source: [`bittorrent.md`](bittorrent.md), [`ipfs-bitswap.md`](ipfs-bitswap.md).*

6. **Self-reported cost is only trustworthy after re-execution, outlier-rejection, and bounding against history.** BOINC's twenty-year credit arms race converged on exactly this: replication + quorum cross-check ("top and bottom claimed credits dropped, average the rest"), claimed-vs-granted separation, and an anomaly cap (~10× the running average — the wiki's own hedged figure). For Myrhiza this validates re-running a candidate event through the existing `state-apply` dry-run as a verification quorum, and bounding any single self-favorable claim against the relationship's running standing. *Source: [`self-reported-cost-verification.md`](self-reported-cost-verification.md).*

7. **"Normalize hardware variance out" vs "keep it in" is an objective choice, not an error.** BOINC's cobblestone deliberately *cancels* hardware differences so the same science earns the same credit on any box — the right call for a fair scientific scoreboard. The reciprocity model deliberately *keeps* hardware variance *in* (a peer's own scarcity is the cost signal that produces gains from trade). Cite BOINC as the canonical "normalize-out" pole so the "keep-in" decision is made consciously, not by accident. *Source: [`self-reported-cost-verification.md`](self-reported-cost-verification.md), [`reports/2026-05-29-reciprocity-economy-brainstorm/`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md).*

## Avoid

| Pitfall | Source | Mitigation |
|---|---|---|
| **Self-reported participation as enforcement.** A custom client that says "I do maintenance" while doing nothing free-rides. PR #636 explicitly calls this out. | [`prior-art/willow/open-problems.md`](../willow/open-problems.md) | Enforcement primitive is refusal-to-serve-non-participants, observed at connection time. Don't trust self-reported metrics. |
| **EigenTrust-style global reputation as v1.** Pre-trusted peers + transitive trust + gossip aggregation = a complex distributed computation that has zero production deployments at P2P scale. Research-grade only. | [`eigentrust.md`](eigentrust.md) | Defer reputation aggregation to v2+. Use per-connection reciprocity in v1. |
| **Pure Sybil-defense without reciprocity.** SybilGuard/SybilLimit can detect Sybil regions but don't enforce participation. A Sybil-defended network where 100% of peers are honest still has free-riders. | [`sybilguard-sybillimit.md`](sybilguard-sybillimit.md) | Sybil-defense + reciprocity together. The first prevents identity-multiplication attacks; the second enforces work-for-work. |
| **Whanau / Sybil-proof DHT as v1.** Whanau (Lesniewski-Laas 2010) is theoretically clean but has no production deployments. Building Myrhiza's transport DHT on Whanau means inheriting research-grade-only infrastructure. | [`whanau.md`](whanau.md) | iroh is the transport (not Whanau). If a DHT is later needed, evaluate Whanau alongside Kademlia + Mainline. |
| **Filecoin proofs of replication / spacetime for general maintenance enforcement.** Heavy machinery (zk-SNARK proofs, blockchain-bound). Reasonable for storage-providers-paid-with-tokens; absurd for a peer running a chat-room maintenance component. | [`prior-art/willow/open-problems.md`](../willow/open-problems.md) | Use lightweight audit-style challenge-response over stored data. Reserve PoSt for the rare case where strong durability guarantees are wanted. |
| **Reputation systems are fragile.** Reputation can be gamed: collusion attacks, whitewashing (new identity for low-rep peer), front-running. Every reputation system has known attacks. | [`eigentrust.md`](eigentrust.md), [`taxonomy.md`](taxonomy.md) | Treat reputation as advisory, not authoritative. Authority decisions (cap-grants, kicks) belong to app-state-apply, not to a reputation aggregator. |
| **Token-incentivized maintenance.** Filecoin and similar token-economies pay peers for work. Myrhiza is a peer-runtime, not a market. Adding a token shifts the project's center of gravity to investor incentives and introduces speculation-as-a-feature. | [`taxonomy.md`](taxonomy.md) | Don't add a token. Use intrinsic-motivation maintenance (peers run maintenance because they want their app to work) + reciprocity enforcement (peers who don't help don't get served). |
| **Assuming the permission graph cannot be attacked.** SybilGuard's "sparse attack edges" assumption can be violated if the attacker compromises a high-trust peer. The permission graph is a strong signal *while* the graph is honest; recovery from a compromise is a separate problem. | [`sybilguard-sybillimit.md`](sybilguard-sybillimit.md) | Spec authors should document the permission-graph trust model explicitly. Account for compromise scenarios. Cross-ref MLS for analogous group-key compromise (PCS). |
| **A spendable cost claim whose signature sits *outside* the hashed/committed event.** Gridcoin's DPOR paid out real token value on a claim whose signature "is not part of the Merkle tree" — so anyone could copy a stranger's public BOINC credentials and redirect the payout, minting 72.4 GRC for zero work. *(Figures verified against the discoverers' RUB writeup.)* | [`self-reported-cost-verification.md`](self-reported-cost-verification.md) | Sign any cost claim that becomes standing *inside* the hashed event, bound to the producing peer; never value standing off unbounded, replay-able external history. |

## Borrow

1. **BitTorrent choking algorithm shape.** Pairwise reciprocity: each peer maintains the set of peers it's currently uploading to, periodically re-evaluates based on observed download rates, and runs optimistic unchoke to discover new peers. Myrhiza's maintenance-component reciprocity primitive can mirror this directly. *See [`bittorrent.md`](bittorrent.md).*

2. **Bitswap's want-list + ledger.** IPFS Bitswap tracks per-peer "blocks they have served me" vs "blocks I have served them" — a local ledger, not a global one. Decisions to serve are local. Myrhiza's per-peer reciprocity tracking should follow this shape. *See [`ipfs-bitswap.md`](ipfs-bitswap.md).*

3. **Permission graph as SybilGuard input.** Run SybilGuard-style random walks over the Myrhiza permission graph to identify the honest region. Peers outside the honest region (Sybil-suspect) get throttled or refused. This is the load-bearing use of Myrhiza's structural advantage. *See [`sybilguard-sybillimit.md`](sybilguard-sybillimit.md), [`prior-art/willow/open-problems.md`](../willow/open-problems.md).*

4. **BAR Gossip's three-party model (Byzantine + Altruistic + Rational).** Even if Myrhiza doesn't implement BAR Gossip directly, the *model* — explicitly distinguishing malicious from rational free-rider from honest contributor — is the right frame for participation specs. *See [`bar-gossip.md`](bar-gossip.md).*

5. **Tribler's BarterCast as deployable-reputation prior art.** Tribler's local-view BarterCast (each peer maintains its own ledger of others' contributions, gossips slowly) is the closest deployed approximation of "reputation that actually works." Not load-bearing for v1, but the right reference if Myrhiza later wants reputation aggregation. *See [`algorithms.md`](algorithms.md).*

6. **Holochain's validator-selection / DHT-responsibility model.** Every peer validates entries it is "responsible for"; non-validation is observable and refusable. The closest existing-system precedent for kernel-mediated maintenance. *See [`prior-art/holochain/lessons.md`](../holochain/lessons.md).*

## The single most important lesson

**Reciprocity beats reputation; reputation is not Sybil-defense; Sybil-defense without reciprocity does not enforce participation.** A Myrhiza v1 with (a) per-connection reciprocity primitives (BitTorrent-shaped), (b) permission-graph-as-Sybil-input (SybilGuard-shaped), and (c) maintenance-as-fourth-profile (Willow-shaped) covers the three legs of the problem with deployed-or-deployable prior art. Aggregated reputation (EigenTrust-shaped) is a v2+ option; token-incentives are out of scope.

## Cross-references

- [`README.md`](README.md), [`taxonomy.md`](taxonomy.md), [`algorithms.md`](algorithms.md)
- Per-paper evidence files
- [`prior-art/willow/open-problems.md`](../willow/open-problems.md) §"Distributed maintenance + Sybil-resistant participation"
- [`prior-art/holochain/`](../holochain/), [`prior-art/iroh/`](../iroh/), [`prior-art/pears/`](../pears/)
- [`prior-art/mls/`](../mls/) (PCS analog for graph compromise)

## Sources

All sources in per-paper evidence files.
