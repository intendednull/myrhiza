**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — Holepunch Inc., Tether funding, and the no-revenue-model precedent

# Holepunch the company

Holepunch Inc. is the corporate vehicle behind Pear, Bare, and the Hypercore stack. It is **not a typical VC-funded startup.** Funding comes from Tether and Bitfinex with an explicit "no monetisation in sight" posture; the company exists to build P2P infrastructure for the open web. For Myrhiza this is a useful precedent — running an OSS-substrate-with-closed-flagship project on patient capital from a single strategic backer is a viable path, and it is the *only* path that has produced consumer mobile P2P at non-trivial scale.

## Founding and structure

Holepunch was launched publicly on **2022-07-25** by Bitfinex, Tether Operations Limited, and the Hypercore project ([Bitfinex media release](https://blog.bitfinex.com/media-releases/bitfinex-tether-hypercore-launch-holepunch/), [Tether announcement](https://tether.io/news/tether-bitfinex-and-hypercore-launch-holepunch-a-platform-for-building-fully-encrypted-peer-to-peer-applications/), [The Block](https://www.theblock.co/post/159423/bitfinex-tether-video-calling-app-keet-holepunch-hypercore)). The corporate entity was structured around the technical work that **Mathias Buus Madsen** had been leading on the Hypercore stack since the Dat project days. The GitHub org `holepunchto` was created earlier — **2021-03-25** (verified `gh api orgs/holepunchto`) — about 16 months before the public launch, suggesting the org was operating in stealth on the runtime work before the funding announcement.

Founders / leadership:

| Person | Role | Background |
|---|---|---|
| Mathias Buus Madsen ([@mafintosh](https://x.com/mafintosh), [LinkedIn](https://ch.linkedin.com/in/mathiasbuus)) | Co-founder and CEO | Core maintainer of Hypercore; previously at the Dat project, funded by Code for Science & Society for P2P-for-science work |
| Paolo Ardoino | Chief Strategy Officer of Holepunch; CEO of Tether and CTO of Bitfinex | The Tether-side leadership link; ensures alignment of Holepunch with Tether's strategic interests |
| Andrew Osheroff | Co-founder | Met Mathias on earlier P2P projects; co-founded Holepunch with him |

The Tether-side leadership embedding (Ardoino as CSO of Holepunch *and* CEO of Tether) is the structural signal that Holepunch is not an at-arm's-length investee — it is a strategic instrument of Tether's broader thesis on decentralised infrastructure.

## The Tether investment

The most-cited concrete number in the press is **$10 million committed to date with up to $50–100 million additional contemplated** ([CryptoNews](https://cryptonews.net/news/blockchain/9573265/), [The Block](https://www.theblock.co/post/159423/bitfinex-tether-video-calling-app-keet-holepunch-hypercore)). The framing in Holepunch's own announcements is softer — "Funding is being provided by Tether and Bitfinex" without specifying an amount ([Tether press release](https://tether.io/news/tether-bitfinex-and-hypercore-launch-holepunch-a-platform-for-building-fully-encrypted-peer-to-peer-applications/)). I could not find a Tether-issued press release that names a dollar figure verbatim — the $10M / $100M figures are reported in third-party crypto press citing Holepunch / Tether sources but not quoted directly.

**Verified facts:**
- Funding source: Tether Operations Limited and Bitfinex ([self-stated](https://tether.io/news/tether-bitfinex-and-hypercore-launch-holepunch-a-platform-for-building-fully-encrypted-peer-to-peer-applications/)).
- No external VC participation; this is not a typical seed/Series-A.
- "Patient capital" posture — Holepunch has been operating for 4+ years (org created 2021-03-25, public launch 2022-07-25, current as of May 2026) without monetisation pressure visible from outside.

**Unverified, treat as approximate:**
- Initial $10M and up-to-$100M follow-on figures cited in crypto press — corroborated across multiple outlets but no primary source naming the figure directly.

For Myrhiza the takeaway is robust regardless of exact dollars: **the funder is one strategic actor with deep pockets and an aligned long-term thesis, not a syndicate of VCs with five-to-seven-year fund clocks.** This is the funding shape that makes "build infrastructure for a decade with no revenue" possible.

## Tether's strategic rationale

Tether is the largest stablecoin issuer and is structurally interested in the existence of payment / communication infrastructure that does not depend on US-jurisdiction-controlled platforms. Paolo Ardoino's framing in the launch press: *"freedom of choice, communication and finances are the lifeblood of the future, and anything that will enhance those freedoms is worth amplifying"* ([Bitfinex media release](https://blog.bitfinex.com/media-releases/bitfinex-tether-hypercore-launch-holepunch/)).

Decoded:

1. **Censorship resistance for finance and communication.** Tether's interest in stablecoins not running on infrastructure that can be turned off by US sanctions or platform decisions is well-documented; Holepunch extends the same logic to the *application* layer. If a P2P-app runtime exists and is widely deployed, Tether-denominated micropayments can ride on it without needing payment-processor agreements.
2. **A non-blockchain alternative for "Web3" rhetoric.** Holepunch is *not* a blockchain project — it integrates Lightning Network for payments but the substrate is Hypercore + Hyperswarm, not a chain. This positions Tether as backing a credible non-blockchain "decentralised internet" thesis, hedging against the various blockchain-based "Web3" projects (most of which Tether's stablecoin ecosystem also touches).
3. **Strategic optionality on payment-rails-for-an-open-web.** If Holepunch's runtime ships at scale and Tether-denominated micropayments are the natural payment primitive on it, Tether captures the payment surface for a hypothetical post-Web2 internet. This is a long-shot bet, but it costs Tether an order of magnitude less than US bank-charter pursuits and pays off asymmetrically if it works.

For Myrhiza: **understand that Holepunch's mission shapes its technical choices.** The "no servers" insistence, the resistance to centralised app-store models, the integration of Lightning into the API surface — all of these are downstream of Tether's strategic interest in censorship-resistant infrastructure. Myrhiza will need its own coherent why-do-we-exist story; "we copied Holepunch" is not it.

## Revenue model

**Holepunch has no public revenue model as of May 2026.** Keet is free with no premium tier, no ads, no in-app purchases. Hypercore / Hyperswarm / Bare / Pear are all permissively licensed open source — no enterprise tier, no commercial license, no support contracts on offer. The Holepunch.to corporate site does not advertise consulting services or hosted infrastructure.

Plausible future monetisation paths (none of which Holepunch has publicly committed to):

- Tether-denominated micropayments inside Pear apps — would generate volume on Tether's stablecoin rather than direct Holepunch revenue.
- Operating a paid push-relay / DHT-bootstrap / hyperswarm-relay tier for high-availability commercial users — feasible, not announced.
- Selling a hosted "publish your Pear app" service on top of `pear stage` / `pear seed` — feasible, not announced.
- Enterprise support contracts for Hypercore-based deployments — not announced.

The honest read: **Holepunch makes no money and is not visibly trying to.** Tether's capital is funding the company indefinitely under what amounts to a subsidy model — the *infrastructure* is the deliverable, not the revenue. This is workable when the funder's strategic interest is in the infrastructure existing, regardless of whether the company that builds it captures revenue.

For Myrhiza this is **the most useful precedent in the prior-art set on the question of "how do you fund a multi-year P2P-runtime project."** The model: find one strategic backer whose thesis is aligned with your existence (not your monetisation), accept that you may never have a normal SaaS revenue line, and build accordingly. The model's main failure mode is "funder loses interest" — Holepunch's risk concentration on Tether is real and should be named as such (see [`./critiques.md`](critiques.md) and [`./open-problems.md`](open-problems.md)).

## Headcount, locations, hiring

Hard data is sparse — Holepunch does not publish org-chart information. Observable signals:

- **Headcount: estimated 30–80 engineers**, based on the volume of GitHub activity across the `holepunchto` org (617 public repos, of which ~50 are actively maintained by 2026; commit cadence on Hypercore / Hyperswarm / Bare / Pear consistent with a team of dozens, not a single-digit team). Cannot be verified more precisely without internal data; the LinkedIn-public profiles tagged "Holepunch" are scattered across Switzerland (Mathias's listed location), Eastern Europe, and Latin America. **Flag: this is an estimate, not a verified number.**
- **No public office locations.** The team is fully remote per multiple interviews and conference talks. No central HQ.
- **Hiring posture.** The Holepunch website does not list a public careers page as of May 2026. Hiring appears to be by direct outreach / network, not by public job postings.
- **Tether-side staffing.** Tether Data (the entity that builds PearPass — see [`./apps.md`](apps.md)) is a separate group within the Tether org; not strictly Holepunch staff but contributing to the Pear ecosystem.

For Myrhiza: a remote-first team of dozens building a P2P runtime is the *minimum* shape that produces shippable consumer mobile output over a 4-year horizon. Smaller teams (Spritely's handful, [`../spritely-ocapn/`](../spritely-ocapn/)) ship excellent research and no flagship app; bigger teams (Holochain Foundation, [`../holochain/`](../holochain/)) ship a runtime and a developer ecosystem but smaller consumer reach. Holepunch sits in the productive middle.

## Other Holepunch products

Holepunch's only consumer product is Keet. PearPass is built by Tether Data, a sibling organisation under the Tether umbrella — not by Holepunch directly. There are no other Holepunch-branded commercial products. The CLI tools (Hyperbeam, Hypershell, Hypertele — see [`./apps.md`](apps.md)) are reference implementations distributed for free.

This means the company's commercial surface is essentially: one free messenger, one open-source runtime, no paid services. The simplicity is intentional and aligned with the "no monetisation" posture above.

## Implications for Myrhiza

1. **Patient capital from a strategic backer is a coherent model — but specify the backer.** Holepunch works because Tether's strategic interest is aligned with Holepunch's mission for a multi-year horizon. Myrhiza should identify (or build for) a comparable backer whose thesis benefits from the runtime existing, regardless of whether the runtime company captures revenue. Without such a backer, the multi-year-no-revenue model is not viable.

2. **OSS-substrate-with-closed-flagship is the precedent.** Hypercore / Hyperswarm / Bare / Pear are open source and permissively licensed; Keet is closed. This split has not deterred third-party developers from building on the substrate (because the substrate is fully usable without Keet), and it gives Holepunch a moat on the consumer surface (the closed Keet code captures whatever Keet-specific UX/notification/call-engine work is hard to replicate). Myrhiza could end up the same — open kernel, open capability layer, open state-machine kit, closed flagship. *Document this option-space explicitly* rather than letting it drift.

3. **No-revenue is acceptable; no-mission is not.** Holepunch has no revenue and a clear mission; the mission is what makes the no-revenue posture coherent. Myrhiza will face the same question: if you don't intend to make money in the next five years, what is the project *for*? "Building cool tech" is not enough — Holepunch's mission is "censorship-resistant infrastructure for finance and communication" and that mission is what justifies the spending.

4. **Funder concentration is a real risk to surface.** Holepunch's existence depends on Tether's continued willingness to fund. If Tether has a regulatory or solvency event, Holepunch loses its capital source. This is a non-zero risk worth naming honestly (see [`./critiques.md`](critiques.md)). Any "we will fund Myrhiza like Holepunch is funded" plan needs to confront this. Diversifying the funder base is an option-space to think through.

5. **Closed flagship requires a viable "rest of the ecosystem can build their own apps" story.** Keet being closed has not stopped Pear from being a real runtime because the substrate primitives (`blind-pairing-core`, `keet-identity-key`, `pear-message`, etc.) that *are* open are sufficient for a third-party developer to build a comparable app from scratch. Myrhiza, if it goes the closed-flagship route, must keep the kernel + capability + state-machine substrate genuinely sufficient for outsiders. The test: can a third-party developer build a Keet-equivalent on top of the open Holepunch stack? Yes — the primitives are all there. They have not because demand is low, not because the substrate is incomplete.

6. **Mission + technical choices must be coherent.** Holepunch's "no servers" / "no central infrastructure" claims are downstream of Tether's "censorship-resistant" interest. Myrhiza's technical choices (capability-mediated host surface, deterministic state-apply, Pear-style sparse content-addressed bundles) need to be downstream of a coherent mission, not a mix-and-match of cool ideas. Pick the mission first; let it constrain the technical decisions.

## See also

- [`./keet-and-apps.md`](keet-and-apps.md) — Keet as the closed flagship on the open substrate
- [`./apps.md`](apps.md) — PearPass and the Tether-Data sibling-org pattern
- [`./pear-runtime.md`](pear-runtime.md) — what Holepunch ships as the runtime
- [`./governance.md`](governance.md) — how decisions get made inside Holepunch (org structure, OSS process)
- [`./history.md`](history.md) — Mathias's pre-Holepunch work (Dat, Code for Science & Society) leading to the Tether deal
- [`./critiques.md`](critiques.md) — funder concentration, no-revenue risks, "no servers" reality
- [`./open-problems.md`](open-problems.md) — what happens if Tether withdraws funding
- [`./lessons.md`](lessons.md) — distilled implications for Myrhiza's funding and corporate structure
- [`../holochain/governance.md`](../holochain/governance.md) — Holochain Foundation as a contrasting non-profit-foundation funding model
- [`../iroh/governance.md`](../iroh/governance.md) — n0 as a contrasting VC-funded model
- [`../spritely-ocapn/`](../spritely-ocapn/) — research-non-profit funding contrast (much smaller, no flagship app)
