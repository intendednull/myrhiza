**Date:** 2026-05-29
**Status:** active
**Subject:** What the capability-OS lineage structurally does NOT solve

# Open problems

What forty years of capability-OS work leaves *un*solved. These are not
implementation gaps; they are limits the paradigm's own literature is honest
about. Read this before treating any capability gate as a complete defense.

## 1. Covert and side channels

The capability model controls **de-jure authority** — who may *hold* a
capability and act through authorized channels. It does **not** control
information leakage over *unauthorized* channels: CPU timing, memory pressure,
cache state, disk/lock contention, power. Lampson named this in 1973
([confinement-and-take-grant.md](confinement-and-take-grant.md)); seL4's
functional-correctness proof explicitly does not cover it
([tcb-and-verification.md](tcb-and-verification.md)). Two cooperating components
that should be isolated can still signal across the boundary by modulating shared
resources. Closing covert channels requires timing/resource isolation in the
*scheduler*, which is a separate, expensive, and never-complete project. The
sharpest evidence that this is a scheduler problem, not a capability-model
problem: seL4's information-flow proof required **replacing the kernel scheduler
with fixed partition scheduling**, because the original scheduler leaked across
domains via timing ([sel4.md](sel4.md); Murray et al., IEEE S&P 2013). Even the
verified kernel had to neuter its scheduler to win the property.
**Myrhiza inherits this gap** — it is correctly scoped out in the spec's
side-channel scope clarification
([determinism.md §5.1](../../specs/2026-05-09-myrhiza-master-design/determinism.md),
which explicitly declines to cover capability-gate dispatch timing and
cache-timing leaks; capability mediation itself only controls de-jure authority,
[capabilities.md §7.5](../../specs/2026-05-09-myrhiza-master-design/capabilities.md)).

## 2. The confined deputy is not the confined *user*

A capability system stops the *confused deputy* ([capability-model.md](capability-model.md))
and bounds what code can do. It does **not** stop a human who knowingly grants a
malicious program full authority. KeyKOS, EROS, seL4, and Myrhiza alike all bottom
out at "the user authorized it." Myrhiza's install-time capability summary
([capabilities.md §7.1](../../specs/2026-05-09-myrhiza-master-design/capabilities.md))
is a *review* surface, not a guarantee — §7.5 lists "a user who explicitly grants
a malicious app full capabilities" as out of scope. No capability mechanism
defends against an authorizing user.

## 3. Revocation at scale

The lineage's revocation primitive (seL4's [CDT](sel4.md), Genode's
destroy-object-revokes-derived) works because the kernel *tracks capability
derivation* on one machine. It is local, synchronous, and complete. The moment
authority crosses a network — sturdyrefs, bearer tokens, delegated grants across
peers — that derivation tree fragments, and revocation degrades to the hard,
partial story the [capability-tokens](../capability-tokens/open-problems.md) and
[spritely-ocapn](../spritely-ocapn/open-problems.md) folders document (mass
revocation, key rotation, propagation latency). Myrhiza's *in-process* handle
revocation is clean (instance teardown); its *cross-peer* authority revocation
inherits the distributed-systems version of this problem, partially handled by
the revocation flow in
[distribution.md §10.7](../../specs/2026-05-09-myrhiza-master-design/distribution.md).

## 4. Naming and discovery

Pure capability systems are *introduce-then-invoke*: you can only invoke an
object whose capability you were handed. They give you no native answer to "find
a service / a peer / a stranger I have never met." KeyKOS and seL4 punt this to
higher layers; OCapN names objects, not principals
([spritely-ocapn/open-problems.md](../spritely-ocapn/open-problems.md)). Myrhiza
must layer discovery on top (gossip / DHT / topic bootstrap), and must not assume
the capability layer provides it.

## 5. The verification cliff and its assumptions

A verified kernel is only verified relative to its assumption list (compiler,
assembly, hardware-as-modeled, no timing). And verification cost grows
super-linearly with TCB size, so the technique only works on a kernel kept
deliberately tiny ([tcb-and-verification.md](tcb-and-verification.md)). seL4's
proof does not extend to the user components that make up a *useful* system — a
verified microkernel hosting buggy unverified drivers is still exploitable
through what those drivers' capabilities reach. Verification bounds the *kernel*,
not the *system*.

## 6. Project fragility vs idea durability

The *ideas* persist (every system here re-implements the same primitives). The
*projects* are fragile: GNOSIS→KeyKOS rode on a company that closed (Key Logic,
1991); EROS→Coyotos rode on one researcher and stopped when Shapiro left for
Microsoft (2009–2010). Only seL4 survived its founders, and only by becoming a
**Foundation**. This is a governance lesson, not a technical one
([coyotos.md](coyotos.md), [lessons.md](lessons.md)).

## 7. Performance and the IPC tax

Capability mediation means *every* cross-component interaction is a kernel-
brokered, typed, bounded call — not a function call. seL4 spent years making IPC
fast precisely because the model puts IPC on the hot path. Myrhiza pays the
analogue: every cross-component call has measurable cost (instantiation, ABI
translation, gate check), which is why the spec mandates **coarse-grained
interfaces** and forbids tight inner-loop callbacks across boundaries
([abi.md §8.6](../../specs/2026-05-09-myrhiza-master-design/abi.md)). The
capability tax is real; the mitigation is granularity, not avoidance.

## Sources

- https://dl.acm.org/doi/10.1145/362375.362389 (Lampson, 1973)
- https://trustworthy.systems/projects/OLD/seL4-verification/
- https://www.ieee-security.org/TC/SP2013/papers/4977a415.pdf (Murray et al., IEEE S&P 2013 — info-flow proof + fixed partition scheduler)
- https://en.wikipedia.org/wiki/Coyotos
- https://flint.cs.yale.edu/cs428/doc/eros-verify.pdf
- ../../specs/2026-05-09-myrhiza-master-design/determinism.md §5.1 (side-channel scope clarification)
