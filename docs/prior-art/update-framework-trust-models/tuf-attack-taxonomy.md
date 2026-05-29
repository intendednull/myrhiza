**Date:** 2026-05-29
**Status:** active
**Subject:** TUF's attack taxonomy — rollback, fast-forward, freeze, endless-data, mix-and-match, and the rest — treated as DISTINCT classes with distinct defenses. The conceptual contribution Myrhiza's revocation design re-derives.

# TUF attack taxonomy

TUF's lasting conceptual contribution is not a protocol — it is a **taxonomy**. It enumerates the ways an update channel fails *given that the attacker can already see and manipulate traffic and may hold some keys*, and shows that several of these are **genuinely distinct** and need **separate** defenses. Lumping them into "tampering" is the mistake TUF was built to prevent. Myrhiza's `distribution.md` §10.7 revocation design independently re-derives the rollback and fast-forward defenses; this file names the full set so a spec author can check coverage.

## The distinct classes

| Attack | What the attacker does | Why it is its own class | TUF defense |
|---|---|---|---|
| **Arbitrary software installation** | Serve a malicious file as if legitimate | The base case; everything else is a way *around* a signature check | signed target hashes (targets role) |
| **Rollback** | Serve an *older*, validly-signed version to reintroduce a known vuln / hide a fix | The bytes are genuinely signed — signature check alone passes | refuse metadata with version < last-seen |
| **Fast-forward** | Inflate the version number absurdly high so later *legitimate* updates look "older" and get rejected | The inverse of rollback; attacks the client's own rollback defense | bounded version increments / re-bootstrap; Myrhiza's `MAX_REVOCATION_JUMP` |
| **Indefinite freeze** | Keep serving the last view the client saw, forever | No tampering at all — the attacker just withholds | short-lived signed timestamp w/ expiry |
| **Endless data** | Answer a download with an infinite byte stream | DoS on disk/memory, not a trust break | signed *length* of every file, checked while downloading |
| **Mix-and-match** | Combine metadata/targets that never coexisted on the repo | Each piece is individually valid; the *combination* is the forgery | snapshot role pins one coherent set |
| **Extraneous dependencies** | Trick the client into pulling unrelated vulnerable software | Exploits the dependency resolver, not the signature | targets metadata is the closed set of installables |
| **Wrong software installation** | Serve a different-but-trusted file than requested | Right signer, wrong artifact | target *path*→hash binding |
| **Malicious mirror** | One mirror among many blocks or degrades to force a freeze/rollback | Availability attack mounted from inside the distribution fabric | timestamp freshness + threshold metadata defeats single-mirror lies |
| **Key compromise** | Steal one or more signing keys | The meta-class the whole design is organized around | role separation + thresholds + offline keys (see [`tuf-key-compromise-recovery.md`](./tuf-key-compromise-recovery.md)) |

## Why "rollback" and "freeze" are NOT the same

This distinction is the single most transferable idea for Myrhiza, so it gets its own treatment.

- A **rollback** is an *active* lie: "version 3 is the latest" when version 5 exists. The defense is a **monotonic version counter** the client refuses to see decrease. The attacker must possess a validly-signed *old* artifact (which exists, because it was once real).
- A **freeze** is *omission*: the client is simply never shown anything newer than what it last saw. There is nothing to "not-verify" — every byte is genuine. The only defense is a **positive freshness assertion that expires** — the timestamp role. Without an expiring beacon, a client cannot distinguish "nothing new has shipped" from "I am being held at version 3 forever."

A monotonic counter defends rollback but is **powerless against freeze**, and an expiry beacon defends freeze but is **powerless against rollback**. You need both. Myrhiza's `revocation-seq` is the rollback half; its 24-hour "potentially stale" warning before installing a new version (`distribution.md` §10.7 stale-network mitigation) is a partial freeze half — TUF would frame that warning as an *under-powered timestamp role* and suggest making freshness a hard, signed, expiring assertion rather than a soft warning. See [`lessons.md`](./lessons.md) §borrow.

## Fast-forward: the trap Myrhiza already saw

The **fast-forward attack** is the one most update systems forget. If your only defense is "reject version ≤ last-seen," an attacker with a signing key can publish version `u64::MAX` once, and now every *legitimate* future version is rejected as a rollback — bricking the channel. Myrhiza's `MAX_REVOCATION_JUMP` (default 1024 per author per 24h, `distribution.md` §10.7) is exactly the standard fast-forward mitigation: bound how far the counter may jump in one step, so a single compromised signature cannot poison the monotonic counter's whole range. TUF literature recommends the same bounded-increment posture plus a root-mediated recovery path. This is a case where Myrhiza got it right by independent derivation — worth *citing* TUF rather than leaving it to look like an unexamined invention.

