# Critiques & honest assessments

A consolidation of substantive third-party and internal critiques of Spritely Goblins and OCapN. Spritely is a 501(c)(3) research institute building a multi-decade ocap-distributed-systems vision. The technology is interesting and the people are credible. The honest read is also: nothing has shipped at consumer scale; the language story is fragmented; the ancestor language (E) never crossed the chasm; and most production CapTP traffic in 2026 runs through Cloudflare and Agoric, not Spritely.

## Research-grade adoption

Goblins is described in its own community forum as "very early," with cautionary advice against using it for anything "production-facing" quite yet (community.spritely.institute, 2025). The Goblins demo chat application is named with the explicit caveat "(emphasis on demo)" in the repo description (`spritely/racket-goblin-chat`, Codeberg). The user-facing artifacts — Goblin Chat, Goblinville (Lisp Game Jam 2025 multiplayer demo), Mandy (ActivityPub-on-Goblins), GoblinShare — are all sub-100-user demos, not production deployments.

The closest thing to a "real" deployment Spritely itself names is the GNU Shepherd port: porting Guix's init system to use Goblins. The institute itself frames this prospectively — "this project will constitute the single largest real-world deployment of Spritely code to date" (Spritely Institute NLnet grant announcement, December 2023). That sentence, read straight, says the largest real deployment is *not yet shipped*; it is a grant deliverable.

LWN's February 2024 review is the cleanest outsider summary: Spritely is in "early stages" with "a long way to go," only Goblins among planned ecosystem components actually exists, NAT traversal is unimplemented, and OCapN is "not yet a standard and is subject to change" ([LWN](https://lwn.net/Articles/960912/)). EFF's 2023 framing is similar: "Spritely is worth keeping an eye on" — not "use Spritely today."

The E lineage matters here. CapTP is not new. E (Miller, Bornstein, Crockford, Morningstar, Electric Communities, 1997) demonstrated capability-secure distributed objects 28 years before this writing. Wikipedia's E entry, citing the project's own retrospective: "As an open source language, E never attracted more than 100 users to the community." That is the comparison group Goblins should be measured against, not blockchain projects with VC marketing budgets.

## Language fragmentation — which dialect is real?

There are at least four implementations of Goblins / CapTP / OCapN, none of which is canonical for everyone:

- **Guile Goblins.** Spritely's primary implementation since the v0.10 unification announcement (Spritely Institute, January 2023). v0.18 ("Sleepy actors") shipped 2025.
- **Racket Goblins.** Maintained for OCapN compatibility, but no longer the primary target. Spritely's own announcement: "the Guile version is considered the canonical version now."
- **Hoot-on-WASM.** Goblins compiled to WASM 3.0 (GC + tail calls) for browser deployment. Released in v0.15.0, January 2025. Requires Wasm GC + tail-call extensions; Safari "is not expected to work properly at this time" (goblin-chat README).
- **Agoric `@endo/captp`.** Independent JavaScript CapTP, used in production by Agoric mainnet smart contracts and MetaMask plugin systems. Compatible-in-principle but not yet wire-compatible with Spritely until OCapN standardization completes.
- **Cap'n Proto RPC (Kenton Varda / Sandstorm / Cloudflare).** A *different* CapTP implementation that predates Spritely, built into Cloudflare Workers RPC and Cap'n Web. Sandstorm's blog: "Cap'n Proto's RPC subsystem is based directly upon E's CapTP protocol."
- **Dart / DObjects / EndoJS.** Three separate implementations are now talking to each other for an OCapN interop test (community.spritely.institute, OCapN interoperability progress thread, 2024–2025).

The honest read: as of 2026 there is no canonical CapTP. There are Spritely's Scheme dialects, Agoric's JS dialect, Cloudflare's C++/JS dialect, and an evolving OCapN draft trying to unify them. Pick one and you may not interop with the others. The OCapN repo itself: "OCapN is still pre-specification, and will likely be the output of examining to what extent the Agoric, Spritely, and potentially Cap'N Proto implementations can be unified" ([github.com/ocapn/ocapn](https://github.com/ocapn/ocapn)).

