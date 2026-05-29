**Date:** 2026-05-29
**Status:** active
**Subject:** Validates / avoid / borrow — what Myrhiza spec authors should do with the SSB → 2P-BFT-Log lineage

# Lessons

The consult-when-designing file. Everything else in this folder is evidence; this
is what to *do* with it. Driven by the Myrhiza decision surface this folder
serves: **author equivocation / single-author-chain fork resolution**
(convergence.md §4.4.1), the `HeadsSummary` sync protocol (§4.2), and the future
`myrhiza-permission-warrants` module.

Each row earns its place. If you can't articulate why it's here, delete it.

## 1. Validates — choices this lineage supports

| Pattern | Why this lineage proves it | Myrhiza application |
|---|---|---|
| **Single-author signed append-only chain as the substrate** | SSB ran on it for a decade; 2P-BFT-Log formalizes it (schema = `author, prev, idx, payload, deps, signature`) | Myrhiza's per-author signed Merkle DAG is the right shape — directly validated, near-identical fields (convergence.md) |
| **At most one same-author predecessor (`prev`) per event** | 2P-BFT-Log M1/M2: this is exactly what "constrains the graph to a sequence if correct, a tree if Byzantine" | Keep Myrhiza's single-`prev` rule; it is what makes a fork *detectable as a diamond* |
| **`prev` (same author) split from `deps` (other authors)** | 2P-BFT-Log uses the same split; Git mapping makes `prev` the first parent | Myrhiza already does this; the split is load-bearing for fork proofs, not cosmetic |
| **Self-certifying messages (id = hash of all fields incl. signature)** | SSB message IDs + 2P-BFT-Log `MsgId` both self-certify; no authority needed to verify | Myrhiza `EventHash` over signed event fields — keep |
| **Compact per-author tip vector for sync** | EBT "notes" (vector clock per feed) is the deployed form of this | `HeadsSummary` author-head vector (§4.2) is the right primitive — EBT validates it at production |
| **Detect-and-repair over prevent** | 2P-BFT-Log argues prevention (consensus) is more expensive than detect-and-exclude | Myrhiza's leaderless model is consistent with this; equivocation handled by detection + warrant, not consensus |
| **Equivocation surfaced to the app, not silently swallowed** | SSB's silence (feed "corrupted") was a UX failure; the fix is to make it visible | §4.7 already marks divergent digests as "author X equivocated" — keep that framing |

## 2. Avoid — pitfalls this lineage demonstrates

| Pattern | Why avoid | Myrhiza mitigation |
|---|---|---|
| **Re-serializing structured data as the signing surface** | SSB pinned signatures to **V8's `JSON.stringify`**; every non-JS validator had to reproduce V8 quirks byte-for-byte | Myrhiza signs **opaque payload bytes** and pins digest encoding to `bincode 1.3.x` with explicit Options (determinism.md §5.4) — *keep this; the SSB pain is why it's worth the cost* |
| **Leaving the fork problem unsolved indefinitely** | SSB lived a decade with "forked feed = corrupted, no recovery"; it was a top complaint and a multi-device blocker | Don't let §4.4.1 stay at first-seen-wins forever — schedule the warrant child spec; borrow §1 below gives the construction |
| **First-seen-wins as a *final* answer** | It permanently partitions peers; SSB and Myrhiza-v1 both have this; convergence never recovers | Acceptable as v1 stopgap *with detection*; not acceptable as the end state — §3 below is the upgrade |
| **Replicate-the-whole-feed** | SSB's replicate-everything didn't scale; meta-feeds/Bamboo arrived too late | Myrhiza v1's "every peer holds everything" (§4.5) is the same bet; track the ceiling; meta-feeds + lipmaa are the decomposition precedents |
| **Format churn without a migration path** | classic → Bendy Butt → meta-feeds → Bamboo → PPPPP, none migrating the base, splitting effort | Pin v1 formats (Myrhiza does: bincode 1.3.x); design format opt-ins as *additive* manifest declarations, not flag-day breaks |
| **Single-maintainer / thin-spec governance** | Patchwork "hard to maintain" with developers "burning out on it" (README); Manyverse one person; no RFC for non-JS implementers | Myrhiza: keep the master spec authoritative and language-neutral; don't let the implementation *be* the spec |
| **Treating accidental and malicious forks identically** | SSB punished multi-device accidents as harshly as attacks | A Myrhiza warrant UX must let an honest author abandon/re-anchor a forked branch (open-problems.md §4) |

## 3. Borrow — concrete constructions worth lifting

### 3.1 The irrefutable fork proof (THE borrow)

**Source:** [fork-proof-construction.md](fork-proof-construction.md); 2P-BFT-Log
§3.2.3, FL6/FL7.

The proof is **two signature-valid events with the same `(author, seq, prev)` and
different `EventHash`**. Self-certifying: no quorum, no timestamp, no trusted
observer — the author's own key convicts them, verifiable locally and
permanently. Myrhiza already has every field this needs, and `HeadsSummary`
already *detects* the trigger ("same seq, different hash", §4.2).

**What to lift into the `myrhiza-permission-warrants` spec:**
- A warrant **carries the two-event proof**, not just an accusation. This makes
  the warrant trustless — upgrading the Holochain "signed attestation" pattern
  (§4.4.1 future direction) from *observer-asserted* to *self-certifying*.
- A gossip + state rule that converges peers onto "author A excluded from seq=N
  onward; here is the proof," replacing first-seen-wins.
