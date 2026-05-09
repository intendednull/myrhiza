**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/Endo/SwingSet — corporate structure, funding, tokenomics, IP posture

# Governance

This file documents the corporate, funding, tokenomic, and IP posture of the Agoric / Endo / SwingSet ecosystem. Some figures here are unverifiable from public sources and are marked as such. The posture matters for Myrhiza spec authors evaluating which patterns can be safely borrowed without inheriting governance debt.

For research-grade ocap sibling governance (a much smaller, NLnet-grant-funded operation) see `../spritely-ocapn/governance.md`. For the transport-layer dependency posture see `../iroh/governance.md`.

## Corporate entity

**Agoric Systems Operating Company** is a Delaware C-corp, incorporated **March 16, 2018**. Verified registrations:

- Delaware (incorporation, March 2018)
- California (foreign-corp registration, May 1, 2018; San Francisco principal address)
- Washington (April 16, 2020) and New Jersey (July 29, 2020) branch registrations
- USPTO trademarks for AGORIC (5671163), BLD (90602699), IST (97394826)

Search engines will sometimes confuse the modern company with **Agorics, Inc.** — Dean Tribble's earlier 1990s company, which Microsoft acquired. They are unrelated entities; the modern Agoric has not been acquired.

There is **no separately-incorporated "Agoric Foundation"** evident in public records as of May 2026 (`unverified`). Governance of the on-chain network runs through BLD-staking on-chain proposals; the operating company employs the core engineering team. This is unlike the typical Cosmos pattern (Interchain Foundation + operating co.) and unlike Ethereum (Ethereum Foundation + clients). Closer to a single-entity model.

## Founders and leadership

