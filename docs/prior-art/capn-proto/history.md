**Date:** 2026-05-22
**Status:** active
**Subject:** Chronological history of the Cap'n Proto / Cap'n Web / Workers RPC lineage, from CapTP (1997) through Cap'n Web (2025)

# History

The CapTP-shaped production lineage is a 28-year arc. The two intellectual roots are Mark Miller's CapTP (mid-1990s) and Kenton Varda's Protocol Buffers work at Google (2005-2013). They converged in 2013 with Cap'n Proto, then took a further decade to ship at hyperscale.

## Pre-history (1997-2012)

- **1997** — Mark Miller and Norm Hardy's [**E programming language**](http://www.erights.org/) ships a working capability-secure distributed object system. **CapTP** (Capability Transport Protocol) is E's wire protocol for distributed capability passing, including the four-table import/export/question/answer architecture and promise pipelining.
- **2008-2013** — Kenton Varda is the lead maintainer of **Protocol Buffers** at Google. He develops opinions about Protobuf's weaknesses: forced encode/decode step, no capability semantics, schema-evolution rules that mostly work but with corner cases.
- **Early 2013** — Varda leaves Google. Begins work on what becomes Cap'n Proto.

## Cap'n Proto: 0.1 to 0.6 (2013-2017)

- **2013-04-01** — **Cap'n Proto 0.1** ships. Initial public release. Zero-copy wire format + schema language. RPC not yet implemented.
- **2013-08-12** — **0.2**: compiler rewritten from Haskell to C++11.
- **2013-09-04** — **0.3**: first non-C++ language support via Jason Paryani's Python bindings.
- **2013-12-12** — **0.4**: promise pipelining RPC system ships. This is the headline feature CapTP-via-Cap'n-Proto becomes about.
- **2014-01-08** — **Sandstorm Development Group, Inc.** is incorporated. The Sandstorm platform begins development, using Cap'n Proto for grain-to-grain RPC.
- **2014-12-15** — **Cap'n Proto 0.5**: generics, C# and Java bindings released.
- **2015-03-02 / 2015-03-05** — First security disclosures: integer overflow / underflow in pointer validation; CPU amplification vulnerability. Disclosed transparently on the news page.
- **2017-03-13** — **Sandstorm team acqui-hired by Cloudflare**. Per the announcement post: *"Sandstorm will no longer be our full-time jobs."* The full team — Varda, Jade Wang, and others — moves to Cloudflare to work on Workers. Sandstorm itself transitions to part-time community-supported maintenance.
- **2017-05-01** — **Cap'n Proto 0.6**: first major release in 2.5 years. Windows/VS support, JSON converter, HTTP library, security hardening. Announcement notes the Sandstorm-to-Cloudflare transition.

## The Cloudflare Workers years (2017-2023)

This is the period where Cap'n Proto becomes load-bearing infrastructure at hyperscale, even though the public release cadence slows.

