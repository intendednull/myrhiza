**Date:** 2026-05-22
**Status:** active
**Subject:** SPKI — Simple Public Key Infrastructure, RFC 2693 (Ellison et al., IETF 1999) — the classical object-capability token canon

# SPKI (Simple Public Key Infrastructure)

SPKI is the historical ancestor of every capability-token format in this folder. Published as IETF [RFC 2693](https://datatracker.ietf.org/doc/html/rfc2693) in September 1999, it proposed a radical alternative to the X.509 / PKIX architecture that was emerging at the time: instead of certificates binding **names** to keys (and access being decided by mapping names to ACLs), SPKI certificates would bind **authorizations** directly to keys, with no intermediate name layer.

The format never achieved wide deployment. Its **design influence**, however, is foundational — Macaroons, Biscuit, UCAN, and ZCAP-LD all inherit ideas SPKI articulated first. This file documents what SPKI got right (the 5-tuple model, transitive delegation), what it got wrong (S-expressions, monolithic spec), and what's worth carrying forward.

## Key facts

| Fact | Value |
|---|---|
| Standard | [RFC 2693](https://datatracker.ietf.org/doc/html/rfc2693), September 1999 |
| Status | Experimental — never advanced to Proposed Standard |
| Authors | Carl Ellison (Intel), Bill Frantz (Electric Communities), Butler Lampson (Microsoft), Ron Rivest (MIT LCS), Brian Thomas (Southwestern Bell), Tatu Ylönen (SSH) |
| Wire format | Canonical S-expressions (canonical-form variant of Lisp S-exprs) |
| Crypto | Any (RFC 2693 era — RSA, DSA were standard) |
| Companion specs | [RFC 2692](https://datatracker.ietf.org/doc/html/rfc2692) (SPKI Requirements), [RFC 2693](https://datatracker.ietf.org/doc/html/rfc2693) (Theory) |
| Lineage | Merger of two efforts: **SPKI** (Ellison) and **SDSI** (Rivest + Lampson, MIT) → "SPKI/SDSI" |
| Deployment | E (the ocap language), various academic systems; no major commercial deployment |

## The 5-tuple model

The central abstraction of SPKI is the **authorization 5-tuple**:

```
(Issuer, Subject, Delegation, Authorization, Validity)
```

Where:
- **Issuer:** the public key (or hash of) granting the authorization.
- **Subject:** the public key (or hash of) receiving the authorization.
- **Delegation:** boolean — may the subject re-delegate?
- **Authorization:** structured S-expression naming the specific right being granted (e.g. `(ssh-login user-alice host-frobnitz)`).
- **Validity:** time bounds + optional online-validation directives.

A chain of these tuples — issued by a sequence of subjects each delegating onward — forms a **certificate chain that constitutes evidence of authorization**. To check "does Eve have right R on resource X?", you reduce a chain: each step must be issued by a party with right R and delegation-permission, ending at Eve.

This is essentially the model that Macaroons (attenuation), Biscuit (block chain with checks), and UCAN (delegation chain) all later implemented in different wire formats with different policy languages. SPKI was first.

## What SPKI got right

1. **Authorization-not-authentication.** SPKI explicitly distinguishes certificates that say "this key has authority to do X" from certificates that say "this key belongs to Alice." It is a *cap token* model, where Macaroons, Biscuit, and UCAN all live.

2. **Self-certifying keys.** SPKI identifies principals by their public key (or a hash thereof), not by names. A name without a binding to a key is meaningless. This is the same insight UCAN's `did:key` formalizes 22 years later.

3. **Reduction-based verification.** To check authorization, the verifier *reduces* a chain of certificates. No central database lookup needed. The mathematics of this are clean.

4. **Delegation flag as first-class.** Each certificate explicitly says whether the subject can re-delegate. Biscuit's seal-vs-attenuable distinction is the same idea.

5. **Local-name-spaces.** SDSI's contribution — every principal can name *other* principals in their own local namespace (`alice's friends`, `bob's accountants`), and these names compose via reduction. This is how the OCapN/Spritely community currently thinks about [petnames](https://files.spritely.institute/papers/petnames.html).

## What SPKI got wrong (or unlucky)

1. **Canonical S-expressions.** SPKI used a strict canonical-S-expression encoding for everything. In 1999 this was elegant; today it's a parsing curiosity. Every later cap format picked a more familiar encoding (JSON for JWT/PASETO, Protobuf for Biscuit, JWT/DAG-CBOR for UCAN). The wire format was a real adoption barrier.

2. **Monolithic spec.** RFC 2693 is 43 pages and tries to specify authorization semantics, name reduction, validity checks, certificate chains, and the encoding all at once. UCAN v1.0's split into five separate specs is the rebuke.

3. **No usable reference implementation at the right time.** The SPKI working group produced documents but not a deployable library for the languages of 1999 (C, Java). By the time E (the ocap language) had a working SPKI-ish implementation, X.509/PKIX had won the certificate market.

4. **No browser story.** SPKI predates the modern web auth flow. There was no `did:web`-like resolution, no OAuth-shaped UX. It couldn't slot into anything HTTPS-shaped.

5. **The IETF effectively killed it.** RFC 2693 is "Experimental" — never moved to Proposed Standard. The SPKI working group was concluded ~2001. PKIX (X.509) was the IETF's blessed identity layer.

## Modern descendants

The capability-token formats in this folder all inherit from SPKI in different ways:

- **Macaroons (2014)** drop the asymmetric crypto entirely (HMAC chain) and add the third-party caveat — but the attenuation chain is structurally an SPKI delegation chain in a different wire format.
- **Biscuit (2019)** keeps the asymmetric chain, adds Datalog for caveats, and uses Protobuf. Closest in structure to SPKI of the modern set.
- **UCAN (2021)** adds DID-rooted identity, JSON Web Token (v0.10) or DAG-CBOR (v1.0) encoding, and the explicit delegation/invocation split. The DID model is the modern `(Issuer-as-pubkey, Subject-as-pubkey)` SPKI realization.
- **ZCAP-LD** uses JSON-LD and Linked Data Proofs to ride atop the W3C VC/DID stack.

## What's worth carrying forward

For Myrhiza, three SPKI ideas are evergreen:

1. **The 5-tuple is the right ontology.** Issuer + Subject + Delegation + Authorization + Validity captures the essential structure. Every format we consider should be expressible in those terms; if it isn't, ask why.

2. **Reduction-based verification.** A verifier walks a chain, reducing it to "does this principal hold this authorization?" — no central lookup. This is exactly what Myrhiza wants for peer-to-peer cap delegation.

3. **Local namespaces (SDSI).** Each peer names its counterparties in its own namespace. There is no global registry of "who is Alice." The OCapN community has been re-discovering this as petnames; SPKI/SDSI articulated it in 1999.

## What to ignore

- **The S-expression encoding.** Use whatever wire format the surrounding system already speaks.
- **The monolithic spec approach.** Split delegation from invocation, validity from authorization. (UCAN v1.0 does this; the SPKI RFC does not.)
- **The IETF process baggage.** SPKI's standardization history is its own cautionary tale. We don't need RFC blessing for a runtime-internal format.

## Implications for Myrhiza

SPKI is the design-space ancestor. We read it to ground ourselves — not to deploy it. Three concrete uses:

1. **Use the 5-tuple as a vocabulary check.** When proposing a Myrhiza capability format, verify each of (issuer, subject, delegation, authorization, validity) has a defined slot. If "delegation" isn't first-class, that's a smell.
2. **Borrow SDSI local-namespaces** for peer-to-peer cap delegation across petnames. This is the OCapN/Spritely concept too — SPKI articulated it first and it remains under-appreciated.
3. **Treat SPKI's failure mode as a warning about wire format.** A capability format with a beautiful semantic model but a hostile encoding doesn't get deployed. Pick a wire format real systems can handle.

See [`lessons.md`](lessons.md) for action items.

## Sources

- Ellison, C.; Frantz, B.; Lampson, B.; Rivest, R.; Thomas, B.; Ylönen, T. "SPKI Certificate Theory." [RFC 2693](https://datatracker.ietf.org/doc/html/rfc2693), September 1999.
- Ellison, C. "SPKI Requirements." [RFC 2692](https://datatracker.ietf.org/doc/html/rfc2692), September 1999.
- Rivest, R.; Lampson, B. ["SDSI — A Simple Distributed Security Infrastructure."](https://groups.csail.mit.edu/cis/sdsi.html) MIT, 1996.
- Stiegler, M. ["An Introduction to Petname Systems."](https://files.spritely.institute/papers/petnames.html) (modern application of SDSI local-namespaces).
- Spritely Institute. [`prior-art/spritely-ocapn/capabilities.md`](../spritely-ocapn/capabilities.md) — petname re-articulation.
