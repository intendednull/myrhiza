**Date:** 2026-05-09
**Status:** active
**Subject:** What MLS structurally does not solve — gaps any consumer (including Myrhiza) inherits

Cross-references: [`protocol.md`](./protocol.md), [`glossary.md`](./glossary.md), [`openmls.md`](./openmls.md), [`production-users.md`](./production-users.md), [`comparisons.md`](./comparisons.md), [`critiques.md`](./critiques.md).

MLS is a *key agreement* protocol. Many things adjacent to "secure group messaging" sit outside its scope by design. These are not bugs — they are the gaps Myrhiza will have to fill if it adopts MLS for group-shaped capabilities.

## 1. Federation across Authentication Services

RFC 9420 deliberately treats the Authentication Service (AS) as an external dependency. The architecture document names it but does not specify how two MLS deployments using different AS choices interoperate. The IETF MIMI (More Instant Messaging Interoperability) WG is working on this layered above MLS, and one survey paper notes the structural gap directly: "*MLS does not tackle the storage or distribution of keys, only how they might be used to send and receive cross-platform messages*" ([arxiv 2303.14178](https://arxiv.org/html/2303.14178v3)). For a P2P runtime with no canonical AS, this is the largest open design question.

## 2. Identity binding (key transparency)

MLS verifies that a leaf signature corresponds to the leaf's `signature_key`. It does *not* verify that the keypair belongs to the claimed real-world identity — that is the AS's job. Key Transparency (KT) is the proposed solution; the MLS architecture allows it ("*the verification function would correspond to verifying a key's inclusion in the log for a claimed identity*" — [draft-ietf-mls-architecture](https://messaginglayersecurity.rocks/mls-architecture/draft-ietf-mls-architecture.html)) but no IETF KT spec is yet at RFC status. Myrhiza must decide what stands in for the AS in a P2P setting.

## 3. Member-list privacy

The Delivery Service sees the full member list as ciphertext envelopes routed per leaf. Even with anonymizing the tree contents, group membership cardinality and per-epoch update patterns are observable. MIMI documents acknowledge: "*an attacker or provider with access to a fragment of message history, and the message logs of a MIMI provider in the path of a message could potentially learn more about the participants of a particular MIMI room or the room's corresponding MLS group if it can see message IDs*" ([MIMI WG](https://datatracker.ietf.org/wg/mimi/about/)). Membership privacy from infrastructure is structural, not a tunable.

## 4. Malicious-member denial of service

A group member with valid credentials can produce expensive proposals, refuse to commit, or commit with poorly-formed content that requires every receiver to validate fully before rejection. RFC 9420 mentions but does not solve this — mitigations are deployment-specific (rate limits, member reputation, application-layer eviction).

## 5. Welcome message size for large groups

Welcomes carry the full ratchet tree and signed group context. For thousands of members the Welcome can be large; new joiners pay the bandwidth cost. Webex notes their workaround: "*new joiners downloading the whole ratchet tree in only a few seconds even in a thousand-person meeting*" ([Webex blog](https://blog.webex.com/collaboration/hybrid-work/scalable-end-to-end-security-in-webex/)) — but that assumes good network conditions and a delivery service that can serve large blobs.

## 6. Post-quantum migration

Current RFC 9420 ciphersuites (X25519, P-256, P-384, P-521, Ed25519) are not PQ-safe. The IETF working group is advancing `draft-ietf-mls-pq-ciphersuites` for ML-KEM and hybrid suites — explicitly motivated by "*harvest now, decrypt later*" ([draft-ietf-mls-pq-ciphersuites-04](https://datatracker.ietf.org/doc/draft-ietf-mls-pq-ciphersuites/)). Cross-version interop during migration is open: a group on a classical suite cannot upgrade in place — Reinit is the mechanism, and its UX semantics are deployment-specific.

## 7. Off-line members and epoch advancement

A member who is offline for many epochs must process every Commit in order to catch up. RFC 9420 specifies no bounded-skip / snapshot mechanism. Cremers et al. ("Quarantined-TreeKEM", 2024) flag the dual problem: "*inactive users — who remain offline for long periods — do not update their encryption keys and therefore represent a vulnerability for the entire group*" ([eprint 2023/1903](https://eprint.iacr.org/2023/1903.pdf)). For a P2P runtime where peers are routinely offline, this is acutely relevant.

## 8. Group merge / split

MLS has no native merge (combine two groups into one) or split (partition members into two). Reinit is the only sanctioned mechanism for non-incremental changes and it produces a fresh group — losing continuity with the original. Application-layer coordination is required for any merge/split UX.

## 9. Reinit ciphersuite migration UX

Reinit is the spec-blessed escape hatch (ciphersuite change, version change, suspected compromise of group state). It produces a new group with the same conceptual identity but the user-facing semantics — message history continuity, presence, capabilities — are entirely the application's problem.

## 10. Voice/video and large media

MLS encrypts a group key. The MLS spec does not define how to use that key for SRTP, large media, or out-of-band attachments. The IETF answer is SFrame ([RFC 9605](https://www.rfc-editor.org/rfc/rfc9605.html)), which derives per-member SRTP keys from the MLS exporter. Adopting MLS for capability streams that include media means adopting the SFrame stack too, or designing an equivalent.

## 11. WASM Component Model integration

OpenMLS, mlspp, and mls-rs all ship as native libraries (Rust / C++). None ship today as a Component Model artifact. Myrhiza will need to wrap a chosen implementation as a host-side capability — MLS itself is not a candidate for in-app sandboxed code given its required access to long-lived keys, randomness, and storage.

## 12. Sources

- [arXiv 2303.14178 — One Protocol to Rule Them All? On Securing Interoperable Messaging](https://arxiv.org/html/2303.14178v3)
- [draft-ietf-mls-architecture — MLS Architecture (RFC 9750)](https://messaginglayersecurity.rocks/mls-architecture/draft-ietf-mls-architecture.html)
- [MIMI WG charter](https://datatracker.ietf.org/wg/mimi/about/)
- [draft-ietf-mls-pq-ciphersuites](https://datatracker.ietf.org/doc/draft-ietf-mls-pq-ciphersuites/)
- [eprint 2023/1903 — Quarantined-TreeKEM](https://eprint.iacr.org/2023/1903.pdf)
- [RFC 9605 — Secure Frame (SFrame)](https://www.rfc-editor.org/rfc/rfc9605.html)
- [Webex blog — Scalable E2E Security in Webex](https://blog.webex.com/collaboration/hybrid-work/scalable-end-to-end-security-in-webex/)
- [GSMA — RCS Universal Profile 3.0 with MLS-based E2EE (2025)](https://thehackernews.com/2025/03/gsma-confirms-end-to-end-encryption-for.html)
