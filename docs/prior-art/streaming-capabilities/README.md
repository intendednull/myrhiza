**Date:** 2026-06-08
**Status:** active
**Subject:** Streaming capabilities — how ocap / capability-token systems model long-lived, mediated, attenuable, revocable streams (not one-shot calls)

# Streaming capabilities

How do object-capability and capability-token systems model a capability that is
**long-lived, mediated, attenuable, and revocable** — a *subscription/stream*,
not a one-shot call? This folder surveys four lineages for the answer Myrhiza needs
when designing **`host.subscribe`**: a kernel-mediated capability letting a
sandboxed *interaction* (UI-projection) component subscribe to multiple gossip
topics, receive a per-topic state/event stream, and aggregate them in-sandbox.

The constraints that make this load-bearing: convergence stays **per-topic** in the
kernel (never in the sandbox); subscription is **non-deterministic and peer-local**
(must never enter a state-digest — `state-apply` rejects `host.subscribe`); the
capability must be **scoped, attenuable, revocable, and an unforgeable handle**
across the WASM boundary; topic IDs are content-addressed BLAKE3; it must run native
(Wasmtime) and browser (jco); and the existing **single-use submit-and-poll token
pattern does not fit per-message stream delivery** — an open problem this corpus
informs.

The survey's consistent finding: every lineage **splits a durable grant from a
live, mediated channel**, and makes **revocation a property of the mediator, not
the holder**. Myrhiza's kernel is already that mediator.

## Key facts

| System | What it contributes | Streaming/long-lived? | Revocation | Verified status |
|---|---|---|---|---|
| **Cap'n Proto** | `-> stream` return type; window = socket send-buffer; `setFlowLimit`; promise pipelining | Yes — stream of *calls on a capability* | Caretaker pattern (bolted on, not built in); C++-only flow control | 0.8 introduced streaming, **2020-04-23** |
| **E / Goblins / CapTP** | Caretaker/revoker (revocable forwarder); membrane; sturdyref↔live-ref | Live ref (in-session) vs sturdyref (persistent) | **Caretaker = the canonical revocation *pattern*** (Redell 1974, Miller 2006) — but not shipped as a primitive | Goblins ships no built-in revocable-ref type; the caretaker is hand-rolled from a proxy + a slot |
| **UCAN** | Delegation (durable grant) vs Invocation (per-use) split | Delegation backs many invocations over time; **no stream primitive** | Expiry only (no instant revocation) | Delegation + Invocation **`1.0.0-rc.1`**, DAG-CBOR (RC, not final) |
| **Macaroons** | First/third-party caveats; "subscribe only if X approves" via discharge | Bearer grant, repeatedly usable | **Expiry / refuse-discharge only** — not instant | NDSS **2014**; authors Birgisson, Politz, Erlingsson, Taly, Vrable, Lentczner |
| **Biscuit** | Attenuation in Datalog; the **determinism caveat** | Bearer grant | Expiry only | Negation-free Datalog; spec sets **no hard eval limits**; Protobuf non-deterministic |
| **Component Model** | `own<T>`/`borrow<T>` unforgeable handles; `stream<T>` async channel | `stream<T>` = unbuffered value channel w/ backpressure | Kernel drops/poisons the handle-table entry | `stream<T>` **cannot contain `borrow`** (the load-bearing restriction) |

## Contents

**The decision file (read this first):**
- [**`lessons.md`**](lessons.md) — **validates / avoid / borrow** for `host.subscribe`. Answers the brief's core questions directly. The one file to read before designing.

**Subsystem files (evidence, each independently skimmable):**
- [**`capnproto-streaming.md`**](capnproto-streaming.md) — `-> stream`, window-based flow control, promise pipelining, eager-ack + late errors.
- [**`ocap-revocation.md`**](ocap-revocation.md) — caretaker/revoker (revocable forwarder), membranes, sturdyref↔live-ref, Goblins in practice.
- [**`token-attenuation.md`**](token-attenuation.md) — UCAN delegation/invocation split, Macaroons third-party caveats, Biscuit Datalog + its determinism caveat.
- [**`handles-across-boundaries.md`**](handles-across-boundaries.md) — Component-Model resource handles as capabilities, `stream<T>` delivery, the borrow-in-stream restriction = Myrhiza's open problem.

**Risk list:**
- [**`open-problems.md`**](open-problems.md) — what no surveyed system solves: per-message capability-checked streaming, the WIT-handle↔durable-token bridge, backpressure over a gossip multiplex, distributed revocation, policy determinism, jco async maturity, idle-handle DoS.

## Canonical reading order

1. [`lessons.md`](lessons.md) — the synthesis and the answer (5 min).
2. [`handles-across-boundaries.md`](handles-across-boundaries.md) — the substrate Myrhiza actually runs on; the borrow restriction.
3. [`ocap-revocation.md`](ocap-revocation.md) — the revocation model (kernel-as-caretaker).
4. [`token-attenuation.md`](token-attenuation.md) — the delegation/invocation split + attenuation + determinism hazard.
5. [`capnproto-streaming.md`](capnproto-streaming.md) — flow-control prior art (and what not to copy).
6. [`open-problems.md`](open-problems.md) — the residual risk list.

