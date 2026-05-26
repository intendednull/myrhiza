**Date:** 2026-05-22
**Status:** active
**Subject:** IPFS Bitswap — ledger-based reciprocity for block exchange. White paper promised more than the deployment delivers.

# IPFS Bitswap

Bitswap is IPFS's block-exchange protocol — the peer-to-peer mechanism by which IPFS nodes ask each other for content-addressed blocks and decide who to serve. It's the **largest currently-deployed P2P data-exchange protocol** after BitTorrent itself (Filecoin, Web3 storage, NFT pinning services, hundreds of dApps).

The original IPFS white paper described a Bitswap with **credit-based reciprocity** akin to BitTorrent's choking — peers track bytes exchanged and bias decisions toward generous peers. The deployment shipped a *simpler* mechanism — best-effort serving with peer-scoring — that is closer to "informational ledger" than "enforced credit." Surfacing this gap clearly is the load-bearing job of this file: **Bitswap's deployed incentive model is weaker than the white paper suggested**, and the literature catches up via independent analyses rather than via IPFS-internal documentation.

The protocol is also being **partially supplanted** by HTTP retrieval (IPFS gateway-style) for many real workloads. The replacement is gradual and not formally announced; the practical effect is that "Bitswap is the only IPFS retrieval path" is no longer true in 2026.

## The protocol