## Documentation maturity

Goblins documentation is improving — there is a manual, a tutorial, a Persistence chapter, a Debugger reference. But the prerequisites are steep: the canonical implementation is in Guile Scheme, an unfamiliar language to most modern P2P developers; the introductory material assumes Lisp comfort; and key concepts (vat, sturdyref, swissnum, near/far refs, syrup serialization) borrow E's vocabulary, which has no popular treatment outside ERights.org wiki pages from the early 2000s. An outsider trying to write a Goblins app in 2026 has to learn Guile, the Scheme module system, the actor / vat model, CapTP, sturdyref bootstrap, the persistence model, and a netlayer — most of which are documented only in reference style, not in tutorial style.

## Performance

Goblins on Racket / Guile inherits Scheme runtime performance: fine for chat-scale traffic, weak for high-throughput. The institute itself frames performance as a recurring release-by-release improvement: v0.17 introduces "Bloblin," a "lightning-fast persistence store" (Spritely Institute 0.17 release post). The framing — "fastest Goblins ever" — implies the prior baseline was slow.

The Hoot/WASM compilation story is honest about what it costs: Hoot produces "binaries that conform to the Wasm 3.0 specification which features tail calls and heap-allocated reference types with garbage collection." Browser support for Wasm GC + tail calls is recent (Chrome and Firefox both 2024); Safari support is partial. So "Goblins runs in the browser" is technically true and operationally constrained.

There is no published benchmark comparing Goblins CapTP throughput to Cap'n Proto RPC, gRPC, or NATS — three protocols any production candidate would have to beat. Cap'n Proto can do millions of calls per second per core in C++; published Goblins benchmarks do not exist in the same regime.

## Discoverability of objects

OCapN explicitly does not solve discovery. A sturdyref is "an unguessable token so by possessing it you have the authority to send messages to the endpoint it designates" (Spritely Goblins persistence docs). You cannot find a counterparty you have never met; someone must hand you a sturdyref out of band. By design — it is the ocap discipline: introduction precedes invocation. But it means OCapN is not a substitute for a discovery layer (DHT, gossip, registry, search). Spritely's response is the petname / Brux system: human-meaningful names → sturdyrefs, locally bound. That helps usability after introduction; it does not solve "two strangers want to meet."

Combined with no Sybil resistance (CapTP names objects, not principals, and any party can spin up infinite vats with infinite swissnums), OCapN structurally cannot do "find a public service by topic" without an out-of-band registry.

## Scheme dependence

