**Date:** 2026-05-29
**Status:** active
**Subject:** Lessons — validates / avoid / borrow for Myrhiza's deferred range-reconciliation decision

# Lessons for Myrhiza

**The decision file.** This folder serves two named-but-deferred spec surfaces:
"negentropy-shape range reconciliation for very large topics" (`networking.md`
§11.3) and the replacement of the `O(authors)` `HeadsSummary` scan with
`O(d log n)` range reconciliation at wiki scale (`convergence.md` §4.5).

**Frame first: this is a scaling-path reference, not a v1 need.** Myrhiza v1
commits to "every peer holds everything" (`convergence.md` §4.5); v1 bootstrap is
full-event-log replay from genesis via `HeadsSummary` delta exchange
(`convergence.md` §4.2). RBSR becomes relevant only when a real app hits the §4.5
ceiling — the spec's worked example puts that at the *wiki shape*: ~1000
contributors, ~18M events/year, ~1.8 GB ("approaching storage ceiling on
consumer devices; v1 is not the right substrate for this shape"). Do not ship any
of this speculatively.

## Validates (Myrhiza choices this corpus supports)

- **`HeadsSummary` is the right v1 choice; RBSR is correctly deferred.** Because
  per-author chains are contiguous, the §4.2 scan gets `O(authors)` *without*
  fingerprints. At v1's author-bounded scope (~tens to hundreds), RBSR's
  `O(d log n)` win does not pay for its added complexity (a sorted
  range-fingerprint index, interactive multi-round sessions). The spec's "deferred"
  posture is well-founded. ([comparisons.md](comparisons.md), [iroh-docs.md](iroh-docs.md))
- **Declining iroh-docs despite having iroh was correct.** iroh-docs *has* RBSR
  in-dependency, but it is an LWW mutable key-value map with
  single-namespace-grants-everything authority — incompatible with Myrhiza's
  append-only signed per-author DAG and per-event authority. Right call to lift
  the *algorithm* later, not the *data model*. ([iroh-docs.md](iroh-docs.md))
- **Declining IBLT/minisketch was correct.** Sketches scale with the *difference*
  and need it pre-sized; Myrhiza's cold-start bootstrap is `d = n` (worst case
  for sketches), and minisketch tops out at ~4096 differences. RBSR handles cold
  start gracefully. The gap-analysis "high novelty is the warning sign" judgment
  holds. ([comparisons.md](comparisons.md))
- **Opaque-event-set sync fits RBSR better than tree structures.** Myrhiza events
  are immutable, signed, per-author-sequential — exactly Negentropy's "set of
  32-byte IDs under a total order" shape, and a poor fit for MST/Prolly *maps*.
  The choice to keep events as a flat causal DAG (not a content-addressed B-tree)
  keeps the future RBSR path simple. ([negentropy.md](negentropy.md), [rbsr-algorithm.md](rbsr-algorithm.md))
- **Hash-lex total order already exists.** `convergence.md` §4.1 already
  tie-breaks concurrent events by `EventHash` lexicographic comparison. That same
  order (or `(HLC, EventHash)`) is a ready-made RBSR sort key — no new ordering
  decision needed when the day comes.

## Avoid (specific pitfalls + Myrhiza mitigation)

