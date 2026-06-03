**Date:** 2026-05-29
**Status:** active
**Subject:** Epidemic Broadcast Trees (EBT) — SSB's deployed replication, the cousin of HeadsSummary delta exchange

# EBT: how SSB feeds actually replicate

SSB's deployed feed-replication protocol is **EBT** — `epidemic-broadcast-trees`,
authored by Dominic Tarr. It superseded the older `createHistoryStream`-based
`ssb-replicate`. It is the **deployed cousin** of Myrhiza's `HeadsSummary` delta
exchange (convergence.md §4.2): both exchange compact per-author tip vectors and
then stream only the missing messages.

| | |
|---|---|
| **npm** | `epidemic-broadcast-trees`, latest **9.0.4**, MIT |
| **Published** | created 2017; last published 2022 (now effectively stalled) |
| **SSB adapter** | `ssbc/ssb-ebt` wires it to secure-scuttlebutt |
| **Basis** | "loosely based on" the Plumtree paper (Leitão/Pereira/Rodrigues, SRDS 2007), adapted for ordered log replication |

## The Plumtree idea it adapts

The Plumtree paper resolves a real trade-off: tree-based broadcast has low
message complexity in steady state but is fragile under failure; gossip
(epidemic) flooding is robust but expensive. Plumtree embeds a **spanning tree
over a gossip overlay** and runs two modes simultaneously:

- **Eager push** along tree links — full message payload, fast.
- **Lazy push** along the remaining gossip links — just message *headers* (IHAVE
  notes). If a peer learns via a lazy header about a message it hasn't received
  eagerly, it grafts that link into the tree and repairs around the failure.

This gives tree efficiency in steady state and gossip resilience under churn.
(Plumtree's lineage is now used in Riak's gossip and elsewhere.)

## EBT's SSB-specific adaptations

EBT is not vanilla Plumtree — it is specialized for replicating *many ordered
logs*:

- **In-order, per-feed.** Messages carry per-feed sequence numbers; EBT sends
  them in order, so a receiver can validate the `previous`/`sequence` chain
  incrementally ([ssb-feed-format.md](ssb-feed-format.md)).
- **Vector-clock "notes".** A peer advertises, per feed, how far it has
  replicated and whether it wants to *receive* that feed — encoded compactly as
  `{ feed: (seq === -1 ? -1 : seq << 1 | !rx) }`. This is the direct analogue of
  a `HeadsSummary` author-head vector.
- **Saved clocks.** `getClock()` / `setClock()` persist a peer's last-known
  vector so reconnecting peers skip re-advertising everything — bandwidth
  optimization the spec calls **request skipping**.
- **Stall detection via timeout.** A `timeout` (default ~3000 ms) decides when a
  feed is switched to another peer, detecting a stalled supplier — the eager/lazy
  graft-repair, in log terms.
- **Cost is linear in messages to send.** The headline claim: "the cost of the
  protocol is linear with the number of messages to be sent."

## What it maps to in Myrhiza

| EBT concept | Myrhiza `HeadsSummary` analogue (convergence.md §4.2) |
|---|---|
| Per-feed vector-clock note | `author-head { author-pubkey, seq, hash }` vector |
| `getClock`/`setClock` saved clock | (Myrhiza re-exchanges; saved-clock optimization is a borrow candidate) |
| Request skipping | Author-by-author "B is ahead / A is ahead" diff, sending only the gap |
| Eager tree + lazy gossip repair | iroh-gossip carries events; Myrhiza relies on gossip's own dissemination |
| Stall timeout → switch supplier | (Not in v1 `HeadsSummary`; relevant if Myrhiza adds supplier selection) |

The key divergence: EBT replicates a **flat set of independent feeds**; Myrhiza
replicates a **cross-author DAG** where events carry `deps` into other authors'
chains. So Myrhiza's sync must also reconcile the causal `deps` closure, not just
per-author tips. EBT is the right *shape* for the per-author-tip half of that;
the cross-author half is closer to range-based set reconciliation (see the
recommended `range-based-set-reconciliation` folder and
[willow/networking.md](../willow/networking.md)).

Note the "same seq, different hash" case in §4.2 — the equivocation flag —
corresponds to two EBT peers advertising conflicting heads for one feed. EBT
*detects* this (notes don't match) but, like SSB, has no resolution; it just
can't make progress on that feed. See [ssb-fork-problem.md](ssb-fork-problem.md).

## Honest caveat

EBT is real and deployed but **lightly specified for non-JS implementers** and no
longer actively published (last npm release 2022). Like much of the SSB stack,
re-implementing it means reading the JS source rather than an RFC — the same
documentation-debt critique levelled at the Hypercore stack
([pears/critiques.md](../pears/critiques.md)). Borrow the *design* (note vectors,
request skipping, in-order streaming), not the artifact.

## Sources

- `epidemic-broadcast-trees` — [GitHub](https://github.com/dominictarr/epidemic-broadcast-trees); [npm](https://www.npmjs.com/package/epidemic-broadcast-trees) (latest 9.0.4, MIT).
- `ssbc/ssb-ebt` — [GitHub](https://github.com/ssbc/ssb-ebt).
- Plumtree — Leitão, Pereira, Rodrigues, "Epidemic Broadcast Trees", SRDS 2007 ([PDF](https://asc.di.fct.unl.pt/~jleitao/pdf/srds07-leitao.pdf)).
- Planetary EBT docs — [dev.planetary.social/replication/ebt.html](http://dev.planetary.social/replication/ebt.html).
- Myrhiza spec — [`convergence.md`](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.2.
