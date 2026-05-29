**Date:** 2026-05-29
**Status:** active
**Subject:** Glossary — terms specific to range-based set reconciliation and its backing structures

# Glossary

- **RBSR (Range-Based Set Reconciliation)** — Aljoscha Meyer's protocol family:
  reconcile two sets by recursively comparing fingerprints over ranges of a
  shared total order, splitting on mismatch. ([rbsr-algorithm.md](rbsr-algorithm.md))
- **Set reconciliation** — computing the union (and per-side difference) of two
  sets held by different parties, ideally transferring only the difference plus
  some overhead.
- **`d` (symmetric difference)** — the items one side holds that the other lacks,
  summed both ways. RBSR communication scales `O(d log n)`; sketches scale `O(d)`.
- **`n` (set size)** — total items in a set. RBSR *message count* scales
  `O(log n)`, independent of `d`.
- **Fingerprint** — a hash summarizing all items in a range. Equality ⇒ the range
  is reconciled; inequality ⇒ split and recurse. ([rbsr-algorithm.md](rbsr-algorithm.md))
- **Incremental fingerprint** — a fingerprint that can be combined over adjacent
  ranges (and updated as items are added) without re-hashing the whole set;
  Negentropy's choice.
- **Cryptographically secure fingerprint** — a fingerprint scheme for which an
  adversary cannot forge a collision; required to prevent silent
  drop-the-difference attacks. Contrast XOR/sum (forgeable).
- **Fingerprint mode / ID-list mode** — RBSR's two per-range behaviors: send a
  hash (recurse on mismatch) vs. enumerate the items directly (when the range is
  small enough). ([rbsr-algorithm.md](rbsr-algorithm.md))
- **Negentropy** — Doug Hoyte's MIT-licensed RBSR implementation; the 1D
  exemplar. ([negentropy.md](negentropy.md))
- **NIP-77** — Nostr Implementation Possibility 77, "Negentropy Syncing": a
  hex-encoding wrapper exposing Negentropy over Nostr relay messages
  (`NEG-OPEN` / `NEG-MSG` / `NEG-CLOSE` / `NEG-ERR`). Status `draft`/`optional`/`relay`.
- **idSize** — the fixed ID length Negentropy reconciles; 32 bytes (a 256-bit
  record hash).
- **`have` / `need`** — Negentropy's output arrays: IDs the local side holds that
  the remote lacks (`have`) and vice versa (`need`).
- **`frameSizeLimit`** — optional cap on Negentropy message size; trades more
  round trips for smaller frames.
- **3D-RBSR** — Willow's generalization of RBSR to a 3-dimensional product space
  (namespace × subspace × path), reconciling rectangular regions.
  ([willow-3d.md](willow-3d.md))
- **Area of interest** — in Willow, a rectangular region of the 3D space a peer
  wants to sync; peers intersect areas before reconciling.
- **MST (Merkle Search Tree)** — Auvolat & Taïani's content-addressed,
  history-independent search tree; a key's tree layer is set by leading zeros of
  its hash. Backs atproto repos. ([merkle-search-trees.md](merkle-search-trees.md))
- **Prolly Tree (probabilistic B-tree)** — a content-addressed history-independent
  B-tree whose node boundaries come from content-defined chunking. Coined by
  Noms; used by Dolt. ([prolly-trees.md](prolly-trees.md))
- **Content-defined chunking (CDC)** — a rolling-hash rule that places chunk/node
  boundaries based on content, giving structural sharing. Prolly Trees' mechanism.
- **History independence** — the property that a structure's shape depends only on
  its current contents, never on operation order. Required for content-addressed
  trees to converge across peers. ([structure-stability.md](structure-stability.md))
- **Structural sharing** — unchanged content-addressed subtrees keep their address
  and are stored/transferred once across versions or replicas.
- **G-tree (Geometric Search Tree)** — Farmer & Meyer's bounded-node-size
  history-independent tree family, fixing MST/skip-tree's unbounded-node hazard.
  ([structure-stability.md](structure-stability.md))
- **IBLT (Invertible Bloom Lookup Table)** — a fixed-size sketch from which the
  set difference can be "peeled"; bandwidth scales with `d` but must be pre-sized.
  Rejected runner-up. ([comparisons.md](comparisons.md))
- **minisketch** — Bitcoin Core's BCH-code set-difference sketch (the engine
  behind Erlay); near-optimal bandwidth for small `d`, CPU grows steeply.
  Rejected runner-up. ([comparisons.md](comparisons.md))
- **`HeadsSummary`** — Myrhiza's v1 sync protocol: a per-author DAG-tip vector
  exchanged to localize the difference in `O(authors)` without fingerprints
  (`convergence.md` §4.2). What RBSR would replace at wiki scale.
- **EOSE / `HistorySyncComplete`** — "end of stored events" / backfill-complete
  signal; tells a joiner when reconciliation/backfill has converged
  (`networking.md` §11.3; Nostr NIP-01 / Willow precedent).

## Sources

- All terms are defined in the per-subsystem files cross-referenced inline; see
  those files' `## Sources` for primary references.
- [Range-Based Set Reconciliation (Meyer)](https://arxiv.org/abs/2212.13567)
- [Negentropy](https://github.com/hoytech/negentropy) / [NIP-77](https://nips.nostr.com/77)
- [Merkle Search Trees (Auvolat & Taïani)](https://inria.hal.science/hal-02303490)
- [DoltHub — Prolly Trees](https://www.dolthub.com/blog/2024-03-03-prolly-trees/)
- [Geometric Search Trees](https://g-trees.github.io/g_trees/)
- Myrhiza spec: `convergence.md` §4.2, `networking.md` §11.3
