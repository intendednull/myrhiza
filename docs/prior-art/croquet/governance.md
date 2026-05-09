**Date:** 2026-05-09
**Status:** active
**Subject:** Croquet/Multisynq governance — three-era lineage (academic 2003 → Croquet Corporation 2018 → Multisynq Network 2024-2025), funding, license posture transition, people, and bus factor.

This file documents how the Croquet name and codebase moved across three distinct organizational eras, who owns what now, and what an open-source rebrand actually means when the server side is still proprietary. For the technical architecture see `architecture.md`; for the programming model see `programming-model.md`; for the platform layer see `multisynq-platform.md`; for distilled lessons see `lessons.md`.

## Three-era lineage at a glance

| Era | Period | Entity | Codebase | License | Status |
|---|---|---|---|---|---|
| 1. Academic | 2001-~2010 | Croquet Project (Smith/Kay/Reed/Raab/Lombardi/McCahill) | Squeak Smalltalk | MIT (SDK 1.0, Dec 24 2009) | Dormant by ~2010; branched into Open Cobalt, Open Croquet, Immersive Terf |
| 2. Commercial | May 2018 - 2024 | Croquet Corporation / Croquet Labs | JavaScript rewrite (`@croquet/croquet`) | Proprietary ("SEE LICENSE.md") through most of this era; the **2.0.4 republish on 2025-06-09 carries `Apache-2.0`** in npm registry metadata, aligning with the Multisynq rebrand | Reflector network operated by Croquet Labs |
| 3. Open-source rebrand | 2024 - present | Multisynq Network (Croquet Labs as primary provider) | `@multisynq/client` JS SDK | Apache-2.0 | Croquet network deprecated 2025-07-30; Multisynq DePIN is successor |

The codebase carries forward across eras 2 → 3. Era 1's Squeak codebase was abandoned; the modern stack is a from-scratch JS rewrite that retained the architectural ideas (deterministic VM, reflector-ordered messages, Model/View split) but not the original implementation.

## Era 1: The academic Croquet Project (2001-~2010)

The 2003 paper *Croquet: A Collaboration System Architecture* (Smith, Kay, Raab, Reed) was presented at C5 2003 (Creating, Connecting and Collaborating through Computing — not OOPSLA, a common citation error). The formal project began in late 2001 with Smith and Kay; Reed and Raab joined immediately, with Lombardi and McCahill added to the core in 2003.

