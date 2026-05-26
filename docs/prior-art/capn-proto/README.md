**Date:** 2026-05-22
**Status:** active
**Subject:** Cap'n Proto / Cap'n Web / Workers RPC — the production CapTP-shaped lineage from Kenton Varda (Google → Sandstorm → Cloudflare)

# Cap'n Proto / Cap'n Web / Workers RPC

This folder covers three related-but-distinct projects, all designed by Kenton Varda and all descended from Mark Miller's CapTP:

- **Cap'n Proto** (2013) — the binary serialization format + schema language + RPC system. Started inside Google as Varda's escape hatch from Protocol Buffers, then moved to Sandstorm (2014), then absorbed into Cloudflare (2017). C++ reference implementation; production-quality bindings in Rust, Go, Python, OCaml, Haskell, C#, Erlang.
- **Workers RPC** (2024-04-05) — Cloudflare's JavaScript-native intra-Workers RPC. *"Under the hood, it is built on Cap'n Proto"* ([blog](https://blog.cloudflare.com/javascript-native-rpc/), 2024-04-05). Open-source as part of [`workerd`](https://github.com/cloudflare/workerd). The deployed CapTP at hyperscale.
- **Cap'n Web** (2025-09-22) — a separate TypeScript/JavaScript-native RPC library, JSON-based wire format, designed for browsers + Workers + Node. *"A spiritual sibling to Cap'n Proto"* — same author, different wire format, no schemas. Public since September 2025, still tagged "highly experimental" at v0.8.0.

This is **the production branch of the ocap/CapTP lineage**. The [Spritely](../spritely-ocapn/) and [Agoric/Endo](../agoric-endo/) folders cover the research-grade branches. Cloudflare's blog ([2024-04-05](https://blog.cloudflare.com/javascript-native-rpc/)) on Workers RPC contains the canonical statement: *"Workers RPC is a JavaScript-native RPC system. Under the hood, it is built on Cap'n Proto."* Both [`spritely-ocapn/critiques.md:56`](../spritely-ocapn/critiques.md) and [`spritely-ocapn/open-problems.md:25`](../spritely-ocapn/open-problems.md) point here.

## Key facts