## Glossary (stub)

- **Grant (durable)** — the long-lived, transferable authority to subscribe to a
  topic; survives restart; UCAN-*delegation*-shaped / sturdyref-shaped.
- **Handle (live)** — the in-session, unforgeable WIT resource (`own<subscription>`)
  the kernel mints from a grant; dies with the session. CapTP "live reference."
- **Enliven** — CapTP's step that turns a durable sturdyref into a live reference
  (a promise resolving to it). Myrhiza's grant→handle minting is this.
- **Caretaker / revoker** — a forwarder holding an enable-slot; the issuer holds the
  switch; flipping it severs the wrapped capability. Redell 1974; Miller 2006.
- **Membrane** — a caretaker that auto-wraps every capability crossing it, sharing
  one enable-slot → transitive revocation.
- **Caveat (first/third-party)** — a Macaroon attenuation predicate; third-party
  caveats require a *discharge macaroon* from a named authority.
- **Delegation vs Invocation** — UCAN's split: granting authority vs exercising it.
- **`own<T>` / `borrow<T>`** — Component-Model owned vs temporarily-loaned resource
  handles; both are unforgeable per-component table indices.
- **`stream<T>` / `future<T>`** — Component-Model unbuffered async channels (`0..N`
  / `0..1` values) with consumer-driven backpressure; cannot contain `borrow`.
- **Backpressure** — consumer read-rate throttling the producer; per-subscription
  and explicit in Myrhiza (never canonical).

## Cross-links to existing corpus

- [`capn-proto/`](../capn-proto/README.md) — full Cap'n Proto / Cap'n Web / Workers RPC project context.
- [`spritely-ocapn/`](../spritely-ocapn/README.md), [`agoric-endo/`](../agoric-endo/README.md) — the research-grade CapTP/ocap lineage (caretaker, sturdyref, sealers).
- [`capability-tokens/`](../capability-tokens/README.md) — wire formats (Macaroons, Biscuit, UCAN, SPKI, ZCAP-LD) in depth.
- [`wasm-component-model/`](../wasm-component-model/README.md) — resource handles + async ABI substrate.
- [`jco/`](../jco/README.md) — browser transpile target (async-ABI maturity risk).
- [`iroh/`](../iroh/README.md) — iroh-gossip transport (the multiplex backpressure runs over).

## Framing disclosure

These docs are written from a **kernel-mediated-Component-Model-host** stance:
Myrhiza brokers all I/O via WIT-typed capabilities and owns the trust boundary, so
every "Implications for Myrhiza" sub-section frames each system's mechanism as
"how the *kernel* should realize this." The corpus is deliberately partisan toward
the **kernel-as-caretaker** conclusion — it treats kernel-owned instant revocation
as available and superior, which is true for *local* handles but glosses the
genuinely hard *distributed/cross-peer grant revocation* problem (surfaced honestly
in [`open-problems.md`](open-problems.md) §4). A second bias: the corpus leans
toward the **host-driven-callback** delivery model over native `stream<T>`, because
jco async maturity is a real risk — a reader who finds the async ABI solid under
both targets should re-weight toward `stream<T>`. This is a learn-from-prior-art
artifact for *one* design (`host.subscribe`), not a neutral survey.

## Sources

- Cap'n Proto 0.8 streaming: <https://capnproto.org/news/2020-04-23-capnproto-0.8.html>
- Caretaker pattern (Redell 1974 attribution): <https://kidneybone.com/c2/wiki/CaretakerPattern>
- Mark Miller, *Robust Composition* (2006): <http://www.erights.org/talks/thesis/markm-thesis.pdf>
- Spritely Goblins CapTP API (sturdyref, enliven): <https://files.spritely.institute/docs/guile-goblins/0.16.1/Using-the-CapTP-API.html>
- Spritely Goblins CapTP protocol (live refs "incredibly cheap, merely represented as integers"): <https://files.spritely.institute/docs/guile-goblins/0.16.1/CapTP-The-Capability-Transport-Protocol.html>
- UCAN Delegation v1.0.0-rc.1: <https://github.com/ucan-wg/delegation>
- UCAN Invocation v1.0.0-rc.1: <https://github.com/ucan-wg/invocation>
- Macaroons, NDSS 2014: <https://www.ndss-symposium.org/ndss2014/ndss-2014-programme/macaroons-cookies-contextual-caveats-decentralized-authorization-cloud/>
- Biscuit specifications: <https://doc.biscuitsec.org/reference/specifications.html>
- Component Model Explainer (resources; borrow-in-stream restriction "could be relaxed in the future"): <https://github.com/WebAssembly/component-model/blob/main/design/mvp/Explainer.md>
- Component Model Concurrency.md (stream/future, `waitable-set.wait`): <https://raw.githubusercontent.com/WebAssembly/component-model/main/design/mvp/Concurrency.md>
- Component Model CanonicalABI.md (stream built-in names, incl. `drop-readable`/`drop-writable`): <https://raw.githubusercontent.com/WebAssembly/component-model/main/design/mvp/CanonicalABI.md>
