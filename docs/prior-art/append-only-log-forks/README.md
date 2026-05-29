**Date:** 2026-05-29
**Status:** active
**Subject:** Append-only log forks (Secure Scuttlebutt → 2P-BFT-Log) — single-author equivocation and the irrefutable fork proof

Entry point for the append-only-log-forks corpus. This folder catalogues the
lineage of **single-author signed append-only logs** — the exact data shape
Myrhiza's per-author signed Merkle DAG inherits — with a sharp focus on the one
problem that shape does *not* solve for free: a Byzantine author who signs two
different messages at the same sequence number (**equivocation / a feed fork**).

The lineage runs from the deployed system (Secure Scuttlebutt, 2014–) that made
the data shape popular and lived with its fork problem unsolved, to the academic
construction (2P-BFT-Log, 2023) that gives the missing piece: an **irrefutable,
self-contained proof** that a fork happened, plus an eventually-consistent
protocol for all correct replicas to converge on the fork point.

## Why this folder exists

Myrhiza's convergence spec ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.4.1) commits v1 to **"first-seen-wins per peer, no automatic resolution"** for
author equivocation, and explicitly hand-waves the real fix to "a future warrant
pattern." That is *precisely* SSB's situation — SSB has lived with the unsolved
feed-fork problem for a decade — and 2P-BFT-Log is the published answer to it.
This folder is the reference a future Myrhiza warrant/equivocation child spec
should read before reinventing the fork-proof construction.

The SSB feed is the **direct ancestor** of Myrhiza's per-author chain. EBT
(Epidemic Broadcast Trees) is the **deployed cousin** of Myrhiza's
`HeadsSummary` delta exchange. The borrows are concrete, not analogies.

## Key facts

