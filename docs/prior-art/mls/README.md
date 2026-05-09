**Date:** 2026-05-09
**Status:** active
**Subject:** Messaging Layer Security (MLS / RFC 9420) + OpenMLS Rust implementation — IETF-standardized group key agreement protocol; the cryptographic primitive Myrhiza would adopt for multi-party room-shaped capabilities

# Messaging Layer Security (MLS) prior art

Folder of reference material on MLS — the IETF Standards Track group key agreement protocol — and OpenMLS, its dominant Rust implementation. 13 files, ~1,560 lines.

This is the *fills the identity / crypto / trust gap* folder. If Myrhiza grows multi-party room-shaped capabilities (channels, group state-apply with shared encrypted state, multi-party caps with rotating membership), MLS is the only IETF-ratified protocol that provides forward secrecy + post-compromise security at scale (2 to thousands of members). The corpus exists so future Myrhiza spec authors building group-cap primitives have a curated reading.

## Key facts at a glance

| Field | Value |
|---|---|
| Spec | **RFC 9420** — *The Messaging Layer Security (MLS) Protocol* — Standards Track / Proposed Standard, July 2023 |
| Architecture spec | **RFC 9750** — *The MLS Architecture* — companion deployment-side document |
| Authors (RFC 9420) | R. Barnes (Cisco), B. Beurdouche (Inria & Mozilla), R. Robert (Phoenix R&D), J. Millican (Meta), E. Omara, K. Cohn-Gordon (Oxford) |
| IETF WG | `mls` (Security Area) — active; post-quantum work targeting Dec 2026 |
| Sister WG | `mimi` (More Instant Messaging Interoperability) — DMA-driven cross-app interop, MLS as substrate |
| Reference Rust impl | **OpenMLS** — `openmls` 0.8.1 crate (2026-02-13), MIT license, 930 stars |
| Reference C++ impl | **mlspp** (Cisco) — used in Webex production |
| Alt Rust impl | **mls-rs** (AWS) — Apache-2.0 OR MIT, 5 crypto backends including CryptoKit, Wire-contributing |
| Production users (verified) | Wire Messenger (GA Apr 2025, RFC 9420), Cisco Webex (production), Discord DAVE (A/V only since Sep 2024), Google RCS UP 3.0 (limited rollout 2026) |
| NOT used by | Apple iMessage (uses PQ3 not MLS — Apple MLS exposure is via RCS UP 3.0 only), Matrix (uses Megolm), WhatsApp (uses Signal Sender Keys), Signal (uses Signal Sender Keys) |
| TreeKEM lineage | Bhargavan, Barnes, Rescorla — original TreeKEM proposal for the MLS WG (NOT the Cohn-Gordon et al. 2018 paper, which is the related but distinct ART) |

## How to use

Read in this order:

