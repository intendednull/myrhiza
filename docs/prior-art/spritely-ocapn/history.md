# History

A chronological narrative of the intellectual ancestry leading to Spritely Goblins and OCapN, from the 1970s actor model through capability-OS research, the E language, Cap'n Proto / Sandstorm, Spritely's 2018 founding, and the OCapN cross-implementation standardization push of 2022 onward. We document this lineage as prior art for Myrhiza's distributed object-capability design space.

## 1970s — Actors and capability OS

The actor model was introduced in **1973** by **Carl Hewitt, Peter Bishop, and Richard Steiger** in the IJCAI'73 paper *"A Universal Modular Actor Formalism for Artificial Intelligence"* ([paper PDF](https://eighty-twenty.org/files/Hewitt,%20Bishop,%20Steiger%20-%201973%20-%20A%20universal%20modular%20ACTOR%20formalism%20for%20artificial%20intelligence.pdf), [Wikipedia: Actor model](https://en.wikipedia.org/wiki/Actor_model)). Hewitt drew explicitly on physics, Lisp, Simula, early Smalltalk, **capability-based systems**, and packet switching — the actor model is genealogically a capability formalism, not just a concurrency formalism.

Capability OSes emerged in parallel. **GNOSIS** was implemented at Tymshare in 370 assembler in the late 1970s; renamed **KeyKOS** when Tymshare became Key Logic, it ran in production for VISA transaction processing from **1983 onward**, designed by **Norm Hardy**, Bill Frantz, Charlie Landau, and others ([EROS Wikipedia](https://en.wikipedia.org/wiki/EROS_(microkernel))).

In **1988**, Norm Hardy published *"The Confused Deputy (or why capabilities might have been invented)"* in ACM SIGOPS, formalizing the canonical attack that capability designs prevent ([Hardy 1988 PDF](http://web.cs.wpi.edu/~cs557/f14/papers/confused_deputy-hardy.pdf)). The same year, **Mark S. Miller** and **K. Eric Drexler** published the *Agoric Open Systems* papers, introducing market mechanisms for resource allocation in computation ([Miller bio](https://en.wikipedia.org/wiki/Mark_S._Miller)).

## 1990s — Electric Communities, Original-E, Joule, E

Through the early 1990s **Jonathan Shapiro** licensed and re-implemented KeyKOS as **EROS** (a clean-room C++ rewrite at the University of Pennsylvania, then Johns Hopkins), eventually his dissertation work ([EROS Wikipedia](https://en.wikipedia.org/wiki/EROS_(microkernel))).

**Electric Communities**, founded by veterans of LucasFilm's Habitat (Chip Morningstar, Randy Farmer, Doug Crockford), built **Electric Communities Habitat** — "a P2P, secure, 3D virtual world with player-run economies and safe, untrusted execution of code" ([Spritely "What is CapTP"](https://spritelyproject.org/news/what-is-captp.html)). To make that secure-distributed-objects vision implementable, in **1997** Mark S. Miller, Dan Bornstein, Doug Crockford, Chip Morningstar, and others at Electric Communities built the **E language**, descending from the concurrent language **Joule** and from **Original-E** (a Java extension for secure distributed programming) ([E Wikipedia](https://en.wikipedia.org/wiki/E_(programming_language))).

E introduced the canonical vocabulary that Goblins inherits today: **vats** (single-threaded event loops containing objects), **near** vs **far** references, **eventual sends** with **promise pipelining**, **sealer/unsealer** pairs, **swiss numbers**, and **sturdyrefs**. The wire protocol used to connect E vats was **CapTP** (Capability Transport Protocol), with **VatTP** as the lower transport layer ([CapTP on erights.org](http://erights.org/elib/distrib/captp/index.html)).

The dot-com crash killed Electric Communities Habitat, but E continued as an open-source project; **erights.org** has been the canonical home of E-language and capability-theory papers ever since ([erights.org](http://erights.org/)).

## 2000s — *Robust Composition*, MarkM at Google, Caja, SES

Mark Miller's writings in this decade are the theoretical core most subsequent ocap work cites:

- **2000** — *"Capability-based Financial Instruments"* (Financial Cryptography), with Tribble, Hardy, Hibbert, et al.
- **2003** — *"The Digital Path: Smart Contracts and the Third World"* with Marc Stiegler.
- **2005** — Miller, Tribble, and Shapiro, *"Concurrency Among Strangers"* (Trustworthy Global Computing) — the canonical paper on E's vat/promise concurrency model.
- **May 2006** — **Mark S. Miller**'s Johns Hopkins PhD thesis *"Robust Composition: Towards a Unified Approach to Access Control and Concurrency Control"*, advised by Jonathan Shapiro ([thesis PDF](http://erights.org/talks/thesis/markm-thesis.pdf)). The most-cited single document in the Spritely lineage; unifies access control and concurrency control under a single capability-passing object model.

From roughly **2007** Miller worked at Google on **Caja** — a sandboxing system for executing untrusted JavaScript using ocap principles — and joined **TC39** to push **Secure ECMAScript (SES)** into the language standard, the work that eventually became **Hardened JS** at Agoric.

## 2014–2017 — Sandstorm.io and Cap'n Proto

**Kenton Varda** (former Google Protocol Buffers maintainer) ran a successful Indiegogo crowdfunding campaign in **2014**, founding **Sandstorm.io** as a self-hostable web-application platform with capability-based security ([Sandstorm Indiegogo](https://www.indiegogo.com/projects/sandstorm-io-personal-cloud-platform), [Redecentralize interview, Aug 2014](https://redecentralize.org/interviews/2014/08/20/17-kenton-varda.html)). To wire its services together Varda built **Cap'n Proto**, whose RPC layer is "a mostly-CapTP for people who don't know they're using a CapTP" — its capability model is explicitly drawn from E's CapTP ([Cap'n Proto Wikipedia](https://en.wikipedia.org/wiki/Cap'n_Proto)).

The startup hired ~7 people, took angel and VC funding through **2016**, then failed to raise its Series A. In **2017** the team was acqui-hired by Cloudflare, where Cap'n Proto continues to be developed (Cloudflare Workers' RPC system uses it). Sandstorm itself transitioned to a community project, and in **January 2024** the sandstorm.io domain was formally handed over to the Sandstorm Community under Open Source Collective ([Sandstorm news](https://sandstorm.io/news/2024-01-14-move-to-sandstorm-org)).

In parallel, **Agoric** (founded by Mark Miller, Dean Tribble, and Bill Tulloh) built another CapTP descendant in JavaScript — **SwingSet** and the **Endo** sandboxing toolkit — for use as a smart-contracting platform.

## 2018 — Spritely founded; Goblins v0.1

**Christine Lemmer-Webber**, lead author and co-editor of the W3C **ActivityPub** specification (Recommendation, January 2018), began the **Spritely Project** in **May 2018** as a research project to determine the future of decentralized social networks following her ActivityPub work. She had co-founded **GNU MediaGoblin** in 2011; Spritely was framed as the next step beyond ActivityPub's limitations ([Lemmer-Webber Wikipedia](https://en.wikipedia.org/wiki/Christine_Lemmer-Webber)).

Lemmer-Webber identified that the object-capability community had answers to most of the security and authority problems she'd encountered with ActivityPub. She began consulting with **Randy Farmer** (Habitat / Electric Communities co-founder), and the two later co-founded the **Spritely Networked Communities Institute**.

**Goblins v0.1** — pre-pre-alpha — was announced **October 31, 2018** on Lemmer-Webber's blog and on the cap-talk mailing list ([Announcing Goblins, Oct 31 2018](https://dustycloud.org/blog/announcing-goblins/)). Initial implementation was Racket. Design was explicitly modeled on E and Electric Communities documentation, with input from Mark Miller.

## 2019–2021 — NLnet grants, CapTP in Goblins

Spritely's funding was bootstrapped largely by **NLnet / NGI Zero** grants from the European Commission's Next Generation Internet program. The first grant — *Content Addressed Descriptors and Interfaces* — funded most of Goblins' CapTP implementation through 2020–2021 ([Spritely NLNet grant blog](https://dustycloud.org/blog/spritely-nlnet-grant/)).

**Jessica Tallon** — co-author of ActivityPub — joined Spritely with her own NLnet grant on **November 10, 2021** to build a petnames system on top of Goblins. Tallon would become the lead developer of Spritely Goblins.

## 2022 — Spritely Institute (501(c)(3)); OCapN bootstrapped

The **Spritely Networked Communities Institute** received its **501(c)(3) tax-exempt status on April 23, 2022** ([Spritely 501(c)(3) approval](https://spritely.institute/news/spritely-institute-501-c-3-approval.html)), formally constituting the nonprofit. EIN 87-4257919. Founders: Christine Lemmer-Webber and Randy Farmer.

**October 19, 2022** — Spritely announced an **NLnet grant to bootstrap OCapN as a cross-implementation standard**, with Jessica Tallon leading ([NLnet bootstraps OCapN](https://spritely.institute/news/nlnet-grant-bootstraps-ocapn-protocol-standardization-effort.html)). The phased plan: draft specs, form a community group, build a compliance/test suite, write an implementer's guide, and submit to a standards body. The **OCapN Pre-standardization Group** was formed and began monthly meetings; minutes have been public since **July 2023**.

## 2023 — Guile Goblins, Hoot, OCapN spec drafting

**January 30, 2023** — **Goblins v0.10** released. Guile became the primary implementation; Racket continued to be maintained but was no longer the canonical version. OCapN-level interop between the Racket and Guile implementations of *goblin-chat* "just worked" — the foundational demonstration of cross-implementation OCapN ([Goblins 0.10](https://spritely.institute/news/spritely-goblins-v0-10-for-guile-and-racket.html)).

The **OCapN Pre-standardization Group** convened multiple organizations: **Spritely**, **Agoric** (Endo team — including Kris Kowal), **Cap'n Proto** (Kenton Varda), and **MetaMask** (which uses Endo for its Snaps plugin system and supply-chain attack defense). The MetaMask team's role is review and unification across the existing Agoric, Spritely, and Cap'n Proto implementations.

**October 16, 2023** — **Guile Hoot v0.1.0** released by Andy Wingo, Christine Lemmer-Webber, Robin Templeton, and David Thompson, **funded by MetaMask** ([Hoot 0.1.0](https://spritely.institute/news/guile-hoot-v0-1-0-released.html)). Hoot is a Scheme-to-WebAssembly compiler backend for GNU Guile and a general WASM toolchain. Targets the WasmGC proposal so Scheme garbage collection maps onto WASM's own GC.

By end of 2023, OCapN draft specifications for **CapTP**, **Netlayers**, and **Locators** were emerging in the [ocapn/ocapn](https://github.com/ocapn/ocapn) repository.

## 2024 — Goblins persistence, libp2p, the Three Vats Three Networks demo

**January 22, 2024** — **Goblins v0.12.0** released, adding the **Prelay** (relay) and **TCP+TLS** netlayers to the existing Tor onion-services netlayer ([Goblins 0.12.0](https://spritely.institute/news/spritely-goblins-v0-12-0-released-two-new-netlayers-join-the-ocapn-family-and-more.html)). With Tor + TCP-TLS + Prelay (and libp2p coming), the OCapN abstraction was demonstrably transport-agnostic — the conceptual setup behind the **"three vats, three networks"** demo.

**April 23, 2024** — **Goblins v0.13.0** released, introducing the first version of the **Aurie** persistence system: a vat-level secure persistence mechanism that serializes a live object graph to disk and restores it later, taking care that restored objects cannot regain capabilities they did not hold at save time ([Goblins 0.13.0](https://spritely.institute/news/spritely-goblins-v0-13-0-object-persistence-and-easier-io.html)).

**May 9, 2024** — Spritely announced new NLnet grants for **Distributed System Daemons**: porting GNU Shepherd (the Guix system layer) to Guile Goblins so a Shepherd daemon can be controlled remotely over OCapN ([Distributed System Daemons](https://spritely.institute/news/spritely-nlnet-grants-december-2023.html)). Pitched as "the single largest real-world deployment of Spritely code to date" — a viable path to replace Kubernetes-style orchestration with capability-secure peer orchestration. The work motivated the Unix-domain-socket netlayer that would land in 0.16.

**LWN coverage — May 2024** ([LWN: A Spritely distributed-computing library](https://lwn.net/Articles/960912/)).

**September 19, 2024** — **Goblins v0.14.0** released, adding the **libp2p netlayer** (via a Go libp2p daemon spawned alongside Goblins, similar to how the onion netlayer drives a Tor daemon), along with intra-vat persistence, vat root-object upgrades, and a `migrations` macro ([Goblins 0.14.0](https://spritely.institute/news/spritely-goblins-v0-14-0-libp2p-and-improved-persistence.html)). With this release Goblins shipped the four-netlayer set — Tor, TCP+TLS, Prelay, libp2p — that crystallized the "transport-agnostic" claim.

**December 4, 2024** — Spritely launched its **2024–2025 Supporter Drive** ([Supporter drive launch](https://spritely.institute/news/spritely-launches-supporter-drive.html)).

## 2025 — Goblins in the browser; Hoot maturity; Shepherd integration

**January 15, 2025** — Supporter drive surpassed its **$80,000 goal** with three weeks remaining, and a $40k stretch goal opened ([Supporter drive success](https://spritely.institute/news/2024-2025-supporter-drive-success.html)).

**January 24, 2025** — **Goblins v0.15.0** released — *"Goblins in the browser."* For the first time, Goblins compiled to WebAssembly via Hoot ran in browsers, with OCapN working over a new **WebSocket netlayer** ([Goblins 0.15.0](https://spritely.institute/news/spritely-goblins-v0-15-0-goblins-in-the-browser.html)). A 150-line goblin-chat ran live in the browser exchanging OCapN messages with desktop peers. 1.2–2× speedups across Goblins. The Spring Lisp Game Jam 2025 included a multiplayer virtual-world demo running Hoot+Goblins.

**August 7, 2025** — **Goblins v0.16.0** released, adding a **Unix Domain Socket netlayer** (with an introduction-server "little OCaps kernel" preventing confused-deputy attacks across local IPC), 10–20× spawn speedups via compile-time identifier resolution, and faster `bcom` (behavior change) via encapsulated-cookie comparison ([Goblins 0.16.0](https://spritely.institute/news/spritely-goblins-v0-16-0-released.html)).

**September 10, 2025** — **Shepherd × Goblins update** demoed three-vat fleet orchestration: a Carol-the-DevOps actor controlling web servers on machines A and B, all three running Shepherd-on-Goblins, all three communicating over OCapN ([Shepherd × Goblins](https://spritely.institute/news/shepherd-goblins-update.html)).

Subsequent 0.17.0 (improved persistence with Bloblin store, October 2025) and 0.18.0 ("sleepy actors" — actors that swap to disk under a configurable caching policy and are revived on message receipt, April 2026) shipped across the late-2025/early-2026 window ([Goblins 0.17.0](https://spritely.institute/news/spritely-goblins-v0-17-0-persistence-is-better-than-ever.html), [Goblins 0.18.0](https://spritely.institute/news/spritely-goblins-v0-18-0-sleepy-actors.html)).

By end of 2025: OCapN draft specs (CapTP, Netlayers, Locators) had matured in the [ocapn/ocapn repo](https://github.com/ocapn/ocapn); a Haskell implementation by Alexander Bondarenko had been added; both Spritely Goblins implementations passed the cross-implementation test suite; and Agoric's Endo and MetaMask's Snaps were the most heavily-deployed CapTP-family code in production.

## 2026 — Current state (May 2026)

Spritely Goblins remains pre-1.0 (Guile is canonical at v0.18.0, Racket maintained at v0.12.x). OCapN remains pre-standardization but has functional drafts and a working multi-implementation interop test suite. Hoot is at 0.7.0 (October 2025) with Goblins-in-browser support. The Distributed System Daemons / Shepherd-on-Goblins work is the leading edge of "Spritely as serious distributed-systems infrastructure" rather than only "social-web research."

The headline story of the Spritely lineage is intact: a continuous theoretical-and-implementation thread from Hewitt 1973 → KeyKOS → Miller's E (1997) → *Robust Composition* (2006) → Cap'n Proto / Sandstorm (2014) → Goblins (2018) → OCapN multi-vendor standardization (2022+). Spritely's specific contribution is reviving the E vocabulary in working open-source code, generalizing CapTP's transport ("VatTP") into pluggable **netlayers**, and getting Agoric, MetaMask, and Cap'n Proto to the same table to formalize a shared protocol.

## Sources

- [Hewitt, Bishop, Steiger — "A Universal Modular ACTOR Formalism for AI" (IJCAI 1973)](https://eighty-twenty.org/files/Hewitt,%20Bishop,%20Steiger%20-%201973%20-%20A%20universal%20modular%20ACTOR%20formalism%20for%20artificial%20intelligence.pdf)
- [Hardy — "The Confused Deputy" (ACM SIGOPS 1988)](http://web.cs.wpi.edu/~cs557/f14/papers/confused_deputy-hardy.pdf)
- [Miller — *Robust Composition* (PhD thesis, JHU 2006)](http://erights.org/talks/thesis/markm-thesis.pdf)
- [erights.org — home of E and CapTP papers](http://erights.org/)
- [E (programming language) — Wikipedia](https://en.wikipedia.org/wiki/E_(programming_language))
- [EROS (microkernel) — Wikipedia](https://en.wikipedia.org/wiki/EROS_(microkernel))
- [Mark S. Miller — Wikipedia](https://en.wikipedia.org/wiki/Mark_S._Miller)
- [Cap'n Proto — Wikipedia](https://en.wikipedia.org/wiki/Cap'n_Proto)
- [Sandstorm — moves to .org (Jan 2024)](https://sandstorm.io/news/2024-01-14-move-to-sandstorm-org)
- [Christine Lemmer-Webber — Wikipedia](https://en.wikipedia.org/wiki/Christine_Lemmer-Webber)
- [Announcing Goblins (Oct 31, 2018)](https://dustycloud.org/blog/announcing-goblins/)
- [Spritely 501(c)(3) approval (Apr 23, 2022)](https://spritely.institute/news/spritely-institute-501-c-3-approval.html)
- [NLnet grant bootstraps OCapN (Oct 19, 2022)](https://spritely.institute/news/nlnet-grant-bootstraps-ocapn-protocol-standardization-effort.html)
- [OCapN repo](https://github.com/ocapn/ocapn)
- [Endo (Agoric)](https://github.com/endojs/endo)
- [Goblins 0.10 (Jan 30, 2023)](https://spritely.institute/news/spritely-goblins-v0-10-for-guile-and-racket.html)
- [Goblins 0.12.0 (Jan 22, 2024)](https://spritely.institute/news/spritely-goblins-v0-12-0-released-two-new-netlayers-join-the-ocapn-family-and-more.html)
- [Goblins 0.13.0 (Apr 23, 2024)](https://spritely.institute/news/spritely-goblins-v0-13-0-object-persistence-and-easier-io.html)
- [Goblins 0.14.0 (Sep 19, 2024)](https://spritely.institute/news/spritely-goblins-v0-14-0-libp2p-and-improved-persistence.html)
- [Goblins 0.15.0 (Jan 24, 2025)](https://spritely.institute/news/spritely-goblins-v0-15-0-goblins-in-the-browser.html)
- [Goblins 0.16.0 (Aug 7, 2025)](https://spritely.institute/news/spritely-goblins-v0-16-0-released.html)
- [Goblins 0.17.0](https://spritely.institute/news/spritely-goblins-v0-17-0-persistence-is-better-than-ever.html), [Goblins 0.18.0](https://spritely.institute/news/spritely-goblins-v0-18-0-sleepy-actors.html)
- [Hoot 0.1.0 (Oct 16, 2023)](https://spritely.institute/news/guile-hoot-v0-1-0-released.html)
- [Distributed System Daemons (Shepherd × Goblins, May 2024)](https://spritely.institute/news/spritely-nlnet-grants-december-2023.html)
- [Shepherd × Goblins update (Sep 10, 2025)](https://spritely.institute/news/shepherd-goblins-update.html)
- [LWN: A Spritely distributed-computing library (May 2024)](https://lwn.net/Articles/960912/)
