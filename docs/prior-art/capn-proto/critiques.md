**Date:** 2026-05-22
**Status:** active
**Subject:** Honest critiques + third-party assessments of the Cap'n Proto / Cap'n Web / Workers RPC family

# Critiques

This is the unflattering side of the corpus. The Cap'n Proto family has real, well-documented production wins; it also has real, well-documented limitations the marketing material soft-pedals. This file collects both honest internal admissions and external skepticism.

## "Infinitely faster" is marketing, not a benchmark

The capnproto.org banner literally says *"infinitely faster"* next to the logo. The implementation backs a meaningful zero-copy advantage on the read path — but published benchmarks are limited, and the "infinity times" framing is rhetorical. Per the FAQ:

> *"Are you communicating between two processes on the same machine? If so, you have unlimited bandwidth, and you should be entirely concerned with CPU."*

This is the load-bearing performance argument: in-process zero-copy beats parse-every-call by an arbitrary factor for trivial reads. For network traffic, the FAQ admits packing achieves *"similar encoding size to Protocol Buffers while still being faster"* — i.e., the size advantage is gone after packing, the speed advantage is real but not infinite.

**Honest framing:** Cap'n Proto is meaningfully faster than Protobuf for read-heavy in-process use; comparable on network traffic; pays for it with larger uncompressed wire size in the common case.

## The C++ reference implementation has not had a formal security review

From the FAQ: *"Cap'n Proto's C++ reference implementation has not yet undergone a formal security review."*

This is honestly disclosed but worth weighing for any deployment that crosses a trust boundary. The CVE history is real:
- 2015-03-02: integer overflow / underflow in pointer validation
- 2015-03-05: CPU amplification vulnerability
- 2022-11-30: CVE-2022-46149 list-of-pointers OOB read

All three were caught by review or fuzzing rather than active exploitation, but they're real bugs in the *parser* — the part of the implementation that has the largest attack surface against untrusted input.

## Schema evolution is unforgiving in practice

The schema-language spec says: *"Any change not listed above should be assumed NOT to be safe."* This is intellectually honest but operationally painful. Real-world schema-evolution bugs in Cap'n Proto deployments include: changing field types (silently corrupts data), reordering fields in a struct (changes wire layout), removing fields without retaining the slot (breaks readers). None of these are listed as safe; all of them have been done in the wild.

The discipline required to evolve `.capnp` files safely over years is comparable to Protobuf — *not* meaningfully better. Cap'n Proto's marketing implies the wire format makes evolution easier; the spec admits it doesn't.

## Capability discipline doesn't enforce itself in C++

A capability-passing RPC system relies on the host language to enforce *no-ambient-authority*. C++ does not. Per the [Spritely critique](../spritely-ocapn/critiques.md), Cap'n Proto's ocap-discipline is *protocol-level* only — a malicious or buggy C++ program in the same process can violate capability boundaries (touch raw memory, call any function, leak refs by typecast). This is unavoidable for any ocap system implemented on top of C++ rather than in an ocap-secure host language.

The implication: Cap'n Proto RPC is best understood as **"capability-passing across process boundaries"**, not "capability-secure within a process." For Myrhiza, which runs apps as WASM components, the within-process discipline is enforced by WASM sandbox — but for the C++ ref impl itself, in-process discipline is on you.

## Level 3 has not shipped

The Cap'n Proto RPC spec defines four levels of capability protocol. Production deployment is **Level 1 only**:
- Level 2 (persistent capabilities across sessions) is partially implemented; the C++ ref impl has a `Persistent` interface but applications mostly DIY persistence.
- Level 3 (three-party handoff — two parties holding caps issued by a third establish a direct connection) is **specified but never shipped**, in either the C++ ref impl or any other binding.
- Level 4 (reference equality / capability joining) is also unimplemented.

The OCapN working group's design ambition includes Level 3; they have not yet shipped a Level 3 production deployment either. The honest assessment: **no production CapTP system has shipped Level 3 in 28 years**. This is a hard problem.

For Myrhiza, this matters because cross-peer capability routing (peer A introduces peer B and peer C; B and C want to talk directly) is structurally a Level 3 problem. Plan to design Level 3 ourselves; don't expect to inherit it from upstream.

## The Rust bus factor is one person

