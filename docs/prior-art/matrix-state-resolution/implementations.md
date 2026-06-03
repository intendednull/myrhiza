**Date:** 2026-05-29
**Status:** active
**Subject:** State resolution implementations — Synapse, Dendrite, Conduit/conduwuit, ruma-state-res; the chain-cover index and performance traps

# Implementations and performance

State resolution has been implemented independently several times — useful for
Myrhiza because the *divergence between implementations* is itself evidence of how
subtle the algorithm is (Dendrite hit an off-by-one that Synapse didn't; see
[state-reset-hazard.md](state-reset-hazard.md)).

| Implementation | Language | Steward | Role |
|---|---|---|---|
| **Synapse** | Python (Twisted) | Element / Matrix.org | The reference homeserver; the de-facto authoritative state-res implementation. |
| **Dendrite** | Go | Element / Matrix.org | Second-generation homeserver; separate state-res impl (v0.13.1 fixed a reset-causing off-by-one). |
| **Conduit / conduwuit / continuwuity** | Rust | community | Lightweight homeservers; consume ruma's state-res crate. |
| **`ruma-state-res`** | Rust | ruma project | A reusable Rust crate (MIT) implementing Matrix state resolution — the closest thing to a "library you could read" for a Rust runtime like Myrhiza. The source lives in the `crates/ruma-state-res/` subdir of the ruma monorepo (historically the `state-res` folder). |
| **erikjohnston/rust-matrix-state** | Rust | author of MSC1442 | The original author's basic Rust implementation accompanying the "Reloaded" proposal. |

(Implementation *versions* not pinned here — they churn weekly and a stale
version number is worse than none. Treat the list as "who has an implementation,"
not "what version.")

## The chain-cover index (the production performance fix)

The textbook algorithm requires computing **full auth chains** and the **auth
difference** ([state-resolution-v2.md](state-resolution-v2.md)) — which naively
means walking backward over potentially huge subgraphs on every resolution. In
large rooms (Matrix HQ, tens of thousands of members, deep DAGs) this is the
dominant cost.

Synapse's mitigation is the **chain cover index**: a precomputed index over the
auth-chain reachability that lets the server answer "is event X in event Y's auth
chain?" and compute auth differences without re-walking the graph. The Synapse
team reported "an order of magnitude speedup for a handful of pathologic cases"
from this and related algorithmic work. The lesson: the *correctness* algorithm
and the *deployable* algorithm are different artifacts; auth-chain reachability
must be indexed, not recomputed.

## Performance traps worth knowing

- **Auth-chain fan-out.** Pulling the full auth chain for a heavily-forked,
  long-lived room is expensive; over-fetching auth chains during v2 state res was
  a real Synapse hot path (PR #6952 "Reduce auth chains fetched during v2 state
  res").
- **The v2.1 conflicted-state subgraph is *more* expensive** than v2's conflicted
  set ([project-hydra-v2.1.md](project-hydra-v2.1.md)) — the implementer's guide
  flags it as "the bulk of this guide" and offers an SCC-based optimisation
  precisely because the naive traversal is costly. Hardening against state resets
  cost CPU.
- **Re-resolution churn.** Every newly-delivered event that merges extremities
  re-triggers resolution. A peer that has been offline and comes back with many
  forks pays a resolution spike — directly analogous to Myrhiza's cold-start
  full-log replay ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
  §4.2/§4.5).

## Implications for Myrhiza

- **`ruma-state-res` is the most directly readable reference** if Myrhiza ever
  power-orders authority events: it is Rust (MIT), it implements the exact
  algorithm this folder describes, and it tracks the v2.1 changes. Cite it, don't
  reinvent.
- **Index auth reachability.** If a Myrhiza RBAC module (§4.5) ever needs to
  power-order an authority subgraph, Matrix's chain-cover index is the precedent
  for *not* recomputing reachability per merge. Myrhiza already indexes events by
  hash / author-chain / topic-membership (§4) — an auth-reachability index would
  be the analogous addition.
- **Multiple independent implementations diverged on the same algorithm.** That
  is the strongest argument for keeping authority ordering *in the kernel* (one
  implementation, like Matrix's per-protocol rules) rather than pushing it into
  per-app `state-apply` WASM where every app re-implements it and re-discovers the
  off-by-one. See [lessons.md](lessons.md) Avoid §1.

## Sources

- <https://crates.io/crates/ruma-state-res> (the published crate, MIT)
- <https://github.com/ruma/ruma> (ruma monorepo; `crates/ruma-state-res/`)
- <https://github.com/erikjohnston/rust-matrix-state>
- <https://github.com/matrix-org/synapse/pull/6952>
- <https://matrix.org/blog/2020/11/20/this-week-in-matrix-2020-11-20/> (chain cover index speedups)
- <https://github.com/matrix-org/dendrite> (v0.13.1 off-by-one fix)
