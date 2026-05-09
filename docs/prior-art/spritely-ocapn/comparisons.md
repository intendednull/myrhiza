# Comparisons

Where Spritely Goblins / OCapN sits relative to neighboring systems. For each comparison: what Spritely claims, where the claim is sound, where it overstates, and what the apples-to-apples picture looks like. Competitive prior art for someone designing a Component-Model P2P runtime — not a marketing matrix.

## vs Holochain

Both are P2P-distributed runtimes. They solve different problems and would compose well.

| Axis | Spritely Goblins / OCapN | Holochain |
|---|---|---|
| Primitive | Live actors (vats) with typed object refs | Source chain (single-writer log) + DHT (multi-writer with validation) |
| Capability model | CapTP — typed object refs, promise pipelining, distributed acyclic GC | Capability grants as DHT entries naming a zome function + caller filter |
| Granularity | Per-method, per-object | Per-zome-function |
| Composition | First-class — refs flow as args/returns | Out-of-band — pass cap secrets, not objects |
| Persistence | Vat snapshotting + sturdyrefs | Append-only source chain entries |
| Storage / replication | Not a concern of the protocol | Sharded DHT with validators |
| Identity model | Object names (swissnums) per vat; no global agent identity | Per-DNA agent pubkeys |
| Discovery | None protocol-level; sturdyrefs handed out of band | DHT lookup by basis hash |

**Where Spritely is right:** ocaps as live, typed object references — composable, transferable as message arguments, distributed-GC-tracked — are strictly more powerful than Holochain's capability *grants*. CapTP lets a function return a capability that itself contains references to further capabilities. Holochain caps cannot do that without app-level convention.

**Where Spritely is wrong / partial:** Spritely has no story for *durable shared state across many participants* the way Holochain's DHT does. A Goblins vat dies → its state is gone unless it was snapshotted somewhere. No replicated multi-writer storage. The model is "live actors talk to each other"; persistence is local and snapshot-shaped, not gossip-replicated.

**Lesson for Myrhiza:** these are *complementary*. CapTP-style typed refs as the message-passing primitive; a Holochain-style validated-DHT as the durable-state primitive. Neither system has both; a Component-Model runtime can.

## vs Erlang / Elixir actors

Both Spritely and BEAM are actor systems, both have lightweight processes, both have asynchronous message passing.

| Axis | Erlang/Elixir (BEAM) | Spritely Goblins |
|---|---|---|
| Actor identity | PID (per node, gossiped via epmd) | Vat-local refs (swissnums); sturdyrefs for cross-vat |
| Authority | Ambient — any process can send to any PID it knows | Capability — only references you have are reachable |
| Distribution | Distributed Erlang via `:net_kernel`, mnesia for state | OCapN (CapTP over netlayers: TCP+TLS, Tor, libp2p, WebSocket) |
| Failure model | Supervision trees, "let it crash" | Transactional turn-per-vat; no supervision-tree primitive |
| GC | Per-process generational, no cross-node | Distributed acyclic GC (no cycle collection across network) |
| Hot upgrade | First-class | Not a primitive |
| Production deployments | WhatsApp, Discord, telecom | None at scale |

**Right:** capability discipline is a real safety property BEAM lacks. In a vanilla Erlang system, leaking a PID leaks unrestricted access. In Goblins, references are unforgeable; a far-ref can be revoked.

**Where Spritely overstates / falls short:** BEAM has 30 years of operational maturity, hot code reload, supervision trees, and a real distribution story. Spritely has none of that yet. A "let it crash" supervision tree is not a Goblins primitive; you build it on top of vats and sturdyrefs by hand.

**Lesson:** capabilities-on-actors is the right idea; BEAM-style supervision is the right deployment shape. Adopt both.

## vs Akka / Pekko

JVM-side actor frameworks. Same lineage skepticism applies as Spritely: research-elegant, niche-deployed.

| Axis | Akka/Pekko | Spritely Goblins |
|---|---|---|
| Host | JVM | Guile / Racket / Hoot-WASM |
| Distributed GC | Recent: CRGC (Pekko, 2025 ACM paper) — fault-recovering cyclic | Acyclic only |
| Capabilities | None as primitive | Yes (CapTP) |
| Transactions | None | Per-turn, per-vat |
| Production scale | LinkedIn, PayPal, Walmart (Akka classic before licensing change) | None |
| Future | Akka Lightbend re-licensed BSL-1.1 (2022); Pekko Apache fork | NLnet-grant-funded ongoing |

**Right (about Spritely):** the actor-with-capabilities combination is the cleaner model. Akka had to bolt on Akka HTTP, Akka Persistence, Akka Streams, Cluster, Sharding — each its own subproject — to approximate what CapTP gives uniformly.

**Where Akka beats Spritely:** runs on a JIT-optimized JVM, integrates with the entire JVM ecosystem (Kafka, JDBC, gRPC), has Cluster Sharding for sharded actor placement, has Akka Persistence + Event Sourcing as production-tested primitives. Spritely has none of these as off-the-shelf libraries.