David Renshaw (@dwrensha) has been the sole maintainer of capnproto-rust for ~13 years. The latest crate (`capnp` v0.25.4, 2026-04-12) ships monthly. The repo has no co-maintainer with merge rights; the [crates.io maintainer page](https://crates.io/users/dwrensha) lists him as the only crate owner.

If Renshaw becomes unavailable, the downstream impact is:
- Iroh, Bazel, Cloudflare Lua-on-rust internals, and a sprawl of P2P Rust projects depend on `capnp` / `capnp-rpc`
- Myrhiza, if it depends on capnproto-rust, inherits this risk
- Community-fork mode is likely viable (the crate is small enough) but not seamless

This is a materially worse bus factor than gRPC's Rust ecosystem (multi-vendor, multiple competing implementations).

## Go bindings lost their co-maintainer in 2023

Ian "zenhack" Denhardt was a key contributor to `go-capnp`, `haskell-capnp`, and Sandstorm itself. He died in an accident in mid-2023. The Go bindings continue under @lthibault and others; the Haskell bindings are in question. This is a real, tragic loss to the ecosystem, and it has measurably affected the bus factor on multiple bindings.

## Sandstorm is the cautionary tale

Sandstorm shipped the right architectural idea (capability-secure self-hostable web apps) for ~5 years and then stalled. Per Varda's 2024-01-14 handoff post:

> *"In early 2023, I gave up pushing monthly releases, since there seemed to be no point: no code changes had been made and no dependencies could be updated."*

The codebase is stuck on MongoDB 2.6 (released 2014). The Tempest rewrite stalled when Ian Denhardt died. The community continues maintenance but at low velocity; no significant new apps have shipped in years. Per the 2017-03-13 post: *"Sandstorm will no longer be our full-time jobs"* — the founders' attention moved to Cloudflare, and the project never recovered.

This matters because Sandstorm was the *proof of concept* for capability-secure end-user applications. It worked technically; it didn't work commercially. The Workers RPC + Cap'n Web combination is the *new* attempt by the same team to bring ocap-RPC to the masses — but via the Cloudflare developer-tools market, not the self-hosting market.

## Cap'n Web is too new + too narrow

The 2025-09-22 announcement explicitly says: *"Cap'n Web is new and still highly experimental. There may be bugs to shake out."*

At v0.8.0 (2026-05-11), the project is 8 months old. The only named production user is Cloudflare's own Wrangler "remote bindings" feature. The library is JS/TS-only — there's no plan to ship in other languages.

Compared to Cap'n Proto's 10-year-to-1.0 trajectory, Cap'n Web is in the earliest stages. Treating it as a stable bet for Myrhiza in 2026 would be premature.

## The "no schema" claim is JS-specific

Cap'n Web's headline differentiator — *"no schemas"* — is true at the API surface for JS/TS but doesn't generalize. The protocol still has implicit schemas in two places:
1. **TypeScript types on the API surface** carry the contract at compile time. Without them you have no editor support; the schema-replacement is `RpcSession<RemoteMainInterface>` generics.
2. **The wire format itself** is a schema (type-tagged JSON arrays). It's just that *every* Cap'n Web peer understands the same set of tags out of box.

For cross-language Cap'n Web (which doesn't exist), you'd need a schema translation layer. Cloudflare hasn't built this because they don't need it (Workers RPC handles non-JS hosts). Anyone else trying to port Cap'n Web to Rust or Python would hit this immediately.

## Workers RPC is single-account scope

The hardest production limitation: *"For now, Service Bindings and Durable Objects only allow communication between Workers running on the same account."* Cross-account Worker-to-Worker RPC is not supported. This is a real product limit that affects every Cloudflare-multi-tenant scenario.

The structural reason: capability-token-as-bearer-trust does not extend naturally across trust boundaries without an introducer or a delegation framework. Cloudflare has not shipped this. Cap'n Web partially fills the gap (browser-as-untrusted-peer) but not for Worker-to-Worker across tenants.

This is the same structural problem Myrhiza will face for cross-peer (= cross-trust-boundary) RPC. Cloudflare hasn't solved it at the protocol level. Plan accordingly.

## Third-party voices

Honest assessments from outside the Cloudflare-funded perimeter:

- **HN on Cap'n Proto 1.0 (2023-07-28)**: [news.ycombinator.com/item?id=36908309](https://news.ycombinator.com/item?id=36908309). General positive, with substantive critique on slow language-binding evolution and the "infinitely faster" marketing.
- **The Spritely community on Cap'n Proto** (per [`../spritely-ocapn/critiques.md`](../spritely-ocapn/critiques.md)): The Cap'n Proto branch is "the lineage shipping" — said in admiration but also as a comparison-against-Spritely's-research-status. Spritely's framing is honest about this gap.
- **OCapN working group**: The OCapN spec exists precisely because Cap'n Proto's RPC, Spritely's RPC, and Agoric's `@endo/captp` don't interoperate at the wire level. Per the OCapN repo: *"OCapN is still pre-specification, and will likely be the output of examining to what extent the Agoric, Spritely, and potentially Cap'N Proto implementations can be unified"*. The fact that this is *pre*-specification is itself a critique: even with three implementations and 12+ years of effort, the ocap-RPC family has not produced an inter-implementation standard.

## What this means for Myrhiza

The honest read of the family is:

- **Cap'n Proto RPC is the production-grade Level-1 capability-RPC system today.** Battle-tested, polished, used at hyperscale. Use it if Myrhiza needs Level-1 ocap-RPC.
- **It is not a Level-2, -3, or -4 system.** Production hasn't shipped these in 28 years. Don't expect Cap'n Proto to grow them; plan to build them yourself if you need them.
- **The bus factor on bindings is real.** Rust + Go + Java have at-most-one-critical-maintainer-per-project. Plan contingencies.
- **Cap'n Web is too early to bet on.** Watch it; revisit in 12-18 months.
- **Sandstorm is a corpse-shaped cautionary tale** about technical-correctness-without-commercial-traction.

## Sources

- [Cap'n Proto FAQ](https://capnproto.org/faq.html) — security review disclosure, performance framing
- [Cap'n Proto news](https://capnproto.org/news/) — CVE disclosures
- [`../spritely-ocapn/critiques.md`](../spritely-ocapn/critiques.md) — Spritely-side critique of Cap'n Proto
- [sandstorm.io/news/2024-01-14-move-to-sandstorm-org](https://sandstorm.io/news/2024-01-14-move-to-sandstorm-org) — *"no code changes had been made"*
- [github.com/ocapn/ocapn](https://github.com/ocapn/ocapn) — *"OCapN is still pre-specification"*
- [HN: Cap'n Proto 1.0](https://news.ycombinator.com/item?id=36908309)
- [blog.cloudflare.com/capnweb-javascript-rpc-library/](https://blog.cloudflare.com/capnweb-javascript-rpc-library/) — *"still highly experimental"*
- [crates.io/users/dwrensha](https://crates.io/users/dwrensha) — sole-maintainer status
