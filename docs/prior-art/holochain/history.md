# History

A chronological narrative of Holochain from its 2016 founding through May 2026. Holochain is a long-running project — older than most "post-blockchain" frameworks — and its history includes a successful ICO, a full rewrite, a network protocol break, and a recent organizational pivot. We document it as prior art for the agent-centric / shared-validation design space that Myrhiza inherits from.

## 2016 — Founding

Holochain was started on **December 31, 2016** by **Arthur Brock** and **Eric Harris-Braun**, working under the **MetaCurrency Project** — a long-running effort by Brock and Harris-Braun (since the early 2000s) to build infrastructure for a P2P economy and currency design. Holochain was carved out of one specific subsystem of an earlier, larger project called **Ceptr** (short for "receptor"), which had been in development for years. Specifically, the part of Ceptr describing how multi-instance "receptors" synchronize state was extracted into a standalone project under the Holochain name as blockchain narratives gained mainstream traction. ([P2P Foundation: Holochain](https://wiki.p2pfoundation.net/Holochain), [Brock — Wins, Missteps](https://medium.com/holochain/holochain-wins-missteps-and-next-steps-600812bc9ecc))

The initial implementation, [`holochain-proto`](https://github.com/holochain/holochain-proto), was written in **Go**, GPL-3.0 licensed, and tagged as a Ceptr sub-project. It introduced the shape of the system that survives today: a per-agent append-only source chain, a content-addressed validating DHT, and per-DNA validation rules expressed as user code (initially Lisp/JavaScript via embedded interpreters).

## 2017 — First hackathons and prototype apps

The first public hackathon ran in **San Francisco, March 2017**, two months after the project began. Demo applications included HoloChat, TrustGraph, and a Twitter clone called Clutter. Sixteen-plus hackathons followed in 2017–2018 across Barcelona, London, Sydney, NYC, and elsewhere, producing roughly 60 prototype apps in various states of completion. **Alpha 0** of `holochain-proto` shipped in **October 2017**, and the [Indiegogo HoloPort campaign](https://www.indiegogo.com/en/projects/holo/holo-take-back-the-internet-shared-p2p-hosting) opened in late 2017, raising over $220K from ~530 backers selling NUC-class hosting hardware.

The dominant strategic narrative in 2017 was that **Holo Ltd.** — a separate Gibraltar-registered company (Holo Limited, registration #116305) — would operate a paid distributed hosting network using Holochain underneath, and pay hosts in a mutual-credit cryptocurrency called **HoloFuel**.

## 2018 — The HOT ICO

Holo's "Initial Community Offering" ran from **March 29 to April 28, 2018**, raising approximately **$20.39M USD** (~30,000 ETH) at **$0.0002 per token** ([CoinCarp tokenomics](https://www.coincarp.com/currencies/holo/project-info/)), near the ~$20.4M cap. The token, **HOT** (an ERC-20), was sold explicitly as a *receipt redeemable 1:1 for HoloFuel* once the hosting network launched, with the redemption window committed to remain open at least six months post-launch. Max supply ~177.62B; circulating supply has since grown to essentially the full max ([HOT vs HoloFuel — Mar 2018](https://medium.com/h-o-l-o/holos-erc20-token-hot-and-mutual-credit-cryptocurrency-holo-fuel-6d8b6d3938d6)).

**Alpha 1** of `holochain-proto` shipped in **May 2018**. **Binance listed HOT on July 24, 2018** (HOT/BTC and HOT/ETH pairs), at which point HOT spiked from ~$0.00066 to ~$0.00092 within hours.

Mid-2018, the team made a fateful decision: rebuild the Go prototype in **Rust + WebAssembly**, with promised parity in three months. The Rust rewrite became the public-facing version (`holochain-rust`), and the team de-emphasized the Go prototype. Brock would later acknowledge this "underplayed the maturity of the prototype and encouraged devs… to wait for the Rust version."

## 2019 — Networking troubles and stopgaps

The Rust rebuild took roughly a year instead of three months, and the *real* P2P networking layer (`lib3h`) lagged badly. To unblock app developers, two stopgaps appeared:

- **`sim1h`** (early 2019): a "DHT" simulator that stuffed everything into a centralized AWS DynamoDB so apps could be tested without working P2P. Built in three weeks at a Montreal dev retreat ([sim1h GitHub](https://github.com/holochain/sim1h)).
- **`sim2h`** (mid-2019): replaced sim1h with a centralized "switchboard" — agents held their own DHT shards but gossip routing went through a central WebSocket server ([Sim2h blog post](https://blog.holochain.org/sim2h-holochains-simple-switch-board-networking/)).

Neither was actually peer-to-peer. The white-paper-promised production networking, `lib3h`, was never finished in usable form.

CAL-1.0 work also began this year: the **first draft of the Cryptographic Autonomy License** was released in February 2019, and Holo formally submitted it to the OSI on **December 4, 2019** via attorney Van Lindberg.

## 2020 — RSM, the full Rust rewrite

By 2020 the team had decided that the Redux-pattern internal state model in `holochain-rust` was making the system impossible to reason about. They announced **RSM ("Refactor — Show Must Go On"**, sometimes glossed as "Refactored State Model"). Rather than refactor in place, they did a *second* full rewrite of the rewrite. ([Announcing the New Holochain](https://blog.holochain.org/announcing-and-unpacking-the-new-holochain/))

The RSM announcement landed **September 16, 2020**, claiming dramatic gains: 10,000× faster execution, 1/10th the memory, WASM call overhead from 100–200ms down to <0.1ms (Wasmer replaced the old wasmi interpreter). The architecture was reorganized around explicit "workflows" with tokio futures and atomic LMDB writes; serialization moved from JSON to MessagePack; QUIC replaced WebSockets between conductors; HDK 3.0 macros cut app code by ~3×; capability-based security and native countersigning landed. The old `holochain-rust` repo was archived; the new code lives at [holochain/holochain](https://github.com/holochain/holochain).

The first real P2P networking landed in **November 2020** ([Networking Has Landed](https://blog.holochain.org/networking-has-landed/)) — sim2h was finally retired in favor of an actual peer-to-peer DHT.

In parallel: **CAL-1.0 was approved by OSI on February 14, 2020**, finally making Holochain's chosen license OSI-blessed.

## 2021 — `kitsune_p2p` and the patent

The November 2020 P2P layer was rebranded and extended into [`kitsune_p2p`](https://crates.io/crates/kitsune_p2p), the first production-shaped networking subsystem. Through 2021 the team shipped continuous bugfixes around gossip reliability, agent re-publishing, peer discovery via bootstrap servers, and the rrDHT arc-resizing algorithm.

In **March 2021**, Holo Ltd. announced a US patent grant for the Holochain framework, which contributed to HOT spiking from ~$0.0007 (Feb 1) to its **all-time high of $0.0315 on April 5, 2021** — a ~4,000% surge in two months, and the high-water mark for the project's market valuation. HOT closed 2021 up ~1,064% YoY.

## 2022 — Stabilization and the integrity/coordinator split

Through 2022 the project pushed through 0.0.x releases with stabilization the explicit goal: HDK API stability, gossip protocol stabilization, and a new arc-resizing/syncing algorithm.

The headline architectural change of 2022 landed in **Holochain 0.0.144 on June 16, 2022**: **integrity zomes split from coordinator zomes**. Integrity zomes contain entry/link definitions and validation rules and are part of the DNA hash; coordinator zomes hold the read/write/messaging API and can be updated *without changing the DNA hash*, meaning a running network can receive bug fixes without forking. The HDI (Holochain Deterministic Integrity) crate was cordoned off from HDK as the long-term stable surface. ([Dev Pulse 121](https://blog.holochain.org/integrity-and-coordination-part-ways/))

Releases through summer 2022 included **0.0.156 (Aug 23)** disabling WASM metering and **0.0.158 (Aug 31)** introducing `must_get_agent_activity` for deterministic validation.

The **0.1.0-beta-rc.0** release shipped **December 15, 2022**, and **0.1.0-beta** in **January 2023**, carrying a 6-month LTS commitment and freezing breaking changes.

## 2023 — 0.1, 0.2, 0.3 cadence

2023 saw a faster release cadence: **0.2.0-beta-rc** dropped Jan 17/20 2023, and through the year the team established the practice of rolling forward through minor versions every few months. Dev Pulse cadence picked up. The integrity/coordinator separation matured into a stable developer story. The Holochain Launcher (an Electron-based app store + runtime for end users) shipped major updates.

Year-end 2023 retrospectives ([2023 Year in Review](https://blog.holochain.org/holochain-2023-year-in-review/)) emphasized stabilization and ecosystem rebuilding rather than headline features.

## 2024 — 0.3, 0.4, DPKI, Volla, HoloFuel

**Holochain 0.3 — June 11, 2024.** Single-threaded validation per DHT space (replacing the multi-thread model that had caused deadlocks); per-UI auth tokens for app WebSocket sessions; wire protocol incompatible with 0.2; mobile support via darksoil studio's **p2p Shipyard** (Android APK + desktop installer bundling); zero-width DHT arcs for mobile/light nodes. ([0.3 + HC on Mobile](https://blog.holochain.org/holochain-0-3-a-new-launcher-and-hc-on-mobile/))

**0.2.7 → 0.2.8 — April 2024.** Backend clone-cell management; WebSocket binding fixes ([Dev Pulse 139](https://blog.holochain.org/dev-pulse-139-holochain-0-2-8-the-weave/)). The Weave initiative (composable hApp framework) became visible.

**Volla partnership — announced July 22, 2024.** Volla Systeme GmbH — a German privacy-focused mobile vendor staffed by ex-Ubuntu engineers — committed to ship the **Quintus** Android phone with two pre-installed Holochain apps: **Relay** (encrypted 1:1/group messenger, beta August 2024) and **Recover** (encrypted, cloud-less incremental backup). No financial terms disclosed; the play is distribution and validation, not licensing revenue. ([Volla Partnership](https://blog.holochain.org/volla-partnership-announcement/))

**HoloFuel audit + planned launch — Q2 2024.** Network release-candidate audit completed (Least Authority cited in community materials). HoloFuel was framed as ready to launch as a mutual-credit currency in 2024, with the HOT→XHF swap window opening on launch. Actual launch slipped, with the HOT/XHF swap being repositioned through 2024–2025 against the new Unyt-built rails.

**Holochain 0.4.0 — December 17, 2024.** The defining decision was **moving experimental features behind a `unstable-*` compile-time flag**: Countersigning, Warrants, App-level peer blocking, DHT sharding, **DPKI/DeepKey**, Chain head coordination, and Task scheduling all moved out of the default build. Conductor "services" abstraction added (DeepKey shipped as the first DPKI service, behind the flag). Integration workflow handling received-gossip messages was audited and refactored.

**Organizational shift — November 15, 2024.** Mary Camacho (Executive Director of Holo since 2018, also serving as ED of the Holochain Foundation) stepped down from the Foundation ED role. Eric Harris-Braun returned to the foreground as the **Holochain Foundation's Executive Director**, and the Foundation announced a strategic shift "from passive holder of Holochain IP… into the active operational entity supporting and managing the Holochain development team." ([Holochain Horizon](https://blog.holochain.org/holochain-horizon-foundation-forward/))

## 2025 — Kitsune2, 0.5, 0.6, Unyt

**Holochain 0.5.0 — April 22, 2025** ("almost ready" blog post May 12, 2025 — referring to ecosystem readiness, not the release itself; recommended 0.5.2 — May 8, 2025 — by [Dev Pulse 148](https://blog.holochain.org/dev-pulse-148-major-performance-improvements-with-0-5/)). The headline change was **Kitsune2**, a from-scratch rewrite of the network/DHT layer. Kitsune2's wire protocol is **incompatible with Holochain 0.4 and earlier** — conductors on 0.5+ cannot speak to conductors on 0.4. The user-visible payoff was DHT sync time dropping from 30+ minutes to ~1 minute for new nodes, and "almost immediate" for already-synced peers. CPU usage dropped notably.

**0.5.4 / 0.4.4 — July 11, 2025.** Critical zome-call atomicity fix (commits could occur despite errors). Two new developers hired to expand the **Wind Tunnel** distributed test runner ([Dev Pulse 150](https://blog.holochain.org/dev-pulse-150-minor-releases-more/)).

**New roadmap launched — July 10, 2025.** Eric Harris-Braun introduced a Kanban-driven roadmap with three states (Released / In Progress / Up Next), velocity metrics, and story-point estimates exposed publicly. ([New Roadmap](https://blog.holochain.org/introducing-the-new-holochain-roadmap/))

**Unyt, Inc. — launched 2025.** A second wholly-owned Foundation subsidiary, originally contracted by Holo to rebuild HoloFuel and the HOT swap rails, but with broader scope as a generalized mutual-credit accounting engine. The Unyt accounting software was released **September 2025**. **Circulo**, a community-currency hApp built on Unyt rails, launched in late September 2025.

**Foundation board — August 14, 2025.** **Madelynn Martiniere** joined the Foundation board, providing direct support to leadership and community.

**Holo Ltd. leadership change — late 2025.** Mary Camacho also stepped down as Executive Director of Holo Ltd. (separately from her earlier Foundation departure). Eric Harris-Braun assumed Holo's ED role on top of the Foundation ED role, consolidating leadership. Director Alastair Ong also stepped down. Holo pivoted strategy to **Edge Node**, an open-source container approach more accessible to typical Docker-using devs. ([Finding Our Edge](https://happeningscommunity.substack.com/p/finding-our-edge-a-strategic-update))

**Holochain 0.6.0 — November 19, 2025.** The "immune system" release: warrant gossip blocks misbehaving agents at the network layer; validation and networking thoroughly overhauled. **DPKI was removed entirely** from the conductor (configuration knobs deleted), reflecting the experimental-feature pruning that started in 0.4. iroh transport landed as an option here, made default in the subsequent 0.6.1-RC line. The release was announced in [Dev Pulse 153](https://blog.holochain.org/dev-pulse-153-holochain-0-6-released-with-immune-system/) (Dec 3, 2025).

**Holochain 0.6.1-rc (early–mid 2026).** Default network transport changed from `tx5` to **`iroh`** (QUIC + hole-punching from the Iroh project), with `relay_url` now required for cross-NAT connectivity. Latest RC tag at time of writing: `0.6.1-rc.8`, April 17, 2026. The first time the entire deployed ecosystem must redeploy networks for a transport change.

End-of-year Foundation summary: 2025 was framed not by features but by *reliability landing* — Kitsune2 integration, validation overhaul, warrants, Wind Tunnel hitting production-ready, the new dev Build Guide, and the public roadmap.

## 2026 — Current state (May 2026)

The active branch is **0.7.0-dev**, with `0.7.0-dev.23` tagged May 4, 2026 and `0.6.1-rc.8` on April 17, 2026 ([releases page](https://github.com/holochain/holochain/releases)). 0.6.1 is finalizing iroh-as-default, and 0.7 is the next minor planned to land further validation/network refactors plus a coordinator-zome update mechanism.

Roadmap items still pending as of May 2026 include: re-introducing some of the unstable-flagged features (countersigning, warrants beyond agent-key authorities, full DHT sharding) on a stable footing; conductor state refactoring for cell consistency; and broader hApp ecosystem tooling. The HOT→XHF swap remains an open commitment, contingent on the Unyt-built mutual-credit infrastructure actually shipping into the Holo network.

The HOT token continues to trade but at a small fraction of the April 2021 peak; multiple centralized exchanges (e.g. Gate) **delisted HOT in 2025** citing failure to meet revised platform standards.

## Sources

- [Brock — "Holochain: Wins, Missteps, and Next Steps" (May 2019)](https://medium.com/holochain/holochain-wins-missteps-and-next-steps-600812bc9ecc)
- [Holo HOT vs HoloFuel — Atkinson, Mar 2018](https://medium.com/h-o-l-o/holos-erc20-token-hot-and-mutual-credit-cryptocurrency-holo-fuel-6d8b6d3938d6)
- [Binance HOT listing — Jul 2018](https://www.binance.com/en/support/announcement/binance-will-list-holo-hot-on-2018-07-24-360007931511)
- [holochain/holochain-proto (Go prototype)](https://github.com/holochain/holochain-proto)
- [holochain/sim1h](https://github.com/holochain/sim1h), [holochain/sim2h](https://github.com/holochain/sim2h), [holochain/lib3h](https://github.com/holochain/lib3h)
- [Announcing and Unpacking the New Holochain (RSM, Sep 2020)](https://blog.holochain.org/announcing-and-unpacking-the-new-holochain/)
- [Networking Has Landed (Nov 2020)](https://blog.holochain.org/networking-has-landed/)
- [Dev Pulse 121 — Integrity and Coordination Part Ways (Jun 2022)](https://blog.holochain.org/integrity-and-coordination-part-ways/)
- [Dev Pulse 128 — Holochain Beta Approaching (Dec 2022)](https://blog.holochain.org/holochain-beta-approaching/)
- [Dev Pulse 139 — Holochain 0.2.8 & The Weave (Apr 2024)](https://blog.holochain.org/dev-pulse-139-holochain-0-2-8-the-weave/)
- [Holochain 0.3, a new Launcher, and HC on Mobile (Jun 2024)](https://blog.holochain.org/holochain-0-3-a-new-launcher-and-hc-on-mobile/)
- [Volla Partnership (Jul 2024)](https://blog.holochain.org/volla-partnership-announcement/)
- [Holochain Upgrade 0.4 → 0.5](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.5/)
- [Dev Pulse 148 — Major Performance Improvements with 0.5 (Jun 2025)](https://blog.holochain.org/dev-pulse-148-major-performance-improvements-with-0-5/)
- [Holochain 0.5 is (Almost) Ready (May 2025)](https://blog.holochain.org/holochain-0-5-is-almost-ready/)
- [Dev Pulse 153 — Holochain 0.6 Released with Immune System (Dec 2025)](https://blog.holochain.org/dev-pulse-153-holochain-0-6-released-with-immune-system/)
- [Holochain Upgrade 0.5 → 0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)
- [2025 at a Glance: Landing Reliability (Dec 2025)](https://blog.holochain.org/2025-at-a-glance-landing-reliability/)
- [Holochain Horizon: Foundation Forward (Aug 2025)](https://blog.holochain.org/holochain-horizon-foundation-forward/)
- [Finding Our Edge — Strategic Update (2025)](https://happeningscommunity.substack.com/p/finding-our-edge-a-strategic-update)