**Funding (2003-2006).** Per Wikipedia, financial backing came from Hewlett-Packard, **Viewpoints Research Institute Inc.** (Alan Kay's nonprofit), the University of Wisconsin–Madison, the University of Minnesota, the Japanese National Institute of Communication Technology (NICT), and private individuals. This was a multi-institution academic effort, not a startup.

**License.** The project shipped under the **MIT License**. Beta of Croquet SDK 1.0 released April 18 2006; final 1.0 December 24 2009. After that, the project went substantially dormant.

**Branches.** Continued development moved into separate branded efforts: **Open Cobalt** (academic continuation), **Open Croquet** (parallel development), and **Immersive Terf** (commercial — former Qwaq/Teleplace technology acquired by ex-Croquet developers). None of these are the same lineage as the modern Croquet Labs / Multisynq stack; they are sibling forks of the Squeak-era code.

## Era 2: Croquet Corporation / Croquet Labs (2018-2024)

**David A. Smith founded Croquet Corporation in May 2018** to build "a software system for creating multiuser digital experiences on the web." This was a from-scratch JavaScript rewrite — the Squeak path was deprecated. Alan Kay became an advisor / academic co-author rather than active engineering contributor.

**Funding.** Croquet Corporation raised a **$2.7M seed round closed on 2020-02-18** (~21 months after founding), including **$2.0M from SIP Global Partners (SIP GP)**, with additional investment from "experienced technology and financial industry veterans." This is a small seed by 2018-2022 startup standards; the company has remained capital-constrained throughout its life.

**License posture.** Earlier `@croquet/croquet` versions shipped on npm with a license string of `"SEE LICENSE.md"` — i.e., proprietary terms. **The 2.0.4 republish on 2025-06-09 carries `Apache-2.0`** in npm registry metadata, aligning the legacy SDK with the Multisynq open-source rebrand. The main `github.com/croquet/croquet` repo (30 stars) is now also detected as Apache-2.0 by GitHub. Some sibling repos at `github.com/croquet` (e.g. `worldcore` 56 stars, `virtual-dom` 26 stars) remain `NOASSERTION` by GitHub's license detector — proprietary in practice for those.

**Reflector network.** The deterministic message-ordering server ("reflector") was operated by Croquet Labs as a hosted service. Apps ran the SDK locally; the reflector network was the centralized authority surface even though the *application logic* ran client-side.

**Marketing pivots.** Croquet went through several positioning iterations: "Live Collaboration Development Platform" (2020), "Open Metaverse OS" / "Microverse IDE" (2022, when Croquet announced "beta, open source, and open standards metaverse portals"), and eventually the Multisynq pivot. The metaverse positioning peaked with the 2022 hype cycle and faded with it.

## Era 3: Multisynq Network (2024-present)

The rebrand happened in **2024**, with the Multisynq seed round in **April 2024 ($2.2M, lead Manifold)** and a public token sale in **February 2025 ($350K)**. Other investors include Arkn Ventures, PHD Capital, Enigma Fund, AlphaCrypto Capital, Republic Crypto, Gmoney, NaniXBT, 0xLawliette, and Hype.eth.

**Why rebrand?** No official "this is why" post is prominent, but two factors are visible:

1. **Searchability.** "Croquet" collides catastrophically with the lawn sport (croquetscores.com, croquet associations, equipment retailers). "Multisynq" is unique. For a developer-platform brand, this matters.
2. **DePIN positioning.** The Multisynq pitch is explicitly a **D**ecentralized **P**hysical **I**nfrastructure **N**etwork — "monetize your excess internet bandwidth by selling it to developers." This frames the product as crypto-DePIN-token-economics rather than as an SDK; Croquet's brand carried 20+ years of academic-software baggage that did not fit that frame.

The relationship: **Croquet Labs is described as "the primary provider to Multisynq Network."** Croquet Labs (the company) develops the technology; Multisynq Network is the brand under which the decentralized infrastructure is sold. Practically, the same team and codebase, now under a different brand and a different commercial frame.

**Croquet network deprecation.** The legacy hosted reflector network was deprecated **2025-07-30**. The successor is the Multisynq DePIN network, in which third-party operators run **Synchronizer** nodes (the renamed reflector) for token rewards.

**Chainlink Build.** Multisynq joined the Chainlink Build program in May 2025, signaling a Web3-aligned go-to-market. Chainlink integration is positioned for "real-time data" use cases.

## License posture transition — what's actually open

The headline change is real but partial:

| Component | Era 2 (Croquet) | Era 3 (Multisynq) | Notes |
|---|---|---|---|
| Client SDK (JS) | `@croquet/croquet` proprietary in earlier versions; 2.0.4 republish (2025-06-09) is Apache-2.0 | `@multisynq/client` 1.1.0, **Apache-2.0** | Both open-source as of mid-2025 |
| React bindings | proprietary | `@multisynq/react` / `react-together` 31 stars, Apache-2.0 | Genuinely open-source |
| WorldCore framework | proprietary (NOASSERTION) | (no clear successor in Multisynq org) | Era 2 framework, not carried over |
| `synchronizer-cli` (management tool) | n/a | Apache-2.0 | CLI to operate a node — open |
| **Synchronizer (reflector) binary** | proprietary | **proprietary Docker image** (`cdrakep/synqchronizer`) | The actual server is **closed** |
| Network membership | n/a | Requires **Synq Key** issued by Multisynq | Permissioned operation |

Translation: **the SDK is open source; the server you must connect to is not.** Running a synchronizer node requires a Synq Key issued by Multisynq the company — meaning Multisynq the company controls who can operate authoritative infrastructure. This is meaningfully different from a fully self-hostable open-source stack like Yjs (where you own the relay), Iroh (where you own the relay), or Holochain (where there is no central operator at all).

For an Apache-2.0 SDK to be useful in a fully self-hosted scenario, you would also need an open-source reflector implementation. As of May 2026, **that does not exist publicly**. Forking the SDK is possible; running it without Multisynq's network is not — at least not without writing your own deterministic-message-ordering server from spec.

## People

### Croquet Project (academic, 2001–~2010)
- **David A. Smith** — co-founder; pioneer of 3D PC graphics (Virtus Corporation, *The Colony* shooter, set visualization for *The Abyss*).
- **Alan Kay** — academic co-architect; Viewpoints Research Institute principal.
- **David P. Reed** — academic co-architect; of "End-to-End Arguments" fame; co-author of *Reed's Law*.
- **Andreas Raab** — academic co-architect; deceased 2013-01-14.
- **Julian Lombardi, Mark P. McCahill** — invited to academic core in 2003.

### Croquet Corporation (Era 2) co-founders
- **David A. Smith** — founder May 2018, CTO and public face. Active.
- **Vanessa Freudenberg (`codefrau`)** — Chief Architect; led the JavaScript rewrite of the Squeak-era Croquet stack. **Died 2025-10-22** ([HN obituary 45672484](https://news.ycombinator.com/item?id=45672484)). Her loss is the load-bearing bus-factor signal for the project.
- **Aran Lunzer** — co-founder.
- **Yoshiki Ohshima** — co-founder.
- **Brian Upton** — co-founder.

### Era 3 (Multisynq) team
The public team page (`multisynq.io/team/`) is gated (returns HTTP 403 to automated fetch). Per LinkedIn and Crunchbase signals, the team is **bus-factor-small** — single-digit-to-low-double-digit headcount typical for a $2-3M-seed startup. After Vanessa Freudenberg's death (Oct 2025), Smith remains the only consistently-public engineering principal.

## Bus factor

Conservative read: **fragile.** The codebase has shipped continuously since ~2018, but on a small team funded at seed-stage levels and now also responsible for token-economics operations. The 2025-10-22 loss of Vanessa Freudenberg — Chief Architect and JS-rewrite lead — is the most concrete bus-factor signal in the corpus. There is no foundation, no second independent maintainer org, no Apache-Software-Foundation or CNCF home. If Smith leaves or Croquet Labs winds down, the SDK is Apache-2.0 (forkable) but the synchronizer network is proprietary (not forkable without a clean-room rewrite of the server protocol).

This is meaningfully worse than the Holochain bus factor (multi-entity foundation + commercial subsidiary + operational dev team, see `../holochain/governance.md`) and worse than Yjs's bus factor (single maintainer Kevin Jahns but the whole stack is open and forkable, see `../crdts/yjs.md`).

## Foundation alignment

**None.** Multisynq is not part of CNCF, Apache Software Foundation, Linux Foundation, or any other neutral steward. The Apache-2.0 license is on the SDK only; the *governance* of the project remains corporate.

## Implications for Myrhiza

1. **Treat as learn-from artifact, not depend-on dependency.** The conceptual model (deterministic VM + reflector-ordered messages + Model/View split, see `architecture.md`) is the load-bearing prior art. The actual code is small-team, single-vendor, and tied to a proprietary network operator.
2. **The "open SDK + closed server" pattern is a trap for self-sovereign apps.** Myrhiza's P2P thesis is incompatible with a network where one company issues the keys to operate authoritative infrastructure. If we copy Croquet's model, we must commit to an open-source sequencer/reflector or replace the role with peer-driven ordering (CRDTs, gossip, or chain-style consensus — see `comparisons.md`).
3. **The brand-rebrand cycle is itself a signal.** Croquet rebranded to Multisynq partly because the academic-Croquet brand collided with both the lawn sport and 20+ years of mostly-dormant repos. If Myrhiza picks a name, pick something that survives a search-engine collision test on day one.
4. **Token-economics-as-funding has tradeoffs we should map before adopting.** Multisynq's DePIN/token model creates a revenue stream but also creates regulatory surface area, sybil-resistance burden, and incentive misalignment risk. The honest comparison set here is Holochain (HOT/HoloFuel, see `../holochain/governance.md`) and Filecoin/Iroh's distinct path (no token, see `../iroh/`).
5. **Small-seed-startup-pace is the upper bound on velocity.** Multisynq has shipped steadily but not fast. If Myrhiza decides to integrate any Multisynq component, assume the integration ages on Multisynq's clock, not ours.

## Sources

- [Croquet Project — Wikipedia](https://en.wikipedia.org/wiki/Croquet_Project)
- [David A. Smith (computer scientist) — Wikipedia](https://en.wikipedia.org/wiki/David_A._Smith_(computer_scientist))
- [Croquet — Crunchbase](https://www.crunchbase.com/organization/croquet-corporation)
- [Multisynq — Crunchbase](https://www.crunchbase.com/organization/multisynq)
- [Multisynq — Crypto-Fundraising project profile](https://crypto-fundraising.info/projects/multisynq/)
- [Croquet Launches Live Collaboration Development Platform (2020)](https://www.businesswire.com/news/home/20200218005229/en/Croquet-Launches-Live-Collaboration-Development-Platform-for-the-Next-Generation-of-Apps-and-Games-for-5G-and-AR)
- [Croquet Announces Beta, Open Source and Open Standards Metaverse Portals (2022)](https://www.prnewswire.com/news-releases/croquet-announces-beta-open-source-and-open-standards-metaverse-portals-301571639.html)
- [Multisynq Joins Chainlink Build (May 2025)](https://medium.com/multisynq/multisynq-the-real-time-and-collaborative-application-layer-joins-chainlink-build-38e8a7bcef7e)
- [Multisynq Pilot Phase Launch — Brave New Coin](https://bravenewcoin.com/insights/multisynq-kicks-off-pilot-phase-pioneering-the-future-of-decentralized-infrastructure)
- [@croquet/croquet — npm](https://www.npmjs.com/package/@croquet/croquet)
- [@multisynq/client — npm](https://www.npmjs.com/package/@multisynq/client)
- [github.com/croquet](https://github.com/croquet)
- [github.com/multisynq](https://github.com/multisynq)
- [github.com/multisynq/synchronizer-cli](https://github.com/multisynq/synchronizer-cli)
- [Mazzy — Multisynq Synchronizer Node guide](https://mazzysweb.medium.com/multisynq-the-future-of-real-time-decentralized-collaboration-and-how-you-can-run-a-bf9e34fdb4e3)
- [Croquet: A Collaboration System Architecture (Smith/Kay/Raab/Reed, C5 2003)](https://www.semanticscholar.org/paper/Croquet-a-collaboration-system-architecture-Smith-Kay/8d3efe9a144a574002bd1f452c3adcf46fa915e2)
- [Authority Magazine — Makers of the Metaverse: David A. Smith](https://medium.com/authority-magazine/makers-of-the-metaverse-david-a-8f285df589b4)