Per the May 20, 2018 founding post and current [Agoric team page](https://agoric.com/team/):

- **Mark S. Miller** — Chief Scientist. ocap research, E language, Caja, TC39 representative. PhD Johns Hopkins 2006.
- **E. Dean Tribble** — CEO. AMIX architect; ex-Agorics, Microsoft, Sun Labs.
- **Brian Warner** — Engineering Lead (per LinkedIn, still at Agoric as of last verification). Tahoe-LAFS, Foolscap, Mozilla.
- **Bill Tulloh** — Economics. AMIX market manager; Agorics Project at George Mason.

No publicly reported departures of the four founders as of this writing. The team has expanded through engineering hires; no major layoff event has been publicly reported for Agoric specifically (though crypto-industry layoffs were widespread 2023–2025; absence of evidence is not evidence of absence — `unverified` whether headcount changes occurred internally).

## Funding rounds

Sourced from CryptoRank, Crunchbase summaries, and public Agoric blog announcements. Crunchbase / PitchBook detail pages are gated; specific round-by-round amounts below are best-effort and partly approximate.

| Round | Date | Amount | Lead / notable investors |
|---|---|---|---|
| Seed | May 2018 | undisclosed (small) | Polychain Capital, Naval Ravikant, Zcash Company |
| Series A (`unverified`) | 2019–2020 (`unverified`) | part of $6M across two pre-token-sale rounds | undisclosed |
| Token private sale | November 12, 2021 | $15.6M @ $0.40/BLD | Polychain, Placeholder, NGC Ventures, Spartan Group, Compound VC, Acrew Capital, Figment, Chorus.One |
| Total raised through TGE | — | ~$52.25M cumulative | (per CryptoRank) |

Outlier Ventures published a "Why we invested in Agoric" thesis post; their position size is undisclosed.

The November 2021 token sale figure is the most visible and best-cited number. Pre-token equity rounds are partially obscured behind paywalled databases (Crunchbase, PitchBook). Mark these as `unverified` if quoted precisely.

## Tokenomics: BLD and IST

### BLD (validator/governance)

- Native staking token of the Agoric chain. Used for: validator stake, governance voting, transaction fees (post-IST-sunset, BLD is becoming the primary fee token; pre-sunset, IST handled fees in a stable unit).
- Initial supply at TGE: ~1B BLD. Total supply as of Q4 2023: ~1.06B BLD; circulating ~649M (per CryptoRank vesting page).
- Public sale price: $0.80 (after the $0.40 private price); TGE January 5, 2022.
- ATH: $0.7512. Price as of May 2026: ~99.4% below ATH (per CoinGecko). Expect figures to drift; the order of magnitude is the point.

Vesting (public sale tranche, per CryptoRank): 33% unlock 6 months post-sale (~July 1, 2022); remaining 67% linear over 12 months. Earlier private/seed allocations had 1.5 to 2 year monthly vesting with cliffs of 7–11 months.

The `(unverified)` part: Agoric never published a single canonical tokenomics doc with the foundation/team/treasury split. Third-party trackers infer from on-chain data, but the categorical labels (foundation, team, treasury, ecosystem) are not authoritatively defined. **A spec author who needs the precise insider allocation should run on-chain forensics, not trust a tracker.**

### IST (collateralized stablecoin) — sunset

- Inter Stable Token. USD-pegged, multi-collateral CDP. Minted via Inter Protocol's Parity Stability Module (against USDC/USDT) and against IST vaults (against ATOM and other collateral).
- Mainnet on October 27, 2022 (Mainnet-1 launch), with Gauntlet engaged for risk-parameter optimization.
- **Sunset June 30, 2025** (governance proposals voted April 28 → May 1, 2025). 60-day wind-down: vault minting disabled; users return IST and reclaim collateral; reserve liquidated. After June 30, 2025 IST is no longer an active token on the chain.

Implication: the dual-token model (BLD governance, IST stable unit) is no longer in effect. As of mid-2025 onward, BLD is the sole economically active token. This is a material simplification of the original Agoric thesis.

## Foundation / Inter Foundation / DCF

The on-chain stewardship body is the **DCF** ("DCF" appears in governance proposals as the "Decentralized Capital Fund" or similar — the name is partially `unverified` from outside; Agoric forum context confirms the abbreviation). DCF and the Agoric "Engineering Council" (EC) jointly authored the Inter Protocol sunset proposals (#102, #112) and the Cosmos Hub ATOM-deployment proposals (#899, #912). The DCF appears to function as a treasury / strategy committee with on-chain accountability via BLD-vote signaling. There does not appear to be a separately-incorporated 501(c)-style foundation.

This single-entity governance model is operationally efficient but concentrates trust. By contrast, the Ethereum Foundation, Interchain Foundation, and Filecoin Foundation are all separate entities from their respective core engineering companies.

## TC39 / SES / standards work

Mark Miller's TC39 work — including the SES, Compartments, and several adjacent proposals — is funded by Agoric as part of the Chief Scientist role. Co-champions are spread across organizations:

- SES proposal: Mark Miller (Agoric), JF Paradis (Agoric), Caridy Patiño (Salesforce), Patrick Soquet (Moddable), Bradley Farias (GoDaddy / Node)
- Non-extensible-applies-to-private (active 2025–2026): Mark Miller, Shu-yu Guo, Chip Morningstar, Erik Marks

This is one of the rare cases where a small commercial entity has sustained multi-year representation on a major language standards committee. The implicit commitment is large: Miller has been on TC39 in some capacity since the Caja / ES5 era circa 2008. Agoric's continued funding of this work is a public good that benefits MetaMask, Salesforce, and Node equally.

## Endo external contributors

Endo (`endojs/endo`) is, by intent, a multi-organization effort even though most commits flow through Agoric staff. Visible non-Agoric contributors include:

- **MetaMask team** — primary external consumer (Snaps, LavaMoat). Contributors include those who land on `endojs/endo` and `LavaMoat/LavaMoat`. ([Cited collaboration in 2019 conference talk by Dan Finlay](https://x.com/agoric/status/1214354114982727681).)
- **Moddable team** — XS engine integration (Patrick Soquet et al.).
- **Salesforce / Node / GoDaddy / etc.** — via TC39 co-championing rather than direct repo commits (`unverified` in terms of code-level contributions).

Definitive contributor breakdowns require parsing `git shortlog`; the GitHub contributors page was not directly readable from the research environment.

## License and IP posture

- **License: Apache 2.0** for both `agoric-sdk` and `endojs/endo`. (Verified via repo metadata.) Apache 2.0 includes a defensive patent grant to downstream users.
- **No public unilateral patent pledge** beyond Apache 2.0's grant has been identified (`unverified`). Agoric Systems Operating Company is the trademark holder (AGORIC, BLD, IST per USPTO).
- AMIX-era prior art is uncovered: the original AMIX team explicitly chose **not** to patent reputation systems / smart contracts. The spirit appears to carry forward, but spec authors should not rely on spirit — they should rely on Apache 2.0's grant.

The relevant patent landscape for ocap / capability-secure systems is generally permissive (KeyKOS / EROS, E, Cap'n Proto, and Spritely all sit on permissive or AGPL licenses). Concrete blocking patents on ocap technique are not known to the author of this file.

## Implications for Myrhiza

- **Single-entity governance is a risk we should not replicate.** Agoric's no-foundation model is operationally fast but creates a single point of failure: if the operating company ceases, no foundation inherits the chain. Myrhiza's P2P design intrinsically avoids this — there is no chain to govern — but our crate stewardship and spec stewardship still need to think about who carries the work after the original authors. This deserves its own design note before we ship anything load-bearing.
- **Apache 2.0 + explicit patent grant is the right baseline.** Match the Agoric / Endo posture. If we adopt code from `endojs/endo` (e.g., reference implementations of marshal or eventual-send patterns), the license is compatible.
- **Tokenomics are not a runtime concern.** Myrhiza is a runtime, not a chain. We should resist any pressure to bake a token into the runtime layer. Agoric's BLD/IST split — and its half-collapse in 2025 — is a cautionary tale: economic primitives chosen at runtime-design time become governance debt years later.
- **Funding for standards work is a real cost.** If Myrhiza wants to push WASM Component Model semantics or new ocap idioms upstream, that costs sustained engineering time. Plan for it explicitly rather than assuming "the community" will absorb it.
- **The IST sunset is the load-bearing 2025 governance event.** Cite it. The single most honest signal Agoric has emitted about product-market fit; we should learn from it without copying the architectural mistake.

## Sources

- https://opencorporates.com/companies/us_ca/C4148174 — Agoric Systems Operating Company (CA registration)
- https://trademarks.justia.com/876/15/agoric-87615854.html — AGORIC trademark
- https://uspto.report/TM/90602699 — BLD trademark
- https://uspto.report/TM/97394826 — IST trademark
- https://agoric.com/team/ — current team
- https://papers.agoric.com/about/ — about / founders
- https://www.cryptoninjas.net/2018/05/21/agoric-completes-seed-round-to-advance-secure-smart-contracts/
- https://cryptorank.io/ico/agoric — ICO data + investor list
- https://cryptorank.io/price/agoric/vesting — BLD vesting schedule
- https://www.coingecko.com/en/coins/agoric — BLD price history
- https://outlierventures.io/research/why-we-invested-in-agoric/
- https://community.agoric.com/t/sunset-inter-protocol-and-begin-wind-down-process/787 — IST sunset proposal
- https://github.com/tc39/proposal-ses — SES proposal (Stage 1)
- https://github.com/tc39/proposal-compartments — Compartments proposal (Stage 1)
- https://hardenedjs.org/ — SES / Hardened JS site
- https://github.com/endojs/endo — Endo monorepo (Apache-2.0)
- https://github.com/Agoric/agoric-sdk — agoric-sdk monorepo (Apache-2.0)
- https://x.com/agoric/status/1214354114982727681 — MetaMask LavaMoat collaboration reference
