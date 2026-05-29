**Date:** 2026-05-29
**Status:** active
**Subject:** EROS — clean-room KeyKOS, the constructor, and the first formal confinement proof (Shapiro)

# EROS

EROS (Extremely Reliable Operating System) is a **clean-room reconstruction of
KeyKOS** for commodity (x86) hardware, led by Jonathan Shapiro. Its lasting
contributions are (1) the **constructor**, a sharpened factory; and (2) the
**first formal verification of a capability confinement mechanism** (2000),
which settled a long-running theoretical dispute about whether capability
systems can enforce strong information-flow policies at all.

## Provenance and dates

- Begun in **1991** — the year Key Logic closed — as a clean-room redo of
  KeyKOS, originally at the **University of Pennsylvania**, where it became
  Shapiro's dissertation work. Moved with Shapiro to **Johns Hopkins
  University** in 2000. Funded by DARPA and the Air Force Research Laboratory.
- **Pure** capability system: authority is conveyed exclusively by capabilities,
  down to individual-page granularity. Inherits KeyKOS's automatic persistence
  and periodic checkpoint.
- Purely a research OS — **never deployed in production**.
- **Development ended in 2005**, in favor of two successors:
  **CapROS** (Charles Landau's open-source, commercially-oriented continuation
  of the EROS codebase) and **Coyotos** (Shapiro's from-scratch successor — see
  [coyotos.md](coyotos.md)).

## The constructor

The EROS **constructor** is the operational heart of EROS confinement and the
direct descendant of the KeyKOS factory ([keykos.md](keykos.md)). A constructor
is a trusted agent that builds a new program instance and can **certify to a
prospective client that the resulting process is confined** — i.e., it holds no
capability that could leak the client's data, except through capabilities the
client explicitly hands in (the authorized "holes"). The client queries the
constructor *before* entrusting it with anything sensitive; a "yes, confined"
answer is a checkable guarantee, not a promise.

This is the constructive primitive behind mutually-suspicious composition: a
client can safely run untrusted code, and a vendor can ship proprietary code to
an untrusted client, with the kernel + constructor mediating. It is the OS-level
analogue of a *membrane* in the language-side ocap world
([spritely-ocapn/capabilities.md](../spritely-ocapn/capabilities.md)).

## The 2000 confinement proof

Shapiro and Sam Weber, *Verifying the EROS Confinement Mechanism*, IEEE
Symposium on Security and Privacy (Oakland) 2000, pp. 166–176. They give a
formal statement of the confinement requirement, build a model of the
architecture's protection state and operational semantics, and prove that
architectures fitting the model enforce confinement.

Its significance is partly theoretical history. Earlier work (notably W.E.
Boebert, 1984) had argued that an *unmodified* capability machine cannot enforce
the **\*-property** (the Bell–LaPadula no-write-down rule that blocks
high-clearance data from flowing to low-clearance subjects), implying
capabilities were unfit for mandatory access control. Shapiro–Weber's result is
the rebuttal: a capability system **can** enforce the \*-property and higher-level
mandatory policies *provided a confinement mechanism exists* — which the
constructor supplies. See
[confinement-and-take-grant.md](confinement-and-take-grant.md) for the full
debate.

Note the scope honestly: this is a proof about the *confinement mechanism and
protection model*, not a KeyKOS-grade whole-kernel functional-correctness proof.
That came later, and on a different kernel — [seL4](sel4.md). EROS aimed at it
but did not reach it; reaching it was an explicit goal of Coyotos.

## The synchronous-IPC flaw

EROS used synchronous IPC. Around 2003 the project identified a class of
vulnerabilities arising from synchronous-IPC semantics (a server could be made
to block, enabling denial-of-service and confused-interaction patterns). Fixing
this structurally was a primary motivation for the [Coyotos](coyotos.md)
redesign rather than a patch to EROS. This is a useful cautionary data point:
the *protection model* was proven sound, yet an *interaction-protocol* choice
(sync vs async IPC) sitting next to it still produced exploitable behavior —
verification of one layer does not immunize the layer beside it.

## Why it matters to Myrhiza

The constructor is the cleanest statement of "**verify confinement before you
trust**," which is exactly the posture Myrhiza takes toward third-party modules:
the manifest-intersection check
([capabilities.md §7.2](../../specs/2026-05-09-myrhiza-master-design/capabilities.md))
is a static, link-time confinement check — a module *cannot* acquire authority
the app did not declare, and the kernel proves this at instantiation. EROS shows
the upside (composability of mutually-suspicious code) and the trap (the
sync-IPC flaw): get the *interaction protocol* wrong and a sound protection
model still leaks. Myrhiza's submit-and-poll async ABI
([abi.md §8.5](../../specs/2026-05-09-myrhiza-master-design/abi.md)) is, in part,
declining EROS's synchronous-IPC mistake.

## Sources

- https://en.wikipedia.org/wiki/EROS_(microkernel)
- https://flint.cs.yale.edu/cs428/doc/eros-verify.pdf (Shapiro & Weber, *Verifying the EROS Confinement Mechanism*, IEEE S&P 2000)
- https://ieeexplore.ieee.org/document/848454/
- http://thomas.enix.org/pub/rmll2005/rmll2005-shapiro1.pdf (Shapiro, *A Look at the EROS Operating System*, 2005)
- https://archiveos.org/eros/
