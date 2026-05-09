**Date:** 2026-05-09
**Status:** active
**Subject:** MLS cryptographic substrate — ciphersuites, TreeKEM, HPKE, and post-quantum migration

> Companion files: [`protocol.md`](./protocol.md) — RFC 9420 walkthrough; [`group-lifecycle.md`](./group-lifecycle.md) — Add/Update/Remove mechanics; [`openmls.md`](./openmls.md) — Rust implementation; [`glossary.md`](./glossary.md), [`comparisons.md`](./comparisons.md), [`production-users.md`](./production-users.md), [`lessons.md`](./lessons.md).

## 1. Ciphersuites

RFC 9420 (§17.1) and the IANA MLS Ciphersuite registry define ciphersuites as integer-tagged tuples of `(KEM, KDF, AEAD, Hash, Signature)`. All seven RFC-9420 ciphersuites are marked **Recommended = Y**:

| ID | Name | KEM | AEAD | Hash | Signature |
|---|---|---|---|---|---|
| `0x0001` | `MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519` | DHKEM(X25519, HKDF-SHA256) | AES-128-GCM | SHA-256 | Ed25519 |
| `0x0002` | `MLS_128_DHKEMP256_AES128GCM_SHA256_P256` | DHKEM(P-256, HKDF-SHA256) | AES-128-GCM | SHA-256 | ECDSA P-256 |
| `0x0003` | `MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519` | DHKEM(X25519, HKDF-SHA256) | ChaCha20-Poly1305 | SHA-256 | Ed25519 |
| `0x0004` | `MLS_256_DHKEMX448_AES256GCM_SHA512_Ed448` | DHKEM(X448, HKDF-SHA512) | AES-256-GCM | SHA-512 | Ed448 |
| `0x0005` | `MLS_256_DHKEMP521_AES256GCM_SHA512_P521` | DHKEM(P-521, HKDF-SHA512) | AES-256-GCM | SHA-512 | ECDSA P-521 |
| `0x0006` | `MLS_256_DHKEMX448_CHACHA20POLY1305_SHA512_Ed448` | DHKEM(X448, HKDF-SHA512) | ChaCha20-Poly1305 | SHA-512 | Ed448 |
| `0x0007` | `MLS_256_DHKEMP384_AES256GCM_SHA384_P384` | DHKEM(P-384, HKDF-SHA384) | AES-256-GCM | SHA-384 | ECDSA P-384 |

Range `0x0A0A`–`0x9A9A` is reserved for **GREASE** (random unallocated values for forward-compat probing); `0xF000`–`0xFFFF` is private-use.

The KDF is implicit: HKDF instantiated over the suite's hash function. Naming convention: `MLS_<security level>_<KEM>_<AEAD>_<HASH>_<SIG>`.

## 2. TreeKEM — the algorithmic core

The ratchet tree is a left-balanced binary tree of public keys. Each member occupies a **leaf**; **interior** (parent) nodes hold aggregate public keys whose private counterparts are known to *every* member in the corresponding sub-tree.

- A member's *direct path* is the chain from their leaf to the root.
- A member's *copath* is the set of sibling subtrees along that path.
- Updating a leaf requires generating fresh path secrets up the direct path and HPKE-encrypting each fresh secret to the **resolution** of the corresponding copath node — i.e. to a small set of nodes whose private keys those sub-tree members know.

This gives `O(log N)` work per Commit for the committer and `O(log N)` ciphertext per recipient. The deterministic tree shape (RFC 9420 §4 — left-balanced, blank-on-remove) is what makes every member compute the same root key.

**Provenance.** TreeKEM was first proposed by Karthikeyan Bhargavan, Richard Barnes, and Eric Rescorla in 2018 ("Asynchronous Decentralized Key Management for Large Dynamic Groups", IETF input draft) and refined through the WG. A separate 2018 line of work — Cohn-Gordon, Cremers, Garratt, Millican, and Milner's "On Ends-to-Ends Encryption" (CCS 2018) — introduced **Asynchronous Ratcheting Trees (ART)**, a closely related construction; ART and TreeKEM together influenced the final design. Subsequent academic work (Tainted TreeKEM, Quarantined TreeKEM, ETK / External-Operations TreeKEM) has tightened security proofs and patched concurrency edge-cases against later RFC versions.

