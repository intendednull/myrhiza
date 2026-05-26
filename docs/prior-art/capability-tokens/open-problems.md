**Date:** 2026-05-22
**Status:** active
**Subject:** Capability tokens — what no format solves

# Open problems

The cap-token formats in this folder differ in attenuation mechanism, identity model, and policy language, but they share a common set of unsolved problems. Anyone adopting any of them inherits all of these — including Myrhiza if we lift the format directly.

## 1. Revocation

The single hardest problem in cap-token design. No format solves it natively.

**The shape of the problem:** A capability token is a *bearer credential* — possession is authorization. Once issued and propagated, the token is "in the wild" and the issuer no longer controls who holds it. If a token is leaked, compromised, or simply no longer should be honored, the issuer needs a way to invalidate it.

**The non-decentralized answer:** Keep a revocation list at a known location; verifiers check the list before honoring a token. This works for OAuth/JWT-shape session tokens with a central auth server. It does *not* work for peer-to-peer scenarios where no party is canonically the "verifier" or "issuer's revocation server."

**The compromise answers:**
- **Short expiry + re-issue.** Embed a `nbf`/`exp` window of minutes-to-hours; force the chain to be reconstructed often. Works for online clients; fails for offline-first apps that need long-lived delegations.
- **Revocation tokens (UCAN v1.0).** Issue a signed "I hereby revoke delegation X" token. Now you have to *distribute* the revocation token, which is the same problem one level removed.
- **Third-party caveats (Macaroons).** "Caveat: present a discharge from auth.example.com." The discharge server can decline to issue a discharge for revoked tokens. Works if you have a discharge server; doesn't work peer-to-peer.
- **Blockchain-style revocation registries.** Some W3C VC profiles use this (`StatusList2021`). Adds cost + dependency on a chain.
- **Don't revoke; let it expire.** Often the right answer for short-lived caps.

**For Myrhiza:** revocation is going to require *peer-to-peer revocation gossip*, with the same convergence/Sybil/freeloader problems as any other gossip. The cap format should *expose* a revocation primitive (revocation-token CID, e.g.), but the cap format alone cannot solve this.

## 2. Key rotation

The next-hardest problem. Capability tokens are typically signed by long-lived issuer keys. What happens when those keys rotate?

- **Macaroons.** Re-key by re-issuing root credentials. All outstanding macaroons must be re-derived.
- **Biscuit / SPKI.** The issuer's public key is baked into the chain. Rotate ⇒ existing chains stop verifying.
- **UCAN.** A `did:key` *is* the key; rotation means a new DID. `did:web`/`did:pkh` allow a stable DID to point at rotating keys. The cost is DID-resolution dependency.
- **ZCAP-LD.** Same as UCAN — DID-mediated.

DID-mediated identity (UCAN, ZCAP-LD) is the only format that even *attempts* graceful key rotation, and only when using a DID method that supports it (`did:web` with rotated keys in the doc). For Myrhiza, this is a real argument for DID-rooted identity even if we don't adopt UCAN wholesale.

## 3. Third-party discharge cost

Macaroons' third-party caveats are conceptually elegant — "you can use this cap, but only if a third party also approves." In practice:

- Each third-party caveat requires a network round-trip to fetch a discharge.
- Deeply nested third-party caveats compound: a cap with three TP-caveats needs three discharge fetches before the verifier can decide.
- Third-party services become available-or-die dependencies. If `auth.example.com` is down, no cap that references it can be used.

UCAN's `prf` chains have the same shape — every link in the chain is a cap token that has to be retrieved (or carried inline). Inline-everything bloats wire size; reference-by-CID needs IPFS / content-addressed retrieval.

**For Myrhiza:** any cap that crosses multiple peers will incur multi-step verification. The protocol design must absorb this — caching, batching, parallel fetch. The cap format alone cannot fix it.

## 4. Denial-of-service via deep delegation chains

A capability with a chain of 1,000 delegations means 1,000 signature verifications. An adversary issuing very long chains can make verification expensive.

**Mitigations:**
- Cap chain length explicitly (the verifier rejects chains > N).
- Use parallel verification.
- Cache intermediate verification results.

No cap format I've surveyed specifies a maximum chain depth. This is an integration-layer concern that bites in production.

## 5. Caveat semantics interoperability

Each cap format leaves caveat semantics to the application:

- Macaroons: predicate strings are opaque to the format.
- Biscuit: Datalog facts/rules are application-defined.
- UCAN: `cmd` strings (`"/fs/read"`) are namespaced by convention but not by spec.

