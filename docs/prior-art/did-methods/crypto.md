**Date:** 2026-05-22
**Status:** active
**Subject:** Cryptographic key types DID methods support, and what the Cremers ETK 2025 ECDSA finding means for Myrhiza's choice.

# Key types across DID methods

The DID spec is key-type-agnostic; each method picks. The choices reveal what each ecosystem is optimizing for.

| Method | Native key types | Notes |
|---|---|---|
| `did:key` | Ed25519, secp256k1, P-256, P-384, P-521, RSA, BLS12-381 | Multicodec prefix encodes type |
| `did:web` | Whatever the DID document declares | Any JWK / verification method type |
| `did:webvh` | Ed25519, secp256k1, P-256 (and any DID-Core-supported) | Same flexibility as `did:web` |
| `did:peer` | Ed25519 + X25519 (canonical for DIDComm v2); secp256k1 supported | Pairwise model encourages key-agreement-friendly types |
| `did:plc` | **secp256k1 (`k256`) or P-256 (`p256`) ONLY** | **No Ed25519.** See below. |
| `did:ion` | secp256k1 | Sidetree protocol fixed to secp256k1 |
| `did:ethr` | secp256k1 (native EOA); other types via key list | Ethereum-native |
| `did:cheqd` | Ed25519 (Cosmos-native), secp256k1 | Best key-type story of the blockchain methods |

Three observations matter for Myrhiza:

## 1. `did:plc` deliberately excludes Ed25519

[`prior-art/at-protocol/identity.md`](../at-protocol/identity.md) §"Curves" covers this in detail. The choice was: **deterministic ECDSA with low-S normalization, on either secp256k1 or P-256, with `EcdsaSecp256k1Signature2019` / `EcdsaSecp256r1Signature2019` JWS-style signatures.** Bluesky's rationale (per the PLC spec discussion):

- W3C VC working group bias toward the EcdsaSecpXXXXSignature201X JWS family.
- ECDSA support in WebCrypto API (browser native) for both `k256` and `p256`; Ed25519 in WebCrypto is more recent and patchy.
- secp256k1 in particular has wide adoption from the Bitcoin/Ethereum world — `did:plc` rotation keys would interop with any ECDSA-secp256k1-aware tool.

The trade-off Bluesky accepted: **forgo Ed25519's faster verification + simpler malleability story** in exchange for browser-native verification + JWS interop.

For Myrhiza, this is the *wrong* trade-off — Myrhiza has no browser-native-WebCrypto constraint at the kernel layer (verification happens in WASM components), and Ed25519's properties (no nonce-reuse risk, fast verification, well-audited Rust crate `ed25519-dalek`) align with its design priors.

## 2. Cremers ETK 2025 — ECDSA risks in group key agreement

The MLS prior-art folder ([`prior-art/mls/`](../mls/)) documents the Cremers et al. ETK 2025 finding ("FCGKA/EUF-CMA Failure with ECDSA Signatures in MLS"): the paper demonstrates an attack on MLS variants when ECDSA signatures are used for the GroupContext signing, because ECDSA's signature malleability allows an active adversary to forge a distinguishable variant of a legitimate signature without forging the underlying message authority.

**The relevance to DID methods:**

The ECDSA malleability property is intrinsic to ECDSA, not specific to MLS. The same property:

- Does **NOT** break `did:plc`'s use of ECDSA for rotation keys *as long as* the canonical-encoding rule is enforced (`did:plc` requires low-S normalization, which prevents the basic ECDSA mutation attack).
- **Could** become relevant if rotation-key signatures are ever embedded in a group-context-style protocol where signature distinguishability creates an exploit. This isn't `did:plc`'s current model, but it's a future-proofing concern.

For Myrhiza, the lesson is: **if Plan B-2's rotation keys ever get used in a group-key-agreement protocol layer (e.g. MLS-over-Myrhiza per `prior-art/willow/open-problems.md` and the deferred `seal-gift-wrap-dms.md`), Ed25519 is the safer choice than ECDSA, irrespective of DID interop concerns.** This is one of the load-bearing reasons to stick with Ed25519 rather than mirror `did:plc`'s secp256k1 / P-256 choice.

