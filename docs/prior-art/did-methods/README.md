**Date:** 2026-05-22
**Status:** active
**Subject:** W3C DID Core + DID method survey — `did:web`, `did:key`, `did:ion`, `did:peer`, `did:plc`, `did:webvh`, `did:ethr`, `did:cheqd` — plus implementation libraries (Spruce ssi-rs, Veramo, DIF universal-resolver). Reference material for Myrhiza Plan B-2 (persistent identity).

# DID Methods — prior art

W3C Decentralized Identifiers (DIDs) define an abstract identity layer: a `did:<method>:<method-specific-id>` URI resolves to a *DID document* containing public keys, service endpoints, and verification methods. Identity is one identifier; the keys it controls can rotate. The mechanism is one of the candidate primitives for Myrhiza Plan B-2 ([`docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md`](../../specs/2026-05-19-plan-b-2-persistent-identity-design.md)) and for the unified "user identity / behaviour identity" problem in [`prior-art/willow/open-problems.md`](../willow/open-problems.md) §"Multi-device identity".

The DID *method* registry currently lists hundreds of entries. The vast majority are research-grade, abandoned, or single-vendor. This corpus covers the eight methods that matter for Myrhiza's decision:

| Method | Operator / registry | Key types | Update mechanism | Recovery | Production scale |
|---|---|---|---|---|---|
| **`did:web`** | Self-hosted HTTPS | Any (DID doc declares) | Edit `.well-known/did.json` | DNS/hosting control | Wide enterprise use; ~hundreds of orgs |
| **`did:key`** | None — key *is* the DID | Ed25519, secp256k1, P-256/384, RSA | **Cannot update** | None | Bootstrap / short-lived only |
| **`did:ion`** | DIF / Sidetree-on-Bitcoin | secp256k1 | Sidetree batch on Bitcoin | Recovery key in operations | Public network nominal; **Microsoft Entra dropped it 2023-12** |
| **`did:peer`** | None — pairwise | Ed25519, X25519 | Out-of-band peer sync | None (rotate pair) | DIDComm v2 production (Aries) |
| **`did:plc`** | Bluesky PBC (centralized) | secp256k1, P-256 | Signed PLC ops to directory | 72-hour rotation-key rewrite | **~12M+ DIDs** (Oct 2024) |
| **`did:webvh`** | Self-hosted HTTPS + JSONL log | Ed25519, secp256k1, P-256 | Append signed entry to log | Authorized-key rotation in log | BC Gov pilot, Trust over IP backers |
| **`did:ethr`** | Ethereum smart contract (ERC-1056) | secp256k1 (native), others via key list | Tx to ethr-did-registry | EOA control | Wide DeFi/SSI use; resolver v13.0.0 (2026-05) |
| **`did:cheqd`** | Cosmos SDK chain (cheqd network) | Ed25519, secp256k1 | Signed tx to cheqd ledger | All-controllers-signed update | Live mainnet; commercial SSI focus |

## How to use this folder

Read [`methods.md`](methods.md) first — it surveys all eight methods side-by-side with the exact same axes. Then dive into [`rotation.md`](rotation.md) (the load-bearing file for Myrhiza) for how each handles the long-term-identity-vs-active-signing-key split. [`did-core.md`](did-core.md) explains the W3C abstraction the methods all conform to; [`implementations.md`](implementations.md) covers the libraries (Spruce ssi-rs, Veramo, DIF universal-resolver). [`lessons.md`](lessons.md) is the consult-this-when-designing-Plan-B-2 synthesis.

**Framing disclosure.** These docs are written from a Myrhiza-already-committed-to-Ed25519-bech32m-keypairs stance — most "Implications for Myrhiza" sub-sections frame each method's choices through that lens. We also already have a production peer (`prior-art/at-protocol/identity.md`'s `did:plc`) that solves the rotation-key-vs-signing-key split *without* DIDs in the wire format. So the corpus has a built-in pull toward "lift the *idea* (rotation vs signing) without lifting the *machinery* (DID Core + method drivers)." Future readers auditing whether DID Core itself is a primitive Myrhiza should adopt should weigh the corpus accordingly: it's a learn-from-DID-methods-into-Myrhiza's-own-identity-shape artifact, not a neutral catalog.

**Cross-link to existing corpus:**

- [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md) — **`did:plc` deep-dive lives there** (not duplicated here). Read it before [`rotation.md`](rotation.md).
- [`prior-art/willow/open-problems.md`](../willow/open-problems.md) §"Multi-device identity" — the Myrhiza problem this corpus informs.
- [`prior-art/signal/identity.md`](../signal/identity.md) — Signal's PNI/ACI is a *non-DID* comparator solving the same long-term-identity problem.
- [`prior-art/mls/`](../mls/) — Cremers ETK 2025 finding on ECDSA signatures matters for which key types to bless.

