**Date:** 2026-05-29
**Status:** active
**Subject:** Merkle Search Trees — Auvolat & Taïani's history-independent content-addressed search tree; the structure behind atproto repos

# Merkle Search Trees (MST)

The MST paper is *Merkle Search Trees: Efficient State-Based CRDTs in Open
Networks* by **Alex Auvolat and François Taïani**, SRDS 2019 (38th IEEE
International Symposium on Reliable Distributed Systems, Lyon, France),
[hal-02303490](https://inria.hal.science/hal-02303490), DOI
10.1109/SRDS.2019.00032.

An MST is a **content-addressed, history-independent, balanced search tree**.
Its thesis (paper abstract, per authors' summary): "pure state-based CRDTs can
be efficiently implemented by encoding states as specialized Merkle trees," and
the approach "is well suited to open networks where many nodes may join and
leave." Where RBSR is a *protocol*, the MST is a *structure* that makes a peer's
range-fingerprint queries trivial — every internal node already *is* a Merkle
hash over its subtree.

## How the structure works

The MST stores a sorted key→value map. The trick is that **a key's layer in the
tree is determined by hashing the key**, not by insertion order:

- Hash each key; count the number of leading zero *bits* in some base-`B`
  representation; that count is the key's layer/level.
- Because the hash is (assumed) uniformly distributed, ~`1/B` of keys land one
  level up, ~`1/B²` two levels up, etc. — giving a probabilistically balanced
  tree with branching factor `B` and `O(log_B n)` depth, **with no rotations or
  rebalancing**.

The payoff is **history independence**: the tree's shape is a pure function of
its current key set, regardless of the insert/delete sequence that produced it.
Two peers with the same key→value contents build the *byte-identical* tree, so
their root hashes match iff their contents match — and a fingerprint mismatch
recurses down exactly the divergent subtrees (RBSR over the tree's structure).

## atproto's instantiation

[atproto](https://atproto.com/specs/repository) (Bluesky) uses an MST as the
authenticated data structure for each user repository:

- **Layer rule (verbatim behavior):** "hash the key (a byte array) with SHA-256,
  with binary output," then "count the number of leading binary zeros in the
  hash, and divide by two, rounding down." Dividing the leading-zero *bit* count
  by 2 means each 2-bit group of leading zeros bumps the layer — i.e. a
  **fanout of ~4**.
- **Determinism (verbatim):** "the overall structure and shape of the MST is
  deterministic based on the current key/value content, regardless of the
  history of insertions and deletions that lead to the current contents."
- Keys are repo paths (`collection/rkey`); "repo paths for all records in the
  same collection are sorted together in the MST, making enumeration (via key
  scan) and export efficient." Nodes are CBOR-encoded and content-addressed by
  CID; the repo's signed commit pins the MST root.

atproto syncs repos largely by **shipping the diff of MST blocks** (CAR files of
changed nodes since a `since` revision), which is RBSR-shaped at the structural
level: identical subtrees share hashes and are skipped.

## Implementations (verified)

- **`merkle-search-tree` (domodwyer)** crate: `0.8.0`, Apache-2.0, ~181K
  downloads (crates.io, verified 2026-05-29). Repo
  `domodwyer/merkle-search-tree`, framed as "Efficient state-based CRDT
  replication and anti-entropy" — this is the *paper's* CRDT use case, **not**
  atproto-compatible.
- **`DavidBuchanan314/merkle-search-tree`** — a separate implementation
  explicitly "structurally compatible with ATProto's instantiation."
- Multiple atproto-MST ports exist (Python, Rust, TypeScript, `hdevalence/mst`).

The split matters: the *paper's MST* and *atproto's MST* are the same idea with
different parameters and encodings, and the two crate families are not
interchangeable. See [`../at-protocol/`](../at-protocol/) for the atproto repo
model in full.

## Implications for Myrhiza

An MST is a **map** structure (sorted key→value). Myrhiza's per-author event DAG
is an **append-only log** — a poorer fit:

- MSTs shine for *mutable, randomly-keyed* state where you want both
  authenticated reads *and* anti-entropy. Myrhiza's events are immutable and
  per-author-sequential; there is no mutable key space to authenticate.
- An MST *could* index the *materialized derived state* (a key→value map) to
  give cheap state-diffing — but Myrhiza's convergence proof already uses
  `state-digest()` over canonical bytes (`convergence.md` §4.3), which is
  simpler and doesn't impose a tree on every app's state shape.
- The one transferable idea: **layer-by-hash gives history independence without
  rebalancing** — useful if Myrhiza ever needs an authenticated index over a
  *set of author heads* at wiki scale. But see
  [structure-stability.md](structure-stability.md) for the unbounded-node-size
  hazard that makes MSTs adversarially fragile in open networks.

## Sources

- [Merkle Search Trees (Auvolat & Taïani, SRDS 2019)](https://inria.hal.science/hal-02303490)
- [MST paper landing (Taïani)](https://ftaiani.ouvaton.org/PUBLI/PEER_REV_CONF/2019_auvolat_hal-02303490.html)
- [atproto repository spec (MST layer rule, determinism)](https://atproto.com/specs/repository)
- [`merkle-search-tree` crate (domodwyer)](https://crates.io/crates/merkle-search-tree) — `0.8.0`, Apache-2.0, verified 2026-05-29
- [DavidBuchanan314/merkle-search-tree (atproto-compatible)](https://github.com/DavidBuchanan314/merkle-search-tree)
- Sibling prior-art: [`../at-protocol/`](../at-protocol/), [`structure-stability.md`](structure-stability.md), [`prolly-trees.md`](prolly-trees.md)
- Myrhiza spec: `convergence.md` §4.3 (`state-digest`), §4.5 (scaling)
