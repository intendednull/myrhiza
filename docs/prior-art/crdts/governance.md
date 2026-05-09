**Date:** 2026-05-09
**Status:** active
**Subject:** CRDT library governance — stewardship, funding, bus-factor, license — across Automerge / Yjs / Loro

# Governance comparison: who maintains these libraries, who funds them, what happens if they walk away

The TL;DR for spec authors: bus factors differ by an order of magnitude. Automerge has institutional backing (Ink & Switch + multiple full-time engineers + a research advisor with deep history). Yjs has *one* maintainer doing essentially all the work, funded by GitHub Sponsors. Loro has two co-founders running a small team with unverified funding. License risk is uniform (MIT across the board) — the differentiator is human capital and continuity.

## 1. Automerge / Ink & Switch

**Stewardship**: Maintained inside Ink & Switch, an industrial research lab. Two full-time engineers on the project: Alex Good (`@alexjg`, 447 commits to the core repo) and Orion Henry (`@orionz`, 420 commits, also an Ink & Switch co-founder via Heroku lineage). Martin Kleppmann (`@ept`, 15 commits, Cambridge) is the original creator and continues as advisor. Brooklyn Zelenka and John Mumm are also cited as Ink & Switch engineering staff on Automerge in the Automerge 2.0 announcement. Conrad Irwin (`@ConradIrwin`, 35 commits) and Joseph Heck (`@heckj`, 10 commits, Swift bindings) are notable external contributors.

