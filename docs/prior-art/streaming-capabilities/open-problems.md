**Date:** 2026-06-08
**Status:** active
**Subject:** What streaming-capability prior art structurally does NOT solve — Myrhiza's risk list for `host.subscribe`

# Open problems

These are the gaps the surveyed systems leave open. Each becomes a risk Myrhiza
must own, because no prior art hands us the answer.

## 1. No system streams *per-message capability-checked delivery* as a primitive

Every lineage gives a long-lived grant **or** a flow-controlled byte/value stream,
but none gives "a revocable, attenuated capability whose *use* is an unbounded
stream of authorization-checked messages":

- **UCAN** has delegation and invocation but **no streaming** — delegation covers
  *commands*, not message streams; one delegation backs repeated discrete
  invocations, not a live feed.
- **Cap'n Proto** streams *calls on a capability* with flow control, but the cap
  itself isn't attenuable/revocable per-token the way Myrhiza needs (revocation is
  the caretaker pattern bolted on, not built in).
- **Component-Model `stream<T>`** is a typed value channel with backpressure but
  carries *no authorization model* — it's plumbing, not a capability.

→ Myrhiza is **composing** (grant + caretaker + delivery channel) something no
single system ships. The composition is the design work; nothing to copy whole.

## 2. The WIT-handle ↔ durable-token mapping is genuinely unsolved upstream

The Component Model gives unforgeable *in-process* resource handles. Tokens (UCAN,
Macaroons, Biscuit) give *durable, transferable* grants. **Nobody has a standard
bridge** between "this `own<T>` table index" and "this signed bearer/delegation
token that survives restart and crosses machines." CapTP's sturdyref↔live-ref split
is the closest conceptual model but is E/Goblins-specific and not WIT-aware.

→ Myrhiza must *invent* the enliven step: durable grant → minted `own<subscription>`
handle, plus the inverse (revoked grant must refuse future enlivening). Risk:
getting the unforgeable-link between the two layers right (a stale handle must not
outlive a revoked grant; a re-enlivened handle must not resurrect a killed one).

## 3. Backpressure across a *gossip multiplex* has no prior art here

Cap'n Proto's window = one TCP socket buffer. Component-Model streams assume a
point-to-point channel. **Neither models N topics multiplexed over iroh-gossip
(HyParView + Plumtree)**, where a topic's "producer" is the whole swarm, not one
peer, and delivery order is non-deterministic by design.

→ Per-subscription backpressure must be *peer-local and non-canonical* (it must
never affect convergence — back-pressuring topic X on this peer cannot change topic
X's state digest). No surveyed system has to keep flow-control out of a consensus
digest; Myrhiza does. Risk: an app that reads slowly must not stall the kernel's
per-topic replay engine, only its own delivery queue. The *aggregation* side (one
component awaiting N topics) has a clean prior-art hook — the Component Model's
`waitable-set.wait` over N per-topic handles (see
[`handles-across-boundaries.md`](handles-across-boundaries.md) §Multi-topic) — but
the *backpressure* side of that multiplex is the open part: there is no prior art
for per-channel windows over a gossip swarm that must not leak into a digest.

## 4. Revocation propagation in a partition-tolerant P2P setting

The caretaker gives *instant local* revocation (the kernel flips a slot). But a
**grant** may have been delegated/attenuated and handed to other peers' apps. Token
systems "revoke" only by expiry or by refusing third-party discharges — both
*eventual*, both needing the holder to re-check. There is **no instant distributed
revocation** in any surveyed system without a central authority (which Myrhiza
lacks). This mirrors the unsolved revocation-at-scale problem already flagged in
[`capability-tokens/open-problems.md`](../capability-tokens/open-problems.md).

→ Myrhiza can instantly revoke a *local* subscription handle, but revoking a *grant
that was re-delegated to another peer* is at best expiry-grade. Risk: design must be
honest about which revocations are instant (local handle) vs eventual (cross-peer
grant), and probably keep subscription grants **non-re-delegable** to dodge this.

## 5. Policy-language determinism vs canonical state

Biscuit demonstrates that even a *carefully restricted* Datalog (negation-free,
safe, fixpoint) does **not** guarantee identical evaluation across implementations,
and the spec sets no hard limits (max facts/iterations/timeout). Protobuf
serialization isn't deterministic either.

→ Any attenuation predicate Myrhiza attaches to a subscription is safe **only** in
the per-peer delivery path (non-canonical). The moment such a predicate's result
could influence a digest, it's a convergence bug. The existing rule "`state-apply`
rejects `host.subscribe`" is the firewall — but Myrhiza must ensure no *derived*
artifact (e.g. a "filtered event count") leaks the non-deterministic delivery
decision into canonical state.

## 6. Async ABI maturity under jco (browser)

Component-Model `stream<T>`/`future<T>` and the async built-ins are recent
(WASI-0.3-era) and unevenly implemented; Cap'n Proto's flow control is **C++-only**.
Myrhiza must run native (Wasmtime) *and* browser (jco transpile). Risk: betting on
`stream<T>` for delivery couples `host.subscribe` to async-ABI maturity in jco. The
host-driven-callback alternative (resource handle + repeated `on-topic-event`)
de-risks this but is more bespoke.

## 7. Liveness / DoS of long-lived handles

A subscription that's opened and abandoned (app stops reading, never closes) holds
kernel resources indefinitely. `stream.cancel-read`/`stream.drop-{readable,writable}`
exist but require cooperation. Cap'n Proto's `setFlowLimit` is a global backstop but no system
auto-reaps idle long-lived caps.

→ Myrhiza needs a kernel-side reaping/quota policy (max open subscriptions per app,
idle timeout, per-peer in-flight cap) — prior art offers backstops but no complete
answer.

## Sources

- Roll-up of the four subsystem files' findings; see their `## Sources`.
- Related corpus risk lists: [`capability-tokens/open-problems.md`](../capability-tokens/open-problems.md), [`capn-proto/open-problems.md`](../capn-proto/open-problems.md).
