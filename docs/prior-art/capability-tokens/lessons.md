**Date:** 2026-05-22
**Status:** active
**Subject:** Lessons for Myrhiza from the capability-token survey — validates / avoid / borrow

# Lessons for Myrhiza

This is the consult-this-when-designing file. The per-format docs in this folder are evidence; this is synthesis.

**Frame:** Myrhiza enforces capability discipline *in-process* via WIT-typed resource handles brokered by the kernel. Capability *tokens* — the focus of this folder — are what those handles look like when they cross peer boundaries. The lessons are organized around the design questions Myrhiza will face when adding the network-transport surface for capabilities.

## Validates

What the survey *confirms* about Myrhiza's current direction.

### V1. Capability discipline at the network layer is a 25-year-old idea

[SPKI (RFC 2693, 1999)](spki.md) articulated the 5-tuple model — issuer/subject/delegation/authorization/validity — before object capabilities had a name. Every modern cap-token format inherits from it. The discipline isn't a contrarian bet; it's mainstream within a niche, with continuous practitioner activity from 1999 through Macaroons (2014) to Biscuit/UCAN (2019/2021) and beyond. Building on this lineage isn't picking the unproven path.

### V2. "Tokens carry authorization, not identity" is the right split

JWT/PASETO are authentication tokens (they say *who you are*). Macaroons/Biscuit/UCAN/SPKI are capability tokens (they say *what you can do*). The distinction is foundational and every serious cap-token design respects it. Myrhiza's choice to broker authorization through capability handles, not user-identity sessions, is on the right side of this distinction.

### V3. Asymmetric crypto is mandatory for peer-to-peer caps

Macaroons' HMAC-only design works when issuer and verifier share a secret. Once verification crosses unrelated peers, asymmetric is required. Biscuit, UCAN, and SPKI all use public-key signatures. Myrhiza's assumption that every peer has a long-lived Ed25519 (or equivalent) keypair is what makes peer-symmetric cap delegation tractable.

### V4. Delegation chains with attenuation are the right primitive

Every cap-token format that targets peer-to-peer use (Biscuit, UCAN, SPKI) implements *some* version of "a chain of signed delegations, each more restrictive than the previous." This is the network-transport realization of the ocap discipline. We should expect Myrhiza's cap-on-the-wire surface to be a chain-of-signed-attenuations of *some* shape. The argument is over wire format, not over the model.

### V5. DID-rooted identity is the modern answer to "who is the issuer"

UCAN's `did:key` and ZCAP-LD both root authorization in DIDs. This is the modern formalization of SPKI's "subjects are keys, not names." For Myrhiza, where peers are already keypair-identified, DID-rooted is a near-zero-cost upgrade from raw-pubkey identity (just spell the same key as `did:key:z...`) and gives us interoperability hooks with the broader DID/VC ecosystem.

## Avoid

What the survey says *not* to do.

### A1. Don't conflate authentication tokens with capability tokens

JWT and PASETO are not capability tokens. A claim like `scope: ["read"]` in a JWT is not an ocap — it's a string the verifier interprets, and the bearer cannot derive a strictly-narrower version without contacting the issuer. If we ever ship a JWT-shaped token for any cap-like purpose, we've made a structural mistake. The closest exception: an opaque kernel-issued session bearer (`PASETO v4.local`) for one-app-instance-to-kernel use, which is *not* a cap because it's not delegable.

### A2. Don't ship algorithm agility (`alg` field) in any token

The JWT `alg: none` family of attacks is real and recurring. PASETO's "version-pins-algorithm" is the correct fix. Whatever cap format we use, version-pin the crypto. If a future version of Myrhiza tokens needs a new algorithm, that's a new version number; never a per-token field.

### A3. Don't claim Macaroons solve revocation. They don't.

