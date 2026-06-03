**Date:** 2026-05-29
**Status:** active
**Subject:** Glossary — append-only-log-fork terms (SSB, EBT, Bamboo, 2P-BFT-Log)

# Glossary

System-specific terms used across this folder. For Myrhiza's own vocabulary see
the master spec; for neighbor systems see their glossaries
([`willow/glossary.md`](../willow/glossary.md), [`pears/glossary.md`](../pears/glossary.md)).

## Secure Scuttlebutt (SSB)

- **Feed** — a single-author append-only log, identified by the author's Ed25519
  public key (`@<base64>.ed25519`). The identity *is* the feed.
- **Message ID** — `%<sha256>.sha256`; the SHA-256 hash of the canonically
  serialized message. Used as the `previous` link.
- **`previous`** — backlink to the prior message's ID; forms the per-author hash
  chain. `null` for the genesis message.
- **`sequence`** — monotonic per-feed counter; `1` for genesis, then `+1`.
- **Canonical JSON** — the one fixed serialization a message is signed/hashed
  over; classic SSB pinned it to V8's `JSON.stringify` behavior.
- **Pub** — a public always-on SSB peer used as a meeting point / relay; later
  partly superseded by **Rooms**.
- **createHistoryStream** — the older request-feed-from-sequence replication RPC;
  superseded by EBT.
- **Bendy Butt** — the binary feed format used for meta-feeds.
- **PPPPP** — Staltz's experimental SSB-successor protocol; abandoned 2024.

## Epidemic Broadcast Trees (EBT)

- **EBT** — `epidemic-broadcast-trees`; SSB's deployed log-replication protocol,
  adapting Plumtree to ordered logs.
- **Plumtree** — the Leitão/Pereira/Rodrigues (SRDS 2007) protocol embedding a
  broadcast tree over a gossip overlay; **eager push** (tree, full payload) +
  **lazy push** (gossip, headers) with tree repair on failure.
- **Note** — a compact per-feed vector-clock advertisement (how far replicated,
  want-to-receive bit). The EBT analogue of a `HeadsSummary` author-head.
- **Clock** — a saved snapshot of a peer's notes (`getClock`/`setClock`) to skip
  re-advertising on reconnect.
- **Request skipping** — EBT's optimization to avoid re-requesting feeds already
  in sync; sends only the gap.

## Meta-feeds / partial replication

- **Meta-feed (root meta-feed)** — a feed whose messages only announce/create
  subfeeds; the root of an author's feed tree.
- **Subfeed** — a normal feed created under a meta-feed, carrying one purpose /
  content type; the replicable leaf.
- **Index feed** — a subfeed that indexes another feed's contents for cheap
  selective fetch.

## Bamboo / lipmaa links

- **Bamboo** — Aljoscha Meyer's single-writer append-only log with logarithmic
  backlinks for verifiable partial replication and local deletion.
- **Lipmaa link** — a second backlink (beyond `previous`) to a
  logarithmically-distant earlier entry, so any two entries are connected by an
  `O(log distance)` path. Basis: Buldas–Laud (1998) time-stamping.
- **Certificate pool** — the `O(log N)` set of intermediate entries a peer keeps
  to verify an entry's membership/position without the full chain.
- **Anti-monotone graph** — the link structure formed by lipmaa links.

## 2P-BFT-Log

- **Fork (equivocation)** — one author, two valid signed messages at the same
  index against the same predecessor; turns the per-author log from a sequence
  into a tree.
- **Irrefutable (fork) proof** — at least two signed messages from the author
  sharing one predecessor; self-certifying evidence the author forked.
  `ForkProof(M, M')`.
- **Growing phase** — `L.forks = ∅`; the log behaves as an ordinary append-only
  log.
- **Shrinking phase** — `L.forks ≠ ∅`; `L.last` becomes the greatest lower bound
  of all known fork branches; the log stays here forever once forked.
- **Greatest lower bound (GLB) / `LogPrefix`** — the longest common prefix of two
  branches; the latest message provably before any fork.
- **`MsgId`** — `hash(author ⊕ prev ⊕ idx ⊕ payload ⊕ deps ⊕ signature)`;
  self-certifying message identifier.
- **`prev` vs `deps`** — `prev` is the single same-author predecessor; `deps` are
  dependencies on *other* authors' messages (the cross-author causal history).
- **Detect-and-repair** — the paradigm of allowing Byzantine behavior, then
  detecting and excluding it, in preference to systematic prevention.
- **Self-certifying reference** — a name/ref (e.g. a key-named Git branch) whose
  consistency with its referent the receiver can check independently.

## Sources

- SSB protocol guide — [ssbc.github.io/scuttlebutt-protocol-guide](https://ssbc.github.io/scuttlebutt-protocol-guide/).
- 2P-BFT-Log — Erick Lavoie, arXiv [2307.08381](https://arxiv.org/abs/2307.08381) v2.
- `epidemic-broadcast-trees` — [GitHub](https://github.com/dominictarr/epidemic-broadcast-trees).
- Bamboo — [github.com/AljoschaMeyer/bamboo](https://github.com/AljoschaMeyer/bamboo).
- `ssbc/ssb-meta-feeds-spec` — [GitHub](https://github.com/ssbc/ssb-meta-feeds-spec).
