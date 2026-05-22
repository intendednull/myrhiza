**Date:** 2026-05-22
**Status:** active
**Subject:** Capability token formats — Macaroons, Biscuit, PASETO, UCAN, SPKI, ZCAP-LD (plus JWT for contrast)

# Capability tokens

A capability token is a transferable credential whose bearer is authorized to perform a specified action — distinct from an authentication token, which only attests *who* a party is. The capability discipline ([Spritely](../spritely-ocapn/capabilities.md), [Agoric](../agoric-endo/capabilities.md), [Holochain](../holochain/capabilities.md)) is principle; this folder documents the *wire formats* that real systems have used to implement it.

The folder surveys six capability-token formats plus one cautionary contrast:

| Format | Year | Provenance | Wire format | Caveat language | What it's for |
|---|---|---|---|---|---|
| [**Macaroons**](macaroons.md) | 2014 | Google (NDSS 2014) | Binary HMAC chain | First/third-party predicate caveats | Bearer cap with attenuation; no asymmetric crypto |
| [**Biscuit**](biscuit.md) | 2019 | Clever Cloud → Eclipse Foundation | Protobuf + Ed25519 chain | Datalog (no negation) | Bearer cap with delegation; public-key verifiable |
| [**PASETO**](paseto.md) | 2018 | Paragon Initiative Enterprises (Scott Arciszewski) | base64url + JSON | None (flat claims) | JWT-shaped replacement; **not** a cap token by design |
| [**UCAN**](ucan.md) | 2021 → 2026 | Fission → DIF + UCAN-WG | JWT (v0.10) → DAG-CBOR (v1.0) | URI-scoped abilities + caveats | DID-native cap with delegation chain |
| [**SPKI**](spki.md) | 1999 | IETF RFC 2693 (Ellison et al.) | Canonical S-expressions | 5-tuple `(issuer, subject, delegate, auth, validity)` | Classical ocap-token canon; never deployed widely |
| **ZCAP-LD** | 2018+ | W3C CCG Community Group | JSON-LD + Linked Data Proofs | Action+target invocation | Capability invocation over Linked Data; draft only |
| **JWT** *(contrast)* | 2015 | IETF RFC 7519 | base64url + JSON | None — claims only | **Authentication** token, not capability — see [`comparisons.md`](comparisons.md) |

[**`lessons.md`**](lessons.md) is the consult-this-when-designing file. The per-format docs are evidence; lessons synthesizes validates / avoid / borrow for Myrhiza.

## Contents

**Per-format files** (each independently skimmable, ends with `## Sources`):

- [**`macaroons.md`**](macaroons.md) — the canonical attenuation primitive. HMAC-chained bearer tokens; third-party caveats are the genuinely novel idea.
- [**`biscuit.md`**](biscuit.md) — public-key-verifiable cap with Datalog policy language. Production at Clever Cloud, Apache Pulsar.
- [**`paseto.md`**](paseto.md) — JWT replacement with versioned crypto suites. Strictly an *authentication* token despite the "PASETO" name suggesting otherwise; included to show how the JOSE-shape can be salvaged.
- [**`ucan.md`**](ucan.md) — DID-native capability with delegation chain. Currently mid-migration from JWT (v0.10) to DAG-CBOR (v1.0). Used by Fission, Storacha, Ducktype; **not** by ATProto/Bluesky despite shared authorship.
- [**`spki.md`**](spki.md) — RFC 2693, the classical ocap-token canon. Five-tuple model + 1999 S-expression encoding. Never widely deployed; design influence is the lasting contribution.

**Synthesis files:**

- [**`comparisons.md`**](comparisons.md) — per-format trade-off matrix: revocation story, attenuation model, delegation model, verifier complexity, crypto agility, browser viability.
- [**`open-problems.md`**](open-problems.md) — what no capability-token format solves: revocation at scale, key rotation, third-party discharge cost, denial-of-service via deep delegation chains.
- [**`lessons.md`**](lessons.md) — **the decision file.** validates / avoid / borrow for Myrhiza's capability design.

## How to use this prior-art doc

