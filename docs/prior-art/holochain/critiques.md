# Critiques & honest assessments

A consolidation of substantive third-party and internal critiques of Holochain. Deliberately surfaces unflattering analysis. Where Holochain marketing makes a strong claim and an outside party pushes back, the pushback is preserved here verbatim where possible.

## Academic / research critiques

### Sybil-vulnerability trilemma (Tandfonline, 2024)

The paper *Sybil Attack Vulnerability Trilemma* (Bautista-Saucedo & Spagnuelo, *Int. J. Parallel Emergent & Distributed Systems*, 2024) formalizes an impossibility theorem: no protocol can simultaneously be **permissionless**, **Sybil-attack resistant**, and **free** (no resource expenditure). Any two are achievable; the third must give. The paper does not single Holochain out by name in the body content recoverable here, but the framework lands directly on Holochain's claim to be all three. Holochain markets itself as permissionless and free (no PoW/PoS resource burn), which by this theorem implies it cannot be globally Sybil-resistant. Holochain's own documentation tacitly concedes this — Sybil resistance is delegated to per-DNA membrane / membership proofs written by app authors, i.e. the system swaps "global Sybil resistance" for "permissioned-per-app Sybil resistance," which the paper would classify as not strongly permissionless. Net: Holochain doesn't escape the trilemma; it picks a corner.

### IoT review (PMC / MDPI Sensors, 2025)

*Among the DLTs: Holochain for the Security of IoT Distributed Networks — A Review and Conceptual Framework* (Sensors 25(13), 2025) is broadly positive but documents real numbers worth pinning: ~50 ms publish / ~30 ms retrieve / ~20 TPS *single-node* on the test rig. The review explicitly enumerates open challenges — interoperability, scalability under heterogeneous device populations, and **regulatory compliance** (GDPR-style right-to-erasure is fundamentally awkward on append-only source chains). The proposed "HoloSec" framework grafts machine-learning anomaly detection on top of Holochain, which is itself an admission that gossip + per-app validation is not sufficient on its own for adversarial IoT environments.

### Earlier academic work

The 2021 arXiv paper *Thinking Out of the Blocks: Holochain for Distributed Security in IoT Healthcare* (arXiv:2103.01322) is positive in tone but predates the Rust rebuild and the post-RSM architecture, so its empirical claims are now historical. Cite for completeness, not for current relevance.

## Industry / community critiques

### Basis Project (basisproject.net, April 2020)

The Basis Project evaluated Holochain as a substrate for an REA/ValueFlows-based economic protocol and walked away. Stated reasons (paraphrased and quoted from the post):

- **No native group agent.** "There's no built-in concept of a group in Holochain." Companies and regions in Basis need to act as a single accounting unit; Holochain's strict per-agent source chain forces every group operation back through individual agents and "we have to work backwards toward data consensus on a group level. This is currently a blocker."
- **Insufficient data consensus.** Some Basis operations require globally agreed state, which Holochain's per-DHT eventual consistency doesn't provide.
- **Immaturity.** "If blockchains are teenagers, Holochain is a toddler."
- **Documentation gaps.** Specifically around what data is available to validation callbacks and how transactions are validated.

One of the most concrete walk-away write-ups from a serious would-be adopter.

### "Friendly Reality Check" (Sacha Pignot / hAppenings.Community, 2025)

Pignot — a hands-on hApp lead, not a hostile outsider — surveyed 140+ ecosystem repos and lands on these honest findings:

- **Every flagship app is alpha.** Acorn ("Currently in Alpha testing phase"), Mewsfeed ("Working towards first major release"), Requests and Offers ("alpha stage"). "Every major application remains in alpha or development stages."
- **Production readiness 1–2 years out.** "1-2 years for complete ecosystem production readiness."
- **Holo hosting is bottlenecked by core Holochain alpha status.** "Production readiness is fundamentally limited by Holochain's own alpha status."
- **No production telemetry visible.** "No visible metrics on production workloads, user bases, or performance under load." "No clear evidence of production workloads or business model validation."
- **Centralization tension.** The Holo hosting model has "philosophical tension with peer-to-peer principles."
- **HoloPort hardware adds friction.** The HoloPort device strategy "adds adoption complexity."

Pignot frames this as bullish-but-honest. Read straight, it is a developer-ecosystem lead admitting that after eight years there are no production hApps.