- **Weak (forgeable) fingerprints silently drop data.** A raw XOR/sum fingerprint
  lets an adversary forge a collision so a divergent range reads as "reconciled."
  *Mitigation:* if Myrhiza ever adopts RBSR, mandate a cryptographically secure
  incremental fingerprint (the design space Meyer's paper formalizes) and treat
  fingerprint choice as a *correctness*, not performance, decision. **The concrete
  scheme to copy is Negentropy's** (Protocol V1): add the 32-byte IDs mod 2²⁵⁶
  (the cheap algebraic accumulator), concatenate the element count as a varint,
  SHA-256 the result, take the first 16 bytes — the SHA-256 wrap is what makes the
  otherwise-forgeable sum collision-resistant. ([rbsr-algorithm.md](rbsr-algorithm.md),
  [open-problems.md](open-problems.md))
- **Don't bake an MST/Prolly Tree into open-network sync.** Their probabilistic
  balancing has *no hard upper bound on node size*; in an open network an
  adversary can craft keys that pile `O(n)` items into one node (G-trees paper,
  verbatim). *Mitigation:* prefer the flat-set protocol (Negentropy) which has no
  shared content-addressed nodes to inflate; if a tree is unavoidable, use a
  bounded-node (G-tree-style) design and treat node size as a security parameter.
  ([structure-stability.md](structure-stability.md))
- **Don't add a second crash-consistent on-disk structure casually.** A
  persistent Merkle tree needs durable maintenance + COW snapshots for concurrent
  sync. *Mitigation:* keep any range-fingerprint index as a rebuildable in-memory
  cache over the event log, not a converged on-disk artifact. (Ties into the open
  `embedded-storage-engines` decision — don't pre-commit storage shape via a sync
  structure.) ([structure-stability.md](structure-stability.md), [open-problems.md](open-problems.md))
- **Don't let RBSR imply partial replication.** Willow's 3D-RBSR is seductive
  ("sync just this slice"), but slicing breaks deterministic full-log replay from
  genesis — the §4.5 invariant. *Mitigation:* if range reconciliation lands, scope
  it to *discovery over the full set* (replacing `HeadsSummary`), not to syncing
  partial views, until the §4.5 "every peer holds everything" commitment is
  formally revisited. ([willow-3d.md](willow-3d.md), [open-problems.md](open-problems.md))
- **Don't conflate discovery with verification.** RBSR finds missing IDs; it says
  nothing about validity. *Mitigation:* keep signature + `prev`/`deps` + `state-apply`
  checks mandatory on ingest regardless of how an event was discovered.
  ([open-problems.md](open-problems.md))
- **Sort-key ranges leak metadata.** Reconciling over `(timestamp, id)` ranges
  reveals time-window holdings to the peer and any relay. *Mitigation:* accept it
  as an extension of the already-accepted relay metadata-correlation risk
  (`networking.md` §11.4), or track Willow's Confidential Sync for the
  interest-hiding variant if metadata privacy becomes a requirement.

## Borrow (primitives worth studying when §4.5 bites)

- **Negentropy as the reference protocol and crate.** Small, MIT, deployed at 10s
  of millions of elements, transport- and authority-agnostic, *no persistent
  tree*. The `negentropy` Rust crate (`rust-nostr/negentropy`, `0.5.0`) is the
  artifact to read first; the protocol is small enough to reimplement against
  Myrhiza's `EventHash` directly. **The recommended exemplar.** ([negentropy.md](negentropy.md))
- **Reconcile the *set of author heads*, not all events.** The cleanest fit:
  when `authors` grows large (wiki shape), keep per-author contiguity but RBSR the
  `author-head` vector instead of scanning it linearly — a minimal change to
  §4.2's protocol that preserves the per-author-DAG contract. ([iroh-docs.md](iroh-docs.md))
- **`(timestamp, id)` sort + incremental fingerprint.** Negentropy's exact
  shape; maps onto Myrhiza's `(HLC, EventHash)` events with no model change. HLC
  is materialization-only for *authority* but a fine *sort* key for a discovery
  protocol that never touches the canonical topo-sort. ([negentropy.md](negentropy.md))
- **EOSE-style "backfill complete" signal.** `networking.md` §11.3 already wants
  a `HistorySyncComplete` marker (Willow/Nostr NIP-01 precedent). Pair it with
  RBSR so a joiner knows when range reconciliation has converged, not just when a
  single exchange returned. ([willow-3d.md](willow-3d.md), [negentropy.md](negentropy.md))
- **iroh-docs as the in-dependency implementation to crib from.** Same transport,
  same blob plane, same Rust idioms — read its RBSR before writing a new one,
  even though its *data model* is the wrong fit. ([iroh-docs.md](iroh-docs.md))
- **G-trees' bounded-`k`-list idea** — file away as the fix if a content-addressed
  tree ever becomes unavoidable and node-size must be bounded against adversarial
  input. ([structure-stability.md](structure-stability.md))
- **A snapshot-aware sync envelope — co-designed with log-pruning, not bolted on.**
  RBSR and eg-walker hit *the same wall* from opposite sides: RBSR keeps
  rediscovering pruned events; eg-walker "stores operations forever" with no
  authoritative snapshot ([`../eg-walker/open-problems.md`](../eg-walker/open-problems.md)
  §1, §5). §4.5's "Snapshot-as-bootstrap with log-pruning" path (which names
  "Eg-walker-style log compaction") is where they meet. The borrowable shape: an
  **authoritative, signed, content-addressed snapshot** that is (a) the catch-up
  bootstrap for fresh joiners and (b) the lower bound above which RBSR reconciles
  only the post-snapshot tail. The eg-walker warning is load-bearing here — a
  *derived-cache* snapshot is **not** sufficient; pruned and full peers must agree
  on the truncation point or sync loops. Design the snapshot envelope (`v2
  myrhiza-state-snapshot-cache`) *before* shipping either log-pruning or RBSR.
  ([open-problems.md](open-problems.md))

## Decision shorthand

If/when `convergence.md` §4.5's ceiling is hit and the bottleneck is measured to
be *author-head scan size or full-set rediscovery bandwidth* (not storage or
replay CPU): adopt **Negentropy-shape 1D RBSR over the `EventHash` set (or author-
head vector)**, with a cryptographically secure incremental fingerprint, riding
the existing gossip plane, scoped to *discovery only* (no partial replication),
keeping verification mandatory on ingest. Do **not** introduce a persistent
MST/Prolly Tree, and do **not** reach for IBLT/minisketch.

## Sources

- [Negentropy / NIP-77 / strfry](negentropy.md) and primary sources therein
- [RBSR algorithm](rbsr-algorithm.md), [comparisons.md](comparisons.md), [structure-stability.md](structure-stability.md), [iroh-docs.md](iroh-docs.md), [willow-3d.md](willow-3d.md), [open-problems.md](open-problems.md)
- [Range-Based Set Reconciliation (Meyer)](https://arxiv.org/abs/2212.13567)
- [Geometric Search Trees (Farmer & Meyer)](https://g-trees.github.io/g_trees/)
- Myrhiza spec: `networking.md` §11.3–11.4, `convergence.md` §4.1–4.5
- Gap analysis: `docs/reports/2026-05-29-prior-art-gap-analysis.md` (Tier 2 entry; IBLT/minisketch Skip)
- Cross-corpus: [`../iroh/`](../iroh/), [`../willow/`](../willow/), [`../crdts/`](../crdts/), [`../eg-walker/`](../eg-walker/), [`../holochain/`](../holochain/), [`../at-protocol/`](../at-protocol/)
