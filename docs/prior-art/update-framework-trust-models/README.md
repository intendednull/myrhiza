**Date:** 2026-05-29
**Status:** active
**Subject:** Update-framework trust models — TUF role separation, Uptane (automotive), FROST threshold Schnorr, in-toto/SLSA provenance, transparency logs, and reproducible builds. The trust-model layer ABOVE artifact-signing mechanics.

# Update-framework trust models

How do you keep a software-update channel trustworthy *after* the signing keys you depend on get stolen? That is the question this folder answers, and it is a different question from "how do I sign a `.wasm`."

The [`app-distribution/`](../app-distribution/) folder covers the *mechanics* — Cosign, Notation, Rekor, OCI Referrers — the concrete tools that attach a signature to an artifact. It explicitly lists **"compromised signer at the time of signing"** and **"root-of-trust compromise"** under *"Doesn't defend"* ([`app-distribution/signing.md`](../app-distribution/signing.md) §threat-model). This folder is the layer above those tools: the **trust-model** discipline that decides *which* keys are authoritative, *how many* must agree, *how* a stolen key is survived, and *how* a client detects it is being shown a stale or rolled-back view of the world. The Update Framework (TUF) is the canonical body of work here; everything else in this folder either derives from it (Uptane), composes with it (in-toto/SLSA, transparency logs, reproducible builds), or supplies a missing primitive (FROST threshold signing).

This is a **multi-subject survey** in the shape of [`crdts/`](../crdts/) or [`app-distribution/`](../app-distribution/), not a single-project deep-dive like [`iroh/`](../iroh/).

## Why this folder exists for Myrhiza

Two Myrhiza spec sections hand-roll TUF primitives **without citing TUF**:

- **`distribution.md` §10.7–10.10** — a built-in Ed25519 pubkey allowlist + three offline backup keys + a kernel-signing-root that is deliberately *distinct* from the module-signing allowlist. That is TUF's **root role + role separation** re-derived from scratch.
- **`distribution.md` §10.7** — a monotonic per-author `revocation-seq: u64` with a `MAX_REVOCATION_JUMP` cap (lines 403–409). That is TUF's **rollback-and-fast-forward resistance via signed version numbers** re-derived from scratch. The implementation plan for this channel is [`2026-05-26-b-10-bundle-distribution-design.md`](../../specs/2026-05-26-b-10-bundle-distribution-design.md) §3.3 ("Revocation topic: per-author append-only log via gossip").

A corpus-wide grep returns **zero** occurrences of "TUF" / "Uptane" / "update framework" in `docs/specs/`. The spec is reinventing a CNCF-Graduated, formally-studied design uncited. This folder grounds those decisions so a future spec edit can cite prior art instead of re-deriving it. See [`lessons.md`](./lessons.md).

**It also records a spec correction.** `distribution.md:461` says FROST-Ed25519 schemes "are not yet RFC-stable." This is **false as of 2026**: FROST shipped as **RFC 9591** (June 2024), and the Zcash Foundation's `frost-ed25519` crate is at 3.0.0 (2026-04-23). The Informational/Standards-Track distinction is real and matters; the "not yet RFC-stable" claim does not. See [`frost-threshold-signing.md`](./frost-threshold-signing.md) and [`lessons.md`](./lessons.md) §correction.

## Key facts (verified 2026-05-29)