A leaked Macaroon stays valid until its embedded time-bound caveats expire. Same applies to Biscuit and UCAN. **Revocation is unsolved across the board.** Don't pick a format because of a revocation story; treat revocation as a separate problem requiring its own mechanism (likely per-peer gossip with eventually-consistent revocation lists). The cap format should *expose a hook* (revocation-token CID, revocation-id field) but solving revocation belongs to the protocol layer above.

### A4. Don't embed a Datalog evaluator in `state-apply`

Biscuit's Datalog dialect is powerful, but Datalog evaluation order is implementation-defined unless we pin it. For Myrhiza's `state-apply` profile — pure functions of `(prior state, event)` — embedding a Datalog evaluator means committing to a specific evaluation order *across implementations*. This is a determinism risk for marginal expressive gain. If we want policy expressiveness in `state-apply`, use a smaller predicate language (UCAN-style `pol` clauses) or compile policies to WASM and execute them deterministically.

### A5. Don't adopt UCAN v0.10 in 2026

v0.10 is JWT-encoded, conflates delegation and invocation, and has no native revocation spec. v1.0-rc.1 is a substantial improvement. If we adopt UCAN, we adopt 1.0. Acknowledge: 1.0 is still a release candidate as of March 2026. We'd be early. Tradeoff.

### A6. Don't pick a cap format for its standardization status

None of the cap-token formats have IETF Standards Track status. Every adopter takes spec-fragility risk. Picking Biscuit (Eclipse Foundation) over UCAN (DIF) doesn't materially reduce this risk — both are small-community specs. The risk-reduction strategy is *to design for format-replaceability* (separate the on-wire format from the in-memory cap model), not to pick the "more standard" format.

### A7. Don't ignore the wire-format adoption barrier

SPKI's canonical S-expressions were elegant but hostile to deployment. PASETO's base64url-JSON encoding looks like JWT and slots into existing HTTP-shaped systems. Biscuit fits in HTTP cookies. UCAN v1.0's DAG-CBOR is excellent for content-addressed storage but unfamiliar to most engineers. Wire format matters for adoption *outside* the runtime — for our own peers it matters less, but if we ever expose caps to web tooling, the encoding choice is real.

## Borrow

What to pull into Myrhiza's design.

### B1. The Macaroon attenuation-by-derivation primitive

Macaroons' "anyone holding a cap can derive a strictly weaker one, no new key material needed" is the cleanest expression of the ocap discipline at the wire layer. The HMAC chain specifically is the wrong primitive for peer-to-peer (no shared root key), but the *property* — attenuation requires no contact with anyone — is what we want.

The asymmetric-crypto realization is Biscuit's ephemeral-keypair-per-block: each holder generates a fresh keypair, signs an attenuation block with the *previous* holder's key, embeds the new public key as the chain head. The Biscuit pattern is what we should borrow, not the Macaroon HMAC.

### B2. SPKI/SDSI local-namespaces (a.k.a. petnames)

Every peer names its counterparties in its own namespace. There's no global registry of "who is Alice." When Alice introduces Bob to Carol, Carol records "Bob (per Alice)" — not "Bob, the canonical Bob." This composes via reduction in the SDSI tradition; the Spritely community calls these petnames.

For Myrhiza, where peers gossip and introduce each other, petname-style namespacing is the natural identity surface. Cap tokens should be expressible using petnames-resolved-at-issue-time rather than global names.

### B3. DID-rooted issuer identity (UCAN model)

Root cap-token issuer identity in DIDs, with `did:key` as the primary case. This gives us:
- Self-certifying primary identity (the DID is the key).
- An escape hatch to richer DID methods (`did:web` for stable identities behind rotating keys).
- Interoperability with the broader DID/VC ecosystem if/when it matters.
- Zero-cost compatibility with UCAN-shape tokens if we adopt UCAN wholesale or partially.

Even if we don't adopt UCAN's *wire* format, using DIDs as the identity layer in our cap format is essentially free and forward-compatible.

### B4. The delegation-vs-invocation split (UCAN v1.0)

