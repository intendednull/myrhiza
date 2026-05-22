**Date:** 2026-05-22
**Status:** active
**Subject:** Bluesky PBC governance, funding, board, leadership, IETF standardization status

# Governance

AT Protocol is steward-led, not foundation-led: **Bluesky Social PBC** designs the spec, operates the reference infrastructure (`plc.directory`, the primary Relay, `bsky.app` AppView), and employs most of the engineers writing the spec drafts. Other organizations contribute (third-party PDSes, alternative AppViews, Germ Network, Brian Newbold's IETF drafts) but the protocol's direction is set by Bluesky PBC.

This is a **single-steward governance model** — closer to TypeScript-under-Microsoft or Rust-pre-Foundation than to ActivityPub-under-W3C. The IETF ATP Working Group, formed November 2025, is the first move toward genuine multi-stakeholder governance, but it's months-old as of this writing and hasn't yet produced ratified specs.

## Corporate structure

**Bluesky Social PBC** is a Delaware Public Benefit Corporation:

- **Incorporated**: October 2021 as an independent company, spinning out of the Twitter-funded "bluesky" working group.
- **Became a benefit corporation**: February 2022.
- **PBC mission statement** (from the certificate of incorporation): *"to develop and drive large-scale adoption of technologies for open and decentralized public conversation."*

PBC status means Bluesky's board is *legally permitted* to weigh public benefit alongside shareholder return. It does **not** mean Bluesky is non-profit, doesn't have shareholders, or can't be acquired. PBCs can be acquired; PBC status can be removed by shareholder vote (subject to PBC procedural requirements).

The 2023 Intercept piece "Is Bluesky Billionaire-Proof?" (cited in [critiques.md](critiques.md)) examined this in detail and concluded: no, it isn't billionaire-proof. The PBC structure provides governance friction, not a structural lock against acquisition. Jay Graber's response was essentially "the protocol's openness is the lock, not the corporate structure" — which puts a lot of weight on the protocol *actually* being usable without Bluesky's infrastructure, a claim that's true on paper and shaky in practice (see [federation.md](federation.md) and [critiques.md](critiques.md)).

## Leadership

| Role | Person | Tenure |
|---|---|---|
| CEO (current, interim) | **Toni Schneider** | Since 2026-03-09 |
| CIO (Chief Innovation Officer) | **Jay Graber** | Since 2026-03-09 (was CEO 2021-2026) |
| Founders (de facto) | Jay Graber (joined August 2021 to lead) + the Twitter-era "bluesky" working group |
| Origin (2019) | Jack Dorsey-era Twitter initiative; Jay Graber hired August 2021 to lead the spinout |

Jay Graber's transition from CEO to Chief Innovation Officer was announced 2026-03-09. The framing was a deliberate-strategic move ("focus on protocol design rather than corporate operations"), not a forced departure. Toni Schneider is interim; a permanent CEO search was announced as ongoing.

**This is a recent transition** (2 months before this corpus date) and the implications for protocol direction haven't fully played out. Watch for: (a) whether the IETF ATP WG sees more Bluesky-employee participation under Graber's CIO role, (b) whether the AppView monoculture of `bsky.app` gets official de-emphasis under a non-founder CEO.

## Funding history

| Round | Date | Amount | Lead investors | Notes |
|---|---|---|---|---|
| Twitter seed | 2019-2021 | undisclosed | Twitter Inc. | Pre-incorporation; Twitter paid Bluesky working group as services income to develop a protocol Twitter could "become a client on" |
| Initial PBC funding | 2022 | undisclosed | Twitter, then Jack Dorsey personally after the Musk acquisition | Post-incorporation; bridged the company from spinout to first market traction |
| Series A | October 2024 | $15M | Blockchain Capital | First institutional round; controversial because of crypto-adjacent investor (Bluesky reiterated "no token, no chain" commitments) |
| Series B | March 2026 | **$100M** | Bain Capital Crypto + Blockchain Capital + others | The major raise; cited "growth from 13M to 43M+ global users" |

The cumulative funding profile is **mid-nine-figures** — substantial but not "Twitter at its peak" scale. Bluesky has stated the funding is for protocol development, scaling infrastructure, and product (`bsky.app`), with no token launch planned. The "no token" commitment is explicit and repeated in every funding announcement.

**The crypto-investor optics** are worth flagging: both Series A and Series B were led/co-led by crypto-aligned funds. The investor mix is a reasonable proxy for what kind of pressure Bluesky will face on monetization — token sales, NFT integrations, paid features, etc. So far Bluesky has resisted; the structural question is "for how long, under what acquisition or IPO pressure."

## Board

Bluesky PBC does not publish a detailed board composition. Known board members as of public filings:

- **Jay Graber** (CIO, formerly CEO) — board member.
- **Jeremie Miller** — board member; XMPP founder; vocal advocate for open protocols.
- **Mike Masnick** — board member; Techdirt founder; cited for moderation/free-speech expertise.
- Various investor representatives from Series A and Series B (composition not publicly enumerated).

The PBC structure requires the board to consider the public-benefit mission in decisions, but the legal teeth on that requirement are thin. In practice, the board operates like a venture-backed company board.

## IETF standardization

The **ATP Working Group** was formed at IETF 124 (Montreal, November 2025) following a Birds-of-a-Feather (BoF) session. Charter approved; mailing list `atp@ietf.org`; documents under `ietf-wg-atp` GitHub org.

Initial scope, per the kickoff blog post:

> *"Daniel Holmgren and Bryan Newbold are expecting to split up the existing draft-holmgren-at-repository text into two separate draft documents: one for the repository data structure (MST), and another for synchronization mechanisms (firehose)."*

What's in scope for IETF standardization:

- **Repository data structure** (MST + commit format) — `draft-ietf-atp-repository-data-structure`
- **Sync protocol** (firehose / subscribeRepos) — `draft-ietf-atp-sync`
- Probably **XRPC** in a future draft

What's explicitly **not** in scope (as of charter):

- `did:plc` itself (Bluesky-operated, not appropriate for IETF standardization without operator-neutrality)
- Lexicon (broader scope than IETF typically takes; W3C-adjacent territory)
- `bsky.app` application layer (out of scope by design — IETF standardizes protocols, not products)

Next major IETF meetings: **IETF 126 in Vienna, July 18-24, 2026**; **IETF 127 in San Francisco, November 14-20, 2026**. First WG drafts targeted for early-2026 publication.

**Honest assessment**: IETF standardization is genuinely useful for credibility and locking in protocol stability, but it's **slow** (years, not quarters) and **scope-limited** (won't address the centralization questions Bluesky's critics raise). Expect IETF ratification to provide more transit-level credibility (`did:plc` resolution, repository sync) and less ecosystem-level credibility (who runs the Relays, who owns the AppView).

## Reference implementations

| Implementation | Language | Maintained by | Status |
|---|---|---|---|
| `bluesky-social/atproto` | TypeScript | Bluesky PBC | Reference SDK + server impl |
| `bluesky-social/indigo` | Go | Bluesky PBC | Relay + PDS (`bigsky` legacy name) |
| `atrium` | Rust | Community (Yuki Sugyan-led) | Client + tooling; well-maintained |
| `atproto-py` | Python | Community | Active client SDK |

The TypeScript reference is dual-licensed **MIT + Apache-2.0** (verified at `github.com/bluesky-social/atproto`). The Go Indigo repo same. The community implementations are generally MIT or Apache-2.0.

## What this prior art tells Myrhiza

On governance specifically:

- **Single-steward + PBC structure** is the deployed answer to "we need to ship a protocol that nobody else has shipped before, but we also want some governance friction against bad-actor founder-pressure." It works for now. It's not billionaire-proof.
- **IETF standardization** is genuinely useful for protocol-level credibility but doesn't address ecosystem-level concerns. Myrhiza-equivalent move would be standardizing the kernel ABI surface; the apps/UI/governance layer wouldn't need IETF.
- **PBC + venture-backed** is a specific governance shape that biases toward growth-at-some-cost; Myrhiza's design bet against this is "no company, no funding, all peer-symmetric" — which means Myrhiza needs a different sustainability story (open-source maintainer convention, foundation, ?).

The interesting governance lesson is **the IETF transition path**. Bluesky operated single-steward for ~4 years, built deployment evidence, then went to IETF for ratification. The order matters: deployment-first, standardization-second. For Myrhiza this is the canonical pattern — ship the runtime, gather evidence, *then* think about external ratification (CNCF? W3C? IETF?). Pre-standardizing produces specs nobody implements.

## Sources

- Bluesky FAQ + company info: <https://bsky.social/about/faq>
- Bluesky Wikipedia: <https://en.wikipedia.org/wiki/Bluesky>
- Jay Graber Wikipedia: <https://en.wikipedia.org/wiki/Jay_Graber>
- "Is Bluesky Billionaire-Proof?" (Intercept 2023): <https://theintercept.com/2023/06/01/bluesky-owner-twitter-elon-musk/>
- Series B announcement (2026-03): <https://bsky.social/about/blog/03-19-2026-series-b>
- IETF ATP WG kickoff: <https://atproto.com/blog/kicking-off-the-atp-working-group>
- Newsweek "Who is Jay Graber" profile: <https://www.newsweek.com/who-jay-graber-bluesky-ceo-1988215>
