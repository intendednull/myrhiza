**Date:** 2026-05-29
**Status:** active
**Subject:** What peer-verified local-first authority structurally does NOT solve — the residue after the product layer answers the CRDT authority gap

# Open problems

[../crdts/open-problems.md](../crdts/open-problems.md) §2–§3 names the
library-layer gap: CRDTs converge regardless of *who* wrote or *whether the
result is legal*. The products in this folder are the **product-layer** answers.
But even the strongest one (cojson's peer-verified model) leaves hard problems
open. This file enumerates them so a Myrhiza spec author does not assume "adopt
the cojson shape" closes everything.

## 1. Cross-object / global invariants

Per-transaction role checks answer *"may this author write this object?"* They do
**not** answer *"is the converged result globally legal?"* — the bank-account
problem ([../crdts/open-problems.md](../crdts/open-problems.md) §3) survives.
cojson validates a transaction against the author's role, not against an invariant
spanning rows that diverged concurrently. Two writers each individually authorized
can still produce a converged state that violates "sum ≤ budget."

- **Trusted-middlebox products cheat here**: a server can run a real transaction
  and reject. That is *the* reason Paradigm B is attractive
  ([paradigm-contrast.md](paradigm-contrast.md)) and why Jazz 2.0 reaches for a
  trusted tier.
- **Myrhiza's answer**: deps-monotonicity + pre-check
  ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.4) push the verdict
  into `state-apply` evaluated against any state containing the event's `deps` —
  but the spec is explicit that an app violating deps-monotonicity diverges. The
  invariant problem is **constrained, not eliminated**; reservation/escrow CRDT
  modeling is still required for true global bounds.

## 2. Author equivocation / fork

A signed-state model trusts the author to maintain one chain. A malicious author
can sign two transactions at the same position. cojson's session model and
Myrhiza's first-seen-wins ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md)
§4.4.1) both leave peers potentially partitioned on different branches with **no
automatic resolution at v1**. Peer-verified authority does not prevent
self-equivocation; it only makes it detectable. See
[../append-only-log-forks/](../append-only-log-forks/) — the SSB → 2P-BFT-Log
lineage is the equivocation-proof construction.

## 3. Revocation is forward-only and racy

Revoking a member rotates the read key so they cannot read **future** content —
but they keep everything decrypted before revocation, and **concurrent** writes by
the about-to-be-revoked member (made before they learn of revocation) are valid
under the role they held at that time. cojson's "role-at-tx-time" semantics make
this principled but still means **revocation is not instantaneous across a
partition**. This is the same forward-secrecy/healing tension MLS addresses
([../mls/open-problems.md](../mls/open-problems.md)) and that Myrhiza's revocation-seq freshness
([distribution.md](../../specs/2026-05-09-myrhiza-master-design/distribution.md) §10.7) only partially bounds.

## 4. Membership-history growth and key-rotation cost

Every role change and key rotation is more signed state on the log. Long-lived
groups with churn accumulate authority history that every peer replays — the same
monotonic-growth problem as CRDT tombstones
([../crdts/open-problems.md](../crdts/open-problems.md) §4, §6). None of these
products solve coordinated GC of *authority* history. Myrhiza inherits this in the
deferred snapshot/log-truncation work ([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.5).

## 5. Confidentiality from the relay ≠ metadata privacy

Encryption-based reads hide *content* from the relay, but the relay still sees
*who talks to whom, when, and how much* — group membership shape, traffic timing.
cojson does not solve metadata privacy; neither does any product here. Myrhiza
scopes this out too ([capabilities.md](../../specs/2026-05-09-myrhiza-master-design/capabilities.md) §7.5;
see [../anonymity-transports/](../anonymity-transports/) for the surface that
would).

## 6. Declarative authorization does not compose cleanly

Rocicorp **deprecated** its declarative permission rules in favor of "just write
server code" ([zero-rocicorp.md](zero-rocicorp.md)) — a candid admission that
compact declarative cross-object authorization hits expressiveness walls. A
Myrhiza `myrhiza-permission-rbac` module that tries to be fully declarative will
hit the same wall; the escape hatch (arbitrary code in `state-apply`) is already
there, but then the authorization logic is as auditable as the app.

## 7. Schema migration over encrypted / authorized state

Garden's own notes: *"e2ee makes automatic migration tricky"*
([maturity-and-the-2.0-pivot.md](maturity-and-the-2.0-pivot.md)). When content is
encrypted and authority is on-log, you cannot migrate it with a server-side
batch job. This compounds [../crdts/open-problems.md](../crdts/open-problems.md)
§1/§5 and lands on Myrhiza's schema-version ABI
([../schema-evolution/](../schema-evolution/)).

## Sources

- [../crdts/open-problems.md](../crdts/open-problems.md)
- Per-product files in this folder.
- Garden `specs/concerns.md`: <https://github.com/garden-co/jazz>
- Zero deprecated permissions: <https://zero.rocicorp.dev/docs/deprecated/rls-permissions>
- Myrhiza spec: [convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.4–4.5,
  [distribution.md](../../specs/2026-05-09-myrhiza-master-design/distribution.md) §10.7.
