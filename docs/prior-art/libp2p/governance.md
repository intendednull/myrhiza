**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — governance, Protocol Labs stewardship, working groups, spec lifecycle

# Governance

libp2p is governed as an **open-source project shepherded by Protocol Labs** (the same entity behind IPFS, Filecoin, and the original libp2p split-out in 2015–16). The governance is loosely structured: working groups own specific protocol areas; major implementations have their own maintainer teams; specs have a formal lifecycle (Working Draft → Candidate Recommendation → Recommendation).

This file documents the governance shape, the **2024 Protocol Labs restructuring** (and what it meant for libp2p stewardship), and the working-group + spec-lifecycle model. The Myrhiza-relevant question — *"is the project healthy, and will it be maintained?"* — is answered by reading the maintenance signals (release cadence, commit activity, governance announcements) honestly.

## Stewardship: who actually maintains libp2p

Per the libp2p.io homepage: *"libp2p is an open source project by Protocol Labs."* In practice the stewardship is distributed across multiple teams:

| Component | Steward team |
|---|---|
| **libp2p/specs** | Protocol Labs network team (lead: rotating; historically [@yusefnapora](https://github.com/yusefnapora), [@raulk](https://github.com/raulk), [@marten-seemann](https://github.com/marten-seemann)) |
| **go-libp2p** | Protocol Labs IPFS team (canonical, drives spec authority) |
| **rust-libp2p** | Elena Frank ([@elenaf9](https://github.com/elenaf9)) + João Oliveira ([@jxs](https://github.com/jxs)) (named maintainers on README); historical author Parity Technologies |
| **js-libp2p** | Protocol Labs (lead: rotating); browser story increasingly maintained by ChainSafe |
| **`@chainsafe/libp2p-gossipsub`** | ChainSafe Systems |
| **nim-libp2p** | Status Research & Development (Vac team) |
| **jvm-libp2p** | ConsenSys (Teku team) + ChainSafe |
| **cpp-libp2p** | Soramitsu |
| **py-libp2p** | Ethereum Foundation (slow recovery from 2021–24 hiatus) |
| **Gossipsub spec** | [@vyzo](https://github.com/vyzo) (v1.0/v1.1 author), [@Nashatyrev](https://github.com/Nashatyrev) + [@Menduist](https://github.com/Menduist) (v1.2 IDONTWANT) |
| **QUIC + WebTransport specs** | [@marten-seemann](https://github.com/marten-seemann) (with quic-go work) |
| **Noise spec** | [@yusefnapora](https://github.com/yusefnapora) |
| **Kademlia spec** | [@raulk](https://github.com/raulk), [@jhiesey](https://github.com/jhiesey), [@mxinden](https://github.com/mxinden) |

The pattern: **specs are stewarded by Protocol Labs + close collaborators; implementations are stewarded by the teams that ship the production code**. This is a normal multi-stakeholder OSS governance structure — closer to Kubernetes' SIG model than to a strict BDFL.

## Spec lifecycle stages

The libp2p specs use an [explicit lifecycle](https://github.com/libp2p/specs/blob/master/00-framework-01-spec-lifecycle.md) (verified at <https://github.com/libp2p/specs/blob/master/00-framework-01-spec-lifecycle.md>):

| Stage | Full name | Description |
|---|---|---|
| **1A** | Working Draft / Active | Under development, actively worked on |
| **1D** | Working Draft / Deprecated | Under development but discouraged |
| **1T** | Working Draft / Terminated | Aged without consensus; auto-ended after 4+ months |
| **2A** | Candidate Recommendation / Active | Technically complete with ≥1 reference implementation; encouraged for adoption |
| **2D** | Candidate Recommendation / Deprecated | Complete but superseded; discouraged for new implementations |
| **3A** | Recommendation / Active | ≥2 interoperable implementations; highest stage; actively encouraged |
| **3D** | Recommendation / Deprecated | Highest stage but no longer applicable |

This is **unusually rigorous for an OSS protocol stack**. Compare:

- Iroh ships protocols *without* an external spec (`prior-art/iroh/open-problems.md` §"No published wire spec").
- IPFS protocols often skip the spec stage entirely (Bitswap had specs lag the implementation by years).
- ActivityPub / W3C protocols have a similar lifecycle but with a single W3C process.

The libp2p lifecycle is the right model. The catch: **the specs are still incomplete**. Per the libp2p/specs README header: *"The specifications for libp2p are currently incomplete, and we are working to address this by revising existing specs to ensure correctness and writing new specifications to detail currently unspecified parts of libp2p."* Several core protocols (multistream-select v2, some muxer details) are still implementation-defined.

## Working groups

libp2p uses informal working groups for major protocol areas. Active groups (as of 2026-05):

- **Networking** — transports, NAT traversal, browser story. Anchor: Marten Seemann.
- **PubSub** — gossipsub evolution. Anchor: vyzo + the Nashatyrev/Menduist axis.
- **Content routing** — Kademlia, indexer networks. Anchor: PL routing team.
- **Cryptography** — Noise, TLS, ciphersuites. Anchor: Yusef Napora.

Working groups are not formal — they're labels for "who's actually doing the work on this area." Decisions surface through GitHub issues + the [discuss.libp2p.io](https://discuss.libp2p.io) forum + periodic open community calls.

## The 2024 Protocol Labs restructuring

In late 2024, **Protocol Labs underwent significant restructuring**, including layoffs across the IPFS / libp2p / Filecoin teams. The exact details are private; public signals included:

- Multiple Protocol Labs employees publicly announcing departures via Twitter/Mastodon in Nov–Dec 2024.
- A noticeable slowdown in libp2p release cadence (rust-libp2p has not had a stable 0.57 release in 11 months as of 2026-05-22).
- The [Hydra Booster](https://github.com/libp2p/hydra-booster) project was archived in 2024–25.
- A general re-emphasis on Filecoin (the revenue-generating product) at the expense of the broader IPFS/libp2p stack as cost centers.

This is not a project death-spiral signal, but it is a **maintenance-velocity signal** worth flagging. The rust-libp2p team specifically has been thinned — both named maintainers (Elena Frank, João Oliveira) are recent additions; the historical Parity-era maintainers have largely moved on.

For Myrhiza: **don't take "libp2p will be maintained forever" as a given**. The MIT license means any libp2p crate can be forked, but the question is whether anyone will keep it shipping. Track release cadence + commit activity quarterly if libp2p ever becomes a hard dependency.

## Funding model

libp2p's funding has historically come from:

- **Protocol Labs operations** — payroll for the IPFS / libp2p team. Mostly self-funded by Filecoin token + early VC rounds (Filecoin raised ~$257M in 2017 ICO + ~$50M in venture rounds earlier).
- **Ethereum Foundation grants** — the EF funds many of the Eth2 libp2p contributions, esp. Discovery v5 work + gossipsub research.
- **Filecoin Foundation grants** — separately from Protocol Labs corporate, the FF makes grants to libp2p-related work.
- **Other downstream funders** — Status (nim-libp2p), ConsenSys (jvm-libp2p), Soramitsu (cpp-libp2p), ChainSafe (Lodestar + various js work) all fund their own teams.
- **Sponsorships** — modest. The [libp2p homepage](https://libp2p.io/) has sponsored historically by IPFS Foundation (separate legal entity from Protocol Labs) and various crypto foundations.

This is a **healthier funding model than iroh's** (which is a single private company, see [`../iroh/governance.md`](../iroh/governance.md)) — there are multiple independent stewards with their own resources. Even if Protocol Labs were to dissolve, ChainSafe, Status, ConsenSys, and Soramitsu would each continue maintaining their implementations.

The trade-off: **decision speed is slower** with multiple stewards. Spec changes that require coordination across go/rust/js/nim/jvm/cpp can take quarters. Iroh ships a transport breaking change in a minor release every 6 weeks because there's one stakeholder.

## Decision-making

There's no formal voting or veto process. Decisions surface through:

1. **GitHub issues** in libp2p/specs (the public deliberation forum for spec evolution).
2. **The [discuss.libp2p.io](https://discuss.libp2p.io) forum** (Discourse).
3. **Periodic community calls** (biweekly per the rust-libp2p README; cross-impl coordination).
4. **Open maintainer calls** scheduled via GitHub Discussions per implementation.
5. **Direct collaboration** between PL and downstream maintainer teams.

This is **mostly informal consensus** with PL acting as benevolent coordinator. In practice it works for the technical decisions; it breaks down on prioritisation (which spec to advance first, which protocols to deprecate, which implementations should land features first). The 11-month rust-libp2p release gap is symptomatic of this — no clear decision-making forum for "should we cut a release with the current API?"

## Security advisories

libp2p uses **GitHub Security Advisories** as the canonical disclosure channel. Per the rust-libp2p README: *"please file a private security vulnerability report. Please do not file a public issue on GitHub."*

Historical advisory volume is modest. Notable disclosures:

- **2022-04 — `multistream-select` denial-of-service** (CVE-2022-29260 area) — go-libp2p had a vector where a malicious peer could send a multistream-select message that caused excessive resource use. Patched in go-libp2p 0.20.x.
- **2022 — `noise` length-prefix attack** — the libp2p Noise handshake had a length-prefix parsing issue. Patched across all implementations.
- **2023 — `gossipsub` resource-exhaustion variants** — several attacks against gossipsub peer-scoring edge cases. Mitigated through score parameter tuning + spec refinements.

No CVE-grade exploit has hit gossipsub v1.1's score function in production since 2021's Filecoin attacks (which v1.1 was designed against). The Eth2 deployment serves as continuous adversarial pressure; no successful gossipsub-level attack is publicly known.

## License decisions

The license diversity (MIT vs Apache-2.0 vs dual) is governance-relevant:

- **go-libp2p, rust-libp2p, py-libp2p:** MIT.
- **js-libp2p, nim-libp2p, cpp-libp2p:** dual Apache-2.0 OR MIT.
- **jvm-libp2p:** MIT + Apache-2.0 via the Permissive License Stack.
- **`@chainsafe/libp2p-gossipsub`:** Apache-2.0 single (drift from js-libp2p's dual — ChainSafe's choice for the higher-stakes gossipsub deployment in Eth2).

No CLA / DCO requirement on any major implementation. Inbound = outbound (default for MIT/Apache-2.0).

## Implications for Myrhiza

- **The libp2p governance model is the healthiest in the P2P space.** Multiple independent stewards, formal spec lifecycle, named protocol-area leads, public deliberation forums. If Myrhiza ever evolves a governance model (kernel maintainer rotation, app behavior approval process, ABI versioning policy), libp2p's working-group + spec-lifecycle pattern is the closest viable model.
- **The 2024 PL restructuring is a maintenance-velocity warning, not a project-death signal.** Iroh's single-company stewardship is *more* fragile than libp2p's multi-steward structure. If Myrhiza ever revisits the iroh-vs-libp2p choice from a maintenance-risk lens, libp2p's distributed stewardship wins.
- **Decision-speed-vs-coordination is a real tradeoff.** Iroh ships breaking changes monthly because one team decides; libp2p coordinates across six teams, slower. Myrhiza is single-stewarded today (good for speed) but should plan for multi-stewarding later if specs ever need to outlive the original kernel team.
- **The spec lifecycle stages are worth borrowing.** Myrhiza's `docs/specs/` does not currently differentiate Working Draft from Recommendation. A 1A/2A/3A label for each Myrhiza spec would be cheap to add and would clarify "this is a sketch" vs "this is the final word."
- **Cross-implementation interop discipline doesn't apply unless Myrhiza ever becomes multi-impl.** Today we're single Rust + jco browser, so the spec rigor we need is just enough for our own deterministic-replay use. If Myrhiza ever ships a Swift kernel or a Kotlin app behavior, libp2p's spec-first discipline becomes the model.

## Sources

- [libp2p.io homepage — "open source project by Protocol Labs"](https://libp2p.io/)
- [libp2p/specs README — spec lifecycle + incompleteness disclosure](https://github.com/libp2p/specs)
- [libp2p spec lifecycle document](https://github.com/libp2p/specs/blob/master/00-framework-01-spec-lifecycle.md)
- [discuss.libp2p.io forum](https://discuss.libp2p.io/)
- [rust-libp2p README — maintainers + community calls](https://github.com/libp2p/rust-libp2p)
- [Protocol Labs layoffs reporting (Nov–Dec 2024)](https://www.coindesk.com/business/2024/11/) — search "Protocol Labs layoffs"
- [Hydra Booster archive notice](https://github.com/libp2p/hydra-booster)
- [iroh — governance (sibling doc)](../iroh/governance.md)
