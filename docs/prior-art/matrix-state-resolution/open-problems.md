**Date:** 2026-05-29
**Status:** active
**Subject:** What Matrix state resolution structurally does NOT solve

# Open problems

What state resolution — even at v2.1 — structurally cannot do. These are the
limits a Myrhiza reader must carry forward, because Myrhiza's design space differs
from Matrix's (no trusted homeserver, P2P, per-app verdict function).

## 1. State resets are *mitigated, not eliminated*

v2.1 "protects against various classes of 'state resets'" — note **classes**, not
*all*. The eight-year incident trail ([state-reset-hazard.md](state-reset-hazard.md))
shows every prior "fix" left a residual head. The honest framing in the project's
own name (*Hydra*): this is an arms race against crafted forks, not a closed
problem. A deterministic re-derivation of authority from a forkable DAG appears to
be *intrinsically* vulnerable to some reorder-induced reset; the work is shrinking
the attack surface, not closing it.

## 2. Pre-v12 rooms keep the old hazard forever

Per-room versioning ([room-versions.md](room-versions.md)) means the v2.1 fix only
protects rooms *created at* v12. The enormous long tail of existing v1–v11 rooms
keeps the old algorithm. There is no in-place upgrade — only the
tombstone-and-recreate dance, which fragments history. Myrhiza's kernel-global
pinning avoids this *if* it accepts ordering changes as kernel-breaking.

## 3. The homeserver is a trusted aggregator — Matrix's whole trust model

State resolution runs **on homeservers**, and a homeserver is implicitly trusted
by its local users (it can lie about state to its own clients, soft-fail
selectively, withhold events). Matrix's threat model is "a *malicious participating
homeserver*" — a peer, but a server-class peer. **Myrhiza has no homeserver.**
Every device runs `state-apply` itself. So Matrix's "trusted-for-its-own-users
aggregator" assumption *does not transfer*; Myrhiza's resolution must be robust
against every *device* being adversarial, a strictly harder bar. Do not import
Matrix's trust simplifications.

## 4. `origin_server_ts` is attacker-controllable and load-bearing in the tie-break

The secondary tie-break is `origin_server_ts` — the *sending server's* wall clock,
which a malicious server sets freely. It only bites after the power-level key, so
it can't directly cause an authority reset, but it can bias ordering of ordinary
state. Matrix tolerates this because the power key dominates for authority. Myrhiza
correctly **excludes HLC timestamps from ordering entirely**
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.1) — a *stronger* position than Matrix's. Don't regress to timestamp tie-breaks.

## 5. No semantic-conflict resolution — only structural

State resolution picks *which event wins* a `(type, state_key)` slot. It does
**not** merge intent: if two users concurrently rename a room, one name is chosen
and the other is *lost*, silently. There is no CRDT-style merge, no
three-way-merge surfacing. For authority this is correct (you want one winner);
for content it is data loss. Apps wanting merge semantics get nothing from state
resolution — exactly Myrhiza's §4.9 "embed a CRDT inside your own state-apply"
position. Cross-ref [`../crdts/`](../crdts/).

## 6. Scale: deep DAGs and heavy forks are expensive

Resolution cost grows with auth-chain depth and fork width
([implementations.md](implementations.md)). Large public rooms are the pathologic
case; the chain-cover index exists because the naive algorithm doesn't scale. The
v2.1 conflicted-state subgraph is *more* expensive than v2. Authority correctness
and resolution cost trade off directly.

## 7. No recovery story for divergence between honest peers

If two implementations diverge (the Dendrite off-by-one), the spec offers no
reconciliation protocol — the rooms simply disagree until a human notices. There
is no built-in cross-peer drift *detection* either (Matrix has nothing like
Croquet's TUTTI vote, [`../croquet/`](../croquet/)). Myrhiza's §4.7
TUTTI-shaped drift detection is *ahead* of Matrix here — Matrix found resets via
user reports and GitHub issues, not an automated check. Validates Myrhiza's choice
to build drift detection in.

## Implications for Myrhiza

- The deepest lesson is **#1 + #3 together**: a deterministic DAG-replay system
  with a *harder* trust model than Matrix's (every device adversarial, no trusted
  aggregator) should expect the state-reset hazard to be *at least as bad* and
  plan accordingly — keep authority simple, power-order it if it must live in the
  DAG, and detect drift ([lessons.md](lessons.md)).
- Matrix's gaps in **drift detection (#7)** and its **timestamp tie-break (#4)**
  are places Myrhiza is already *ahead* (§4.7 TUTTI, §4.1 no-timestamp-ordering) —
  worth recording so those choices aren't second-guessed.

## Sources

- <https://matrix.org/blog/2025/08/project-hydra-improving-state-res/>
- <https://github.com/matrix-org/synapse/issues/15987>
- <https://www.tenable.com/cve/CVE-2025-49090>
- <https://matrix.org/docs/older/stateres-v2/>
