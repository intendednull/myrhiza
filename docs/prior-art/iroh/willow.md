**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — `iroh-willow` Willow protocol implementation (preview / stalled)

# iroh-willow

A partial Rust implementation of the [Willow protocol](https://willowprotocol.org/) — *"a minimal implementation of Willow, Meadowcap, and Confidential Sync with iroh"* ([README](https://github.com/n0-computer/iroh-willow/blob/main/README.md)). Important caveat up front: **this crate has had no substantial code change in over a year.** Don't plan a 2026 dependency on it without a fallback.

## Status (verified 2026-05-08)

[`n0-computer/iroh-willow`](https://github.com/n0-computer/iroh-willow); not archived. But:

- **Single GitHub release: `v0.0.1`.** No release line, no tagged versions tracking iroh's evolution.
- **`Cargo.toml` declares `version = "0.28.0"`** ([file](https://github.com/n0-computer/iroh-willow/blob/main/Cargo.toml)) — a number that has not advanced.
- **Dependencies are stuck at iroh 0.34.** Today's iroh is 1.0.0-rc.0; iroh-blobs is 0.100; iroh-docs is 0.99. iroh-willow points at `iroh = "0.34.0"`, `iroh-blobs = git "matheus23/verified-streams"` (a feature branch).
- **Last functional commit was March 25, 2025** (`feat: Upgrade to Iroh 0.34.0`, [d68310a](https://github.com/n0-computer/iroh-willow/commit/d68310a)). Everything since is dependabot updates, CI fixes, and a single doc-rename: `WGPS → confidential sync` in October 2025. No protocol work, no API work.

The crate is not abandoned (someone still merges dependabot PRs), but it is **stalled**. Number 0's main-line ecosystem (iroh, iroh-blobs, iroh-gossip, iroh-docs) has converged on the iroh 1.0-rc track that iroh-willow has not been ported to. If iroh-willow is going to ship as the canonical replacement for iroh-docs, it has not started that path yet.

## What Willow is

Willow is a P2P data model + sync protocol designed by Aljoscha Meyer and Sam Gwilym, published at [willowprotocol.org](https://willowprotocol.org/). It started as *"a minimalistic reimagining of Earthstar"* ([about](https://willowprotocol.org/more/about-us/index.html)) — Earthstar's successor in spirit, sharing the same author-signed-entries-with-paths model but with a tighter spec and stronger access control.

The data model has four layers ([data model spec](https://willowprotocol.org/specs/data-model/index.html)):

- **Namespace** — top-level data domain. *"Data from a public wiki should live in a separate namespace than data from a photo-sharing application."*
- **Subspace** — within a namespace, *"each user writes to their own, separate universe of data."* Subspaces are how per-user authorization is anchored.
- **Path** — hierarchical sequence of bytestring components, like a filesystem path. Bounded by `max_component_count` and `max_component_length` per namespace.
- **Payload** — arbitrary bytestring up to `2^64 - 1` bytes. The entry holds a digest, not the bytes.
- **Timestamp** — 64-bit, *"interpret as microseconds in International Atomic Time (TAI)."*

An entry is `(namespace_id, subspace_id, path, timestamp, payload_length, payload_digest)` plus an authorization signature. Newer-than-comparison is `(timestamp, payload_digest)`. **Prefix pruning** is the killer feature: a newer entry at path `/blog/idea` deletes all entries prefixed by it (`/blog/idea/1`, `/blog/idea/2`, etc.). True destructive editing — entries actually go away, no tombstones piling up.

## Meadowcap (the capability layer)

[Meadowcap](https://willowprotocol.org/specs/meadowcap/index.html) is Willow's access control. *"A capability is an unforgeable token that bestows read or write access for some data to a particular person, issued by the owner of that data."* Two namespace flavors:

- **Communal namespace** — each subspace is owned by whoever holds the keypair whose public key *is* the SubspaceId. No central authority.
- **Owned namespace** — one keypair owns the whole namespace and delegates restricted capabilities (path prefixes, time bounds) to others.

Writes require a Meadowcap capability *and* a signature over the entry. The capability chains back to either a subspace owner (communal) or the namespace owner (owned). This is meaningfully stronger than iroh-docs' single-NamespaceId-grants-everything model.

## 3d range-based set reconciliation

Willow's sync protocol generalizes Meyer's [RBSR](https://arxiv.org/abs/2212.13567) to three dimensions: namespace × subspace × path. Per the [sync spec](https://willowprotocol.org/specs/sync/index.html):

1. Peers declare *areas of interest* (rectangular regions of the 3D space).
2. They intersect their areas; for each intersection they exchange `ReconciliationSendFingerprint` messages.
3. Mismatches recursively split the 3D region; matches end the recursion.
4. At leaf granularity peers exchange `ReconciliationAnnounceEntries` and ship the actual payloads.

The 3D structure is what lets Willow scale: a peer can sync just `/photos/2024/` from one user without enumerating the whole namespace, because path is a coordinate that can be range-restricted independently of subspace.

## Confidential Sync

The README mentions [Confidential Sync](https://willowprotocol.org/specs/confidential-sync/index.html) — formerly called WGPS, the *Willow General Purpose Sync* protocol. This is the wire-level framing: how RBSR runs over a connection, plus the Pseudo-IDs and SubspaceCapabilities that prevent metadata leakage. The subspaces a peer is *interested in* shouldn't be inferable by an eavesdropper.

The Oct 2025 rename in iroh-willow's docs is the only acknowledgment of this evolution in the implementation; the rest of WGPS-vs-Confidential-Sync detail lives in willowprotocol.org spec drafts.

## Implications for Myrhiza

Willow is the most *spec-coherent* answer in the prior-art set to "what does P2P state look like." The data model maps directly to Myrhiza's needs:

- **Determinism is achievable.** Newer-than ordering is deterministic (`(timestamp, payload_digest)` lex-compare). A state-apply over Willow entries within a path range is a pure fold — the convergence guarantees of RBSR mean every peer sees the same final entry set, and the deterministic ordering means every peer folds them in the same order.
- **Capability semantics are first-class.** Meadowcap is closer to Spritely-style capability discipline than anything else in the iroh ecosystem. Owned namespaces give Myrhiza a clean "this app has an admin key" pattern; communal namespaces give it "every user owns their slice."
- **Prefix pruning is the right deletion model.** Better than tombstones, better than iroh-docs' silent-old-entry approach. Aligns with how Myrhiza apps will likely model "delete a thread / a profile / a workspace."
- **3D RBSR scales the way an app runtime needs.** Per-app subspaces, per-room paths, per-time-window ranges — all queryable as 3D regions, all syncable independently.

But: **iroh-willow is not the implementation to ship against today.** A Myrhiza spec that depends on Willow either commits to writing the implementation work iroh-willow is missing (port to iroh 1.0, fill in the WGPS gaps, ship a real release), or commits to using a different Willow implementation when one matures, or commits to writing one. Given the scope of work and Number 0's apparent prioritization order (iroh-docs got the recent attention, not iroh-willow), the realistic position for Myrhiza specs is:

- **Use Willow's data model and sync algorithm as the design target** for any Myrhiza state-sync substrate.
- **Don't depend on iroh-willow as the runtime** without a "we'll fork or rewrite" contingency.
- **Track [willowprotocol.org](https://willowprotocol.org/) directly** — the spec is moving faster than its Rust implementation.

If a future iroh release reactivates iroh-willow on the 1.0 line and ships Confidential Sync, that becomes the load-bearing dependency for Myrhiza state. Until then, treat Willow as the design north star, iroh-docs as the available-today fallback (with [its limitations](./docs.md)), and budget for the "we may end up writing this" path.

## Sources

- [iroh-willow repository](https://github.com/n0-computer/iroh-willow)
- [iroh-willow README on main](https://github.com/n0-computer/iroh-willow/blob/main/README.md)
- [iroh-willow Cargo.toml on main](https://github.com/n0-computer/iroh-willow/blob/main/Cargo.toml)
- [iroh-willow commit d68310a (Iroh 0.34.0 upgrade — last functional change)](https://github.com/n0-computer/iroh-willow/commit/d68310a)
- [Willow Protocol home](https://willowprotocol.org/)
- [Willow data model spec](https://willowprotocol.org/specs/data-model/index.html)
- [Meadowcap spec](https://willowprotocol.org/specs/meadowcap/index.html)
- [Willow sync spec (3d RBSR)](https://willowprotocol.org/specs/sync/index.html)
- [Confidential Sync spec](https://willowprotocol.org/specs/confidential-sync/index.html)
- [Willow about — relationship to Earthstar](https://willowprotocol.org/more/about-us/index.html)
- [Range-based set reconciliation paper (Meyer 2022)](https://arxiv.org/abs/2212.13567)
- [iroh-blobs — sibling doc](./blobs.md)
- [iroh-docs — sibling doc](./docs.md)
- [iroh-gossip — sibling doc](./gossip.md)
- [Spritely persistence — sibling prior-art doc](../spritely-ocapn/persistence.md)