## Table of contents

- [`did-core.md`](did-core.md) — the W3C abstraction (Recommendation 2022-07-19; 1.1 in progress).
- [`methods.md`](methods.md) — per-method side-by-side comparison.
- [`rotation.md`](rotation.md) — **load-bearing for Myrhiza** — rotation mechanisms across methods.
- [`implementations.md`](implementations.md) — Spruce ssi-rs, Veramo, DIF universal-resolver, libp2p PeerID (related but not a DID method).
- [`crypto.md`](crypto.md) — key types each method supports; Ed25519/secp256k1/P-256/ECDSA matters for Myrhiza.
- [`adoption.md`](adoption.md) — honest scale + production-deployment audit. Outside Bluesky and a few enterprise SSI deployments, DIDs are research-grade.
- [`history.md`](history.md) — DID Core standardization politics (Google/Mozilla formal objections), DID-WG rechartering 2026-03.
- [`abandoned.md`](abandoned.md) — `did:ion`'s Microsoft retreat, `didkit` archival, the long tail of dead methods.
- [`open-problems.md`](open-problems.md) — what the DID layer structurally doesn't solve.
- [`lessons.md`](lessons.md) — **the consult-this-when-designing decision file.** validates / avoid / borrow.
- [`glossary.md`](glossary.md) — terms specific to the DID ecosystem.

## Key facts (verified 2026-05-22)

| Fact | Value |
|---|---|
| W3C DID Core 1.0 | Recommendation 2022-07-19 |
| W3C DID Core 1.1 | In progress (Working Draft, no REC date) |
| DID-WG charter (current) | Until 2026-10-28 (previous WG archived 2026-03-24, rechartered) |
| Formal objectors to 1.0 | Google, Mozilla, _Anonymized1_ (NOT Apple — common misattribution) |
| Director's resolution | Objections overruled; 1.0 published as REC |
| DID method registry | W3C DID Extensions Methods, published 2026-04-10 |
| Method count | "Hundreds" — most provisional / abandoned |
| `did:plc` scale | 12M+ DIDs (Oct 2024, Bluesky-operated `plc.directory`) |
| `did:ion` operator status | DIF + Microsoft Sidetree; **Microsoft Entra dropped it Dec 2023**; public ION network nominally runs |
| `did:webvh` rename | Formerly `did:tdw`; spec v1.0 stable, DIF + Trust over IP |
| `didkit` (Spruce JS/CLI/WASM) | **Archived 2025-07-10**; users redirected to `ssi` crate + `sprucekit-mobile` |
| `spruceid/ssi` (Rust) | v0.16.0 (2026-04-16); Apache-2.0; 160k+ downloads |
| `@veramo/core` (JS) | v7.0.0 (2026-02-11); Apache-2.0; active |
| DIF universal-resolver | Apache-2.0; ~70+ method drivers; last tagged release v0.5.0 (2022-01-07); commits ongoing |
| libp2p PeerID | **NOT a DID method**; multihash of public key; could wrap in `did:key` |

## Sources

- W3C DID Core 1.0 Recommendation — <https://www.w3.org/TR/did-1.0/> (publication 2022-07-19).
- W3C DID Extensions — Methods (registry) — <https://www.w3.org/TR/did-extensions-methods/> (published 2026-04-10).
- W3C DID 1.0 Formal Objections Report — <https://www.w3.org/2022/03/did-fo-report.html>.
- W3C DID Working Group (current) — <https://www.w3.org/groups/wg/did/> (chartered to 2026-10-28).
- W3C DID Working Group (archived) — <https://github.com/w3c/did-wg> (archived 2026-03-24).
- ION repository — <https://github.com/decentralized-identity/ion>.
- Microsoft Entra Verified ID — what's new — <https://learn.microsoft.com/en-us/entra/verified-id/whats-new>.
- did:plc spec — <https://github.com/did-method-plc/did-method-plc>.
- did:webvh (formerly did:tdw) — <https://identity.foundation/didwebvh/>.
- did:peer spec — <https://identity.foundation/peer-did-method-spec/>.
- ethr-did-resolver — <https://github.com/uport-project/ethr-did-resolver>.
- cheqd DID method ADR — <https://docs.cheqd.io/product/architecture/adr-list/adr-001-cheqd-did-method>.
- Spruce `ssi` crate — <https://crates.io/crates/ssi> (v0.16.0, 2026-04-16).
- Spruce `didkit` archival — <https://github.com/spruceid/didkit> (archived 2025-07-10).
- Veramo `@veramo/core` — <https://www.npmjs.com/package/@veramo/core> (v7.0.0, 2026-02-11).
- DIF universal-resolver — <https://github.com/decentralized-identity/universal-resolver>.
- libp2p PeerID spec — <https://github.com/libp2p/specs/blob/master/peer-ids/peer-ids.md>.
