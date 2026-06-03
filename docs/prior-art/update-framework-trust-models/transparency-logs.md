**Date:** 2026-05-29
**Status:** active
**Subject:** Transparency logs as a single subject — Certificate Transparency → Go checksum database → Trillian/Tessera/Sunlight, plus Sigstore Rekor and binary transparency. AND the honest tension with Myrhiza's no-central-service rule.

# Transparency logs

A **transparency log** is an append-only, cryptographically-verifiable public record of events (certificates issued, modules published, signatures made), built on a Merkle tree so that anyone can verify (a) an entry is *included* and (b) the log is *append-only* (never rewritten), without trusting the log operator. It is the standard answer to the attack TUF's channel-protection cannot reach: **a malicious-but-validly-signed artifact, or a silently re-signed one.** TUF stops a stolen *channel* key; a transparency log makes a *signer's own* misbehavior publicly detectable after the fact.

**This file carries the folder's central honest tension** (see [`README.md`](./README.md) framing disclosure): every system below assumes a **central, operated, monitored log**. Myrhiza's `distribution.md` §10.8 forbids exactly that ("No Myrhiza-operated registry … No reliance on any centralized service"). So this material is housed here as the *trust-model option Myrhiza has deferred to v2*, not as a v1 recommendation. `distribution.md` §10.10 itself names "kernel-binary transparency log + community attestation" as **future direction (v2+)**.

## The lineage (one subject, three steps)

### 1. Certificate Transparency (CT)

CT is the origin deployment: a public, append-only Merkle log of issued TLS certificates so that mis-issuance (a CA signing a cert it shouldn't) becomes publicly visible. Browsers require certs to carry proof of CT-log inclusion. The model: **operators run logs; independent monitors and auditors watch them; the Merkle structure means no operator can lie about history undetected.** CT proved the pattern works at internet scale.

### 2. The Go checksum database (sumdb)

The Go toolchain ships a **transparency log for module checksums**: when you fetch a Go module, the toolchain checks its hash against `sum.golang.org`, a tile-based transparency log, so that a module's content cannot be silently changed after first publication without detection. This is **binary/artifact transparency** in a mainstream toolchain, and it is **CT-inspired but uses a tile format** for efficient caching. It is the closest analog to "what if Myrhiza had a transparency log for module hashes" — and notably, it is still a *served* log (`sum.golang.org`).

### 3. Trillian → Tessera / Sunlight (the backend)

**Trillian** (Google, launched 2016) is the general-purpose verifiable-log backend that powers CT, binary transparency, and others — a Merkle tree served from a storage layer, scalable to very large trees. The newer direction is **tile-based logs**: split the Merkle tree into tiles (each holding many hashes) served as static files, so the read path is "a filesystem + a page cache," no database. **Tessera** (the Trillian team's tiled-log library, alpha) and **Sunlight** (Let's Encrypt's scalable CT implementation, 2024) are the current embodiments. The trend is toward **cheaper, static-file, operator-light** logs — relevant because it lowers the bar for a community-operated (vs vendor-operated) log, which is the only kind §10.8 could ever tolerate.

## Sigstore Rekor

**Rekor** is Sigstore's transparency log for *signatures* (see [`app-distribution/signing.md`](../app-distribution/signing.md) for the signing mechanics). Its trust-model job: record every keyless signature so that "this artifact was signed by identity X within the validity window of X's short-lived Fulcio cert" is publicly auditable, and a **silent re-sign is detectable**. Rekor v2 — the tile-backed redesign — reached GA October 10, 2025. Rekor is the canonical "transparency log over software signatures" — and it is exactly the **Rekor-shape central service** that is in tension with Myrhiza §10.8. Sigstore's *own root* is distributed via TUF ([`tuf-implementations-and-deployments.md`](./tuf-implementations-and-deployments.md)); its *signature history* lives in Rekor. Both are operated services.

## The P2P tension, stated plainly

A transparency log's security comes from **a public, monitored, append-only record everyone can audit**. That requires:

- a place the log lives (an operator), and
- monitors/auditors who watch it and gossip its head (a witness network).

Myrhiza has **neither** by design. Options, none free:

1. **Defer (current v1 stance).** Rely on TUF-style channel protection + reproducible builds + multi-channel announcement. Accept that "malicious-but-signed by the real author" is detectable only out-of-band. This is what §10.10 does.
2. **Reuse an existing log (v2).** Publish module hashes to `sum.golang.org`-style or Rekor — but that *is* depending on a central service, contradicting §10.8 unless framed as "optional external corroboration, not required for operation."
3. **Build a P2P witness gossip (research).** Myrhiza already gossips per-author revocation topics over iroh-gossip (§10.7). A signed, append-only *log head* could be gossiped the same way, with peers acting as witnesses — a genuinely P2P transparency log, but unproven and a real research project. The tile-based static-file trend (Tessera/Sunlight) makes the storage side cheaper, but the **witness/monitor** side is the hard part in a network with no fixed membership. See [`open-problems.md`](./open-problems.md).

The honest conclusion: **transparency logs are the right answer to the gap TUF leaves, but their shape fights Myrhiza's no-server axiom.** Record the option, defer the build, and if v2 wants it, the iroh-gossip witness path is the only one that doesn't reintroduce a central service. See [`lessons.md`](./lessons.md) §borrow and §avoid.

## Sources

- Certificate Transparency: <https://certificate.transparency.dev/>
- Go checksum database / tile-based logs: <https://transparency.dev/articles/tile-based-logs/>
- Trillian: <https://google.github.io/trillian/>
- Tessera (tiled logs): <https://transparency.dev/>
- Sunlight (Let's Encrypt CT): <https://letsencrypt.org/2024/03/14/introducing-sunlight>
- Rekor: <https://github.com/sigstore/rekor>, <https://docs.sigstore.dev/>
