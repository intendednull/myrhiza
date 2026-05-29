**Date:** 2026-05-29
**Status:** active
**Subject:** 2P-BFT-Log — a two-phase Byzantine-fault-tolerant append-only log that converges in the presence of forks

# 2P-BFT-Log: the construction

**Paper:** Erick Lavoie (University of Basel), *"2P-BFT-Log: 2-Phase
Single-Author Append-Only Log for Adversarial Environments"*, arXiv
[2307.08381](https://arxiv.org/abs/2307.08381), v2 dated 28 Jul 2023.
Single-author paper with definitions, algorithms, convergence proofs, and a
Git-backed reference implementation ([git-implementation.md](git-implementation.md)).

## The problem it fixes

Existing append-only logs (SSB, Hypercore, etc.) "assume replicas faithfully
maintain the sequential properties of logs and do not provide eventual
consistency when malicious participants fork their logs." A fork partitions
correct replicas by which branch they saw first — exactly
[ssb-fork-problem.md](ssb-fork-problem.md), exactly Myrhiza §4.4.1. 2P-BFT-Log's
novel contribution: a **second "shrinking" phase** after a fork is discovered, so
that all correct replicas converge on the earliest observed fork point **plus an
irrefutable proof it happened**, instead of partitioning forever.

## The message schema (compare to Myrhiza's event)

A message `M` has fields (Table 1 of the paper):

| 2P-BFT-Log field | Meaning | Myrhiza event field |
|---|---|---|
| `M.author` | author public key | `author` |
| `M.prev` | the **single** same-author predecessor (`⊥` if first) | `prev` |
| `M.idx` | sequence index | `seq` |
| `M.payload` | opaque payload | opaque payload bytes |
| `M.deps` | dependencies on **other** authors' messages (full causal history) | `deps` (cross-author causal heads) |
| `M.signature` | Ed25519-style signature over the fields | Ed25519 signature |

`MsgId(M) = hash(author ⊕ prev ⊕ idx ⊕ payload ⊕ deps ⊕ signature)` — making the
(id, message) pair **self-certifying**. This schema is *strikingly* close to
Myrhiza's: "author, sequence number…, `prev`…, `deps` (array of cross-author
causal heads), opaque payload bytes, and Ed25519 signature" (convergence.md). The
paper restricts a message to **at most one same-author dependency** (`prev`),
which "constrains the message graph of any given author to a sequence, if
correct, or a tree, if Byzantine" — the precise framing Myrhiza needs.

## Message validity (M1–M6)

A message is valid iff:

- **M1 (single previous):** `prev = ⊥`, or `prev` is the id of one valid message.
- **M2 (single author):** if `prev` references `M'`, then `M'.author = M.author`.
- **M3 (valid external deps):** every id in `deps` resolves to a valid message
  from a *different* author.
- **M4 (single author dependencies):** at most one `deps` entry per other author.
- **M5 (self-certifying):** the signature is consistent with `author`.
- **M6 (acyclic):** `prev` does not reference a successor of `M`.

A correct replica validates a message only once all its predecessors/deps are
themselves validated — so even a (hypothetical) hash cycle from a Byzantine
author is never accepted. This is the "verify authorship and venue" step: a
message is admitted only if its author and its place in the graph check out.

## The two phases (as a state-based CRDT)

A log state `L` carries: `L.author`, `L.last` (the last message of the strict
sequence), and `L.forks` (a set of messages proving a fork, possibly empty).

- **Growing phase** (`L.forks = ∅`): behaves like an ordinary append-only log.
  Appending `M`: if `M` extends `L.last` it advances; if `M` is a duplicate it's
  ignored; **if `M` is concurrent with `L.last` (same `prev`, different message),
  a new fork is found and the log switches to the shrinking phase.**
- **Shrinking phase** (`L.forks ≠ ∅`): `L.last` becomes the **greatest lower
  bound (longest common prefix)** of all known forked branches — the latest
  message provably *before* any fork. `L.forks` holds ≥2 messages sharing
  `L.last` as predecessor. **Once forked, a log stays in the shrinking phase
  forever** (forks are permanent facts; you cannot un-observe one).

Validity properties FL1–FL7 govern the shrinking phase. The load-bearing ones:
**FL6 (valid proof)** — there exist two distinct `M, M' ∈ forks` with
`M.prev = M'.prev` — and **FL7 (consistent proof)** — that shared predecessor is
`L.last`. Together, the phase rules "constrain a Byzantine log author to only two
options: either produce a correct sequential log, or produce forks of valid
messages." Any other state is ignored by correct replicas.

## What converges

The CRDT merge is the key result: merging two log states takes the **greatest
lower bound** when they are on different branches, so all correct replicas
monotonically shrink toward the **same** earliest-fork point and accumulate the
**same** fork proof. This restores eventual consistency — the property SSB lost
to forks. Crucially it is a **detect-and-repair** paradigm, not prevention: forks
are *allowed to happen* and then detected and excluded, which the paper argues is
cheaper than systematically preventing Byzantine behavior up front.

The detailed proof object is in [fork-proof-construction.md](fork-proof-construction.md);
what to take from it for Myrhiza is in [lessons.md](lessons.md).

## Honest limits

- **Single-author paper, no scale deployment.** The reference implementation is
  Git-backed (instructive, not production). Treat as a vetted design.
- **Resolution = exclusion, not reconciliation.** 2P-BFT-Log converges everyone
  on "this author forked here, here's proof, ignore everything after." It does
  **not** merge the two branches' content into one valid history — the post-fork
  payloads on both branches are discarded as untrusted. For Myrhiza this means a
  warrant pattern can *quarantine* an equivocating author cleanly, but cannot
  recover the contested events. See [open-problems.md](open-problems.md).

## Sources

- 2P-BFT-Log — Erick Lavoie, arXiv [2307.08381](https://arxiv.org/abs/2307.08381) v2 (28 Jul 2023). Schema Table 1; validity M1–M6, CL1–CL3, FL1–FL7; phases §3.3.
- [ResearchGate mirror](https://www.researchgate.net/publication/372416608) (same paper).
- Myrhiza spec — [`convergence.md`](../../specs/2026-05-09-myrhiza-master-design/convergence.md) (event schema; §4.4.1).
