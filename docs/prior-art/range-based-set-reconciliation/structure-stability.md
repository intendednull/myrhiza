**Date:** 2026-05-29
**Status:** active
**Subject:** Structure stability — history independence, the unbounded-node-size hazard, G-trees, and why Negentropy keeps no persistent tree

# Structure stability

The task asks to compare these systems on "structure stability." This is the
axis where the structural alternatives (MST, Prolly) and the flat-set protocol
(Negentropy) diverge most sharply — and where RBSR's *own author* found a real
flaw worth recording.

## History independence is the load-bearing property

For any content-addressed tree to converge across peers, its shape must be a
pure function of its current contents — **history independence**: "any data set
has a unique representation, regardless of the sequence of operations that led to
the current state" ([DoltHub](https://www.dolthub.com/blog/2024-03-03-prolly-trees/)).
Both MSTs and Prolly Trees achieve it (by layer-from-key-hash and by
content-defined chunking respectively, see
[merkle-search-trees.md](merkle-search-trees.md) /
[prolly-trees.md](prolly-trees.md)). Without it, two peers with identical
contents would build different trees → different root hashes → false "we
differ," and RBSR-over-the-tree breaks.

History independence is what makes a *standard* mutable B-tree unusable for this:
a B-tree's shape depends on insertion order and rebalancing history, so two peers
reaching the same key set could hold differently-shaped (differently-hashed)
trees. The randomized layer/chunk rule removes that dependence at the cost of
giving up deterministic node sizes — which is the next problem.

## The unbounded-node-size hazard (the real flaw)

The probabilistic balancing that gives MSTs/Prolly Trees history independence
also means **node sizes are random, with no hard upper bound**. Meyer himself
(RBSR's author) co-wrote the follow-on **Geometric Search Trees (G-trees)**
paper (Carson Farmer, Textile; Aljoscha Meyer, TU Berlin) precisely to fix this.
Verbatim from [g-trees.github.io](https://g-trees.github.io/g_trees/):

> "All prior work that does achieve sufficient simplicity buys it at the price of
> vertices that must store a dynamic, unbounded number of items."

And on MSTs / skip-trees specifically, the paper states verbatim: "**None of
these data structures can provide a** non-probabilistic upper bound on the number
of items per vertex. This hampers efficient implementation; and **adversarial
data suppliers can trivially produce n items in O(n) expected time that must all
be stored in the same vertex.**"

That last clause is the security teeth: in an **open network** — exactly
Myrhiza's threat model, where any permitted author submits events — an adversary
can craft keys whose hashes collide on the layer rule and **pile arbitrarily many
items into one node**, blowing up that node's size and the cost to fingerprint or
ship it. G-trees fix it with bounded `k`-lists (`O(k)` expected items/node), but
that is a *newer, less-deployed* structure. The takeaway for Myrhiza: **MST/Prolly
node-size is an adversarial-input surface in open networks**, and the canonical
fix is research-grade.

## RBSR's COW-snapshot tax

A second stability cost of persistent Merkle trees: to serve concurrent sync
sessions against a *changing* set, you need **copy-on-write snapshots** of the
tree, or a mutation invalidates in-flight sessions. Unlike those rigid
structures, Hoyte calls out, an RBSR implementation over a mutable source of
truth keeps no such snapshot: "RBSR can freely modify its single source of truth
without invalidating sync sessions started in the past"
([logperiodic.com/rbsr.html](https://logperiodic.com/rbsr.html)).

## Why Negentropy's "no persistent tree" is the stable choice

Negentropy sidesteps both hazards by **not maintaining a content-addressed tree
at all**. It keeps one mutable sorted source of truth and computes range
fingerprints on demand from a local order-statistics index (an implementation
detail, not a converged structure):

- **No unbounded-node adversarial surface** — there are no shared content-addressed
  nodes whose size an attacker can inflate; fingerprints are computed over
  whatever items fall in a range at query time.
- **No COW snapshot tax** — the set can mutate freely between/within sessions.
- **No history-independence encoding burden** — the wire protocol only needs the
  two sides to agree on a *total order*, which both derive deterministically from
  `(timestamp, id)`. The local index need not be byte-identical across peers.

The cost is recomputing `O(log n)` fingerprints per session, which a cached
order-statistics index makes cheap.

## Implications for Myrhiza

- Myrhiza's events are immutable and append-only, so the COW-snapshot tax is
  *mostly* moot (the set only grows) — but a per-author chain *forks* under
  equivocation (`convergence.md` §4.4.1), which a naive content-addressed tree
  would represent as two divergent shapes. A flat-set protocol handles this
  cleanly (both branches are just IDs in the set).
- The unbounded-node-size hazard is a direct argument **against** baking an MST
  or Prolly Tree into Myrhiza's open-network sync: it adds an adversarial
  amplification surface that the per-author-DAG + `HeadsSummary` model does not
  have. If a tree is ever needed, prefer a *bounded-node* design (G-tree-style)
  and treat node size as a security parameter.
- This is the single strongest structural reason the lessons file recommends
  **Negentropy's flat-set RBSR over the tree structures** if Myrhiza ever scales
  past §4.5.

## Sources

- [Geometric Search Trees (Farmer & Meyer) — unbounded-node-size flaw, adversarial O(n) node](https://g-trees.github.io/g_trees/)
- [Doug Hoyte — RBSR explainer (no COW snapshot needed)](https://logperiodic.com/rbsr.html)
- [DoltHub — Prolly Trees (history independence definition)](https://www.dolthub.com/blog/2024-03-03-prolly-trees/)
- [Merkle Search Trees (Auvolat & Taïani)](https://inria.hal.science/hal-02303490)
- Sibling: [merkle-search-trees.md](merkle-search-trees.md), [prolly-trees.md](prolly-trees.md), [negentropy.md](negentropy.md)
- Myrhiza spec: `convergence.md` §4.4.1 (equivocation forks), §4.5 (scaling)