### Hacker News, recurring critiques

- *HN 23280478* (2020): "Allowing any program to define its own set of validation rules in the hopes that they will be faster doesn't make things safer. It just makes it more likely for fails." Compares the validation surface to the Ethereum smart-contract bug class.
- *HN 32604428* (2022): "Store things cheaply, without blockchains or servers, so where is it stored? No infrastructure, so who is paying for it?" "Their speak around doesn't make them different, they are still crypto based, have a token, and spray buzzwords everywhere." "I would bet very strongly that nobody has, or ever will use this for a serious project or service."

The consistent HN throughline since 2018: marketing prose ("multicellular social organism") that reads as parody, undelivered hosting marketplace, an Ethereum-issued token gated against a runtime that has its own currency claim (HoloFuel), and no shipping reference apps.

### HOT token / ICO

Holochain raised ~30,000 ETH (~$20M at the time) in April 2018 in what the team called an "Initial Community Offering." Recurring critique: HOT was minted as an ERC-20 placeholder pending migration to HoloFuel mutual-credit. As of 2026 that migration has still not closed for the average HOT holder, and the hosting marketplace it was meant to fund is not in production. Bitcointalk and Holochain forum threads from 2018–2021 capture the slow-development narrative; price action (10x post-ICO, then sub-$0.0005 within months) reflects the gap between roadmap and delivery. This is not evidence of fraud, but it is a durable crypto-skeptic critique: an ICO funded on the promise of a hosting product that, eight years later, the ecosystem itself describes as alpha.

### Web3-aligned critique: "no consensus = centralized in practice?"

Because Holochain has no global consensus, "truth" inside an app is whatever the DHT validators in your shard say it is. Combined with the fact that Holo hosting is operated by a small set of HoloPort operators (and bootstrap / signaling servers are operated by the Holochain Foundation), Web3-aligned critics argue Holochain is *less* censorship-resistant than a chain in practice: no global record means a coordinated subset of authorities for a given hash neighborhood can simply refuse to gossip, and there is no chain-level fork to point at. Honest validation is enforced *per node* — defensive — but data *availability* is not.

## Holochain's own honest assessments

### Brock — "Wins, Missteps, and Next Steps" (Medium, 2020)

Brock acknowledged, in his own words:

- "In retrospect, we underestimated the consequences of the rebuild." (Go → Rust)
- "We underplayed the maturity of the prototype and encouraged devs who were ready to build on it to wait for the Rust version."
- "Holochain has gotten harder to use." (loss of multi-language SDKs; Rust-only HDK; lost RAD tooling)
- Open question whether they should have spun off a team to keep the Go prototype alive in parallel.

Net: Holochain shipped a prototype, told the world to wait, then took years to ship the replacement, and lost ecosystem momentum doing it.

### "2025 at a Glance: Landing Reliability" (Holochain Blog, late 2025)

The Foundation's own retrospective concedes, verbatim:

- "DHT synchronization that could take 30 minutes or more — if it completed at all."
- "Peer discovery that didn't quite discover, sync that sometimes just... didn't."
- The validation pipeline "had accumulated enough edge cases and failure modes that behavior was, to put it charitably, inconsistent."

The same post claims Kitsune2 brought sync down to "about a minute or faster in most cases." The honest read: through 2024, Holochain in production-like conditions had unbounded sync latency and unreliable peer discovery. That is the ecosystem state during which "Holochain is the future of the internet" was a public marketing line.

### HoloFuel testing post-mortem (Holochain Blog)

The HoloFuel pilot surfaced concrete problems:

- Source-chain branching during HoloPortOS upgrades caused validators to demand proofs from nodes that no longer had them.
- "We fixed Holochain so that validation would fail if the author's node can't produce a validation package" — and immediately "Holochain did what it was supposed to, and marked nearly everyone's data as invalid."
- Workaround: rotate the DNA hash and reset the DHT each test cycle, i.e. don't try to recover from a degraded eventually-consistent state, just blow it away.

The most concrete acknowledgement on record that eventual consistency on a per-app DHT is not free, and that the team chose `rm -rf the network` over building robust convergence.

## Specific technical critiques

### "Eventual consistency without ordering guarantees is just chaos"

