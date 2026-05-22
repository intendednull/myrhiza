**Date:** 2026-05-22
**Status:** active
**Subject:** W3C DID Core 1.0 — the abstract identity layer all DID methods conform to.

# DID Core — the W3C abstraction

DID Core defines a URI scheme + a JSON-LD/JSON document format. It does **not** define how a DID resolves to a document — that's the *method's* job. Every method must spec its own CRUD (create, read/resolve, update, deactivate) operations and prove cryptographically that the DID document is authorized by the DID controller. See [`methods.md`](methods.md) for how each candidate method does this.

## What's in DID Core

The 2022-07-19 Recommendation defines:

1. **The DID URI scheme** — `did:<method-name>:<method-specific-id>`. Methods are registered in the W3C DID Extensions — Methods registry (republished 2026-04-10, hundreds of entries).
2. **The DID document format** — JSON (and JSON-LD profile). The document contains:
   - `id`: the DID itself.
   - `verificationMethod`: array of public keys with `controller`, `type` (e.g. `Ed25519VerificationKey2020`, `JsonWebKey`), and key material.
   - **Verification relationships:** `authentication`, `assertionMethod`, `keyAgreement`, `capabilityInvocation`, `capabilityDelegation`. Each maps to a verification method by reference. A key authorized for `authentication` is not automatically authorized for `assertionMethod` — these are the cap-like distinctions DID-Core bakes in.
   - `service`: arbitrary service endpoints (DIDComm endpoints, hub URLs, etc.).
   - `controller`: optional pointer to *another* DID that has authority.
3. **DID resolution** (defined separately in [DID Resolution v0.3](https://w3c-ccg.github.io/did-resolution/)): inputs are a DID + resolution options, outputs are a DID document + metadata. Each method provides its own resolver.
4. **Verifiable Data Registry (VDR)** is the abstract storage layer — could be a blockchain, a directory, an HTTPS server, or nothing (key-is-the-DID).

DID Core is intentionally minimal: it specifies the *interface* and leaves *every architectural choice* (centralized vs decentralized VDR, key types, update authority model, recovery semantics) to the method. **This is the central design trade-off and the source of most criticism** — see [`history.md`](history.md) and [`open-problems.md`](open-problems.md).

## What DID Core is NOT

- **NOT a credential format.** Verifiable Credentials (VCs) are a separate W3C spec (VC Data Model 1.1 / 2.0). VCs *use* DIDs as issuer/subject identifiers, but the two are independent specs. Common conflation in marketing material.
- **NOT a wire protocol.** DIDComm v2 (DIF) is one protocol that uses DIDs; sd-jwt-vc is another. DID Core doesn't say how DIDs flow between agents.
- **NOT a key-management spec.** Each method handles its own keys; there's no DID-Core-level guarantee on rotation, recovery, or revocation. This is the load-bearing gap [`rotation.md`](rotation.md) audits.
- **NOT decentralized by mechanism.** A DID method can be entirely centralized (`did:plc`, run by Bluesky PBC) and still call itself "decentralized identifier." DID-Core's "D" is aspirational; the realized property is method-dependent.

## DID Core 1.0 → 1.1

DID Core 1.0 is the current Recommendation (2022-07-19). DID Core 1.1 is in active development by the *current* W3C DID Working Group (chartered until 2026-10-28). The previous WG was archived 2026-03-24 and rechartered with a narrower scope.

**1.1 scope (in-progress, no REC date):**

- Tighter resolution conformance (a 1.0 criticism was that resolution was barely tested).
- Clarification of `equivalentId` and `canonicalId` for migrations between methods.
- Non-human identity considerations (agents, delegated authority — issue #927, opened 2026-03-30).
- Better alignment with VC 2.0 + JOSE/COSE.

**No new normative requirements on methods are expected.** 1.1 is a refinement, not a redesign. The fundamental property — every method picks its own VDR — survives.

## Conformance

A *conforming* DID method must:

1. Publish a spec (typically at `https://www.w3.org/TR/did-methods-<name>/` or in DIF's `identity.foundation`).
2. Register in the DID Extensions — Methods registry.
3. Implement create + resolve at minimum; update + deactivate are method-discretionary.
4. Pass the DID Core test suite (105 implementations submitted to the original 1.0 test suite per the 2022-07 press release).

**The bar is low.** Three of the eight methods in this survey (`did:key`, `did:peer`, `did:plc`) deliberately omit *update* — `did:key` has no update mechanism at all; `did:peer` rotates by replacing the DID; `did:plc` updates via signed PLC operations but is operated by one organization. DID Core does not require methods to be updateable.

## Implications for Myrhiza

DID Core gives Myrhiza a *naming convention* — `did:myrhiza:<wuser1...>` is a perfectly conforming method name, and the DID document format is a serializable representation of a key+capability layout. **What DID Core does not give Myrhiza:**

- A rotation mechanism (each method invents its own).
- A key-type policy (each method picks).
- Determinism guarantees (DID-Core resolution is *not* deterministic across methods; results depend on the VDR's state at resolve time).
- A wire format (DIDComm is an *option*, not a requirement).

The realistic question is **not** "should Myrhiza adopt DID Core?" but **"would adopting the DID-document JSON format for external interop be worth the conformance overhead?"** — see [`lessons.md`](lessons.md) §"Borrow".

## Sources

- W3C DID Core 1.0 Recommendation — <https://www.w3.org/TR/did-1.0/>.
- W3C DID Resolution v0.3 — <https://w3c-ccg.github.io/did-resolution/>.
- W3C DID Working Group (current) — <https://www.w3.org/groups/wg/did/>.
- W3C DID Working Group (legacy, archived 2026-03-24) — <https://github.com/w3c/did-wg>.
- W3C DID Extensions — Methods — <https://www.w3.org/TR/did-extensions-methods/> (2026-04-10).
- DID Core 1.0 announcement — <https://www.w3.org/2022/07/pressrelease-did-rec.html.en>.
- W3C DID 1.0 Formal Objections Report — <https://www.w3.org/2022/03/did-fo-report.html>.
- DID Core GitHub issues (incl. #927 non-human identity for v1.1) — <https://github.com/w3c/did/issues>.