- The **greatest-lower-bound** resolution: canonical state = everything strictly
  before the fork; both branches' post-fork events excluded. **Name the
  tradeoff** (agreement-with-data-loss vs first-seen-wins's
  data-kept-without-agreement) in the spec body — see
  [fork-proof-construction.md](fork-proof-construction.md) and
  [open-problems.md](open-problems.md) §2.
- **A warrant-channel admission/rate-limit rule.** Self-certification bounds the
  *content* attack (a malformed "proof" validates-and-drops cheaply) but not the
  *volume* attack on the gossip topic that carries warrants. Mirror the
  per-author revocation channel (distribution.md §10.7): a derived topic,
  auto-subscribe on relevance, HeadsSummary-shape backfill, and a monotonic
  seq with a max-jump cap. The twist vs. §10.7: a warrant is published *about*
  one author *by another* peer, so "only the subject signs its own channel" does
  not transfer — the spec must name the eligible publisher set and its rate
  limit. See [open-problems.md](open-problems.md) §8.

### 3.2 The two-phase log state machine

**Source:** [2p-bft-log.md](2p-bft-log.md) §3.3.

Model an author's chain as a state with `last` + `forks`: **growing phase**
(`forks = ∅`, behaves as today) and **shrinking phase** (fork found, `last` =
GLB, `forks` holds the proof, **stays forever**). This is a clean CRDT framing
for the warrant module's per-author state. Borrow the state shape and the
monotone "once forked, always forked" invariant.

### 3.3 Deps-discipline against post-fork leakage

**Source:** [git-implementation.md](git-implementation.md); 2P-BFT-Log §3.4.3.

Once a peer holds a fork proof for author A, **refuse new events whose `deps`
reach A's excluded post-fork branch**. This bounds the detection-window damage.
Add as an explicit kernel rule in the warrant spec (Myrhiza v1 has no such rule —
open-problems.md §6).

### 3.4 Lipmaa links for verifiable partial replication (later)

**Source:** [bamboo-lipmaa-links.md](bamboo-lipmaa-links.md).

When Myrhiza crosses the §4.5 scaling ceiling and wants "snapshot-as-bootstrap
with log-pruning," lipmaa links give `O(log N)` membership proofs for a
per-author chain — verify a head without holding every prior event. Pair with
meta-feed-style decomposition ([meta-feeds.md](meta-feeds.md)) for the
across-chain axis. Study now, adopt at the ceiling, not before.

### 3.5 EBT sync optimizations

**Source:** [ebt-replication.md](ebt-replication.md).

Two `HeadsSummary` optimizations EBT proves out: **saved clocks**
(`getClock`/`setClock` — persist a peer's last-known tip vector so reconnects
skip re-advertising) and **request skipping** (only stream the gap). Borrow if
`HeadsSummary` round-trips become a bottleneck at scale.

## How to use this file

| Writing a spec on… | Read |
|---|---|
| Equivocation / fork resolution / warrants | §3.1, §3.2, §3.3; [fork-proof-construction.md](fork-proof-construction.md), [open-problems.md](open-problems.md) |
| Event schema / per-author chain integrity | §1; [2p-bft-log.md](2p-bft-log.md), [ssb-feed-format.md](ssb-feed-format.md) |
| Sync protocol (`HeadsSummary`) | §1, §3.5; [ebt-replication.md](ebt-replication.md) |
| Signing / canonical encoding | §2 row 1; [ssb-feed-format.md](ssb-feed-format.md) |
| Partial replication / scaling (v2+) | §3.4; [bamboo-lipmaa-links.md](bamboo-lipmaa-links.md), [meta-feeds.md](meta-feeds.md) |

**Rule of thumb:** borrow 2P-BFT-Log's *construction* (the proof, the two-phase
state, deps-discipline); borrow SSB's *data shape* (already done); treat SSB's
*unsolved problems* as your prioritized backlog, not your destiny.

## Cross-references

- [fork-proof-construction.md](fork-proof-construction.md) — the headline borrow in detail
- [2p-bft-log.md](2p-bft-log.md) — the full construction
- [open-problems.md](open-problems.md) — limits of the borrow
- [decline.md](decline.md) — why SSB's unsolved problems are credible warnings
- Neighbors: [`holochain/`](../holochain/) (warrant *signaling* pattern), [`willow/`](../willow/) (the DAG model + Aljoscha Meyer lineage), [`pears/`](../pears/) (Hypercore `fork` counter), [`at-protocol/`](../at-protocol/), [`mls/`](../mls/) / [`did-methods/`](../did-methods/) (key recovery, out of this lineage's scope)

## Sources

- 2P-BFT-Log — Erick Lavoie, arXiv [2307.08381](https://arxiv.org/abs/2307.08381) v2 (fork proof §3.2.3, FL6/FL7; two-phase state §3.3; deps-discipline §3.4.3).
- SSB protocol guide — [ssbc.github.io/scuttlebutt-protocol-guide](https://ssbc.github.io/scuttlebutt-protocol-guide/); `ssb-validate2-rsjs` (V8-stringify signing surface).
- `epidemic-broadcast-trees` — [GitHub](https://github.com/dominictarr/epidemic-broadcast-trees) (saved clocks, request skipping).
- Bamboo / lipmaa links — [github.com/AljoschaMeyer/bamboo](https://github.com/AljoschaMeyer/bamboo).
- Myrhiza spec — [`convergence.md`](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.2, §4.4.1, §4.5, §4.7; [`determinism.md`](../../specs/2026-05-09-myrhiza-master-design/determinism.md) §5.4; [`distribution.md`](../../specs/2026-05-09-myrhiza-master-design/distribution.md) §10.7 (warrant-channel precedent).
