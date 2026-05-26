**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — third-party critiques and honest assessments

# Critiques & honest assessments

The substantive third-party critiques of libp2p, mostly from HN, Reddit, blog posts, and competing projects' framing. Verbatim quotes where preserved. Soft-pedals nothing — the value of this file is hearing the unflattering observations clearly.

The dominant critique theme is **complexity-vs-effectiveness**. libp2p ships breadth (many transports, many protocols, many configuration knobs) at the cost of: configuration burden, latency overhead from negotiation chains, fragmented documentation, and a real hole-punching success-rate ceiling. The iroh team has codified this critique most clearly; this file presents both their version and others'.

## The iroh-team critique (the most-cited)

The iroh team's framing has become the canonical "Why not libp2p?" position. From [Comparing iroh & libp2p](https://www.iroh.computer/blog/comparing-iroh-and-libp2p) (Jan 2024):

> *"Libp2p is built to keep its reliance on central points of failure at an absolute minimum, which comes at the cost of effectiveness. Iroh is built to maximize effectiveness, which comes at the cost of a little centralization."*

> *"Most p2p projects end up defaulting into a boil-the-ocean stance where they try to ship one of everything: a DHT, transports, pubsub, RPC. Sometime last year we realized it just wouldn't be possible to ship all this stuff with the team we had, so we picked the transport layer, and are focused on integrating with other projects."*

Specific technical critiques from the same post:

> *"Libp2p's hole punching success rate caps at around 70%."*

> *"With libp2p, your ability to connect to a specific given peer is much more dependent on the network conditions between you and that peer. This can be very frustrating if you're building an app that expects to, say, send a message from one friend to another."*

> *"Libp2p's extensive configurability, while powerful, can be a double-edged sword [requiring] a deep understanding of the framework and its numerous modules, which can lead to a steep learning curve and potential misconfigurations."*

**Honest assessment:**

- The 70% hole-punching number is libp2p's own published [field data](https://blog.libp2p.io/2022-01-20-libp2p-hole-punching/). Iroh's critique is sourced, not invented.
- The "boil-the-ocean" critique is true — libp2p ships DHT, gossipsub, transports, identify, AutoNAT, AutoRelay, DCUtR, Circuit Relay, rendezvous, mDNS. That breadth costs configuration and documentation.
- The "centralization tradeoff" framing is **the team's own framing**. Iroh openly acknowledges they trade some pure-P2P-ness for relay-mediated reliability; libp2p partisans see this as a step backward.

**Where iroh's critique is fair:** the multistream-select latency, the multi-transport configuration burden, and the hole-punching success rate are all real. A simple "two-friends-want-to-chat" app *is* harder to ship on libp2p than on iroh.

**Where iroh's critique is marketing:** the framing implies libp2p is unable to do what iroh does. Actually, libp2p has supported relay-with-direct-upgrade since Circuit Relay v2 + DCUtR in 2021. The difference is libp2p's defaults aren't tuned for "always relay first, race direct" — but the primitive is there.

## Configurability complaint (the recurrent HN refrain)

The most common HN comment pattern when libp2p comes up:

> *"libp2p has a hundred knobs and the defaults aren't always what you want. It took us weeks to figure out the right config for our use case."*

This is real. The libp2p config surface in any implementation has dozens of fields:

- Which transports (`tcp`, `quic`, `ws`, `wss`, `webrtc`, `webrtc-direct`, `webtransport`).
- Which muxers (`yamux`, optionally `mplex`).
- Which encrypters (`noise`, `tls`).
- Which protocols (kad, gossipsub, identify, ping, autonat, autorelay, dcutr, ...).
- Per-protocol tuning (gossipsub D/D_lo/D_hi/heartbeat, kad bucket size, identify timeout, ...).
- Connection limits.
- Memory limits.
- Connection-manager pruning policy.

For a simple "ping a peer over QUIC" app, you set ~10–20 config fields. For a production app, more. Compare iroh: `iroh::Endpoint::builder().bind().await?` is enough for a working stack.

