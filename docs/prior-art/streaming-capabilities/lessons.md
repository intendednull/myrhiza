**Date:** 2026-06-08
**Status:** active
**Subject:** Decision file — what streaming-capability prior art validates / tells Myrhiza to avoid / tells Myrhiza to borrow for `host.subscribe`

# Lessons for Myrhiza: `host.subscribe`

The one file to read before designing `host.subscribe`. Every bullet is tied to
the design. Evidence is in the subsystem files; this is the synthesis.

The central question — *how to represent "a subscription to topic X" as an
unforgeable, per-topic-scoped, attenuable, revocable handle across the WASM
boundary, with the right delegation/delivery split, flow control, and a
revoker* — has a consistent answer across four independent lineages. They all
**split a durable grant from a live, mediated channel**, and they all make
**revocation a property of the mediator, not the holder**.

## Validates

- **The two-layer split (durable grant vs live handle) is the right shape.**
  UCAN splits *delegation* (long-lived authority) from *invocation* (one use);
  CapTP splits *sturdyref* (persistent URI) from *live reference* (in-session
  integer); Cap'n Proto splits the *capability* from the *stream of calls on it*.
  Myrhiza's instinct — a manifest-declared, content-addressed grant that the kernel
  *enlivens* into a per-session WIT handle — is exactly this pattern. Reconnect =
  re-enliven; the grant outlives the handle.
  ([`token-attenuation.md`](token-attenuation.md), [`ocap-revocation.md`](ocap-revocation.md))
- **A resource handle is a sound unforgeable capability across the WASM boundary.**
  Component-Model `own<T>` handles are unforgeable per-component table indices the
  app can't fabricate. A subscription as `own<subscription>` gives Myrhiza the ocap
  "reference = authority" property *enforced by the ABI*, for free.
  ([`handles-across-boundaries.md`](handles-across-boundaries.md))
- **Revocation belongs to the mediator.** The caretaker/revoker pattern (Redell
  1974, Miller 2006) puts the enable-slot in the *issuer's* hands, not the holder's.
  Myrhiza's kernel already *is* that mediator — it owns the handle table and the
  network. Kernel-flips-the-slot revocation is instant and, **for local handles**,
  strictly stronger than what Goblins or Macaroons offer. (Revoking a *grant
  re-delegated to another peer* is only expiry-grade — see
  [`open-problems.md`](open-problems.md) §4.)
  ([`ocap-revocation.md`](ocap-revocation.md))
- **Backpressure should be consumer-driven.** The Component Model's unbuffered
  completion-based streams throttle the producer when the consumer reads slowly —
  the clean model. The subscription's consumer (the sandboxed UI component) sets the
  pace. ([`handles-across-boundaries.md`](handles-across-boundaries.md))
- **Attenuation must be monotone — and structural, not content.** UCAN, Macaroons,
  and Biscuit all enforce narrow-only delegation. A subscription handle's
  *structural* attenuation (which topics, which mode — e.g. {X,Y}→{X}, read-only)
  must be derivable from a broader grant, never the reverse, enforced at mint-time
  by the kernel. **Do not fold a per-event content filter ("author A only") into the
  grant** — that is delivery-side, peer-local, and non-canonical; keep it out of the
  capability's authority. ([`token-attenuation.md`](token-attenuation.md))

## Avoid

- **Do NOT model the subscription as a single-use token or a one-shot call.**
  Myrhiza's existing async pattern (a `*-submit` returns a single-use request-token;
  kernel re-enters via `on-*-completion`) does **not** fit per-message stream
  delivery — the brief flags this and the prior art confirms it. A subscription is a
  *long-lived capability repeatedly delivered to*, like a Cap'n Proto streaming
  capability, not a token consumed once. Generalize the re-entry handler from
  single-use to *reusable handle + repeated `on-topic-event`*.
- **Do NOT copy Cap'n Proto's socket-buffer-as-window flow control.** It works only
  because one capability rides one TCP socket. Myrhiza multiplexes many topics over
  iroh-gossip; one OS buffer can't express per-topic backpressure. Make the window
  **per-subscription and explicit**. ([`capnproto-streaming.md`](capnproto-streaming.md))
