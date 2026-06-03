**Date:** 2026-05-29
**Status:** active
**Subject:** Comparisons — O(d log n) RBSR vs O(n) naive vs O(authors) HeadsSummary; and the rejected IBLT/minisketch runner-up

# Comparisons

This file places RBSR against the alternatives Myrhiza actually weighs: the
naive full-set exchange, the v1 `HeadsSummary` per-author scan, and the
**set-difference sketches (IBLT / minisketch)** that the master spec already
declines.

## Cost models

`n` = total set size, `d` = symmetric difference (items one side lacks),
`a` = number of authors.

| Method | Communication | Round trips | State per session | Notes |
|---|---|---|---|---|
| Naive "send all IDs" | `O(n)` | 1 | none | Baseline. Cheap to implement, scales badly. |
| `HeadsSummary` per-author scan (Myrhiza v1) | `O(a)` summary + `O(d)` events | 1–few | none | Exploits per-author contiguity; no fingerprints. Adequate when `a` small. |
| **RBSR** (Negentropy, iroh-docs, Willow) | `O(d log n)` | `O(log n)` | a sorted index for range fingerprints | Independent of `d` in *message count*; interactive. |
| IBLT | `~O(d)` (constant-factor overhead) | 1 (sized correctly) | none | One-shot; must guess `d` to size the sketch. |
| minisketch (BCH) | `~O(d)` (near-optimal) | 1 | none | Best bandwidth for small `d`; CPU grows steeply with set/difference size. |

The key axis: **RBSR's message count scales with `log(set size)`, not with the
difference** ([logperiodic.com/rbsr.html](https://logperiodic.com/rbsr.html));
sketches scale with the *difference*, not the set. They win in opposite regimes.

## RBSR vs O(n) and vs HeadsSummary

- vs **O(n) naive**: RBSR's whole reason to exist. When two peers are *mostly*
  in sync (`d ≪ n`), naive ID exchange wastes `O(n)` bandwidth confirming items
  both already have; RBSR confirms agreement in `O(log n)` fingerprints and
  spends bandwidth only on the actual difference.
- vs **HeadsSummary (v1)**: `convergence.md` §4.2's scan is `O(a)` because a
  per-author chain is contiguous — one `(pubkey, seq, hash)` tuple summarizes an
  entire author's chain, no fingerprinting needed. **RBSR only beats it when `a`
  is large** (the wiki shape: ~1000 contributors, `convergence.md` §4.5). Then
  the per-author summary itself becomes `O(a)`-large and you want to reconcile
  the *set of author heads* with RBSR. This is the precise §11.3 deferral.

## The rejected runner-up: IBLT / minisketch

The gap-analysis report explicitly lists **set-difference sketches (IBLT /
minisketch)** under "Skip — already covered or speculative": "the runner-up to
RBSR, which the spec already declines. High novelty here is the warning sign, not
the virtue." Documented here only to record *why* RBSR wins for Myrhiza's shape.

- **IBLT** (Invertible Bloom Lookup Table, Goodrich & Mitzenmacher) encodes a
  set into a fixed-size table of cells; XOR-ing two parties' tables and "peeling"
  recovers the difference. Blockstream's verbatim assessment: "While IBLT has
  relatively low CPU demands, this is achieved at the expense of relatively high
  bandwidth requirements, particularly when the number of differences is small"
  ([Blockstream](https://blog.blockstream.com/en-minisketch-reducing-node-bandwidth-requirements/)).
  (The "2–10× overhead factor" sometimes quoted for IBLTs is **not** in that
  Blockstream post — it traces to the Difference-Digest / set-reconciliation
  literature; left out here rather than misattributed.) Fatal flaw: **you must
  size the table to the expected `d` in advance**; guess too low and decoding
  fails, forcing a resize and retry.
- **minisketch** (Bitcoin Core, the engine behind Erlay transaction relay) uses
  BCH codes for near-optimal sketches — "the sketch size is very close to the
  size needed to transfer the elements of the difference naively." But CPU cost
  grows steeply: per Hoyte, "As the size grows, the CPU requirements grow
  rapidly," and "the largest size currently of interest to the authors is 4096"
  ([logperiodic.com/rbsr.html](https://logperiodic.com/rbsr.html)) — i.e.
  practical sizes top out around **4,096 elements** of difference. Built for
  Bitcoin's *small frequent deltas* (a few hundred new transactions), not a
  cold-start peer fetching a year of wiki history.

### Why RBSR over sketches for Myrhiza

1. **Unknown / large `d` at cold start.** A peer joining a topic for the first
   time has `d = n` (it has nothing). Sketches degrade exactly there; RBSR's
   `O(log n)` recursion handles cold start gracefully. Myrhiza v1 bootstrap is
   *full-event-log replay from genesis* (`convergence.md` §4.2) — the worst case
   for a difference-sized sketch.
2. **No pre-sizing guesswork.** RBSR adapts to whatever `d` turns out to be; no
   "guess the table size, retry on failure" loop.
3. **Deployed at the right scale.** Negentropy runs at "10s of millions of
   elements" in strfry ([negentropy.md](negentropy.md)); minisketch tops out at
   thousands of *differences*.
4. **Interactivity is acceptable.** Myrhiza already runs an interactive gossip
   exchange; RBSR's `O(log n)` round trips ride the existing plane
   (`convergence.md` §4.3). The one-shot advantage of sketches buys little here.

Sketches' one genuine edge — minimal bandwidth for *tiny known differences* —
matches the *steady-state* gossip case, which Myrhiza already handles by gossiping
new events as they are authored (no reconciliation needed at all). Reconciliation
is for *catch-up*, where `d` is large or unknown, and that is RBSR's regime.

## Sources

- [Doug Hoyte — RBSR explainer (round-trip math; minisketch 4096 ceiling; vs Merkle COW)](https://logperiodic.com/rbsr.html)
- [Blockstream — Minisketch: Reducing Bitcoin Node Bandwidth (IBLT low-CPU/high-bandwidth tradeoff)](https://blog.blockstream.com/en-minisketch-reducing-node-bandwidth-requirements/)
- [Eppstein, Goodrich, Uyeda & Varghese — "What's the Difference? Efficient Set Reconciliation without Prior Context" (SIGCOMM 2011; Difference-Digest / IBLT overhead literature)](https://dl.acm.org/doi/10.1145/2043164.2018462)
- [bitcoin-core/minisketch](https://github.com/bitcoin-core/minisketch)
- [Invertible Bloom Lookup Tables (Goodrich & Mitzenmacher)](https://people.cs.georgetown.edu/~clay/classes/fall2017/835/papers/IBLT.pdf)
- [Range-Based Set Reconciliation (Meyer)](https://arxiv.org/abs/2212.13567)
- Myrhiza spec: `convergence.md` §4.2, §4.5; `networking.md` §11.3
- Gap analysis: `docs/reports/2026-05-29-prior-art-gap-analysis.md` (IBLT/minisketch "Skip" rationale)
