**Date:** 2026-05-22
**Status:** active
**Subject:** UCAN — User Controlled Authorization Network, DID-rooted capability token with delegation chain (Fission → DIF + UCAN-WG, 2021–)

# UCAN (User Controlled Authorization Network)

UCAN is a capability token where the issuer and audience are identified by **DIDs (Decentralized Identifiers)**, and capabilities are delegated peer-to-peer via a signed chain. The defining commitments: **self-certifying identity** (the DID *is* the public key, in the `did:key` case), **arbitrary delegation depth** without a central authority, and **separation of delegation from invocation** in the v1.0 redesign.

## Key facts

| Fact | Value |
|---|---|
| Origin | [Fission](https://fission.codes/) (Brooklyn Zelenka + Philipp Krüger, ~2021) |
| Current stewardship | [`ucan-wg`](https://github.com/ucan-wg) — a DIF working group |
| Current spec | **v1.0.0-rc.1** (DAG-CBOR encoded), as of Mar 17, 2026 — [ucan-wg/spec](https://github.com/ucan-wg/spec) |
| Authors (v1.0) | Irakli Gozalishvili (Protocol Labs), Daniel Holmgren (Bluesky), Philipp Krüger (number zero), Brooklyn Zelenka (Witchcraft Software) — *editor* |
| v0.10 spec | JWT-encoded, last v0.x release. Authors: Zelenka & Krüger (Fission), Holmgren (Bluesky), Gozalishvili (Protocol Labs) |
| License | Community Specification License v1.0 (spec); Apache-2.0 + MIT (most impls) |
| DID scheme | `did:key` is the standard; `did:web`, `did:pkh`, others permitted |
| Implementations | TypeScript [`ts-ucan`](https://github.com/ucan-wg/ts-ucan), Rust [`rs-ucan`](https://github.com/ucan-wg/rs-ucan), Go [`go-ucan`](https://github.com/ucan-wg/go-ucan) |
| Production users | [Fission](https://fission.codes/), [Storacha/web3.storage](https://web3.storage/), [Ducktype](https://ducktype.org/) |
| **Not** used by | **ATProto / Bluesky** — see below |

## What changed between v0.10 and v1.0

This is the most important structural fact about UCAN today: the spec is **mid-migration**. Existing production usage (Storacha, Fission's WebNative) is on v0.10 (JWT). The v1.0 release candidate is a redesign:

| Aspect | v0.10 | v1.0 |
|---|---|---|
| Encoding | JWT (base64url + JSON) | DAG-CBOR (IPLD content-addressed) |
| Spec split | One spec | Five separate specs: Token, Delegation, Invocation, Promise, Revocation |
| Delegation vs invocation | Conflated in one token | **Separate primitives** — delegation builds a chain; invocation is a separate signed request that *cites* the chain |
| Caveats | `caveats: [{...}]` in resource | Within `Delegation` spec, more structured |
| Capability shape | `{ with: <resource>, can: <ability> }` | Same model, more rigorously specified |
| Promises | (not in spec) | First-class — invocations can reference outputs of other invocations |
| Revocation | Ad-hoc | First-class spec with revocation tokens |

Anyone evaluating UCAN today must pick: build against the production-deployed-but-frozen v0.10, or build against the active-development v1.0-rc which is not yet 1.0-final.

## v1.0 structure (Delegation)

A v1.0 UCAN Delegation is a DAG-CBOR object signed by the issuer:

```
Delegation {
  iss: <DID-of-issuer>,           // who is delegating
  aud: <DID-of-audience>,         // who receives the capability
  sub: <DID-of-subject>,          // whose resource is being authorized (often = iss)
  cmd: <command-string>,          // e.g. "/fs/read" — URI-shaped ability
  pol: [<policy clauses>],        // structured caveats (range, equals, prefix, etc.)
  nonce: <bytes>,                 // replay protection
  meta: { ... },                  // arbitrary issuer metadata
  nbf: <timestamp>,               // not-before
  exp: <timestamp>,               // expiry
}
signature: Ed25519/RSA/secp256k1 over CBOR-encoded delegation
```

A capability is *invoked* via a separate signed object that **proves the invoker's audience-position in a chain back to the resource owner**:

```
Invocation {
  iss: <DID-of-invoker>,
  sub: <DID-of-subject>,
  cmd: <command>,
  args: { ... },
  prf: [<CIDs of delegations forming the chain>],
  ...
}
signature: ...
```

The verifier walks the `prf` chain: each delegation must have `aud` matching the next link's `iss`, the chain must terminate at the resource owner (issuer == subject), and the `cmd` + `pol` at each step must be at-least-as-permissive as the invocation. This is the ocap discipline encoded in a token chain — **monotonic attenuation along the chain** is enforced structurally.

## Why DID-rooted matters

DIDs solve a real problem in capability tokens: **how do you name the issuer when there's no central authority?** Macaroons name the issuer implicitly (by which root key the verifier has). Biscuit names by raw public key. UCAN names by DID, which can be:

- `did:key:z6MkhaXgBZD...` — self-certifying, the DID *is* the public key.
- `did:web:example.com` — resolves to a document containing the public key.
- `did:pkh:...` — points at a blockchain-resolvable identity.

This is the right abstraction for cross-system peer authorization. A capability delegated from a `did:key` is independently verifiable by anyone, without needing a registry to map "key fingerprint X = Alice."

## Production usage — and the Bluesky caveat

**Fission, Storacha, Ducktype** all use UCAN as primary auth. Storacha (formerly web3.storage) is the largest deployment by volume — every upload to the network is gated by a UCAN delegation chain rooted at the user's `did:key`.

**Bluesky/ATProto does NOT use UCAN.** This is widely misunderstood because Daniel Holmgren (Bluesky engineer) is a UCAN spec co-author. The current state ([atproto.com/specs/auth](https://atproto.com/specs/auth), as of May 2026):

> "OAuth is the primary mechanism in atproto for clients to make authorized requests to PDS instances."

ATProto uses **OAuth 2.1 + DPoP + PKCE + PAR + JWT client assertions** — a profile of OAuth 2.0/2.1, not UCAN. Earlier in Bluesky's history there were JWT-based session tokens; the current direction is mainstream OAuth with security upgrades. The DID-key + capability model influenced ATProto's design philosophically, but the wire format is OAuth-shaped.

This matters for Myrhiza: if you want to learn from ATProto's auth, you read OAuth specs, not UCAN. If you want DID-native capability semantics, UCAN is the live option.

## What UCAN does NOT solve

- **Revocation latency.** v1.0 has a Revocation spec, but revocation requires either time-bounded delegations (then re-issue often) or out-of-band distribution of revocation tokens. Decentralized revocation is hard; UCAN doesn't have a silver bullet.
- **DID resolution cost.** `did:key` is cheap (key is in the DID). `did:web` requires HTTP. Other DID methods require blockchain lookups. The choice of DID method has real performance + availability implications, and UCAN delegates this entirely to the surrounding system.
- **Caveat semantics interoperability.** Each application defines what `cmd: "/fs/read"` means. Two apps using UCAN for filesystem caps might disagree on `/fs/*` semantics. There's no central registry.
- **Spec stability.** v0.10 → v1.0 is a non-trivial migration. Anyone building today has to pick which side they live on.
- **Browser cost.** Ed25519 signing in browser is feasible (Web Crypto), but chain-walking verification of N delegations is N signature checks. For deep chains, this adds up.

## Implications for Myrhiza

UCAN is the closest format to "what we'd want for cross-peer cap delegation":

1. **DID-rooted identity** maps cleanly onto Myrhiza's per-peer keypairs. A peer's identity IS its capability-issuance authority for resources it controls.
2. **Delegation-vs-invocation split** (v1.0) is a clean separation we'd want anyway: delegation chains can be cached, invocations are per-request.
3. **DAG-CBOR encoding** is content-addressable — fits with deterministic state semantics. Two peers with the same delegation chain will hash to the same CID.

The harder questions:

- **Component-Model handles vs. UCAN delegations.** Inside the runtime, ocap discipline is enforced by WIT resource handles (unforgeable, scope-bound). UCAN is what those handles look like when serialized across peers. The architecture is "WIT handles internally; UCAN-on-the-wire for delegations that cross peer boundaries." That's a useful framing but needs design work — what's the exact mapping from a WIT resource handle to a UCAN delegation chain?
- **v0.10 vs v1.0 commitment.** If we adopt UCAN, we adopt 1.0 (better model, future-proof). But it's a release candidate; we'd be early.
- **Policy language gap.** v1.0 has structured `pol` clauses but it's not Datalog — it's a small predicate DSL. Less powerful than Biscuit, more powerful than Macaroon first-party caveats.

See [`lessons.md`](lessons.md) for action items and [`comparisons.md`](comparisons.md) for the side-by-side.

## Sources

- UCAN Working Group. [`ucan-wg/spec`](https://github.com/ucan-wg/spec). v1.0.0-rc.1.
- UCAN Working Group. v0.10 spec: [`ucan-wg/spec@v0.10.0`](https://github.com/ucan-wg/spec/blob/v0.10.0/README.md).
- Fission. [Original UCAN announcement](https://fission.codes/blog/auth-without-backend/) (2021).
- ATProto. [Authentication and Authorization spec](https://atproto.com/specs/auth) — confirms OAuth 2.1 + DPoP, NOT UCAN.
- Storacha (web3.storage) UCAN usage: [`storacha-network`](https://github.com/storacha-network) on GitHub.
- DIF UCAN-WG: [`ucan-wg` GitHub org](https://github.com/ucan-wg).
