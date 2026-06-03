**Date:** 2026-05-29
**Status:** active
**Subject:** The theory — Lampson confinement (1973), Lipton–Snyder take-grant (1977), the *-property debate

# Confinement and take-grant

The capability-OS lineage is not just engineering; it rests on a small body of
protection theory that tells you *what is decidable* about who can end up with
what authority. Three results matter, and they are why "confinement" — not
"capabilities" — is the load-bearing word in this folder.

## Lampson's confinement problem (1973)

Butler Lampson, *A Note on the Confinement Problem*, Communications of the ACM,
1973. The question: how do you run an untrusted program with some data **and
guarantee it cannot leak that data** to an unauthorized party? Lampson's lasting
contribution is twofold:

1. He framed confinement as the central problem of safe composition: a confined
   program may compute, but it must not *communicate* outward beyond authorized
   channels.
2. He introduced the **covert channel** — and showed it is the reason
   confinement is *hard*. A confined program denied all legitimate output can
   still leak bits by modulating something observable: CPU time, memory
   pressure, disk usage, even a shared lock's contention. You can block the
   *authorized* channels by construction; you cannot block all the
   *unauthorized* ones without controlling timing and resource accounting too.

Every system in this folder either solves confinement *over the authorized
channels* (KeyKOS factory, EROS constructor) or explicitly notes that covert
channels remain — see [open-problems.md](open-problems.md).

## Take-grant (Lipton–Snyder, 1977)

Lipton and Snyder, *A Linear Time Algorithm for Deciding Subject Security*,
1977. Take-grant is a graph model of how authority propagates: subjects and
objects are nodes; edges are rights; two special rights, **take** and **grant**,
let one node pull a capability from another or push one to another. The
celebrated result: in this model, the question **"can subject A *possibly*
acquire right r over object B?"** is **decidable in linear time**.

This is the good news that makes capability systems analyzable: unlike the
general access-matrix safety problem (Harrison–Ruzzo–Ullman, 1976, which is
**undecidable**), the take-grant restriction buys you a fast, exact answer to
"who can ever reach what." Capability propagation is tractable *precisely because*
authority moves only by explicit take/grant along edges — the same property that
makes [the capability model](capability-model.md) reasoned-about by hand.

The catch take-grant itself surfaces: **de-facto** vs **de-jure** authority.
Take-grant decides who can come to *hold a capability* (de-jure). It does **not**
by itself decide who can come to *know information* — the "conspiracy" / de-facto
flow analyses extend the model, and they intersect Lampson's covert-channel
problem. Holding-authority and learning-information are different questions; a
capability system gives you a clean answer to the first and only a partial one to
the second.

## The *-property debate (and why EROS's proof mattered)

A long-standing objection (W.E. Boebert, 1984) held that **an unmodified
capability machine cannot enforce the \*-property** — the Bell–LaPadula
"no write-down" rule that forbids high-secrecy data from flowing into
low-secrecy containers. If true, capabilities would be unfit for mandatory
(lattice / MLS) security and confined to discretionary use only.

[EROS's 2000 confinement proof](eros.md) (Shapiro & Weber) is the rebuttal: a
capability system **can** enforce the \*-property and higher-level mandatory
policies **provided a confinement mechanism exists** — and the EROS constructor
*is* such a mechanism. The trick is that confinement lets you guarantee a newly
built subject holds *no* outward-leaking capability beyond authorized holes, so
you can place it safely in the lattice. This closed the theoretical gap that had
hung over capability systems for ~15 years.

The honest residue: the proof is about *de-jure* authority flow over authorized
channels. Boebert-style covert channels (timing, resource exhaustion) sit
outside it — they are Lampson's problem, and no capability mechanism alone
closes them. See [open-problems.md](open-problems.md).

## Why this matters to Myrhiza

Myrhiza's capability gating is a *de-jure* authority story, and the theory tells
you exactly what that buys and what it doesn't:

- **What it buys:** the manifest intersection `M_effective = A_ambient ∩ M_required`
  ([capabilities.md §7.2](../../specs/2026-05-09-myrhiza-master-design/capabilities.md))
  is a take-grant-style propagation bound, computed *statically* at link time.
  Because Myrhiza forbids ambient authority and forbids modules amplifying
  beyond their declaration, "can module M ever reach capability c?" is decidable
  by inspecting the manifest graph — the take-grant tractability property,
  inherited.
- **What it does *not* buy:** confinement over covert channels. Two Myrhiza
  components on one peer share a CPU, a memory budget, and a clock. A malicious
  pair can signal across the capability boundary via timing or resource
  pressure. The spec already scopes this out
  ([determinism.md §5.1](../../specs/2026-05-09-myrhiza-master-design/determinism.md)
  side-channel scope clarification explicitly declines capability-gate dispatch
  timing and cache-timing leaks; metadata correlation is a separate accepted risk
  in [networking.md §11.4](../../specs/2026-05-09-myrhiza-master-design/networking.md)).
  Forty years of this literature says that is the *correct* thing to scope out at
  this layer — closing covert channels requires timing/resource isolation that
  belongs to the kernel scheduler, not the capability model. Name the gap; don't
  pretend the gate closes it.

## Sources

- https://dl.acm.org/doi/10.1145/362375.362389 (Lampson, *A Note on the Confinement Problem*, CACM 1973)
- https://www.pls-lab.org/en/Confinement
- https://handwiki.org/wiki/Take-grant_protection_model
- https://flint.cs.yale.edu/cs428/doc/eros-verify.pdf (Shapiro & Weber, IEEE S&P 2000)
- https://www.researchgate.net/publication/2646629_Conspiracy_and_Information_Flow_in_the_Take-Grant_Protection_Model
- ../../specs/2026-05-09-myrhiza-master-design/determinism.md §5.1 (side-channel scope clarification)
