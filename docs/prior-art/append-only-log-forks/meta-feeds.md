**Date:** 2026-05-29
**Status:** active
**Subject:** SSB meta-feeds — a tree of subfeeds for selective partial replication and format migration

# Meta-feeds: SSB's answer to "I don't want your whole feed"

Classic SSB replicates a feed **whole**: to follow someone you replicate every
message they ever published, of every type. That does not scale, and it couples
"I want your posts" to "I must download your reactions, your private-group ops,
your app-specific noise." **Meta-feeds** (`ssbc/ssb-meta-feeds-spec`, originally
incubated under `ssb-ngi-pointer`) restructure a feed into a **tree**:

- A **root meta-feed**, derived from the identity, publishes only *announcements*
  that create **subfeeds**.
- Each **subfeed** is itself a normal append-only feed, carrying one content type
  / purpose (e.g. `aboutMe`, `contacts`, an application's data).
- The tree follows a "tree structure v1" where consumers care about the **leaf
  feeds**.

A peer meeting you for the first time replicates your **root meta-feed**,
discovers your subfeeds, and then replicates *only the leaves it cares about* —
e.g. just `aboutMe` and `contacts` to place you socially, deferring the rest.

## What meta-feeds enable

- **Partial / selective replication.** Subscribe per-subfeed, not per-identity.
  Sharded subfeeds let a peer deterministically compute which shard to replicate
  for a content type it wants.
- **Feed-format migration.** A subfeed can carry a *new* feed format while the
  old root stays valid — the migration path classic SSB never had cleanly
  (`ssb-meta-feeds-migration-spec`). This is the SSB analogue of a schema/format
  version bump without forking every follower.
- **Index feeds.** Special subfeeds that index another feed's contents, so a
  consumer can fetch an index cheaply and then pull only matching messages.
- **Group feeds.** `ssb-meta-feeds-group-spec` layers private-group membership
  on the tree.

## Why it matters for Myrhiza

Myrhiza's v1 commits to "every peer holds everything" for a topic
(convergence.md §4.5) and defers partial replication to v2+. Meta-feeds are a
worked example of the **decomposition strategy** Myrhiza will eventually need:
split a monolithic per-author stream into typed sub-streams so consumers
replicate a subset. The closest existing Myrhiza precedent cited in the spec is
**Holochain's DHT op decomposition** (§4.5, [holochain/](../holochain/)); SSB
meta-feeds is the *same idea at the feed layer instead of the DHT layer* —
decompose the author's output into independently-replicable typed strands.

A subtlety worth flagging for that future spec: meta-feeds **multiply the number
of feeds an author owns**, which multiplies the number of independent chains that
can fork. Each subfeed is a single-author log with its own fork exposure
([ssb-fork-problem.md](ssb-fork-problem.md)). Partial replication and equivocation
interact: a peer replicating only a subset of an author's subfeeds may not hold
the subfeed where the fork is visible. Resolution machinery
([2p-bft-log.md](2p-bft-log.md)) must be reasoned about per-chain, not
per-identity.

## Relation to lipmaa links

Meta-feeds give partial replication **across feeds** (pick which subfeeds). Bamboo's
lipmaa links ([bamboo-lipmaa-links.md](bamboo-lipmaa-links.md)) give verifiable
partial replication **within a feed** (prove a single message belongs to the feed
without the whole chain). They are complementary, not competing; a fully partial
system wants both axes.

## Sources

- `ssbc/ssb-meta-feeds-spec` — [GitHub](https://github.com/ssbc/ssb-meta-feeds-spec) (design doc for subfeeds).
- `ssbc/ssb-meta-feeds` — [README](https://github.com/ssbc/ssb-meta-feeds/blob/master/README.md) (JS implementation).
- `ssbc/ssb-meta-feeds-migration-spec` — [GitHub](https://github.com/ssbc/ssb-meta-feeds-migration-spec).
- `ssbc/ssb-subset-replication-spec` — [GitHub](https://github.com/ssbc/ssb-subset-replication-spec).
- Myrhiza spec — [`convergence.md`](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.5.
