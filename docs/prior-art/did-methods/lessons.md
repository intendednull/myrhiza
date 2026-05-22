**Date:** 2026-05-22
**Status:** active
**Subject:** The decision file — what DID method prior art validates, what to avoid, what to borrow when designing Myrhiza's persistent-identity and multi-device specs.

# Lessons for Myrhiza — DID methods

Synthesis across [`did-core.md`](did-core.md), [`methods.md`](methods.md), [`rotation.md`](rotation.md), [`crypto.md`](crypto.md), [`implementations.md`](implementations.md), [`adoption.md`](adoption.md), [`abandoned.md`](abandoned.md), [`history.md`](history.md). Format: validates / avoid / borrow.

## Validates

1. **Long-term identity ≠ active signing key.** Every serious DID method (`did:plc`, `did:webvh`, `did:ion`, `did:peer`) separates identity from key material. The DID identifier is stable; the key(s) bound to it rotate. This is the load-bearing principle for Myrhiza's multi-device story — the AuthorKeypair shipping in Plan B-2 should be one *binding* of the identity, not the identity itself. *Source: [`rotation.md`](rotation.md), [`did-core.md`](did-core.md), cross-ref [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md).*

2. **Multiple keys per identity is the norm, not the exception.** `did:plc` supports up to 5 rotation keys in priority order. `did:webvh` carries explicit `verificationMethod` arrays. MLS member-key updates assume per-device leaves under a long-term identity. The single-keypair-per-user model is mainstream-rejected. *Source: [`rotation.md`](rotation.md), [`prior-art/mls/`](../mls/).*

3. **Resolver-backed identity is deployable.** `did:plc` resolves through `plc.directory` (a single Bluesky-operated server) and works at 12M+ identifiers. `did:web` resolves through HTTPS. `did:key` resolves locally (no network). Even centralized resolution can deliver utility if the resolver is well-operated. *Source: [`methods.md`](methods.md), [`adoption.md`](adoption.md).*

4. **Ed25519 is the safe default.** Most DID methods support multiple key types but default to Ed25519 (or secp256k1). The Cremers et al. ETK 2025 finding ([`prior-art/mls/`](../mls/)) about ECDSA's EUF-CMA failure reinforces this — *prefer Ed25519 over ECDSA throughout Myrhiza*. *Source: [`crypto.md`](crypto.md), cross-ref [`prior-art/mls/lessons.md`](../mls/lessons.md).*

