**Date:** 2026-05-22
**Status:** active
**Subject:** Comparison matrix — Macaroons, Biscuit, UCAN, PASETO, SPKI, ZCAP-LD, JWT

# Comparisons

Cross-format trade-offs for the cap-token contenders, with JWT and PASETO included for contrast (they're not cap tokens, but they sit in the same wire shape and are commonly mistaken for one). The shape of this table is intended to support **format-selection** questions: "given these requirements, which format do we reach for first?"

## At a glance

| Format | Cap token? | Attenuation | Delegation chain | Verifier needs | Crypto model | Wire format | IETF/W3C |
|---|---|---|---|---|---|---|---|
| **Macaroons** | Yes | First/third-party caveats | Single issuer | Root key (shared) | Symmetric (HMAC) | Binary or text | None |
| **Biscuit** | Yes | Datalog blocks | Yes (per-block keys) | Issuer pubkey + Datalog eval | Asymmetric (Ed25519/ECDSA) | Protobuf | None |
| **UCAN v1.0** | Yes | Policy clauses | Yes (DID chain) | DID resolution + pubkey | Asymmetric (Ed25519/RSA/secp256k1 via DID) | DAG-CBOR | DIF working group |
| **UCAN v0.10** | Yes | Caveats in resource | Yes (DID chain) | DID resolution + pubkey | Asymmetric (per DID method) | JWT (JOSE) | Same |
| **SPKI** | Yes | Authorization S-expr | Yes (5-tuple chain) | Issuer pubkey | Asymmetric (RSA/DSA era) | Canonical S-expressions | RFC 2693 (Experimental) |
| **ZCAP-LD** | Yes | Caveats in invocation | Yes (chain of capabilityInvocations) | LD-Proof verifier + DID | Asymmetric (Ed25519Signature2020) | JSON-LD | W3C CCG draft (no Rec) |
| **PASETO** | **No** | None | None | Symmetric key OR pubkey | v3 = NIST; v4 = modern | base64url + JSON | None (expired draft) |
| **JWT** | **No** | None | None | Symmetric key OR pubkey | per `alg` header | base64url + JSON | RFC 7519 (Standards Track) |

## Per-axis discussion

### Crypto model

- **Symmetric (HMAC) — Macaroons, JWT-HSxxx, PASETO-local.** Issuer and verifier must share a secret. Cheap to verify, but caps can only be issued by parties holding the secret. No third-party verification.
- **Asymmetric (signature) — Biscuit, UCAN, SPKI, JWT-RS/ES, PASETO-public, ZCAP-LD.** Anyone with the issuer's public key can verify. Required for any peer-to-peer use case.

For Myrhiza (peer-symmetric, no shared secrets between unrelated peers), **asymmetric is mandatory**. This rules out Macaroons in their pure form, though Macaroon-shaped HMAC chains may still have a role for in-process attenuation where the kernel is both issuer and verifier.

### Attenuation mechanism

| Format | How attenuation works |
|---|---|
| Macaroons | Append a caveat; HMAC the previous signature with the caveat to extend the chain. No new key material. |
| Biscuit | Append a block with stricter Datalog checks; sign with the ephemeral key from the previous block. New ephemeral keypair per attenuation step. |
| UCAN v1.0 | Issue a new Delegation, citing the previous as `prf`. New signature by the previous audience. |
| SPKI | Issue a new 5-tuple with stricter authorization; sign with the previous subject's key. |
| ZCAP-LD | Issue a new capability with `parentCapability` pointing at the previous; LD-Proof signed. |
| PASETO/JWT | **No native attenuation.** Either re-issue from authority, or build the chain at the application layer. |

The Macaroon "no new keys needed" property is unique. Biscuit/UCAN/SPKI all require a fresh signing operation per delegation step.

### Delegation chain depth

All cap-token formats support arbitrary delegation depth in principle. In practice:

- **Macaroons** chain depth is limited by the caveat-encoding overhead — each caveat adds bytes; HTTP cookie limits become real around 5–10 caveats.
- **Biscuit** chain depth is bounded by Protobuf size (Biscuit designs for HTTP cookie size).
- **UCAN** chain depth can be arbitrary because the proofs are content-addressed CIDs (delegations are stored separately and referenced) — but verification cost grows linearly with chain depth, and CID resolution may require IPFS/network calls.
- **SPKI** chains are length-unbounded but face the same verifier-cost-grows-linearly issue.
- **JWT/PASETO** have no chain; there's a single signature.

### Caveat / policy expressiveness

| Format | Policy language | Power |
|---|---|---|
| Macaroons | Application-defined predicate strings | Whatever the verifier implements |
| Biscuit | Datalog (no negation, with closures, expressions) | High — Turing-incomplete but rich |
| UCAN v1.0 | Structured `pol` clauses (range, equals, prefix, etc.) | Medium — predicate-DSL, not a general language |
| UCAN v0.10 | Free-form `caveats` JSON | Application-defined |
| SPKI | Authorization S-expression with intersection rules | Custom per application |
| ZCAP-LD | `caveat` array of LD-typed clauses | Application-defined |

Datalog is the most powerful but the most expensive to embed. For a deterministic `state-apply` component, Datalog evaluation order matters and is non-obvious; UCAN's smaller `pol` DSL is easier to make deterministic.

### Revocation

**Every format here has the same revocation answer: out-of-band.** No format solves revocation natively because solving it requires a shared, eventually-consistent revocation list — and that contradicts the "decentralized verification" goal.

| Format | Recommended revocation pattern |
|---|---|
| Macaroons | Short expiry + re-issue; or third-party "issuer hasn't revoked" caveat. |
| Biscuit | `revocation_id` per block; verifier checks a blacklist. |
| UCAN v1.0 | Dedicated Revocation spec; revocation tokens distributed out-of-band. |
| SPKI | Online-validation directive in validity field. |
| ZCAP-LD | Revocation list (W3C VC StatusList2021). |
| JWT/PASETO | Blacklist `jti` (token ID); requires stateful verifier. |

For Myrhiza: revocation is going to require **per-peer revocation gossip**, regardless of which format we pick. Picking a format that has a revocation primitive (UCAN v1.0, Biscuit's `revocation_id`) gives us a hook to hang it off.

### Identity model

| Format | Issuer identity |
|---|---|
| Macaroons | Implicit (which root key the verifier has) |
| Biscuit | Raw public key |
| UCAN | DID (`did:key` most common; `did:web`, `did:pkh`, etc.) |
| SPKI | Public key or hash thereof |
| ZCAP-LD | DID (W3C-blessed methods) |
| JWT/PASETO | `iss` claim (string; identity model is app-defined) |

UCAN's DID model is the richest. For Myrhiza, DID-rooted is the right abstraction because each peer is already keypair-identified.

### Verifier complexity

| Format | What a verifier must implement |
|---|---|
| Macaroons | HMAC-SHA-256; predicate parser for first-party caveats; HTTP fetch for third-party discharge |
| Biscuit | Ed25519 + ECDSA sigs; Protobuf decoder; **full Datalog evaluator** |
| UCAN v1.0 | Ed25519/RSA/secp256k1 sigs (per DID method); DAG-CBOR; DID resolver; policy-clause evaluator |
| UCAN v0.10 | Ed25519/RSA/secp256k1 sigs; JOSE/JWT parser; DID resolver |
| SPKI | RSA/DSA sigs (era-appropriate); canonical S-expression parser; reduction algorithm |
| ZCAP-LD | LD-Proofs; JSON-LD canonicalization; DID resolver |
| PASETO | Per-version primitive only; JSON parser |
| JWT | Per-`alg` primitive; JSON parser |

The verifier-cost-to-feature ratio matters a lot for embedded/browser/`state-apply` contexts. UCAN is heavier than it looks because of the DID-resolution cost — `did:key` is cheap but `did:web` requires HTTPS. Biscuit's Datalog evaluator is the largest single cost.

### Browser / constrained-runtime viability

| Format | Browser viability |
|---|---|
| Macaroons | Easy — HMAC-SHA-256 is in Web Crypto. |
| Biscuit | Possible but heavy — Protobuf + Datalog evaluator must ship in JS/WASM. |
| UCAN v1.0 | DAG-CBOR + Ed25519 in Web Crypto. Workable. |
| UCAN v0.10 | JWT, well-supported. |
| SPKI | No browser tooling. |
| ZCAP-LD | JSON-LD canonicalization in browser is slow. |
| PASETO/JWT | Trivial. |

For Myrhiza's eventual browser-runtime story, **UCAN v1.0 with `did:key` is the cleanest fit**. Biscuit is feasible but the Datalog evaluator weighs against it.

### Spec maturity & ecosystem

| Format | Spec status | Ecosystem health |
|---|---|---|
| Macaroons | NDSS 2014 paper; no IETF | Mature — `libmacaroons` + ports stable for ~10y |
| Biscuit | v3.3 (Dec 2024); Eclipse Foundation | Active — recent move to Eclipse signals institutional commitment |
| UCAN | v1.0.0-rc.1 (Mar 2026); DIF | Active but mid-migration; v0.10 deployed, v1.0 not yet final |
| SPKI | RFC 2693 Experimental (1999) | Effectively abandoned; design influence only |
| ZCAP-LD | v0.3 W3C CCG draft | Stalled — never reached W3C Recommendation |
| PASETO | Community spec (no RFC) | Stable; small community |
| JWT | RFC 7519 (2015) | Vast — the deployed default |

UCAN's "mid-migration" status is the biggest risk factor for new adopters. Biscuit's recent move to Eclipse is the biggest positive signal for institutional adoption.

## JWT: what NOT to do

JWT is included here because it's the format people reach for when they want a cap token but don't understand the distinction. Three structural problems make JWT *unsafe as a capability token*:

1. **No native attenuation.** A JWT-style "scope" claim is an arbitrary string the verifier interprets. To restrict a JWT, you have to re-issue it from the issuer — which means the issuer must always be online, which contradicts the decentralized-cap thesis.
2. **Algorithm agility (`alg` header) creates confusion attacks.** The `alg: none` family of attacks, and the RS256/HS256 key-confusion family, have been responsible for years of JWT CVEs. PASETO's "version pins the algorithm" is the structural fix.
3. **Claims are not capabilities.** A JWT `sub: alice, scope: read` says "Alice is allowed to read." A capability token says "the bearer of this token can read [this specific resource]." Different model. JWT is suitable for authentication tokens (session bearers); it's not designed for cap delegation chains.

Sven Slootweg's ["Stop using JWT for sessions"](http://cryto.net/~joepie91/blog/2016/06/13/stop-using-jwt-for-sessions/) (2016) is the canonical critique. Paragon IE's ["No way, JOSE!"](https://paragonie.com/blog/2017/03/jwt-json-web-tokens-is-bad-standard-that-everyone-should-avoid) (2017) is the JWT-vs-PASETO motivation. Both are worth reading.

## Decision matrix for Myrhiza

If we want a cap token format **today**, the candidates rank:

| Criterion | Macaroons | Biscuit | UCAN v1.0 | SPKI |
|---|---|---|---|---|
| Peer-to-peer (asymmetric crypto) | — (HMAC only) | Yes | Yes | Yes |
| DID-rooted identity | — | — (raw pubkey) | **Yes** | — |
| Browser-viable | Yes | Heavy (Datalog) | Yes | — |
| Spec maturity | Mature | Mature | Mid-migration | Abandoned |
| Verifier complexity | Low | High | Medium | Low (paper only) |
| Revocation primitive | — | `revocation_id` | Revocation spec | Online-validation |
| Content-addressable encoding | — | — | **Yes (DAG-CBOR)** | — |

**Reading:** UCAN v1.0 is the closest fit on identity/encoding/spec direction; Biscuit is the closest fit on production-readiness and tooling. The honest answer is "we may want to lift UCAN's delegation model but choose our own wire format pinned to the WIT-resource-handle ontology." See [`lessons.md`](lessons.md).

## Sources

- All per-format `## Sources` sections of [`macaroons.md`](macaroons.md), [`biscuit.md`](biscuit.md), [`paseto.md`](paseto.md), [`ucan.md`](ucan.md), [`spki.md`](spki.md).
- Slootweg, S. ["Stop using JWT for sessions."](http://cryto.net/~joepie91/blog/2016/06/13/stop-using-jwt-for-sessions/) 2016-06-13.
- Arciszewski, S. ["No way, JOSE!"](https://paragonie.com/blog/2017/03/jwt-json-web-tokens-is-bad-standard-that-everyone-should-avoid) Paragon IE blog, 2017.
- Auth0. ["Critical vulnerabilities in JSON Web Token libraries."](https://auth0.com/blog/critical-vulnerabilities-in-json-web-token-libraries/) Tim McLean's `alg: none` writeup.
