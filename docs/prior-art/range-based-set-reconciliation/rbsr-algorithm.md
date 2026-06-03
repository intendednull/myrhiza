**Date:** 2026-05-29
**Status:** active
**Subject:** RBSR algorithm — Aljoscha Meyer's recursive-fingerprint-over-a-range set reconciliation

# The RBSR algorithm

Range-based set reconciliation is, per Meyer's abstract verbatim:

> "a simple approach to efficiently computing the union of two sets over a
> network, based on recursively partitioning the sets and comparing fingerprints
> of the partitions to probabilistically detect whether a partition requires
> further work."

The paper's two contributions over prior art: (1) "a more generic description
and analysis in the broader context of set reconciliation" that "precisely
capturing the design space for fingerprinting schemes allows us to survey for
cryptographically secure schemes"; and (2) "we reduce the time complexity of
local computations by a logarithmic factor compared to previous publications"
(arXiv:2212.13567 abstract). The idea predates the paper (it appears in
Earthstar's `range-reconcile` and others); Meyer's contribution is the *naming,
generalization, and analysis*, not the invention.

## The core loop

Both peers keep their set in a **total order** (sort by item, e.g. by a 32-byte
record hash, or by `(timestamp, id)`). Reconciliation is divide-and-conquer over
*ranges of that order*, not over array indices — because, per Doug Hoyte's
explainer, "we cannot use the indices of the elements in our sorted list because
other protocol participants may have entirely different sets"
([logperiodic.com/rbsr.html](https://logperiodic.com/rbsr.html)).

1. Initiator computes a **fingerprint** (a hash) over *all* its items in some
   range `[lo, hi)` and sends `(lo, hi, fingerprint)`.
2. Responder computes the fingerprint of *its own* items in `[lo, hi)`.
   - **Match** ⇒ the range is reconciled. Stop recursing here.
   - **Mismatch** ⇒ split `[lo, hi)` into sub-ranges, fingerprint each, and
     send them back. (Split point is chosen so each side has roughly equal item
     counts in its own copy — splitting by *median item*, not by value
     midpoint.)
3. Recurse until ranges are small enough; then ship the actual items.

Two operating modes per range:
- **Fingerprint mode** — send a hash; cheap, probabilistic, recurses.
- **ID-list mode** — when a range holds few enough items, just enumerate the
  IDs (or items) so the other side can diff directly. Hoyte: "most RBSR
  implementations will want to stop splitting ranges once the number of elements
  they contain becomes sufficiently small."

## Complexity — why it beats O(n)

The message count is logarithmic in *set size* and independent of *difference
size*. From Hoyte's worked example: for 1,000,000 elements with branching factor
16, `log(1e6)/log(16) ≈ 4.98` messages, ≈ 2.49 (→ 3) round trips; for a billion
elements, ≈ 4 round trips. "The protocol's message count depends on dataset
size, not the number of differences"
([logperiodic.com/rbsr.html](https://logperiodic.com/rbsr.html)).

The total *data* transferred is dominated by the actual items in the difference
plus the `O(d log n)` fingerprints/range-descriptors needed to localize them
(`d` = symmetric difference, `n` = set size). Contrast: a naive "send all my IDs"
exchange is `O(n)`; a `HeadsSummary`-style per-author scan is `O(authors)` (see
[comparisons.md](comparisons.md)).

The number of round trips is the price: RBSR is **interactive and stateful per
session** — several back-and-forth messages, unlike a one-shot IBLT/minisketch
sketch ([comparisons.md](comparisons.md)).

## Fingerprints: the design space the paper formalizes

A fingerprint must be combinable over adjacent ranges so a node can compute any
range's fingerprint cheaply from a precomputed structure (an order-statistics
tree, an MST, a Prolly tree — see [structure-stability.md](structure-stability.md)).
The paper's framing: fingerprints live on a spectrum from cheap-but-attackable
to cryptographically secure.

- **Naive: XOR / sum of item hashes** — incrementally updatable but
  **forgeable**: an adversary can craft two different sets with the same
  fingerprint, causing peers to wrongly believe a range is reconciled and
  silently drop the difference. Meyer's paper surveys this hazard explicitly.
- **Algebraic hashing (e.g. over an elliptic curve / a large field)** — keeps
  the incremental-combine property while resisting forgery. This is the
  "cryptographically secure schemes" the paper surveys for.

**Negentropy's concrete construction** (the answer to "so which fingerprint?")
is specified verbatim in the [Negentropy Protocol V1 spec](https://github.com/hoytech/negentropy/blob/master/docs/negentropy-protocol-v1.md#fingerprint-algorithm).
The fingerprint of a range is computed by:

1. **Add the element IDs mod 2²⁵⁶** (each 32-byte ID interpreted as a little-endian
   unsigned integer) — this is the *incremental/algebraic* part: adding an ID to a
   range, or merging two adjacent ranges, is just modular addition, so any
   sub-range's running sum is derivable without re-touching the whole set.
2. **Concatenate the element count**, encoded as a varint.
3. **SHA-256** the result.
4. **Take the first 16 bytes** as the fingerprint.

The spec's rationale (per Hoyte): the bare mod-2²⁵⁶ sum *alone* is forgeable —
an attacker can craft IDs that sum identically — so the SHA-256 + count steps
turn the cheap algebraic accumulator into something an adversary cannot collide
without breaking SHA-256. That is the design-space point the paper formalizes,
made concrete: an incrementally-combinable accumulator wrapped in a cryptographic
hash, *not* a raw XOR/sum. This is the scheme to copy if Myrhiza ever adopts RBSR
(see [lessons.md](lessons.md) "Avoid: weak fingerprints").

## Suitability for an append-only signed event set

RBSR reconciles *sets of opaque IDs*. For Myrhiza, the natural item is the
32-byte `EventHash`; the total order is byte-lex on the hash (which the spec
*already uses* for topo-sort tie-break, `convergence.md` §4.1). Key fit notes:

- It does **not** need the items to be mutable key-value entries (unlike MST /
  Prolly, which are *maps*). A flat set of event hashes is exactly RBSR's
  natural input — this is why Negentropy, not a tree structure, is the closest
  exemplar.
- It is **orthogonal to authority**: RBSR tells you *which events you lack*, not
  whether they are *valid*. Validity (signature, `prev`/`deps` chain, `state-apply`
  verdict) is still checked on ingest. RBSR replaces the *discovery* half of
  sync, not the *verification* half.
- Per-author monotonicity makes a degenerate-but-cheap variant possible: because
  a Myrhiza author chain is a contiguous `seq`-ordered run, the `HeadsSummary`
  scan already gets `O(authors)` "for free" without fingerprints (see
  [`../willow/networking.md`](../willow/networking.md)). RBSR's win is only when
  *authors* itself grows large (wiki shape) — then you reconcile the *set of
  author heads* with RBSR. See [lessons.md](lessons.md).

## Sources

- [Range-Based Set Reconciliation (Meyer 2022/2023)](https://arxiv.org/abs/2212.13567)
- [RBSR PDF v2](https://arxiv.org/pdf/2212.13567)
- [RBSR on IEEE Xplore (SRDS 2023)](https://ieeexplore.ieee.org/document/10419244/)
- [Doug Hoyte — RBSR explainer (round-trip / complexity worked examples)](https://logperiodic.com/rbsr.html)
- [Negentropy README (incremental fingerprint)](https://github.com/hoytech/negentropy)
- [Negentropy Protocol V1 spec — Fingerprint Algorithm (add IDs mod 2²⁵⁶ + count varint → SHA-256 → first 16 bytes)](https://github.com/hoytech/negentropy/blob/master/docs/negentropy-protocol-v1.md#fingerprint-algorithm)
- [earthstar-project/range-reconcile (earlier RBSR impl)](https://github.com/earthstar-project/range-reconcile)
- Myrhiza spec: `convergence.md` §4.1 (lex-hash tie-break), §4.5 (scaling)
