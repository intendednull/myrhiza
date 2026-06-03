**Date:** 2026-05-29
**Status:** active
**Subject:** THE decision file — what Myrhiza's signing-root / revocation / threshold design should take from TUF, Uptane, FROST, transparency logs, and reproducible builds. Validates / avoid / borrow / correction, tied to distribution.md §10.7–10.10.

# Lessons for Myrhiza

Other files are evidence; this file is the takeaway. It is driven by the decision surface named in the task:

- **`distribution.md` §10.7–10.10** — built-in Ed25519 allowlist + 3 offline backup keys + kernel-signing-root vs module-signing-root separation.
- **`distribution.md` §10.7** (lines 403–409) — monotonic `revocation-seq` + `MAX_REVOCATION_JUMP` rollback resistance (implemented per plan [`2026-05-26-b-10-bundle-distribution-design.md`](../../specs/2026-05-26-b-10-bundle-distribution-design.md) §3.3).

Both **hand-roll TUF primitives, uncited.** This folder is the trust-model layer *above* [`app-distribution/`](../app-distribution/)'s tool mechanics; keep that altitude — these lessons are about *which keys are authoritative and how compromise is survived*, not about Cosign/OCI plumbing.

## Validates

The corpus **confirms** these Myrhiza bets:

- **Role separation is the right instinct.** §10.10's kernel-signing-root distinct from §10.9's module-signing allowlist is exactly TUF's root-vs-targets separation. The intuition — "the key that authenticates the runtime is not the key that authenticates the apps" — is the load-bearing idea of a CNCF-Graduated, IEEE-standardized (via Uptane) framework. Myrhiza got the *shape* right. ([`tuf-roles-and-metadata.md`](./tuf-roles-and-metadata.md))
- **Monotonic signed version numbers are the correct rollback defense.** §10.7's `revocation-seq: u64` is TUF's anti-rollback rule. Independent re-derivation of a vetted design is a *good* sign, not a smell. ([`tuf-attack-taxonomy.md`](./tuf-attack-taxonomy.md))
- **Bounding the version jump is the correct fast-forward defense.** `MAX_REVOCATION_JUMP` is the textbook mitigation for the fast-forward attack most update systems forget. Myrhiza anticipated a class TUF had to name explicitly.
- **Reproducible builds belong in v1.** §10.10's "build from source, compare checksums" is a mature, server-free practice now gating a major distro (Debian 14). It composes perfectly with content-addressed iroh-blobs transport. ([`reproducible-builds.md`](./reproducible-builds.md))
- **Deferring a central transparency log is defensible.** A Rekor-shape log fights §10.8's no-server axiom. §10.10 correctly files it as v2+. ([`transparency-logs.md`](./transparency-logs.md))
- **Offline backup keys + emergency re-bootstrap is the right recovery anchor.** §10.10's out-of-band root recovery matches TUF's "the one irreducibly out-of-band moment" (root re-bootstrap). ([`tuf-key-compromise-recovery.md`](./tuf-key-compromise-recovery.md))

## Avoid

The corpus shows where the **easy mistakes** are:

- **Don't leave the TUF lineage uncited.** A future reviewer of §10.7–10.10 will see hand-rolled role-separation + version-monotonicity and wonder if the attack classes were enumerated. **Cite TUF** (CNCF-Graduated 2019, the "Survivable Key Compromise" CCS 2010 paper) and the attack taxonomy so the design reads as *informed by prior art*, not *improvised*. This is the single highest-value action.
- **Don't conflate rollback and freeze.** They need **different** defenses (monotonic counter vs expiring freshness beacon). §10.7 has a strong rollback defense (`revocation-seq`) but only a **soft** freeze defense (a 24h "potentially stale" *warning*). A warning is an under-powered timestamp role. Decide deliberately whether freshness should be a hard, signed, expiring assertion. ([`tuf-attack-taxonomy.md`](./tuf-attack-taxonomy.md))
- **Don't trust the local clock for freshness.** The §10.7 staleness warning assumes an honest clock. Uptane treats secure time as a first-class attack surface; Myrhiza should at least *document* clock-trust as a known limitation, or design a P2P time source. ([`uptane.md`](./uptane.md), [`open-problems.md`](./open-problems.md) §2)
- **Don't call a manual ceremony a threshold.** §10.9 is honest that the 3 backups are "not used as a cryptographic threshold signature" — good. But the wording elsewhere ("defense in depth via separate trust roots") can read as if threshold security exists. It does not, until FROST or N-of-M TUF-root signing is adopted. Keep that distinction crisp.
- **Don't adopt the served-repository or central-Director topology.** Take TUF's *role model* and Uptane's *Image/Director separation*, but not their server-shaped deployment — §10.8 forbids it. The mistake would be reaching for "just run a TUF repo." ([`open-problems.md`](./open-problems.md) §1)
- **Don't let SLSA/provenance be misread as a behavior or determinism guarantee.** Provenance attests *how it was built*, not that a `state-apply` component is pure. Determinism comes from the profile model, not a build badge. ([`in-toto-slsa-provenance.md`](./in-toto-slsa-provenance.md))

## Borrow

Specific primitives worth studying / lifting:

