**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — origins, IPFS pivot, release timeline

# History

A chronological narrative of Iroh from its 2022 founding as an IPFS implementation, through its 2023 pivot, the 2024 module spinouts, and the May 2026 1.0 release candidate. Iroh is a load-bearing dependency we will likely commit to heavily for Myrhiza, so this file is biased toward facts a spec author needs to assess credibility, longevity, and trajectory.

## 2022 — Origins as "iroh: a next-gen IPFS implementation"

Iroh started life inside **Number 0** (also written **n0** or **number zero**), an "open R&D organization focused on efficient distributed systems" ([n0.computer](https://n0.computer/)) founded by **Brendan O'Brien** (GitHub: [`b5`](https://github.com/b5)). O'Brien had entered tech through graphic design, then through the **Data Rescue** project (preserving EPA climate datasets in 2016), which connected him to early IPFS developers ([Agentic Tech profile](https://agentictech.substack.com/p/the-network-revolutionary-how-brendan)). The founding team's IPFS heritage is central — most of the early engineers had spent years inside the **Protocol Labs** / IPFS world before leaving to build Iroh.

The first repository was named [`beetle`](https://github.com/n0-computer/beetle) and tagged itself as *"a next-generation implementation of the Interplanetary File System (IPFS) for Cloud & Mobile platforms"*. The plan, as the team summarized it later, was: *"Build a version of IPFS in rust that talks to kubo. Measure. Improve."* ([A new direction for iroh](https://www.iroh.computer/blog/a-new-direction-for-iroh)). Three deployment shapes were envisioned — `iroh cloud` (microservices), `iroh one` (single binary), `iroh mobile` (iOS/Android libs). The HN launch on **November 1, 2022** ([HN 33376205](https://news.ycombinator.com/item?id=33376205)) drew the predictable mix of curiosity and IPFS skepticism.

The early version of `beetle` *did* use [`rust-libp2p`](https://github.com/libp2p/rust-libp2p) to interoperate with kubo ([discussion #1277](https://github.com/n0-computer/iroh/discussions/1277)). This matters: Iroh did not start as a libp2p competitor — it started as a libp2p *consumer* and only diverged after running into specific limitations in production-shaped tests.

## 2023 — The pivot: dropping IPFS interop

After roughly nine months and five releases, the team published [**A new direction for iroh**](https://www.iroh.computer/blog/a-new-direction-for-iroh) on **February 17, 2023** — the canonical pivot post. The framing is empirical, not ideological: their kubo-compatible build needed *"2,000 simultaneous p2p connections"* to reliably retrieve content, and they wanted to get to *"200 peers"* with *"less than 1 message per block"* and *"95%+ reliability."* IPFS-as-specified made those targets unreachable. The post originally read *"breaking from the IPFS spec"* and was edited; the team's clarified position was *"IPFS 2.0"* via *"quantified design and real world adoption."*

In practice the pivot meant: drop bitswap, drop multistream, drop the libp2p stack, and rebuild on top of **QUIC + ALPN + BLAKE3-verified streaming**. The public framing softened over the following year ("we're still IPFS-flavored") but the engineering reality was a clean break. The `beetle` repo was archived in late 2023; the last GitHub push was **2023-11-22** (the formal archive event itself isn't directly exposed by the GitHub API, but no activity has occurred since).

## 2024 — Direct connections, the libp2p comparison, module spinouts

[**Comparing iroh & libp2p**](https://www.iroh.computer/blog/comparing-iroh-and-libp2p) (b5, **January 5, 2024**) was the first formal positioning piece. Verbatim: *"Libp2p is built to keep its reliance on central points of failure at an absolute minimum, which comes at the cost of effectiveness. Iroh is built to maximize effectiveness, which comes at the cost of a little centralization."* This is the definitive sentence for understanding iroh's design tradeoff against libp2p.

Selected 2024 milestones (dates throughout this section are crates.io publication dates from the [crates.io API](https://crates.io/crates/iroh); the corresponding blog post on iroh.computer/blog is sometimes one day later):

- **0.14.0 — Apr 18, 2024.** "Dial the world." Significant DNS / discovery work.
- **0.16.0 — May 13, 2024.** New client API; the start of the slimming-down arc.
- **0.17.0 — May 24, 2024.** Forked Quinn ([Why we forked quinn](https://www.iroh.computer/blog/why-we-forked-quinn), May 20). The fork eventually becomes `noq`.
- **0.20.0 — Jul 9, 2024.** "More ways to connect." NAT-traversal hardening.
- **0.23.0 — Aug 21, 2024.** Node.js bindings ([blog](https://www.iroh.computer/blog/iroh-0-23-welcoming-nodejs-to-the-family)).
- **0.25.0 — Sep 17, 2024.** *Custom protocols for all* — the API shape that survives into 1.0: write your own ALPN, ride iroh's endpoint.
- **0.28.0 — Nov 5, 2024 (yanked; see 0.28.1 same day).** [**Let them have crates**](https://www.iroh.computer/blog/iroh-0-28-let-them-have-crates) — the **module spinout**. `iroh-blobs`, `iroh-docs`, `iroh-gossip` moved to their own repos with their own version numbers. Verbatim: *"We are slimming down the iroh codebase, starting with pulling the protocols that we've developed at number0… into their own crates."* This is the structural decision that makes iroh a *transport library*, not a kitchen sink. (Dates here track crates.io publication; the [blog post](https://www.iroh.computer/blog/iroh-0-28-let-them-have-crates) was published Nov 6.)
- **0.29.0 — Dec 2, 2024.** "Net is the new iroh" — `iroh-net` collapsed into the top-level `iroh` crate; the slimming-down completed. **This is the actual `iroh-net` fold-in event** (some older write-ups misattribute it to the 0.90 "Canary Series" reorg).
- **0.30.0 — Dec 16, 2024.** Continued shrinkage.

Two events outside the version bump rhythm:

- **November 5, 2024 — Global relay outage** ([post-mortem](https://www.iroh.computer/blog/relay-down-a-post-mortem)). ~12 hours of degraded relay service triggered by a memory leak that filled disks with logs. The first public failure of n0-operated infrastructure, honest post-mortem published.
- **October 28, 2024 —** [**Iroh 1.0 Roadmap**](https://www.iroh.computer/blog/road-to-1-0) (b5). Public commitment: 1.0 in *H2 2025*. Verbatim: *"Our plan has nothing left to remove."*

[`iroh-willow`](https://github.com/n0-computer/iroh-willow) — an implementation of the **Willow protocol** for sync — has lived as a separate experimental repo throughout this period.

## 2025 — Approaching 1.0; QAD, multipath, browsers

The 2025 release cadence is roughly monthly. Selected milestones:

- **0.31.0 — Jan 15, 2025.** "Back at fighting fit." Stability + perf.
- **0.32.0 — Feb 4, 2025.** Browsers (alpha), QAD, `n0-future`. **QUIC Address Discovery (QAD)** replaces STUN ([Moving from STUN to QUIC Address Discovery](https://www.iroh.computer/blog/qad), Sep 1, 2025).
- **0.33.0 — Feb 25, 2025.** Browsers, discovery, 0-RTT.
- **0.34.0 — Mar 18, 2025.** **Raw Public Keys in TLS** — node identity fully ed25519, no certificate dance.
- **0.35.0 — May 13, 2025.** *Prepping for 1.0*.
- **0.90.0 — Jun 26, 2025.** [**The Canary Series**](https://www.iroh.computer/blog/iroh-0-90-the-canary-series) — the version-number jump from 0.35 to 0.90 signals "this is the API shape that will become 1.0." Module versions decoupled.
- **0.91.0 — Jul 30, 2025.** Relays moved to standards-compliant protocols ("the last relay break").
- **0.92.0 — Sep 19, 2025.** mDNS improvements.
- **0.93.0 — Oct 9, 2025.** [**Iroh Services**](https://www.iroh.computer/blog/iroh-0-93-iroh-online) launched — the commercial managed-relay/DNS offering. The funding model goes public.
- **0.94.0 — Oct 22, 2025.** "The Endpoint Takeover" — single-endpoint API consolidation.
- **0.95.0 — Nov 5, 2025 (yanked; replaced by 0.95.1 same day).** New relay implementation, error-handling overhaul.
- **0.96.0 — Jan 28, 2026** (blog post Jan 27). [**The QUIC Multipaths to 1.0**](https://www.iroh.computer/blog/iroh-0-96-0-the-quic-multipaths-to-1-0) — multipath QUIC: a single Connection can ride multiple network paths concurrently. Major architectural payoff. Patch `0.96.1` followed Feb 6.
- **0.97.0 — Mar 16, 2026.** [**Custom Transports & noq**](https://www.iroh.computer/blog/iroh-0-97-0-custom-transports-and-noq) — switch to **noq**, n0's own QUIC implementation in Rust ([noq announcement](https://www.iroh.computer/blog/noq-announcement)). Custom transports (Tor included) become first-class. The Quinn fork becomes a separate project.
- **0.98.0 — Apr 17, 2026.** [**Getting back to traversing NATs**](https://www.iroh.computer/blog/iroh-0-98-0-getting-back-to-traversing-nats). Pluggable crypto backends; NAT-traversal regression fixes.

## 2026 — 1.0 release candidate

- **1.0.0-rc.0 — May 7, 2026** ([releases](https://github.com/n0-computer/iroh/releases)). The release candidate landed *the day before* this document was written. As of May 8, 2026, iroh is technically still pre-1.0; the team's roadmap timeline slipped from *H2 2025* to *Q1–Q2 2026* — a roughly two-quarter slip on a public commitment, but well inside the realistic range for a project of this scope.

Holochain shipped iroh as its **default network transport** in `0.6.1-rc` (April 2026), making Holochain the first major P2P framework to bet hard on iroh as a load-bearing dependency. Myrhiza joins this group.

## Why the IPFS heritage matters

The team's prior IPFS / Protocol Labs experience cuts both ways:

- **Credibility:** these are people who have been operating P2P networks at scale for years. The tradeoffs they make ("a little centralization for a lot of effectiveness") are made by engineers who watched libp2p's hole-punching cap out around 70%, not by people guessing.
- **Honest lessons:** the pivot post and the relay post-mortem are both unusually candid for a venture-backed company. The team writes "consensus is impossible" essays ([Feb 21, 2025](https://www.iroh.computer/blog/consensus-is-impossible)) instead of marketing the contrary.
- **Caveat:** the IPFS / Protocol Labs alumni network has produced more than one project that promised more than it delivered. Iroh's pivot itself is partly a quiet repudiation of the IPFS-as-shipped story. A spec author should weigh the engineering credibility (high) against the brand credibility of the lineage (mixed).

## Sources

- [iroh — A new direction for iroh (Feb 17, 2023)](https://www.iroh.computer/blog/a-new-direction-for-iroh)
- [iroh — Comparing iroh & libp2p (Jan 5, 2024)](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
- [iroh — Why we forked Quinn (May 20, 2024)](https://www.iroh.computer/blog/why-we-forked-quinn)
- [iroh — Iroh 1.0 Roadmap (Oct 28, 2024)](https://www.iroh.computer/blog/road-to-1-0)
- [iroh — Let them have crates (Nov 6, 2024)](https://www.iroh.computer/blog/iroh-0-28-let-them-have-crates)
- [iroh — Relay outage post-mortem (Nov 19, 2024)](https://www.iroh.computer/blog/relay-down-a-post-mortem)
- [iroh — Consensus is Impossible (Feb 21, 2025)](https://www.iroh.computer/blog/consensus-is-impossible)
- [iroh — The Canary Series (Jun 27, 2025)](https://www.iroh.computer/blog/iroh-0-90-the-canary-series)
- [iroh — iroh services (Oct 9, 2025)](https://www.iroh.computer/blog/iroh-0-93-iroh-online)
- [iroh — The QUIC Multipaths to 1.0 (Jan 27, 2026)](https://www.iroh.computer/blog/iroh-0-96-0-the-quic-multipaths-to-1-0)
- [iroh — Custom Transports & noq (Mar 16, 2026)](https://www.iroh.computer/blog/iroh-0-97-0-custom-transports-and-noq)
- [iroh — Getting back to traversing NATs (Apr 17, 2026)](https://www.iroh.computer/blog/iroh-0-98-0-getting-back-to-traversing-nats)
- [iroh — Moving from STUN to QUIC Address Discovery (Sep 1, 2025)](https://www.iroh.computer/blog/qad)
- [GitHub — n0-computer/iroh releases](https://github.com/n0-computer/iroh/releases)
- [GitHub — n0-computer/beetle (archived)](https://github.com/n0-computer/beetle)
- [GitHub — iroh discussion #1277 (relation to rust-libp2p)](https://github.com/n0-computer/iroh/discussions/1277)
- [HN 33376205 — Iroh: A New Implementation of IPFS (Nov 2022)](https://news.ycombinator.com/item?id=33376205)
- [Agentic Tech — The Network Revolutionary (Brendan O'Brien profile)](https://agentictech.substack.com/p/the-network-revolutionary-how-brendan)
- [n0.computer — company site](https://n0.computer/)