## 3. HPKE — Hybrid Public Key Encryption (RFC 9180)

Every MLS encryption to a tree node uses **HPKE** (RFC 9180; Barnes, Bhargavan, Lipp, Wood; Feb 2022). HPKE composes a KEM, a KDF, and an AEAD into a one-shot or streaming public-key encryption primitive with IND-CCA2 security. It is parameterized by exactly the (KEM, KDF, AEAD) tuple MLS ciphersuites name.

In MLS, HPKE is used to:
- Encrypt path secrets along the direct path during Commit (to copath resolutions).
- Encrypt the joiner's `EncryptedGroupSecrets` to their KeyPackage `init_key` in a Welcome.
- Seal the `external_pub`-encapsulated secret in an external commit's `ExternalInit` proposal.

HPKE's modes (Base / PSK / Auth / AuthPSK) — MLS uses Base mode plus a context-binding string per call site.

## 4. Signing

Two layers of signatures:

- **LeafNode signatures** — every `LeafNode` is signed by its member's `signature_key`. The signed payload includes the leaf's identity, encryption key, capabilities, and (when relevant) `group_id` for context binding. This binds a leaf to its credential.
- **Commit / FramedContent signatures** — every `Commit`, `Proposal`, and `Application` message in `FramedContent` form is signed by the sender's signature key. Receivers verify against the sender's known leaf.

Signatures bind handshake transcripts to authoring members, so a hostile DS cannot forge or reorder messages without detection. The signature algorithm is the suite's named primitive (Ed25519, Ed448, ECDSA P-256/P-384/P-521, or — in the PQ drafts — ML-DSA-65/87).

## 5. Hash function

Used pervasively:

- **Transcript hashes** (`confirmed_transcript_hash`, `interim_transcript_hash`) — running hash over all confirmed handshake messages. Anchor for the `confirmation_tag` MAC.
- **Tree hash** — recursive hash of the ratchet tree's node contents. Lets a joining member verify the GroupInfo they receive matches what existing members compute.
- **Parent hash** — chain of hashes up the direct path; binds parent nodes to their children to prevent malicious tree manipulations.
- **Epoch authenticator** — a per-epoch keyed hash exportable to the application as a "channel binding" identifier.

## 6. AEAD

The suite's AEAD encrypts:
- Application messages, keyed by the per-sender ratchet output (a fresh nonce + key per message).
- Sender data and content in `PrivateMessage` framing.
- Welcome contents (the `encrypted_group_info` field).

AES-GCM and ChaCha20-Poly1305 are the only AEADs in the RFC-9420 set; both are 128-bit-tag MAC-then-encrypt-equivalent constructions.

## 7. Post-quantum considerations

The MLS WG charter has post-quantum support as a **December 2026** milestone. The active document is `draft-ietf-mls-pq-ciphersuites` (currently `-04`, May 2026; expiration Sep 2026; in WG Last Call). It registers nine new ciphersuites — both **PQ/T hybrid** (PQ KEM + classical signature, providing post-quantum *confidentiality* but classical-only *authenticity*) and **pure PQ** (PQ KEM + ML-DSA signature, providing full post-quantum security):

| Name | KEM | Signature |
|---|---|---|
| `MLS_128_MLKEM768X25519_AES128GCM_SHA256_Ed25519` | ML-KEM-768 + X25519 (hybrid) | Ed25519 |
| `MLS_128_MLKEM768X25519_AES256GCM_SHA384_Ed25519` | ML-KEM-768 + X25519 (hybrid) | Ed25519 |
| `MLS_128_MLKEM768P256_AES128GCM_SHA256_P256` | ML-KEM-768 + P-256 (hybrid) | ECDSA P-256 |
| `MLS_128_MLKEM768P256_AES256GCM_SHA384_P256` | ML-KEM-768 + P-256 (hybrid) | ECDSA P-256 |
| `MLS_192_MLKEM1024P384_AES256GCM_SHA384_P384` | ML-KEM-1024 + P-384 (hybrid) | ECDSA P-384 |
| `MLS_128_MLKEM768_AES256GCM_SHA384_P256` | ML-KEM-768 (pure PQ KEM) | ECDSA P-256 |
| `MLS_192_MLKEM1024_AES256GCM_SHA384_P384` | ML-KEM-1024 (pure PQ KEM) | ECDSA P-384 |
| `MLS_192_MLKEM768_AES256GCM_SHA384_MLDSA65` | ML-KEM-768 (pure PQ KEM) | **ML-DSA-65** |
| `MLS_256_MLKEM1024_AES256GCM_SHA384_MLDSA87` | ML-KEM-1024 (pure PQ KEM) | **ML-DSA-87** |

