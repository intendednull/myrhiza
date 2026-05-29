**Date:** 2026-05-29
**Status:** active
**Subject:** FROST — threshold Schnorr signing (RFC 9591). What it is, why it is the primitive Myrhiza's §10.9 backup-key ceremony actually wants, and the verified correction to distribution.md:461's "not yet RFC-stable" claim.

# FROST threshold signing

**FROST** (Flexible Round-Optimized Schnorr Threshold) is a protocol that lets **M of N** signers cooperatively produce a **single, ordinary Schnorr signature** that verifies against one group public key. Crucially: the verifier sees a *normal* signature and *one* public key — it does not need to know the signing was distributed, run threshold logic, or hold N keys. The threshold is invisible on the verification side. This is precisely the primitive Myrhiza's `distribution.md` §10.9 "three offline backup keys for community-attested rotation" is groping toward but explicitly declines to use.

## What FROST is, precisely

- A **threshold signature scheme**: a secret signing key is split (via distributed key generation, DKG, or a trusted dealer) into N shares; any M shares can jointly sign; fewer than M learn nothing and cannot sign.
- **Schnorr-based**, so the output is a standard Schnorr signature — for the Ed25519 ciphersuite, an ordinary Ed25519 signature verifiable by any stock Ed25519 verifier.
- **Two-round** signing (the "round-optimized" part): one preprocessing/commitment round, one signing round. It defends against a forgery class (the "ROS"/concurrent-session attacks) that broke earlier multi-round threshold Schnorr constructions.

Contrast with the **N-independent-signatures threshold** that plain TUF root uses ([`tuf-key-compromise-recovery.md`](./tuf-key-compromise-recovery.md)): TUF root collects M *separate* signatures on one `root.json` and the verifier checks M of them. FROST collapses those into *one* signature against *one* key. Both achieve "no single key suffices"; they differ in verifier complexity and in what the verifier must know about the key structure.

## The spec correction (load-bearing)

> `distribution.md:461` states: *"proper threshold-Ed25519 schemes (e.g. FROST-Ed25519 IETF draft) are not yet RFC-stable … Future kernel majors may adopt FROST-Ed25519 once it reaches RFC."*

**This is factually out of date as of 2026.** Verified:

- FROST shipped as **RFC 9591** — *"The Flexible Round-Optimized Schnorr Threshold (FROST) Protocol for Two-Round Schnorr Signatures"* — **published June 2024**, authors D. Connolly, C. Komlo, I. Goldberg, C. A. Wood.
- It includes an **Ed25519 ciphersuite** (FROST(Ed25519, SHA-512)).
- The Zcash Foundation's `frost-ed25519` crate is at **3.0.0 (2026-04-23)**, MIT OR Apache-2.0, RFC 9591-conformant, partially audited by NCC. `frost-core` + per-ciphersuite crates (`frost-ed25519`, `frost-secp256k1`, `frost-secp256k1-tr` for Bitcoin Taproot) are described by the maintainers as **stable and feature-complete**.
- Original academic publication: Komlo & Goldberg, **SAC 2020** (LNCS 12804, pp. 34–65); IACR ePrint **2020/852**.

So "FROST-Ed25519 is not yet RFC-stable" is **false**: it has both an RFC and a stable, audited Rust crate. **Flagged for a spec edit** (this folder does not edit the spec; see [`lessons.md`](./lessons.md) §correction).

## The honest caveat the correction must carry

There is a *real* nuance the spec's conclusion can still rest on, just for the right reason:

- **RFC 9591 is Informational, not Standards-Track.** It is a product of the **IRTF CFRG** (Crypto Forum Research Group), published for informational purposes — not an IETF Standards-Track consensus document. "RFC" does not imply "IETF Standard." A conservative kernel-TCB policy *may* legitimately wait for Standards-Track status or broader formal-verification coverage before adding threshold-signing verification logic to the kernel. That is a defensible reason to defer. "Not yet RFC-stable" is **not** that reason and should be replaced with the accurate one.

So the corrected §10.9 rationale should read, roughly: *"FROST-Ed25519 is specified in RFC 9591 (Informational, IRTF/CFRG, 2024) with a stable audited Rust implementation; we defer adopting it not because it lacks an RFC but because (a) it is Informational rather than Standards-Track and (b) adding threshold-signing verification to the kernel TCB at v1 is premature."* — if that is in fact the decision.

## Why FROST fits the §10.9 problem exactly

`distribution.md` §10.9 wants: the official signing root rotatable by a quorum of maintainers, without a single maintainer's key being able to act alone, and **without the kernel verifier needing to understand a multi-key ceremony**. That is the FROST value proposition verbatim:

- **N maintainers each hold a share** of the official signing key.
- **Any M of them** can co-sign a new allowlist / rotation.
- The **kernel verifies one ordinary Ed25519 signature** against **one** `wpub-myrhiza` group public key — no change to the verifier, no threshold logic in the TCB beyond standard Ed25519 verification.

This is strictly stronger than the v1 "three backup keys used as a manual ceremony, not a cryptographic threshold" posture, and it removes the §10.9 admission that the backups are "not used as a cryptographic threshold signature." The cost is the **distributed-signing ceremony** on the maintainer side (DKG, two-round signing, share custody) — operational complexity that lives *off* the verification path. That is the genuine tradeoff to weigh, and it is an operational one, not an "is there an RFC" one.

## Determinism note (Myrhiza-specific)

FROST *signing* is interactive and randomized (nonce commitments) — it must **not** run inside a `state-apply` component. But FROST *verification* is just Ed25519 verification, which is deterministic and already in Myrhiza's crypto surface ([`crypto.md`](../../specs/2026-05-09-myrhiza-master-design/crypto.md), neighbor folder [`mls/crypto.md`](../mls/crypto.md) on Ed25519/SUF-CMA). So adopting FROST changes the *signing ceremony*, not the kernel's deterministic verification path. This keeps it compatible with the determinism discipline in `CLAUDE.md`.

## Sources

- RFC 9591 (FROST): <https://www.rfc-editor.org/rfc/rfc9591.html>, <https://datatracker.ietf.org/doc/html/rfc9591>
- FROST origin (SAC 2020 / ePrint): <https://eprint.iacr.org/2020/852>, <https://link.springer.com/chapter/10.1007/978-3-030-81652-0_2>
- frost-ed25519 crate: <https://crates.io/crates/frost-ed25519>, <https://docs.rs/frost-ed25519>
- Zcash Foundation FROST: <https://github.com/ZcashFoundation/frost>
- NIST MPTS 2020 FROST talk (Komlo): <https://csrc.nist.gov/CSRC/media/Events/mpts2020/slides/mpts2020-1b3-talk-chelsea.pdf>