| Thing | Fact | Source |
|---|---|---|
| TUF — CNCF status | Accepted Incubating **2017-10-24**; **Graduated 2019-12-18** (first CNCF spec project to graduate) | CNCF project page |
| TUF — academic root | "Survivable Key Compromise in Software Update Systems", Samuel, Mathewson, Cappos, Dingledine — **ACM CCS 2010** (pp. 61–72) | ACM DL / freehaven PDF |
| TUF spec | living spec, v1.0.x (1.0.34-era as of early 2026) | theupdateframework spec repo |
| python-tuf | **7.0.0** (2026-05-18), Apache-2.0 OR MIT, CNCF reference impl | PyPI `tuf` |
| go-tuf | legacy v0.7.0 **deprecated**; **go-tuf/v2** (ex-`rdimitrov/go-tuf-metadata`) is the maintained line | theupdateframework/go-tuf |
| tough (Rust TUF) | AWS `awslabs/tough`, built for Bottlerocket (2019) | awslabs/tough |
| Uptane | first release IEEE-ISTO **6100.1.0.0**, **2019-07-31** (= Standard v1.0.0); current Standard **2.1.0** (2023-06-23); two-repository (Image + Director) model | uptane.org / GitHub releases |
| FROST | **RFC 9591**, June 2024, **Informational** (IRTF/CFRG stream), authors Connolly, Komlo, Goldberg, Wood | RFC 9591 |
| FROST academic origin | Komlo & Goldberg, **SAC 2020** (LNCS 12804, pp. 34–65); IACR ePrint 2020/852 | eprint.iacr.org |
| frost-ed25519 (Zcash Fn) | **3.0.0** (2026-04-23), MIT OR Apache-2.0, RFC 9591-conformant, partially NCC-audited | crates.io |
| in-toto | CNCF **Graduated 2025-02-10** (announced 2025-04-23); spec v1.0 (2023) | CNCF announcement |
| SLSA | v1.0 GA **2023-04-19** (OpenSSF); track-based since v1.0 | slsa.dev / OpenSSF |
| Trillian | Google verifiable-log backend, launched 2016; tile-based logs; Tessera is the newer tiled line | transparency.dev |
| Go checksum db (sumdb) | tile-based transparency log, CT-inspired, ships in the Go toolchain | transparency.dev |
| Reproducible Builds | Debian **14 ("Forky")** to penalize reproducibility regressions; `reproduce.debian.net` rebuilderd live | reproducible-builds.org |

**Verify-before-lifting:** TUF's spec is *living* (no frozen "1.0 release"; the version string climbs). FROST being an **Informational** RFC (not Standards-Track) is load-bearing for the §10.9 decision — record it honestly. in-toto's graduation date is **2025-02-10**, not 2023 (the 2023 date is the *spec* v1.0); [`app-distribution/supply-chain.md`](../app-distribution/supply-chain.md) carries the older figure — see [`open-problems.md`](./open-problems.md) §corpus-drift.

## Canonical reading order

1. [`README.md`](./README.md) — this file
2. [`tuf-roles-and-metadata.md`](./tuf-roles-and-metadata.md) — the four roles; role separation; M-of-N thresholds; offline vs online keys
3. [`tuf-attack-taxonomy.md`](./tuf-attack-taxonomy.md) — rollback / freeze / fast-forward / endless-data / mix-and-match as **distinct** attack classes
4. [`tuf-key-compromise-recovery.md`](./tuf-key-compromise-recovery.md) — what survives which key loss; the recovery procedure
5. [`tuf-implementations-and-deployments.md`](./tuf-implementations-and-deployments.md) — python-tuf, go-tuf, tough; PyPI, Sigstore root, RustSec, Bottlerocket
6. [`uptane.md`](./uptane.md) — the ISO/IEEE-standardized automotive derivative
7. [`frost-threshold-signing.md`](./frost-threshold-signing.md) — RFC 9591; the spec correction
8. [`in-toto-slsa-provenance.md`](./in-toto-slsa-provenance.md) — build-provenance context (cross-links app-distribution)
9. [`transparency-logs.md`](./transparency-logs.md) — CT → Go sumdb → Trillian; binary transparency; **the P2P tension**
10. [`reproducible-builds.md`](./reproducible-builds.md) — the "is this binary really from that source" leg
11. [`open-problems.md`](./open-problems.md) — what these systems structurally do NOT solve
12. [`lessons.md`](./lessons.md) — **the decision file**: validates / avoid / borrow
13. [`glossary.md`](./glossary.md) — system-specific terms

## How to use / framing disclosure

