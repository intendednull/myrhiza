**Date:** 2026-05-22
**Status:** active
**Subject:** Cap'n Proto language ecosystem — implementations by language, RPC vs serialization-only split, maintainer status

# Ecosystem

The cross-language reach of Cap'n Proto is the strongest evidence that the wire format and schema language are the durable parts of the project. The RPC layer ships in fewer languages because RPC is harder; the wire format ships nearly everywhere relevant.

## Languages with RPC support

The headline list — these are the languages where you can write a Cap'n Proto **RPC client and server**. Source: [capnproto.org/otherlang.html](https://capnproto.org/otherlang.html).

| Language | Project | Repo | Maintainer | Notes |
|---|---|---|---|---|
| **C++** | Cap'n Proto (reference) | [capnproto/capnproto](https://github.com/capnproto/capnproto) | @kentonv + Cloudflare Workers team | Reference impl. 13,036 stars. 1.0 LTS on `master`; 2.0 on `v2` branch. |
| **Rust** | capnproto-rust | [capnproto/capnproto-rust](https://github.com/capnproto/capnproto-rust) | @dwrensha (David Renshaw) — sole maintainer ~13 years | 2,460 stars. `capnp` v0.25.4 (2026-04-12). MIT. Single-bus-factor risk. |
| **Go** | go-capnp | [capnproto/go-capnp](https://github.com/capnproto/go-capnp) | Was @zenhack (Ian Denhardt, d. 2023) + @lthibault; @lthibault and others continue | 1,397 stars. Latest tag v3.1.0-alpha.2 (2025-10-25). MIT (LICENSE file). Beta status: *"Until the official Cap'n Proto spec is finalized, this repository should be considered beta software."* |
| **Python** | pycapnp | [capnproto/pycapnp](https://github.com/capnproto/pycapnp) | @jparyani (Jason Paryani) | Wraps the C++ ref impl. Active. |
| **OCaml** | capnp-ocaml | [capnproto/capnp-ocaml](https://github.com/capnproto/capnp-ocaml) | @pelzlpj (serialization), @talex5 (RPC) | Active; @talex5 (Thomas Leonard) brought RPC. |
| **Haskell** | haskell-capnp | [zenhack/haskell-capnp](https://github.com/zenhack/haskell-capnp) | Was @zenhack | Active but maintenance status complicated by Ian's death. |
| **C#** | capnproto-dotnetcore | [c80k/capnproto-dotnetcore](https://github.com/c80k/capnproto-dotnetcore) | @c80k | Active. |
| **Erlang** | ecapnp | [ecapnp.astekk.se](http://ecapnp.astekk.se/) | @kaos | Active. |
| **JavaScript (Node.js)** | node-capnp | [capnproto/node-capnp](https://github.com/capnproto/node-capnp) | @kentonv | Node binding to the C++ ref impl. Active. Distinct from Cap'n Web. |

## Serialization-only (no RPC)

These bindings give you Cap'n Proto's wire format but no RPC. Many production users only need serialization.

| Language | Project | Maintainer | Status |
|---|---|---|---|
| C | c-capnproto | @eqvinox | *"no longer maintained"* |
| C (fork) | c-capnproto | @jonahbeckford ([gitlab.com/dkml/ext/c-capnproto](https://gitlab.com/dkml/ext/c-capnproto)) | Forked + maintained |
| D | capnproto-dlang | @ThomasBrixLarsen | Active |
| Java | capnproto-java | @dwrensha | Active. Same maintainer as Rust. |
| JavaScript (browser) | capnp-js plugin | @popham | Active |
| JavaScript (alt) | capnproto-js | @jscheid | Abandoned |
| Lua | lua-capnproto | Cloudflare / @calio | Active. **This is the binding Cloudflare originally used in their logging pipeline pre-acqui-hire** (per the 2017-03-13 Sandstorm post). |
| Nim | capnp.nim | @zielmicha | Active |
| Ruby | capnp-ruby | @cstrahan | Active |
| Scala | capnp-scala | @katis | Active |

## Maintainer concentration

Three people own a startling fraction of the multi-language production-RPC-capable ecosystem:

- **@kentonv** (Kenton Varda) — C++ reference impl, node-capnp, Cap'n Web. Day job at Cloudflare.
- **@dwrensha** (David Renshaw) — Rust + Java. Independent for ~13 years.
- **@zenhack** (Ian Denhardt, deceased 2023) — Go (co-maintained), Haskell. The Go ecosystem has continued under @lthibault and others; Haskell is in question.

This concentration is the single biggest structural risk for downstream users. Compare to gRPC (Google + CNCF + dozens of maintainers across language bindings) — Cap'n Proto's bus factor is materially worse.

## Cap'n Web is JS-only

Cap'n Web does not have a multi-language ecosystem and is unlikely to grow one in the near term. The wire format (JSON-with-tagged-arrays) is implementable in any language, but the design center is JavaScript/TypeScript and there is no incentive for Cloudflare to invest in other-language ports while Workers RPC handles their non-JS-host needs internally (via Cap'n Proto-proper).

The Workers RPC ↔ Cap'n Web interop layer means that Cloudflare's two-tier architecture (Cap'n Proto for inside-Workers, Cap'n Web for browsers + non-Workers JS) only needs the JS side to exist — every other language goes through Cap'n Proto.

## Workers RPC has only one implementation

`cloudflare/workerd` is the only Workers RPC implementation. The wire protocol (Cap'n Proto with the `worker-interface.capnp` schema) is open-source so external runtimes *could* speak it, but no one has shipped that. In practice Workers RPC means "RPC inside Cloudflare Workers."

## Sandstorm's app ecosystem

Sandstorm shipped with an apps marketplace — Etherpad, Wekan, RocketChat, GitWeb, others. As of 2026 the marketplace is community-maintained at [apps.sandstorm.io](https://apps.sandstorm.io/) with the same low-activity profile as the platform. Apps are bundled as `.spk` files; the format and tooling are stable but not getting new apps at meaningful rate. The Sandstorm-as-platform story does not have an active developer-onboarding pipeline today.

## Community sites

- [sandstorm.io](https://sandstorm.io/) — historical site, still live; news posts from Varda's stewardship era
- [sandstorm.org](https://sandstorm.org/) — current community-led site (since 2024-01-14)
- [capnproto.org](https://capnproto.org/) — official Cap'n Proto homepage, Cloudflare-stewarded
- Sandstorm Matrix chat — community coordination
- The capnproto/* GitHub org — code home for many bindings

## Implications for Myrhiza

- **Rust bindings exist, are good, and depend on one person.** Plan a contingency: pin capnproto-rust to a specific version; budget for forking + maintenance if @dwrensha becomes unavailable. The crate is small enough to fork.
- **Production RPC across languages is feasible** if Myrhiza apps speak Cap'n Proto schemas — Rust kernel, Python tooling, Go ops scripts, C# desktop apps can all interop. Adoption surface is wider than gRPC's per-language story is brittle.
- **There is no Cap'n Web for Rust / WIT / WASM Component Model.** If Myrhiza needs a JSON-RPC-with-capability-passing for the browser tier, we're either (a) re-implementing Cap'n Web in Rust+wasm-bindgen, (b) building atop the Cap'n Web JS library via interop, or (c) building our own JSON-tagged-array protocol with Cap'n Web's design lessons. Each has real cost.
- **The bus factor on go-capnp, after Ian Denhardt, is concerning** for any Go-side integration. Verify @lthibault's continued involvement before betting Myrhiza tooling on Go bindings.
- **The Sandstorm app ecosystem is not a market we can target.** Myrhiza apps and Sandstorm apps are different shapes (Sandstorm = Linux container with capability-broker; Myrhiza = WASM component with capability imports). Don't plan ecosystem-overlap.

## Sources

- [capnproto.org/otherlang.html](https://capnproto.org/otherlang.html) — canonical language-implementation list
- [github.com/capnproto](https://github.com/capnproto) — code org
- [github.com/capnproto/go-capnp](https://github.com/capnproto/go-capnp) — Go bindings, 1,397 stars
- [github.com/capnproto/capnproto-rust](https://github.com/capnproto/capnproto-rust) — Rust bindings, 2,460 stars
- [github.com/capnproto/capnproto-java](https://github.com/capnproto/capnproto-java) — Java bindings, 443 stars
- [crates.io/users/dwrensha](https://crates.io/users/dwrensha) — Rust maintainer crate profile
- [apps.sandstorm.io](https://apps.sandstorm.io/) — Sandstorm app marketplace
