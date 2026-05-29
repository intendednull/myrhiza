**Date:** 2026-05-29
**Status:** active
**Subject:** TUF's four roles (root / targets / snapshot / timestamp), delegation, M-of-N thresholds, and the offline/online key split — the structural core of the framework.

# TUF roles and metadata

TUF's central insight is **separation of responsibility across roles**, each with its own key(s), so that compromising one key compromises only what that role is allowed to assert. This is the part Myrhiza's `distribution.md` §10.9–10.10 re-derives: a kernel-signing-root distinct from the module-signing allowlist is exactly root-vs-targets separation. See [`lessons.md`](./lessons.md).

## The four top-level roles

A TUF repository is described by four signed metadata files. Each is a JSON document carrying a payload, a **version number**, an **expiration date**, and one or more signatures.

| Role | File | Asserts | Key location | Defeats (primarily) |
|---|---|---|---|---|
| **Root** | `root.json` | which keys are authoritative for *every* role (incl. itself) | **offline** (cold storage / HSM) | key-compromise (the recovery anchor) |
| **Targets** | `targets.json` | the hash + size of each actual target file; may delegate | **offline** | arbitrary-install, wrong-software |
| **Snapshot** | `snapshot.json` | version numbers of all targets-metadata files (a consistent set) | **offline** (online in PyPI's online-only profile) | mix-and-match |
| **Timestamp** | `timestamp.json` | hash + size of the current `snapshot.json`; short expiry | **online** (auto re-signed on a schedule) | freeze / indefinite-freeze |

The split is deliberate: the **timestamp** key is online (it must re-sign frequently to prove freshness), so it is the *most exposed* key — and by design it can assert the *least*. A stolen timestamp key lets an attacker withhold updates (a freeze) but **cannot** introduce a malicious target, because the target hashes are pinned by the offline targets key, whose set is pinned by snapshot, whose authority is pinned by root. This is "minimal trust in high-risk keys" made structural.

## Root: the trust anchor

`root.json` lists, for each of the four roles, the set of authorized public keys and a **threshold** (the minimum number of those keys that must sign). Root signs *itself* too, so root-key rotation is expressed as a new `root.json` signed by the *old* threshold of root keys — a client that trusts version N can verify version N+1 without any out-of-band step (the **root chaining** property). The client is bootstrapped with an initial trusted `root.json` once (the only out-of-band trust step), and walks the chain forward from there.

Root keys are kept offline and are the highest-value, lowest-frequency keys in the system. A typical deployment holds them on hardware tokens distributed across multiple people — see [`tuf-key-compromise-recovery.md`](./tuf-key-compromise-recovery.md) and the Sigstore five-keyholder ceremony in [`tuf-implementations-and-deployments.md`](./tuf-implementations-and-deployments.md).

## Targets and delegated targets

`targets.json` is the authoritative list of *what may be installed*: each entry is a target path plus its cryptographic hash and length. It can **delegate** subtrees of the target namespace to other roles via glob patterns (`projects/foo/*` → "foo-maintainer" role). Delegated targets metadata has the same shape as top-level targets and chains its authority from the delegating role. Delegation is how a large repository (PyPI) lets individual project owners sign their own packages without holding a repository-wide key — the seed of TUF's PEP 480 "developer signing" extension.

## Snapshot: the consistency lock

`snapshot.json` lists the version number of `targets.json` and of every delegated targets file. Its job is to pin a **single coherent view**: a client that trusts a snapshot is guaranteed it is seeing target-metadata files that all existed *together* at one repository state. Without snapshot, an attacker who controls the mirror could serve `targets.json` from today but a delegated file from last year — a **mix-and-match attack** ([`tuf-attack-taxonomy.md`](./tuf-attack-taxonomy.md)).

## Timestamp: the freshness beacon

`timestamp.json` is tiny — just the hash, size, and version of the current `snapshot.json` — and carries a **short expiration** (hours-to-days). It is re-signed automatically on a schedule by an online key. A client that fetches an expired or stale timestamp knows it is being **frozen** (denied fresh metadata) even though the attacker cannot forge a *newer* one. Freshness is thus a positive assertion ("this view is current as of T"), not an absence-of-evidence guess.

## Version numbers and expiration as first-class fields

Every role's metadata carries a monotonic **version number** and an **expiration**. Clients enforce two rules that together kill a whole attack class:

- **Never accept metadata with a lower version than already seen** (anti-rollback).
- **Never accept expired metadata** (anti-freeze).

This is precisely the shape of Myrhiza's `revocation-seq: u64` + `MAX_REVOCATION_JUMP` in `distribution.md` §10.7 (lines 403–409; implemented per plan [`2026-05-26-b-10-bundle-distribution-design.md`](../../specs/2026-05-26-b-10-bundle-distribution-design.md) §3.3) — a signed monotonic counter the client refuses to see decrease. TUF generalizes it across *all four* metadata files rather than just the revocation channel. The `MAX_REVOCATION_JUMP` cap is Myrhiza's own addition to defend against the **fast-forward** variant; see [`tuf-attack-taxonomy.md`](./tuf-attack-taxonomy.md).

## What this buys, structurally

The roles compose into a single property: **no single key compromise lets an attacker install arbitrary software undetected.** That is the headline TUF result, and it is achieved by partitioning authority, not by making any one key unbreakable. The recovery story — what you do *after* a key is stolen — is in [`tuf-key-compromise-recovery.md`](./tuf-key-compromise-recovery.md).

## Sources

- TUF metadata/roles: <https://theupdateframework.io/docs/metadata/>
- TUF spec (roles, thresholds, delegation): <https://theupdateframework.github.io/specification/latest/>
- TUF overview: <https://theupdateframework.io/>
- PEP 458 (online-only PyPI profile): <https://peps.python.org/pep-0458/>
- PEP 480 (delegated developer signing): <https://peps.python.org/pep-0480/>
