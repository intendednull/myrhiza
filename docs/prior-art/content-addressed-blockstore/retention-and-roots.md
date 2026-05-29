**Date:** 2026-05-29
**Status:** active
**Subject:** Retention and roots — pin / tag / ref / snapshot as one primitive under five names

# Retention and roots

"What keeps a block alive?" has the same answer in every system here — *an
explicit root that GC walks from* — but each names it differently and supports
different granularities. This file lines them up so the Myrhiza retention design
can pick the right granularity deliberately.

## The one primitive, five names

| System | Retention primitive | Recursive? | Notes |
|---|---|---|---|
| git | **ref** (branch/tag/HEAD) + reflog | yes (commit → tree → blobs) | reflog auto-retains recently-detached history for a window |
| restic | **snapshot** | yes (snapshot → tree → blobs) | snapshots are the *only* roots; `forget` removes them |
| IPFS/boxo | **pin** (recursive / direct / indirect) + MFS | recursive pin: yes; direct: no | indirect pin = "alive because an ancestor is recursively pinned" |
| casync | **index file** (`.caidx`/`.caibx`) | yes (index lists all chunk hashes) | no online GC; retiring an index makes its unique chunks sweepable |
| iroh-blobs | **tag** | a tag on a HashSeq retains members | only retention primitive; no refcount/LRU/quota |

The convergence: a root is a *named, externally-managed pointer the store
promises not to collect*, and retention is recursive — pinning/tagging/branching
a root keeps its entire transitive closure. boxo's **direct pin** (keep this one
block, not its children) is the one non-recursive variant, useful when you want
to retain an interior node without its subtree.

## Two retention granularities worth distinguishing

1. **Whole-graph roots** (git ref, restic snapshot, recursive pin, iroh HashSeq
   tag): "keep this root and everything under it." This is what you want for
   "keep this installed app bundle" or "keep this snapshot."
2. **Single-object roots** (boxo direct pin): "keep exactly this block." Useful
   for pinning an interior block you're actively serving even though no whole-
   graph root currently covers it — which is precisely the concurrent-serve
   protection problem from a different angle
   ([concurrency-and-locking.md](concurrency-and-locking.md)).

## Grace windows are *implicit* roots

git's reflog (30-day unreachable grace) and boxo's MFS root ("implicit pinning")
are both **roots you didn't explicitly create**. They exist to keep alive things
the user is likely to still want (recently-reset commits) or things the system is
mid-operation on. The lesson: a retention model needs not just user-declared
roots but **system-derived implicit roots** for in-flight and recently-detached
state, or GC will delete things out from under live operations. See
[git.md](git.md) and [gc-strategies.md](gc-strategies.md).

## Myrhiza relevance

Myrhiza's retention roots are already emerging across the spec; this survey says
to model them uniformly as a root set, not as ad-hoc special cases:

- **Installed bundles** — B-10 ships the recursive-root model exactly: a
  `bundle/<manifest_hash>` tag keeps the bundle's HashSeq alive while installed,
  dropped on uninstall
  ([b-10 spec](../../specs/2026-05-26-b-10-bundle-distribution-design.md) §4.3).
  This is the iroh-blobs-tag / restic-snapshot pattern.
- **Snapshot cache** — the v2 `myrhiza-state-snapshot-cache`
  ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
  §4.2; [risks.md](../../specs/2026-05-09-myrhiza-master-design/risks.md) §19)
  needs each cached snapshot to be a root keeping its anchor event reachable.
- **Live heads** — the current heads of every author chain are implicit roots:
  log truncation must never collect events still reachable from a live head, even
  if they're below a snapshot anchor.
- **Implicit in-flight roots** — events a sync provider is actively serving, or a
  partial blob a download is filling, need grace-window/implicit-root protection
  (boxo's MFS-style "protected but not pinned," git's reflog window).

The design recommendation: a single explicit root set
(installed-bundle tags + snapshot anchors + live heads) plus a short grace window
or pin-lock for in-flight serves, with reachability re-derived each sweep. See
[lessons.md](lessons.md).

## Sources

- [IPFS pinning docs](https://docs.ipfs.tech/how-to/pin-files/)
- [restic forget/prune documentation](https://restic.readthedocs.io/en/stable/060_forget.html)
- [git-gc(1) documentation](https://git-scm.com/docs/git-gc)
- [`prior-art/iroh/blobs.md`](../iroh/blobs.md)
