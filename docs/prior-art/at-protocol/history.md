**Date:** 2026-05-22
**Status:** active
**Subject:** AT Protocol chronological history — Twitter origins, ADX, atproto, BGS-to-Relay, Series B, IETF

# History

A timeline of the protocol's evolution from a Twitter initiative to a federated network at ~40M users. The chronology matters because **the protocol's design constraints reflect its history**: it had to bootstrap quickly with off-the-shelf infrastructure, it had to support migration *eventually* (because the first generation was all Bluesky-PBC-hosted), and it had to credibly look like a Twitter-shaped product on day one to attract users.

## 2019 — Origin at Twitter

December 2019: Jack Dorsey, then CEO of Twitter, announces "bluesky" — a small independent team Twitter is funding to develop an open social protocol. The framing (per Dorsey's announcement) is that Twitter would eventually become "a client on" the protocol rather than the operator.

Why Twitter funded this: a mix of reasons that depend on who you ask. Officially: vision for a more open social web. Pragmatically: regulatory pressure on content moderation creating an incentive to push moderation responsibility downstream. Cynically: protocol development as a hedge in case Twitter's economic model collapsed.

The early bluesky team operated as a working group inside Twitter for ~2 years before spinning out.

## 2021 — Spin-out + Jay Graber

**August 2021**: Jay Graber is hired to lead bluesky. She joins from Happening (a calendar/events startup she founded) with prior background in software at Zcash. Her hiring signals the project is going somewhere — until then it had been a research effort with no shipping target.

**October 2021**: Bluesky incorporated as **Bluesky Social** (Delaware C-corp initially, later PBC). Independent from Twitter as of incorporation, though Twitter remained the primary funder via paid services agreement.

## 2022 — ADX

The first protocol draft was called **ADX** — *"Authenticated Data Experiment."* Published in early 2022 as a working sketch of identity + repository + sync. ADX laid down the core ideas:

- DID-based identity (initially `did:web` only; `did:plc` came later).
- Content-addressed repositories with signed commits.
- A federation model with personal data servers + aggregators.

ADX is the conceptual ancestor of atproto. The repo `bluesky-social/adx` was the original codebase; it was archived in late 2022 when the renamed `bluesky-social/atproto` repo started fresh.

**February 2022**: Bluesky becomes a Public Benefit Corporation. The PBC mission statement names "open and decentralized public conversation" as the public benefit.

**April 2022**: Twitter is acquired by Elon Musk. Bluesky was already an independent company by this point. Twitter continues a degree of funding/support through 2022 but the relationship attenuates.

## 2023 — atproto, BGS, the closed beta

**March 2023**: AT Protocol formally announced as the successor to ADX. Repo rebranded to `bluesky-social/atproto`. Key changes from ADX:

- `did:plc` introduced as the default DID method (the rotation-key + signing-key separation lands here — see [identity.md](identity.md)).
- The PDS / Big Graph Service / AppView trio formalized.
- Lexicon schema system formalized with NSID namespacing.

**February-October 2023**: `bsky.app` launches in closed beta. Invite-only. Substantial press attention because Twitter is in chaos.

**November 10, 2023**: **BGS-to-Relay rename**. Discussion #1847 in the atproto repo announces the change. The rationale, verbatim:

> *"BGS as an acronym ('big graph server') has always been a placeholder name, and creates extra confusion and friction for folks learning about the atproto federation architecture."*

The functionality didn't change. The Go implementation directory `bluesky-social/indigo/cmd/bigsky` retained its name for backwards compatibility (a Relay still answers to `bigsky` in the codebase, just not the documentation).

## 2024 — Open registration, federation opening, Series A

**February 6, 2024**: `bsky.app` opens to the public. End of invite-only beta. User growth begins in earnest.

**February 2024**: **Federation opens** in a limited form. Third-party PDSes become possible to operate. The first non-Bluesky PDSes spin up (small operators, hobbyists, organizations testing the model). Account migration becomes possible at this point.

**October 2024**: `plc.directory` reaches **~12 million DIDs registered**. This is the first quantitative milestone for the identity layer at scale.

**October 2024**: **Series A — $15M led by Blockchain Capital.** Bluesky's first institutional funding round outside the Twitter relationship. Crypto-adjacent investor leads, drawing some community concern, addressed by Bluesky's reiterated "no token, no chain" public commitment.

**November 2024**: User growth surge tied to U.S. election; Bluesky goes from ~10M to ~25M registered users in weeks. The Relay infrastructure strains; the firehose dropouts that follow drive the Sync v1.0 → v1.1 design work.

## 2025 — Sync v1.1, federation reality, IETF prep

**May 2025**: New Relay deployment with **Sync v1.1** announced at `relay1.us-west.bsky.network` and `relay1.us-east.bsky.network`. Staged rollout planned for `bsky.network`. Sync v1.1 fixes cursor instability and separates account events from the repo firehose.

**September 2025**: "Enabling Account Migration Back to Bluesky's PDS" blog post — third-party PDS users can now migrate *back* to Bluesky's PDSes, not just away. The bidirectional migration story is now complete.

**October 2025**: Protocol check-in post discussing non-Bluesky PDS hosting improvements, auto-scaling rate limits at the Relay, etc. Tone is "supporting the long tail of independent operators" — acknowledgement that the ecosystem is real but small.

**November 2025**: **IETF 124 in Montreal — ATP WG kicked off** following a BoF session. Charter approved. Initial drafts: Daniel Holmgren + Bryan Newbold begin splitting `draft-holmgren-at-repository` into separate repository-data-structure and sync drafts.

## 2026 — Series B, leadership transition, Sync v1.1 cutover, Germ DM, DDoS

**January 27, 2026**: **Final Sync v1.1 cutover** on the main `bsky.network` Relay endpoint. Consumers warned of WebSocket reconnections and "modest event duplication" during the transition.

**February 18, 2026**: **Germ DM launches** as the first private messenger to launch natively within the Bluesky app. Germ uses MLS (RFC 9420) for E2E encryption and atproto DIDs/handles for identity. Notable because **atproto has no native E2E**; Germ is a third-party overlay.

**March 9, 2026**: **Leadership transition.** Jay Graber steps down as CEO; transitions to Chief Innovation Officer. Toni Schneider takes over as interim CEO. Permanent CEO search announced. Framing is strategic, not forced.

**March 19, 2026**: **Series B announced — $100M.** Bluesky reports growth from 13M to 43M+ global users since prior raise. Bain Capital Crypto + Blockchain Capital among the leads. Reiterated "no token, no chain" commitment.

**April 15-20, 2026**: **DDoS incidents.** Multiple service interruptions across April 16, 17 (morning + afternoon), and 20. The repeated attacks expose the Relay tier as a centralized choke point — when Bluesky's primary Relay goes down, the firehose stops, and downstream AppViews including `bsky.app` lose live data. Mitigations in progress as of corpus date.

**May 2026** (corpus date): ~42.3M registered, ~27.5M MAU (third-party estimate). IETF ATP WG continuing draft work; next major checkpoint IETF 126 in Vienna (July 2026).

## Patterns visible in the history

A few things worth noting for Myrhiza:

1. **Bootstrap order: protocol → AppView → federation.** Bluesky shipped `bsky.app` before federation opened. The "federate eventually" sequence let them iterate on the protocol with one operator before locking in the spec. This is opposite to ActivityPub (federation-first; AppViews-much-later) and similar to how iroh shipped before Quic-Cloud or other independent deployments emerged. For Myrhiza: ship the runtime, ship one app, *then* invite others.
2. **Renames are cheap; semantic changes are expensive.** BGS → Relay was friction-free. Sync v1.0 → v1.1 took ~8 months of telegraphing and a deliberate cutover. Myrhiza should expect the same: vocabulary cleanups are cheap; wire-protocol revisions are major events.
3. **Funding rounds drive infrastructure investment, not protocol changes.** The protocol didn't fundamentally shift after either funding round. Series A enabled scaling the existing protocol; Series B enables scaling further plus IETF work. Money buys operations, not protocol design.
4. **Federation "opens" gradually.** It wasn't a switch flip in early 2024; it was a multi-year gradient from "Bluesky-only" → "Bluesky-and-some-friends" → "long tail of small operators." The story is still "long tail," not "third parties at parity." Watch for whether IETF standardization changes this.
5. **The DDoS resilience question.** April 2026's outages reveal a real structural fragility: the firehose is centralized enough that single-operator outages cascade. This is the kind of thing that produces protocol evolution if it keeps happening, possibly toward more distributed Relays or content-addressed pull rather than firehose push.

## Sources

- Bluesky Wikipedia: <https://en.wikipedia.org/wiki/Bluesky>
- atproto Wikipedia: <https://en.wikipedia.org/wiki/AT_Protocol>
- Jay Graber Wikipedia: <https://en.wikipedia.org/wiki/Jay_Graber>
- BGS-to-Relay rename: <https://github.com/bluesky-social/atproto/discussions/1847>
- Series A announcement (2024-10): historical Bluesky blog
- Series B announcement (2026-03): <https://bsky.social/about/blog/03-19-2026-series-b>
- IETF ATP WG kickoff: <https://atproto.com/blog/kicking-off-the-atp-working-group>
- April 2026 DDoS posts: bsky.social/about/blog (April 16-20, 2026)
- Sync v1.1 announcement: <https://docs.bsky.app/blog/relay-sync-v1.1>
- Germ DM launch: <https://techcrunch.com/2026/02/18/a-startup-called-germ-becomes-the-first-private-messenger-that-launches-directly-from-blueskys-app/>
