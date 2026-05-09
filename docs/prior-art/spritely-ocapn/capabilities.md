# Capabilities

Spritely Goblins is the cleanest in-the-wild expression of object-capability discipline, descended directly from Mark Miller's E and earlier Electric Communities Habitat work ([E language](https://en.wikipedia.org/wiki/E_(programming_language))). The discipline reduces to one slogan from the [Spritely whitepaper](https://files.spritely.institute/papers/spritely-core.html): "if you don't have it, you can't use it." Authority follows reference-passing. There is no ambient authority and no global identity check. Everything follows.

## Refs as first-class values

A reference to a Goblins actor *is* the capability to invoke that actor. The runtime has no concept of "the agent who made this call" — calls happen because the caller holds a reference, full stop. References are passable as ordinary arguments and returns, both within a vat (`$`) and across the network (`<-`). The cap mechanism is reference-passing, not a side channel.

A canonical greeter:

```scheme
(define gary (spawn ^greeter "world"))
($ gary "Alice")           ; near, sync
(<- gary 'greet "Bob")     ; far, async vow
```

The same shape works across machines: serialize `gary` to a sturdyref, hand the URL out, and the holder gets `(<- gary 'greet "Bob")` over CapTP. The *act of having the ref* — whether as a Scheme binding or as a sturdyref URL containing a swiss-num — is the authority.

## Attenuation