The libp2p docs ([docs.libp2p.io](https://docs.libp2p.io)) are *good*, but the surface they cover is large. The activation energy to write your first useful libp2p app is real.

## Documentation fragmentation

Related to the above. libp2p docs live in **five places**:

1. [docs.libp2p.io](https://docs.libp2p.io) — top-level concepts + tutorials.
2. [libp2p/specs](https://github.com/libp2p/specs) — protocol specifications.
3. Per-implementation docs — [docs.rs](https://docs.rs/libp2p) for rust, [pkg.go.dev](https://pkg.go.dev/github.com/libp2p/go-libp2p) for go, etc.
4. Per-implementation example folders ([rust-libp2p/examples/](https://github.com/libp2p/rust-libp2p/tree/master/examples), [universal-connectivity](https://github.com/libp2p/universal-connectivity)).
5. Discussion forum + GitHub Issues archive for resolved-but-undocumented edge cases.

For any non-trivial question, you typically need to read 2–3 of these. The fragmentation is partly inherent to a multi-impl project (each impl has its own APIs), partly a documentation-coverage gap that gets called out regularly in HN threads.

## Latency overhead from multistream-select

Documented in [`architecture.md`](architecture.md): for TCP-Noise-yamux, opening the first application stream costs **~3 sequential RTTs** for multistream-select negotiation alone. On a residential 30ms-RTT connection, that's ~90ms of overhead before any application traffic.

The libp2p team is aware. multistream-select v2 has been proposed for years. The pragmatic answer is "use QUIC" — but TCP-Noise-yamux remains the fallback transport in every libp2p implementation, with the 3-RTT cost.

**HN critique (paraphrased, multiple threads):**

> *"libp2p is slow to open a stream. Three nested handshakes is too many. They should have shipped multistream-select v2 by now."*

This is fair. The fix is design-complete; it just hasn't shipped.

## "Spec is incomplete"

Verbatim from [libp2p/specs README](https://github.com/libp2p/specs):

> *"The specifications for libp2p are currently incomplete, and we are working to address this by revising existing specs to ensure correctness and writing new specifications to detail currently unspecified parts of libp2p."*

This is honest self-disclosure. Some core protocols (multistream-select v2, some muxer details, the QUIC NAT-traversal extension, certain DCUtR edge cases) are implementation-defined.

**The spec-is-incomplete critique cuts both ways:** libp2p has *more spec coverage than iroh* (which has no public wire spec at all) but *less than it claims*. The 3A Recommendation badges hide the gap between "the protocol is specified" and "every edge case is specified" — for some protocols there's slippage.

## DHT performance at scale

Documented in [`discovery.md`](discovery.md). Cold provider-record lookups in the IPFS DHT take 10–60 seconds in production. This is the canonical "IPFS is slow" complaint, and the team has invested in mitigations:

- [Accelerated DHT client](https://blog.ipfs.io/2023-09-13-accelerated-dht-client/) — aggressive caching + parallelisation.
- [Hydra Booster](https://github.com/libp2p/hydra-booster) — purpose-built indexer peers. Now archived.

Quoted from the iroh team ([HN 39033100 — rklaehn](https://news.ycombinator.com/item?id=39033100)):

> *"IPNS is that the performance is… not great… to put it politely, so it is not really a useful primitive."*

The honest read: Kademlia at IPFS scale (25k+ peers, billions of records) has fundamental cost-of-correctness issues. Provider lookups are inherently O(log N · k · roundtrips). Optimizing the constants helps but doesn't change the asymptotics.

## "Why is there no libp2p browser-to-browser browser kernel?"

A real critique: **libp2p browser support is via libraries that browsers consume, not via a built-in browser feature**. Every "libp2p in a browser" deployment is a JS library shipping in the page. The cost: page-load time (js-libp2p is multi-MB after bundling) + memory + the JS-engine performance ceiling.

For Myrhiza this isn't a problem (we ship WASM Component kernels, not JS libraries). But it's a real criticism of libp2p's browser story for typical web apps.

## "Spec-vs-implementation drift"

Several specs are 2A Candidate Recommendation but only one implementation has shipped them at production quality. Examples:

- **WebRTC** (2A) — js-libp2p has production WebRTC; rust-libp2p has alpha; go-libp2p has WebRTC-Direct only.
- **gossipsub v1.2 IDONTWANT** (1A Working Draft) — implemented in go/rust/js/nim, missing in jvm and cpp.
- **WebTransport** — js-libp2p has it browser-side; rust has webtransport-websys (client-only); go has server-side.

The spec lifecycle stages help (they're explicit about what's experimental), but a Myrhiza spec author considering "I'll just use libp2p's WebRTC" needs to know that the rust implementation is alpha-only as of 2026-05.

## Security advisory volume

Modest historical volume (see [`governance.md`](governance.md) §"Security advisories"). The honest assessment:

- **No CVE-grade gossipsub-specific exploit since v1.1's 2021 deployment.** This is unusual — at Ethereum's scale, an exploit would be highly visible.
- **Several multistream-select and noise length-prefix issues** in 2022, all patched.
- **Resource-exhaustion classes** keep recurring as new transports / muxers ship; the team responds with limits + tuning.

This is a healthy security posture for a stack at this scale. The number of public security advisories is *low*, which can be interpreted as "well-audited" or "under-reported"; the Eth2 production exposure makes "under-reported" implausible.

## "iroh is more pure P2P than libp2p"

The pushback to iroh's centralization critique. Stated by libp2p partisans:

> *"iroh's default deployment depends on n0-operated relays for first-hop connectivity. libp2p doesn't require any operator-run infrastructure to bootstrap; you can dial a peer directly if you have its multiaddr."*

This is true. libp2p's "pure P2P" claim is honest in the technical sense (the stack doesn't require any centralized server to function), even if 70% hole-punching success means many users will still need relays in practice.

The iroh team's counter: "yes, but in practice everyone uses central bootstrap nodes (`bootstrap.libp2p.io`, IPFS bootstrap peers) anyway, and the difference between 'optional infrastructure' and 'required infrastructure' is operational, not architectural."

Both positions are defensible. The honest take: **neither stack is fully decentralized in production deployment patterns** — both rely on some infrastructure that someone has to operate. The difference is who, and what the metadata leak shape looks like.

## "libp2p / Protocol Labs maintenance velocity is slowing"

Real signal. Documented in [`governance.md`](governance.md) and [`history.md`](history.md). The 2024 PL restructuring + the 11-month rust-libp2p release gap are objective facts. Whether they portend long-term decline or are temporary turbulence is unknowable today.

The honest framing: **libp2p has the most diverse stewardship of any P2P stack** (six implementations, multiple non-PL stewards), so even significant PL turbulence wouldn't kill the project. But Protocol Labs is the spec authority; if PL's investment in the network team continues to thin, spec evolution slows. The downstream impact on, say, gossipsub v2 or multistream-select v2 is real.

## Net assessment

libp2p is **good but heavy**. The complaints are real:

- Configuration burden — real, mitigated by sensible defaults but never eliminated.
- multistream-select latency — real, only fixable by switching to QUIC (which the team recommends).
- Hole-punching ceiling — real, ~70% in field conditions, falls back to relay otherwise.
- DHT performance — real, mitigated by accelerated-client + indexer work.
- Spec incompleteness — disclosed by the team; better than iroh's zero-spec state; worse than IETF-spec ecosystems.
- Maintenance velocity post-2024 — slowing in some areas, healthy in others.

The pattern: **none of these are disqualifying for libp2p as a P2P stack**; all of them are reasons a new project (like iroh, or Myrhiza) might choose a narrower scope and simpler shape. The decision between libp2p and iroh is not "which is correct" but "which workload does the breadth-vs-simplicity tradeoff serve."

For Myrhiza: we chose iroh's tradeoff. This file documents libp2p's choices honestly so a future spec author auditing our choice can read the original arguments in their primary form.

## Sources

- [iroh — Comparing iroh & libp2p (Jan 5, 2024)](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
- [iroh — A new direction for iroh (Feb 17, 2023)](https://www.iroh.computer/blog/a-new-direction-for-iroh)
- [libp2p — Hole punching in libp2p (Jan 20, 2022)](https://blog.libp2p.io/2022-01-20-libp2p-hole-punching/)
- [libp2p — Accelerated DHT client (Sep 13, 2023)](https://blog.ipfs.io/2023-09-13-accelerated-dht-client/)
- [HN 39033100 — IPNS performance (rklaehn)](https://news.ycombinator.com/item?id=39033100)
- [HN 39027630 — Iroh comparison to IPFS](https://news.ycombinator.com/item?id=39027630)
- [HN 33376205 — Iroh: A New Implementation of IPFS (Nov 2022)](https://news.ycombinator.com/item?id=33376205)
- [libp2p/specs README — spec incompleteness disclosure](https://github.com/libp2p/specs)
- [docs.libp2p.io](https://docs.libp2p.io/)
- [iroh — critiques (sibling doc)](../iroh/critiques.md)
