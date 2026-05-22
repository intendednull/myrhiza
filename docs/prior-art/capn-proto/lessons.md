**Date:** 2026-05-22
**Status:** active
**Subject:** Lessons for Myrhiza — validates / avoid / borrow — the consult-this-when-designing file

# Lessons for Myrhiza

This is the synthesis file. Other files in this folder are evidence; this is the decision file. Format: **validates** what Myrhiza is already doing; **avoid** patterns this lineage shows are wrong; **borrow** concrete subsystems we should mirror or directly depend on.

## Validates

Things Myrhiza's current design intuitions get right, evidenced by this lineage:

1. **Capability-based RPC is production-viable at hyperscale.** Cloudflare Workers ships CapTP-shaped traffic — Service Bindings + Durable Objects calls run on Cap'n Proto under the hood, at the scale Cloudflare Workers operates (a substantial fraction of global web traffic). The "ocap is academic" reading is wrong; the production deployment exists. ([`workers-rpc.md`](workers-rpc.md))

2. **Promise pipelining is the load-bearing latency win.** Every CapTP-shaped system (Cap'n Proto, Workers RPC, Cap'n Web, Spritely, Endo) ships promise pipelining and treats it as a headline feature. If Myrhiza's kernel-app boundary is RPC-shaped, ship pipelining from day one. ([`capnp.md §RPC`](capnp.md))

3. **Schemas + numbered fields + explicit safe-change rules are the right schema-evolution model.** Cap'n Proto's `.capnp` and Protobuf's `.proto` converged on the same answer despite design differences elsewhere. Myrhiza's state-apply event-schema evolution should adopt the same model: every field numbered, "any change not in the safe list is unsafe" as the default rule. ([`capnp.md §Schema`](capnp.md))

4. **One protocol per trust tier is fine; one ocap semantic across all tiers is non-negotiable.** Cloudflare ships Workers RPC (binary, intra-account) + Cap'n Web (JSON, inter-browser) + bridges between them. The wire format adapts to the trust tier; the *semantic* (capabilities as first-class refs, pipelining, bidirectional calls) is identical. Myrhiza's intra-peer vs cross-peer split can take the same shape. ([`comparisons.md`](comparisons.md))

5. **Same-process zero-copy + cross-process wire encoding is the right dual.** Cap'n Proto's zero-copy reads + packed-on-the-wire is the right dual encoding pattern; Workers RPC's same-thread-when-possible + Cap'n Proto wire across machines is the dynamic dispatch version. Myrhiza's kernel-app boundary should optimize the same way: same-process Component Model resource calls are free, cross-process calls pay wire encoding. ([`workers-rpc.md`](workers-rpc.md))

6. **Long-term-support branches matter for protocol stability.** Cap'n Proto 1.0 LTS (since 2023-07-28) lets downstream users freeze against a stable API while v2 development continues. Myrhiza will eventually have the same need; plan for an LTS branch + dev branch from day one. ([`history.md`](history.md))

7. **Open-source-the-wire-protocol + open-source-the-reference-implementation is the right governance choice.** Cloudflare open-sourced workerd including the Cap'n Proto wire format. Cap'n Proto itself is MIT. Cap'n Web is MIT. This lets downstream users (including Myrhiza) understand + verify + fork what they depend on. Match this in Myrhiza. ([`governance.md`](governance.md))

## Avoid

Patterns this lineage shows are wrong or load-bearing-incorrect:

1. **Don't bet on Level 3 / three-party handoff arriving from upstream.** In 28 years of CapTP, no production system has shipped Level 3. Cap'n Proto's C++ ref impl specifies it but has not implemented it. If Myrhiza needs cross-peer capability routing, design it yourself; don't expect it from any of the existing CapTP implementations. ([`open-problems.md §1`](open-problems.md))

2. **Don't make capability discipline depend on host-language correctness.** Cap'n Proto's C++ ref impl can't enforce ocap discipline within a process because C++ doesn't enforce no-ambient-authority. Myrhiza's apps-as-WASM-components model side-steps this by relying on the WASM sandbox; *don't* relax that. The within-process discipline is what makes ocap real. ([`critiques.md`](critiques.md))

