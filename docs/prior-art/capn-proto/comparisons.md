**Date:** 2026-05-22
**Status:** active
**Subject:** Cap'n Proto family compared against Spritely Goblins, Agoric/Endo, gRPC, Protobuf, FlatBuffers — and internal comparisons among the three subjects

# Comparisons

Two clusters: external (Cap'n Proto vs other RPC/serialization systems) and internal (Cap'n Proto vs Cap'n Web vs Workers RPC).

## Internal: Cap'n Proto vs Cap'n Web vs Workers RPC

The three subjects of this folder are all Varda-designed, all CapTP-derived, but solve different problems:

| Dimension | Cap'n Proto | Workers RPC | Cap'n Web |
|---|---|---|---|
| Year | 2013 | 2024-04-05 | 2025-09-22 |
| Target language | C++ (ref), multi-language bindings | JavaScript-in-workerd | JavaScript / TypeScript |
| Wire format | Binary (zero-copy, mmap-friendly) | Cap'n Proto binary under the hood | JSON-with-tagged-arrays |
| Schemas | Required (`.capnp` files) | Internal only (in workerd); app authors don't write them | None |
| Table model | 4 tables (imports/exports/questions/answers) | 4 tables (inherited from Cap'n Proto) | 2 tables (questions+imports unified, answers+exports unified) |
| Transports | Multi (KJ-async, TCP, WebSocket, in-process) | In-process when possible, workerd-internal cross-machine | HTTP batch, WebSocket, postMessage, MessagePort |
| Trust scope | Any cross-process | Same Cloudflare account only | Browser ↔ Worker, or any JS-to-JS |
| Promise pipelining | Yes | Yes | Yes |
| Three-party handoff (Level 3) | Specified, not impl'd | Not relevant (single-account scope) | Not impl'd |
| Persistent caps (Level 2) | Specified, partial | Not exposed at JS layer | No |
| Bus factor (steward) | Cloudflare Workers team (C++); @dwrensha (Rust) | Cloudflare | Cloudflare |
| License | MIT | Apache-2.0 (in workerd) | MIT |
| Production scale | Millions of RPC/sec/core in C++; hyperscale at Cloudflare | Cloudflare Workers global | Wrangler "remote bindings" + experimentation |

The three-way relationship: Cap'n Proto is the wire format underlying Workers RPC. Cap'n Web is a separate protocol that **interoperates** with Workers RPC at the Worker boundary (stubs are automatically wrapped/proxied between formats). So the deployment shape Cloudflare is targeting is:

```
[Browser/Node/JS] <—Cap'n Web—> [Worker A] <—Workers RPC—> [Worker B / Durable Object]
                                       ↓
                                       Same machine in-process when possible;
                                       Cap'n Proto binary across machines.
```

## External: Cap'n Proto vs gRPC

The closest competitive comparison. Both are schema-driven RPC systems at scale. Differences:

| Dimension | Cap'n Proto | gRPC |
|---|---|---|
| Wire format | Cap'n Proto binary (zero-copy) | Protocol Buffers binary (parsed) |
| Schema language | `.capnp` (own) | `.proto` (Protocol Buffers v3) |
| Transport | Anything (KJ-async, TCP, WS); HTTP-over-Cap'n-Proto exists | HTTP/2 (required) |
| Capability semantics | Yes — interfaces are object references, promise pipelining | No — methods are global by service name |
| Promise pipelining | Yes | No (each call independent) |
| Streaming | Yes (multi-stream flow control since 0.8) | Yes (client/server/bidi streams) |
| Governance | Cloudflare-stewarded, no foundation | CNCF graduated, Google + open contributors |
| Language ecosystem | ~15 languages, 9 with RPC, varying maintainer quality | ~12 languages, Google-maintained primaries |
| Browser support | None native (HTTP-over-Cap'n-Proto + extensions) | gRPC-Web (separate protocol bridging to HTTP/1.1) |
| Bus factor | ~3 key maintainers across implementations | Multi-vendor, dozens of maintainers |
| Adoption posture | Strong at Cloudflare; smaller everywhere else | Industry-standard for service-to-service RPC |

The honest comparison: **gRPC won the service-to-service RPC mind-share war.** It's the default at Google, Netflix, Lyft, most CNCF-ecosystem companies. Cap'n Proto is faster on the wire and ergonomically nicer for capability-passing, but lacks the multi-vendor governance + the HTTP/2-everywhere ecosystem fit.

Where Cap'n Proto wins: same-machine zero-copy (gRPC always parses), capability-passing (gRPC doesn't model this), promise pipelining (gRPC doesn't). For Myrhiza's intra-host kernel-app boundary, these are the load-bearing wins.

## External: Cap'n Proto wire format vs Protocol Buffers vs FlatBuffers

A serialization-only comparison.

| Dimension | Cap'n Proto | Protocol Buffers | FlatBuffers |
|---|---|---|---|
| Created | 2013 (Varda post-Google) | 2008 public; older internally at Google | 2014 (Google, Wouter van Oortmerssen) |
| Zero-copy on read | Yes | No (must parse) | Yes |
| In-place mutation | Yes (after build) | No | Limited |
| Schema-evolution rules | Numbered fields, strict safe-changes list | Numbered fields, similar safe-changes list | Numbered fields + careful versioning |
| Wire size (uncompressed) | Larger than Protobuf | Smallest | Comparable to Cap'n Proto |
| Packed mode | Yes (~Protobuf size, slower) | N/A | Yes |
| RPC layer | Yes (Cap'n Proto RPC) | No (Protobuf doesn't include RPC; gRPC does) | No |
| Capability semantics | Yes (interfaces) | No | No |
| Language reach | ~15 languages | Universal | ~10 languages |
| Industry adoption | Modest | Universal | Niche (gaming, embedded) |

The take: Cap'n Proto and FlatBuffers are technically very similar (both zero-copy, similar wire size). Cap'n Proto has the RPC layer and the capability story; FlatBuffers is gaming-and-embedded-focused. Protocol Buffers is universal but pays the parse step.

## External: Cap'n Proto RPC vs Spritely Goblins / OCapN

The headline ocap-RPC competitive comparison. Both descend from CapTP/E; both implement promise pipelining; both treat interfaces as first-class capabilities. They diverge sharply on everything else.

| Dimension | Cap'n Proto RPC | Spritely Goblins / OCapN |
|---|---|---|
| Year of first production use | 2014 (Sandstorm) | 2023 (Spritely Brassica) — still research-grade |
| Wire format | Cap'n Proto binary | Syrup (Lisp-friendly self-describing) |
| Primary language | C++ | Guile Scheme |
| Schema | `.capnp` files | None (Scheme structures) |
| Promise pipelining | Yes | Yes |
| Three-party handoff (Level 3) | Specified, not impl'd | OCapN spec explicitly designs for it; impl in progress |
| Persistent caps (Level 2) | Sturdyref pattern partial | First-class sturdyrefs, persistence built-in |
| Production scale | Hyperscale at Cloudflare Workers | Demo apps (Goblinville, Brassica); no production deployments at scale |
| Governance | Cloudflare corporate steward | Spritely Networked Communities Institute (501(c)(3)) + OCapN working group |
| Funding | Cloudflare-internal salaries | NLnet / NGI Assure grants, donor drives ($90K from 500+ donors in '24-'25) |
| Browser story | Cap'n Web (different protocol) | Hoot WASM compilation; Safari "not expected to work properly" |

The verdict from [`spritely-ocapn/critiques.md:56`](../spritely-ocapn/critiques.md): *"Cloudflare's blog: 'Workers RPC is built on Cap'n Proto RPC, which in turn is based on CapTP.' That is the lineage shipping. The Spritely flavor remains research-grade."*

This is *the* defining external comparison for this folder. Spritely is the research vanguard; the Cap'n Proto family is the production deployment.

## External: Cap'n Proto RPC vs Agoric `@endo/captp`

Two ocap-RPC systems from the wider lineage, both shipping in production but at different scales.

| Dimension | Cap'n Proto RPC | `@endo/captp` |
|---|---|---|
| Steward | Cloudflare | Agoric |
| Primary language | C++ (ref) / Rust / Go / ... | JavaScript (in SES Hardened-JS) |
| Wire format | Cap'n Proto binary | Pluggable; CapTP-shaped messages |
| Production deployment | Cloudflare Workers global | Agoric chain (smart contracts), MetaMask Snaps |
| Schema | `.capnp` required | None (JS objects) |
| Session model | Live; "session severance breaks all live refs" (E semantics) | Store-and-forward; "no notion of a session ever breaking" |
| Three-party handoff | Specified, not impl'd | Partially supported via virtual offer system |
| Interop with siblings | Cap'n Web (auto-stub-proxy) | None yet (OCapN aims to unify) |

The semantic divergence on session-vs-store-and-forward is real and load-bearing — see [`spritely-ocapn/critiques.md:69`](../spritely-ocapn/critiques.md): *"Agoric-flavor CapTP has no notion of 'session ever breaking' because it assumes store-and-forward, which 'ends up in an irreconcilable state if one machine is restored from a backup with earlier message states while the other side has advanced further.'"* Cap'n Proto inherits E's "session is live; on break, references throw" semantics; Agoric inherits a different durability assumption.

## External: Cap'n Web vs tRPC / GraphQL

A more JS-ecosystem-flavored comparison. The Cap'n Web announcement post explicitly frames this:

> *"GraphQL gave us a way to flatten REST's waterfalls. Cap'n Web lets us go even further: it gives you the power to model complex interactions exactly the way you would in a normal program."*

| Dimension | Cap'n Web | tRPC | GraphQL |
|---|---|---|---|
| Wire format | JSON-tagged-arrays | JSON | JSON |
| Schema | None (TS-only) | TypeScript-inferred end-to-end | SDL or schema-first |
| Capability semantics | Yes (stubs as first-class refs) | No | No |
| Pipelining / waterfall flattening | Yes via promise chains | No (each call independent) | Yes via single-query nesting |
| Bidirectionality | Yes | Limited (server→client subscriptions) | Subscriptions only |
| Cross-language | JS/TS only | JS/TS only | Any (HTTP) |
| Production scale | Experimental | Mid-scale | Hyperscale (Facebook, Shopify, GitHub) |

Cap'n Web is *not* trying to compete with tRPC or GraphQL on the JS-developer-experience axis — those are ahead. It's competing on **bidirectionality + capability passing + pipelining**, which neither tRPC nor GraphQL do. The pitch is for *new* applications that need browser-as-peer rather than browser-as-pure-client.

## Implications for Myrhiza

- **Don't fight gRPC on its turf.** If Myrhiza needs a Service Bindings-equivalent for the kernel-app boundary, Cap'n Proto RPC's capability-passing + promise pipelining is the differentiator. Frame Myrhiza's choice this way: gRPC for "outside the runtime"; Cap'n Proto RPC for "inside the runtime, between kernel and app."
- **Treat Spritely as the upper bound on Level 3 ambition.** If Myrhiza wants three-party handoff, look at OCapN's work — they're doing the design, even if they haven't shipped yet. Cap'n Proto C++ ref impl is not the place to find Level 3 today.
- **The wire-format-per-trust-tier pattern is real.** Workers RPC uses Cap'n Proto binary intra-Cloudflare; Cap'n Web uses JSON inter-browser; Cap'n Proto bridges. Myrhiza's intra-peer vs cross-peer split could analogously use a binary format inside and a more interoperable format outside.
- **Borrow Cap'n Web's two-table simplification.** The 4-table → 2-table protocol-design move is a clean engineering call that doesn't sacrifice ocap semantics. If Myrhiza ships an intra-runtime CapTP, the 2-table model is the right starting point.
- **Don't bet on FlatBuffers.** Functionally Cap'n Proto-equivalent with no RPC, no capability story, weaker schema-evolution discipline. Cap'n Proto is strictly better for Myrhiza's use case.

## Sources

- [Cap'n Proto FAQ — "Why not just use Protocol Buffers?"](https://capnproto.org/faq.html)
- [blog.cloudflare.com/capnweb-javascript-rpc-library/](https://blog.cloudflare.com/capnweb-javascript-rpc-library/) — Cap'n Web announcement, GraphQL framing
- [`../spritely-ocapn/critiques.md`](../spritely-ocapn/critiques.md) — Cap'n Proto vs Spritely critique synthesis
- [`../spritely-ocapn/open-problems.md`](../spritely-ocapn/open-problems.md) — *"the fastest CapTP-shaped traffic in production today is Cloudflare Workers RPC"*
- [`../agoric-endo/captp-and-network.md`](../agoric-endo/captp-and-network.md) — Agoric's `@endo/captp` design
- [grpc.io](https://grpc.io/) — gRPC reference
- [protobuf.dev](https://protobuf.dev/) — Protocol Buffers reference
- [flatbuffers.dev](https://flatbuffers.dev/) — FlatBuffers reference