**Lesson:** Akka shows what the productionization of an actor framework actually looks like — many specialized libraries on top, lots of JVM ecosystem leverage. Spritely starting in Scheme means most of that has to be re-built. Pekko's CRGC paper ([dl.acm.org/doi/10.1145/3729288](https://dl.acm.org/doi/pdf/10.1145/3729288)) is the model for how to do *real* distributed GC; Goblins' acyclic-only model is an honest simplification.

## vs Cap'n Proto RPC (Sandstorm / Cloudflare)

The most direct comparison: Cap'n Proto's RPC subsystem is *also* CapTP, by the same E lineage.

| Axis | Cap'n Proto RPC / Cap'n Web | Spritely Goblins / OCapN |
|---|---|---|
| Lineage | Kenton Varda → Sandstorm → Cloudflare | Christine Lemmer-Webber → Spritely Institute |
| Schema | Strongly typed `.capnp` IDL | Untyped Scheme records / syrup |
| Wire format | Cap'n Proto encoding (zero-copy) | Syrup (canonical s-expressions) |
| Promise pipelining | Yes | Yes |
| Distributed GC | Limited — relies on three-party introductions | Acyclic distributed GC |
| Browser story | Cap'n Web (pure JS) — shipped, in production at Cloudflare | Hoot-WASM Goblins — beta, Wasm GC required |
| Production deployments | **Cloudflare Workers RPC, MetaMask Snaps via Endo** | Demos, Shepherd port (in progress) |

**Where Spritely is right:** distributed acyclic GC is genuinely ahead of Cap'n Proto, which historically punted on GC. Spritely has the most complete implementation of the original E CapTP semantics.

**Where Spritely is overstated:** Cap'n Proto and Cap'n Web *ship*. Cloudflare Workers RPC is the largest deployment of CapTP-shaped ideas in history, by a margin of millions of users. If your bet is "ocap RPC is the future," Cap'n Proto won that bet five years ago — just not under the OCapN brand.

**Where they should converge:** OCapN's stated mission is to unify Spritely + Agoric + (potentially) Cap'n Proto on one wire format. As of 2026 this is "still pre-specification." The strategic risk is OCapN ships a spec Cap'n Proto's existing wire format doesn't comply with, fragmenting rather than unifying.

**Lesson:** Component Model with WIT-typed handles maps cleanly onto Cap'n Proto's typed-IDL approach. Trying to reuse the OCapN wire encoding for cross-language CapTP is reasonable; trying to reuse the Spritely runtime semantics (Scheme-shaped, syrup-encoded) is not.

## vs Endo / Agoric

Spritely and Endo are sister projects on different host languages.

| Axis | Endo / Agoric (`@endo/captp`) | Spritely Goblins |
|---|---|---|
| Host | Hardened JavaScript (SES) | Guile / Racket |
| CapTP completeness | Production smart contracts on Agoric mainnet (Nov 2021) | Demos, Shepherd in progress |
| Confinement | SES — capability-secure JS subset | Vat isolation in Scheme |
| Production deployment | **Agoric chain (Inter Stable Token, Vaults), MetaMask Snaps** | None |
| Petnames / discovery | Endo Pet Daemon | Brux |

**Where Spritely is right:** distributed acyclic GC, third-party handoffs, sturdyref bootstrap — Spritely is "a little bit ahead" of Agoric's CapTP per Spritely's own framing.

**Where Endo / Agoric is right:** they ship in production. Agoric's mainnet runs SwingSet vats holding real value (IST stablecoin) since November 2021. MetaMask Snaps use SES + CapTP for sandboxed extension code. The deployment evidence base is real.

**Lesson:** for any production CapTP traffic in 2026, the JS implementation is ahead. For a Rust + Component Model runtime, neither Spritely's Scheme nor Agoric's hardened JS is a reusable runtime; both are useful as wire-format references and semantic references.

## vs Component Model + WIT (wasmCloud, Spin)

The comparison most relevant to Myrhiza.

| Axis | wasmCloud / Spin (Component Model) | Spritely Goblins |
|---|---|---|
| Module format | WASM Component Model | Native Scheme + Hoot-WASM |
| Capability primitive | WIT resource handles (unforgeable, table-indexed) | CapTP refs (live actors, sturdyrefs for serialization) |
| Composition | WIT imports/exports — typed at link time | Object-at-runtime; types are Scheme records |
| Cross-language | Native — Rust, Go, JS, Python all compile to components | Scheme only (Racket/Guile/Hoot) |
| Distribution | wasmCloud lattice over NATS; Spin single-host | OCapN over netlayers (TCP+TLS, Tor, WebSocket, libp2p) |
| Identity | wasmCloud: NATS ed25519 JWT | None at runtime |
| State | App-supplied (KV, SQL, NATS JetStream) | Vat-local persistence (Bloblin store, v0.17+) |
| P2P | No (NATS broker required) | Yes (OCapN netlayers) |

**Where Spritely is right:** the *semantics* are richer. Component Model handles are unforgeable, but they don't compose across the network the way CapTP refs do. A WIT import gives you a local capability set; OCapN gives you a *remote* capability that you can pipeline against, return to other vats, and have GC'd when no one needs it.