| | |
|---|---|
| **SSB** | Secure Scuttlebutt; created by Dominic Tarr, 2014. Signed single-author append-only feeds; gossip replication; offline-first |
| **SSB message crypto** | SHA-256 message IDs/links, Ed25519 detached signatures, canonical-JSON signing surface |
| **SSB academic paper** | Tarr, Lavoie, Meyer, Tschudin, "Secure Scuttlebutt: An Identity-Centric Protocol…", ACM ICN 2019 |
| **SSB status** | In decline / fragmenting; flagship clients (Patchwork deprecated; Manyverse / Planetary wound down 2024) — see [decline.md](decline.md) |
| **EBT** | `epidemic-broadcast-trees` (Dominic Tarr); npm latest `9.0.4`, MIT, last published 2022. Adapts the Plumtree paper (Leitão/Pereira/Rodrigues, SRDS 2007) to log replication |
| **Bamboo** | Aljoscha Meyer's single-writer append-only log with **lipmaa links** for verifiable partial replication; CC-BY-SA-4.0 spec |
| **Meta-feeds** | SSB tree-of-subfeeds spec (`ssbc/ssb-meta-feeds-spec`) enabling selective partial replication |
| **2P-BFT-Log** | Erick Lavoie (University of Basel), arXiv [2307.08381](https://arxiv.org/abs/2307.08381), v2 28 Jul 2023. Two-phase BFT append-only log; irrefutable fork proof; Git-backed reference implementation |

## How to use this folder

1. Read this README.
2. Read [`lessons.md`](lessons.md) — the validates / avoid / borrow decision file.
   The load-bearing borrow (the irrefutable fork proof) lives there.
3. Dive into subsystem files when writing a Myrhiza spec touching equivocation,
   per-author chain integrity, or sync.

## Reading order (canonical)

1. [`ssb-feed-format.md`](ssb-feed-format.md) — the data shape: signed
   single-author feed, `previous`/`sequence` hash chain. Myrhiza's ancestor.
2. [`ssb-fork-problem.md`](ssb-fork-problem.md) — the unsolved problem: feed
   fork = equivocation; why hash-chaining alone cannot recover. **The core.**
3. [`ebt-replication.md`](ebt-replication.md) — Epidemic Broadcast Trees; the
   deployed cousin of `HeadsSummary`.
4. [`meta-feeds.md`](meta-feeds.md) — tree-of-feeds for partial replication.
5. [`bamboo-lipmaa-links.md`](bamboo-lipmaa-links.md) — verifiable partial
   replication via logarithmic backlinks.
6. [`2p-bft-log.md`](2p-bft-log.md) — the construction: two-phase BFT log, the
   message schema (near-identical to Myrhiza's event), validity properties.
7. [`fork-proof-construction.md`](fork-proof-construction.md) — the irrefutable
   proof in detail: two messages, one predecessor. **The load-bearing borrow.**
8. [`git-implementation.md`](git-implementation.md) — 2P-BFT-Log on Git; why the
   mapping is instructive for Myrhiza.
9. [`decline.md`](decline.md) — SSB's fragmentation and decline. Honest framing.
10. [`open-problems.md`](open-problems.md) — what this lineage structurally does
    NOT solve.
11. [`lessons.md`](lessons.md) — validates / avoid / borrow for Myrhiza.
12. [`glossary.md`](glossary.md) — system-specific terms.

## Framing disclosure

**Written through Myrhiza's lens, not as a neutral catalog.** These docs are
authored from Myrhiza's *current* design stance — capability-mediated host
surface, P2P-only (no servers), Component-Model-on-Wasmtime execution,
event-log-replay `state-apply` over a per-author signed Merkle DAG. Every
"validates / avoid / borrow" judgment in [lessons.md](lessons.md) reads SSB and
2P-BFT-Log through that stance: a primitive is "validated" when it supports a
choice Myrhiza has *already made*, and "avoided" when it would break one. A
reader pursuing a different architecture (consensus-ordered logs, a server tier,
a non-WASM runtime) should re-weigh these calls — the evidence files are
reusable, but the verdicts are Myrhiza-specific.

Because this lineage is a **load-bearing precedent** for a problem Myrhiza
*shares unsolved* (the §4.4.1 fork), there is a structural incentive to
soft-pedal the parts that are bad news for Myrhiza — that first-seen-wins
permanently partitions peers, that the clean fix discards contested data
([open-problems.md](open-problems.md) §2), that the warrant channel is its own
attack surface (§8), and that the strongest borrow comes from an *academic,
unscaled* paper, not a deployed system. Those are surfaced deliberately here;
treat any place this corpus sounds reassuring about Myrhiza inheriting an SSB
problem as a place to double-check.

**Maturity, separately.** SSB *shipped* and was used by real communities for
years; its lessons are battle-tested, including the unflattering ones (the fork
problem was never solved in production; the ecosystem is fragmenting). 2P-BFT-Log
is *academic* — a single-author paper with a Git-based reference implementation,
not a deployed system at scale. Treat its construction as a vetted design to
adapt, not a load-tested artifact. The honest read: SSB tells you the problem is
real and persistent; 2P-BFT-Log tells you a clean solution exists on paper. The
**irrefutable-fork-proof construction is the load-bearing borrow** — everything
else here is context that earns it.

Neutral neighbors for the wider P2P-log design space: [`pears/`](../pears/)
(Hypercore append-only logs + `fork` counter), [`willow/`](../willow/) (the DAG
model Myrhiza generalizes), [`holochain/`](../holochain/) (per-agent source
chain + warrants), [`at-protocol/`](../at-protocol/) (signed repo, different
recovery model).

## Glossary stub

- **Feed** — an SSB single-author append-only log, identified by the author's
  Ed25519 public key. (Full terms: [`glossary.md`](glossary.md).)
- **Fork / equivocation** — one author, two valid signed messages at the same
  sequence number against the same predecessor.
- **Fork proof** — two such messages held together; self-certifying evidence the
  author equivocated.
- **Growing / shrinking phase** — 2P-BFT-Log's two phases (valid log / fork
  detected).
- **Lipmaa link** — a second backlink to a logarithmically-distant prior entry,
  enabling short verification certificates.

## Sources

- Secure Scuttlebutt — [Wikipedia](https://en.wikipedia.org/wiki/Secure_Scuttlebutt); [scuttlebutt-protocol-guide](https://ssbc.github.io/scuttlebutt-protocol-guide/).
- 2P-BFT-Log — Erick Lavoie, arXiv [2307.08381](https://arxiv.org/abs/2307.08381) (v2, 28 Jul 2023).
- Epidemic Broadcast Trees (Plumtree) — Leitão, Pereira, Rodrigues, SRDS 2007 ([PDF](https://asc.di.fct.unl.pt/~jleitao/pdf/srds07-leitao.pdf)).
- `epidemic-broadcast-trees` — [npm](https://www.npmjs.com/package/epidemic-broadcast-trees) (latest 9.0.4); [GitHub](https://github.com/dominictarr/epidemic-broadcast-trees).
- Bamboo — Aljoscha Meyer, [github.com/AljoschaMeyer/bamboo](https://github.com/AljoschaMeyer/bamboo).
- Myrhiza spec — [`convergence.md`](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.4.1.
