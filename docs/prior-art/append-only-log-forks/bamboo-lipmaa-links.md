**Date:** 2026-05-29
**Status:** active
**Subject:** Bamboo and lipmaa links — verifiable partial replication of an append-only log

# Bamboo: a single-writer log with logarithmic backlinks

**Bamboo** (Aljoscha Meyer) is a "cryptographically secure, distributed,
single-writer append-only log that supports transitive partial replication and
local deletion of data." It is explicitly inspired by Scuttlebutt — it
generalizes SSB's signed linked list "to a binary anti-monotone graph to allow
partial replication." Where classic SSB makes you walk the whole chain to verify
a message belongs to a feed, Bamboo lets you verify it with a **logarithmic**
number of intermediate entries.

> Aljoscha Meyer is the same researcher behind Range-Based Set Reconciliation and
> the **Willow** protocol — see [willow/](../willow/) and the recommended
> `range-based-set-reconciliation` folder. The lineage from Bamboo's
> partial-verifiability to Willow is a single author's research arc.

## The single-backlink problem

A classic SSB feed has one backlink per message: `previous`. To prove message at
sequence `N` is really part of the feed, you must produce the *entire* chain from
genesis to `N` — `O(N)` hashes. That defeats partial replication: you can't
fetch "just the recent tail" and trust it.

## Lipmaa links

Bamboo adds a **second** backlink to each entry: a **lipmaa link** pointing not
to the immediate predecessor but to a *carefully chosen* logarithmically-distant
earlier entry. The targets are chosen so that the links form a structure where
**any two entries are connected by a path of `O(log distance)` hops**. (The
scheme derives from Buldas–Laud's 1998 work on digital time-stamping; Helger
Lipmaa's name attaches to this anti-monotone linking pattern.)

Consequences:

- For any entry of interest, a peer stores a **logarithmically-sized
  "certificate pool"** — the intermediate entries that form verification paths.
- The union of certificate pools for any two entries always contains a path
  proving their relationship. So you can verify a message's membership and
  position **without** the full chain — `O(log N)` instead of `O(N)`.
- This is **transitive partial replication**: peers verify membership through
  each other's partial data, none holding the whole log.

Bamboo also permits **local deletion** of an entry's payload while keeping its
verifiable position (the entry's hash + links remain), enabling forget-the-data
storage management — a property SSB lacks (SSB feeds are append-only forever).

## Relevance to Myrhiza

Myrhiza v1 ships full-log replay from genesis (convergence.md §4.2, §4.5) and
explicitly defers snapshots and partial replication. Lipmaa links are the
**within-a-chain** verifiable-partial-replication primitive Myrhiza would want
when it crosses the §4.5 scaling ceiling — specifically for the
"snapshot-as-bootstrap with log-pruning" evolution path the spec names. The
relevant question for a future spec: *can a peer verify a per-author chain head
without holding every prior event?* Lipmaa links answer yes, at `O(log N)` proof
size, and would let a joining peer trust a pruned/sparse chain.

The trade-off to weigh against Myrhiza's hash-DAG: lipmaa links add a second link
per event and a fixed verification-path computation, in exchange for sublinear
membership proofs. Myrhiza's `deps` already make events a DAG, not a list, so the
exact lipmaa construction (defined over a single sequence) would need adapting —
but the *property* (sublinear verifiable partial replication of a per-author
chain) is the borrow. See [lessons.md](lessons.md) Borrow §3.

## Status caveat

Bamboo is a **specification with reference implementations**, not a system
deployed at scale. The newer SSB-family format **Bendy Butt** (the meta-feed
format) and Bamboo influenced each other; neither displaced classic SSB in
deployed clients before the ecosystem's decline ([decline.md](decline.md)).
Treat Bamboo as a clean primitive to study, not a load-tested artifact.

## Sources

- Bamboo — Aljoscha Meyer, [github.com/AljoschaMeyer/bamboo](https://github.com/AljoschaMeyer/bamboo) (spec; CC-BY-SA-4.0).
- Willow protocol (same author) — [willowprotocol.org](https://willowprotocol.org/).
- Buldas, Laud, "New Linking Schemes for Digital Time-Stamping" (1998) — cited as the lipmaa-link basis in the Bamboo spec.
- Neighbor: [`willow/`](../willow/) prior-art folder.
