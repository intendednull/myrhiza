# Lessons for Myrhiza

This is the consult-this-when-designing file. The other Spritely prior-art files are evidence; this file is decisions.

## Validates

These Myrhiza design choices are confirmed by Spritely / OCapN / E experience:

- **Capabilities as unforgeable references, not bearer tokens.** Spritely's CapTP refs and the E lineage demonstrate the model end-to-end: refs are the only authority, they flow as message arguments, they compose. WIT resource handles in the Component Model are the same idea expressed in a typed module system. See [`captp-and-ocapn.md`](captp-and-ocapn.md), [`capabilities.md`](capabilities.md).
- **Promise pipelining as a first-class wire feature.** Cap'n Proto and CapTP both make the same bet: the round-trip cost of "ask where, then call there" is unacceptable; the protocol must let you address the result of an in-flight call. Production validation comes from Cloudflare Workers RPC. Borrow the wire pattern. See [`captp-and-ocapn.md`](captp-and-ocapn.md).
- **Sturdyref-as-URI for capability bootstrap.** Cryptographic, unguessable, transferable out-of-band. The Tahoe-LAFS / OCapN convergence on "capability is its own URI" is a genuinely portable design. See [`persistence.md`](persistence.md), [`comparisons.md`](comparisons.md).
- **Vat = isolation unit + transactional turn.** Per-vat single-threaded turn semantics gives clean local reasoning and makes time-travel debugging tractable. Component Model components are an analogous isolation unit. See [`architecture.md`](architecture.md).
- **Distributed acyclic GC is enough for v1.** Spritely's honest scope reduction — drop cross-network cycle collection but keep acyclic GC — is the right MVP. Cycles are an order of magnitude more complex and rarely the load-bearing case. See [`captp-and-ocapn.md`](captp-and-ocapn.md), [`critiques.md`](critiques.md).
- **Petnames for human-readable naming on top of cryptographic refs.** Spritely's three-tier model (petnames / self-proclaimed / edge names) is well-thought-out and directly applicable. See [`ecosystem.md`](ecosystem.md).
- **Netlayer pluralism (Tor, libp2p, TCP+TLS, WebSocket).** Decoupling the capability protocol from the transport is correct; multiple netlayers behind one CapTP runtime is a proven shape. See [`captp-and-ocapn.md`](captp-and-ocapn.md).

## Avoid

| Pitfall | Source | Mitigation |
|---|---|---|
| **Single-host-language ecosystem.** Spritely-canonical = Guile. Anyone wanting a Rust client must port the protocol from a not-yet-stable spec. | [`implementations.md`](implementations.md), [`critiques.md`](critiques.md) | Ship the wire protocol and reference implementation in the canonical Myrhiza language (Rust + Component Model), but treat the *protocol* as the artifact, not the implementation. Publish a stable wire spec before declaring 1.0. |
| **Pre-specification status that lasts years.** OCapN has been "still pre-specification" since 2022; multi-year drift creates incompatible dialects. | [`captp-and-ocapn.md`](captp-and-ocapn.md), [`critiques.md`](critiques.md) | Freeze the wire format at v0.1. Version explicitly. Allow multiple wire versions to coexist in the runtime. |
| **No flagship app after 5+ years.** Goblin Chat, Mandy, Goblinville — all sub-100-user demos. Even Spritely's own framing of the Shepherd port is "the largest deployment *will be*." | [`apps.md`](apps.md), [`critiques.md`](critiques.md) | Make the first Myrhiza demo target real end users, not framework developers. Demo apps that are "elegant 150-LOC" are not the same as users-day-1. |
| **No discovery primitive at the protocol layer.** OCapN is structurally introduce-then-invoke; "find a stranger" is out of scope. | [`open-problems.md`](open-problems.md), [`captp-and-ocapn.md`](captp-and-ocapn.md) | Decide explicitly whether discovery is in or out of Myrhiza's scope. If in, layer a DHT or gossip-bootstrap on top of the cap layer. If out, document the gap loudly and provide an integration story for app-level discovery. |
| **Performance not in the critical path.** Goblins gets faster release-by-release; nobody publishes Goblins-vs-gRPC throughput numbers. | [`critiques.md`](critiques.md) | Publish benchmarks vs gRPC, Cap'n Proto, NATS, libp2p ping-pong, from MVP. If Myrhiza is 10× slower than gRPC, know it. |
| **Browser viability via runtime port (Hoot-WASM).** Spritely had to compile its host language to WASM 3.0 GC + tail-calls; Safari support is partial; the toolchain is an entire sub-project. | [`implementations.md`](implementations.md) | Component Model + jco gives Myrhiza browser viability without a host-language port. Don't rebuild this. |
| **No production deployment evidence.** Spritely is a 501(c)(3) research institute; that is honest, but it means there is no operational hardening data. | [`apps.md`](apps.md) | Run Myrhiza's reference services in production from day one (the project's own bug tracker, chat, governance). Eat the cooking. |
| **No Sybil resistance, no global identity.** OCapN names objects, not principals. | [`open-problems.md`](open-problems.md) | Document explicitly that the runtime doesn't solve Sybil; expose hooks for app-level membrane proofs. Don't pretend caps-on-the-wire imply identity-on-the-wire. |
| **Distributed GC marketed as solved.** Spritely shipped acyclic distributed GC; cycles are not handled. RMI/DCOM/CORBA are the cautionary tales. | [`captp-and-ocapn.md`](captp-and-ocapn.md), [`critiques.md`](critiques.md) | Be explicit about what distributed GC does and doesn't do. Acyclic-only is fine if documented. Pekko's CRGC is the model for going further. |
| **Promise-pipelining-as-throughput-claim.** Pipelining helps round-trip latency, not raw throughput. | [`captp-and-ocapn.md`](captp-and-ocapn.md), [`critiques.md`](critiques.md) | Don't sell pipelining as a throughput win. Sell it as the latency primitive it is. |
| **Confusing "demo works" with "deployable."** Goblin Chat is 150 LOC of core code; deployable secure chat is dozens of subsystems beyond that. | [`apps.md`](apps.md) | Mark each subsystem as either runtime-shipped, app-pattern-documented, or out-of-scope. No magic. |
| **Session model assumptions that diverge under recovery.** Agoric assumes store-and-forward; Spritely supports live sessions; reconciling backups across them produces irreconcilable state. | [`captp-and-ocapn.md`](captp-and-ocapn.md), [`critiques.md`](critiques.md) | Pick one model (live or store-and-forward) per channel type. Document recovery semantics from MVP. |

