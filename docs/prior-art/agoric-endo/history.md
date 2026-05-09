**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/Endo/SwingSet — chronological history from the E lineage to present

# History

This file traces the lineage of Agoric, Endo, and SwingSet from Mark Miller's earliest ocap research through the modern chain and Endo monorepo. The arc matters: Agoric is not a 2018 startup that picked ocap as a feature — it is the production-engineering arm of a research program that has been running, with substantial continuity of personnel, since the late 1980s.

For the research-grade ocap sibling project, see `../spritely-ocapn/`. For the unrelated transport substrate occasionally referenced here, see `../iroh/`.

## 1988–1991: AMIX and the prehistory

The American Information Exchange (AMIX) was a marketplace for information goods built by Phil Salin starting in 1984, with Chip Morningstar as chief architect and Randy Farmer involved. Autodesk acquired 80% in 1988. It pioneered reputation systems, escrow, and what Salin called "smart contracts" — the term predates Nick Szabo's later coinage. **Dean Tribble** designed the negotiation process for AMIX, and **Bill Tulloh** was its market manager. AMIX shut down shortly after Salin's death in December 1991 ([Wikipedia: AMIX](https://en.wikipedia.org/wiki/American_Information_Exchange)).

This matters because three of the four founders of modern Agoric (Tribble, Tulloh, Miller — Miller via the Agoric Open Systems papers) trace their professional lineage through this 1980s information-marketplace project. The Agoric pitch is not a 2018 reinvention; it is the same idea with thirty years of accumulated theory.

## 1990s: Electric Communities and the E language

Mark Miller and others (Dan Bornstein, Douglas Crockford, Chip Morningstar, et al.) created the **E programming language** in 1997 at Electric Communities. E descends from Joule (a concurrent capability language) and from Original-E (capability extensions to Java for distributed programming). The E concurrency model — event loops + promises, structured to make deadlock impossible — is the direct ancestor of CapTP and of JavaScript's eventual `Promise` semantics ([Wikipedia: E](https://en.wikipedia.org/wiki/E_(programming_language))).

Brian Warner — later an Agoric founder — was part of the Tahoe-LAFS storage project and built **Foolscap**, a Python implementation of similar ocap-distributed-object ideas. He also led the security design of Firefox Sync and Firefox Accounts at Mozilla.

## 2006: Mark Miller's dissertation

The canonical academic citation for ocap as a unified discipline is:

> Mark S. Miller, "Robust Composition: Towards a Unified Approach to Access Control and Concurrency Control," PhD dissertation, Johns Hopkins University, May 2006.

Available at `http://erights.org/talks/thesis/markm-thesis.pdf` (link was unreachable from the research environment used for this file — verify before deep-linking; the file is widely mirrored). The dissertation argues that access control and concurrency control are not separate concerns: both are about *what an object is allowed to do, and when*. Capabilities, properly designed, give you both at once. This is the foundational text for Agoric, Spritely, and Cap'n Proto's RPC layer, which all cite it.

## 2007–2014: Caja at Google

Mark Miller joined Google in 2007. Caja ("box" / "capabilities-Ja[vaScript]") was a project led by Jasvir Nagra, with Miller as the key JavaScript designer, to sanitize untrusted third-party HTML/CSS/JS — used by Google Apps Script, Yahoo!, and MySpace ([Wikipedia: Caja project](https://en.wikipedia.org/wiki/Caja_project)). Caja was a source-to-source translator into a safe ES5 subset; it is the direct technical ancestor of SES (Secure EcmaScript), which generalized the "freeze the primordials, sandbox the rest" pattern.

Google archived Caja on January 31, 2021, citing "known vulnerabilities and lack of maintenance to keep up with the latest web security research." By that time the active line of development had migrated through Agoric's SES-shim and `realms-shim`.

Miller stayed at Google through 2017 (~10 years), during which he also began the long-running TC39 work that became Realms, SES, and Compartments.

## 2018: Agoric founded

**Agoric Systems Operating Company** was incorporated in Delaware on March 16, 2018 (verified via OpenCorporates / Justia trademark filings). Founders, per the May 20, 2018 "Introducing Agoric" post ([agoric.com](https://agoric.com/blog/announcements/introduction/)):

- **Mark S. Miller** — Chief Scientist, ocap research, E language, Caja, TC39
- **E. Dean Tribble** — CEO, AMIX architect, ex-Microsoft (acquired earlier company "Agorics" — note the trailing 's', distinct from "Agoric")
- **Brian Warner** — engineering lead, Tahoe-LAFS, Foolscap, Firefox Sync security
- **Bill Tulloh** — economist, AMIX market manager, Agorics Project at George Mason

Note: search engines occasionally surface a claim that "Microsoft acquired Agoric on January 1, 2025." This is wrong — it conflates the modern Agoric chain with Tribble's earlier company **Agorics, Inc.**, which Microsoft acquired in the 1990s. The modern entity is independent and has not been acquired.

The seed round (May 2018) included Polychain Capital, Naval Ravikant, and Zcash Company; amount undisclosed at the time ([CryptoNinjas](https://www.cryptoninjas.net/2018/05/21/agoric-completes-seed-round-to-advance-secure-smart-contracts/)). The pitch: an open, JavaScript-based ocap programming layer for smart contracts, with composable contract patterns ("Zoe") and a deterministic VM ("SwingSet") modeled on KeyKOS domains.

## 2019–2021: SwingSet, SES-shim, and pre-mainnet

Through 2019–2021 the team shipped:

- **SES-shim** — production-quality lockdown of the JS environment (eventually used by MetaMask Snaps and LavaMoat)
- **Realms shim** — the precursor TC39 effort
- **PlaygroundVat** — the prototype host that became SwingSet (now archived; see `agoric-labs/PlaygroundVat`)
- **SwingSet** — the deterministic vat host modeled after KeyKOS, originally a separate repo (`Agoric/SwingSet`, since absorbed into the monorepo and marked "MOVED TO MONOREPO")
- **xsnap / XS integration** — the choice of Moddable's XS engine as the on-chain JS engine. Rationale: no JIT, smaller surface, more deterministic, snapshot-friendly. See [Moddable: Hardening the XS JavaScript Engine](https://www.moddable.com/hardening-xs).

The private token sale on November 12, 2021 raised $15.6M at $0.40/token; investors included Polychain, Placeholder, NGC Ventures, Spartan Group, Compound VC, Acrew Capital, Figment, and Chorus.One ([CryptoRank ICO](https://cryptorank.io/ico/agoric)). Total raised across rounds: ~$52.25M; total token supply at TGE on the order of 1B BLD. (`unverified` precise breakdown — Agoric did not publish a single canonical tokenomics doc and figures vary across third-party trackers.)

## 2021-11-01: Mainnet-0

Mainnet-0 launched November 1, 2021 ([CoinDesk](https://www.coindesk.com/business/2021/11/11/smart-contract-platform-agoric-launches-public-chain)). Mainnet-0 was effectively a validator-bootstrap chain — staking, governance, the BLD token — *without* user-deployable smart contracts. Contracts and Zoe came in Mainnet-1.

## 2022-10-27: Mainnet-1

Mainnet-1 launched October 27, 2022 ([Agoric blog](https://agoric.com/blog/announcements/agoric-composable-smart-contract-framework-reaches-mainnet-1-milestone/)). Key milestones:

- First production deployment of the SwingSet kernel
- Zoe smart-contract framework live (offer safety = quid-pro-quo escrow)
- Inter Protocol MVP — the Parity Stability Module (PSM) for IST minting against USDC/USDT
- Smart Wallet integrated with Keplr

## 2023: Mainnet-1B and Endo split

Mainnet-1B (March 2023) added vault-based IST issuance and oracle infrastructure.

The **Endo split** is hard to date precisely from the outside — `endojs/endo` was created as a public monorepo in late 2020, but the migration of core packages (`@agoric/marshal`, `@agoric/eventual-send`, the SES shim) out of `agoric-sdk` and into `endo` was a multi-quarter process through 2021–2022 (`unverified` — would need first-commit timestamps for `packages/ses`, `packages/marshal`, `packages/captp` to pin exactly). The motivation, per the README, is that Endo packages serve clients beyond Agoric — most importantly **MetaMask** (Snaps + LavaMoat) — and so should not live in the chain monorepo.

The relationship is layered: agoric-sdk depends on endo. Endo provides SES, marshal, eventual-send, CapTP, and the compartment/bundle tooling. Agoric-sdk provides SwingSet, Zoe, the cosmic-swingset bridge to the Cosmos SDK, and on-chain vat orchestration.

## 2024: Orchestration pivot

Through 2024 Agoric pivoted hard from "Inter Protocol stablecoin platform" to "cross-chain orchestration platform." Upgrade 16 (July 2024) introduced the Orchestration API — async cross-chain workflows over IBC — and **Fast USDC** (late 2024) became the flagship use case: sub-minute USDC bridging from Ethereum/Base/Optimism/Arbitrum/Polygon via Circle's CCTP and Agoric Orchestration.

A 2024 Cosmos Hub governance arrangement (Proposals 899, 912 — "DCF Agoric Cosmos Hub") allocated initial 4% (rising to 10%) of the Cosmos Community Pool ATOM to be deployed via an Agoric-managed strategy ([Cosmos forum](https://forum.cosmos.network/) — the "DCF" = Decentralized Capital Fund / similar; consult forum for exact mechanism).

## 2025: Inter Protocol sunset

This is the single most important 2025 fact and the spec authors should not gloss over it.

The Decentralized Capital Fund (DCF) and Agoric Engineering Council jointly proposed sunsetting Inter Protocol in April 2025. Signaling proposal opened April 16, 2025; on-chain vote April 28 → May 1, 2025; passed; 60-day wind-down began; **final sunset date: June 30, 2025** ([Agoric forum: Sunset Inter Protocol](https://community.agoric.com/t/sunset-inter-protocol-and-begin-wind-down-process/787)).

Stated reasons (paraphrased from the proposal): market appetite for decentralized stablecoins declined; operational complexity rose; IST consistently failed to fit Orchestration use cases (builders preferred USDC/USDT or native tokens); and Agoric wanted to consolidate around BLD as the core economic token. Inter Protocol's TVL at the time of the sunset proposal was on the order of $100K (per DefiLlama, accessed May 2026: ~$103K).

By any honest read this is a strategic retreat from the original "JS-native CDP stablecoin" vision toward a narrower "JS-native cross-chain orchestrator" identity. BLD price as of May 2026 is ~99.4% below its all-time high of $0.7512 ([CoinGecko](https://www.coingecko.com/en/coins/agoric)).

## 2024–2026: SES standardization status

The **SES proposal at TC39** ([github.com/tc39/proposal-ses](https://github.com/tc39/proposal-ses)) is **Stage 1** as of this writing, with champions Mark S. Miller (Agoric), JF Paradis (Agoric), Caridy Patiño (Salesforce), Patrick Soquet (Moddable), and Bradley Farias (GoDaddy / Node). The proposal repo itself notes it is somewhat outdated and that the active proposal is now **proposal-compartments** (also Stage 1) and a constellation of related proposals (module source, ModuleSource, etc.). Practical adoption has run ahead of standardization: the SES shim is in production at MetaMask and Agoric, and SES 1.9.0 (October 2024) was the most recent release captured in the 2024 blog cadence.

The bet implied by the current standards posture: SES will not become a single normative ECMA proposal; instead, the constellation of compartments + module-source + immutable-array-buffer + non-extensible-applies-to-private will land piecewise, and SES-the-shim remains the integration layer.

## Implications for Myrhiza

- **Lineage continuity is rare and valuable.** Agoric is the only ocap project that has run continuously from AMIX → E → Caja → SES → SwingSet with the same core contributors. Myrhiza inherits a substantial citation graph just by adopting ocap discipline; we should make this lineage explicit in spec front matter rather than reinventing it.
- **The dissertation is the canonical text.** Cite Miller (2006) once in the master spec, not as boilerplate, but as the framing for *why* ocap is the determinism-friendly authority model.
- **The strategic pivot is informative.** Inter Protocol failed not because the tech was wrong — it shipped, it minted, vaults worked — but because the *product* didn't fit the market. Builders preferred USDC. For Myrhiza, this argues we should not bake a single flagship application into the runtime; the runtime should be agnostic to whichever apps win.
- **WASM is the path Agoric did not take.** Agoric chose JS + XS for the engine. We choose WASM Component Model. We get cross-language and a different determinism story (deterministic-by-construction WASM execution + state-apply purity); they get JS familiarity. Track the divergence in `comparisons.md`.

## Sources

- https://en.wikipedia.org/wiki/American_Information_Exchange
- https://en.wikipedia.org/wiki/E_(programming_language)
- https://en.wikipedia.org/wiki/Mark_S._Miller
- https://en.wikipedia.org/wiki/Caja_project
- http://erights.org/talks/thesis/markm-thesis.pdf — Mark S. Miller, "Robust Composition," PhD dissertation, Johns Hopkins University, May 2006 (canonical citation; mirror locations exist if the original is unreachable)
- https://agoric.com/blog/announcements/introduction/ — "Introducing Agoric," May 20, 2018
- https://www.cryptoninjas.net/2018/05/21/agoric-completes-seed-round-to-advance-secure-smart-contracts/
- https://www.coindesk.com/business/2021/11/11/smart-contract-platform-agoric-launches-public-chain/
- https://agoric.com/blog/announcements/agoric-composable-smart-contract-framework-reaches-mainnet-1-milestone/
- https://cryptorank.io/ico/agoric
- https://github.com/tc39/proposal-ses
- https://github.com/tc39/proposal-compartments
- https://github.com/endojs/endo
- https://github.com/Agoric/agoric-sdk
- https://github.com/Agoric/SwingSet
- https://www.moddable.com/hardening-xs
- https://community.agoric.com/t/sunset-inter-protocol-and-begin-wind-down-process/787
- https://defillama.com/protocol/inter-protocol
- https://www.coingecko.com/en/coins/agoric
- https://hardenedjs.org/blog/