- **Spec:** [Bitswap Protocol Specification](https://specs.ipfs.tech/bitswap-protocol/), Protocol Labs, ongoing.
- **Reference implementation:** [go-bitswap](https://github.com/ipfs/go-bitswap) (deprecated, folded into [boxo/bitswap](https://github.com/ipfs/boxo/tree/main/bitswap)); historically [js-ipfs-bitswap](https://ipfs.github.io/js-ipfs-bitswap/).

### Protocol primitives

Two-message protocol over libp2p:

- **WANTLIST.** A peer says "I want these CIDs." Sends to neighbors; neighbors decide whether to serve.
- **BLOCK.** A peer sends the requested block to a wanter.

A peer's neighbors are tracked in a session. Each session has:

- **A wantlist** of CIDs the local peer wants, ordered by priority.
- **A ledger** of bytes-sent / bytes-received per other peer.
- **A peer-scoring system** that elevates peers who have been helpful in the past.

### The ledger and the "credit" model (white paper)

The original IPFS white paper (2014) proposed:

- Each peer keeps a *running credit* per other peer: `credit_j = bytes_received_from_j − bytes_sent_to_j`.
- When deciding who to serve, peers prefer requesters with low (more positive) credit — i.e., peers who have served us in the past.
- If credit grows too negative (we've served them too much), serve them last.

This is essentially BitTorrent choking adapted to content-addressed blocks: **per-pair accounting, with serve-preferential-treatment to past-cooperators**.

### What deployed go-bitswap actually does

The deployed `go-bitswap` (and now `boxo/bitswap`) is closer to **best-effort with peer-scoring**:

- Peers serve any requested block they have, prioritizing by request-time (FIFO-ish) with light peer-scoring.
- The "ledger" exists but is mostly informational — visible via `ipfs.bitswap.ledger` admin queries, but rarely consulted for serve decisions.
- The 2024–2025 **Bitswap Broadcast Reduction** work added more sophisticated peer-state tracking: which peers actually respond to which CIDs, so broadcasts are reduced from 80–98%.
- The 2024 **WithScoreLedger** option (added in [PR #430](https://github.com/ipfs/go-bitswap/pull/430)) gave operators a hook to plug in custom credit policies; it's a *configurable extension*, not a default.

The IPFS community has had this gap surfaced repeatedly:

- The [2018 forum thread "Bitswap ledger as a source of truth"](https://discuss.ipfs.tech/t/bitswap-ledger-as-a-source-of-truth/2114) is the canonical user-side complaint.
- The [2020 forum thread on credit/strategy realization](https://discuss.ipfs.tech/t/go-bitswap-has-realized-the-logic-of-credit-strategy-and-ledger-in-the-ipfs-white-paper/4144) gets a Protocol Labs response acknowledging that the deployed protocol does *not* implement the white-paper credit model.

**This is a real divergence**, not just a documentation gap. The deployed protocol is incentive-*lighter* than the white paper described, by design.

## Why the divergence

Several reasons surfaced in IPFS-community discussions and protocol-design history:

1. **Ledger maintenance is expensive at scale.** A peer with N neighbors needs N ledger entries, each updated per byte transferred. For a popular IPFS node with thousands of neighbors per day, this is non-trivial overhead.
2. **The Sybil problem.** A credit-based ledger is only as useful as identity is stable. IPFS peer-IDs are cheap to generate; a peer can drop their negative-credit identity and rejoin fresh. Without a Sybil defense, the ledger doesn't enforce.
3. **Asymmetric workloads.** IPFS is dominated by *content publishers* (sources with high upload) and *content consumers* (sinks with high download). Strict credit accounting punishes consumers who can't reciprocate by uploading the same content back — a misalignment between the credit model and the actual workload pattern.
4. **HTTP gateways and pinning services.** Much of IPFS's real traffic flows through gateways (web2-IPFS bridges) and dedicated pinning services. These have their own incentive layer (subscription billing); the underlying Bitswap is just a transport.

The pragmatic effect: deployed Bitswap is **Sybil-tolerant per connection** (no defense, just per-connection-bounded harm) and **free-rider-tolerant globally** (no enforcement, but the cost of free-riding is bounded by the consumer's own bandwidth). The IPFS ecosystem has accepted this trade-off in exchange for protocol simplicity and operational tractability.

## Replacement: HTTP retrieval

The 2024–2025 IPFS evolution (Shipyard 2025 review) introduced **HTTP retrieval** alongside Bitswap:

- Peers can advertise availability via HTTP (gateway endpoints).
- A retrieval can fall back to HTTP fetching from a known gateway, bypassing Bitswap entirely.
- For most read workloads (web pages, NFT metadata, content discovery), HTTP retrieval is faster and simpler.

The architectural implication: **Bitswap is no longer the only IPFS retrieval path**, and probably no longer the primary one for most users. Bitswap remains essential for the *peer-to-peer* property (no central server) but is not load-bearing for casual consumers.

There is **no formal deprecation of Bitswap**, and search for "bao" or "beetswap" as named replacements returns no matches in 2026. Bitswap is being incrementally extended (Broadcast Reduction, peer-scoring improvements) rather than replaced.

## What Bitswap doesn't solve

- **Sybil resistance.** Peer-IDs are cheap to generate.
- **Free-rider enforcement.** The ledger is informational.
- **Long-running misbehavior.** No reputation that persists across sessions.
- **Asymmetric-bandwidth fairness.** A peer with no upload can consume without paying.
- **Storage-providing accountability.** Bitswap is a *retrieval* protocol; it does not verify that any peer is *storing* content. Filecoin's PoSt (see [Filecoin PoRep/PoSt in `algorithms.md`](algorithms.md)) is the cryptographic answer; IPFS pinning services are the operational answer.

## Implications for Myrhiza

1. **Don't mistake "ledger exists" for "ledger enforced."** Bitswap demonstrates that you can deploy a P2P content-exchange protocol at scale with essentially zero enforcement — and the network mostly works. The free-rider tax is real but bounded. Myrhiza could ship something similar as a v1 and tighten later; the deployed-at-scale precedent supports this.
2. **HTTP retrieval is a real escape valve.** Bitswap is not the only IPFS path because HTTP gateways subsume many workloads. The lesson for Myrhiza: a P2P maintenance-enforcement scheme should have an explicit "fall back to a server" mode for read workloads that don't need full P2P guarantees. The relay-as-bridge model in `prior-art/iroh/` and Willow's existing relay-capability-doc already point this direction.
3. **Per-pair credit doesn't scale without identity stability.** IPFS hit this. Myrhiza's invite graph gives identity stability that IPFS lacks — credit-style accounting is more viable in Myrhiza than in IPFS. But identity stability does not imply credit *should* be the accounting unit; per-event maintenance contribution or per-topic responsibility might be better units.
4. **The white-paper-vs-deployment gap is a warning.** Designs that look good on paper but require expensive per-peer state often get simplified at deployment time. Myrhiza specs for maintenance enforcement should model the deployment-cost-per-decision; over-engineered protocols don't ship.
5. **Bitswap is a useful comparison case for the lessons file** — *"like BitTorrent, but with content-addressing and weaker enforcement."* Its incentive-light deployment is honest about what works at scale.

## Sources

- [Bitswap Protocol Specification (IPFS standards)](https://specs.ipfs.tech/bitswap-protocol/).
- [Bitswap concept doc (IPFS docs)](https://docs.ipfs.tech/concepts/bitswap/).
- [boxo/bitswap (Go reference implementation)](https://pkg.go.dev/github.com/ipfs/boxo/bitswap).
- [js-ipfs-bitswap (JavaScript implementation, historical)](https://ipfs.github.io/js-ipfs-bitswap/).
- [IPFS White Paper (Benet, 2014)](https://github.com/ipfs/papers/raw/master/ipfs-cap2pfs/ipfs-p2p-file-system.pdf) — §3.4 on Bitswap.
- [IPFS roadmap (GitHub)](https://github.com/ipfs/roadmap/blob/master/README.md).
- [Shipyard 2025: Bringing IPFS Home](https://ipshipyard.com/blog/2025-shipyard-ipfs-year-in-review/) — current state and direction.
- [Forum: "Bitswap ledger as a source of truth"](https://discuss.ipfs.tech/t/bitswap-ledger-as-a-source-of-truth/2114) — community surface of the white-paper-vs-deployment gap.
- [Forum: "go-bitswap has realized the logic of credit, strategy and ledger in the IPFS white paper?"](https://discuss.ipfs.tech/t/go-bitswap-has-realized-the-logic-of-credit-strategy-and-ledger-in-the-ipfs-white-paper/4144) — Protocol Labs response.
- [WithScoreLedger PR (#430)](https://github.com/ipfs/go-bitswap/pull/430) — the operator-extension hook.
- [Enhancing IPFS Bitswap (2024)](https://www.researchgate.net/publication/384428232_Enhancing_IPFS_Bitswap) — recent academic analysis.
- [IPFS and Friends: A Qualitative Comparison (arXiv 2102.12737, 2021)](https://arxiv.org/pdf/2102.12737) — comparative analysis vs other P2P data layers.
- Cross-references: [`bittorrent.md`](bittorrent.md), [Filecoin PoRep/PoSt in `algorithms.md`](algorithms.md), [`taxonomy.md`](taxonomy.md) §1, [`lessons.md`](lessons.md).
