**Date:** 2026-05-29
**Status:** active
**Subject:** iroh-docs — iroh's own RBSR implementation, and why Myrhiza chose the per-author `HeadsSummary` scan instead

# iroh-docs

[`iroh-docs`](https://github.com/n0-computer/iroh-docs) is Number 0's
"multi-dimensional key-value documents with an efficient synchronization
protocol." It is the most relevant RBSR implementation to Myrhiza because **iroh
is already Myrhiza's locked load-bearing transport** (`networking.md` §11.1,
[`../iroh/`](../iroh/)).

The README states verbatim that its sync "is based on a technique called
_range-based set reconciliation_, based on [this paper] by Aljoscha Meyer"
([arXiv:2212.13567](https://arxiv.org/abs/2212.13567)), "recursively
partitioning the sets and comparing fingerprints of the partitions to
probabilistically detect whether a partition requires further work." Crate
version `0.100.0` (verified via repo, May 2026) — iroh-docs tracks the iroh
1.0-rc mainline, unlike the stalled `iroh-willow`
([`../iroh/willow.md`](../iroh/willow.md)).

## The data model

iroh-docs replicas hold entries identified by `(namespace, author, key)` with a
value (a content hash into iroh-blobs). It is **last-writer-wins per
(author, key)** — a mutable key-value store, like atproto's MST repo or a Dolt
table, not an append-only event log. Sync runs 1D RBSR over the entry set.

A known sharp edge documented in [`../iroh/docs.md`](../iroh/docs.md): iroh-docs'
authorization is **single-`NamespaceId`-grants-everything** (anyone with the
namespace write capability can write any key under any author), and old entries
are silently shadowed rather than tombstoned — weaker than Willow's Meadowcap or
Myrhiza's per-event-signed authority model.

## Why Myrhiza did NOT just use iroh-docs

This is the load-bearing comparison. Myrhiza's `convergence.md` chose a
**per-author signed Merkle DAG** with a `HeadsSummary` delta protocol, *not*
iroh-docs, despite iroh-docs being available in the same dependency. Reasons
drawn from the spec:

1. **Append-only signed log vs mutable LWW map.** Myrhiza events are immutable,
   per-author-sequential, Ed25519-signed, with `prev`/`deps` chaining. iroh-docs'
   LWW-key-value model would discard the causal DAG and the signature-chain
   integrity that `state-apply` depends on.
2. **`HeadsSummary` already gets the per-author win cheaply.** Because each
   author chain is a contiguous `seq`-ordered run, exchanging per-author DAG-tip
   vectors (`author-head { pubkey, seq, hash }`, `convergence.md` §4.2) localizes
   the difference in `O(authors)` *without* fingerprint recursion. RBSR's
   `O(d log n)` advantage only materializes when *authors* itself is large.
   At v1's author-bounded scope (~tens to hundreds), the scan is simpler and
   adequate.
3. **Authority is first-class in Myrhiza.** iroh-docs' single-namespace grant is
   exactly the model Myrhiza rejects in favor of per-event authority verdicts
   from `state-apply`.

So iroh-docs is best read as **the available-today RBSR implementation Myrhiza
deliberately declined**, for data-model and authority reasons — not performance.
If the §4.5 scaling ceiling is reached, the lesson is to lift RBSR's *algorithm*
(over the set of author heads), not iroh-docs' *data model*. See
[lessons.md](lessons.md).

## Implications for Myrhiza

- If a future spec adds range reconciliation, iroh-docs is the **in-dependency
  reference implementation** to read first — same transport, same blob plane,
  same Rust idioms.
- But the entry-level RBSR in iroh-docs reconciles *mutable entries*; Myrhiza
  would reconcile *immutable event hashes* (or author-head vectors). The wire
  algorithm transfers; the data model does not.
- iroh-gossip + iroh-blobs (already committed) carry the events; RBSR would only
  replace the *discovery* step (`HeadsSummary`), riding the existing gossip plane
  per `convergence.md` §4.3 ("no new transport surface").

## Sources

- [iroh-docs (RBSR / Aljoscha Meyer attribution; crate 0.100.0)](https://github.com/n0-computer/iroh-docs)
- [Range-Based Set Reconciliation (Meyer)](https://arxiv.org/abs/2212.13567)
- Sibling prior-art: [`../iroh/docs.md`](../iroh/docs.md), [`../iroh/willow.md`](../iroh/willow.md), [`../iroh/gossip.md`](../iroh/gossip.md)
- Myrhiza spec: `networking.md` §11.1–11.3, `convergence.md` §4.1–4.5
