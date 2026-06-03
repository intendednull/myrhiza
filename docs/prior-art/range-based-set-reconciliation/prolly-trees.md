**Date:** 2026-05-29
**Status:** active
**Subject:** Prolly Trees — probabilistic B-trees (Noms → Dolt); content-defined chunking for history-independent content-addressed B-trees

# Prolly Trees

"Prolly Tree" is short for **probabilistic B-tree**. The term was coined by
**Noms** (Attic Labs, 2015); per DoltHub's history, an earlier *unnamed* use
appears in **`bup` (2009)**, which "split large files up into a tree of Git
objects by recursively applying a rolling hash chunker" but "didn't give a name
to the technique"
([DoltHub](https://www.dolthub.com/blog/2025-06-03-people-keep-inventing-prolly-trees/)).
Today the canonical production user is **Dolt** (DoltHub's "Git for data" SQL
database), where "almost everything … is a Prolly Tree: tables, schemas,
indexes."

A Prolly Tree is, like an MST, a **content-addressed, history-independent,
balanced search tree** — "nodes in a Prolly Tree are referenced by their
content-address rather than a file-pointer"
([DoltHub](https://www.dolthub.com/blog/2024-03-03-prolly-trees/)). The DoltHub
docs name three properties: **searchable** (ordered lookups and range scans),
**history independence** ("any data set has a unique representation, regardless
of the sequence of operations that led to the current state"), and
**self-balancing** ("probabilistically balanced").

## The structural difference from MSTs: where the dice are rolled

MSTs and Prolly Trees solve the *same* problem (history-independent
content-addressed search tree) by rolling the randomness in *different places*:

- **MST** hashes each **key** and uses the leading-zero count to fix that key's
  *layer*. Boundaries are a property of keys.
- **Prolly Tree** uses **content-defined chunking (CDC)**: it scans the
  in-order sequence with a rolling hash and declares a chunk boundary whenever
  the rolling hash hits a target pattern. Boundaries are a property of the
  *byte stream*, so they survive insertions/deletions elsewhere in the tree —
  giving **structural sharing** (unchanged chunks keep their content address and
  are stored once).

DoltHub tunes chunks to **~4 KB average**, "which means a probability of 1/4096
or 0.02% of triggering a chunk boundary shift when changing a single byte"
([DoltHub chunker post](https://www.dolthub.com/blog/2022-06-27-prolly-chunker/)).
Larger chunks ⇒ fewer nodes/less metadata but coarser diffs; smaller chunks ⇒
finer structural sharing but more overhead. This is the central Prolly tuning
knob and has no MST analogue.

## They are the same idea, reinvented repeatedly

DoltHub's "People Keep Inventing Prolly Trees" post is the honest reference here.
It explicitly treats **MSTs as functionally equivalent**: the Inria MST design
"has all the same properties as prolly trees, although they call their design
'Merkle Search Trees' instead." The distinction it draws is only the boundary
mechanism — MSTs work by "hashing the data only once and using the number of
leading zeros in the hash of each key to determine how many levels of the tree
will split off a new chunk." The post's honest caveat: "It's fully possible that
there are even more people who have independently discovered this data structure,
and are using it in their own work with little fanfare." Treat "Prolly Tree" and
"Merkle Search Tree" as siblings of one family, not rivals.

## Where Prolly Trees win — and the RBSR connection

Because identical subtrees share content addresses, two replicas diff by walking
from the roots and skipping any subtree whose hash matches — the structural
embodiment of RBSR's fingerprint-mismatch recursion. Dolt leans on this for
cheap branch/merge/diff over "millions of versions, branches, and rows"
([DoltHub scaling post](https://www.dolthub.com/blog/2025-05-16-millions-of-versions/)).
The CDC structural sharing also means a single-row change rewrites only the
~`O(log n)` chunks on its path, not the whole tree — the property that makes
copy-on-write versioning affordable.

## Implications for Myrhiza

Same verdict as MSTs ([merkle-search-trees.md](merkle-search-trees.md)): a
Prolly Tree is a **map/B-tree** optimized for *mutable, versioned key-value
state*, which is not the shape of Myrhiza's append-only per-author event log.

- It is a strong structure for the **on-disk store** if Myrhiza ever wants
  branch/diff/merge over *materialized state* — but that overlaps with the
  separate `embedded-storage-engines` / `content-addressed-blockstore` gap-
  analysis candidates, not this folder.
- The CDC tuning knob (chunk size ↔ diff granularity ↔ metadata overhead) is the
  kind of decision Myrhiza would inherit if it adopted any content-addressed
  tree — worth knowing it exists before reaching for one.
- For *discovery* sync (the §11.3 deferral), a flat-set protocol (Negentropy)
  beats a tree: no second on-disk structure to keep crash-consistent, no chunk
  tuning, no history-independence encoding burden. See [lessons.md](lessons.md).

## Sources

- [DoltHub — People Keep Inventing Prolly Trees (origin: Noms 2015; bup 2009; MST equivalence)](https://www.dolthub.com/blog/2025-06-03-people-keep-inventing-prolly-trees/)
- [DoltHub — Prolly Trees (content-address vs file-pointer; three properties)](https://www.dolthub.com/blog/2024-03-03-prolly-trees/)
- [DoltHub — How to Chunk Your Database into a Merkle Tree (4 KB avg, 1/4096)](https://www.dolthub.com/blog/2022-06-27-prolly-chunker/)
- [DoltHub — How Dolt Scales to Millions of Versions](https://www.dolthub.com/blog/2025-05-16-millions-of-versions/)
- [Dolt Prolly Tree architecture docs](https://docs.dolthub.com/architecture/storage-engine/prolly-tree)
- [attic-labs/noms (origin of the term)](https://github.com/attic-labs/noms)
- Sibling prior-art: [`merkle-search-trees.md`](merkle-search-trees.md), [`structure-stability.md`](structure-stability.md), [`../crdts/`](../crdts/)