Spritely's flavor of CapTP is Scheme-first. The browser story is Scheme-via-Hoot. The persistence story uses Scheme-native serialization (syrup). Most prior art and most working implementations of object capabilities outside Spritely use JavaScript (Agoric/Endo, Cloudflare Cap'n Web), C++ (Cap'n Proto), or Java (E itself). A Rust developer evaluating Spritely in 2026 will find no Rust binding, no documented Rust port plan, and a primary implementation that requires learning Guile.

The implication: the *protocol* (OCapN) is portable in principle; the *ecosystem* is Scheme-bound in practice. Anyone building a Rust runtime that wants to interop with Spritely will need to implement OCapN themselves from the draft spec, which is not yet stable.

## Mark Miller's E never shipped at scale — is Goblins different?

E shipped a working capability-secure distributed object system in 1997. It influenced JavaScript (via Crockford and Miller on TC39), it influenced Cap'n Proto, it influenced Agoric. As a deployed system used by end users, E never crossed 100 users. The honest critique of Spritely is structural: it is the same idea, by the same intellectual lineage, with mostly the same people, on different host languages, 25+ years later. The substrate is better (Wasm GC, modern crypto, NLnet funding); the open question is whether it now succeeds where E did not.

Adjacent evidence: the *capability-secure RPC* idea has finally crossed into mass production — but via Cloudflare Workers RPC (Cap'n Web), not Spritely. Cloudflare's blog: "Workers RPC is built on Cap'n Proto RPC, which in turn is based on CapTP." That is the lineage shipping. The Spritely flavor remains research-grade.

## Honest project statements

Christine Lemmer-Webber's own talks consistently flag the work as in-progress. The institute's "Distributed System Daemons" announcement frames Shepherd-on-Goblins as the deliverable that will *make* this real-world. The supporter drive language is honest about the multi-decade horizon. There is no "Goblins is production-ready" claim from Spritely itself; the critique is not that they are misleading users, it is that the gap between "interesting research" and "shipping P2P substrate" is not yet closed.

## Adjacent skepticism

HN discussion threads on Goblins releases are mostly supportive. The substantive criticism that does appear:

- **Distributed GC fragility** ([HN 26665387](https://news.ycombinator.com/item?id=26665387), sriram_malhar): "Distributed GC...is a non-starter for any environment where the network cannot be taken for granted. RMI/DCOM/CORBA had no shortage of problems due to distributed GC alone." Webber's response: Goblins doesn't attempt cross-network cycle collection (only acyclic distributed GC), which is honest but also a reduction in the original E ambition.
- **Recovery / transaction guarantees** (same thread): "Do transactions span distributed actors? Perhaps I missed it, but I see no recovery log." Goblins' transactional turn-per-vat is local; cross-vat transactions are not provided as a primitive.
- **Library ecosystem**: Racket as a host language has the bigger-library-ecosystem problem; Guile arguably more so. The same critique applies to Spritely as a developer platform.
- **Session model** (cap-talk groups list): Agoric-flavor CapTP has no notion of "session ever breaking" because it assumes store-and-forward, which "ends up in an irreconcilable state if one machine is restored from a backup with earlier message states while the other side has advanced further." Goblins' implementation diverges here, which is good engineering but also a sign the protocol is still being co-designed.

## Sources

- [Spritely Institute Goblins page](https://spritely.institute/goblins/)
- [Spritely Goblins v0.15.0 (browser via Hoot)](https://spritely.institute/news/spritely-goblins-v0-15-0-goblins-in-the-browser.html)
- [Spritely Goblins v0.17.0 (Bloblin persistence)](https://spritely.institute/news/spritely-goblins-v0-17-0-persistence-is-better-than-ever.html)
- [Spritely Goblins v0.18.0 (Sleepy actors)](https://spritely.institute/news/spritely-goblins-v0-18-0-sleepy-actors.html)
- [Spritely Institute "Distributed System Daemons / NLnet grants"](https://spritely.institute/news/spritely-nlnet-grants-december-2023.html)
- [Spritely Goblins v0.10 unification announcement (Guile primary)](https://spritely.institute/news/spritely-goblins-v0-10-for-guile-and-racket.html)
- [LWN, *A Spritely distributed-computing library*](https://lwn.net/Articles/960912/)
- [EFF, *Meet Spritely and Veilid*](https://www.eff.org/deeplinks/2023/12/meet-spritely-and-veilid)
- [OCapN repo (pre-spec status)](https://github.com/ocapn/ocapn)
- [E (programming language) — Wikipedia (history, ~100 users)](https://en.wikipedia.org/wiki/E_(programming_language))
- [Sandstorm "Joining Cloudflare"](https://sandstorm.io/news/2017-03-13-joining-cloudflare)
- [Cloudflare *Cap'n Web*](https://blog.cloudflare.com/capnweb-javascript-rpc-library/)
- [Cloudflare *Workers RPC*](https://blog.cloudflare.com/javascript-native-rpc/)
- [Agoric `@endo/captp`](https://endojs.github.io/endo/modules/_endo_captp.html)
- [HN thread (2021)](https://news.ycombinator.com/item?id=26665387)
- [HN thread (Goblins persistence, 2024)](https://news.ycombinator.com/item?id=40135942)
- [HN thread (Goblins in browser, 2025)](https://news.ycombinator.com/item?id=42859463)
- [Spritely Petnames paper](https://files.spritely.institute/papers/petnames.html)