A delegation chain ("Alice → Bob → Carol can read /files/a") is a long-lived object, possibly cached. An invocation ("Carol, right now, reads /files/a") is a per-request signed object that cites the chain. Separating these means:
- Delegations can be content-addressed and cached.
- Invocations can be fast (one fresh signature, plus chain reference).
- Replay protection is per-invocation, not baked into the chain.

This split is right regardless of wire format. Adopt it.

### B5. PASERK-shape for key serialization

Whatever cap format we pick, we need a sibling format for packaging keys: raw export, key IDs, password-wrapped, key-wrapped-under-another-key. PASETO's PASERK is the right shape. UCAN and Biscuit don't have an equivalent. Define one when we land identity infrastructure.

### B6. Version-pinned algorithm choice (PASETO model)

No `alg` field. The version string of the cap format pins the algorithm. If we need to migrate algorithms, bump the version. This is the right structural defense against `alg`-confusion attacks even though Myrhiza's verifier is likely the kernel itself (and thus controls algorithm acceptance).

### B7. Third-party caveats as a *concept*, not necessarily as a mechanism

Macaroons' third-party caveats ("you can use this cap, but only if a third party also approves") is conceptually exactly what Myrhiza needs for cross-peer conditional delegation ("Bob can read /files/a, but only if his DPKI cert is still valid per peer X"). The Macaroon-specific implementation (discharge macaroon, HMAC machinery) is heavy. The *idea* — a caveat that references an external verifier and is satisfied by an additional signed object — is portable.

This may be where Myrhiza adds value over existing cap-token formats: a third-party-caveat mechanism native to the deterministic-event-replay model, where the "discharge" is the most-recent `state-apply` output of a designated peer.

## Action items

If we were writing a spec for Myrhiza's cap-on-the-wire surface today, the design decisions would be:

| Decision | Choice | Justification |
|---|---|---|
| Identity model | DIDs, `did:key` primary | B3, V5 |
| Crypto | Ed25519 over Web Crypto–supported algorithm set | A2, B6 |
| Delegation chain | Signed per-link with ephemeral keys (Biscuit pattern) | B1 |
| Delegation vs invocation | Split (UCAN v1.0 pattern) | B4 |
| Policy expressiveness | Small predicate DSL, NOT Datalog | A4 |
| Encoding | DAG-CBOR or Protobuf (open question) | A7 tradeoff |
| Revocation | Out-of-band gossip; expose revocation-token CID hook | A3 |
| Key rotation | DID-mediated via `did:web` for stable identities | open-problems #2 |
| Wire-format choice | **Open** — pin to UCAN v1.0 OR ship homegrown with UCAN-compatible chain semantics | A6 |

The biggest open question is whether to **adopt UCAN v1.0 wire format wholesale** or **ship a homegrown format with UCAN-compatible semantics**. Adopting UCAN gives us:
- Reuse of `rs-ucan` / `ts-ucan` for verification logic.
- Interop with Storacha, Fission, the DIF ecosystem.
- Cost: pinning to a release-candidate spec.

Going homegrown gives us:
- Wire format that matches WIT-resource-handle semantics natively.
- No external spec dependency.
- Cost: we're shipping a new cap-token format and accepting all spec-maintenance burden ourselves.

**Recommendation for a future spec:** adopt UCAN v1.0 *delegation chain semantics*, but specify our own ability namespace + caveat predicate language pinned to WIT-resource-handle types. Document the mapping explicitly. This is the "lift the semantic model, choose our own caveat DSL" middle path.

See [`open-problems.md`](open-problems.md) for what no cap format solves; that's the work above-and-beyond format selection.

## Sources

- All per-format files in this folder.
- [`prior-art/spritely-ocapn/capabilities.md`](../spritely-ocapn/capabilities.md) — in-process ocap discipline.
- [`prior-art/agoric-endo/capabilities.md`](../agoric-endo/capabilities.md) — capability-passing in deterministic-replay.
- [`prior-art/holochain/capabilities.md`](../holochain/capabilities.md) — grant-based bearer-secret cap tokens (Holochain-shaped, not standardized).
