**Date:** 2026-06-08
**Status:** active
**Subject:** Token-based delegation + attenuation — UCAN (delegation vs invocation), Macaroons (third-party caveats), Biscuit (Datalog + its determinism caveat)

# Token attenuation & the delegation/invocation split

Three capability-*token* formats, read specifically for what they teach about
**long-lived grants vs per-use delivery** and **attenuation under revocation**.
Wire-format detail and provenance live in
[`capability-tokens/`](../capability-tokens/README.md); this file is the
streaming-subscription cut.

## UCAN 1.0 — delegation ≠ invocation (the key split)

UCAN's most useful idea for Myrhiza is that it **separates two artifacts** that
naive systems conflate:

- **Delegation** (`ucan/dlg@1.0.0-rc.1`) — "passes and secures authority in a
  partition-tolerant manner." Fields: `iss`, `aud`, `sub` (subject DID or null),
  `cmd` (the command to *eventually* invoke), `pol` (policy/caveats), `exp`,
  `nonce`. A delegation is the **long-lived grant**. `cmd` is a `/`-delimited path
  and "covers the exact command and all commands nested under it" — hierarchical
  attenuation.
- **Invocation** (`ucan/inv@1.0.0-rc.1`) — "exercises the delegated authority." A
  request to perform one Task, backed by a delegation chain, yielding a **Receipt**.
  Each invocation carries its own `nonce`, so **one delegation backs many
  invocations over time**.

Status (verified 2026-06-08): both are **`1.0.0-rc.1`** — release *candidate*, not
final. Encoding is **DAG-CBOR** for canonical signing (DAG-JSON permitted for
presentation). A separate **UCAN Promise** spec extends invocation with pipelining
("RECOMMENDED"); receipts may request the invoker enqueue further tasks (`cause`
links the chain).

**Mapping to Myrhiza:** delegation = the *subscription grant* (long-lived, scoped
to a topic, attenuable via policy, expirable). Per-message delivery is *not* an
invocation in UCAN's sense — UCAN has no streaming primitive (delegation covers
commands, not message streams). So Myrhiza borrows the *split* (durable grant vs.
per-event act) but must supply its own delivery channel. See
[`open-problems.md`](open-problems.md).

## Macaroons — third-party caveats = "subscribe only if X approves"

Macaroons (Birgisson, Politz, Erlingsson, Taly, Vrable, Lentczner; NDSS 2014) are
HMAC-chained bearer tokens. Attenuation = append a **caveat**; the chained HMAC
means a holder can *narrow* but never *widen*. Two caveat kinds:

- **First-party caveat** — a predicate the verifier checks locally (`topic = X`,
  `time < T`).
- **Third-party caveat** — "satisfy this only if a *named third party* attests."
  The holder must obtain a **discharge macaroon** from that third party and present
  it alongside; the verifier recursively checks the discharge. This is exactly the
  "**subscribe only if authority X approves**" shape in Myrhiza's brief: the
  subscription grant can carry a caveat "valid only if the topic's admin issues a
  discharge," and the admin can decline to renew discharges to effect a soft
  revocation.

Macaroons have **no revocation of their own** (bearer tokens; once issued they're
valid until expiry) — third-party discharge is the closest thing: stop discharging
and the cap stops working. This is *expiry-driven* revocation, not instant.

## Biscuit — attenuation in Datalog, and the determinism caveat

Biscuit (Clever Cloud → Eclipse Foundation) is a public-key-signed token with
**offline attenuation** (append a signed block) and policies in a **Datalog**
dialect. For Myrhiza the load-bearing fact is the **determinism caveat**, because
Myrhiza's whole architecture rests on deterministic `state-apply`:

- Biscuit's Datalog is **negation-free**, requires **rule safety** (every head
  variable appears in the body), and evaluates to a **least fixpoint** (reapply
  rules until no new facts). Those structural choices keep evaluation terminating
  and order-independent *in principle*.
- **BUT** the spec does **not** mandate identical execution traces across
  implementations, and there are **no spec'd hard limits** (max facts, max
  iterations, timeout). Different implementations could reach the same fixpoint via
  different evaluation orders. Separately, **Protobuf serialization is not
  guaranteed deterministic**, so a Biscuit implementation must retain the original
  serialized bytes to recompute signatures when appending a block — verbatim from
  the biscuit-haskell library docs: "Protobuf serialization does not have a
  guaranteed deterministic behaviour, so we need to keep the initial serialized
  payload around in order to compute a new signature when adding a block."

