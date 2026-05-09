# Ecosystem

## Holochain Foundation

Established as the IP-holder for the Holochain project; in **August 2025** it transitioned from passive IP steward to "the active operational entity supporting and managing the Holochain development team" ([horizon-foundation-forward](https://blog.holochain.org/holochain-horizon-foundation-forward/)).

- **Eric Harris-Braun** — Executive Director (assumed role 2025; co-founder, formerly running development).
- **Mary Camacho** — Former ED, moved to focus on Holo Inc. commercial work; spoke at [Emerging Tech 2024](https://www.holochain.org/events/emerging-tech-2024).
- **Madelynn Martiniere** — Joined the board in 2025, focused on community/dev engagement.
- **Arthur Brock** — Co-founder; involved as advisor/board member.

In 2025 the Foundation spun out **Unyt, Inc.** as a wholly-owned subsidiary developing a generalized mutual-credit accounting engine (the multi-unit accounting framework intended to be a settlement layer paired with HoloFuel). The Foundation owns Holo Limited (the company that issued HOT in 2018), making the structure: Foundation → (Holo Ltd, Unyt Inc).

## Holo Inc. — hosting business model

Holo's pitch since 2018: a marketplace where **hosts** (running HoloPort hardware or commodity machines) sell hosting capacity for hApps to web users, paid in HoloFuel ([about-holo](https://holo.host/about/holo/)). The classic "Airbnb for hosting" framing.

**HoloFuel (XHF)** — mutual-credit currency. Audited and launched Q2 2024 ([buyholo news](https://www.buyholo.net/en/learn/news)). HoloFuel is the network-internal currency.

**HOT** — ERC-20 IOU sold in the 2018 ICO at $0.0006/token, intended to swap 1:1 for HoloFuel once mainnet hosting launched. The HOT→XHF swap window opened in 2024.

**Hosting status, May 2026.** Full hosting still has not launched at the scale promised in 2018. Holo announced ([Year in Review 2025](https://holo.host/blog/2025-year-in-review-the-year-we-built-the-edge-XqpCNKmMRVh/)) that they're using **HOT** (not HoloFuel) as the payment token for the initial hosting launch because "the counter-signature feature could not pass the required quality tests" required for HoloFuel. A static-site hosting product was the targeted Q3 2025 milestone; per the buyholo news tracker it is in beta as of late 2025 / early 2026. **The original "host any hApp for end users via the open web" vision has not shipped in eight years.**

## Volla partnership

[Announcement](https://blog.holochain.org/volla-partnership-announcement/), August 2024. **Volla Systeme GmbH** is a small German privacy-phone OEM (Volla OS = de-Googled Android, also ships Ubuntu Touch). The Volla **Quintus** smartphone began shipping fall 2024 with two preloaded Holochain apps:

- **Volla Messages** (rebrand of Terran Collective's [Relay](https://blog.holochain.org/happs-spotlight-relay/)) — E2E P2P messenger.
- **Recover** — encrypted incremental device backup with no cloud.

Both run on Holochain 0.4 and were built using p2p Shipyard tooling. Commercial relationship: Holochain Foundation provided engineering, Volla integrated; no public revenue share has been disclosed. The only commercial OEM hardware preinstall to date — and the first time native Holochain hApps shipped on a consumer smartphone ([mobile-applications-shipped](https://blog.holochain.org/mobile-holochain-applications-shipped/)).

## Funding history

- **2018 ICO** — $20,388,500 raised between March 29 – April 28, 2018 at $0.0006/HOT, 133.2B tokens minted ([cryptorank](https://cryptorank.io/ico/holo)). Funded HoloPort hardware, HoloFuel R&D, and the Holochain core rewrite from Go (`holochain-proto`) to Rust.
- No subsequent priced round at the company level is publicly listed (Crunchbase/Pitchbook profiles show 2018 as the funded event).
- **Operational funding 2020–2026** appears to come from HOT treasury sales, HoloPort hardware sales, and grants. The Foundation does not publicly publish a treasury balance.
- **Grants outbound:** small-grant programs occasionally announced via blog (Sensemaker grants, hackathon prizes).

## GitHub activity (2024–2026)

[holochain/holochain](https://github.com/holochain/holochain): 1.4k stars, 187 forks, ~13.4k commits on `develop`, 345 releases, license CAL-1.0, primary language Rust (99.3%). ~1,363 commits in the past year as of mid-2025 — a steady cadence dominated by the core team. Latest release at time of writing: **0.6.0 (November 19, 2025)**.

The [2025 ecosystem audit](https://soushi888.github.io/alternef-digital-garden/blog/holochain-ecosystem-reality-check-2025) ("A Friendly Reality Check") catalogued 140+ repositories across `holochain` and `Holo-Host` orgs, 56 Rust crates in the main monorepo, and 92 repos under Holo-Host. Contributor base is small — order of single-digit core committers per repo, dozens of historical contributors. The Holochain v0.6 milestone tracker was at 67% (247/365 points) in the analysis snapshot.

## Forum & community

- **forum.holochain.org** — Discourse-based, the primary developer Q&A venue since 2019. Replaced an earlier Mattermost. Still moderately active in 2025, slower than its 2020–2022 peak.
- **Discord** — `discord.gg/52Y8A7pVxu`, the developer chat. Co-runs with the **hAppenings Community Discord**, a community-organized server publishing the [hAppenings substack newsletter](https://happeningscommunity.substack.com/) which is the de-facto ecosystem news source.
- **Reddit, Twitter/X (@holochain), YouTube** — secondary channels.
- **Loomio** — used for some governance discussions (Moss/Weave team and others).
- The community organization **Happenings Community C.I.C.** (a UK Community Interest Company) coordinates ecosystem comms and event organization.

## Other shipping hApps

Beyond [`apps.md`](apps.md):

- **Moss / The Weave** ([lightningrodlabs/moss](https://github.com/lightningrodlabs/moss)) — by Eric Harris-Braun's Lightning Rod Labs; a "group OS" runtime where each Moss group composes a private peer-to-peer mesh out of pluggable Tools (chat, KanBan, video, governance, collab editing). Reference implementation of [The Weave](https://theweave.social/) interaction pattern. Active 2025 releases. Material precedent for a P2P composition framework.
- **Requests and Offers** — alpha hApp for community resource exchange, notable for its Effect-TS architecture (cited in 2025 audit).
- **Volla Tablet apps** — Relay/Messages confirmed for Volla's tablet line in addition to the Quintus.
- **Sensorica's NRP-CAS / true_commons** ([Sensorica/true_commons](https://github.com/Sensorica/true_commons)) — open-value-network resource planning built on hREA + ValueFlows.
- **HoloFuel apps** — the wallet UI and counter-signature flows are themselves hApps.

## Conferences & talks (2024–2026)

- **ETHDenver 2024** — Holochain sponsored, ran a booth and workshops, multiple core team members on site ([ETHDenver 2024](https://www.holochain.org/events/ethdenver-2024)).
- **Emerging Tech 2024** (Washington DC, June 14, 2024) — Mary Camacho on the decentralized web panel.
- **Digital Identity unConference Europe 2024** (Zurich, June 2024) — Matthew Schutte on agent-centric identity.
- **Web3 Summit 2024** (Berlin, August 19–21) — Holochain ran an open workshop/AMA track.
- **Token2049 Singapore** (September 2024) — Holo announced a partnership at the event.
- **Volla Community Days 2024** — Holochain hApps demoed alongside Volla hardware.
- 2025–2026 conference presence is lighter; focus visibly shifted to shipping 0.5/0.6 and the Volla rollout.

## Sources

- [Foundation page](https://www.holochain.org/foundation/)
- [Foundation Forward (Aug 2025)](https://blog.holochain.org/holochain-horizon-foundation-forward/)
- [Foundation Coming of Age](https://blog.holochain.org/the-holochain-foundation-is-coming-of-age/)
- [Holo about](https://holo.host/about/holo/)
- [HOT stats / ICO details](https://holo.host/hot-stats/)
- [Cryptorank ICO](https://cryptorank.io/ico/holo)
- [BuyHolo news tracker](https://www.buyholo.net/en/learn/news)
- [Holo 2025 Year in Review](https://holo.host/blog/2025-year-in-review-the-year-we-built-the-edge-XqpCNKmMRVh/)
- [Volla partnership announcement](https://blog.holochain.org/volla-partnership-announcement/)
- [Mobile applications shipped](https://blog.holochain.org/mobile-holochain-applications-shipped/)
- [Volla Quintus](https://happeningscommunity.substack.com/p/introducing-the-volla-quintus-smartphone)
- [holochain/holochain repo](https://github.com/holochain/holochain)
- [Ecosystem reality check 2025](https://soushi888.github.io/alternef-digital-garden/blog/holochain-ecosystem-reality-check-2025)
- [forum.holochain.org](https://forum.holochain.org/)
- [hAppenings community](https://happeningscommunity.substack.com/)
- [Moss / The Weave](https://github.com/lightningrodlabs/moss) / [The Weave](https://theweave.social/)
- [ETHDenver 2024](https://www.holochain.org/events/ethdenver-2024) / [Emerging Tech 2024](https://www.holochain.org/events/emerging-tech-2024) / [Web3 Summit 2024](https://www.holochain.org/events/web3-summit-2024)
- [Sensorica true_commons](https://github.com/Sensorica/true_commons)