Designing Myrhiza's capability surface? Start with [**`lessons.md`**](lessons.md) for the synthesis. Then read the format file matching the design space you're exploring — Macaroons if attenuation/caveats are the question, Biscuit if Datalog policy is on the table, UCAN if DID-rooted delegation, SPKI for the historical canon.

**Framing disclosure.** These docs are written from a Component-Model-host-with-ocap-discipline stance — Myrhiza brokers all I/O via WIT-typed capabilities, and most "Implications for Myrhiza" sub-sections frame each format's wire layer through that lens. The corpus treats the Macaroon/Biscuit/UCAN attenuation chains as candidate *transport* layers for capabilities that exist as WIT resource handles inside the runtime — not as the cap discipline itself. Future readers auditing whether WIT-resource-handles-as-the-primitive is itself the right bet should weigh the corpus accordingly: it's a learn-from-token-formats-into-typed-handles artifact, not a neutral catalog.

A second bias worth surfacing: the corpus treats **revocation, key rotation, and third-party discharge** as load-bearing problems because Myrhiza is decentralized. A centralized service can solve these with a token-blacklist database; Myrhiza cannot. Tokens that look fine in a server-backed deployment look much worse in our setting. Adjust accordingly when reading.

## Cross-links to existing corpus

- [`prior-art/spritely-ocapn/capabilities.md`](../spritely-ocapn/capabilities.md) — E-language ocap discipline; capability tokens are the network-transport-shape of these in-process handles.
- [`prior-art/agoric-endo/capabilities.md`](../agoric-endo/capabilities.md) — capability-passing in deterministic-replay; tokens are external-realm equivalent.
- [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md) — UCAN context. **ATProto uses OAuth 2.0/2.1 + DPoP, NOT UCAN proper**, despite the shared spec authorship via Daniel Holmgren.
- [`prior-art/holochain/capabilities.md`](../holochain/capabilities.md) — grant-based bearer-secret capability tokens with no IETF/W3C lineage.
- [`prior-art/mls/`](../mls/) — comparator for group-key vs capability separation; MLS solves "who's in the group," not "what can the bearer do."

## Reading order

1. [`lessons.md`](lessons.md) — if you have 5 minutes.
2. [`comparisons.md`](comparisons.md) — if you have 10 more.
3. [`macaroons.md`](macaroons.md) + [`ucan.md`](ucan.md) — if you have 20 more. These are the two most influential live designs.
4. [`spki.md`](spki.md) + [`open-problems.md`](open-problems.md) — historical canon + unsolved problems.
5. [`biscuit.md`](biscuit.md) + [`paseto.md`](paseto.md) — for completeness.

## Sources

- Birgisson, A. et al. "Macaroons: Cookies with Contextual Caveats for Decentralized Authorization in the Cloud." NDSS 2014. DOI: [10.14722/ndss.2014.23212](https://doi.org/10.14722/ndss.2014.23212)
- Eclipse Biscuit Project. [`eclipse-biscuit/biscuit`](https://github.com/eclipse-biscuit/biscuit). Spec v3.3, Dec 17 2024.
- Arciszewski, S. PASETO. [`paseto.io`](https://paseto.io/), [`paseto-standard/paseto-spec`](https://github.com/paseto-standard/paseto-spec).
- UCAN Working Group. [`ucan-wg/spec`](https://github.com/ucan-wg/spec). v1.0.0-rc.1.
- Ellison, C. et al. "SPKI Certificate Theory." [RFC 2693](https://datatracker.ietf.org/doc/html/rfc2693), September 1999.
- W3C CCG. "Authorization Capabilities for Linked Data v0.3." [`w3c-ccg.github.io/zcap-spec`](https://w3c-ccg.github.io/zcap-spec/).
- Jones, M. et al. "JSON Web Token (JWT)." [RFC 7519](https://datatracker.ietf.org/doc/html/rfc7519), May 2015.
- Slootweg, S. (joepie91). "Stop using JWT for sessions." June 13 2016. [cryto.net](http://cryto.net/~joepie91/blog/2016/06/13/stop-using-jwt-for-sessions/).
- ATProto. [Authentication and Authorization spec](https://atproto.com/specs/auth).
