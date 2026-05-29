**Date:** 2026-05-29
**Status:** active
**Subject:** in-toto + SLSA — build-provenance attestation, the "how was it built" layer that complements TUF's "is the channel trustworthy" layer. Context, with the mechanics deferred to app-distribution.

# in-toto and SLSA provenance

TUF answers *"is the update channel trustworthy?"* Signing ([`app-distribution/signing.md`](../app-distribution/signing.md)) answers *"did this come from a claimed identity?"* **in-toto + SLSA** answer a third, orthogonal question: *"how was this artifact built, and can I verify that claim?"* This file is **context** — the mechanics (DSSE envelopes, OCI Referrers attachment, `cosign attest`) already live in [`app-distribution/supply-chain.md`](../app-distribution/supply-chain.md). Here we place them in the trust-model picture and correct one stale fact.

## in-toto: attestations

**in-toto** is a framework for **attestations** — signed statements *about* an artifact. The statement format binds a `subject` (artifact name + digest) to a `predicateType` (a schema URL) and a `predicate` (the structured claim). The predicate slot can carry provenance, an SBOM, a test result, a code-review record — anything with a schema. Attestations are wrapped in a **DSSE** (Dead Simple Signing Envelope) for signing.

**Verified status:** in-toto reached **spec v1.0 in 2023**, and moved to **CNCF Graduated on 2025-02-10** (announced 2025-04-23). Note: [`app-distribution/supply-chain.md`](../app-distribution/supply-chain.md) records "in-toto CNCF Graduated 2023-09-12" — that conflates the *spec v1.0* (2023) with the *graduation* (2025). The graduation date is **2025-02-10**. See [`open-problems.md`](./open-problems.md) §corpus-drift (this folder does not edit the neighbor; the discrepancy is flagged for the index owner).

## SLSA: provenance levels

**SLSA** (Supply-chain Levels for Software Artifacts) defines *how trustworthy the build process was*, expressed as levels on a Build track:

| Level | Signal |
|---|---|
| L0 | "I have an artifact." |
| L1 | Provenance exists; build documented. |
| L2 | Hosted build service signs the provenance. |
| L3 | Hardened build platform; non-falsifiable provenance. |

**Verified status:** SLSA **v1.0 GA 2023-04-19** (OpenSSF), restructured into per-area tracks. SLSA provenance is typically the predicate inside an in-toto attestation, signed (often Cosign-keyless against the build platform's OIDC identity, e.g. GitHub Actions).

## Where this sits relative to TUF

These are **complementary, not substitutes**:

- **TUF / Uptane** protect the *delivery channel* — rollback, freeze, key-compromise survivability. They say nothing about how the artifact was built.
- **in-toto / SLSA** protect the *build provenance* — they say "GitHub Actions built this exact `.wasm` from commit X via workflow Y," but say nothing about whether the channel that delivered it to you is being frozen or rolled back.
- **Reproducible builds** ([`reproducible-builds.md`](./reproducible-builds.md)) close the gap SLSA leaves: SLSA attests *a* build happened; reproducibility lets *anyone* re-run it and confirm the bytes match.

A fully defended artifact wants all three. TUF is the one Myrhiza's spec re-derives uncited; in-toto/SLSA are the ones Myrhiza's `app-distribution/supply-chain.md` already recommends ("SLSA L3 floor is achievable for free in GitHub Actions").

## Implications for Myrhiza

- **Provenance does not need a Myrhiza server.** Unlike a TUF *repository* or a Rekor *log*, an in-toto attestation is a self-contained signed blob that can ride alongside a bundle over iroh-blobs. This makes SLSA provenance the **most P2P-compatible** member of this folder — it does not conflict with §10.8.
- **The verify-burden caveat from `app-distribution`** applies: each attestation type needs its own verifier; bundle the policy into one declarative check (Notation `trustpolicy.json` is the model).
- **Build provenance ≠ behavior guarantee.** SLSA attests the *build inputs*, not that the resulting WASM is safe or deterministic. Myrhiza's determinism guarantees come from the kernel/profile model, not from provenance. Don't let an SLSA L3 badge be read as "this state-apply component is deterministic."
- **SBOM-for-WASM is still an ecosystem gap** (Syft does not parse Component Model custom sections as of mid-2025) — already noted in `app-distribution/supply-chain.md`; not Myrhiza's to fix.

## Sources

- in-toto: <https://in-toto.io>
- in-toto CNCF graduation (2025-04-23 announcement): <https://www.cncf.io/announcements/2025/04/23/cncf-announces-graduation-of-in-toto-security-framework-enhancing-software-supply-chain-integrity-across-industries/>
- SLSA: <https://slsa.dev>
- SLSA v1.0 (OpenSSF, 2023-04-19): <https://openssf.org/press-release/2023/04/19/openssf-announces-slsa-version-1-0-release/>
- DSSE: <https://github.com/secure-systems-lab/dsse>