These docs are written from **Myrhiza's current design stance**, not as a neutral catalog. That stance is: **capability-mediated** (apps reach the host only through declared imports), **P2P-only** (no Myrhiza-operated service — [`distribution.md`](../../specs/2026-05-09-myrhiza-master-design/distribution.md) §10.8: "No Myrhiza-operated registry. No sigstore dependency. No reliance on any centralized service"), **Component-Model-on-Wasmtime**, and **event-log-replay `state-apply`** (deterministic materialization over a per-author Merkle event DAG). The lessons read every system through that lens. TUF, Uptane, and transparency logs were all designed for a **server-operated repository** the client polls; Myrhiza has no such server. So the "Implications for Myrhiza" sections consistently ask "what survives the move to P2P?" rather than "how do we deploy this repository?" — the **role-separation and version-monotonicity *ideas*** transfer; the **served-repository deployment shape** mostly does not. A reader auditing whether Myrhiza should adopt a served TUF repository after all (a v2 question) should weigh this folder accordingly: it is a learn-the-trust-model-into-a-keyless-P2P-runtime artifact, not a how-to-run-TUF catalog. The transparency-log and reproducible-builds material is housed here but flagged as **v2-deferred** because a Rekor-shape log is in direct tension with §10.8.

**Load-bearing-target caveat — the incentive to soft-pedal.** This is reference material for a design Myrhiza is leaning on; that creates a structural bias toward making the prior art *validate* what the spec already chose. Guard against it: where these systems solve a problem Myrhiza's hand-rolled design does **not** (in-band root-chaining rotation, a hard expiring freshness assertion, secure time, after-the-fact misbehavior detection), the folder is obligated to say so plainly rather than rationalize the gap as "deferred." The known gaps Myrhiza would *inherit* are collected in [`open-problems.md`](./open-problems.md) and the "avoid" column of [`lessons.md`](./lessons.md); read those before concluding the spec's approach is complete.

## Glossary stub

Full terms in [`glossary.md`](./glossary.md). Quick orientation:

- **Root role** — the offline trust anchor; signs which keys are authoritative for every other role.
- **Threshold (M-of-N)** — require M of N keys to sign before metadata is trusted; survives loss of up to M−1 keys.
- **Rollback attack** — serving an *older* signed view to hide a fix or reintroduce a vuln.
- **Freeze attack** — serving a *stale* view forever so the client never sees new versions.
- **FROST** — a threshold *Schnorr* signing protocol producing a single ordinary signature from M-of-N signers (RFC 9591).
- **Transparency log** — an append-only, cryptographically-verifiable public log of signing/issuance events.

## Sources

- TUF / CNCF: <https://www.cncf.io/projects/the-update-framework-tuf/>, <https://theupdateframework.io/>
- TUF spec: <https://theupdateframework.github.io/specification/latest/>
- "Survivable Key Compromise in Software Update Systems" (CCS 2010): <https://www.freehaven.net/~arma/tuf-ccs2010.pdf>, <https://dl.acm.org/doi/10.1145/1866307.1866315>
- python-tuf: <https://pypi.org/project/tuf/>, <https://github.com/theupdateframework/python-tuf>
- go-tuf: <https://github.com/theupdateframework/go-tuf>
- tough: <https://github.com/awslabs/tough>
- Uptane: <https://uptane.org/papers/ieee-isto-6100.1.0.0.uptane-standard.html>
- RFC 9591 (FROST): <https://www.rfc-editor.org/rfc/rfc9591.html>
- FROST origin (SAC 2020): <https://eprint.iacr.org/2020/852>
- frost-ed25519: <https://crates.io/crates/frost-ed25519>, <https://github.com/ZcashFoundation/frost>
- in-toto graduation: <https://www.cncf.io/announcements/2025/04/23/cncf-announces-graduation-of-in-toto-security-framework-enhancing-software-supply-chain-integrity-across-industries/>
- SLSA v1.0: <https://openssf.org/press-release/2023/04/19/openssf-announces-slsa-version-1-0-release/>
- Trillian / transparency.dev: <https://transparency.dev/articles/tile-based-logs/>, <https://google.github.io/trillian/>
- Reproducible Builds: <https://reproducible-builds.org/>, <https://reproduce.debian.net/>