**Where the Component Model is right:** the *engineering* is far more advanced. WIT compiles to seven host languages. Components compose at link time with type-checked interfaces. The Component Model wins on tooling, language pluralism, and host-portability. Spritely loses every one of those.

**The unfilled niche:** no shipping system today combines Component-Model + WIT-typed capability imports with peer-to-peer CapTP-style distributed object refs. wasmCloud has CM+WIT but cluster-only. Spin has CM+WIT but request/response-only. Spritely has P2P CapTP but Scheme-only, no Component Model.

## vs Croquet / Multisynq

Both are deterministic-shaped systems, but at opposite ends of latency/scope.

| Axis | Croquet / Multisynq | Spritely Goblins |
|---|---|---|
| Replication unit | Computation (events advance a shared model) | Object messages; state is per-vat |
| Sync model | Reflector-mediated lockstep | Asynchronous message passing, eventual delivery |
| Determinism | Required end-to-end (every replica replays identically) | Transactional per-turn; not cross-vat-deterministic |
| Scope | Session (small group, real-time multiplayer) | Open-ended (small groups, distributed services) |
| Failure model | Reflector outage = session pauses | Connection break = far-ref breaks (E semantics) |

**Right (about Spritely):** for non-real-time, asynchronous distributed services (chat, file sharing, ActivityPub-shape), gossip-of-objects is correct.

**Partial:** Spritely has no story for *real-time co-presence*. The Goblins demo at Lisp Game Jam 2025 worked because it was small-scale; raw Goblins is not the right primitive for a 60Hz multiplayer game. Croquet *is*.

**Lesson:** these are complementary. A general P2P runtime should host both: Croquet-shaped session VMs for real-time, CapTP-shaped object refs for asynchronous services.

## vs Tahoe-LAFS

Adjacent storage-cap project. Same intellectual lineage (Brian Warner was at Mojo Nation with Zooko Wilcox-O'Hearn; ocap discipline runs through both).

| Axis | Tahoe-LAFS | Spritely Goblins |
|---|---|---|
| Primitive | Filecaps (encoded read/write/verify caps) | Object refs / sturdyrefs |
| Storage | Erasure-coded shares across N storage nodes | Vat-local; sturdyrefs persist in Bloblin |
| Capability flavor | Cryptographic — capability *is* the URI | Cryptographic — sturdyref *is* the unguessable token |
| Live vs static | Static (files) | Live (actors) |
| Production | Used by privacy-focused storage projects (e.g. PrivateStorage.io) | Demos |

**Right (about both):** capability-as-unguessable-string for bootstrap is the same idea. Tahoe filecap and OCapN sturdyref are conceptual cousins — both encode "you can't guess this; if you have it you have authority."

**Where they diverge:** Tahoe is "data at rest with redundancy"; Spritely is "behavior in flight." The capabilities mean different things — a filecap names a blob; a sturdyref names a service.

**Lesson:** the *encoding* of a capability as a URI plus secret is a portable design. Both Tahoe filecaps and OCapN sturdyrefs are useful prior art for whatever Myrhiza chooses for its on-the-wire capability format.

## Sources

- [Spritely OCapN intro](https://spritely.institute/news/introducing-ocapn-interoperable-capabilities-over-the-network.html)
- [Spritely Goblins persistence docs](https://files.spritely.institute/docs/guile-goblins/latest/Persistence.html)
- [Cap'n Proto site](https://capnproto.org/)
- [Cap'n Proto Wikipedia](https://en.wikipedia.org/wiki/Cap%27n_Proto)
- [Cloudflare *Cap'n Web*](https://blog.cloudflare.com/capnweb-javascript-rpc-library/)
- [Cloudflare *Workers RPC*](https://blog.cloudflare.com/javascript-native-rpc/)
- [Endo `@endo/captp` API](https://endojs.github.io/endo/modules/_endo_captp.html)
- [Endo repo](https://github.com/endojs/endo)
- [Agoric Mainnet-1 announcement](https://agoric.com/blog/announcements/agoric-composable-smart-contract-framework-reaches-mainnet-1-milestone/)
- [E (programming language) Wikipedia](https://en.wikipedia.org/wiki/E_(programming_language))
- [ERights.org SturdyRef](http://wiki.erights.org/wiki/SturdyRef)
- [wasmCloud capabilities](https://wasmcloud.com/docs/concepts/capabilities/)
- [Spin docs](https://spinframework.dev/)
- [sunfishcode, *What is a Capability?*](https://blog.sunfishcode.online/what-is-a-capability)
- [CRGC: Fault-Recovering Actor Garbage Collection in Pekko](https://dl.acm.org/doi/pdf/10.1145/3729288)
- [Croquet / Multisynq](https://croquet.io/)
- [Tahoe-LAFS](https://tahoe-lafs.org/)
- [OCapN repo](https://github.com/ocapn/ocapn)
- [Sandstorm "Joining Cloudflare"](https://sandstorm.io/news/2017-03-13-joining-cloudflare)
- [Spritely Petnames paper](https://files.spritely.institute/papers/petnames.html)
