**Date:** 2026-05-29
**Status:** active
**Subject:** Open problems — what RBSR and its backing structures structurally do NOT solve

# Open problems

RBSR and its tree structures solve *one* problem well: efficiently discovering
the set difference between two replicas. They leave the following unsolved, by
construction. These are the things a Myrhiza spec must supply *around* any RBSR
adoption.

## RBSR / Negentropy doesn't solve

- **Authority / validity.** RBSR tells you *which IDs you lack*, never whether
  they are *valid*. Signature checks, `prev`/`deps` chain integrity, and the
  `state-apply` verdict are all out of scope. A malicious peer can offer IDs for
  events that fail verification; you waste a fetch discovering this. Myrhiza's
  per-event Ed25519 signatures and `state-apply` (`convergence.md` §4.4) remain
  mandatory on top.
- **What to do with the difference.** RBSR produces `have`/`need` lists; *moving*
  the records is a separate transport step. Negentropy explicitly does not ship
  payloads — the caller fetches `need` over its own transport (iroh-blobs/gossip
  for Myrhiza).
- **Fingerprint forgery (if you pick a weak scheme).** A non-cryptographic
  fingerprint (raw XOR/sum) lets an adversary forge a collision so peers wrongly
  believe a range is reconciled and **silently drop the difference**. Mitigation
  is choosing a cryptographically secure incremental fingerprint — the design
  space Meyer's paper formalizes ([rbsr-algorithm.md](rbsr-algorithm.md)). The
  concrete known-good answer is **Negentropy's**: sum the IDs mod 2²⁵⁶ (the cheap
  incremental part), then SHA-256 the sum plus the element count and truncate to
  16 bytes (the part that defeats forgery). This is a *correctness* hazard, not
  just performance.
- **Total-order agreement.** Both sides must derive the *same* total order over
  items, or ranges don't line up. For Myrhiza this means agreeing on the sort key
  (`(HLC-timestamp, EventHash)` or pure `EventHash`-lex). Cheap, but a real
  precondition.
- **Metadata leakage of the sort key.** Reconciling over `(timestamp, id)` ranges
  reveals *which time windows / ID ranges* each side holds to the other party
  (and to any relay observing the exchange). This compounds the accepted relay
  metadata-correlation risk (`networking.md` §11.4). Willow's Confidential Sync
  addresses interest-set leakage; 1D Negentropy does not.

## The tree structures (MST / Prolly) additionally don't solve

- **Bounded node size in open networks.** Adversarial keys can inflate a single
  node to `O(n)` items (G-trees paper, [structure-stability.md](structure-stability.md)).
  No fix in the deployed MST/Prolly designs; the bounded-node fix (G-trees) is
  research-grade.
- **Append-only / immutable-log fit.** MSTs and Prolly Trees are *mutable maps*.
  An append-only signed event log is a poor fit — there is no key space to
  authenticate, and equivocation forks (`convergence.md` §4.4.1) don't map onto a
  single-shape content-addressed tree.
- **A second crash-consistent on-disk structure.** Adopting a persistent tree
  means maintaining it durably alongside the event store — extra surface for the
  unaddressed `embedded-storage-engines` / `content-addressed-blockstore`
  decisions (gap-analysis Tier 1). Negentropy's order-statistics index can be an
  in-memory cache, rebuildable from the log.
- **COW snapshots for concurrent mutation.** Serving sync against a changing
  persistent Merkle tree needs copy-on-write snapshots; Negentropy sidesteps this
  ([structure-stability.md](structure-stability.md)).

## What none of them solve (shared with the broader corpus)

- **Partial replication's deterministic-replay break.** Syncing only a *slice*
  (Willow 3D-RBSR's whole point) breaks deterministic full-log replay from
  genesis — the same warning logged in [`../holochain/open-problems.md`](../holochain/open-problems.md)
  and the gap-analysis `partial-replication-shapes` maybe. Myrhiza v1's "every
  peer holds everything" (`convergence.md` §4.5) deliberately avoids this; any
  RBSR-for-slices adoption re-opens it.
- **No WASM Component Model artifact.** None of negentropy / MST / Prolly crates
  ship a CM artifact with a WIT interface — same gap noted for the CRDT libs in
  [`../crdts/open-problems.md`](../crdts/open-problems.md) and
  [`../eg-walker/open-problems.md`](../eg-walker/open-problems.md). Any Myrhiza
  use would be kernel-internal Rust, not an app-facing component.
- **Garbage collection / log truncation interaction.** RBSR reconciles whatever
  is in the set; if a peer truncates old events past a snapshot, the two peers no
  longer hold the same set and reconciliation will repeatedly "rediscover" the
  truncated difference — a peer that *kept* the old events keeps offering them to
  a peer that *pruned* them, which re-requests-then-re-prunes forever — unless the
  protocol learns about snapshot boundaries. Unsolved at the RBSR layer; needs a
  snapshot-aware sync envelope.

  This is exactly where RBSR meets the **eg-walker cluster**. `convergence.md`
  §4.5's first evolution path is "**Snapshot-as-bootstrap with log-pruning**,"
  and the spec names "**Eg-walker-style log compaction**" there by name (calling
  it "research-grade"). The two corpora describe *the same unsolved problem from
  two angles*: [`../eg-walker/open-problems.md` §1](../eg-walker/open-problems.md)
  ("Garbage collection at scale") says eg-walker "stores operations forever" and
  that "garbage collection requires coordination across all replicas that may
  still hold divergent branches" — and §5 ("Offline merge cost") notes its
  snapshot is "a derived cache, not an authoritative state form." Put together:
  log-pruning needs an **authoritative, content-addressed snapshot** that doubles
  as the catch-up bootstrap *and* the lower bound RBSR reconciles above (you only
  range-reconcile the post-snapshot tail; the snapshot itself is fetched whole).
  Neither RBSR nor eg-walker supplies that snapshot envelope — Myrhiza must
  design it (the v2 `myrhiza-state-snapshot-cache`, `convergence.md` §4.2). The
  eg-walker lesson is the warning: a *derived-cache* snapshot is not enough; the
  snapshot has to be authoritative (signed, content-addressed) so a pruned peer
  and a full peer agree on the truncation point without re-exchanging the pruned
  prefix.

## Sources

- [Geometric Search Trees (unbounded node size)](https://g-trees.github.io/g_trees/)
- [Doug Hoyte — RBSR explainer (COW, fingerprint choice)](https://logperiodic.com/rbsr.html)
- [Range-Based Set Reconciliation (Meyer) — fingerprint design space](https://arxiv.org/abs/2212.13567)
- [Negentropy README (have/need; no payload transfer)](https://github.com/hoytech/negentropy)
- [Negentropy Protocol V1 spec — Fingerprint Algorithm (concrete secure scheme)](https://github.com/hoytech/negentropy/blob/master/docs/negentropy-protocol-v1.md#fingerprint-algorithm)
- Sibling: [structure-stability.md](structure-stability.md), [comparisons.md](comparisons.md)
- Cross-corpus: [`../holochain/open-problems.md`](../holochain/open-problems.md), [`../crdts/open-problems.md`](../crdts/open-problems.md), [`../eg-walker/open-problems.md`](../eg-walker/open-problems.md)
- Myrhiza spec: `convergence.md` §4.4–4.5, `networking.md` §11.3–11.4