**Lesson for Myrhiza:** an embedded policy/Datalog engine is attractive for
expressing attenuation ("deliver only events where author ∈ set"), but **a policy
language is a non-determinism hazard** if its result ever enters canonical state.
Myrhiza already forbids `host.subscribe` from `state-apply` — that ban *is* the
mitigation. Any attenuation predicate on a subscription must run **only** in the
kernel's per-peer delivery path, never inside a state-digest. If Myrhiza adopts a
Datalog-ish caveat language, it must also impose the bounded-evaluation limits
Biscuit leaves unspecified.

## Cross-cutting: attenuation monotonicity

All three are **monotone**: a delegate can only narrow authority, never broaden it
(HMAC chain / signed block / policy intersection). Myrhiza's WIT handle should
enforce the same — an attenuated subscription handle must be derivable from a
broader one but not vice versa, and the kernel must enforce the narrowing at mint
time, not trust the app.

## What can a subscription be attenuated *on*? (structural vs delivery)

A subtlety the token formats gloss but Myrhiza cannot: **two different things hide
under "attenuation," and they live on opposite sides of the canonical boundary.**

- **Structural attenuation — narrows the grant itself.** Which *topics* the handle
  may subscribe to (a subset of the manifest-declared set), and the *mode*
  (read-only vs. able to also publish, if publish is ever folded in). This is a
  monotone property of the capability, enforced by the kernel at mint time, and it
  is *peer-independent* — narrowing topic {X,Y}→{X} is the same on every peer and
  touches no event stream. This is the part that is genuinely "attenuation" in the
  UCAN/Macaroon/Biscuit sense.
- **Delivery-side filtering — narrows the message stream, not the grant.** "Only
  events authored by A," "only since timestamp T," "only matching predicate P" are
  **content filters applied to the per-peer delivery feed**. They are *not* part of
  the grant's authority; they are an optimization/UX convenience on a stream the
  app is already authorized to receive in full. Critically, **which messages a
  filter admits is non-deterministic and peer-local** (depends on what this peer has
  received and in what order) — so a delivery filter must run *only* in the
  non-canonical delivery path and its result must never feed a state-digest, for the
  exact reason `state-apply` rejects `host.subscribe`.

The trap: an example like *"topic X, author A only"* reads as one attenuation but is
really **structural (topic X) + delivery filter (author A)**. Conflating them would
push a content filter into the grant and tempt an implementation to let its result
influence canonical state. Keep the grant's attenuation **structural-only**; treat
every per-event predicate as delivery-side. See
[`open-problems.md`](open-problems.md) §5.

## Implications for Myrhiza

- **Adopt the delegation/invocation split conceptually**: a durable, attenuable,
  expirable *grant* (UCAN-delegation-shaped, content-addressed to a topic) distinct
  from the *live delivery channel*. Don't try to make the grant itself stream.
- **Third-party caveats** are the clean model for "topic admin must approve this
  subscription" — but they give expiry-grade, not instant, revocation. Pair with
  the kernel-as-caretaker instant revocation from
  [`ocap-revocation.md`](ocap-revocation.md).
- **Treat any caveat/policy language as a determinism boundary.** It may gate
  *delivery* (per-peer, non-canonical) but must never feed a digest.

## Sources

- UCAN Delegation v1.0.0-rc.1: <https://github.com/ucan-wg/delegation>
- UCAN Invocation v1.0.0-rc.1 (+ Promise pipelining): <https://github.com/ucan-wg/invocation>
- UCAN spec index (DAG-CBOR): <https://github.com/ucan-wg/spec>
- Macaroons, NDSS 2014 (authors + third-party caveats): <https://www.ndss-symposium.org/ndss2014/ndss-2014-programme/macaroons-cookies-contextual-caveats-decentralized-authorization-cloud/>
- Biscuit specifications (Datalog: no negation, rule safety, least-fixpoint; no hard eval limits): <https://doc.biscuitsec.org/reference/specifications.html>
- Biscuit Protobuf-determinism caveat ("keep the initial serialized payload around … to compute a new signature when adding a block"), biscuit-haskell `Auth.Biscuit.Token` docs: <https://hackage.haskell.org/package/biscuit-haskell-0.4.0.0/docs/Auth-Biscuit-Token.html>
- Sibling corpus: [`capability-tokens/`](../capability-tokens/README.md)