1. **[protocol.md](protocol.md)** — RFC 9420 design walkthrough. AS/DS split, threat model, wire format, MLSMessage envelope.
2. **[crypto.md](crypto.md)** — the seven mandatory ciphersuites with full `(KEM, KDF, AEAD, Hash, Signature)` decomposition, TreeKEM, HPKE/RFC 9180, post-quantum drafts (`pq-ciphersuites-04`, ML-KEM hybrids).
3. **[group-lifecycle.md](group-lifecycle.md)** — KeyPackage, Add/Update/Remove/Commit/Welcome/Reinit mechanics. The state machine spec authors must understand.
4. **[openmls.md](openmls.md)** — the Rust implementation. Workspace layout, trait abstractions (`StorageProvider`, `OpenMlsCrypto`, `OpenMlsRand`), WASM compilability (compiles to `wasm32-unknown-unknown`; **no Component Model artifact** — load-bearing gap for Myrhiza), sync API (no async).
5. **[other-implementations.md](other-implementations.md)** — mlspp (Cisco/Webex, BSD-2), mls-rs (AWS, Apache/MIT), Wire's `core-crypto`, Phoenix R&D's homeserver, libcrux (Cryspen, formally-verified primitives), Apple PQ3 distinction, Google RCS rollout.
6. **[production-users.md](production-users.md)** — verified deployments only. Wire RFC 9420 GA April 2025; Webex shipped on draft and upgrading; Discord DAVE A/V-only since Sep 2024; RCS UP 3.0 limited rollout. NOT Apple iMessage, NOT WhatsApp.
7. **[governance.md](governance.md)** — MLS WG (chairs Sullivan/Turner; PQ Dec 2026); MIMI WG (chairs Cooper/Geoghegan; DMA-aligned); academic lineage; NLnet/NGI Assure funding for OpenMLS via Almeos UG.
8. **[comparisons.md](comparisons.md)** — MLS vs Signal Sender Keys (CGKA, O(log N) vs O(N)), MLS vs Megolm (Matrix's "giant leap" framing + Nebuchadnezzar 2022 break), MLS vs OTR.
9. **[critiques.md](critiques.md)** — third-party voices. Cremers ETK 2025 (MLS *fails* FCGKA with EUF-CMA-only signatures like ECDSA — published-RFC-level finding), Wire deployment pain quotes, Quarantined-TreeKEM offline-user weakness, OpenMLS GHSA advisories.
10. **[open-problems.md](open-problems.md)** — what MLS structurally doesn't solve. AS federation (deferred to MIMI), identity binding (key transparency unstandardized), member-list privacy, malicious-member DoS, post-quantum migration.
11. **[lessons.md](lessons.md)** — *the decision file*. Validates / avoid / borrow + recommendation for Myrhiza group-cap design.
12. **[glossary.md](glossary.md)** — MLS-specific vocabulary.

If you only have time for two files: read **lessons.md** + **protocol.md**.

## Why this folder exists

For the use case sketched above, three protocols compete for the slot:

- **MLS (RFC 9420)** — IETF-standardized, CGKA-based, O(log N) scaling, FS + PCS, growing production deployment, post-quantum migration in progress.
- **Signal Sender Keys** — proprietary, used by Signal/WhatsApp, simpler implementation, weaker post-compromise security, no IETF process.
- **Megolm (Matrix)** — open spec, per-sender ratchet, no aggregate group key, broken by Nebuchadnezzar 2022 in cross-signing scenarios.

For an open-protocol P2P runtime, MLS is the only reasonable IETF-aligned choice. The corpus surfaces what adopting it actually costs (storage abstraction, sync-only API, no Component Model artifact, ECDSA signature pitfall) and where it leaves Myrhiza on its own (federation across Authentication Services, identity binding, malicious-member DoS).

## Honest scale disclosure

- **Wire Messenger** GA April 2025 with RFC 9420 — verified primary source.
- **Cisco Webex** production on draft, migrating to RFC 9420 — verified.
- **Discord DAVE** is MLS for *audio/video traffic only* (since September 2024), not for chat — verify per Discord's "DAVE protocol whitepaper."
- **RCS UP 3.0** (Google Messages, iOS 26.5) — limited rollout in 2026.
- **WhatsApp / Messenger** — no production MLS deployment verified despite Meta authoring of RFC 9420.
- **Apple iMessage** does NOT use MLS. iMessage uses PQ3 (Apple's own quantum-resistant protocol announced 2024). Apple's only MLS exposure is via the RCS UP 3.0 path.

A reader auditing "is MLS proven at scale" should weigh that Webex shipped on draft MLS (now migrating to RFC 9420), Wire reached RFC 9420 GA only in April 2025, and Discord DAVE — though shipped on RFC 9420 — covers audio/video traffic only, not chat. Remaining production users are at low-to-medium scale relative to Signal/WhatsApp's billions.

## Framing disclosure

These docs are written from the **Myrhiza-as-capability-mediated-runtime** stance — the "Implications for Myrhiza" sub-sections frame MLS through that lens. The corpus surfaces, but does not advocate for, MLS adoption: a reader auditing whether *any* group-key protocol is the right primitive for Myrhiza's current spec needs should weigh the [open-problems.md](open-problems.md) and [critiques.md](critiques.md) carefully. The Cremers ETK 2025 finding (FCGKA fails with EUF-CMA-only signatures) is a published-RFC-level flaw worth reading before committing.

The corpus also reads through the **WASM Component Model substrate** lens (see [`../wasm-component-model/`](../wasm-component-model/) and [`../crdts/`](../crdts/)) — OpenMLS does not ship as a Component Model artifact today, which means adopting MLS requires Myrhiza to author the WIT contract for `MlsGroup` operations.

## Sources

Per-file `## Sources` sections list URLs cited in that file. Aggregate top-level sources:

- RFC 9420: <https://www.rfc-editor.org/rfc/rfc9420.html>
- RFC 9750 (MLS Architecture): <https://www.rfc-editor.org/rfc/rfc9750.html>
- IETF MLS WG: <https://datatracker.ietf.org/wg/mls/>
- IETF MIMI WG: <https://datatracker.ietf.org/wg/mimi/>
- OpenMLS: <https://github.com/openmls/openmls>, <https://openmls.tech>
- mlspp (Cisco): <https://github.com/cisco/mlspp>
- mls-rs (AWS): <https://github.com/awslabs/mls-rs>
- libcrux (Cryspen): <https://github.com/cryspen/libcrux>
- IETF MLS implementations index: <https://github.com/mlswg/mls-implementations>