| Fact | Value |
|---|---|
| Creator | Kenton Varda (kentonv) |
| Cap'n Proto created | 2013-04-01 (first public release; project started at Google ~2013) |
| Cap'n Proto repo | [`github.com/capnproto/capnproto`](https://github.com/capnproto/capnproto) — 13,036 stars, C++, default branch `v2`, last push 2026-05-21, MIT |
| Cap'n Proto current version | 1.x (LTS) on `master` since 2023-07-28; 2.x in development on `v2` branch |
| Cap'n Proto Rust | [`github.com/capnproto/capnproto-rust`](https://github.com/capnproto/capnproto-rust) — `capnp` crate v0.25.4 (2026-04-12), `capnp-rpc` v0.25.1 (2026-04-29), `capnpc` v0.25.3 (2026-04-02). Sole maintainer: David Renshaw (@dwrensha). MIT. |
| Cap'n Web repo | [`github.com/cloudflare/capnweb`](https://github.com/cloudflare/capnweb) — created 2025-06-08, public-announce 2025-09-22, 3,813 stars, TypeScript, MIT |
| Cap'n Web current version | npm `capnweb` v0.8.0 (2026-05-11). First public version 0.0.1 (2025-09-12); 0.1.0 (2025-09-21) was the launch tag |
| Workers RPC | Cloudflare blog announcement [2024-04-05](https://blog.cloudflare.com/javascript-native-rpc/); implementation in [`cloudflare/workerd`](https://github.com/cloudflare/workerd) (Apache-2.0); schema in [`src/workerd/io/worker-interface.capnp`](https://github.com/cloudflare/workerd/blob/main/src/workerd/io/worker-interface.capnp) |
| Sandstorm | [`github.com/sandstorm-io/sandstorm`](https://github.com/sandstorm-io/sandstorm) — 7,025 stars, Apache-2.0, founded 2014, *acqui-hired into Cloudflare 2017-03-13 for $0*; ownership moved to `sandstorm.org` community 2024-01-14 (led by Jacob "ocdtrekkie" Weisz). Effectively stalled: last meaningful code activity ~2019; "no code changes had been made and no dependencies could be updated" (Varda, 2024-01-14). |
| Bus factor | C++ ref impl: Cloudflare Workers team ("now the primary developers and maintainers", per capnproto.org FAQ). Rust: @dwrensha sole maintainer for ~13 years. Go: previously co-maintained by Ian Denhardt (@zenhack), who died mid-2023; @lthibault and others continue. |
| License | Cap'n Proto: **MIT** (verified via LICENSE file; GitHub API reports `NOASSERTION` due to copyright header format). Cap'n Web: **MIT**. Workers RPC inside workerd: **Apache-2.0**. Sandstorm: **Apache-2.0**. |
| Adjacent stack | [`spritely-ocapn/`](../spritely-ocapn/) (research-grade CapTP), [`agoric-endo/`](../agoric-endo/) (`@endo/captp` is a sibling JS CapTP, distinct from Cap'n Web), OCapN draft spec (attempts to unify Spritely + Endo + Cap'n Proto). |

## Contents

Each file is independent and skimmable. Cross-linked from siblings as needed.

**The three subjects (one file each):**
- [**Cap'n Proto**](capnp.md) — the binary format, schema language, RPC layers 1-4, promise pipelining, C++ reference impl, the 1.0 LTS vs 2.0/v2 story.
- [**Cap'n Web**](capnweb.md) — the 2025 TypeScript-native variant. JSON wire format, two-table (vs four-table) protocol, no schemas, browser + Workers + Node, "spiritual sibling" framing.
- [**Workers RPC**](workers-rpc.md) — Cloudflare's intra-Workers RPC. JS-native, built on Cap'n Proto under the hood, ships at hyperscale. Service Bindings + Durable Objects deployment shape.

**Project lens:**
- [**Sandstorm**](sandstorm.md) — the origin story. Where Cap'n Proto came from, why the team ended up at Cloudflare, what's left of the project (community-maintained since 2024, mostly stalled).
- [**History**](history.md) — chronological narrative from CapTP/E (1997) → Protobuf (2008) → Varda leaves Google → Cap'n Proto + Sandstorm (2013-14) → Cloudflare acqui-hire (2017) → 1.0 LTS (2023-07-28) → Workers RPC (2024-04) → Cap'n Web (2025-09).
- [**Ecosystem**](ecosystem.md) — language implementations table, RPC-vs-serialization-only matrix, who maintains what.
- [**Governance**](governance.md) — Cloudflare stewardship of C++ ref impl + Workers RPC + Cap'n Web; community stewardship of Sandstorm; bus-factor notes.
- [**Comparisons**](comparisons.md) — vs Spritely Goblins / OCapN, Agoric `@endo/captp`, gRPC, Protocol Buffers, FlatBuffers; and Cap'n Web vs Workers RPC.

**Reference:**
- [**Lessons for Myrhiza**](lessons.md) — **validates / avoid / borrow** — the consult-this-when-designing file.
- [**Open problems**](open-problems.md) — what this lineage structurally doesn't solve (browser CapTP-without-trust-in-Cloudflare, three-party handoff in production, persistent capabilities, post-quantum, multi-language interop).
- [**Critiques**](critiques.md) — third-party + honest assessments. The "infinitely faster" benchmark; the schema-vs-no-schema split; Sandstorm's commercial failure; the bus-factor on Rust + Go bindings.
- [**Glossary**](glossary.md) — cap, stub, promise pipelining, three-party handoff, Level 1-4, sturdyref/cap-restore, vat (and how it isn't used in this branch), KJ, workerd, JSValue.

## How to use this prior-art doc

Designing a Myrhiza feature touching capability-RPC, intra-host IPC, or browser-to-runtime communication? Start with [`lessons.md`](lessons.md). Then drop into the relevant subject file:

- *Binary RPC + schema* → [`capnp.md`](capnp.md).
- *JS/TS object-capability RPC for browsers* → [`capnweb.md`](capnweb.md).
- *Production CapTP at hyperscale* → [`workers-rpc.md`](workers-rpc.md).
- *Why did the ocap RPC idea finally ship?* → [`history.md`](history.md) + [`sandstorm.md`](sandstorm.md).

Doc lives, not snapshot — bump the date in this file's header on every meaningful update.

**Framing disclosure.** These docs are written from a Component-Model-as-foundation, P2P-runtime stance — most "Implications for Myrhiza" sub-sections frame the Cap'n Proto / Workers RPC / Cap'n Web choices through that lens. The corpus is deliberately partisan toward the lessons-learned-from this branch of CapTP, in particular the trade-off Varda made *away from* schema-driven Cap'n Proto *toward* schemaless Cap'n Web. Future readers auditing whether schema-driven WIT-style typing is itself the right primitive should weigh the corpus accordingly: it's a learn-from-Cloudflare-into-CM artifact, not a neutral catalog. The [Spritely](../spritely-ocapn/) and [Agoric/Endo](../agoric-endo/) folders carry the same disclosure for the same reason. **Additional disclosure: this branch ships at hyperscale (Workers).** That biases this corpus toward soft-pedaling production-scale concerns we will *not* face at Myrhiza's scale (multi-DC routing, Cloudflare-specific isolate optimization) and toward over-weighting browser + JS ergonomics, since that is where Cap'n Web is uniquely strong. Read with that bias in mind.

## Sources

- [capnproto.org](https://capnproto.org/)
- [capnproto.org/rpc.html](https://capnproto.org/rpc.html) (canonical CapTP-lineage statement)
- [capnproto.org/news/](https://capnproto.org/news/) (release history)
- [blog.cloudflare.com/javascript-native-rpc/](https://blog.cloudflare.com/javascript-native-rpc/) (Workers RPC announcement, Kenton Varda, 2024-04-05)
- [blog.cloudflare.com/capnweb-javascript-rpc-library/](https://blog.cloudflare.com/capnweb-javascript-rpc-library/) (Cap'n Web announcement, Kenton Varda + Steve Faulkner, 2025-09-22)
- [github.com/capnproto/capnproto](https://github.com/capnproto/capnproto)
- [github.com/cloudflare/capnweb](https://github.com/cloudflare/capnweb)
- [github.com/cloudflare/workerd](https://github.com/cloudflare/workerd)
- [github.com/sandstorm-io/sandstorm](https://github.com/sandstorm-io/sandstorm)
- [sandstorm.io/news/2024-01-14-move-to-sandstorm-org](https://sandstorm.io/news/2024-01-14-move-to-sandstorm-org)
- [sandstorm.io/news/2017-03-13-joining-cloudflare](https://sandstorm.io/news/2017-03-13-joining-cloudflare)
- [crates.io/crates/capnp](https://crates.io/crates/capnp)
- [npmjs.com/package/capnweb](https://www.npmjs.com/package/capnweb)