- **Do NOT let an attenuation/policy language touch canonical state.** Biscuit's
  Datalog is negation-free and fixpoint-evaluated yet *still* doesn't guarantee
  identical traces across implementations, and the spec sets no hard evaluation
  limits; Protobuf serialization isn't even deterministic. A caveat language is a
  non-determinism hazard. It may gate *per-peer delivery* (already non-canonical)
  but must never feed a state-digest — which is exactly why `state-apply` must
  reject `host.subscribe`. ([`token-attenuation.md`](token-attenuation.md))
- **Do NOT rely on expiry/discharge as your only revocation.** Macaroons can only
  "revoke" by ceasing to issue discharge macaroons — expiry-grade, not instant.
  Bearer tokens with no central state can't be pulled back. Myrhiza needs instant
  revocation, so the kernel-caretaker is mandatory; token expiry is at most a backstop.
- **Do NOT put `borrow` handles inside a delivered stream.** The Component Model
  *currently* forbids `stream<T>`/`future<T>` whose `T` contains `borrow`
  (dangling-reference hazard across async suspension); the spec notes this "could be
  relaxed in the future," but it's load-bearing today. If `stream<T>` is used,
  deliver owned values or plain records.
  ([`handles-across-boundaries.md`](handles-across-boundaries.md))
- **Do NOT make subscriptions mint further capabilities** (avoid needing membranes).
  Keep them leaf-shaped so single-caretaker O(1) revocation suffices; a subscription
  that hands out sub-caps would need a membrane to revoke transitively.

## Borrow

- **The delegation/invocation vocabulary and split** (UCAN). Name the durable grant
  and the live act separately in Myrhiza's own design; one grant backs many
  deliveries over time (each with its own nonce/sequence). Don't import UCAN tokens
  wholesale — UCAN has *no streaming primitive* (delegation covers commands, not
  message streams) — borrow the *structure*.
- **The caretaker/revoker as the kernel's internal model** (E / Miller). The WIT
  handle the app holds is the forwarder; the kernel holds the enable slot; revoke =
  flip the slot, next `host.*` call on the handle fails. Make this a kernel
  primitive, not a pattern apps assemble.
- **Sturdyref `enliven` → promise → live reference** (CapTP) as the
  reconnect/restart story. The durable grant is content-addressed (topic = BLAKE3)
  *plus* an unforgeable component so a revoked grant can't be re-enlivened. Enliven
  returns immediately (a promise) — no blocking round-trip.
- **Third-party caveats** (Macaroons) as the model for "topic admin must approve
  this subscription": a grant caveat "valid only if admin issues a discharge,"
  discharge-renewal declined = soft revoke. Use *alongside* kernel instant
  revocation, not instead of it.
- **Consumer-driven unbuffered backpressure + `stream.cancel-read`/`drop-*`
  lifecycle** (Component Model streams) as the delivery-channel semantics, even if
  delivery is a host-driven callback rather than a literal `stream<T>`.
- **`setFlowLimit`-style global backstop** (Cap'n Proto) — a kernel-wide cap on
  total in-flight delivered-but-unacked events across *all* a peer's subscriptions,
  as DoS protection independent of per-subscription windows.

## The one-line answer to the brief's core question

Represent a subscription as a **kernel-owned caretaker fronted by an
`own<subscription>` WIT resource handle** (unforgeable, per-topic-scoped,
mint-time-attenuated, kernel-revocable), **enlivened from a durable
content-addressed grant** (the delegation, manifest-declared, surviving restart),
with **per-message delivery as a repeated host-driven callback** (not a one-shot
token, not necessarily a `stream<T>`) under **explicit per-subscription
consumer-driven backpressure**. The WIT-handle ↔ cap-token mapping *is* the
sturdyref ↔ live-reference split: the handle is the enlivened token.

## Sources

See the per-file `## Sources` sections:
[`capnproto-streaming.md`](capnproto-streaming.md),
[`ocap-revocation.md`](ocap-revocation.md),
[`token-attenuation.md`](token-attenuation.md),
[`handles-across-boundaries.md`](handles-across-boundaries.md).
