**Date:** 2026-05-29
**Status:** active
**Subject:** The SSB feed-fork problem — single-author equivocation that hash-chaining cannot recover from

# The feed fork: SSB's unsolved problem, Myrhiza's §4.4.1

This is the central file of the folder. Everything else is context for it.

## What a feed fork is

A **feed fork** (equivocation) is when one author signs **two different valid
messages at the same `sequence`**, each with the same `previous` link. Because
both are signed by the author's key and chain correctly, both are individually
valid. The single-author feed, meant to be a list, becomes a **tree** — two
branches diverging from a shared predecessor.

2P-BFT-Log states the failure precisely: cryptographic hashing of predecessors
"does not prevent a malicious author from creating two concurrent messages for
the same index, thereby forking their log and turning it into a tree.
Cryptographic hashes are therefore not sufficient to sequentially order the
messages of a log."

## Why it cannot be recovered by the format alone

The failure is structural, not a bug:

- **Both branches verify.** There is no signature, hash, or sequence check that
  rejects the second branch — the author had the key and used it correctly.
- **First-seen-wins partitions the network.** SSB validators reject any message
  whose `previous`/`sequence` doesn't extend the chain head they already hold.
  So once replica X has replicated branch B1, it treats branch B2's messages as
  invalid — and vice-versa for replica Y that saw B2 first. Result: "a
  partitioning of correct replicas, preventing convergence" (2P-BFT-Log). The
  two halves of the network can **never reconcile** by replicating more
  messages, because each rejects the other's branch.
- **The damage is permanent under the deployed design.** Dominic Tarr, SSB's
  creator, described the behaviour directly in `ssbc/ssb-db#157`: "the current
  behavior is if the network has already received the original n_0 message, it
  would be expecting a n_1 message that points to that. If some nodes have not
  seen the original n_0 then they will accept the new one and your feed will be
  forked." The recovery story he wanted — "when a fork is detected, that message
  is used to create a proof that the feed has forked, which can be given out when
  someone requests the feed, killing that feed" — was, in his words, "hasn't been
  implemented yet" and never shipped in classic SSB. That unimplemented proof is
  precisely what 2P-BFT-Log later formalized
  ([fork-proof-construction.md](fork-proof-construction.md)).

## How forks happen accidentally (it's not only attackers)

The most common cause is **multi-device key sharing**. If a user copies their
feed keypair to a second device and both publish, the two devices race and
produce messages at the same sequence — an accidental fork. This is why SSB
warns hard against key sharing and why "use a feed per device" became the
de-facto pattern; it is also the motivation for meta-feeds
([meta-feeds.md](meta-feeds.md)) and for SSB's lack of clean multi-device
identity ([open-problems.md](open-problems.md)). The relevant tracker:
`ssbc/ssb-server#252` ("Multiple device support") and `ssbc/ssb-db#157`
("Network reaction to falsified/forked own feed?").

## The exact Myrhiza parallel (convergence.md §4.4.1)

Myrhiza's spec describes the identical situation and the identical v1 stance:

> "A malicious author may sign two events with the same `seq` against the same
> `prev`. Different peers see one or the other first… **v1 resolution:
> first-seen-wins per peer.** … equivocating authors can permanently fork their
> own chain across peers. … **v1 does not provide automatic resolution.**"

So Myrhiza v1 **is** classic SSB on this axis: same data shape, same
first-seen-wins, same permanent partition, same "punt the real fix." The spec
names the future fix as the "Holochain warrant pattern" — but the *concrete
construction* for that warrant is exactly what 2P-BFT-Log supplies
([fork-proof-construction.md](fork-proof-construction.md)). Myrhiza has two
prior-art veins feeding one decision: Holochain warrants (the *signaling*
mechanism) and 2P-BFT-Log (the *proof* that the signal carries).

## Detection vs. resolution — keep them separate

Myrhiza already *detects* the fork: `HeadsSummary` exchange flags "same seq,
different hash" as equivocation (convergence.md §4.2), and §4.7 drift detection
explicitly marks divergent digests as "author X equivocated; peers diverged on
which branch was first-seen" rather than "your state is buggy." What v1 lacks is
**resolution** — converging the partitioned peers back onto one canonical view
(or a shared "this author is excluded" verdict). 2P-BFT-Log is a resolution
protocol, not just a detector; that is its contribution over SSB.

## Sources

- 2P-BFT-Log — Erick Lavoie, arXiv [2307.08381](https://arxiv.org/abs/2307.08381) v2, Introduction (quotes on fork = tree, partitioning).
- `ssbc/ssb-db#157` — [Network reaction to falsified ("forked") own feed](https://github.com/ssbc/ssb-db/issues/157).
- `ssbc/ssb-server#252` — [Multiple device support](https://github.com/ssbc/ssb-server/issues/252).
- SSB spec, Feeds and Messages — [spec.scuttlebutt.nz/feed/messages.html](https://spec.scuttlebutt.nz/feed/messages.html).
- Myrhiza spec — [`convergence.md`](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.2, §4.4.1, §4.7.
