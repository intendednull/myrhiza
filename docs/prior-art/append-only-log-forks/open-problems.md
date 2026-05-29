**Date:** 2026-05-29
**Status:** active
**Subject:** What the append-only-log-fork lineage structurally does NOT solve

# Open problems — the limits of this lineage

This file lists what *neither* SSB *nor* 2P-BFT-Log solves, so Myrhiza doesn't
mistake the borrow for more than it is. The headline: 2P-BFT-Log restores
**convergence on the fact of a fork** — it does not restore the **lost
data** or prevent the fork in the first place.

## 1. Forks are detected, never prevented

The whole paradigm is **detect-and-repair**, explicitly chosen over prevention
because prevention (consensus, ordering services) is expensive. A determined
Byzantine author *will* equivocate; the system only guarantees that correct
replicas eventually agree it happened. Myrhiza inherits this: a warrant pattern
catches equivocation after the fact; it cannot stop a keyholder from signing two
events. **Prevention requires consensus** (a quorum deciding the canonical next
event) — which Myrhiza's leaderless per-author-chain model deliberately rejects
(convergence.md: "event-log replay is the convergence paradigm").

## 2. Resolution discards the contested events

2P-BFT-Log converges everyone on the **greatest lower bound** — the last message
*before* the fork. Everything on *either* branch after the fork point is excluded
from the canonical log. So the "resolution" is **exclusion of the author from
that point forward**, not a merge that salvages legitimate post-fork activity. If
an honest user accidentally forked (multi-device race,
[ssb-fork-problem.md](ssb-fork-problem.md)), all their genuine subsequent posts on
both branches are discarded too. There is no notion of "this branch is the honest
one." Myrhiza's first-seen-wins keeps one branch but disagrees across peers;
2P-BFT-Log agrees across peers but keeps neither. **No option recovers the
contested data as canonical.**

## 3. The detection-to-propagation window is exploitable

Between the first replica detecting a fork and the proof reaching all replicas,
the malicious author can keep extending branches for unaware replicas (paper
§3.4.3). The damage is *bounded* (only matters if correct logs record `deps` onto
malicious branches) but **non-zero**. A financial/accounting app can suffer real
double-spends in that window and must replicate both branches afterward to
*compute and repair* the damage. Myrhiza apps with value-bearing state inherit
this window; the spec's drift detection (§4.7) narrows but does not close it.

## 4. Accidental forks are indistinguishable from malicious ones

The proof shows *that* an author signed two messages on one predecessor — not
*why*. A lost laptop that kept publishing, a key copied to two devices, and a
deliberate double-spend produce **identical** proofs. SSB's whole multi-device
pain ([decline.md](decline.md)) is this: the format punishes accidental forks as
harshly as attacks. Myrhiza's per-`(peer, instance)` and per-author scoping
helps, but any spec that auto-excludes on a fork proof will sometimes exclude an
honest, clumsy user. A humane warrant UX must allow an author to **publicly
abandon** a forked branch and re-anchor — a problem neither system solves.

## 5. Identity recovery / key rotation is unaddressed

If the author's key is compromised, the attacker can fork at will and there is no
in-band way to say "that key is no longer me." SSB never solved multi-device or
key rotation cleanly. 2P-BFT-Log assumes a fixed `author` key per log. Myrhiza's
identity model ([identity.md](../../specs/2026-05-09-myrhiza-master-design/identity.md))
and revocation ([distribution.md](../../specs/2026-05-09-myrhiza-master-design/distribution.md)
§10.7) live outside this lineage; the fork machinery cannot substitute for them.
This is the boundary with [`mls/`](../mls/) (forward secrecy / post-compromise)
and [`did-methods/`](../did-methods/) (key rotation).

## 6. Cross-author causality interacts with single-author forks

2P-BFT-Log notes that a correct author's `deps` may point into a forked author's
branch, which is how malicious branches leak into honest state. The repair
(stop listing forked authors in `deps`) is *per correct author's discipline*, not
mechanically enforced. Myrhiza has the same exposure: an honest event's `deps`
could reference an equivocating author's post-fork event. The kernel would need a
rule — "reject events whose `deps` reach a known-forked author's excluded branch"
— which v1 does not have.

The concrete v1 hazard a designer will hit, even *before* any warrant exists, is
narrower and worth stating plainly: under first-seen-wins, a peer that accepted
branch B1 will have *rejected* the equivocating author's branch B2; if it then
receives an otherwise-honest event whose `deps` reference an event on B2, that
`dep` is unsatisfiable on this peer and the event cannot be admitted — while a
peer that saw B2 first admits it. The same honest cross-author event is thus
admissible on some peers and not others, which is exactly the kind of branch-keyed
divergence convergence.md §4.7 drift detection must classify as downstream fallout
of an upstream author's equivocation, not as local state corruption.

## 7. Scaling: the proof doesn't shrink the log

Fork resolution is orthogonal to the §4.5 scaling ceiling. Even a fork-free log
grows without bound under full replication. Lipmaa links
([bamboo-lipmaa-links.md](bamboo-lipmaa-links.md)) and meta-feeds
([meta-feeds.md](meta-feeds.md)) address *that* axis; the fork proof does not.
Don't conflate "we can resolve forks now" with "we can scale now."

## 8. The warrant's own distribution channel is its own threat surface

This lineage tells you how to *construct and converge* a fork proof, but not how
to *carry* it without opening a new attack. 2P-BFT-Log sidesteps the question:
the proof is self-certifying, so anyone may relay it
([fork-proof-construction.md](fork-proof-construction.md)) and a bogus "proof"
that does not contain two same-`(author, seq, prev)` signature-valid events is
rejected on inspection — it costs nothing to validate-and-drop. That bounds the
*content* attack. It does **not** bound the *volume* attack: a Myrhiza warrant
event still has to be admitted to, and gossiped over, some iroh-gossip topic, and
nothing in SSB or 2P-BFT-Log says who may publish a warrant or what stops a flood
of well-formed-but-irrelevant warrant events from saturating that topic.

Myrhiza already has a precedent for exactly this admission problem in its
**per-author revocation channel** (distribution.md §10.7): revocations ride a
per-author gossip topic (`BLAKE3("myrhiza/revocations/v1" | author_pubkey)`),
auto-subscribed on install, backfilled with a HeadsSummary-shape sync, and
rate-limited by a monotonic `revocation-seq` with a `MAX_REVOCATION_JUMP` cap so a
compromised key cannot brick or flood the channel. A warrant channel inherits the
same questions — per-(forked-author) topic vs. a shared topic, who is allowed to
post, seq/rate-limiting, stale-network withholding — and §10.7 is the design to
mirror, not reinvent. The open part is that a warrant is published *about* an
author by *another* peer, so the §10.7 "only the author signs its own channel"
shape does not transfer cleanly; the warrant spec must decide the publisher set
and its rate-limit explicitly. See [lessons.md](lessons.md) §3.1.

## Sources

- 2P-BFT-Log — Erick Lavoie, arXiv [2307.08381](https://arxiv.org/abs/2307.08381) v2 §3.4.3 (impact of forks, deps leakage, double-spend repair).
- SSB multi-device / fork issues — `ssbc/ssb-server#252`, `ssbc/ssb-db#157`.
- Myrhiza spec — [`convergence.md`](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.4.1, §4.5, §4.7; [`distribution.md`](../../specs/2026-05-09-myrhiza-master-design/distribution.md) §10.7 (per-author revocation topic; warrant-channel precedent).