Attenuation is the composition primitive. A more-powerful object is wrapped to expose a subset of its surface, and the wrapper is what gets passed onward. Concrete patterns from the [Heart of Spritely whitepaper](https://files.spritely.institute/papers/spritely-core.html):

- A **blog admin** holds the full editor object; she spawns a `science-fair-editor` proxy that exposes only `set-title` and `set-body` and hands it to a guest writer.
- A separate `science-fair-reviewer` proxy exposes only `approve` and goes to a different reviewer.
- Each proxy is itself a Goblins actor — *attenuation costs one actor* — so revocation, logging, and rate-limiting compose into the wrapping layer.

The **`Ward`** module ([Ward docs](https://files.spritely.institute/docs/guile-goblins/0.12.0/Ward.html)) provides a sealer-pair pattern for finer attenuation: `spawn-warding-pair` gives you a *warden* and an *incanter*; behaviors sealed by the warden can only be invoked through the matching incanter, and `enchant` produces a proxy that mediates such invocations.

The `spawn-logged-revocable-proxy-pair` utility (used in the whitepaper) returns `(proxy revoke-cell)`; the proxy logs every call and consults `revoke-cell` before forwarding. To revoke, set the cell to `#t`. This is *just composition*: a proxy actor + a cell + the original ref, no special framework primitive. That's the point.

## No-forge invariants

How does the runtime guarantee a guest can't manufacture a reference it wasn't given?

- **Within a vat**: the actormap is opaque to user code. References are wrapped opaque values; there's no public constructor that takes a name and returns a ref to "any actor with that name." Lexical scoping is the enforcer — to get a ref, someone must have passed it to you.
- **Across vats**: the imports/exports tables are session-scoped indexed by integers, but those integers are wire-side state in a session you already opened. You can't fabricate `desc:import-object 7` and have it bind to anything you weren't told about.
- **Across machines**: sturdyrefs require swiss-numbers, which are unguessable strings (cryptographically-sized random tokens). Without the swiss-num you can't even ask the bootstrap object's `fetch` for it. Network identity itself is a public key (Tor onion ID, TLS cert hash, libp2p peer ID), so a man-in-the-middle can't spoof a destination machine.
- **Sealers** ([Sealers docs](https://files.spritely.institute/docs/guile-goblins/0.13.0/Sealers.html)) extend this to *values*: a sealed object can only be unwrapped by the matching unsealer, providing nominal-typing-like guarantees implemented as a closure over a private cookie.

The slogan from [the whitepaper](https://files.spritely.institute/papers/spritely-core.html): "lambda is already the ultimate security mechanism." The lexical scope of the host language is the TCB.

## Comparison to Holochain capabilities

Holochain's `ZomeCallCapGrant` model (see [holochain/capabilities.md](../holochain/capabilities.md)) is *capability-shaped* but operates at a coarser grain:

| Dimension | Holochain | Spritely Goblins |
|---|---|---|
| **What is a cap?** | A `(secret, zome, fn)` triple stored as a source-chain entry. | A live actor reference (handle), or a sturdyref+swiss-num pair for the persistent form. |
| **Granularity** | Per-zome-function. Cannot say "this counter, only `increment`, only 5 calls." | Per-method per-instance (a proxy actor can wrap any method+arity+precondition). |
| **Composition** | None at the cap layer. You exchange secrets out-of-band. | Native: pass refs as arguments and returns, wrap to attenuate, seal to brand. |
| **Forgeability** | Bearer secrets in `Transferable` mode leak; `Assigned` adds pubkey-binding but the secret itself is plaintext on both source chains. | Refs are first-class but not forgeable — there's no constructor that takes a name and produces a ref. Swiss-nums close the network gap. |
| **Pipelining** | None. Each `call_remote` is a full round-trip. | Built into the wire protocol via `answer-pos`. |
| **Distributed GC** | Grants live forever in the source chain; revocation is `delete_cap_grant`, an *additional* action. | Refcount-based via `op:gc-exports`/`op:gc-answers`; revocation is "drop the proxy and let GC reclaim it." |

Holochain capabilities are a retrofit — bearer tokens with optional pubkey-binding bolted onto a per-zome-function exposure model. Spritely's are the actual ocap discipline encoded in the language. Strictly stronger.

## Comparison to Component Model handles

WIT's `resource` types and `(own $r) / (borrow $r)` semantics are also genuine ocaps: handles are non-forgeable inside the host, transferable as values, scope-bounded by ownership transfer. The two models converge here — a Component Model `resource` and a Goblins near ref are the same primitive. Where Spritely goes further:

1. **Promise pipelining.** WIT method calls are synchronous in the abstract; even with the future async story, there is no wire-level analog of `desc:answer N` that lets you pin a not-yet-existing return value as a target. Component Model leaves this to library convention; Spritely makes it a protocol invariant.

2. **Distributed GC.** Component handles are well-defined within a single component instance graph; what happens when a handle has to outlive a process is an open Component Model question. Spritely answers it: refcount across CapTP sessions, sturdyrefs for persistence, plus the explicit acknowledgment that *cycles spanning machines leak*.

3. **Far refs as a first-class concept.** WIT doesn't yet distinguish "this handle is to a resource in this same instance" from "this handle proxies across an asynchronous boundary." Spritely makes the synchrony boundary visible at the call-site syntax (`$` vs `<-`), which is load-bearing for reasoning about transactionality.

4. **Sealers as a built-in.** Component Model has no equivalent; one would build it on top of resources.

The right reading: Component Model gives Myrhiza *the right typed-handle primitive at the language level*; Spritely gives the design pattern for what to build on top of it once you go peer-to-peer.

## Sources

- [Heart of Spritely whitepaper](https://files.spritely.institute/papers/spritely-core.html)
- [Sealers — guile-goblins 0.13.0](https://files.spritely.institute/docs/guile-goblins/0.13.0/Sealers.html)
- [Ward — guile-goblins 0.12.0](https://files.spritely.institute/docs/guile-goblins/0.12.0/Ward.html)
- [What is Goblins? (Racket docs)](https://docs.racket-lang.org/goblins/intro.html)
- [Conceptual Introduction to Goblins (Bovid blog)](https://blog.bovid.space/conceptual-intro-to-spritely-goblins.html)
- [E (programming language) — Wikipedia](https://en.wikipedia.org/wiki/E_(programming_language))
- [awesome-ocap](https://github.com/dckc/awesome-ocap)
- [Holochain capabilities (sibling prior-art doc)](../holochain/capabilities.md)