## Per-class coverage against Myrhiza's current spec

[`lessons.md`](./lessons.md) repeatedly says to "walk the attack taxonomy as a coverage checklist." This is that walk over all ten classes above, mapping each to Myrhiza's current `distribution.md` §10.x posture. The dominant structural fact: Myrhiza is **content-addressed** — the BLAKE3 bundle hash *is* the trust binding (§10.6: "semver is informative only"), fetched by hash over iroh-blobs (§10.5 step 2). Several TUF attack classes exist *only because* TUF binds trust to a mutable name a repository serves; content-addressing neutralizes those at the root, so the residual Myrhiza surface is narrower than TUF's — but not empty.

| Attack class | Myrhiza posture | Status |
|---|---|---|
| **Arbitrary software installation** | Ed25519 signature verified against the author pubkey embedded in the manifest (§10.5 step 3); no field to declare a non-Ed25519 algorithm | **covered** |
| **Rollback** | `revocation-seq: u64` monotonic per author (§10.7); kernel rejects lower-or-equal seq | **covered** (revocation channel) |
| **Fast-forward** | `MAX_REVOCATION_JUMP` (default 1024 / author / 24h, §10.7) bounds the jump | **covered** |
| **Indefinite freeze** | 24h "potentially stale" *warning* only (§10.7), trusting the local clock | **partial / soft** — no hard expiring signed freshness assertion; see [`open-problems.md`](./open-problems.md) §2 |
| **Endless data** | BLAKE3/Bao verified streaming commits total length in the root hash, so an over-long stream against a *known* hash fails verification ([`iroh/blobs.md`](../iroh/blobs.md) §Bao). Residual: a *first* fetch of an unknown-size blob has no a-priori length bound | **mostly covered**, residual first-fetch DoS |
| **Mix-and-match** | Module deps pin **content hashes** recursively (§10.5 step 4, §10.6); the dep tree is one hash-closed set per app version. No `snapshot.json`-style cross-author consistency lock, but per-app the set is pinned | **covered per-app**; no cross-author snapshot analog (likely unneeded with content addressing) |
| **Extraneous dependencies** | Install resolves only the manifest's declared, hash-pinned deps (§10.5 step 4) — a closed installable set, not a name-resolver | **covered** |
| **Wrong software installation** | Content hash *is* the identifier; there is no name→artifact indirection to subvert | **covered** (structurally moot) |
| **Malicious mirror** | iroh-blobs is content-addressed P2P; any provider's bytes are BLAKE3-verified against the requested hash, so a bad provider can withhold (a freeze/availability attack) but cannot substitute | **covered for substitution**; availability folds into the freeze row + [`open-problems.md`](./open-problems.md) §7 |
| **Key compromise** | Built-in allowlist + offline backups + kernel-vs-module root separation (§10.9–10.10); but the backups are a manual ceremony, not an enforced threshold | **partial** — see [`tuf-key-compromise-recovery.md`](./tuf-key-compromise-recovery.md), [`open-problems.md`](./open-problems.md) §5 |

The two genuinely open cells are **freeze** (soft warning, not a hard expiring assertion) and **key-compromise threshold** (manual ceremony, not crypto) — both already in [`open-problems.md`](./open-problems.md) and [`lessons.md`](./lessons.md) §avoid. Endless-data's first-fetch residual is worth an explicit per-fetch byte cap in the install flow.

## What the taxonomy does NOT cover

TUF's taxonomy is about the *channel*. It says nothing about whether the signed artifact is itself **correct, safe, or non-malicious-by-its-author** — that is the "compromised signer at time of signing" gap noted in [`app-distribution/signing.md`](../app-distribution/signing.md) and addressed (partially) by transparency logs ([`transparency-logs.md`](./transparency-logs.md)) and reproducible builds ([`reproducible-builds.md`](./reproducible-builds.md)). See [`open-problems.md`](./open-problems.md).

## Sources

- TUF security / attack taxonomy: <https://theupdateframework.io/docs/security/>
- TUF spec (version/expiration semantics): <https://theupdateframework.github.io/specification/latest/>
- "Survivable Key Compromise in Software Update Systems" (CCS 2010): <https://www.freehaven.net/~arma/tuf-ccs2010.pdf>
- Fast-forward attack discussion (TUF spec history): <https://github.com/theupdateframework/specification>
