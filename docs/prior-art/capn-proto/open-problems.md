**Date:** 2026-05-22
**Status:** active
**Subject:** What the Cap'n Proto / Cap'n Web / Workers RPC family structurally does not solve

# Open Problems

These are the gaps the production CapTP-shaped lineage has *not* closed, despite 28 years of design and 13 years of shipping production code. Distinguished from [`critiques.md`](critiques.md) (which covers honest assessments of what *did* ship): this file covers what *didn't*.

## 1. Three-party handoff (Level 3 CapTP)

The hardest problem in distributed ocap-RPC, and the one the production deployments have not solved.

**The setup:** Peer A holds a capability issued by peer C. Peer A introduces peer C's capability to peer B. Peer B wants to invoke the capability directly against C (without proxying every call through A).

**Why it's hard:**
- B and C have never spoken before — they have no shared connection state.
- The capability token is currently scoped to A↔C's session.
- Establishing a B↔C direct connection requires either A acting as a connection-introducer (a new protocol message) or C exposing a public-facing endpoint (defeating ocap's "no ambient authority" discipline).
- All this must happen *without* B or C trusting A more than they did before.

**Status:**
- Cap'n Proto C++ ref impl: Specified in RPC spec, **never implemented**.
- capnproto-rust: Not implemented.
- Workers RPC: Not relevant (single-account scope; everyone in the same trust domain).
- Cap'n Web: Not implemented.
- Spritely Goblins / OCapN: Design in progress, no production deployment yet.
- Agoric `@endo/captp`: Partial via virtual-offer system, not general Level 3.

**The structural reason:** Level 3 requires either a third trusted party (a connection introducer) or a public-key-routing layer (peers can reach each other by ID independent of session). Cap'n Proto's design predates modern P2P discovery primitives; it assumes session-based introductions, which can't extend cleanly to third-party direct connections.

**For Myrhiza:** Cross-peer capability routing is a Level 3 problem. Plan to design this yourself; don't expect to inherit it. Borrow vocabulary from the OCapN spec where useful.

## 2. Persistent capabilities (Level 2) at scale

Sturdyrefs — capabilities that survive across sessions and machine restarts — are partially specified but rarely deployed.

**The Cap'n Proto state:** A `Persistent` interface exists in `persistent.capnp`; `SaveAs` operations are part of the RPC protocol; but in practice **most apps build their own persistence** rather than relying on the protocol-level mechanism. The C++ ref impl's `Persistent` is more of a hook than a complete system.

**Workers RPC state:** Not exposed at the JS layer. Durable Objects provide stateful endpoints but the capability-restoration semantics are not the same as Level 2 sturdyrefs.

**Cap'n Web state:** Capabilities are tied to `RpcSession` lifetime. Once the WebSocket closes, the export table is gone.

**The structural reason:** Persistent capabilities need (a) a stable identifier independent of session state, (b) a re-authentication path when reconnecting, (c) a revocation mechanism, (d) garbage-collection of stale persistent caps. Each is hard alone; together they require coordination across the protocol, host runtime, and storage layer. No production system has shipped this cleanly.

**For Myrhiza:** State persistence + capability persistence are coupled but separable. Design state persistence first (it's tractable); design capability persistence after, with explicit attention to revocation.

## 3. Cross-implementation interop

There is no production CapTP-shaped wire format that interoperates across Cap'n Proto, Spritely Goblins, Agoric `@endo/captp`, and Cap'n Web. Each speaks its own dialect:

- Cap'n Proto RPC: binary, schema-driven.
- Spritely / OCapN: syrup, schema-less self-describing.
- Agoric `@endo/captp`: JS objects, pluggable transport.
- Cap'n Web: JSON-tagged-arrays.

OCapN aims to unify these but is **pre-specification** as of 2026. Even the Spritely + Endo + Cap'n Proto interop demo is still under construction.

**The structural reason:** Each implementation chose a wire format optimized for its host language. Unifying requires either (a) a least-common-denominator format (which sacrifices each implementation's wins), (b) a translation layer at every boundary, or (c) one implementation absorbing the others. None has happened.

**For Myrhiza:** Don't plan on cross-CapTP interop being available. If Myrhiza needs to talk to a non-Myrhiza CapTP system, plan to write a custom bridge. Cap'n Web's auto-stub-proxy pattern (with Workers RPC) is the only working example.

## 4. Browser ocap discipline without trusting the server

Cap'n Web brings ocap RPC to the browser, but the browser is not an ocap-secure host. Once a capability stub is in JavaScript's memory, any other JavaScript code in the same origin can reach it (via XSS, third-party scripts, devtools, etc.).

**The structural reason:** The browser's same-origin policy is the trust boundary, not the JavaScript reference graph. A capability discipline at the JS level can be defeated by anyone who can run JS in the same origin.

**Cap'n Web's framing:** *"No runtime type checking; malicious clients can send unexpected types."* — i.e., the discipline is best-effort, not a security primitive.

**For Myrhiza:** If Myrhiza apps run in browser contexts (web-frontend communicating with peer), assume the browser is a hostile environment for capability hygiene. Treat browser-held capabilities as throwaway-per-session; durable authority lives in the peer process, not the browser.

## 5. Post-quantum-safe signing of capabilities

Cap'n Proto does not standardize a signature scheme over its messages. Applications layering signed capabilities on top use whatever crypto they pick — typically Ed25519. None of the production deployments have shipped a post-quantum-safe story.

**The structural reason:** PQ signing (ML-DSA, SLH-DSA) has larger keys + signatures than Ed25519, which affects wire size + processing cost. The migration story for already-issued capabilities is unclear. No protocol-level guidance from Cap'n Proto upstream.

**For Myrhiza:** Plan a PQ-signing migration story for capability-restore tokens. Borrow from the MLS or Iroh playbooks rather than expecting Cap'n Proto to dictate.

## 6. Capability revocation

Per Mark Miller's E lineage, capability *revocation* (the issuer of a cap can later invalidate it) is supported via the "membrane" or "revoker" pattern: caps are issued through a per-context proxy that the issuer can shut down. In production deployments:

- **Cap'n Proto:** Application-level only. The protocol does not provide a revocation primitive.
- **Workers RPC:** Capabilities are scoped to a session/binding; revoking a binding implicitly revokes all caps it exposed. No finer-grained revocation.
- **Cap'n Web:** Same as Workers RPC — session-bounded.

**Mass revocation** (an issuer wants to revoke 10K caps at once because some shared state changed) is essentially intractable with bearer-token capabilities. The fix is *not* to issue bearer tokens for revocation-required authority; use call-time authorization checks instead.

**For Myrhiza:** Plan revocation at the application layer, not at the kernel-capability layer. If Myrhiza apps need fine-grained revocation, they're responsible for proxying through revokable membranes.

## 7. Performance vs alternatives

No published, peer-reviewed benchmark compares Cap'n Proto RPC throughput to gRPC, Capnp's own packed format vs Protobuf, or Cap'n Web vs tRPC. The "infinitely faster" marketing and the FAQ's "millions of calls per second per core in C++" claims are real but uncalibrated.

**The structural reason:** Benchmark publishing is a real-cost activity that doesn't have a single funding incentive. Cloudflare runs Cap'n Proto in production at hyperscale and has internal data; the public domain has small comparison studies, mostly from years ago.

**For Myrhiza:** Bench Cap'n Proto RPC against alternatives ourselves for our specific workload (small messages, cross-process, on the kernel-app boundary). Don't trust marketing numbers.

## 8. KJ as a non-standard C++ ecosystem

The KJ toolkit Cap'n Proto uses is a non-standard alternative to the C++ standard library — its own promise / event-loop / async / string / collection types. Using Cap'n Proto from C++ effectively requires learning KJ. Bindings in other languages hide this but the C++ ref impl is KJ-flavored throughout.

**The structural reason:** Varda built KJ to solve C++ ergonomics problems for himself; it predates many newer C++ features (coroutines, ranges) and uses its own paradigms. The v2 / `v2` branch is partly a KJ modernization project.

**For Myrhiza:** Don't write C++ that consumes Cap'n Proto directly. Use the Rust bindings (capnproto-rust does not expose KJ).

## 9. Sandstorm-as-platform did not scale

The original Sandstorm vision — capability-secure self-hostable web apps for end users — did not reach mass adoption. The platform is stalled (see [`sandstorm.md`](sandstorm.md)). This is an open problem in the sense that **the user-facing-product use case for capability-secure RPC has not been demonstrated at scale**.

Workers RPC ships at hyperscale but to *developers*, not end users. Cap'n Web is too new. Spritely has no flagship end-user app.

**The structural reason:** "Capabilities" are a developer-experience improvement; end users don't perceive them. Mass adoption of an ocap-secure platform requires either a 10x UX improvement that depends on ocap (e.g., easier sharing, stronger sandboxing) or a regulatory tailwind. Neither has materialized.

**For Myrhiza:** Plan for the same problem. Myrhiza's value proposition to end users cannot be "capabilities"; it must be a felt benefit (offline-first, no platform lock-in, easy multi-device, etc.) that capabilities happen to enable.

## 10. Multi-implementation governance has not emerged

The CapTP family has multiple production implementations and no umbrella governance. Cap'n Proto (Cloudflare-funded), Spritely Goblins (NLnet-funded), Agoric `@endo/captp` (Agoric-funded), Cap'n Web (Cloudflare-funded). OCapN is the closest thing to a unifying working group; it is pre-specification.

Compare to gRPC (CNCF-graduated), MLS (IETF RFC), CRDTs (Inria + academic + multi-vendor). The ocap-RPC family is structurally less coordinated.

**For Myrhiza:** Don't expect protocol-level decisions from a foundation; expect them from one of the funding entities. Decide which implementation's wire format Myrhiza will track (probably Cap'n Proto, but document the choice).

## Implications for Myrhiza

The open-problem list is, in aggregate, the list of things Myrhiza will probably have to design ourselves rather than inherit from upstream:

1. Three-party / cross-peer capability handoff — design ourselves.
2. Persistent capabilities — design ourselves; borrow sturdyref vocabulary from Spritely.
3. Cross-CapTP interop — write our own bridge if needed.
4. Browser ocap discipline — treat browser caps as throwaway.
5. PQ signing — plan migration story.
6. Revocation — application-layer.
7. Performance — bench ourselves.
8. C++ KJ — avoid; use Rust.
9. End-user adoption — don't sell capabilities; sell what they enable.
10. Multi-implementation governance — pick a wire format, document the choice.

This is a substantial design surface. The wins from adopting Cap'n Proto RPC are real (battle-tested Level-1, capability-passing, promise pipelining); the unsolved problems are also real and constitute the bulk of an ocap runtime's distinguishing value.

## Sources

- [Cap'n Proto RPC spec](https://capnproto.org/rpc.html) — Level 1-4 taxonomy
- [`capnp/persistent.capnp`](https://github.com/capnproto/capnproto/blob/v2/c++/src/capnp/persistent.capnp) — Persistent interface
- [`../spritely-ocapn/open-problems.md`](../spritely-ocapn/open-problems.md) — overlapping open-problem analysis
- [github.com/ocapn/ocapn](https://github.com/ocapn/ocapn) — OCapN spec working group
- [blog.cloudflare.com/javascript-native-rpc/](https://blog.cloudflare.com/javascript-native-rpc/) — single-account-scope admission
- [blog.cloudflare.com/capnweb-javascript-rpc-library/](https://blog.cloudflare.com/capnweb-javascript-rpc-library/) — "highly experimental" disclosure
