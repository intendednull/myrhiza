**Date:** 2026-05-29
**Status:** active
**Subject:** What these trust models structurally do NOT solve — the gaps Myrhiza inherits when it adopts (or re-derives) TUF-shaped trust, plus corpus-drift facts flagged for the index owner.

# Open problems

What TUF / Uptane / FROST / transparency logs / reproducible builds **cannot** do for a P2P, no-central-service runtime. Each entry: the gap, why it bites Myrhiza, and where to look.

## 1. The served-repository assumption

Every system here except FROST-verification and reproducible-builds assumes a **central, operated repository or log the client polls** — PyPI, Sigstore's CDN, Bottlerocket's update server, Uptane's Director, Rekor, `sum.golang.org`. Myrhiza's `distribution.md` §10.8 forbids that shape outright. So the *role/threshold/version/freshness model* transfers, but the *deployment* does not. The unsolved work: re-express TUF's freshness beacon (timestamp role) and consistency lock (snapshot role) over **iroh-gossip** with **no fixed operator**. The §10.7 per-author revocation topic + HeadsSummary sync is a partial freshness mechanism; turning it into a hard, signed, expiring freshness assertion is open spec work. See [`transparency-logs.md`](./transparency-logs.md), [`tuf-attack-taxonomy.md`](./tuf-attack-taxonomy.md).

## 2. Freeze defense without a trusted clock or trusted timestamper

A freeze attack ([`tuf-attack-taxonomy.md`](./tuf-attack-taxonomy.md)) is only detectable via an **expiring** freshness assertion, which needs **current time**. Myrhiza's §10.7 "potentially stale after 24h" warning trusts the **local clock** — an attacker who can skew the clock (or simply a long-offline peer) defeats it. Uptane solves this with an explicit secure-time / nonce mechanism ([`uptane.md`](./uptane.md)); Myrhiza has no equivalent. Open: a P2P secure-time source, or accept clock-trust as a documented limitation.

## 3. Malicious-but-signed author (the gap above the channel)

TUF protects the *channel*; it does nothing about an author who signs genuinely malicious software with their real key — the "compromised signer at time of signing" gap from [`app-distribution/signing.md`](../app-distribution/signing.md). The standard answer is a **transparency log + monitoring**, which §10.8 defers. Reproducible builds + SLSA provenance narrow it (you can see *what was built*) but cannot judge intent. This gap is **acknowledged-and-deferred**, not solved, in v1. See [`transparency-logs.md`](./transparency-logs.md), [`in-toto-slsa-provenance.md`](./in-toto-slsa-provenance.md).

## 4. Root rotation without out-of-band re-bootstrap

TUF's root-chaining lets clients accept a new `root.json` in-band. Myrhiza's hard-coded `const` allowlist (§10.9) has **no chaining** — every allowlist change is a kernel re-install (an out-of-band re-bootstrap), even for routine non-root rotations. Adopting root-chaining means adding root-metadata verification to the kernel TCB, which §10.9 deliberately avoided. Genuine tradeoff, not yet decided. See [`tuf-key-compromise-recovery.md`](./tuf-key-compromise-recovery.md), [`lessons.md`](./lessons.md).

## 5. Threshold is a manual ceremony, not enforced crypto

§10.9's three backup keys are "not used as a cryptographic threshold signature in v1." So "no single key suffices" is a **policy promise**, not a **structural guarantee** — a single compromised primary key, plus social engineering of the announcement channels, is the actual attack surface. FROST (RFC 9591, [`frost-threshold-signing.md`](./frost-threshold-signing.md)) is the primitive that makes it structural while keeping the kernel verifier unchanged; the unsolved part is whether the maintainer-side DKG/signing ceremony operational cost is worth it for v1.

## 6. Transparency without an operator or a witness network

A P2P transparency log (option 3 in [`transparency-logs.md`](./transparency-logs.md)) needs **witnesses** that gossip and cross-check the log head. In a network with no fixed membership, "who are the witnesses and why trust their gossip?" is unsolved — it edges into the Sybil-resistance problem (neighbor [`sybil-resistance/`](../sybil-resistance/)). Tile-based static logs (Tessera/Sunlight) cheapen storage but not the witness problem. Pure research for now.

## 7. Revocation reach vs. partition

§10.7 propagates revocations over gossip; a partitioned or freshly-installed peer may simply not have heard the latest revocation. TUF's timestamp-freshness model would surface this as "your view is expired"; Myrhiza's soft warning does not hard-block. The interplay of revocation reach, freeze defense, and partition tolerance is one combined open problem — and it touches convergence (neighbor [`willow/`](../willow/), the per-author chain / HeadsSummary surface).

## Corpus-drift facts (flagged for the index owner — not edited here)

Per the task rules, this folder does not modify neighbor folders or the index. Two verified facts that *contradict* existing corpus entries, surfaced for whoever owns those files:

- **in-toto graduation date.** [`app-distribution/supply-chain.md`](../app-distribution/supply-chain.md) line 53 says "in-toto (CNCF Graduated **2023-09-12**)." Verified: in-toto's **spec** reached v1.0 in 2023, but **CNCF graduation was 2025-02-10** (announced 2025-04-23 by CNCF). The 2023-09-12 date appears to conflate the two. See [`in-toto-slsa-provenance.md`](./in-toto-slsa-provenance.md).
- **RustSec as a "deployed TUF" example.** The gap-analysis report lists RustSec among TUF deployments. Verified caveat: RustSec's advisory DB primary distribution is a **git repository**; full TUF coverage of crates.io has been *proposed* more than *shipped*. Cite RustSec as the Rust vulnerability channel, but verify its exact TUF integration before calling it a live TUF *repository*. See [`tuf-implementations-and-deployments.md`](./tuf-implementations-and-deployments.md).

## Sources

- Synthesized from sibling files; primary URLs cited there.
- TUF security model (channel-only scope): <https://theupdateframework.io/docs/security/>
- in-toto graduation (2025-04-23): <https://www.cncf.io/announcements/2025/04/23/cncf-announces-graduation-of-in-toto-security-framework-enhancing-software-supply-chain-integrity-across-industries/>
- RustSec: <https://rustsec.org/>, <https://github.com/rustsec/advisory-db>