A separate draft, `draft-ietf-mls-combiner` (Dec 2026 milestone), defines a **Flexible Hybrid PQ MLS Combiner** — a way to layer a PQ key agreement *on top of* a classical MLS group, rather than replacing the ciphersuite outright.

## 8. Quantum-safe migration story

RFC 9420's design explicitly anticipated ciphersuite turnover. The `capabilities` field in every `LeafNode` advertises supported ciphersuites, letting an application detect when all members can move to a new suite. The transition is then mechanical:

1. Members rotate their KeyPackages to advertise the new suite.
2. A current member proposes `ReInit` naming the new ciphersuite (see [`group-lifecycle.md`](./group-lifecycle.md) §9).
3. The Commit closes the old group; a new group is bootstrapped at epoch 0 with the same members, tied to the old via a resumption PSK.

For Myrhiza: ciphersuite agility from day one is the cheap insurance. Picking a single hardcoded suite at the kernel boundary is a foot-gun once PQ ciphersuites become recommended.

## 9. Implementations of the cryptographic substrate

OpenMLS does **not** ship its own primitives. Architecture (from `openmls/openmls`):

- `openmls_traits` — a trait crate defining the crypto surface MLS needs (HPKE, signing, AEAD, hash, PRF, randomness, key store).
- `openmls_rust_crypto` — default provider, bridging to RustCrypto crates and `hpke-rs` (an HPKE implementation).
- `libcrux_crypto` — alternative provider built on Cryspen's `libcrux`, including formally-verified primitive backends (notable for the PQ work — libcrux ships ML-KEM).
- Consumers bring their own provider by implementing the traits.

This is the cleanest abstraction boundary in the OpenMLS workspace and the right place to look when integrating MLS into a runtime that already has its own crypto provider story.

## 10. Sources

- RFC 9420 — *MLS Protocol*, §5 (Cryptographic Objects), §17.1 (Ciphersuite registry). <https://www.rfc-editor.org/rfc/rfc9420.html>
- IANA MLS Registry — Ciphersuites table. <https://www.iana.org/assignments/mls/mls.xhtml>
- RFC 9180 — *Hybrid Public Key Encryption*. <https://www.rfc-editor.org/rfc/rfc9180.html>
- `draft-ietf-mls-pq-ciphersuites-04` — *ML-KEM and Hybrid Cipher Suites for MLS*. <https://datatracker.ietf.org/doc/draft-ietf-mls-pq-ciphersuites/>
- `draft-ietf-mls-combiner` — *Flexible Hybrid PQ MLS Combiner*. <https://datatracker.ietf.org/doc/draft-ietf-mls-combiner/>
- Bhargavan, Barnes, Rescorla — *TreeKEM* IETF input draft (2018).
- Cohn-Gordon, Cremers, Garratt, Millican, Milner — *On Ends-to-Ends Encryption: Asynchronous Group Messaging with Strong Security Guarantees*, CCS 2018. <https://eprint.iacr.org/2017/666>
- Wallez, Protzenko, Beurdouche, Bhargavan — *TreeSync: Authenticated Group Management for MLS*, USENIX Security 2023 (Distinguished Paper; Internet Defense Prize). <https://eprint.iacr.org/2022/1732>
- *TreeKEM: A Modular Machine-Checked Symbolic Security Analysis*, IACR ePrint 2025/410. <https://eprint.iacr.org/2025/410>
- *ETK: External-Operations TreeKEM and the Security of MLS in RFC 9420*, IACR ePrint 2025/229. <https://eprint.iacr.org/2025/229>
- OpenMLS — `openmls_traits`, `openmls_rust_crypto`, `libcrux_crypto`. <https://github.com/openmls/openmls>
- `hpke-rs` — Rust HPKE implementation. <https://github.com/franziskuskiefer/hpke-rs>
