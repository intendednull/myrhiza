**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric — production deployments and the honest adoption picture

# Apps and downstream users

This file is the reality-check sibling of [chain.md](./chain.md) and [contracts.md](./contracts.md). The Agoric chain runs. Inter Protocol issues IST. None of those numbers are large. **The real-world impact of this technology stack is overwhelmingly through Endo-via-MetaMask, not Agoric-the-chain.** That fact should shape how we read the prior art.

## Inter Protocol

The flagship application Agoric the company built. A multi-collateral stablecoin protocol structurally similar to MakerDAO, native to the Agoric chain.

### What it is

- **IST** ("Inter Stable Token") — an over-collateralized stablecoin pegged to USD. Minted against collateral (originally ATOM, later other Cosmos assets) via vaults, or 1:1 against IBC-USDC via the Parity Stability Module (PSM).
- **BLD** — Agoric's staking and governance token. Inter Protocol's *governance* is by BLD holders; BLD is also one of the collateral types. The two tokens are intertwined.
- **PSM** — first contract launched on mainnet-1 (October 27, 2022). Allows 1:1 mint/burn between IBC-USDC and IST. This was the seed liquidity mechanism.
- **Vaults** — Maker-style CDPs. Lock collateral, mint IST, pay stability fees on close. Liquidations through a Dutch-auction mechanism Agoric calls the "Inter Auction." Vaults UI lives at [app.inter.trade](https://app.inter.trade).

### The numbers (verified May 2026)

- **IST market cap: ~$1.4M** (CoinGecko: ~$1.01/IST, ~1.4M circulating).
- **IST all-time-high price**: $1.31 on Nov 2, 2023 (CoinMarketCap). Note that IST is *supposed* to peg at $1.00 — a $1.31 print indicates a thin, dislocated market, not a feature.
- **TVL on DefiLlama**: ~$103,000 across the Agoric chain. (DefiLlama, "[Inter Protocol](https://defillama.com/protocol/inter-protocol)".)
- **BLD market cap: ~$2.8M** (CoinGecko, May 2026). Down dramatically from ICO valuation.

These numbers are small enough that the protocol is, in practical DeFi terms, dormant. Vaults still exist, the app still loads, IST still trades — but the protocol is not a meaningful piece of the stablecoin landscape. By comparison, the next-smallest stablecoins DeFiLlama tracks at the major-protocol tier are in the $10M–$100M range; the leaders (USDT, USDC, USDS) are $5B+. (DL News, [State of DeFi 2025](https://www.dlnews.com/research/internal/state-of-defi-2025/).)

### Why it didn't grow

This is editorial — not in a primary source — but reading the public record:

- Launched into the **Cosmos DeFi winter** (late 2022). UST (Terra) had collapsed in May 2022; the entire Cosmos stablecoin thesis was under suspicion.
- Permissionless contract installation never landed (still gated by governance core-eval as of May 2026; see [chain.md](./chain.md)). So nobody else built apps that needed IST.
- Agoric the company appears to have **shifted focus toward orchestration / cross-chain primitives** (`@agoric/orchestration`, the Ymax product line — see the agoric-sdk release tags from 2026, which are nearly all `ymax-*` and `itest*` orchestration releases, not Inter Protocol releases).

The honest read: Inter Protocol is the proof-of-concept that mainnet-1 works. It does work. It is not the killer app.

## MetaMask Snaps — the actual production deployment of Hardened JS

This is the part of the Agoric/Endo story that has hundreds of millions of dollars depending on it.

### The integration

MetaMask Snaps are sandboxed JavaScript extensions to the MetaMask wallet. They run in an isolated execution environment built on **SES** (Secure ECMAScript), which is the npm package `ses` published by the Endo project.

The architecture, paraphrased from MetaMask's [snap execution environment docs](https://docs.metamask.io/snaps/learn/about-snaps/execution-environment/) and audit reports:

- Each Snap runs in an **SES Compartment** — a JavaScript sandbox with no ambient authority. The Compartment has access only to a curated `globalThis` and to capabilities MetaMask explicitly endows.
- MetaMask calls **`lockdown()`** at startup to freeze JavaScript primordials (Object, Array, etc.) so a malicious Snap cannot poison shared prototypes.
- Snaps reach the host (the user's wallet, the Ethereum provider, key material) only through **explicit endowments** declared in their manifest — `endowment:ethereum-provider`, `endowment:cronjob`, etc.

This is exactly the ocap discipline from [contracts.md](./contracts.md), applied to wallet plugins instead of smart contracts. **The largest deployment of Hardened JavaScript in production runs in MetaMask, not on Agoric.**

### Verified package usage

From inspecting `package.json` files in [github.com/MetaMask/snaps](https://github.com/MetaMask/snaps) (May 2026):

- **`ses`** — `^1.15.0` is pinned in `snaps-execution-environments`, `snaps-utils`, and elsewhere. `ses` v2.0.0 was published April 2026 but Snaps had not yet upgraded.
- **`@lavamoat/lavatube`**, **`@lavamoat/webpack`**, **`@lavamoat/allow-scripts`**, **`lavamoat`** — LavaMoat is the build-time + runtime supply-chain hardening layer, separate from but in the same ecosystem.

Notably, the MetaMask Snaps monorepo does **not** depend on the `@endo/*` scoped packages directly — only on the lower-level `ses` package. This is a deliberate choice: `@endo/*` packages have a faster release cadence and broader API surface; `ses` is the stable, audit-friendly substrate. (It's the same reason the Hardened JS proposal at TC39 keys off `ses`, not `@endo/lockdown`.)

### Timeline

- **September 12, 2023** — MetaMask Snaps Open Beta launches in MetaMask Extension v11.0+. ([metamask.io/news/two-exciting-updates-to-metamask-snaps](https://metamask.io/news/two-exciting-updates-to-metamask-snaps), [Snaps in MetaMask Stable announcement](https://metamask.io/news/snaps-in-metamask-stable-and-where-we-go-from-here))
- Allowlist-restricted at first (~30 audited Snaps).
- Through 2024 and 2025 the directory expanded; by May 2026 it includes a substantial set of non-EVM chain integrations and account-abstraction Snaps. ([snaps.metamask.io](https://snaps.metamask.io/))

MetaMask itself has on the order of tens of millions of monthly active users (the exact figure varies by source; MetaMask's own blog posts have cited ~30M MAU during the 2024 timeframe, *unverified* against an independent source for May 2026). Whatever the exact number, **the SES sandbox in MetaMask is by far the largest user-facing deployment of object-capability JavaScript in production.**

## LavaMoat

[LavaMoat](https://github.com/LavaMoat/LavaMoat) is a separate-but-adjacent project: a JS supply-chain hardening toolchain built on SES Compartments. Maintained by the MetaMask team with input from the Agoric / Endo crowd, supported (per project README) by ConsenSys and Agoric.

LavaMoat protects at three points:
- **Install time** — `@lavamoat/allow-scripts` blocks unauthorized npm `postinstall` scripts.
- **Build time** — webpack / browserify plugins emit per-package SES compartments with policy files declaring what each transitive dependency may do.
- **Runtime** — `lavamoat` (Node) wraps each module in a Compartment with a JSON policy specifying which globals and which other packages it may access.

This is what protects MetaMask itself (the wallet, not the Snaps) against compromised npm dependencies. After the December 2023 Ledger ConnectKit npm supply-chain attack, MetaMask published [a post explaining how LavaMoat blocked the same class of attack](https://metamask.io/news/lavamoat-and-the-ledger-software-supply-chain-attack).

For Myrhiza this matters as evidence: SES Compartments **are deployable in performance-sensitive production code paths** (a wallet extension's hot path), not just as a smart-contract VM curiosity.

## Other Endo / SES adopters

A short, honest list:

- **Moddable / XS** — Moddable's XS JavaScript engine has implemented Compartments for embedded use (see Endo's GitHub issues referencing XS). XS is one of the few JS engines that has shipped a real Compartments implementation outside of V8/SpiderMonkey wrappers. Significance: ocap discipline can survive in tiny embedded targets, not just in V8.
- **TC39 ShadowRealms / Hardened JS proposal** — `ses` is the reference implementation that drove the TC39 proposals. Status as of May 2026: ShadowRealm is at Stage 3; the broader Hardened JS proposal incorporating `lockdown` is moving more slowly. Real-world impact of this work is mostly captured by `ses` already being in production via MetaMask.
- **Spritely Goblins / OCapN** — Goblins is a Scheme/Racket implementation of distributed ocap that interoperates with Endo's CapTP through the OCapN protocol. See `../spritely-ocapn/` in this repository for the sibling write-up. Endo's `@endo/captp` and `@endo/ocapn` (v1.0.0, April 2026) are the JS side of that interop story.
- **Salesforce Lightning Web Components (LWC)** — historically used SES (via the Locker Service) for sandboxing third-party LWC components in the Salesforce platform. Documentation has been less public in recent years; treat this as *(unverified for current state in 2026)*.

## The "ecosystem" honest picture

If you read Agoric's marketing, you see a thriving smart-contract platform with composable JS contracts. If you read DefiLlama and CoinGecko, you see a chain with $103K TVL and a stablecoin under $1.5M market cap. Both are true. The reconciliation:

- **Agoric the chain is small.** It has not won the Cosmos DeFi market, and Cosmos DeFi itself has been overtaken by EVM-on-Cosmos (Berachain, Sei) and by Solana for retail flow.
- **Inter Protocol exists but is not the win.** Treat it as a working reference implementation of "what a Zoe-hosted DeFi app looks like end-to-end," not as a measure of the technology's traction.
- **Endo via MetaMask is the win.** SES is in MetaMask. MetaMask has tens of millions of users. That is real adoption — but it's adoption of *one library* (`ses`), not of the whole Agoric stack.
- **The agoric-sdk roadmap has shifted to orchestration.** Looking at 2026 release tags ([github.com/Agoric/agoric-sdk/releases](https://github.com/Agoric/agoric-sdk/releases)), the dominant project is Ymax (cross-chain yield), not Inter Protocol.

For Myrhiza authors deciding what to borrow: take the runtime ideas (vats, capabilities, marshal, CapTP), recognize the production validation comes from MetaMask's deployment of SES, and don't assume the chain-tier success will materialize.

## Implications for Myrhiza

1. **Production validation lives in `ses` + Compartments, not the chain.** When we cite "this technology is battle-tested," the honest cite is MetaMask Snaps, not Agoric mainnet. That's still a strong cite — MetaMask is one of the largest non-custodial wallets in the world — but it's a different argument.
2. **TVL and chain adoption do not validate the runtime.** The runtime works; what hasn't worked is product-market fit at the chain layer. We are not building a chain, so the failure mode that hit Agoric the company is not directly applicable to us.
3. **The Compartment / endowment pattern is portable to embedding contexts.** MetaMask demonstrates SES Compartments can sandbox third-party JS in a high-stakes user-facing context. The analog for us is sandboxing third-party WASM components — same authority discipline, different VM.
4. **LavaMoat is the model for supply-chain defense.** If Myrhiza ever needs to defend against malicious dependencies in an app's component graph, LavaMoat's "policy file declaring per-package authority" pattern is directly portable to the Component Model imports/exports schema.
5. **Watch the orchestration vs Inter Protocol split.** Agoric's pivot to orchestration suggests the company's bet is now on cross-chain async coordination as the killer app, not on hosting one DeFi protocol. The lesson: a P2P runtime's killer app is unlikely to be a single high-profile flagship; it'll be a substrate that other people compose.

## Sources

- [Inter Protocol on DefiLlama](https://defillama.com/protocol/inter-protocol)
- [IST market data, CoinMarketCap](https://coinmarketcap.com/currencies/inter-stable-token/)
- [BLD market data, CoinGecko](https://www.coingecko.com/en/coins/agoric)
- [Inter Protocol vaults UI](https://app.inter.trade/)
- [Mainnet-1 launch announcement (Oct 27, 2022)](https://agoric.com/blog/announcements/agoric-composable-smart-contract-framework-reaches-mainnet-1-milestone/)
- [MetaMask Snaps documentation](https://docs.metamask.io/snaps/)
- [MetaMask Snaps execution environment docs](https://docs.metamask.io/snaps/learn/about-snaps/execution-environment/)
- [MetaMask Snaps Open Beta announcement (Sept 12, 2023)](https://metamask.io/news/two-exciting-updates-to-metamask-snaps)
- [Snaps in MetaMask Stable post](https://metamask.io/news/snaps-in-metamask-stable-and-where-we-go-from-here)
- [Snaps Directory](https://snaps.metamask.io/)
- [Least Authority — Secure Development of MetaMask Snaps](https://leastauthority.com/blog/secure-development-of-metamask-snaps/)
- [OtterSec — MetaMask Snaps: Playing in the Sand](https://osec.io/blog/2023-11-01-metamask-snaps/)
- [LavaMoat project](https://github.com/LavaMoat/LavaMoat)
- [MetaMask — LavaMoat and the Ledger ConnectKit attack](https://metamask.io/news/lavamoat-and-the-ledger-software-supply-chain-attack)
- [DL News — State of DeFi 2025](https://www.dlnews.com/research/internal/state-of-defi-2025/)
- [agoric-sdk releases (verified May 2026)](https://github.com/Agoric/agoric-sdk/releases)
- [Endo monorepo](https://github.com/endojs/endo)
