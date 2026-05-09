# Goblins / OCapN Ecosystem

## Spritely Networked Communities Institute

[Spritely Networked Communities Institute](https://spritely.institute/) is a US 501(c)(3) nonprofit. It grew out of "the Spritely Project," a 2018 personal research effort by Christine Lemmer-Webber after her work co-editing W3C ActivityPub. The institute was incorporated to formalize the work and accept tax-deductible support. It is **co-founded by Christine Lemmer-Webber and Randy Farmer** (the latter a veteran of Electric Communities, the company that produced the E language).

Mission, in its own words: *advance networked user freedom* — steward standardization and base implementations for decentralized networked communities, promote user agency, develop everything as FLOSS, and "facilitate the framing and narrative of network freedom" ([about page](https://spritely.institute/about/)).

### Current leadership (May 2026)

- **Executive Director:** Christine Lemmer-Webber
- **CTO:** David Thompson (also primary maintainer of Hoot and Guile Goblins)
- **Founding Technologist:** Jessica Tallon (Goblins/OCapN lead, ActivityPub co-editor)
- Andy Wingo (Igalia) consults on Wasm/Guile work
- Juliana Sims contributes on Shepherd integration and ocap docs

### Board of directors

Karen Sandler (Software Freedom Conservancy), Deb Nicholson (Python Software Foundation), Alex Handy (The MADE).

## Funding

Spritely has been **NLnet/NGI-heavy plus individual donors**, with periodic top-up grants and one supporter drive on the books:

- **NLnet / NGI Assure / NGI Zero grants** — multiple awards over 2020–2026 to Jessica Tallon and Spritely directly. The OCapN standardization grant ran Aug 2022 – Oct 2023 ([NLnet project page](https://nlnet.nl/project/SpritelyOCapN/)). NLnet has separately funded Ridley (Dart "DObjects" OCapN port), a Haskell ocap layer ([NLnet Haskell-OCAP](https://nlnet.nl/project/Haskell-OCAP/)), and E2EE OCapN federated relays ([NLnet OCapN-federatedrelays](https://nlnet.nl/project/OCapN-federatedrelays/)). Specific dollar amounts per grant are not published but NGI Assure typical awards are €30–50k.
- **Sovereign Tech Fund** — referenced in some Spritely communications, though the institute's "more than $3M raised" figure includes mixed sources.
- **Supporter Drive (Dec 2024 – early 2025).** First-ever direct fundraising drive. **$80k goal hit early; final total ~$90k from 500+ donors, ~300 of them new** ([retrospective post](https://spritely.institute/news/2024-2025-supporter-drive-retrospective.html)).
- **Cumulative fundraising** is reported by Spritely as "over $3 million" raised since inception across grants and donations.

A healthier funding picture than most ocap research efforts, but still grant-shaped — payroll for ~3–5 senior engineers, not a venture-backed runtime company.

## Mark Miller's involvement

[Mark S. Miller](https://en.wikipedia.org/wiki/Mark_S._Miller) is the intellectual grandfather of the entire object-capability lineage — co-creator of [E](https://en.wikipedia.org/wiki/E_(programming_language)) at Electric Communities (1997), now Chief Scientist at **Agoric**. Not a direct Spritely employee but centrally involved in **OCapN governance** as one of the principal voices keeping Goblins, Endo, and Cap'n Proto's CapTP variants speakable to each other. Randy Farmer (Spritely co-founder) and Miller share the Electric Communities lineage, which is why Spritely's framing carries forward E's vat/promise/ocap vocabulary essentially intact.

## OCapN governance

[OCapN](https://github.com/ocapn/ocapn) is currently a **multi-org working group operating pre-spec** — there is no formal SDO. Coordination happens via:

- The OCapN GitHub org (specs, drafts, issues)
- Monthly meetings with published [meeting minutes](https://ocapn.org/meeting-minutes/september-2024.html)
- Spritely's community Discourse for public threads

Participating orgs: **Spritely** (CapTP features and the netlayer abstraction); **Agoric** (Endo, JS implementation, blockchain integration via IBC); **MetaMask** (uses Endo CapTP for Snaps); **Cap'n Proto** (long-term interop target via Kenton Varda); **independent implementers** like Ridley (Dart) and the Haskell ocap layer team. The standardization push is funded by NLnet/NGI Assure. Decision model is rough consensus among implementers; nothing has yet been pushed to IETF/W3C.

## Adjacent projects

- **Endo** ([endojs/endo](https://github.com/endojs/endo)) — Agoric's distributed JS sandbox built on SES; provides `@endo/captp`. The JavaScript anchor of OCapN.
- **MetaMask Snaps** — uses Endo to sandbox third-party MetaMask plugins; effectively the largest production user of CapTP-style ocap messaging today (millions of MetaMask installs).
- **Agoric blockchain** — uses Endo + a CapTP variant for inter-vat / inter-chain messaging; OCapN compatibility is a stated long-term goal.
- **Cap'n Proto** (Kenton Varda) — its existing CapTP is the elder cousin of OCapN's protocol; cross-implementation interop with Cap'n Proto is intentionally deferred until Goblins↔Endo is solid.
- **DObjects** (Dart, Ridley) and **Haskell ocap layer** — NLnet-funded language-specific ports.
- **GNU Shepherd** — being extended with Goblins for distributed sysadmin.
- **Mandy** — Goblins-on-ActivityPub bridge.

## Community size

Modest but engaged. Indicators:

- The [Spritely community Discourse](https://community.spritely.institute) is active with multiple threads per week and core maintainers responding.
- Spritely runs a Matrix room (`#spritely:matrix.org`) used as the main developer chat.
- Guile Goblins repo: ~1,939 commits, mostly David Thompson and Jessica Tallon as top contributors. Outside contributions exist but are sparse.
- Conference presence is heavy for the size: at FOSDEM 2025 alone Spritely had **seven distinct talks** across the Declarative & Minimalistic Computing, Distributed Systems, and other devrooms ([recap post](https://spritely.institute/news/spritely-presented-spirited-speeches-spanning-the-planet.html)). Earlier Strange Loop appearances (2019, 2021) by Christine Lemmer-Webber were how many people first heard of Goblins.

A "small core team, well-amplified by conferences, slowly growing peripheral implementer community" shape — closer in spirit to the Racket community than to Rust- or JS-scale ecosystems.

## Sources

- [Spritely Institute — About](https://spritely.institute/about/)
- [Christine Lemmer-Webber — Wikipedia](https://en.wikipedia.org/wiki/Christine_Lemmer-Webber)
- [Mark S. Miller — Wikipedia](https://en.wikipedia.org/wiki/Mark_S._Miller)
- [E programming language — Wikipedia](https://en.wikipedia.org/wiki/E_(programming_language))
- [OCapN GitHub repo](https://github.com/ocapn/ocapn)
- [OCapN September 2024 meeting minutes](https://ocapn.org/meeting-minutes/september-2024.html)
- [NLnet — Spritely OCapN grant](https://nlnet.nl/project/SpritelyOCapN/)
- [NLnet — Haskell OCAP](https://nlnet.nl/project/Haskell-OCAP/)
- [NLnet — OCapN federated relays](https://nlnet.nl/project/OCapN-federatedrelays/)
- [Spritely 2024–2025 supporter drive retrospective](https://spritely.institute/news/2024-2025-supporter-drive-retrospective.html)
- [FOSDEM 2025 Spritely recap](https://spritely.institute/news/spritely-presented-spirited-speeches-spanning-the-planet.html)
- [Endo (Agoric) repo](https://github.com/endojs/endo)
- [MetaMask Snaps docs](https://docs.metamask.io/snaps/)
- [Awesome ocap reference list](https://github.com/dckc/awesome-ocap)