- **2018-08-28** — **Cap'n Proto 0.7**: full Windows / Visual Studio support. KJ HTTP library leveraged extensively in production at Cloudflare Workers.
- **2019** — Cloudflare Workers begin shipping Durable Objects (announced earlier, GA'd around this period). Internal usage of Cap'n Proto RPC grows substantially.
- **2020-04-23** — **Cap'n Proto 0.8**: multi-stream flow control, HTTP-over-Cap'n-Proto protocol, KJ fiber support.
- **2021-08-14** — **Cap'n Proto 0.9**: announcement notes *"Cloudflare Workers project now heavily uses Cap'n Proto RPC for Durable Objects communication."* This is the moment the production-CapTP claim becomes load-bearing.
- **2022-06-03** — **Cap'n Proto 0.10**: minor bug-fix release. The implicit message: the 0.x line is stable and being polished, not redesigned.
- **2022-09-27** — Cloudflare open-sources **`workerd`** as Apache-2.0. The Cap'n Proto RPC implementation behind Workers becomes inspectable.
- **2022-11-30** — **CVE-2022-46149**: list-of-pointers OOB read in C++ ref impl. Disclosed and patched.
- **Late 2022** — Ian "zenhack" Denhardt begins **Tempest**, a from-scratch Sandstorm rewrite in Go. Contributions to the original Sandstorm slow.
- **Mid-2023** — Ian Denhardt dies in an accident. Tempest stalls; sandstorm-io project momentum continues to decline. The `go-capnp` Cap'n Proto Go bindings, which Ian co-maintained, lose one of their key contributors.

## Cap'n Proto 1.0 LTS (2023)

- **2023-07-28** — **Cap'n Proto 1.0** ships. The announcement title: *"It's been a little over ten years since the first release of Cap'n Proto, on April 1, 2013."* The 1.0 designation is explicitly an **LTS** ("long-term support") commitment: `master` becomes the 1.0 LTS branch; v2 development moves to a new `v2` branch.
- The same announcement frames the v2 plans: C++20/C++23 with coroutines required; major breaking changes to the KJ toolkit and C++ API; *"motivated by experience building Cloudflare Workers runtime, `workerd`."* The serialization format and RPC protocol are **not** changing — the breakage is in C++ ergonomics only.
- Cloudflare's institutional position is formalized in the FAQ: *"The Cloudflare Workers team are now the primary developers and maintainers of Cap'n Proto's primary C++ implementation."*

## Workers RPC public launch (2024)

- **2024-04-05** — Cloudflare announces **Workers RPC** in [*"Use one RPC system for browser-to-server, server-to-server, and worker-to-worker calls"*](https://blog.cloudflare.com/javascript-native-rpc/). The canonical statement: *"Under the hood, it is built on Cap'n Proto."* JS-native Proxy-based API; same-thread when possible, Cap'n Proto wire when crossing the network; Service Bindings + Durable Objects deployment shapes.
- Workers RPC is *not* a new Cap'n Proto version — it's a JavaScript binding to Cap'n Proto RPC, exposed inside workerd. The schema is `worker-interface.capnp`, in the workerd repository, Apache-2.0.

## Cap'n Web public launch (2025)

- **2024-01-14** — **Sandstorm community handoff**. Varda transfers ownership to Sandstorm Community under Open Source Collective, led by Jacob "ocdtrekkie" Weisz. *"I gave up pushing monthly releases, since there seemed to be no point: no code changes had been made."*
- **2025-06-08** — `github.com/cloudflare/capnweb` repo created (private). Development begins.
- **2025-09-12** — First public npm publish: `capnweb@0.0.1`.
- **2025-09-21** — `capnweb@0.1.0` launch tag.
- **2025-09-22** — Cloudflare announces **Cap'n Web** in [*"Cap'n Web: a new RPC system for browsers and web servers"*](https://blog.cloudflare.com/capnweb-javascript-rpc-library/). Authors: Varda + Steve Faulkner. *"A spiritual sibling to Cap'n Proto."* MIT, JSON-based, no schemas, "highly experimental." Initial production user: Wrangler's "remote bindings" feature.

## The current state (2026-05)

- `capnp` Rust crate: v0.25.4 (2026-04-12). Monthly cadence. 11.5M lifetime downloads.
- `capnp-rpc` Rust crate: v0.25.1 (2026-04-29).
- `capnweb` npm: v0.8.0 (2026-05-11). Roughly monthly minors since 0.1.0 launch.
- Cap'n Proto C++ ref impl: 1.x LTS on `master` (since 2023-07-28); v2 development on `v2` branch (default branch on GitHub).
- Sandstorm: community-maintained, sandstorm.org-owned, dependency-update mode. Tempest (Go rewrite) stalled.
- Workers RPC: deployed at hyperscale on Cloudflare Workers + Durable Objects. Same-account scope. Production for >2 years.
- Cap'n Web: experimental, single confirmed Cloudflare production user (Wrangler), 3,813 stars, ~8 months old.

## The strategic narrative

The story this timeline tells: **the production ocap-RPC idea took 28 years (1997-2025) to ship to the mass-deployed Web tier, and it required all three of (a) zero-copy binary format as the entry wedge, (b) one corporate steward willing to invest for a decade, (c) one *different* wire format (Cap'n Web) for the browser tier**. None of (a), (b), or (c) was sufficient alone. The E-language ambition of "everyone writes ocap-secure distributed code" was achieved only at the Cloudflare-internal scope; pushing it into browsers required a second protocol (Cap'n Web), and pushing it cross-trust-boundary remains an open problem.

Spritely is trying to do all of this in two-three additional decades; the Cloudflare branch shipped (some of) it in one.

## Implications for Myrhiza

- **The 10-to-25-year horizon is real for ocap-RPC adoption.** Plan accordingly; don't expect the Cloudflare-velocity for Myrhiza's adoption curve.
- **Single corporate steward is the determining factor.** Cap'n Proto exists in its current state because Cloudflare paid for it for 10 years. Myrhiza needs an equivalent steward (foundation, corporate sponsor, sustainable services-business) or it will drift to maintenance mode in 3-5 years like Sandstorm did.
- **One protocol per trust tier.** Workers RPC and Cap'n Web are two protocols solving the *same problem at different trust boundaries*. The lesson is not "have one protocol everywhere" but "have one ocap semantic everywhere, swap wire format at trust boundaries." Myrhiza's intra-peer vs cross-peer split is the same shape.
- **The LTS-branch + dev-branch pattern works.** Cap'n Proto 1.0 LTS on `master` + 2.0 on `v2` lets long-running deployments stay on a stable target while fast-moving work continues. Adopt this for Myrhiza's runtime; do not collapse to a single rolling main.

## Sources

- [capnproto.org/news/](https://capnproto.org/news/) — release-by-release history
- [Cap'n Proto 1.0 LTS announcement (2023-07-28)](https://capnproto.org/news/) — ten-year retrospective + v2 framing
- [sandstorm.io/news/2017-03-13-joining-cloudflare](https://sandstorm.io/news/2017-03-13-joining-cloudflare) — acqui-hire announcement
- [sandstorm.io/news/2024-01-14-move-to-sandstorm-org](https://sandstorm.io/news/2024-01-14-move-to-sandstorm-org) — Varda's hand-off
- [blog.cloudflare.com/javascript-native-rpc/](https://blog.cloudflare.com/javascript-native-rpc/) — Workers RPC announcement (2024-04-05)
- [blog.cloudflare.com/capnweb-javascript-rpc-library/](https://blog.cloudflare.com/capnweb-javascript-rpc-library/) — Cap'n Web announcement (2025-09-22)
- [github.com/cloudflare/workerd](https://github.com/cloudflare/workerd) — workerd open-sourced 2022-09-27
- [github.com/cloudflare/capnweb](https://github.com/cloudflare/capnweb)
- [erights.org](http://www.erights.org/) — E and CapTP root source