## Borrow

Concrete subsystems worth deep study and possible direct adaptation:

1. **CapTP wire format and four-table abstraction (questions / answers / imports / exports).** This is the canonical CapTP machinery from E onward, present in Cap'n Proto, Endo, and Spritely. Steal the abstraction; lift the on-the-wire encoding from OCapN's draft spec where possible. See [`captp-and-ocapn.md`](captp-and-ocapn.md).
2. **Sturdyref encoding (locator + netlayer + hints + swissnum).** The unguessable-token-as-URI design is portable across implementations. Use as the wire shape for Myrhiza persistent capability references. See [`persistence.md`](persistence.md).
3. **Promise pipelining state machine.** Three-table coordination of in-flight calls and their derived references. Both Cap'n Proto and OCapN have working implementations to study. See [`captp-and-ocapn.md`](captp-and-ocapn.md).
4. **Third-party handoff protocol.** The CapTP "introduction" pattern: Alice gives Bob a reference to Carol such that Bob can talk to Carol directly without going through Alice. Non-trivial, well-specified in OCapN drafts. See [`captp-and-ocapn.md`](captp-and-ocapn.md).
5. **Vat snapshot + sturdyref persistence.** The Spritely persistence model — serialize an entire live object graph, restore later, sturdyrefs survive — is a clean design. Bloblin (v0.17) and Sleepy Actors (v0.18) are the load-tested primitives. See [`persistence.md`](persistence.md).
6. **Distributed acyclic GC (drop ack, hand-off counted).** Honest scope reduction from full distributed GC. Document the limit explicitly. See [`captp-and-ocapn.md`](captp-and-ocapn.md).
7. **Petname system (three-tier: petname / self-proclaimed / edge name).** The Spritely petnames paper is the design rationale. Genuinely useful for any P2P runtime that needs human-readable names. See [`ecosystem.md`](ecosystem.md), [`comparisons.md`](comparisons.md).
8. **Time-travel distributed debugger.** Free consequence of transactional vat turns + persisted snapshots. If Myrhiza vats are transactional, debugger comes nearly free. See [`architecture.md`](architecture.md).
9. **Netlayer abstraction.** A protocol-agnostic transport layer (TCP+TLS, Tor, libp2p, WebSocket, prelay) underneath one CapTP. The abstraction is right; just adapt to Myrhiza's iroh-based default transport. See [`captp-and-ocapn.md`](captp-and-ocapn.md).
10. **Syrup canonical s-expression encoding.** Cryptographically-tractable, deterministic, simpler than CBOR for nested structures. Worth comparing to the Component Model's canonical ABI. See [`captp-and-ocapn.md`](captp-and-ocapn.md), [`glossary.md`](glossary.md).

## How to use this file

When designing a Myrhiza feature:

1. Find the row in **Avoid** that names a pitfall close to your design. Read the linked subsystem file for the full evidence.
2. Find the row in **Borrow** that names a primitive close to what you're designing. Read the upstream Spritely / OCapN / Cap'n Proto docs to understand the shape, then adapt for Component Model.
3. Promote any decision into a Myrhiza spec under `docs/specs/` — this file captures what we learn from prior art, not our own decisions.
