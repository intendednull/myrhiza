# Governance & funding

A reference snapshot of who runs Holochain, how decisions get made, what licensing it uses, and where the money came from. Current as of May 2026.

## The two-entity structure

Holochain has lived its entire existence as a **two-entity** stack, with a third subsidiary added in 2025:

| Entity | Form | Purpose | Founded |
|---|---|---|---|
| **Holochain Foundation** | Nonprofit foundation | Holds the Holochain framework IP, employs core dev team (post-2024), license/spec stewardship | (founded as IP-holding entity; reorganized into operational role Nov 2024) |
| **Holo Ltd.** (Holo Limited) | Gibraltar private company, reg #116305, Suite 23 Portland House, Glacis Road | For-profit operator of the Holo hosting network; issuer of HOT token; commercial vehicle for hosting + HoloFuel | 2017 |
| **Unyt, Inc.** | Foundation-owned subsidiary | Mutual-credit accounting engine (originally contracted by Holo to rebuild HoloFuel rails) | 2025 |

**Holo Ltd. is fully owned by the Holochain Foundation.** Holo's hosting business and HoloFuel cryptocurrency operations are how the Foundation's open-source mission was originally funded. ([Holochain — The Foundation](https://www.holochain.org/foundation/))

A separately registered "HOLO LTD" UK private company (#11847834) was dissolved on **January 4, 2022**; the active legal entity is the Gibraltar one.

## Funding history

### The 2018 ICO

Holo Ltd. ran an "Initial Community Offering" from **March 29 to April 28, 2018**, raising **~$20.39M USD** (~30,000 ETH) at $0.0006/HOT, hitting near the $20.4M cap (soft cap €1M, hard cap €25M). 133.2B HOT were minted for the ICO out of a 177.6B max supply (~75% to ICO participants). ([CoinCarp tokenomics](https://www.coincarp.com/currencies/holo/project-info/), [CryptoRank ICO page](https://cryptorank.io/ico/holo))

The single largest funding event in the project's history. The ICO funds were directed at building the Holochain framework (the open-source side), the Holo hosting network, the HoloPort hardware, and HoloFuel.

### HOT token mechanics

HOT is an **ERC-20 receipt token** — its purpose-of-record is to be redeemed 1:1 for HoloFuel (XHF) once HoloFuel launches on the Holo network, with redemption available "for at least six months" after launch. HOT is *not* the production currency; it is a tradeable IOU against the production currency. ([HOT vs HoloFuel — Atkinson, 2018](https://medium.com/h-o-l-o/holos-erc20-token-hot-and-mutual-credit-cryptocurrency-holo-fuel-6d8b6d3938d6))

Price history high points: ICO at $0.0006; **all-time high $0.0315 on April 5, 2021** (driven by a US patent grant announcement in March 2021); ~1,064% YoY in 2021. As of 2025, several centralized exchanges (e.g. Gate) **delisted HOT** for failing updated listing criteria; HOT continues to trade but has not approached its 2021 highs.

### HoloPort hardware sales

A late-2017 **Indiegogo campaign** sold three SKUs of "HoloPort" hosting hardware (Nano $99 ARM, $449/$999 Intel models). Initial reporting cited $220K+ pledged from ~530 backers, eventually growing larger. Manufacturing was in mass production by Dec 2018. HoloPorts continue to be sold at [store.holo.host](https://store.holo.host/).

## HoloFuel (XHF)

HoloFuel is a **mutual-credit** cryptocurrency. The defining property is that **net supply is always zero**: every positive balance is offset by an equal-magnitude negative balance somewhere else, and currency is created by being *spent into existence* by an account going negative within its credit limit. There is no scarce mineable supply; the active supply expands and contracts with real economic activity, and is "limited to the productive capacity of the network" (i.e., available hosting service). The model is closer to LETS / commercial barter / credit unions than to Bitcoin. ([HoloFuel model repo](https://github.com/Holo-Host/holofuel-model), [Mutual Credit Part 1](https://blog.holochain.org/mutual-credit-part-1-a-new-type-of-cryptocurrency-as-old-as-civilisation/))

**Audit and launch.** A network release candidate audit was completed (Least Authority is cited in community materials as a partner). HoloFuel was positioned to launch in Q2 2024 with the HOT→XHF swap window opening at launch, but actual launch has slipped repeatedly. As of 2025–2026, the rebuild of HoloFuel runs through **Unyt**, which was originally contracted by Holo specifically to deliver the swap rails before its scope broadened.

## Roles (as of May 2026)

- **Eric Harris-Braun** — co-founder; **Executive Director, Holochain Foundation** (returned to ED role announced Nov 15, 2024); also assumed **ED of Holo Ltd.** in late 2025 after Mary Camacho stepped down. Author of major recent strategic posts and the new public roadmap. ([Holochain Horizon, Aug 2025](https://blog.holochain.org/holochain-horizon-foundation-forward/), [Finding Our Edge](https://happeningscommunity.substack.com/p/finding-our-edge-a-strategic-update))
- **Arthur Brock** — co-founder; **Systems Architect** at Holochain. Currently most active at **Unyt** (the mutual-credit subsidiary), shipping crypto-accounting tooling and Circulo (community-currency hApp). Less directly involved in core Holochain framework dev day-to-day; focused on the currency/accounting layer.
- **Mary Camacho** — ED of Holo since 2018 and Foundation ED until Nov 2024; both ED roles have now transitioned to Eric. Continues advisory support for financial and strategic planning during transitions.
- **Madelynn Martiniere** — joined the **Holochain Foundation board** August 2025; described as providing direct support to the leadership team and broad community.
- **Alastair Ong** — stepped down as Director of Holo Ltd. in 2025.
- **Core dev team** — single-digit to low-double-digit headcount; two new hires in mid-2025 specifically for Wind Tunnel testing infrastructure. The Foundation's own description (post-Nov-2024 reorg) is that it now directly employs and manages the dev team rather than holding IP at arms-length.

## Decision-making process

There is **no formal HIP / RFC process** equivalent to Ethereum's EIPs or IETF RFCs at the protocol-governance layer for Holochain itself. The community has explicitly noted this gap, and the **"Holochain Emerging Standards"** protocol (with a hApp called `how`) is the proposed-but-still-emerging convergence mechanism, currently more aspirational than load-bearing. ([holochain-apps/how](https://github.com/holochain-apps/how))

In practice, major decisions are made by:

1. **Core team decisions on GitHub** — the [holochain/holochain](https://github.com/holochain/holochain) repo is the primary venue. Issues and PRs.
2. **Kanban-driven roadmap with public visibility** — since July 2025, [holochain.org/roadmap](https://www.holochain.org/roadmap/) exposes the team's actual story-point backlog, in-progress epics, and velocity metrics. Decisions about what releases contain are visible there.
3. **Dev Pulses** — a regular blog series ([Dev Pulse tag](https://blog.holochain.org/tag/dev-pulse/)) used as the public communication channel for release notes and direction.
4. **Foundation operational decisions** — post-Nov 2024 reorg, made by the Foundation's executive (Harris-Braun) with board oversight.

Two recent examples illustrate the process:

- **Removing DPKI (0.4 → 0.6).** DPKI/DeepKey moved behind a `unstable-*` compile-time flag in **0.4.0 (Dec 17, 2024)** as part of a broader pruning of experimental features (also: countersigning, warrants, app-level peer blocking, DHT sharding, chain-head coordination, task scheduling). In **0.6.0 (Dec 3, 2025)**, DPKI was removed from the conductor entirely, with config knobs deleted. Top-down decision communicated via dev pulses and the upgrade docs; not an open RFC.
- **Switching default transport to iroh (0.6.1).** Default network transport changed from `tx5` to **iroh** in 0.6.1 (early 2026), trading a homegrown WebRTC-based stack for the Iroh project's QUIC + hole-punching library. Communicated via blog/Twitter; the user-visible config consequence is that a `relay_url` is now required.

Roadmap priorities are set by the core team and Foundation leadership; the public roadmap surfaces *what* is being worked on, but priorities are not put to community vote.

## Licensing — CAL-1.0

Holochain is licensed under the **Cryptographic Autonomy License v1.0**, OSI-approved on **February 14, 2020**, after a 14-month review process. The license was **commissioned by Holo** and drafted by attorney **Van Lindberg** (formerly of Cisco/Cloudflare/python.org), submitted to the OSI on **December 4, 2019** as the fourth revision. ([OSI — CAL-1.0](https://opensource.org/license/CAL-1.0), [Heather Meeker — CAL approval](https://heathermeeker.com/2020/02/15/cryptographic-autonomy-license-approved-by-osi/))

### Why a custom license

Existing copyleft licenses (GPL, AGPL) trigger source-disclosure obligations on the *operator* of modified software. They were drafted assuming a server-client model where the operator holds user data. CAL was written for the **distributed-app case**, where if you're running a node holding *other users' data and keys*, the operator-vs-user power asymmetry is dramatically worse than a typical SaaS — and AGPL doesn't address it. Bruce Perens **resigned from the OSI in protest** of CAL's approval, arguing the user-data clauses overstepped Open Source Definition section 6 (no discrimination against fields of endeavor). The OSI approved it anyway.

### What CAL-1.0 actually requires

- **Source-code reciprocity** — strong copyleft like AGPL: modifications must be released under CAL-1.0 (or a compatible open-source license).
- **User data portability** — licensees **cannot withhold user data** from the user; must provide it "in commonly used electronic form."
- **No cryptographic lockout** — licensees cannot use cryptographic methods or DRM-like technical measures to **deny users access to functionality or their own keys**. The unique CAL clause.
- **No contractual override** — recipients can't be contractually prevented from exercising their rights under the license.
- **Combined Work Exception** — a marked file boundary lets licensed code be linked into proprietary code under different terms (so framework-vs-application can be separated).
- **90-day vulnerability embargo** allowed before source release.

The user-autonomy clauses are the philosophically distinctive part: CAL ties open-source obligations specifically to **end-user control of data and keys**, which is the property Holochain's architecture is designed to enforce. A license that pattern-matches the system it's licensing.

## Industry partnerships

The most prominent external partnership is **[Volla Systeme GmbH](https://volla.online/)** — a German privacy-oriented mobile vendor. The **Volla Quintus** Android phone (announced July 22, 2024, shipping fall 2024 via Kickstarter pre-sale) ships with two pre-installed Holochain hApps: **Relay** (encrypted messenger) and **Recover** (cloud-less encrypted backup). No financial terms disclosed; the play is distribution and ecosystem validation. ([Volla Partnership](https://blog.holochain.org/volla-partnership-announcement/))

Other ecosystem players (e.g. **darksoil studio**, builders of the p2p Shipyard mobile bundler, and the broader **Neighbourhoods/We/Weave** ecosystem) operate as independent communities/companies rather than formal Foundation partners. **Coasys/AD4M** is a notable independent project that builds on top of Holochain.

## Conflicts, criticism, and drama

**Slow delivery vs. ICO promises.** The most persistent criticism — internal and external — is that the project under-delivered relative to ICO timelines. Brock himself acknowledged this publicly in [Wins, Missteps, and Next Steps (May 2019)](https://medium.com/holochain/holochain-wins-missteps-and-next-steps-600812bc9ecc): "we underestimated the consequences of the rebuild" and "we underplayed the maturity of the prototype and encouraged devs… to wait for the Rust version." The Go prototype was usable; the Rust rewrite took roughly a year instead of the promised three months; then RSM in 2020 was effectively a *second* full rewrite. By 2019 the project was 18 months past ICO with no mainnet alpha. By 2026 the HOT→XHF swap has still not occurred — eight years post-ICO.

**HOT investor frustration.** Forum and Twitter sentiment has periodically been hostile, especially during 2022–2024 when development cadence felt opaque. The Foundation's 2024–2025 reorganization (public roadmap, exec-director shift to a co-founder, Unyt as a contractor to *finally* deliver the swap rails) is best read as a direct response to this pressure.

**Exchange delistings (2025).** Multiple centralized exchanges including Gate delisted HOT in 2025 citing failure to meet revised listing standards. Holo's response was that compliance and HoloFuel readiness — rather than HOT trading — is the priority, since HOT is by design a temporary receipt for HoloFuel.

**Bruce Perens / OSI resignation over CAL.** When the OSI approved CAL-1.0 in February 2020, Perens — one of the original drafters of the Open Source Definition — resigned from the OSI in protest over the user-data clauses, arguing they discriminated against certain use cases. A significant open-source-community fight at the time, but did not result in CAL being un-approved.

**Phishing scams targeting HOT.** Numerous third-party phishing scams have impersonated Holochain/Holo on social media offering airdrops or token swaps. The official Holo position ([holo.host/airdrop](https://holo.host/airdrop)) is that it has never run an airdrop; any "airdrop" is fraudulent. More an industry-wide phenomenon than Holochain-specific drama, but it has affected community trust.

**No notable governance forks.** Despite the slow delivery and rewrite cycles, the project has not seen a hostile fork. The community runs in/around the Foundation rather than against it.

## Sources

- [Holochain Foundation page](https://www.holochain.org/foundation/)
- [Holo Ltd. registration — Datocapital Gibraltar](https://www.datocapital.com.gi/companies/Holo-Ltd.html)
- [Holochain Horizon: Foundation Forward (Aug 14, 2025)](https://blog.holochain.org/holochain-horizon-foundation-forward/)
- [The Holochain Foundation is Coming of Age](https://blog.holochain.org/the-holochain-foundation-is-coming-of-age/)
- [Finding Our Edge — Strategic Update](https://happeningscommunity.substack.com/p/finding-our-edge-a-strategic-update)
- [HOT vs HoloFuel — Atkinson, Mar 2018](https://medium.com/h-o-l-o/holos-erc20-token-hot-and-mutual-credit-cryptocurrency-holo-fuel-6d8b6d3938d6)
- [CoinCarp HOT tokenomics](https://www.coincarp.com/currencies/holo/project-info/)
- [CryptoRank Holo ICO](https://cryptorank.io/ico/holo)
- [Holochain Indiegogo campaign](https://www.indiegogo.com/en/projects/holo/holo-take-back-the-internet-shared-p2p-hosting)
- [HoloFuel model repo](https://github.com/Holo-Host/holofuel-model)
- [Holochain Blog — Mutual Credit Part 1](https://blog.holochain.org/mutual-credit-part-1-a-new-type-of-cryptocurrency-as-old-as-civilisation/)
- [Brock — Wins, Missteps, and Next Steps](https://medium.com/holochain/holochain-wins-missteps-and-next-steps-600812bc9ecc)
- [Heather Meeker — CAL approved by OSI](https://heathermeeker.com/2020/02/15/cryptographic-autonomy-license-approved-by-osi/)
- [OSI — Cryptographic Autonomy License](https://opensource.org/license/CAL-1.0)
- [Volla Partnership Announcement](https://blog.holochain.org/volla-partnership-announcement/)
- [Introducing the New Holochain Roadmap](https://blog.holochain.org/introducing-the-new-holochain-roadmap/)
- [Holochain Emerging Standards — `how`](https://github.com/holochain-apps/how)
- [holo.host airdrop scam warning](https://holo.host/airdrop)
