**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/Endo/SwingSet — third-party critiques and pain reports

# Critiques

This file collects honest, attributed third-party critiques of Agoric, Endo, SwingSet, and Hardened JS. The point is not to pile on. The point is: **Myrhiza inherits whatever ocap discipline tax Agoric is paying, and we should know what users of the existing system actually complain about.**

The single most load-bearing critique in 2025–2026 is from Agoric's own community: **Inter Protocol failed in the market and is being shut down on June 30, 2025.** The DCF and Engineering Council wrote it themselves. We treat that as a primary source.

Where I could not find a quotable verbatim source for a category of critique, I say so. Several searches for HN / Reddit / Twitter critique returned thin results — Agoric is a niche-enough project that public critique is sparse, which itself is a signal.

## On-record self-critique: Inter Protocol sunset

The clearest, best-cited critical analysis of Agoric's product-market fit comes from Agoric's own governance. From the [Sunset Inter Protocol forum proposal](https://community.agoric.com/t/sunset-inter-protocol-and-begin-wind-down-process/787) (April 16, 2025; on-chain vote passed May 1, 2025):

> "The market appetite for decentralized stablecoins has declined, while operational complexity and costs have risen."

> "IST hasn't integrated well with Agoric's Orchestration layer. Whenever the Agoric team had offered to put IST front and center in an Orchestration use case, IST turned out to be an obstacle. Builders favored either mainstream stablecoins like USDC/USDT, or their own native tokens."

— DCF + Engineering Council, official sunset rationale (paraphrased from forum digest; verify exact wording in the linked proposal text)

Community responses on the forum (verbatim where retrievable, paraphrased where the forum was paginated and individual posts were not directly captured by the research environment):

> "Shutting it down now, when the crypto market—especially the Cosmos ecosystem—is deeply in the red, might not be ideal."

— forum participant, April 2025 (paraphrase of [community.agoric.com](https://community.agoric.com/t/the-future-of-inter-protocol/775) thread, post not individually attributed in retrieved excerpt)

> "[Was] the initial thinking [behind IST] just flawed?"

— forum participant questioning the original IST design, [community.agoric.com](https://community.agoric.com/t/the-future-of-inter-protocol/775) (paraphrase)

The on-chain TVL signal supports the self-critique: Inter Protocol TVL on DefiLlama is **~$103K** as of May 2026 ([defillama.com/protocol/inter-protocol](https://defillama.com/protocol/inter-protocol)), ranking it #90 among CDP protocols and effectively a rounding error in Cosmos DeFi. For comparison, MakerDAO's DAI sits in the $4–5B range. IST never crossed $50M peak TVL.

This is the most important critique on the page. **The team was honest about it.** That honesty is a signal of healthy engineering culture, but the underlying fact — three years of Inter Protocol shipped, peaked low, never gained traction — is not something to soften.

## "JS-on-chain is too slow" / vat-replay overhead

Concrete cited concerns from [Agoric/agoric-sdk issue #511](https://github.com/Agoric/agoric-sdk/issues/511):

> "The current approach simulates heap snapshots by recording all inbound messages, which are then replayed one at a time at restart time, taking O(N) space and time, where N is the number of messages that have been sent."

— `agoric-sdk#511`, issue body (engineering self-acknowledgment; pre-xsnap)

This is the engineering team flagging the cost themselves. xsnap (XS-engine heap snapshots) was the answer; it lands restart cost at O(active set) instead of O(transcript length), but this is a structural cost that lives in the architecture forever — vats *do* replay on restart, and you *do* pay XS interpretation cost vs JIT-V8 cost.

From [agoric-sdk#1127](https://github.com/Agoric/agoric-sdk/issues/1127) ("vat-container options: XS, Worker, WASM, etc"):

> "SwingSet and most of Agoric's code is written in JavaScript, which involves a complicated process that starts up the JavaScript environment, starts a SwingSet instance, and then connects through Go to the Cosmos SDK modules, the consensus algorithm in Tendermint, and back again."

— Agoric engineering, on the JS-Go-Tendermint sandwich's overhead

I did not find a verbatim third-party benchmark publicly comparing Agoric throughput to CosmWasm or EVM. This is partly because Agoric's contract throughput has not been a top-of-mind sales pitch — Agoric's positioning is "ocap and composability," not "TPS leader." The honest read: vat-replay costs real CPU and real memory; XS is slower than V8 by ~2-5× on most workloads (`unverified` exact ratio); and the JS-Go-Tendermint pipeline adds latency. Whether any of this *matters* depends on workload — for slow, low-throughput contracts (loans, vaults), it doesn't.

## "ocap discipline has a steep learning curve"

From a 2019 InfoQ interview with Mark Miller, *paraphrasing* Miller himself: ocap requires "a different way of thinking" about authority that does not match the dominant access-control-list mental model. Miller has been honest about this in talks for over a decade.

I did not find a strong, verbatim, recently-dated third-party complaint on this axis from Reddit or Hacker News. The closest signal: search results on "Reddit Agoric developer experience" returned official content and zero discussion threads. **Agoric is small enough that the public critique surface is small.** This is itself a signal — the community is small, the user pool is small, and learning-curve frustration that would be loud for a larger project is just absent here.

A reasonable proxy is the broader "ocap is unfamiliar" critique that recurs around Spritely Goblins on HN ([news.ycombinator.com/item?id=26665387](https://news.ycombinator.com/item?id=26665387) and [news.ycombinator.com/item?id=38295524](https://news.ycombinator.com/item?id=38295524)). The pattern in those threads: people who already know ocap love it; people coming from Solidity / Ethereum see it as exotic; the documentation gap between "I know capabilities" and "I can write a contract" is real.

## "Agoric the chain has weak adoption"

Concrete signals (cited):

- BLD price ~99.4% below ATH ([CoinGecko](https://www.coingecko.com/en/coins/agoric)). ATH $0.7512 → current ~$0.005 range.
- Inter Protocol TVL ~$103K (DefiLlama, May 2026).
- Agoric's pivot to Orchestration in 2024 implicitly acknowledges that the chain's standalone DeFi ecosystem did not materialize. Fast USDC is the flagship 2024–2025 use case, and Fast USDC is intentionally cross-chain — i.e., the value is being delivered to other chains' users, not Agoric-chain users.
- Validator set / staking participation is healthy in absolute terms but small in $-staked terms relative to other Cosmos chains (`unverified` precise ranking; consult [stakingrewards.com](https://docs.stakingrewards.com/staking-data/methodologies/cosmos-ecosystem-srb/agoric-srb)).

A direct quote I could not retrieve from Reddit / HN. But the price action and the Inter sunset are not subtle signals.

## "MetaMask Snaps too restrictive / hard to write"

This critique exists in the developer-tooling space around Snaps. Snaps run under SES inside MetaMask; Snap authors hit the friction of:

- **Permission model is opaque to users.** Per [MetaMask docs on Snap permissions](https://docs.metamask.io/snaps/reference/permissions/), each Snap declares permissions like `endowment:rpc`, `snap_getBip44Entropy`, `endowment:cronjob` — but users see prompts for these without much context. Quote from MetaMask's own docs (verbatim): *"Snaps should only ask for access to specific functionalities or data necessary for their operation to minimize the attack surface."* The doc itself acknowledges that authors over-request permissions.

- **SES + dependency graph mismatch.** From the [`ses` npm package docs](https://www.npmjs.com/package/ses): in older transitive dependencies that pull `regenerator-runtime` versions 0.10.5–0.13.7, applications are *incompatible* with SES out of the box, and SES 1.8.0 (Aug 27, 2024) had to add a `legacyRegeneratorRuntimeTaming: 'unsafe-ignore'` opt-out to handle the mismatch. **The fix has "unsafe" in its name** — that's the SES team's own honest signaling.

- **Auditing pain.** From Osec's audit blog [osec.io/blog/2023-11-01-metamask-snaps](https://osec.io/blog/2023-11-01-metamask-snaps), Snap authors hit fingerprinting issues, permission-prompt fatigue, and the difficulty of reasoning about what a Snap can do given its declared permissions plus the JSON-RPC surface MetaMask exposes. (Verify the article's specific claims if you cite them in a downstream spec.)

These are real costs of "lock down JS to make it ocap-safe." MetaMask absorbs them because Snaps without SES would be catastrophic — a Snap that can read `globalThis` reads the whole wallet — but Snap authors absorb the friction of writing for the SES subset.

## "SES too aggressive / breaks libraries"

Direct quotes from the SES ecosystem documentation:

> "Lockdown patches up and hardens the shared intrinsics so they are safe to share with other parties and invulnerable to prototype pollution attacks. Additionally, after calling lockdown(), `new Function.prototype.constructor(code)` throws an error so it cannot evaluate code outside the compartment or access the true `globalThis`."

— [SES guide](https://github.com/endojs/endo/blob/master/packages/ses/docs/guide.md)

> "Certain libraries that make tweaks to the standard built-ins may fail in Hardened JavaScript, and the SES wiki tracks compatibility reports for NPM packages, including potential workarounds."

— [hardenedjs.org](https://hardenedjs.org/)

The compatibility list exists *because there are real incompatibilities*. The most common pattern: a library mutates a primordial prototype (e.g., adds a method to `Array.prototype`); after `lockdown()` the prototype is frozen; the assignment silently or loudly fails; the library breaks. Workarounds exist (`Object.defineProperties`, restructuring), but they require library-author cooperation, and many older libraries are not maintained.

The SES 1.8.0 release post titled *"SES 1.8.0 adds flexibility to Lockdown"* is itself an admission that strict lockdown was breaking too much. The "unsafe-ignore" opt-out for regenerator-runtime is the SES team meeting users halfway.

This is the cost of SES that Myrhiza does *not* inherit. WASM has no shared mutable primordials to freeze.

## "TVL low on Inter Protocol"

Already covered above. Concrete: ~$103K TVL May 2026 (DefiLlama). Sunset June 30, 2025. The most cited quantitative critique on this page.

## On "JS is the future of smart contracts"

The thesis articles (e.g., [thenewstack.io/is-javascript-the-future-of-smart-contracts](https://thenewstack.io/is-javascript-the-future-of-smart-contracts/)) are mostly Agoric-positive. I did not retrieve a strong, dated, verbatim counter-essay from a credible Solidity or Rust author dunking on Agoric specifically. The closest pattern is cultural: Solidity developers don't take JS-on-chain seriously, and Rust developers (CosmWasm, Solana) view Agoric as a side-bet. Absence of a specific quote is itself worth recording — Agoric's tech is not engaged with much in adversarial detail by mainstream smart-contract communities.

## What's *not* being criticized

It is worth naming what *isn't* in the critique pile:

- **No major SES exploit has been publicly documented.** Lockdown's security guarantees have held up in the wild. MetaMask Snaps in production rely on this.
- **No CapTP-level protocol break.** The protocol has evolved (OCapN is harmonizing) but no published exploit of CapTP semantics has surfaced.
- **No catastrophic chain-level outage.** Mainnet has had upgrades and incidents but no high-profile halt that drew sustained press.
- **No ocap-discipline-broke-in-production story.** The thesis that "ocap reduces a class of vulns" is supported by the absence of those vulns in Agoric production.

The critiques are about adoption, ergonomics, and product-market fit. The technology, on its own terms, has held up.

## Implications for Myrhiza

- **The Inter sunset is the lesson.** Build the runtime; do not build a flagship application. If Agoric — with the ocap dream team and seven years of execution — couldn't make a JS-native CDP stablecoin work, the failure was not technical. Don't repeat the architectural mistake of fusing one app to the runtime.
- **Hardened-JS-style friction is not our friction.** SES's "freeze the primordials" tax is the price of bolting confinement onto JS. WASM is born confined. We pay a different tax (curating deterministic imports, surfacing capability handles in Component Model worlds) but it is structurally smaller.
- **Permission UX is hard.** MetaMask Snaps' permission-prompt fatigue is a real signal. If Myrhiza apps declare capabilities and end-users see prompts, we should design those prompts as a first-class concern, not an afterthought. Pair with the user-trust spec when one is written.
- **Niche-project critique is sparse and that's a liability for spec authors.** We won't get a rich corpus of HN takedowns to learn from. We have to be self-critical and rely on the on-record events (sunsets, governance decisions) for honest signal.
- **Vat-replay overhead is a structural cost we don't inherit.** Our state-apply model is "pure function of (prior state, event)" — no transcript replay required because state-apply *is* deterministic by construction. Make sure the spec calls out this delta from Agoric's vat-replay model so future readers don't import the wrong cost model.

## Sources

- https://community.agoric.com/t/sunset-inter-protocol-and-begin-wind-down-process/787 — sunset proposal
- https://community.agoric.com/t/the-future-of-inter-protocol/775 — community discussion thread
- https://defillama.com/protocol/inter-protocol — current TVL
- https://www.coingecko.com/en/coins/agoric — BLD price
- https://github.com/Agoric/agoric-sdk/issues/511 — vat heap snapshot issue
- https://github.com/Agoric/agoric-sdk/issues/1127 — vat-container options
- https://github.com/Agoric/agoric-sdk/issues/2615 — sufficiently-deterministic GC
- https://github.com/endojs/endo/blob/master/packages/ses/docs/guide.md — SES guide
- https://hardenedjs.org/blog/ses-1.8.0/ — SES 1.8.0 (regenerator-runtime fix)
- https://www.npmjs.com/package/ses
- https://docs.metamask.io/snaps/reference/permissions/
- https://osec.io/blog/2023-11-01-metamask-snaps/
- https://news.ycombinator.com/item?id=26665387 — Spritely Goblins thread
- https://news.ycombinator.com/item?id=38295524 — OCapN thread
- https://thenewstack.io/is-javascript-the-future-of-smart-contracts/
- https://docs.stakingrewards.com/staking-data/methodologies/cosmos-ecosystem-srb/agoric-srb