3. **Don't sell "capabilities" to end users.** Sandstorm shipped the right architecture for 5 years; commercial adoption did not happen. End users do not perceive capabilities as a benefit; sell what capabilities *enable* (easy sharing, isolation, offline-first, no platform lock-in). ([`sandstorm.md`](sandstorm.md), [`open-problems.md §9`](open-problems.md))

4. **Don't depend on a single-maintainer crate for load-bearing infrastructure without a fork plan.** capnproto-rust has been one person for 13 years. Iroh, several Bazel projects, and others depend on it. If Myrhiza depends on `capnp` / `capnp-rpc`, plan a contingency fork. ([`critiques.md`](critiques.md), [`ecosystem.md`](ecosystem.md))

5. **Don't write C++ that consumes Cap'n Proto.** KJ toolkit is non-standard C++ with its own paradigms; the v2 / `v2` branch is partly a KJ modernization project. If Myrhiza needs Cap'n Proto, use capnproto-rust. ([`open-problems.md §8`](open-problems.md))

6. **Don't expect Cap'n Web 1.0 ergonomics in 2026.** Cap'n Web is 8 months old, "highly experimental", single-major-product anchor. If Myrhiza needs a JSON-RPC-with-capability-passing for the browser tier in 2026-2027, plan to either re-implement Cap'n Web's protocol ourselves (the spec is in `protocol.md`) or build atop the JS library via WASM interop. Don't bet a Myrhiza-1.0 ship date on Cap'n Web upstream maturity. ([`capnweb.md`](capnweb.md))

7. **Don't try to copy the JS Proxy-based dotted-path RPC dispatch.** Workers RPC works because `Proxy` is a JS-only language feature. Rust + WIT do not have this. Don't try to replicate the *ergonomics* of `env.MY_BINDING.foo.bar.baz()` in a typed language; replicate the *idea* (an interface is a callable surface) via WIT typed resources. ([`workers-rpc.md`](workers-rpc.md))

8. **Don't drop schemas because Cap'n Web did.** Cap'n Web works without schemas because TypeScript is the only target. Myrhiza is cross-language by construction (Rust kernel + apps in arbitrary WIT-targeting languages); we need schemas. Borrow Cap'n Web's *protocol* lessons (2-table, JSON-tagged-array, transport-agnostic); don't take the "no schema" choice. ([`capnweb.md`](capnweb.md))

9. **Don't expect cross-CapTP interop to be available.** Cap'n Proto, Spritely, Endo, and Cap'n Web don't speak each other's wire formats. OCapN is pre-specification. If Myrhiza needs to bridge to a non-Myrhiza CapTP, plan to write that bridge ourselves. ([`open-problems.md §3`](open-problems.md))

10. **Don't assume Sandstorm's app marketplace is a model.** It worked technically, didn't reach commercial scale, and is now in maintenance mode. The Sandstorm-app-format (`.spk`) is a useful reference for "how do you package a capability-confined web app" but the *market* never developed. Myrhiza's app distribution should not assume a marketplace-as-default model. ([`sandstorm.md`](sandstorm.md), [`ecosystem.md`](ecosystem.md))

## Borrow

Concrete subsystems Myrhiza should either depend on directly or mirror in our design.