**Org structure**: Ink & Switch is an independent research lab, not a startup or a non-profit foundation. Director: Peter van Hardenberg (formerly Heroku). The lab funds itself through a mix of corporate research partnerships (Notion on Peritext, Endless OS on Beckett), philanthropic / public funding (NLnet's NGI Zero Core, EU NGI programme), corporate members (Ellipsus, Common Tools), and individual long-term supporters (Adam Wiggins, James Lindenbaum, Avi Bryant). Roughly Heroku-alumni-and-friends in funding profile.

**Automerge-specific funding**: Ink & Switch staff time + open-source sponsorships from Fly.io and Prisma + paid support contracts with GoodNotes and Bowtie + NLnet grant for production-readiness work. This is the most diversified funding picture of the three libraries.

**Decision process**: Discord-based community + GitHub issues + community calls (Automerge community page). No formal RFC repo; design discussion happens in Ink & Switch's research notes (e.g., Patchwork notebook posts) and in the `automerge` GitHub issue tracker. Effectively maintainer dictatorship by Alex Good + Orion Henry, anchored by Kleppmann's research authority.

**Bus-factor assessment**: Healthy. Two full-time engineers, an institutional employer (Ink & Switch), an active research advisor, and several externally-funded support relationships. Loss of any one person would slow the project, not kill it.

## 2. Yjs / Kevin Jahns

**Stewardship**: Single maintainer. Kevin Jahns (`@dmonad`) has 2,017 commits to the main `yjs/yjs` repo. The next-most-active contributor has 8 commits. This is not exaggeration — it's the actual contributor distribution. Yjs is, by every meaningful measure, a one-person project.

**Origin**: Jahns' research at RWTH Aachen University's i5 / Advanced Community Information Systems group. The 2015 Springer paper ("Yjs: A Framework for Near Real-Time P2P Shared Editing on Arbitrary Data Types") is the academic foundation.

**Funding model**: GitHub Sponsors. Jahns has 43 active sponsors and 143 past sponsors with public tiers from $4 to $3,300/month. Notable named sponsors include Liveblocks, GitBook, Roboflow, and Syntax FM. Corporate "support contract" tier is $500/month (weekly video calls, faster issue response). He is also available as a contractor for collaborative-application work. Based in Berlin.

**Decision process**: GitHub issues + the Yjs Community Discourse forum (`discuss.yjs.dev`). No formal RFCs. Jahns is the sole arbiter on protocol changes (visible in the Yjs 14 RC currently being prepared).

**Bus factor: 1.** This is the headline risk. A roughly 22,000-star library that ships in JupyterLab, Proton Docs, Linear, and ~12 editor bindings has a single human being as its load-bearing maintainer. The Rust port `yrs` is in a separate `y-crdt` org that is also maintained by a small group, providing some insulation, but breaking changes to the JS-side reference implementation still flow from Jahns alone.

Mitigations to weigh: (a) yrs as Rust source-of-truth would survive Jahns stepping back, since the protocol spec is documented; (b) Liveblocks, Tiptap, and others have commercial incentives to keep Yjs alive and could absorb maintenance; (c) the protocol is sufficiently mature that "no new releases" is closer to "frozen library" than "abandoned." Still — a Myrhiza commit to Yjs/yrs is implicitly a bet on either Jahns continuing or on the wider ecosystem rallying.

## 3. Loro / loro-dev

**Stewardship**: Two-person founder team plus a small group of contributors. Zixuan Chen (`@zxch3n`, 1,584 commits) is the lead and effectively the BDFL. Leon Zhao (`@Leeeon233`, 350 commits) is co-founder per his GitHub bio ("@loro-dev co-founder"). Zhao's bio also notes he's "building lody.ai" — suggesting the founders are diversifying into adjacent products. After the two co-founders the contributor list drops to GitHub Actions bots and individual external contributors with single-digit commit counts.

**Location**: Founder names (Zixuan Chen, Leon Zhao) and the separate Chinese-language docs repo (`loro-docs-zh`) suggest at least partly China-based or Chinese-diaspora maintainership. Public profiles do not list locations explicitly, so this is inference, not confirmed. (unverified specifics)

**Funding**: Not publicly documented. No GitHub Sponsors page on the org level visible; no Crunchbase / PitchBook entry I could verify for a "Loro" CRDT company specifically (multiple unrelated companies share the name). VC-backed status: unverified. Whether loro-dev pays its founders via consulting, a separate product (`lody.ai`), grants, or savings is not public. (unverified)

**Decision process**: GitHub issues + the loro-dev org's repos. No public RFC process. Effectively founder-led.

**Bus-factor assessment**: Unclear and probably 1–2. zxch3n has roughly 4.5x the commit count of the next contributor; if zxch3n stops, Loro's velocity drops sharply. The active diversification (`lody.ai`, separate loro-mirror / loro-react-native / loro-codemirror sub-projects) cuts both ways: it shows the team is investing in the ecosystem, but also that founder attention is spread across multiple products.

## 4. License risks

All three libraries are MIT-licensed. No copyleft surface, no AGPL viral risk in the *core* libraries.

Caveat: the *commercial servers* around Yjs vary in license. Liveblocks' server-side packages are AGPL-3.0 (client SDKs are Apache-2.0). Hocuspocus is MIT. Y-Sweet is MIT. Self-hosting Liveblocks' server requires sharing modifications. None of this affects a Myrhiza commit to the core CRDT library — it only matters if we adopt a third-party Yjs sync server.

There's also a `yjs/funding.json` file in the Yjs repo, which is the GitHub funding metadata, not a license-risk signal.

## 5. What this means for a Myrhiza commit

Three different bus-factor profiles, three different cost-of-commitment stories:

| Library | Bus factor | Funding | Commercial vendor risk | Recommended posture |
|---|---|---|---|---|
| Automerge | ~3+ (Good, Henry, Kleppmann + Ink & Switch staff) | Diversified: research lab + grants + paid support contracts | None — no commercial vendor in the loop | Lowest commitment risk. If Myrhiza picks Automerge, we're betting on an institution. |
| Yjs (via yrs for our use case) | 1 (Jahns) for JS reference; small team for yrs | GitHub Sponsors, contractor work | Several vendors (Liveblocks, Tiptap, Jamsocket) have commercial skin in the game | Highest interop value, single-human risk. Mitigated by yrs as a stable Rust target and by ecosystem incentives. |
| Loro | 1–2 (zxch3n primary, Zhao secondary) | Unverified | None public | Highest velocity / newest design, lowest production validation, opaque funding. Bet on technical merit + accept that long-term continuity is unproven. |

For a runtime that wants to be alive in 5–10 years, the conservative pick is Automerge on governance grounds. The pragmatic pick on ecosystem and interop grounds is yrs (Rust port of Yjs). Loro is a technical-merit pick that requires accepting governance ambiguity.

Whichever we choose, we should plan for the CRDT layer to be replaceable behind a Myrhiza-internal interface — the Component Model encourages this, and the governance picture across these three libs makes it prudent.

## Sources

- Automerge contributor stats (Alex Good 447, Orion Henry 420, Kleppmann 15) — `gh api repos/automerge/automerge/contributors`
- Automerge 2.0 announcement — https://automerge.org/blog/automerge-2/
- Automerge community page — https://automerge.org/community/
- Ink & Switch supporters — https://www.inkandswitch.com/supporters/
- NLnet funding for Automerge — https://nlnet.nl/project/Automerge-MST/
- Yjs contributor stats (Kevin Jahns 2017, next 8) — `gh api repos/yjs/yjs/contributors`
- Kevin Jahns GitHub profile — https://github.com/dmonad
- Kevin Jahns GitHub Sponsors — https://github.com/sponsors/dmonad (43 current, 143 past sponsors; tiers $4–$3,300/month)
- Yjs original paper (RWTH Aachen) — https://link.springer.com/chapter/10.1007/978-3-319-19890-3_55
- Loro contributor stats (zxch3n 1584, Leeeon233 350) — `gh api repos/loro-dev/loro/contributors`
- zxch3n profile (Zixuan Chen) — https://github.com/zxch3n
- Leeeon233 profile (Leon Zhao, loro-dev co-founder, lody.ai) — https://github.com/Leeeon233
- Loro org repositories — https://github.com/loro-dev
- License files in each repo (MIT, MIT, MIT) — https://github.com/automerge/automerge/blob/main/LICENSE ; https://github.com/yjs/yjs/blob/main/LICENSE ; https://github.com/loro-dev/loro/blob/main/LICENSE
- Liveblocks license posture (AGPL-3.0 server, Apache-2.0 client) — https://liveblocks.io/blog/open-sourcing-the-liveblocks-sync-engine-and-dev-server