## 3. Ed25519 is well-supported in DID-Land — just not by `did:plc`

The DID-Core JSON-LD vocabulary defines `Ed25519VerificationKey2020` and `Ed25519Signature2020`. JOSE (`EdDSA` algorithm with Ed25519 curve) covers it. JWK encoding (`OKP` family) covers it. The `multibase + multicodec` encoding has Ed25519 multicodecs (`0xed` for public, `0x1300` for private).

Methods supporting Ed25519:

- `did:key` (multicodec `0xed`).
- `did:web` (any type the document declares).
- `did:webvh` (same as `did:web`).
- `did:peer` (canonical for DIDComm v2).
- `did:cheqd` (Cosmos-native).

Methods NOT supporting Ed25519:

- `did:plc` (secp256k1 / P-256 only — see above).
- `did:ion` (secp256k1, Sidetree-fixed).
- `did:ethr` (secp256k1, ERC-1056-fixed; can list non-secp256k1 keys in the DID doc but the *controller* must be secp256k1).

**Conclusion:** if Myrhiza ever wants to publish a DID-format representation of its Ed25519 author keys for external resolvers, **`did:key`, `did:web`, `did:webvh`, and `did:peer` all work**. The two methods that don't accept Ed25519 are `did:plc` (which Myrhiza wouldn't use anyway — Bluesky-operated) and `did:ion` / `did:ethr` (which require blockchain anchoring — wrong shape for Myrhiza).

## What about post-quantum?

Post-quantum signatures (Falcon, Dilithium, SPHINCS+) are not standardized in any current DID method. The W3C VC Data Integrity drafts and JOSE/COSE registries are adding ML-DSA (FIPS 204) and SLH-DSA (FIPS 205) but DID method specs have not updated.

**Pre-rotation commitments** (used by `did:ion` and `did:webvh`) are the current quantum-hedging strategy: commit to the *hash* of the next key, not the key itself. When that future signature time arrives, reveal the actual key (which can be a PQ signature algorithm). The commitment only needs collision resistance, which SHA-256 still provides under standard quantum assumptions.

For Myrhiza, this matters when Plan B-2 evolves: **the rotation-key set should commit to hashes of future keys, not the future keys themselves**, to preserve the option of swapping in ML-DSA without changing the protocol. See [`rotation.md`](rotation.md).

## Summary for Myrhiza

| Question | Answer |
|---|---|
| What key type for Plan B-2 author identity? | **Ed25519** — already in use, well-supported in DID land, Cremers ETK 2025 favors over ECDSA |
| What key type for Plan B-2 rotation keys? | **Ed25519** — consistency, no malleability risk if ever used in MLS-style protocols |
| What about `did:plc` interop? | Not interoperable at the key level (Ed25519 vs secp256k1/P-256). External users wanting AT-Protocol interop would need separate key material. Acceptable cost. |
| What about post-quantum readiness? | Pre-rotation commitments now; ML-DSA swap when standardized. Don't bet on PQ-DID specs anytime soon. |
| What about secp256k1 for wallet interop (`did:ethr` / Sign-In With Ethereum) | Out of scope for Plan B-2 primary identity. If Myrhiza ever wants Ethereum-wallet interop, add a separate secp256k1 *credential* (not a primary identity key). |

## Sources

- W3C DID Core verification method registry — <https://www.w3.org/TR/did-extensions/>.
- `did:plc` spec, crypto section — <https://github.com/did-method-plc/did-method-plc>.
- `did:key` multicodec table — <https://w3c-ccg.github.io/did-key-spec/>.
- Cremers et al. ETK 2025 — covered in-tree at [`prior-art/mls/`](../mls/).
- AT Protocol identity crypto — [`prior-art/at-protocol/crypto.md`](../at-protocol/crypto.md).
- JOSE Ed25519 (`EdDSA`) — RFC 8037.
- libp2p PeerID key types — <https://github.com/libp2p/specs/blob/master/peer-ids/peer-ids.md>.