| What to borrow | From | Why | Where in this folder |
|---|---|---|---|
| Cap'n Proto wire format + RPC for kernel-app boundary | `capnp` v0.25.x + `capnp-rpc` v0.25.x (Rust) | Production-grade Level-1 capability-RPC; zero-copy on read; promise pipelining; ~monthly releases; MIT | [`capnp.md`](capnp.md), [`ecosystem.md`](ecosystem.md) |
| The 4-level CapTP taxonomy as vocabulary | Cap'n Proto RPC spec | Lets Myrhiza specs say "Level 1 only" or "Level 2 ambition" precisely, so the gap to Level 3+ is visible | [`capnp.md §RPC`](capnp.md), [`open-problems.md`](open-problems.md) |
| Promise pipelining design | Cap'n Proto / Cap'n Web | Single feature, multiple round-trip-eliminating wins; ship it from MVP | [`capnp.md`](capnp.md) |
| Schema language with numbered fields + safe-change rules | `.capnp` | Mature schema-evolution model; "any change not in the safe list is unsafe" | [`capnp.md §Schema`](capnp.md) |
| 2-table protocol model (questions+imports unified, answers+exports unified) | Cap'n Web | Engineering simplification of CapTP without sacrificing ocap semantics | [`capnweb.md`](capnweb.md) |
| JSON-tagged-array wire format for debug + cross-trust-boundary tier | Cap'n Web `protocol.md` | Portable, debuggable wire format if Myrhiza needs a text-mode CapTP alongside the binary one | [`capnweb.md`](capnweb.md) |
| Wire-format ↔ wire-format auto-stub-proxy bridge pattern | Cap'n Web ↔ Workers RPC | The right answer to "we have two formats and need them to interop": automatic proxy at the bridge point | [`workers-rpc.md`](workers-rpc.md), [`comparisons.md`](comparisons.md) |
| Same-process zero-cost RPC + cross-process Cap'n Proto wire dispatch | Workers RPC Service Bindings | "Same thread = function call; same machine = IPC; cross-machine = wire encoding"; Myrhiza's kernel-app boundary should be similarly tiered | [`workers-rpc.md`](workers-rpc.md) |
| Workers RPC's `worker-interface.capnp` schema shape | `cloudflare/workerd` | Reference for how to schema the host-app RPC boundary: typed methods, JsValue for opaque values, externals for stream/RPC-stub refs | [`workers-rpc.md`](workers-rpc.md) |
| LTS branch + dev branch model | Cap'n Proto 1.0 LTS + v2 | Long-running deployments freeze against stable API while fast-moving work continues | [`history.md`](history.md), [`governance.md`](governance.md) |
| Honest CVE disclosure cadence | Cap'n Proto news page | Public, transparent, on the project site, separate from release notes | [`critiques.md`](critiques.md) |
| Per-repo license per binding | `capnproto/*` org pattern | Multi-implementation org with per-binding maintainers; each binding's LICENSE is verified independently (capnproto-rust = MIT, capnproto-java = MIT, etc.) | [`governance.md`](governance.md), [`ecosystem.md`](ecosystem.md) |

## Concrete Myrhiza implications

Reading across the validate/avoid/borrow synthesis, the load-bearing implications for Myrhiza's runtime spec:

1. **Use `capnp` + `capnp-rpc` for the kernel-app RPC boundary.** This is the highest-leverage borrow. The Rust crates are production-grade, MIT, ~monthly releases. Plan a contingency fork because of the single-maintainer bus factor. Vendor-pin in `Cargo.toml`.

2. **Document Myrhiza's CapTP-level posture explicitly.** "Myrhiza ships Level 1 only at v1; Level 2 (cap persistence) on roadmap; Level 3 (cross-peer handoff) is a stretch goal" — write this down so spec readers know the gap.

3. **Design the wire format for the cross-peer tier separately from the intra-peer tier.** Intra-peer = Cap'n Proto binary via capnp-rpc. Cross-peer = either Cap'n Proto-over-encrypted-transport or a new JSON-tagged-array-Cap'n-Web-style format. Decide explicitly; document the trade-offs.

4. **Plan the LTS-branch governance early.** Don't wait until 1.0 to think about LTS; the Cap'n Proto pattern (1.0 LTS at the 10-year mark + v2 branch in parallel) is the right shape. Myrhiza should have a 1.0 LTS commitment language in its first spec release.

5. **Plan for capabilities as developer ergonomics, not user marketing.** Borrow the technical pattern from Workers RPC; sell Myrhiza on what capabilities *enable* for end users (sharing, isolation, offline, multi-device), not on capabilities themselves.

## Sources

This file synthesizes [`capnp.md`](capnp.md), [`capnweb.md`](capnweb.md), [`workers-rpc.md`](workers-rpc.md), [`sandstorm.md`](sandstorm.md), [`critiques.md`](critiques.md), [`open-problems.md`](open-problems.md), [`comparisons.md`](comparisons.md), [`history.md`](history.md), [`ecosystem.md`](ecosystem.md), and [`governance.md`](governance.md). Original sources cited in each. Cross-referenced against [`../spritely-ocapn/lessons.md`](../spritely-ocapn/lessons.md), [`../agoric-endo/lessons.md`](../agoric-endo/lessons.md), and [`../iroh/lessons.md`](../iroh/lessons.md).
