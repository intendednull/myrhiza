**Date:** 2026-05-22
**Status:** active
**Subject:** PASETO — Platform-Agnostic SEcurity TOkens, JWT replacement with versioned crypto suites (Paragon Initiative Enterprises, 2018–)

# PASETO

PASETO (**P**latform-**A**gnostic **SE**curity **TO**kens) is a JWT-shaped token format that fixes the well-known JWT footguns by **eliminating algorithm agility** — each PASETO version specifies one symmetric AEAD construction (for `local`) and one signature suite (for `public`), with no `alg` field for an attacker to trick. The token carries a JSON claims payload, like JWT, but the cryptographic choices are made by the version, not by the token.

**Important framing:** PASETO is included in this folder for *contrast*. It is **not** a capability token — it has no attenuation, no delegation chain, no caveats. It's a stateless authentication token (JWT shape) with better crypto hygiene. We document it because it's frequently confused with cap tokens and because it answers the question "if you want a token that looks like JWT but doesn't suck, what does that look like?"

## Key facts

| Fact | Value |
|---|---|
| Creator | Scott Arciszewski |
| Organization | Paragon Initiative Enterprises (Paragon IE) |
| Reference impl | [`paragonie/paseto`](https://github.com/paragonie/paseto) — PHP, v3.5.0 (Jul 19 2025), ISC license |
| Spec repo | [`paseto-standard/paseto-spec`](https://github.com/paseto-standard/paseto-spec) — v1.0.2 (Jun 10 2022) |
| Versions | v1, v2 (legacy); **v3, v4** (current) |
| IETF status | No RFC; an expired draft (`draft-paragon-paseto-rfc-00`) existed; community-driven |
| Wire format | `<version>.<purpose>.<base64url-payload>[.<base64url-footer>]` |
| PASERK | Companion spec for **PASETO Serialized Keys** — key wrapping, IDs, password-based wrapping |

## Versions and their crypto

PASETO eliminates `alg`-confusion attacks by binding the crypto to the version string. The four current versions:

| Version | Purpose | Algorithm |
|---|---|---|
| **v1** | local | AES-256-CTR + HMAC-SHA-384 |
| **v1** | public | RSA-PSS 2048 + SHA-384 |
| **v2** | local | XChaCha20-Poly1305 |
| **v2** | public | Ed25519 |
| **v3** | local | AES-256-CTR + HMAC-SHA-384 |
| **v3** | public | ECDSA P-384 + SHA-384 |
| **v4** | local | XChaCha20-Blake2b |
| **v4** | public | Ed25519 |

The split: **v1 + v3 use NIST-compliant primitives** (for environments that require FIPS-validated crypto); **v2 + v4 use modern primitives** (libsodium-flavored). v3 and v4 are the recommended current versions; v1 and v2 are kept for backward compatibility.

The wire format makes the version + purpose **mandatory to parse** — a verifier reading `v4.public.<payload>.<footer>` cannot be tricked into accepting a v2 payload with v4 keys. This is the structural defense against JWT's `alg: none` and key-confusion attacks.

## What PASETO is for

A PASETO `local` token is symmetric-AEAD encrypted: the issuer and verifier share a key, and the token is opaque to anyone else. A `public` token is signed: anyone with the public key can verify, but only the holder of the private key could have produced it.

The payload is a JSON object of claims. By convention these mirror JWT registered claims (`iss`, `sub`, `aud`, `exp`, `nbf`, `iat`, `jti`).

```
v4.public.eyJzdWIiOiJhbGljZSIsImV4cCI6IjIwMjYtMDYtMDFUMDA6MDA6MDBaIn0.<ed25519 sig>
```

The footer carries unauthenticated-but-tamper-evident metadata (key IDs, hints). The implicit assertion (passed alongside the token at verification time) lets the verifier bind context — "this token is only valid for *this* operation" — without putting it in the payload.

## PASERK

PASERK is the companion spec for **PASETO Serialized Keys** — a standard format for wrapping, identifying, and exchanging the symmetric/asymmetric keys that PASETOs are signed/encrypted with. Types include `k4.local.<base64>` (raw v4 local key), `k4.lid.<id>` (key ID), `k4.local-wrap.<wrapped>` (key-wrapping a key under another key), `k4.local-pw.<...>` (password-wrapped). This makes key rotation and bootstrapping tractable in a way JWT's JOSE family vaguely handles via JWK.

## What PASETO does NOT do

**The big one: PASETO is not a capability token.**
- No attenuation primitive (no caveats, no Datalog, no chain).
- No delegation chain (no concept of "delegate this cap to someone weaker").
- No third-party discharge.
- No DID rooting.

A PASETO carries claims. The claims describe *who you are* and *what scope you're operating in*. They do not describe *what you're allowed to invoke*, beyond what an application coordinates through the `aud` or custom claims. This is the JWT model with better crypto.

**No revocation.** Same as Macaroons, Biscuit, UCAN: token-blacklist or short expiry. PASETO has no native answer.

**No IETF standardization.** Despite multiple attempts, PASETO has no RFC. The spec lives at [`paseto-standard/paseto-spec`](https://github.com/paseto-standard/paseto-spec) and is maintained by a small community. This is a real risk for long-term adoption.

**Smaller deployment than JWT.** PASETO is the technically-better JWT replacement, but JWT is the deployed standard. Most "I want a stateless auth token" deployments still pick JWT.

## Why include this in a capability-token folder?

Three reasons:

1. **It's the right answer for stateless authentication tokens.** If Myrhiza ever needs a JWT-shape thing (e.g., for a kernel-issued session token bound to a specific app-instance), PASETO is the cryptographically-safer choice. JWT's alg-confusion family is real and widespread.
2. **It draws the distinction clearly.** PASETO and UCAN look similar on the wire (base64 segments, signature at the end) but model totally different things. A reader who can articulate "PASETO has no `prf` chain because it's not a cap" understands the cap/auth distinction.
3. **PASERK is the right idea for key serialization.** UCAN and Biscuit both lack a standard way to package "here's a private key, password-wrapped for backup." If we adopt UCAN or Biscuit, we'd want a PASERK-shaped sibling for key materials.

## Implications for Myrhiza

We probably don't ship PASETO directly. But:

- **If we need a kernel-internal session token format** (one app instance → kernel, short-lived, no delegation), PASETO v4.local is the right shape. AEAD + version-pinned algorithm is the safe default.
- **The "no algorithm agility" lesson generalizes.** Whatever cap format we adopt (UCAN, Biscuit, or homegrown), version-pin the crypto. Do not have an `alg` field. The PASETO version string is the model.
- **PASERK is the right shape for key packaging.** Worth a sibling design when we land identity infrastructure.

## Sources

- Arciszewski, S. PASETO. [`paseto.io`](https://paseto.io/).
- [`paseto-standard/paseto-spec`](https://github.com/paseto-standard/paseto-spec) — current spec v1.0.2.
- [`paseto-standard/paserk`](https://github.com/paseto-standard/paserk) — PASERK key serialization spec.
- [`paragonie/paseto`](https://github.com/paragonie/paseto) — PHP reference implementation, v3.5.0.
- Paragon Initiative Enterprises blog: [No way, JOSE!](https://paragonie.com/blog/2017/03/jwt-json-web-tokens-is-bad-standard-that-everyone-should-avoid) — the canonical PASETO motivation essay.