This means two applications using the same cap format can disagree on what a cap *means*. A Biscuit issued by Service A saying `right("read", "/files/*")` may semantically mean something different than the same string in Service B.

**For Myrhiza:** if we ship a cap format, we need a registry (or convention) for ability names. WIT-typed handles solve this in-process: the type *is* the schema. For wire-transported caps, we'd need a parallel schema.

## 6. Delegation depth vs. ergonomics

Cap tokens are typically issued by traversing the delegation graph from the resource owner forward. In practice, *most* delegation in real systems is one hop deep ("Alice grants Bob a cap"). Deep delegation chains exist in theory but are rare in deployment.

This is a design tension. The format must *support* arbitrary depth (because the cap-discipline thesis demands it), but the dominant case is shallow. Heavyweight per-link machinery (ephemeral keypairs per attenuation, DID resolution per `iss`) doesn't pay off in the one-hop case.

UCAN's split into Delegation + Invocation in v1.0 partially addresses this: a one-hop delegation is short; the invocation cites it.

## 7. Time-bounded vs. event-bounded validity

Every cap format here supports time bounds (`nbf`/`exp` or equivalent). None natively supports *event* bounds — "valid until the resource owner publishes the next state-apply event."

For Myrhiza's deterministic state model, event-bounded validity is more natural than wall-clock time bounds. We'd need to extend whatever format we pick (or build our own predicate).

## 8. Browser viability + Web Crypto algorithm subset

Web Crypto API supports a small algorithm set:

- HMAC-SHA-256: Macaroons OK.
- Ed25519 (recently widely supported): UCAN/Biscuit OK.
- ECDSA P-256/384: Biscuit secondary algo OK.
- RSA-PSS: UCAN with RSA OK.

Algorithms outside Web Crypto (XChaCha20-Blake2b, secp256k1) require shipping a JS/WASM implementation. This affects browser bundle size and verification speed.

PASETO v4 uses Blake2b — *not* in Web Crypto. PASETO v3 uses NIST primitives — all in Web Crypto. For browser use, v3 > v4 if Web Crypto compatibility matters.

## 9. Standardization risk

| Format | Risk |
|---|---|
| Macaroons | No IETF / W3C standard; canonical implementation = `libmacaroons` |
| Biscuit | No IETF; Eclipse Foundation institutional steward |
| UCAN | DIF working group; not IETF; v1.0 release-candidate, not final |
| SPKI | RFC Experimental, effectively abandoned |
| ZCAP-LD | W3C CCG draft, never Recommendation |
| PASETO | Community spec, expired IETF draft |
| JWT | RFC 7519, deployed default |

JWT is the only one with full IETF blessing — but it's not a cap token. **Every actual cap-token format here is a small-community spec.** Adopting one means living with that risk: a small community can fold, fork, or shift direction. Eclipse Biscuit's recent move into the Foundation is a hedge against this; UCAN's DIF stewardship is similar.

For Myrhiza: pinning to a specific cap format is committing to a small-community spec. The alternative is to ship our own format (which is also a small-community spec by definition). Neither is better; pick deliberately.

## 10. The "cap token in an oCap discipline" impedance mismatch

Inside the runtime, Myrhiza enforces capability discipline via WIT-typed resource handles: unforgeable, scope-bound, transferable as values. These are *the* capabilities.

Capability *tokens* are what those handles look like when serialized for network transport. There's an impedance mismatch: a handle is a runtime object with type identity; a token is a bytestring with semantic claims. The mapping must:

- Preserve unforgeability (a token can be verified to grant only what the handle granted).
- Preserve attenuation (a token derived from another token grants a subset).
- Handle revocation, rotation, expiry — which the in-process handle doesn't need.

No existing cap-token format is designed with WIT-resource-handle interop in mind. This is design work Myrhiza will have to do regardless of which format we pick (or build). See [`lessons.md`](lessons.md).

## Sources

- All per-format `## Sources` sections.
- UCAN Revocation spec: [`ucan-wg/revocation`](https://github.com/ucan-wg/revocation).
- W3C VC StatusList2021: [`w3c/vc-status-list-2021`](https://github.com/w3c/vc-status-list-2021).
- Web Crypto API algorithm support: [MDN — SubtleCrypto](https://developer.mozilla.org/en-US/docs/Web/API/SubtleCrypto).
- [`prior-art/spritely-ocapn/capabilities.md`](../spritely-ocapn/capabilities.md) — the in-process ocap discipline that cap tokens are the serialized form of.