Holochain provides per-source-chain total order (an agent's own actions are linearized by their hash chain) but no cross-agent order. Any application that needs a globally agreed sequence — auctions, double-spend detection, scarce-resource allocation, "first to claim" — must reconstruct ordering at the validation layer using application-specific tie-breaks. The HoloFuel post-mortem (currency app: validation depends on the *sum of all prior transactions of both counterparties*) demonstrates the cost: validation can be unboundedly expensive, can timeout, and a timeout produces an *indeterminate* result that is retried later. There is no formal SLA on convergence.

### Validation determinism enforced by the host, not the type system

Holochain forbids non-deterministic host calls inside validation callbacks (no `now()`, no RNG, no reading cell-owner private state, no cross-cell calls). Enforcement is runtime, not type-level. Concrete attack class: a malicious or buggy zome can attempt to embed nondeterminism via, e.g., undefined HashMap iteration order, floating-point edge cases, or WASM-host calls whose determinism status changes between Holochain releases. If validators reach different conclusions, the malicious entry can sit in the "indeterminate, retry later" bucket indefinitely while still being gossipped — a soft DoS on validation. The Component Model + WASI-style typed effect interfaces address this at the type layer; Holochain's HDK does not.

### DHT availability under partition

Per-shard authority means a small partition of authorities for a given hash neighborhood can become unavailable, and the data they hold simply is unfetchable until they return or until rebalance picks new authorities. There is no global redundancy floor. Combined with "validation timeout = indeterminate," partitions cause cascading retries rather than a clean partition-tolerance story. The inverse of CAP: Holochain explicitly chose AP within an app, but in practice loses A under correlated partitions because validation requires synchronous fetches of dependency entries.

### Storage-cost economics — why hosting can't be profitable yet

A hosting node must (a) store its DHT shard, (b) run zome validation for everything in that shard (CPU per validation), (c) gossip with neighbors (bandwidth), and (d) terminate WebSocket connections from web users via the Holo bridge. HoloFuel pays per "service log." The economics question — never publicly closed — is whether mutual-credit HoloFuel earnings cover the host's amortized hardware + electricity + bandwidth at a price competitive with AWS. As of 2025 the Holo network is not in revenue-generating production (per Pignot), and HoloPort operators have not been shown a steady-state P&L. The Web3-skeptic version: "decentralized hosting beats AWS on price" is a claim that has gone unfalsified for eight years because the marketplace has not been turned on.

## Sources

- *Sybil Attack Vulnerability Trilemma*, Tandfonline 2024 — https://www.tandfonline.com/doi/full/10.1080/17445760.2024.2352740 ; KCL preprint https://kclpure.kcl.ac.uk/ws/files/256740842/sybil_attack_vulnerability_trilemma_v3_4.pdf
- *Among the DLTs: Holochain for the Security of IoT Distributed Networks*, Sensors 25(13), 2025 — https://pmc.ncbi.nlm.nih.gov/articles/PMC12251913/
- *Thinking Out of the Blocks*, arXiv 2103.01322 — https://arxiv.org/abs/2103.01322
- Basis Project, *ValueFlows, Holochain, and blockchain* — https://basisproject.net/posts/2020/04/valueflows-blockchain-holochain/
- Sacha Pignot, *The Holochain Ecosystem in 2025: A Friendly Reality Check* — https://happeningscommunity.substack.com/p/the-holochain-ecosystem-in-2025-a
- HN thread (2020) — https://news.ycombinator.com/item?id=23280478
- HN thread (2022) — https://news.ycombinator.com/item?id=32604428
- Bitcointalk *HOT (HOLO) scam?* — https://bitcointalk.org/index.php?topic=5171677.0
- Coin Bureau Holochain review — https://coinbureau.com/review/holochain-hot/
- Arthur Brock, *Holochain: Wins, Missteps, and Next Steps* — https://medium.com/holochain/holochain-wins-missteps-and-next-steps-600812bc9ecc
- Holochain Blog, *2025 at a Glance: Landing Reliability* — https://blog.holochain.org/2025-at-a-glance-landing-reliability/
- Holochain Blog, *Challenges Endured … HoloFuel* — https://blog.holochain.org/part-1-challenges-endured-and-wisdom-gained-building-and-testing-holofuel/
- Forum, *Elemental Chat's weird bugs, eventual consistency, and you* — https://forum.holochain.org/t/elemental-chats-weird-bugs-eventual-consistency-and-you/4463
- Holochain validation docs — https://developer.holochain.org/concepts/7_validation/
