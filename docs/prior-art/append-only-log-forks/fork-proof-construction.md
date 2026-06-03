**Date:** 2026-05-29
**Status:** active
**Subject:** The irrefutable fork proof — two messages, one predecessor — the load-bearing borrow

# The irrefutable fork proof

This is the construction Myrhiza's "future warrant pattern" (convergence.md
§4.4.1) needs, and the reason this folder exists. The rest of the corpus is
context for it.

## The definition (verbatim shape)

From 2P-BFT-Log: an **irrefutable proof** of a fork is

> "at least two signed messages from the malicious author that have the same
> predecessor message."

Formally (paper §3.2, FL6/FL7): given the longest common prefix `P =
LogPrefix(M, M')` of two branches, the proof is the first divergent message of
each branch:

```
ForkProof(M, M') = { M'' ∈ (P, M] ∪ (P, M'] : P = LogPrefix(M, M') ∧ P = M''.prev }
```

In plain terms: take the two messages that **both name `P` as their `prev`** and
are **both signed by the same author**. That pair *is* the proof.

## Why it is irrefutable

The proof is **self-contained and self-certifying** — it needs no trusted third
party, no quorum, no timestamp, and no access to the rest of the log:

1. Both messages carry the author's valid signature (M5). Only the keyholder can
   produce them — so the author cannot deny authorship.
2. Both name the **same** `prev` and have the **same** `idx` but **different**
   `MsgId` — by the M1/M2 validity rules, a correct author publishes *at most
   one* message per `prev`. Two means the author broke the rule.
3. The signatures commit to the differing contents, so neither message can be
   forged or altered by a relayer.

Anyone holding the two-message pair can verify the equivocation **locally and
permanently**. There is no "he said / she said": the author's own key convicts
them. This is what "irrefutable" means — the proof transfers trust to no one;
it carries its own evidence.

## How it propagates and converges

Once any correct replica observes the two conflicting messages, it constructs the
proof, switches that author's log to the **shrinking phase**
([2p-bft-log.md](2p-bft-log.md)), and **gossips the proof** like any other
message. Because the merge takes the greatest-lower-bound of known forks, every
correct replica that receives the proof converges on the *same* earliest fork
point and the *same* proof set. The set of replicas still unaware of the fork
**shrinks monotonically**. Resolution outcome: the author is **excluded from
further progress** — correct replicas accept nothing after the fork point.

## The exact mapping to Myrhiza

Myrhiza events already have every field the proof needs:

| Proof ingredient | Myrhiza has it? |
|---|---|
| `author` (Ed25519 pubkey) | yes |
| `prev` (same-author predecessor hash) | yes |
| `seq` / `idx` | yes |
| Ed25519 signature over the event | yes |

So a Myrhiza fork proof is literally **two events with the same `(author, seq,
prev)` and different `EventHash`, both signature-valid**. Myrhiza's
`HeadsSummary` exchange *already surfaces the trigger*: "same seq but different
hash: Equivocation detected (§4.4.1) — flag and continue" (convergence.md §4.2).
The missing piece is not detection — it is (a) **packaging** the two events as a
portable proof, and (b) a **gossip + state rule** that converges peers onto
"author A is excluded from seq=N onward, here is the proof."

This is precisely the shape of the **Holochain warrant** the spec names as the
future direction: "warrants are signed attestations — 'I observed equivocation by
author A at seq=N' — broadcast on-DAG." 2P-BFT-Log refines that: the warrant need
not be merely an *attestation by an observer* (which itself must be trusted); it
can carry the **self-certifying two-message proof**, so the warrant is
trustless. A future `myrhiza-permission-warrants` module should carry the proof,
not just an accusation. See [lessons.md](lessons.md) Borrow §1 — the headline
borrow of this whole folder.

## A subtle design choice for Myrhiza

2P-BFT-Log converges on the **greatest lower bound** (everything strictly before
the fork is canonical; everything at/after is discarded for that author). Myrhiza
v1 instead does **first-seen-wins** (one branch becomes that peer's head). These
differ: first-seen-wins keeps *one* branch's post-fork events but disagrees
across peers; GLB keeps *neither* branch's post-fork events but agrees across all
peers. A warrant spec must choose: **converge-by-discarding-both** (2P-BFT-Log,
agreement but data loss) vs **keep-first-seen** (v1, no agreement). The
2P-BFT-Log answer is the one that actually restores convergence — at the cost of
the contested events. Name this tradeoff explicitly when the spec lands.

## Sources

- 2P-BFT-Log — Erick Lavoie, arXiv [2307.08381](https://arxiv.org/abs/2307.08381) v2: irrefutable-proof definition (Introduction); `ForkProof` / `LogPrefix` (§3.2.3); FL6/FL7 (§3.3.1); convergence/shrinking (§3.3, proofs §5).
- Myrhiza spec — [`convergence.md`](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.2, §4.4.1.
- Holochain warrants — [`holochain/`](../holochain/) prior-art folder; spec §4.4.1 "Future direction (Holochain warrant pattern)".