- **TUF's full attack taxonomy as a spec checklist.** [`tuf-attack-taxonomy.md`](./tuf-attack-taxonomy.md) now carries a per-class coverage table against §10.x. Content-addressing (the BLAKE3 hash *is* the trust binding) structurally moots wrong-software, extraneous-deps, and substitution-by-mirror, and pins the mix-and-match set per app. The two cells still open are **freeze** (a soft staleness warning, not a hard expiring signed assertion) and **key-compromise threshold** (a manual backup ceremony, not enforced crypto); endless-data leaves a residual first-fetch byte-cap to add.
- **Root-chaining for in-band rotation.** TUF lets a client accept a new root signed by the *old* root threshold — no re-install. If §10.9's "kernel re-install to rotate the allowlist" friction becomes painful, root-chaining is the design that removes it (at the cost of root-metadata verification in the TCB — weigh it). ([`tuf-key-compromise-recovery.md`](./tuf-key-compromise-recovery.md))
- **FROST for the §10.9 quorum, when ready.** RFC 9591 FROST-Ed25519 turns the 3 backup keys into a *real* M-of-N threshold whose output is **one ordinary Ed25519 signature** — zero new verification logic in the kernel TCB beyond stock Ed25519. It is the exact primitive §10.9 wants. ([`frost-threshold-signing.md`](./frost-threshold-signing.md))
- **Uptane's Image/Director split** for any "operator targets/endorses a module at a peer cohort" feature: keep authenticate-the-artifact and target-the-recipient as separate roots. ([`uptane.md`](./uptane.md))
- **Uptane's full-vs-partial verification tiering** for heterogeneous peers (a browser/jco peer is not as capable a verifier as a native kernel).
- **Reproducible-build discipline** (`SOURCE_DATE_EPOCH`, `--remap-path-prefix`, pinned WASM toolchain) for *both* the kernel binary and published modules, so any peer can rebuild-and-compare. ([`reproducible-builds.md`](./reproducible-builds.md))
- **The iroh-gossip witness path** as the *only* §10.8-compatible route to a future transparency log: gossip signed log heads over the existing revocation-topic machinery, peers as witnesses. Research, not v1. ([`transparency-logs.md`](./transparency-logs.md), [`open-problems.md`](./open-problems.md) §6)

## Correction (action item for a spec edit — not made here)

**`distribution.md:461` is factually stale.** It says FROST-Ed25519 schemes "are not yet RFC-stable." Verified false: FROST is **RFC 9591** (June 2024, Informational, IRTF/CFRG; authors Connolly/Komlo/Goldberg/Wood), and `frost-ed25519` 3.0.0 (2026-04-23, MIT OR Apache-2.0, NCC-partially-audited) is a stable Rust implementation.

The §10.9 *decision* (defer FROST for v1) may still be correct — but for the **right reason**. Recommended replacement rationale:

> *FROST-Ed25519 is specified in RFC 9591 (Informational, IRTF/CFRG, 2024) with a stable, audited Rust implementation. We defer adopting it not because it lacks an RFC but because (a) RFC 9591 is Informational, not IETF Standards-Track, and (b) adding threshold-signing ceremony tooling is maintainer-side operational cost we defer past v1. Verification cost is nil — a FROST signature verifies as an ordinary Ed25519 signature.*

This folder does not edit the spec (task rule). Flagged for the reviewer / spec owner. The gap-analysis report ([`docs/reports/2026-05-29-prior-art-gap-analysis.md`](../../reports/2026-05-29-prior-art-gap-analysis.md) §spec-hygiene) already logs this; this folder supplies the verified replacement text.

## Recommended posture for the runtime spec

A defensible default:

1. **Cite TUF** in §10.7–10.10 — the design is sound; ground it in the prior art instead of re-deriving it silently.
2. **Walk the attack taxonomy** as a coverage checklist; explicitly state which classes are in-scope/out-of-scope at v1.
3. **Strengthen the freeze defense** from a soft staleness warning toward a hard, signed, expiring freshness assertion (timestamp-role analog), and document the clock-trust assumption.
4. **Keep FROST on the roadmap** with corrected rationale; adopt it when the maintainer ceremony cost is justified — it makes "no single key suffices" structural at no verifier cost.
5. **Hold transparency logs at v2**, and if pursued, only via the iroh-gossip witness path (never a central Rekor-shape service).
6. **Commit reproducible builds in v1** for kernel + modules; it is the server-free leg that hardens §10.10's acknowledged "compromised release infra + package manager" risk.

## Sources

This file synthesizes the sibling evidence files; primary URLs are cited there. Key anchors:

- TUF role/attack/recovery model — [`tuf-roles-and-metadata.md`](./tuf-roles-and-metadata.md), [`tuf-attack-taxonomy.md`](./tuf-attack-taxonomy.md), [`tuf-key-compromise-recovery.md`](./tuf-key-compromise-recovery.md)
- FROST correction — [`frost-threshold-signing.md`](./frost-threshold-signing.md); RFC 9591: <https://www.rfc-editor.org/rfc/rfc9591.html>; frost-ed25519: <https://crates.io/crates/frost-ed25519>
- Uptane — [`uptane.md`](./uptane.md)
- Transparency / reproducibility — [`transparency-logs.md`](./transparency-logs.md), [`reproducible-builds.md`](./reproducible-builds.md)
- Mechanics layer below this folder — [`app-distribution/signing.md`](../app-distribution/signing.md), [`app-distribution/supply-chain.md`](../app-distribution/supply-chain.md)
