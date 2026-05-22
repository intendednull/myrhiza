**Date:** 2026-05-22
**Status:** active
**Subject:** Third-party critiques of AT Protocol — what observers outside Bluesky have said about the architecture

# Critiques

Independent assessments of atproto's architecture and Bluesky's stewardship. Quoted where possible; paraphrased where source material is long. This file exists because the official atproto docs are written by people invested in the protocol's success — a balanced reading needs the critical voices.

## "Is Bluesky Billionaire-Proof?" (Lee Fang, The Intercept, June 2023)

The most-cited public critique of Bluesky's structural claims. Key arguments:

- The PBC structure provides governance friction, not a structural lock against acquisition. Bluesky PBC can in principle be acquired; PBC status can be removed by shareholder vote (subject to PBC procedural requirements).
- The "protocol-not-platform" rhetoric assumes the protocol becomes viable independent of Bluesky's operation. This is true on paper and shaky in practice as long as Bluesky operates the registry, the primary Relay, and the dominant AppView.
- Jay Graber's response (quoted in the article) was substantively *"the protocol's openness is the lock, not the corporate structure."* This puts a lot of weight on the protocol actually being usable without Bluesky's infrastructure — a claim that's defensible in 2023 but has only partially been borne out by 2026 federation reality.

Direct quote (from Bluesky's published response at the time):

> *"We do not want a future where we are the only major operator of the network."*

That's the stated goal. Whether the protocol's design choices serve that goal is the question critics raise. The Intercept piece's answer: *"the design choices make Bluesky's continued centrality the path of least resistance."*

**Status as of 2026**: the centralization concentration figure is still ~99% on Bluesky-operated tiers. The IETF ATP WG (Nov 2025) is a step toward broader stakeholder governance but is months old and hasn't ratified anything.

## "Glass Floor of Digital Sovereignty" (gelbphoenix.de blog)

A technical critique focused on the federation tier hardware barriers. Direct excerpts:

> *"This architectural choice, prioritizing consistency alongside efficiency over full-mesh replication, imposes significant hardware costs (e.g., terabytes of storage) that create high economic barriers to entry, structurally favoring large-scale providers over small, independent operators and centralizing the 'reach' layer despite the decentralized 'speech' layer."*

The "speech vs reach" framing is useful:

- **Speech layer** (PDSes) is genuinely decentralized — anyone can run one.
- **Reach layer** (Relays) is structurally centralized — only well-resourced operators can run one.
- A user has free speech (can self-host) but limited reach (depends on the centralized Relay tier to be heard).

This is sharper than the typical "atproto isn't really federated" critique because it identifies the *specific* tier that creates the centralization and explains *why* — bandwidth and storage costs, not protocol design per se.

**Implications for atproto's "credible exit" claim**: you can exit the PDS tier but not the Relay tier. Your followers still subscribe to AppViews that subscribe to Bluesky's Relay. Exiting the PDS doesn't exit Bluesky's reach-tier influence.

## Wikipedia AT Protocol federation assessment

From the Wikipedia article on AT Protocol (as of early 2026):

> *"The AT Protocol has a federated network architecture, meaning that account data is stored on host servers, as opposed to a peer-to-peer model between end devices."*

This is the careful framing — atproto is *not* P2P, it's a federated client-server protocol. Don't mistake the marketing.

> *"As of early 2026, approximately 99% of users are hosted on Bluesky PBC's infrastructure, leading to practical centralization despite the federated design."*

Cited from third-party analyses.

> *"In January 2026, the charter for the working group tasked with the standardization was published."*

The ATP WG formation is a credibility move that critics view positively (genuine stakeholder governance is possible) but cautiously (years of standardization work ahead; no immediate change in operator concentration).

## Federation critique from atproto community

The atproto-community-wiki and various community-blog posts have explored the federation question from inside the ecosystem. Common themes:

- **Self-hosting works but is socially marginal.** A self-hosted PDS user can do everything; they just don't have anyone to talk to on their own host.
- **AppView diversity is real but small.** Frontpage, Statusphere, Smoke Signal, Whitewind each serve their niches well but none has >1% of the user base.
- **Relay hosting is the actual bottleneck.** The community frequently discusses "how to make Relays cheaper to run" as the structural blocker for genuine federation.

The community is honest about this in a way the official marketing isn't. Read the docs.bsky.app blog posts about non-Bluesky-PDS hosting alongside the atproto-community-wiki for a fuller picture.

## DID:plc as centralization vector (varied sources)

A frequent technical critique: the rotation-key recovery story depends on `plc.directory` honoring the priority-rotation rule. The directory operator can in principle:

- **Refuse to apply a recovery operation** that should be valid.
- **Apply an operation** that shouldn't be valid (e.g., signed by a now-revoked rotation key, or one that exceeds the 72-hour window).
- **Reorder operations** to favor specific outcomes.

Bluesky operates plc.directory in good faith and publishes the audit log. Critics' argument is **not that Bluesky is currently doing any of these** — it's that the trust model assumes good faith and continued operation. There is no cryptographic guarantee that `plc.directory` will continue to behave correctly, and no fallback if it doesn't.

The defense: the audit log is publicly verifiable. A community could in principle detect misbehavior. But detection isn't the same as remediation — if `plc.directory` rewrites your identity in a way you don't want, what's your recourse? The answer is "social pressure on Bluesky" which is not a cryptographic guarantee.

## Comparison-with-IPFS critiques

A line of critique from the IPFS / content-addressed-storage community: atproto's MST is essentially a re-implementation of an IPFS Merkle structure but tied to a single PDS rather than a content-addressable network. Why not use IPFS directly?

Bluesky's published response (paraphrased from various engineering posts):

- **IPFS's general content addressing has latency and discovery issues atproto's curated PDS-Relay model avoids.**
- **The MST's specific tree-shape is tuned for record collections rather than arbitrary DAGs.**
- **CAR format compatibility lets atproto borrow IPFS's tooling without buying into IPFS's network model.**

This is a defensible choice — atproto gets the determinism of IPFS-style content addressing without the open-network discovery problems. The cost is reduced interoperability with IPFS-native tools; CAR files are interchangeable but the broader IPFS ecosystem doesn't directly host atproto repos.

## What this critique landscape means for Myrhiza

A summary of where the critiques land:

1. **Federation diversity isn't structural — it's aspirational.** The Hard parts (Relay tier hardware, registry trust model) are unsolved at the protocol level. Saying "you can federate" and "people actually federate at scale" are different claims; atproto delivers on the first and not yet the second.
2. **PBC governance is friction, not protection.** Don't expect corporate structure to guarantee what protocol design hasn't.
3. **Single-operator registries are a real risk.** The `plc.directory` trust model is "transparent server with good intentions." For Myrhiza, that risk shape is unavailable — there's no Bluesky-PBC-equivalent operator to host it.
4. **Critiques sharpen the design.** The "speech vs reach" framing in particular is a useful lens for evaluating any federation design. Myrhiza-equivalent question: which Myrhiza tier is speech (user-runnable) vs reach (requires substantial resources)? If any tier is reach-shaped, federation diversity will be aspirational regardless of protocol claims.

## Sources

- "Is Bluesky Billionaire-Proof?" (Intercept, June 2023): <https://theintercept.com/2023/06/01/bluesky-owner-twitter-elon-musk/>
- "Glass Floor of Digital Sovereignty": <https://blog.gelbphoenix.de/the-glass-floor-of-digital-sovereignty/>
- AT Protocol Wikipedia (federation section): <https://en.wikipedia.org/wiki/AT_Protocol>
- atproto-community-wiki: <https://atproto.wiki/>
- docs.bsky.app federation posts: <https://docs.bsky.app/docs/advanced-guides/federation-architecture>
- Various community-blog discussions of Relay hosting cost
- IPFS-vs-atproto discussions in `bluesky-social/atproto` issues
