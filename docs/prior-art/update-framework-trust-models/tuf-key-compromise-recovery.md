**Date:** 2026-05-29
**Status:** active
**Subject:** TUF's compromise-survivability model — what an attacker can do with each stolen key, and the recovery procedure. The "survivable key compromise" thesis that gives the framework its name.

# TUF key-compromise recovery

TUF's founding paper is literally titled *"Survivable Key Compromise in Software Update Systems"* (Samuel, Mathewson, Cappos, Dingledine — ACM CCS 2010). The thesis: you **will** lose a key eventually, so design so that losing one is survivable rather than catastrophic. This is the part Myrhiza's `distribution.md` §10.9 gestures at ("defense in depth via separate trust roots") but stops short of formalizing.

## What each stolen key buys an attacker

The role separation from [`tuf-roles-and-metadata.md`](./tuf-roles-and-metadata.md) pays off here. With a single stolen key, an attacker's reach is bounded:

| Stolen key | Attacker can | Attacker **cannot** |
|---|---|---|
| **Timestamp** (online, most exposed) | mount a freeze (withhold updates) | install any new or altered target |
| **Snapshot** | (with timestamp) mount mix-and-match within already-signed targets | introduce a *new* malicious target hash |
| **Targets** (offline) | sign a malicious target — **but** snapshot+timestamp must also vouch, and root can revoke the targets key | survive a root-mediated key rotation |
| **A delegated targets** | forge only within its delegated path glob | touch any other project's namespace |
| **Root** (offline, threshold) | nothing, *unless* enough root keys (≥ threshold) are stolen to forge a new `root.json` | act at all below the threshold |

The headline: **arbitrary software installation requires compromising the targets key AND defeating the snapshot/timestamp freshness AND evading root-mediated revocation.** No single key is sufficient. This is "defense in depth via separate trust roots" stated precisely — exactly the property `distribution.md` §10.9 claims qualitatively but does not partition.

## Thresholds: surviving partial compromise

Each role can require **M-of-N** signatures (threshold). Root commonly uses a real threshold (e.g. 3-of-5) so that:

- losing up to **M−1** root keys leaks *nothing* (an attacker below threshold cannot sign);
- losing up to **N−M** root keys still lets the legitimate holders rotate (enough remain to meet threshold).

This is the spread Sigstore uses with its five-keyholder root ceremony ([`tuf-implementations-and-deployments.md`](./tuf-implementations-and-deployments.md)). Note the threshold here is **N independent signatures on one document**, not a single aggregated signature — that is the difference from FROST, which produces *one* signature from M-of-N signers. See [`frost-threshold-signing.md`](./frost-threshold-signing.md) for why that distinction matters to Myrhiza's §10.9 decision.

## The recovery procedure

When a non-root key is suspected compromised:

1. The **root** role (offline, threshold) signs a **new `root.json`** that lists fresh public keys for the affected role and revokes the old ones.
2. Clients, walking the **root chain**, verify the new `root.json` against the *previous* trusted root's threshold — **no out-of-band step required**. The compromise is repaired in-band.
3. New snapshot/timestamp metadata is issued under the rotated keys; old metadata fails verification on version/expiry grounds.

When **root itself** is compromised (≥ threshold root keys stolen), there is no in-band fix — recovery requires an **out-of-band re-bootstrap**: ship a fresh initial `root.json` through a trusted channel (new release, OS package, signed announcement). This is the one irreducibly out-of-band moment, and it is exactly the moment Myrhiza's `distribution.md` §10.10 handles with "emergency kernel binary update + multi-channel announcement." TUF's contribution is to make this the *only* moment that needs out-of-band trust; Myrhiza currently needs out-of-band trust at more points because it has no root-chaining.

## Myrhiza's gap, named precisely

`distribution.md` §10.9 hard-codes the official allowlist as a `const` in the kernel binary, so **updating the allowlist requires re-installing the kernel**. That is a TUF root role with **no root-chaining**: every rotation is an out-of-band re-bootstrap, even for a routine non-root rotation. The three offline backup keys are present but "not used as a cryptographic threshold signature in v1" — i.e. they are an *N* without an *M*, a manual ceremony rather than enforced threshold logic. TUF's recovery model is the design that turns those backups into a real threshold with in-band rotation. Whether Myrhiza wants that (it adds verification logic to the kernel TCB, the very thing §10.9 declined) is a genuine tradeoff — see [`lessons.md`](./lessons.md) §borrow and §avoid.

## Sources

- "Survivable Key Compromise in Software Update Systems" (CCS 2010): <https://www.freehaven.net/~arma/tuf-ccs2010.pdf>, <https://dl.acm.org/doi/10.1145/1866307.1866315>
- TUF security model: <https://theupdateframework.io/docs/security/>
- TUF spec (root chaining, thresholds): <https://theupdateframework.github.io/specification/latest/>
- Sigstore root key ceremony: <https://github.com/sigstore/root-signing>
