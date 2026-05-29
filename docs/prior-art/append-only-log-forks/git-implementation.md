**Date:** 2026-05-29
**Status:** active
**Subject:** 2P-BFT-Log on Git — why mapping the design onto commits is instructive

# 2P-BFT-Log over Git

The paper's reference implementation maps the design onto **Git** — "an
eventually consistent replicated database originally designed for distributed
version control." The mapping is deliberately small ("at most a few hundred lines
of Bash," portable to any `libgit2` binding) and is worth reading because Git is
a system every engineer already understands, so it makes the abstract CRDT
concrete.

## Messages as commits (Table 2)

| 2P-BFT-Log field | Git commit field |
|---|---|
| `M.author` | commit `author.name` (the `author.email` is unused) |
| `M.prev` + `M.deps` | commit **`parents`** — first parent is always `prev`, the rest are `deps` |
| `M.payload` + `M.signature` | commit `message` body |
| committer | unused |
| tree-hash | unused — "left open for applications" |

So `prev` and `deps` collapse into Git's natural multi-parent commit graph, with
`prev` distinguished as the *first* parent. Because commits are cryptographically
signed, "it is not possible for an adversary to forge alternative commits for
authors for which they do not possess the private key" — Git inherits the
self-certifying property directly.

> Myrhiza's events are also a multi-parent DAG (`prev` + `deps`). The "first
> parent is the same-author predecessor, remaining parents are cross-author deps"
> convention is exactly Myrhiza's split. The Git mapping is a sanity check that
> Myrhiza's event shape is a clean DAG, not an accident.

## The last message as a self-certifying branch ref

`L.last` is stored as a Git branch named `<author-pubkey>/last`. Using the
**public key as the branch name** makes the reference self-certifying: a receiver
can check the branch name matches the `author` of the commits it points to, which
are themselves signed by that author. An adversarial *relayer* can withhold
commits (hurting liveness) but **cannot** substitute commits it didn't sign —
"they cannot wrongly attribute commits not signed with the author's private key."

## The fork proof as a "diamond" commit + ref

The fork proof set `L.forks` is encoded as a **fork commit** whose parents point
to the first divergent commit of each branch — and both of those share
`<author>/last` as their parent. This forms a literal **diamond** in the commit
graph, anchored at the last pre-fork message. The proof is stored under a branch
`<author>/forks/<last-id>` (where `<last-id>` is the commit `<author>/last`
resolves to). Checking whether a log has forked is then a two-step dereference:
resolve `<author>/last` → `<last-id>`, then test whether `<author>/forks/<last-id>`
exists and points to a valid fork commit (≥2 valid parents, both having
`<last-id>` as parent). This is FL6/FL7 ([2p-bft-log.md](2p-bft-log.md)) rendered
as Git refs.

## The liveness window and the double-spend repair note

Two operational subtleties the paper is honest about — both directly relevant to
a Myrhiza warrant spec:

1. **Detection-to-propagation window.** "There is a window of opportunity between
   the detection of a fork by the first replica and the propagation of the fork
   proof to all replicas." During it, a malicious author can keep extending
   branches for replicas that haven't seen the proof yet. The damage is bounded:
   forked branches affect a correct replica's state **only if** a correct log
   records an explicit `deps` dependency onto a malicious branch. A correct
   author's mitigation: **stop listing the forked author's messages as `deps`.**
   For Myrhiza, the analogue is: once a peer holds a fork proof for author A, it
   should refuse to admit new events that declare `deps` into A's post-fork
   branches.

2. **Forked branches must still be replicated for repair.** Counter-intuitively,
   the post-fork branches "still need to be replicated to properly construct the
   causal history" — e.g. an accounting app must replicate both branches to
   compute "how many tokens were double-spent in forked branches and properly
   repair the damage." Exclusion-of-author does not mean discard-the-evidence: a
   Myrhiza app may need both branches' contents to *adjudicate* the contested
   state, even though neither branch is canonical. See
   [open-problems.md](open-problems.md).

## Why this is *not* a recommendation to use Git

Git is the paper's pedagogical and prototype substrate, not a deployment target.
Myrhiza's substrate is its own iroh-gossiped event DAG; the borrow is the
**mapping discipline** (how `prev`/`deps`/proof become graph structure + a
self-certifying ref), not Git itself.

## Sources

- 2P-BFT-Log — Erick Lavoie, arXiv [2307.08381](https://arxiv.org/abs/2307.08381) v2, §3.4.3 (impact of forked logs) and §4 (implementation over Git: Tables 2–3, §4.1–4.3).
- Myrhiza spec — [`convergence.md`](../../specs/2026-05-09-myrhiza-master-design/convergence.md) (event DAG, `deps`).
