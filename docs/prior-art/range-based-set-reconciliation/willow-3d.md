**Date:** 2026-05-29
**Status:** active
**Subject:** Willow 3D-RBSR — Meyer & Gwilym's generalization of RBSR to a 3-dimensional product space

# Willow's 3D range-based set reconciliation

Willow is the P2P data model + sync protocol by **Aljoscha Meyer and Sam
Gwilym** (worm-blossom), at [willowprotocol.org](https://willowprotocol.org/).
Per Willow's own about page, their first collaboration "was a proof-of-concept
implementation of range based set reconciliation" — Willow is RBSR's author
applying it to a richer data model. This file covers the *3D-RBSR* sync
algorithm specifically; the broader Willow data model (Meadowcap, prefix
pruning, namespaces) is covered in [`../iroh/willow.md`](../iroh/willow.md).

> **Name collision warning.** There are *two unrelated "Willow"s* in this
> corpus. This file (and [`../iroh/willow.md`](../iroh/willow.md)) is about
> **Meyer & Gwilym's `willowprotocol.org` protocol**. The
> [`../willow/`](../willow/) sibling folder documents a *fork of Myrhiza's
> ancestor codebase that is named "Willow"* — a different artifact entirely.
> They share no code. Cross-links below are deliberate per target.

## From 1D to 3D

1D RBSR (Negentropy, iroh-docs) reconciles a set under one total order. Willow's
data is addressed by a **3-tuple coordinate**: `namespace × subspace × path`
(plus a timestamp for newer-than ordering). A peer wants to sync, say, just
`/photos/2024/` from *one user* within *one namespace* — a rectangular region of
the 3D space — without enumerating the rest.

Willow's [3D-RBSR spec](https://willowprotocol.org/specs/3d-range-based-set-reconciliation/index.html)
generalizes the recursion to operate on **3D ranges (rectangular regions)**
instead of 1D intervals. The flow (per
[`../iroh/willow.md`](../iroh/willow.md) and the sync spec):

1. Peers declare **areas of interest** — rectangular regions of the 3D space
   they care about.
2. They intersect their areas; for each intersection they exchange
   `ReconciliationSendFingerprint` messages over a 3D sub-region.
3. A fingerprint mismatch **recursively splits the 3D region** (along whichever
   dimension best bisects the item count); a match ends that branch's recursion.
4. At leaf granularity, peers exchange `ReconciliationAnnounceEntries` and ship
   the actual payloads.

The 3D structure is what lets Willow sync a path-prefix from one subspace
without touching the rest of the namespace — "path is a coordinate that can be
range-restricted independently of subspace"
([`../iroh/willow.md`](../iroh/willow.md)).

## Why 3D matters (and why Myrhiza probably doesn't need it)

3D-RBSR's win is **partial replication with structure**: sync a *slice* of a
large shared store cheaply. That is exactly the capability Myrhiza's
`convergence.md` §4.5 *defers* ("every peer holds everything" in v1) and the
partial-replication warning the gap-analysis flags as a v2+ concern.

For Myrhiza v1 the dimensions don't map cleanly:
- Myrhiza's unit is a **topic** (`convergence.md` §4.6), and within a topic the
  set is a flat per-author event DAG, not a 3D key space. The "namespace" axis is
  roughly the topic; "subspace" is roughly the author; but there is no native
  "path" axis — events are opaque payloads, not path-addressed entries.
- Willow's prefix-pruning destructive-edit model conflicts with Myrhiza's
  **append-only signed log** (events never disappear; deletion is an app-level
  derived-state concern). Adopting 3D-RBSR would mean adopting a different data
  model, not just a different sync protocol.

So Willow is the **design north star for partial replication at scale**, not a
v1 sync swap. If a Myrhiza app ever needs "sync only thread-1234 from author X,"
3D-RBSR is the precedent — but that lands well past the §4.5 ceiling.

## Implementation status caveat

The Rust implementation `iroh-willow` is **stalled** (no substantial code change
in over a year as of 2026-05; stuck on iroh 0.34 while iroh shipped 1.0-rc) — see
[`../iroh/willow.md`](../iroh/willow.md) for the verified status. The *spec*
moves faster than its code. Treat Willow's 3D-RBSR as a *spec to read*, not a
*dependency to add*.

## Sources

- [Willow Protocol home](https://willowprotocol.org/)
- [Willow 3D-RBSR spec](https://willowprotocol.org/specs/3d-range-based-set-reconciliation/index.html)
- [Willow sync spec](https://willowprotocol.org/specs/sync/index.html)
- [Willow — about us (Meyer & Gwilym, worm-blossom)](https://willowprotocol.org/more/about-us/index.html)
- [Aljoscha Meyer homepage](https://aljoscha-meyer.de/)
- Sibling prior-art: [`../iroh/willow.md`](../iroh/willow.md) (iroh-willow status + Meadowcap + data model), [`../willow/`](../willow/) (the *unrelated* Myrhiza-ancestor fork)
- Myrhiza spec: `convergence.md` §4.5–4.6, `networking.md` §11.3
