# Open problems Spritely / OCapN doesn't solve

Problems the architecture doesn't structurally solve, regardless of effort applied. Myrhiza will face most of them — be modest about the same boundaries.

## 1. Discovery — how do strangers meet?

OCapN is structurally *introduce-then-invoke*. A sturdyref is "an unguessable token so by possessing it you have the authority to send messages to the endpoint it designates" ([Spritely Goblins persistence docs](https://files.spritely.institute/docs/guile-goblins/latest/Persistence.html)). The protocol gives you no way to find a counterparty you have never met.

Two strangers cannot bootstrap a CapTP session from zero shared knowledge. Some channel — QR code, email, hyperlink, business card, gossip overlay, DHT — must convey a sturdyref out of band. Spritely's response is the petname system (Brux), which provides usability *after* introduction, not introduction itself. For a P2P runtime that wants public services, search, or topical pub/sub, OCapN must be paired with a discovery layer it does not provide.

## 2. Sybil resistance and global identity

OCapN names objects (vats and swissnums), not principals. Anyone can spin up an arbitrary number of vats with arbitrary numbers of object references. There is no global identity, no proof-of-personhood, no stake, no Sybil gate.

This is consistent with the ocap discipline — capabilities are about authority transfer, not identity verification — but it means OCapN cannot be the layer where Sybil resistance is enforced. An app built on OCapN that needs Sybil-resistance (voting, airdrop allocation, scarce-resource distribution) must layer it elsewhere: a chain, a federation, a web-of-trust petname graph, an external proof-of-personhood service.

## 3. Persistence durability under host loss

The Spritely persistence model is local snapshots. Vats serialize their object graph to a Bloblin store ([Goblins v0.17 release notes](https://spritely.institute/news/spritely-goblins-v0-17-0-persistence-is-better-than-ever.html)); sturdyrefs survive vat restart; sleepy actors page hot/cold actors to disk.

What this *does not* give you: distributed durability. If the machine running a vat dies, the data dies with it unless the operator backed it up. There is no replication, no quorum, no erasure coding, no peer-pinning. By design — Spritely is "live actors talking to each other," not "Tahoe-LAFS." But it means OCapN cannot be the layer where durability lives. Apps that need durability under host loss must build it themselves or layer Spritely on top of a separate replicated store.

## 4. Performance / high-throughput services

Promise pipelining helps round-trip latency, not raw throughput. The CapTP machinery (object table maintenance, distributed GC bookkeeping, syrup encoding/decoding) has per-message overhead. Goblins on Guile / Racket / Hoot inherits Scheme runtime performance — fine for chat, weak for high-throughput. There are no published benchmarks comparing Goblins to gRPC, Cap'n Proto C++, or NATS. The fastest CapTP-shaped traffic in production today is Cloudflare Workers RPC ([blog.cloudflare.com](https://blog.cloudflare.com/javascript-native-rpc/)) — built on Cap'n Proto, not Spritely.

For an application like real-time analytics, a high-frequency message bus, or a 60Hz multiplayer game, raw OCapN-on-Goblins is unlikely to be the right primitive. Croquet/Multisynq's reflector-mediated lockstep model is the right shape there. The runtime that wants both must host both.

## 5. Adoption funnel — what would unlock production?

E never reached meaningful adoption as an open-source language ([Wikipedia](https://en.wikipedia.org/wiki/E_(programming_language))). Spritely is a generation later, with better tooling, a better host language story (Guile + Hoot), and grant funding. As of 2026 there is no flagship Spritely app at scale. Demos exist; production does not.

The structural reasons:

- **Multi-language fragmentation.** Guile / Racket / Hoot-WASM / Endo / Cap'n Proto are five different "CapTP" implementations, none of which is canonical for everyone. OCapN's stated mission is to converge them; convergence is not done.
- **Scheme is not a mass-market host language.** A Rust developer in 2026 has no Spritely SDK; the Scheme-first ecosystem is a real adoption barrier.
- **No native browser story without a Wasm-3.0-GC + tail-call host.** Hoot ships, but the browser support floor is high.
- **Research-grade by self-description.** The Spritely Institute has not claimed production readiness; that is honest, but it also means production users are not coming.

What would unlock production: one flagship app, in one host language, with one canonical runtime. Cloudflare Workers RPC achieved that for the Cap'n Proto branch of CapTP. Spritely has not. Whether OCapN unification can produce the same outcome for the Spritely lineage is the open strategic question.

## 6. Real-time co-presence

OCapN's message-passing semantics are asynchronous and eventually-delivered. There is no protocol primitive for "all participants see this state at the same instant" — the requirement of a 60Hz multiplayer game, a shared whiteboard, a CRDT-replicated document. Goblinville (Lisp Game Jam 2025) demonstrated multiplayer over OCapN; the demos are honest about being small-scale. For real-time co-presence at scale, Croquet / Multisynq is the model, not OCapN. A general P2P runtime needs both shapes.

## 7. Cross-implementation interop under failure

Spritely's CapTP supports live sessions where "on connection severance all live references break and throw relevant errors" — the original E semantics, "extremely sensible for distributed video games and other latency-sensitive use cases." Agoric's `@endo/captp` assumes store-and-forward, "no notion of a 'session' ever breaking." When backups, restarts, or network failures cause one side to advance and the other to roll back, "those two systems can end up in an irreconcilable state…it would not be possible for those two systems to communicate without establishing a new session" — Spritely's own framing.

OCapN must specify a single recovery semantics across implementations. As of 2026 it has not. Solvable but not solved.

## 8. Formal verification

Goblins' transactional turn model and CapTP's three-table coordinator are state machines that *could* be specified in TLA+, model-checked in Loom (for a Rust port), or proved in a theorem prover. Nothing of the sort is published. Correctness is asserted at the documentation level, validated empirically by tests and by the (small) live deployments. The problem is not unique to Spritely — Holochain has the same gap — but it is real, and it is one Myrhiza can avoid by formalizing the state machine before locking the wire format.

## 9. Distributed GC of cycles

Spritely's distributed GC is *acyclic*. Cross-network reference cycles leak. Webber acknowledges this on HN ([26665387](https://news.ycombinator.com/item?id=26665387)): the original E handled cycles, modern Goblins does not. Honest, and fine for v1 — cycles are an order of magnitude more complex than acyclic. But applications with rich back-references (mutual subscriptions, peer chat, social graphs) will accumulate uncollected garbage over long-running sessions. Pekko's CRGC paper ([dl.acm.org/doi/10.1145/3729288](https://dl.acm.org/doi/pdf/10.1145/3729288)) is the state of the art for going further; nothing in the Spritely roadmap proposes adopting it.

## 10. Capability revocation at scale

In CapTP, a capability is revoked by the holder dropping it (then GC reclaims) or by the issuer interposing a revocable forwarder. Both work for small N. At scale — "revoke this capability across 10,000 holders" — the model is hand-coded membrane patterns, not a runtime primitive. Spritely has not addressed mass-revocation; neither has Endo. For a runtime hosting credentials, session tokens, or shared service caps, this is a real productionization gap.

## Implications for Myrhiza

Don't pretend Myrhiza solves these. The PR's research-notes file already flags discovery and durability as the two biggest open problems, which is the right framing.

Specific decisions where this matters:

- **Discovery (1):** Decide whether Myrhiza ships a discovery primitive (DHT or gossip overlay) or explicitly delegates. Document the choice loudly.
- **Sybil / identity (2):** Document explicitly that Myrhiza caps name authority, not principals. Provide hooks for membrane proofs; don't bake a specific scheme.
- **Durability (3):** Decide whether Myrhiza is "live actors only" (Spritely-shaped) or also a replicated store (Holochain-shaped) or both. Don't ship "we'll figure it out."
- **Performance (4):** Publish benchmarks vs gRPC, Cap'n Proto, libp2p, NATS from MVP. If Myrhiza is 10× slower, know it before the marketing site does.
- **Adoption (5):** One flagship Myrhiza app, in Rust, with one canonical runtime. Don't fragment.
- **Recovery semantics (7):** Pick live-or-store-and-forward per channel; document explicitly; do not let it diverge across host languages.
- **Formalization (8):** Specify the wire and the vat-state state machine in TLA+ before 1.0. Use Loom on the Rust runtime from day one. Both are cheap if done early.
- **GC (9):** Acyclic for v1 is fine; document the limit; do not market as "full distributed GC."
- **Revocation (10):** Provide a revocable-forwarder pattern as a host import, not as an app-pattern.

## Sources

- [Spritely Goblins persistence docs](https://files.spritely.institute/docs/guile-goblins/latest/Persistence.html)
- [Goblins v0.17.0 (Bloblin)](https://spritely.institute/news/spritely-goblins-v0-17-0-persistence-is-better-than-ever.html)
- [Goblins v0.18.0 (Sleepy actors)](https://spritely.institute/news/spritely-goblins-v0-18-0-sleepy-actors.html)
- [OCapN repo (pre-spec status)](https://github.com/ocapn/ocapn)
- [Spritely Introducing OCapN](https://spritely.institute/news/introducing-ocapn-interoperable-capabilities-over-the-network.html)
- [LWN, A Spritely distributed-computing library](https://lwn.net/Articles/960912/)
- [E (programming language) Wikipedia](https://en.wikipedia.org/wiki/E_(programming_language))
- [HN 26665387 (distributed GC critique)](https://news.ycombinator.com/item?id=26665387)
- [Cloudflare Workers RPC](https://blog.cloudflare.com/javascript-native-rpc/)
- [CRGC: Fault-Recovering Actor Garbage Collection in Pekko](https://dl.acm.org/doi/pdf/10.1145/3729288)
- [Spritely Petnames paper](https://files.spritely.institute/papers/petnames.html)
