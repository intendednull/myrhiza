**Date:** 2026-05-29
**Status:** active
**Subject:** Range-based set reconciliation (RBSR / Negentropy / Merkle Search Trees / Prolly Trees) — efficient set-difference sync over an ordered set, and the content-addressed trees that back it

# Range-based set reconciliation

This folder surveys the family of protocols and data structures that let two
peers compute the *difference* between their sets in roughly `O(d log n)`
communication instead of `O(n)` — without a trusted middlebox, without
enumerating either side's full set, and without a global index. The lineage:

- **Range-Based Set Reconciliation (RBSR)** — Aljoscha Meyer's 2023 paper
  ([arXiv:2212.13567](https://arxiv.org/abs/2212.13567), SRDS 2023) that names
  and generalizes the recursive-fingerprint-over-a-range technique. The
  foundational idea. → [rbsr-algorithm.md](rbsr-algorithm.md)
- **Negentropy / Nostr NIP-77** — Doug Hoyte's clean, deployed *1D* embodiment,
  shipped in the `strfry` relay and `rust-nostr`. **The recommended deployed
  exemplar.** → [negentropy.md](negentropy.md)
- **Willow 3D-RBSR** — Meyer & Gwilym's generalization of RBSR to a
  3-dimensional product space (namespace × subspace × path). Distinct from the
  ancestor [`../willow/`](../willow/) folder, which documents a *fork named
  Willow*, not Meyer's protocol. → [willow-3d.md](willow-3d.md)
- **Merkle Search Trees (MST)** — Auvolat & Taïani's 2019 content-addressed,
  history-independent search tree; the structure behind atproto repositories.
  → [merkle-search-trees.md](merkle-search-trees.md)
- **Prolly Trees** — probabilistic B-trees (Noms → Dolt); the same
  history-independent content-addressed B-tree by content-defined chunking.
  → [prolly-trees.md](prolly-trees.md)
- **iroh-docs** — iroh's own RBSR implementation (the transport Myrhiza already
  depends on). → [iroh-docs.md](iroh-docs.md)

RBSR is the *protocol*; MSTs and Prolly Trees are *structures* that make a peer's
fingerprint-over-a-range query cheap. Negentropy deliberately uses neither (it
keeps one mutable sorted source of truth) — that tension is the core lesson.
**IBLT / minisketch** are the rejected runner-up paradigm (see
[comparisons.md](comparisons.md)).

## Why this folder exists for Myrhiza

Myrhiza's master spec defers two decisions straight into this corpus:

- **`networking.md` §11.3** lists "negentropy-shape range reconciliation for
  very large topics" as deferred future work, by name.
- **`convergence.md` §4.2** defines the v1 `HeadsSummary` per-author delta
  protocol; **§4.5** ("Future direction: scaling") explicitly acknowledges
  event-log replay's linear scaling as the named-but-deferred problem. RBSR is the
  documented replacement at wiki scale. (The `O(authors)` / `O(d log n)` notation
  throughout this folder is *this corpus's own analysis* — the spec itself uses no
  big-O; it states the per-author scan and the wiki-scale ceiling in prose and
  back-of-envelope numbers.)

Honest scope: **Myrhiza v1 commits to "every peer holds everything"**
(`convergence.md` §4.5). v1 sync is full-event-log replay from genesis via
`HeadsSummary` delta exchange. This folder is a **scaling-path reference, not a
v1 need.** It becomes load-bearing only when a real app hits the §4.5 ceiling
(~the wiki shape: 1000 contributors, ~1.8 GB/year). See
[lessons.md](lessons.md).

## Key facts at a glance

| Item | Value | Source |
|---|---|---|
| RBSR paper | Meyer, *Range-Based Set Reconciliation* | [arXiv:2212.13567](https://arxiv.org/abs/2212.13567) |
| RBSR dates | v1 2022-12-27, v2 2023-02-07 | arXiv abstract page |
| RBSR venue | SRDS 2023, pp. 59–69, [IEEE Xplore 10419244](https://ieeexplore.ieee.org/document/10419244/) | dblp / IEEE |
| RBSR contribution | generic framework + `−1 log factor` on local compute vs prior work | arXiv abstract |
| Negentropy author / license | Doug Hoyte (hoytech); MIT | [github.com/hoytech/negentropy](https://github.com/hoytech/negentropy) |
| Negentropy idSize | 32 bytes (256-bit, typically a record hash) | negentropy README; NIP-77 |
| Negentropy fingerprint | add IDs mod 2²⁵⁶ + count varint → SHA-256 → first 16 bytes | [Protocol V1 spec §Fingerprint Algorithm](https://github.com/hoytech/negentropy/blob/master/docs/negentropy-protocol-v1.md#fingerprint-algorithm) |
| NIP-77 status | `draft` `optional` `relay`; protocol version byte `0x61` | [nips.nostr.com/77](https://nips.nostr.com/77) |
| `negentropy` crate (rust-nostr) | `0.5.0`, MIT, 804K downloads | crates.io (verified 2026-05-29) |
| strfry / NIP-77 scale | "routinely used to synchronise data-sets … in the 10s of millions of elements" | [logperiodic.com/rbsr.html](https://logperiodic.com/rbsr.html) |
| MST paper | Auvolat & Taïani, *Merkle Search Trees: Efficient State-Based CRDTs in Open Networks*, SRDS 2019 | [hal-02303490](https://inria.hal.science/hal-02303490), DOI 10.1109/SRDS.2019.00032 |
| MST in atproto | SHA-256 leading-zero-pairs → layer; fanout ~4; deterministic by content | [atproto.com/specs/repository](https://atproto.com/specs/repository) |
| `merkle-search-tree` crate (domodwyer) | `0.8.0`, Apache-2.0, 181K downloads | crates.io (verified 2026-05-29) |
| Prolly Trees origin | term coined by Noms (Attic Labs, 2015); earlier unnamed use in `bup` (2009) | [DoltHub](https://www.dolthub.com/blog/2025-06-03-people-keep-inventing-prolly-trees/) |
| iroh-docs sync | RBSR, "based on this paper by Aljoscha Meyer" | iroh-docs README; crate `0.100.0` |

## Canonical reading order

1. [rbsr-algorithm.md](rbsr-algorithm.md) — what RBSR *is* and why it beats `O(n)`.
2. [negentropy.md](negentropy.md) — the cleanest deployed 1D embodiment.
3. [comparisons.md](comparisons.md) — `O(d log n)` vs `O(n)`; the IBLT/minisketch runner-up.
4. [merkle-search-trees.md](merkle-search-trees.md) + [prolly-trees.md](prolly-trees.md) — the structural alternatives.
5. [structure-stability.md](structure-stability.md) — the unbounded-node-size hazard; why Negentropy uses *no* persistent tree.
6. [willow-3d.md](willow-3d.md) + [iroh-docs.md](iroh-docs.md) — the multi-dimensional and iroh-native variants.
7. [open-problems.md](open-problems.md) — what this family structurally does NOT solve.
8. [lessons.md](lessons.md) — **the decision file** for Myrhiza.

## Glossary stub

Full terms in [glossary.md](glossary.md). The load-bearing few:

- **RBSR** — Range-Based Set Reconciliation. Recursive fingerprint-over-a-range diff.
- **Fingerprint** — a (usually incremental, algebraic) hash of all items in a range. Mismatch ⇒ split the range.
- **`d`** — the *symmetric set difference* (items one side has and the other lacks). RBSR cost scales with `d log n`, not `n`.
- **History independence** — the property that a structure's shape depends only on its current contents, never on insertion order. Required for MST/Prolly content-addressing to converge.
- **Content-defined chunking (CDC)** — a rolling-hash boundary rule that decides node/chunk splits by content, giving structural sharing. Prolly Trees' mechanism.

## How to use / Framing disclosure

These docs are **not a neutral catalog of set-reconciliation research.** They are
written from Myrhiza's current design stance and read every system through that
lens. That stance, made explicit:

- **Capability-mediated** — apps reach the host only through declared imports;
  sync is a kernel-internal concern, not an app-facing surface.
- **P2P-only** — no trusted middlebox; reconciliation must work peer-to-peer.
- **Component-Model-on-Wasmtime** — the runtime target, which is why
  [open-problems.md](open-problems.md) flags that none of these libraries ship a
  WASM Component-Model artifact.
- **Event-log-replay `state-apply`** — state is a deterministic fold over a
  per-author signed Merkle event DAG, not a mutable key-value map.

The "Implications for Myrhiza" notes therefore ask one question — "would this
replace the `HeadsSummary` scan at scale?" — which biases the whole folder
**toward Negentropy** (a set-diff over opaque IDs that fits a signed append-only
event set cleanly) and **away from MST/Prolly** (map structures optimized for
mutable key-value state, a poorer fit for an append-only log). A reader auditing
whether Myrhiza needs *any* of this in v1 should weigh
[open-problems.md](open-problems.md) and the §4.5 "every peer holds everything"
commitment first.

**Incentive caveat (read skeptically).** RBSR is the *documented* replacement for
two deferred Myrhiza surfaces (§11.3, §4.5) and the closest exemplar, Negentropy,
is a near-locked recommendation — so this corpus has a standing incentive to
**soft-pedal the problems Myrhiza would inherit** by adopting it: the interactive
multi-round-trip cost, the sort-key metadata leak, the truncation/GC loop, and
the fact that a "no persistent tree" claim still means *some* local
order-statistics index to keep crash-consistent. Those are surfaced in
[open-problems.md](open-problems.md) and [lessons.md](lessons.md) "Avoid" —
treat that list as the load-bearing counterweight to the favorable framing here,
not an afterthought.

Cross-corpus: [`../iroh/willow.md`](../iroh/willow.md) and
[`../willow/networking.md`](../willow/networking.md) already cite RBSR; this
folder is the dedicated treatment those gesture at. The truncation/GC tie-in to
[`../eg-walker/open-problems.md`](../eg-walker/open-problems.md) is developed in
[open-problems.md](open-problems.md) and [lessons.md](lessons.md).

## Sources

- [Range-Based Set Reconciliation (Meyer 2022/2023)](https://arxiv.org/abs/2212.13567)
- [RBSR on IEEE Xplore (SRDS 2023, doc 10419244)](https://ieeexplore.ieee.org/document/10419244/)
- [Negentropy (Doug Hoyte / hoytech)](https://github.com/hoytech/negentropy)
- [NIP-77 Negentropy Syncing](https://nips.nostr.com/77)
- [Doug Hoyte — Range-Based Set Reconciliation explainer](https://logperiodic.com/rbsr.html)
- [Merkle Search Trees (Auvolat & Taïani, SRDS 2019)](https://inria.hal.science/hal-02303490)
- [atproto repository spec (MST)](https://atproto.com/specs/repository)
- [DoltHub — People Keep Inventing Prolly Trees](https://www.dolthub.com/blog/2025-06-03-people-keep-inventing-prolly-trees/)
- [iroh-docs](https://github.com/n0-computer/iroh-docs)
- crates.io (`negentropy`, `merkle-search-tree`) — verified 2026-05-29
- Sibling prior-art: [`../iroh/willow.md`](../iroh/willow.md), [`../willow/networking.md`](../willow/networking.md), [`../crdts/`](../crdts/), [`../eg-walker/`](../eg-walker/), [`../at-protocol/`](../at-protocol/)
