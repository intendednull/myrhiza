**Date:** 2026-05-09
**Status:** active
**Subject:** Glossary of MLS terms used across this folder

# Glossary

MLS-specific vocabulary used across [protocol.md](protocol.md), [group-lifecycle.md](group-lifecycle.md), [crypto.md](crypto.md), [openmls.md](openmls.md), and the cross-cutting files.

## Protocol

- **MLS** — Messaging Layer Security. The IETF-standardized group key agreement protocol. RFC 9420 (July 2023, Standards Track / Proposed Standard).
- **RFC 9420** — *The Messaging Layer Security (MLS) Protocol*. The core protocol document. See [protocol.md](protocol.md).
- **RFC 9750** — *The MLS Architecture*. Companion deployment-side document covering Authentication Service / Delivery Service expectations and federation framing.
- **CGKA** — Continuous Group Key Agreement. The class of protocols MLS belongs to: groups continuously rotate a shared key with FS + PCS as members join/leave/update. See [crypto.md](crypto.md), [comparisons.md](comparisons.md).
- **FS** — Forward Secrecy. Compromising current keys does not reveal past messages. See [group-lifecycle.md](group-lifecycle.md).
- **PCS** — Post-Compromise Security. After a compromised member rotates their key, future messages become safe again. See [group-lifecycle.md](group-lifecycle.md).
- **AS** — Authentication Service. Out-of-scope-of-RFC-9420 service that verifies member identities. Deployments choose: certificate-based, SAML-style, key-transparency, etc. See [protocol.md](protocol.md).
- **DS** — Delivery Service. Out-of-scope-of-RFC-9420 service that routes MLS messages between members. May be centralized (Wire's server) or federated. See [protocol.md](protocol.md).

## Group state

- **Group** — the set of members sharing keys. Identified by a `group_id` (opaque bytes). See [group-lifecycle.md](group-lifecycle.md).
- **Epoch** — monotonic counter; advances on every Commit. Each epoch has its own derived key material. See [group-lifecycle.md](group-lifecycle.md).
- **Member** — one party in the group. Identified by a leaf in the ratchet tree.
- **Leaf node** — member's slot in the ratchet tree. Holds the member's signature key, encryption key, capabilities, lifetime.
- **Parent node** — interior tree node holding aggregate keys derived from descendants.
- **Ratchet tree** — left-balanced binary tree of public keys; ratchet up the tree on key updates. Core data structure. See [crypto.md](crypto.md), [group-lifecycle.md](group-lifecycle.md).
- **TreeKEM** — the cryptographic algorithm operating on the ratchet tree (Bhargavan, Barnes, Rescorla — original proposal for MLS WG; *NOT* the Cohn-Gordon et al. 2018 ART paper, which is a related but distinct CCS 2018 construction). See [crypto.md](crypto.md), [governance.md](governance.md).

## Messages

- **KeyPackage** — pre-published bundle a member uploads to the DS so others can add them to a group. Contains init_key, leaf_node, signature, lifetime, capabilities. See [group-lifecycle.md](group-lifecycle.md).
- **Welcome** — encrypted bundle sent to a newly-added member, lets them catch up to current group state without seeing past keys. Sent alongside the Commit that added them.
- **Commit** — atomic message that applies a batch of Proposals + advances epoch + derives new keys. Signed by committer.
- **Proposal** — pending change (Add / Update / Remove / Reinit / etc.) that takes effect when batched into a Commit.
- **MLSMessage** — the wire-format envelope. Two variants: PrivateMessage (encrypted) and PublicMessage (signed but not encrypted, used for some control flow). See [protocol.md](protocol.md).
- **Application message** — actual user data, encrypted with epoch-derived keys.
- **HandshakeMessage** — control-plane message (Commit / Proposal / Welcome).

## Operations

- **Add** — Proposal to add a new member referenced by their KeyPackage.
- **Update** — Proposal to rotate the proposer's own leaf key (no membership change).
- **Remove** — Proposal to remove a member.
- **Commit** — applies pending Proposals.
- **External Commit** — joining a group without a Welcome, using a `GroupInfo` published by an existing member.
- **Reinit** — switch ciphersuite by transitioning the group to a new group (new group_id) with a new ciphersuite. See [group-lifecycle.md](group-lifecycle.md).
- **PathSecret** — secret stored in a node of the ratchet tree; cascades up on Commit to derive new key material.

## Cryptography

- **Ciphersuite** — tuple `(KEM, KDF, AEAD, Hash, Signature)` identifying the cryptographic primitives used. RFC 9420 defines seven mandatory ciphersuites identified by integer in the IANA registry. See [crypto.md](crypto.md).
- **HPKE** — Hybrid Public Key Encryption (RFC 9180). The KEM-based encryption primitive MLS uses for encrypting PathSecrets. See [crypto.md](crypto.md).
- **AEAD** — Authenticated Encryption with Associated Data (e.g. AES-128-GCM, ChaCha20-Poly1305). Encrypts application messages with epoch-derived keys.
- **KEM** — Key Encapsulation Mechanism (e.g. DHKEM-X25519-SHA256). The asymmetric primitive in HPKE.
- **KDF** — Key Derivation Function (e.g. HKDF-SHA256). Derives keys from secrets.
- **Signature** — digital signature scheme (e.g. Ed25519, ECDSA-P256). Authenticates leaf nodes, KeyPackages, Commits.
- **EUF-CMA** — Existential UnForgeability under Chosen Message Attack. The standard signature security property. **MLS with EUF-CMA-only signatures (ECDSA) fails FCGKA per Cremers ETK 2025**; SUF-CMA (e.g. Ed25519) is required for full security. See [critiques.md](critiques.md).
- **PQ** — Post-Quantum. Hybrid PQ ciphersuites in `draft-ietf-mls-pq-ciphersuites-04` use ML-KEM-768/1024 + ML-DSA-65/87. See [crypto.md](crypto.md), [open-problems.md](open-problems.md).
- **ML-KEM** — Module-Lattice-based Key Encapsulation Mechanism (NIST FIPS 203, the standardized form of CRYSTALS-Kyber).
- **ML-DSA** — Module-Lattice-based Digital Signature Algorithm (NIST FIPS 204, the standardized form of CRYSTALS-Dilithium).

## Implementation

- **OpenMLS** — the Rust implementation. `openmls 0.8.1` crate. See [openmls.md](openmls.md).
- **mlspp** — Cisco's C++ reference implementation; powers Webex production. See [other-implementations.md](other-implementations.md).
- **mls-rs** — AWS's Rust implementation (formerly aws-mls). Apache-2.0 OR MIT. Five crypto backends. Wire-contributing. See [other-implementations.md](other-implementations.md).
- **libcrux** — Cryspen's formally-verified cryptographic primitives library; Cryspen also maintains an MLS impl using libcrux. NOT itself an MLS implementation but a candidate crypto provider for OpenMLS. See [other-implementations.md](other-implementations.md).
- **`StorageProvider`** — OpenMLS trait for persisting group state. Application provides; OpenMLS does no IO itself. See [openmls.md](openmls.md).
- **`OpenMlsCrypto`** — OpenMLS trait for cryptographic operations. Default impl `openmls_rust_crypto` (rust-crypto crates); alt `openmls_libcrux_crypto`. See [openmls.md](openmls.md).

## Adjacent / non-MLS

- **PQ3** — Apple iMessage's post-quantum group protocol (announced 2024). NOT MLS. Apple's only MLS exposure is via RCS UP 3.0. See [other-implementations.md](other-implementations.md).
- **Signal Sender Keys** — proprietary group protocol used by Signal and WhatsApp. O(N) scaling vs MLS's O(log N). See [comparisons.md](comparisons.md).
- **Megolm** — Matrix's group ratchet. Per-sender ratcheting; no aggregate group key. Different design point from MLS. See [comparisons.md](comparisons.md).
- **OTR** — Off-the-Record Messaging. Two-party only; legacy. See [comparisons.md](comparisons.md).
- **MIMI** — More Instant Messaging Interoperability. IETF WG building cross-app messaging interop using MLS as substrate; DMA-driven. See [governance.md](governance.md).
- **DMA** — Digital Markets Act (EU). Regulatory driver for MIMI; gatekeepers must support cross-app messaging interop.
- **SFrame** — IETF SFrame WG protocol for end-to-end encrypted media (audio/video) frames; Discord DAVE uses MLS for the SFrame key. See [open-problems.md](open-problems.md), [production-users.md](production-users.md).

## Cross-substrate (for comparison with neighbor folders)

- **Capability** (Myrhiza / [Spritely OCapN](../spritely-ocapn/)) — the only host surface in Myrhiza. Group capabilities (multi-party caps) are the use case driving the MLS folder.
- **CRDT** ([crdts/](../crdts/)) — a CRDT can converge state across replicas; MLS encrypts that state across members. The two are orthogonal: Myrhiza could combine CRDT-state-apply with MLS-encrypted-state.
- **state-apply** (Myrhiza) — pure WASM Component Model function `(prior_state, event) → next_state`. If shared encrypted state is in scope, MLS provides the group key; state-apply happens on decrypted state inside the member's compute.

## Sources

- RFC 9420: <https://www.rfc-editor.org/rfc/rfc9420.html>
- RFC 9180 (HPKE): <https://www.rfc-editor.org/rfc/rfc9180.html>
- RFC 9750 (Architecture): <https://www.rfc-editor.org/rfc/rfc9750.html>
- IANA MLS Registry: <https://www.iana.org/assignments/mls/>
- Bhargavan, Barnes, Rescorla TreeKEM proposal: <https://datatracker.ietf.org/doc/draft-mls-protocol/>
- Cohn-Gordon, Cremers, Garratt, Millican, Milner 2018 (ART): CCS 2018
- See per-file Sources sections.
