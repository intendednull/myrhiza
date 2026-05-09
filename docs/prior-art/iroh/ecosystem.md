**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — wider ecosystem and community

## Number 0 — the company behind iroh

Iroh is built by **number 0** (n0), a startup whose own FAQ describes its funding as *"partly venture capital and partly founder backed"* ([iroh FAQ](https://docs.iroh.computer/about/faq)). The team page at [n0.computer/people](https://www.n0.computer/people/) lists ~10 engineers and operations staff with backgrounds in libp2p, IPFS, and distributed-systems infrastructure — notably Franz Heinzmann, Friedel Ziegelmayer (ex-libp2p / ex-Protocol Labs), Rüdiger Klaehn (content-addressed storage), Floris Bruynooghe (low-level networking), and Philipp Krüger (cryptography, browser support).

Specific funding numbers are not public. n0 generates revenue via **Iroh Services** — managed relay infrastructure and DNS discovery, with a free public tier (the default `dns.iroh.link` and public relays) and paid dedicated deployments. This is the same "open-source library + paid hosted infrastructure" pattern Tailscale rode to scale, and the homepage's customer list (Spacedrive, Delta Chat, Holochain, Paycode, Nous, Shaga, Rave) suggests the relay-hosting product is the commercial wedge.

A 2025 history shift worth noting: n0 originally pitched iroh as "next-generation IPFS" and shipped a `beetle` repo ([n0-computer/beetle](https://github.com/n0-computer/beetle)) that was an IPFS-shaped product. The pivot away from that toward "modular networking library" is documented in [A New Direction for Iroh](https://n0.computer/blog/a-new-direction-for-iroh/). Today nothing in iroh's surface advertises IPFS compatibility — content addressing is in `iroh-blobs` (BLAKE3-keyed) and is not bytewise-compatible with IPFS's CIDs.

## Adjacent libraries from the same team

n0 maintains a small constellation of crates around iroh's core, each independently versioned post-1.0:

- **iroh-blobs** — BLAKE3-keyed content-addressed blob transfer with verified streaming and resume.
- **iroh-gossip** — pub/sub overlay built on iroh connections.
- **iroh-docs** — eventually-consistent multi-writer key-value store backed by `redb`.
- **iroh-router** — protocol multiplexing on a shared endpoint (different ALPNs over one QUIC stack).
- **n0-future** — async-runtime convergence layer that papers over `tokio` vs browser/wasm async, vendoring the futures-* and tokio bits that work in both ([0.32 release post](https://www.iroh.computer/blog/iroh-0-32-0-browser-alpha-qad-and-n0-future)). Quietly the most reusable thing in the suite for non-iroh Rust code that targets browsers.
- **noq** — n0's own QUIC implementation, introduced March 2026 ([noq announcement](https://www.iroh.computer/blog/noq-announcement)). Custom-transports v1 in 0.97 made it a first-class option alongside Quinn.
- **iroh-willow** ([repo](https://github.com/n0-computer/iroh-willow)) — work-in-progress implementation of the [Willow Protocol](https://willowprotocol.org/) for synchronizable, capability-secured digital spaces. The most interesting forward-looking thing in the n0 stack from a Myrhiza perspective: capability-based access control over E2E-encryptable replicated data. Not yet feature-complete or shipped at version 1.

## libp2p, IPFS, and other Rust P2P

The honest framing on libp2p interop: there isn't much. n0 explicitly positions iroh as *simpler than libp2p* ([Comparing iroh and libp2p](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)) — different mental model, different transport stack, different identity assumptions. A community crate [`libp2p-iroh`](https://github.com/rustonbsd/libp2p-iroh) wraps iroh QUIC connections as a libp2p transport for the niche where you want libp2p semantics on iroh's hole-punching plumbing; this is third-party, not n0-blessed.

There is no IPFS compatibility. The shared heritage is people, not protocols.

Willow community presence is mostly through iroh-willow and adjacent academic / Earthstar work. **Earthstar** (a small replicated-document protocol predating Willow; same lineage of authors) does not currently target iroh as transport; it has its own pluggable replicators. There is no formal n0/Earthstar collaboration.

## Conferences and community presence

Verified 2025–2026 events with iroh / n0 presence:

- **FOSDEM 2026** (Brussels, February 1, 2026) — Floris Bruynooghe presented "iroh p2p connections" in the Decentralized Internet and Privacy track ([event page](https://fosdem.org/2026/schedule/event/T9ACNE-iroh_p2p_connections/), [slides](https://fosdem.org/2026/events/attachments/T9ACNE-iroh_p2p_connections/slides/267568/iroh_2p2_bineq6t.pdf)).
- **Pass the SALT 2025** — Holger Krekel (Delta Chat) presented "Usable end-to-end security with Delta Chat and Chatmail" covering the iroh integration ([slides](https://archives.pass-the-salt.org/Pass%20the%20SALT/2025/slides/PTS2025-TALK-11-Delta_Chat.pdf)).
- **Hacker News attention** — significant front-page traction in mid-2025 ([HN thread #44379173](https://news.ycombinator.com/item?id=44379173) — "Iroh: A library to establish direct connection between peers"), with a follow-up [#44706595](https://news.ycombinator.com/item?id=44706595) discussing the dial-by-NodeID model.

The community lives primarily on:

- **Discord** ([invite via Discussion #1856](https://github.com/n0-computer/iroh/discussions/1856)) — the active chat venue. Cited as "the iroh discord" on most release blog posts.
- **GitHub Discussions** ([n0-computer/iroh/discussions](https://github.com/n0-computer/iroh/discussions)) — substantive technical Q&A; the team responds quickly.
- **Mastodon @n0iroh@mastodon.social** — release announcements and ecosystem signal-boosts.
- **X / Twitter @n0computer** — same purpose, lower velocity.
- **Blog at [iroh.computer/blog](https://www.iroh.computer/blog)** — the canonical narrative source. Roughly monthly long-form releases with technical depth (e.g. "QUIC packet rejection in practice," "Running iroh on an ESP32," "iroh for payments").

There is **no Zulip**, no public mailing list, and no IRC. The whole thing is GitHub + Discord + blog.

## Honest assessment of community size — May 2026

This is a **small but high-quality engineering community**. Some calibrating numbers:

- [n0-computer/iroh](https://github.com/n0-computer/iroh) — 8.5k GitHub stars, 406 forks, ~2,464 commits on `main`, v1.0.0-rc.0 released May 7 2026 with 100% of 48 milestone issues closed. Active commit cadence over 2025–2026, dominated by the n0 team; community contributors visible but the ratio is heavily team-weighted.
- **40+ projects** building on iroh per n0's own count ([1.0 roadmap](https://www.iroh.computer/blog/road-to-1-0)).
- **~500k unique nodes** hitting public infrastructure in a 30-day window, "roughly doubles" with private relays counted.

For comparison, libp2p has 6+ language implementations, multiple billion-dollar projects depending on it, and a much larger contributor surface. Iroh in May 2026 is closer in shape to early Tailscale circa 2020 — a small, deeply-engineered Rust library with a handful of marquee customers and a quiet steady output of high-quality blog content.

The risks are obvious: small team, single-language core, vendor of the default relay infrastructure. The mitigations are also clear: BSD-style permissive licensing (Apache-2.0 + MIT dual), self-hostable relays, simple-enough wire protocol that a re-implementation is feasible if needed, and Holochain depending on iroh as transport adds a real second large user (post-Delta Chat) that creates external pressure to keep the protocol stable.

For Myrhiza: the right framing is "we are early to a small, serious community with high engineering bar." Plan to be one of the larger downstreams. Plan to contribute fixes upstream. Plan to maintain enough internal expertise that iroh's bus factor is not Myrhiza's bus factor.

## Implications for Myrhiza

- **n0's commercial model is aligned.** Their incentive is to keep the library free and capable so people pay for hosted infrastructure. This is the same shape as Tailscale and is a healthy alignment for an open-source dependency.
- **Track iroh-willow.** Willow's capability + replication model is closer to Myrhiza's "state-apply + capabilities" worldview than iroh-docs is. If iroh-willow lands a stable surface, it could be the right substrate for Myrhiza's state replication — strictly better than rolling our own.
- **Community is small enough to have direct relationships.** GitHub Discussions and Discord both work; n0 engineers respond. Feature requests for things Myrhiza needs (e.g. additional discovery hooks for state-apply determinism, custom keystore traits) are realistic conversations to have, not shouts into a void.
- **Don't depend on iroh-ffi.** Unmaintained for production (README self-declares "reference example only" since Feb 2025). Maintain Myrhiza's own narrow FFI atop `iroh-c-ffi` and treat it as core infrastructure. See [`mobile-and-wasm.md`](mobile-and-wasm.md).

## Sources

- [iroh FAQ — funding model](https://docs.iroh.computer/about/faq)
- [number 0 team page](https://www.n0.computer/people/)
- [A New Direction for Iroh (IPFS pivot)](https://n0.computer/blog/a-new-direction-for-iroh/)
- [iroh 1.0 roadmap (40+ projects, 500k nodes)](https://www.iroh.computer/blog/road-to-1-0)
- [iroh v1.0.0-rc.0 milestone](https://github.com/n0-computer/iroh/milestone/34)
- [Comparing iroh and libp2p](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
- [iroh-willow repo](https://github.com/n0-computer/iroh-willow)
- [Willow Protocol homepage](https://willowprotocol.org/)
- [libp2p-iroh community crate](https://github.com/rustonbsd/libp2p-iroh)
- [iroh 0.32.0 release (n0-future, browser alpha)](https://www.iroh.computer/blog/iroh-0-32-0-browser-alpha-qad-and-n0-future)
- [noq announcement (Mar 2026)](https://www.iroh.computer/blog/noq-announcement)
- [FOSDEM 2026 — iroh p2p connections](https://fosdem.org/2026/schedule/event/T9ACNE-iroh_p2p_connections/)
- [Pass the SALT 2025 — Delta Chat](https://archives.pass-the-salt.org/Pass%20the%20SALT/2025/slides/PTS2025-TALK-11-Delta_Chat.pdf)
- [Discord invite discussion #1856](https://github.com/n0-computer/iroh/discussions/1856)
- [HN thread #44379173](https://news.ycombinator.com/item?id=44379173)
- [HN thread #44706595](https://news.ycombinator.com/item?id=44706595)
- [iroh blog](https://www.iroh.computer/blog)
- [n0-computer GitHub org](https://github.com/n0-computer)