5. **Spec stability is achievable.** W3C DID Core 1.0 reached Recommendation 2022-07-19 despite Google + Mozilla formal objections (overruled by Director's resolution). The spec is now stable, and DID Core 1.1 is the polish-pass. Myrhiza building on DID Core 1.0 inherits a stable substrate. *Source: [`did-core.md`](did-core.md), [`history.md`](history.md).*

## Avoid

| Pitfall | Source | Mitigation |
|---|---|---|
| **`did:plc` centralization framing as "decentralized."** The `plc.directory` registry is Bluesky-operated. `did:plc` is *portable* (the DID document is signed by rotation keys) but *resolved* via one operator. Marketing it as decentralized is misleading. | [`adoption.md`](adoption.md), cross-ref [`prior-art/at-protocol/`](../at-protocol/) | If Myrhiza wants centralized resolution, say so. Don't borrow `did:plc`'s framing without inheriting the resolver-operator question. |
| **`did:ion` as a production target.** Microsoft Entra dropped ION December 2023. The public ION network nominally runs but has no enterprise steward driving roadmap. Bitcoin-anchored DIDs are heavy machinery for low benefit at Myrhiza's scale. | [`methods.md`](methods.md), [`abandoned.md`](abandoned.md) | Skip ION. The Bitcoin-anchoring is solving a problem (anchor trust) Myrhiza doesn't have (cap-grants are the trust root). |
| **`didkit` (Spruce's old CLI) as a Myrhiza dependency.** Archived 2025-07-10. Users redirected to `ssi` crate + `sprucekit-mobile`. | [`implementations.md`](implementations.md) | Use `spruceid/ssi` (Rust crate, v0.16.0 2026-04-16, Apache-2.0, 160k+ downloads). |
| **DID method proliferation as a feature.** The W3C registry lists "hundreds" of methods; most are provisional or abandoned. Supporting many methods means supporting many resolver edge cases. | [`methods.md`](methods.md) | Pick a small set Myrhiza supports natively (`did:key` always; one of `did:web`/`did:plc`/`did:webvh` for resolvable IDs). Treat others as advisory. |
| **VC (Verifiable Credentials) entanglement.** VCs are a separate W3C spec, often built on top of DIDs. Conflating "DID" with "VC" pulls Myrhiza into an ecosystem of credential exchange that's orthogonal to peer identity. | [`did-core.md`](did-core.md) | DID for *peer identity*; VC stays out of scope for v1. |
| **ECDSA-based methods (`did:ethr`).** Cremers ETK 2025 demonstrated MLS fails FCGKA under ECDSA signatures. Ethereum-anchored DIDs typically use secp256k1 + ECDSA. | [`crypto.md`](crypto.md), [`prior-art/mls/`](../mls/) | Stick with Ed25519. `did:key` over Ed25519 is the safe Myrhiza primitive. |
| **DIF Universal Resolver as a runtime dependency.** Last tagged release 2022-01-07 (v0.5.0); commits continue but the project is operationally low-velocity. Not the kind of dependency you build a P2P runtime on. | [`implementations.md`](implementations.md) | Implement Myrhiza-side resolution for the methods Myrhiza supports. Don't run a Universal Resolver. |
| **Bluesky as a model for DID governance.** Bluesky PBC operates `plc.directory` unilaterally. The genesis-block-style trust ("Bluesky promises not to misuse the registry") is a single-operator trust model. Acceptable for one app; not a governance template. | cross-ref [`prior-art/at-protocol/governance.md`](../at-protocol/governance.md) | Treat each Myrhiza-native identity binding (peer keypair, author keypair) as self-asserted; don't introduce a kernel-mediated registry. |

## Borrow

1. **`did:plc` rotation-key construction.** A small set (1-5) of rotation keys, signed in priority order, with a 72-hour clobber window for resolver-level recovery. Even if Myrhiza doesn't use `did:plc` directly, the construction is borrowable: AuthorKeypair (signing) bound to a PeerKeypair (rotation) bound to ... rotation hierarchy. *See [`rotation.md`](rotation.md).*

2. **`did:webvh` (web + verifiable history) as the federated default.** When Myrhiza needs a DID-shaped identifier resolvable over HTTPS, `did:webvh` is the current best practice — it adds verifiable history to `did:web`'s simple HTTPS resolution. *See [`methods.md`](methods.md).*

3. **`did:key` for ephemeral identifiers.** When a Myrhiza identifier doesn't need persistence (one-shot session keys, capability tokens), `did:key` (no resolution, just the pubkey-as-ID) is the right primitive. *See [`methods.md`](methods.md), cross-ref [`prior-art/capability-tokens/`](../capability-tokens/).*

4. **`spruceid/ssi` Rust crate as the DID resolution dependency.** Apache-2.0, actively maintained, comprehensive method coverage. Myrhiza needing DID resolution should depend on this rather than implementing the DID Core parser itself. *See [`implementations.md`](implementations.md).*

5. **multibase + multicodec key encoding (from `did:key`).** The pubkey-in-an-identifier construction (`did:key:z6Mk...`) uses well-defined multibase + multicodec; Myrhiza's bech32m peer/author keys are an alternative encoding for the same primitive. *See [`crypto.md`](crypto.md).*

6. **AT Protocol's rotation-key application.** Plan B-2 is the active Myrhiza spec; AT Proto's pattern (rotation keys publish updates that the resolver respects within a clobber window) is directly applicable. *See [`rotation.md`](rotation.md), cross-ref [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md), and `docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md`.*

## The single most important lesson

**The DID community converged on "identity ≠ active key" with multiple production deployments. Adopt the principle, not the specific format.** A Myrhiza PeerKeypair / AuthorKeypair split that follows `did:plc`'s rotation-key / signing-key pattern (without literally using `did:plc`) gives Myrhiza the multi-device story it needs while staying self-contained. Treat the DID specs as design influence, not as a wire-format commitment.

## Cross-references

- [`README.md`](README.md), [`did-core.md`](did-core.md), [`methods.md`](methods.md), [`rotation.md`](rotation.md), [`crypto.md`](crypto.md), [`implementations.md`](implementations.md), [`adoption.md`](adoption.md), [`abandoned.md`](abandoned.md), [`history.md`](history.md)
- [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md), [`prior-art/mls/`](../mls/), [`prior-art/signal/identity.md`](../signal/identity.md), [`prior-art/capability-tokens/`](../capability-tokens/)
- Active spec: `docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md`

## Sources

All sources in per-file evidence files.
