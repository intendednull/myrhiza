**Date:** 2026-05-22
**Status:** active
**Subject:** Macaroons — HMAC-chained bearer credentials with first-party and third-party caveats (Google, NDSS 2014)

# Macaroons

Macaroons are bearer credentials authenticated by a chain of HMACs, where each link in the chain corresponds to a *caveat* attenuating the credential. The novelty is that **anyone holding a macaroon can derive a strictly weaker one** by appending a caveat — the derivation requires no contact with the issuer, no asymmetric crypto, and produces a macaroon the original issuer can still verify with its root key.

## Key facts

| Fact | Value |
|---|---|
| Origin | Google research, published NDSS 2014. DOI [10.14722/ndss.2014.23212](https://doi.org/10.14722/ndss.2014.23212) |
| Authors | Arnar Birgisson, Joe Gibbs Politz, Úlfar Erlingsson, Ankur Taly, Michael Vrable, Mark Lentczner |
| Reference impl | [`rescrv/libmacaroons`](https://github.com/rescrv/libmacaroons) — C (with Go + Python bindings), BSD-3-Clause, by Robert Escriva |
| Python impl | [`ecordell/pymacaroons`](https://github.com/ecordell/pymacaroons) — MIT |
| Shipped at | Ubuntu Snappy (snap auth), Matrix (login-via-Single-Sign-On), HyperDex data store, [PyPI](https://pypi.org/) (publish tokens) |
| Wire format | Binary (length-prefixed packets) or text (JSON-ish); not standardized as RFC |
| Crypto | HMAC-SHA-256 only — **no asymmetric primitive needed** |

## Core construction

A macaroon is a tuple `(location, identifier, [caveats], signature)` where:

```
sig_0 = HMAC(root_key, identifier)
sig_i = HMAC(sig_{i-1}, caveat_i)
final_signature = sig_n
```

Each appended caveat re-keys the HMAC with the previous chain signature. To verify, the issuing service recomputes the chain from the root key it stored against `identifier` and checks the final HMAC matches.

**Crucially: a holder can append a caveat without knowing any secret.** They just compute `sig_{n+1} = HMAC(sig_n, new_caveat)` using the current chain signature as the new key. The original issuer's verification still works because the chain remains computable from the root key forward. The holder cannot *remove* a caveat without breaking the chain (HMAC is preimage-resistant), nor forge a new macaroon (they don't have `root_key`).

This is the ocap discipline at the network layer: **handing someone a capability inherently includes the right to attenuate it.** That's structurally unforgeable — attempts to widen the cap break the chain.

## First-party caveats

The simple case: opaque strings the verifier knows how to interpret.

```
caveat: "account = 12345"
caveat: "expires < 2026-06-01T00:00:00Z"
caveat: "operation in {read, list}"
```

The service receiving the macaroon parses each caveat against its policy. No standard format — every deployment defines its own caveat predicate language. This is a pragmatic strength (no parser to embed) and a weakness (each application reinvents).

## Third-party caveats

The genuinely novel mechanism. A caveat can demand "the holder must additionally present a *discharge macaroon* from third party Z, attesting predicate P."

A third-party caveat carries `(caveat_key, identifier_for_3p_service, location_hint)` where `caveat_key` is encrypted under a shared secret with the third party. To use the macaroon, the holder must:

1. Contact the third-party service (e.g. an auth server) at `location_hint`.
2. Receive a discharge macaroon, itself an HMAC chain rooted in `caveat_key`.
3. Present both the original macaroon AND the discharge to the verifier.

The verifier checks the discharge satisfies the third-party caveat by combining the chain signatures. The third-party service never communicates with the verifier directly — the discharge macaroon *is* the message. This enables **decentralized authorization across mutually-distrusting services** without an OAuth-style ceremony per-request.

The third-party discharge is what gives Macaroons their decentralized-auth credibility. It's also the most complex part of the spec — most deployments skip it.

## What macaroons do NOT solve

- **Revocation.** A leaked macaroon stays valid until any of its embedded time-bound caveats expire. The standard answer is "embed a short expiry caveat and re-issue often" — but for long-lived caps, you need an out-of-band blacklist or a third-party "the issuer has not revoked this" caveat. The paper acknowledges this; Macaroons do not solve revocation.
- **Asymmetric provenance.** All verifications require the *issuer's root key*. A macaroon cannot be verified by a stranger without prior key sharing. (Contrast with Biscuit and UCAN, which use public-key signatures so any party with the public key can verify.) This is by design — Macaroons target the cookie/session-token shape where the issuer is also the verifier.
- **Delegation across security domains.** A macaroon attenuates within a domain controlled by one issuer. Delegating between issuers requires the third-party caveat dance, which has its own discharge service.

## Deployments

- **Ubuntu Snappy** — Canonical's snap store uses macaroons for publish/install authorization ([Snappy authentication](https://snapcraft.io/docs/snap-store-tokens)). The most production-deployment-of-record.
- **Matrix** — Macaroons historically used for Single-Sign-On login tokens ([Synapse SSO docs](https://matrix-org.github.io/synapse/latest/sso_mapping_providers.html)). Newer Matrix deployments use OIDC.
- **HyperDex** — distributed key-value store using macaroons for client-issued caps.
- **PyPI** — `pip publish` uses macaroon-shaped tokens (technically, *very* simplified macaroons).

Notably, the Macaroons paper has been widely cited but the format itself has *not* been widely standardized: there is no IETF RFC, no W3C draft. The reference implementation `libmacaroons` is the de facto standard, plus per-language ports.

## Implications for Myrhiza

The Macaroon **attenuation-by-derivation** primitive is structurally what we want in a peer-symmetric setting: a peer who receives a cap can pass on a strictly-narrower version without contacting any issuer. Three concrete uses worth borrowing:

1. **Per-call attenuation of host capabilities.** When an interaction component wants to delegate a sub-cap to a behavior component (e.g. "you can `network-send`, but only to this peer"), the attenuation primitive avoids a kernel round-trip.
2. **Third-party caveats for off-band authorization.** "You can call this fn, but only if your local DPKI cert is still valid" — the discharge service is the local kernel checking DPKI state.
3. **HMAC-only crypto** is appealing for in-process capability handoffs because we avoid Ed25519 per-op.

But the **HMAC chain** is specifically *not* what we want for peer-to-peer: Myrhiza peers don't share root keys. The shape that translates is Biscuit/UCAN — public-key-signed chains. See [`comparisons.md`](comparisons.md) §"crypto model."

The third-party-caveat construction is conceptually the most useful idea — a *protocol* for "I delegate to you, conditional on a separate service's blessing" that doesn't require the conditioner to talk to the verifier directly. That generalizes beyond HMAC. Worth considering as Myrhiza thinks about cross-peer attenuation.

## Sources

- Birgisson, A.; Politz, J.G.; Erlingsson, Ú.; Taly, A.; Vrable, M.; Lentczner, M. "Macaroons: Cookies with Contextual Caveats for Decentralized Authorization in the Cloud." [NDSS 2014](https://research.google/pubs/macaroons-cookies-with-contextual-caveats-for-decentralized-authorization-in-the-cloud/). DOI [10.14722/ndss.2014.23212](https://doi.org/10.14722/ndss.2014.23212).
- Escriva, R. [`rescrv/libmacaroons`](https://github.com/rescrv/libmacaroons). BSD-3-Clause.
- Cordell, E. [`ecordell/pymacaroons`](https://github.com/ecordell/pymacaroons). MIT.
- Wikipedia. [Macaroons (computer science)](https://en.wikipedia.org/wiki/Macaroons_(computer_science)).
- Canonical. [Snap Store tokens](https://snapcraft.io/docs/snap-store-tokens).
