**Date:** 2026-05-29
**Status:** active
**Subject:** Coyotos — the EROS successor that aimed at full verification and went dormant

# Coyotos

Coyotos is Jonathan Shapiro's capability-microkernel successor to
[EROS](eros.md), developed at The EROS Group, LLC. Its stated ambition was to
become **the first formally verified general-purpose operating system** — and
the path it chose to get there (verify the kernel by writing it in a new,
verifiable systems language) is precisely the contrast that makes
[seL4](sel4.md), which reached verification a different way, the live anchor of
this folder. Coyotos is **dormant** — a cautionary tale about the cost of the
"verify via a new language" route.

## What it changed from EROS

- **Fixed the synchronous-IPC flaw.** Coyotos redesigned IPC to remove the
  blocking/DoS vulnerability class that EROS discovered around 2003 (see
  [eros.md](eros.md)). The redesign was structural, not a patch — one of the
  main reasons Coyotos was a new kernel rather than an EROS revision.
- **Refined the capability/object model** inherited from KeyKOS → EROS, with an
  eye to making the kernel small and regular enough to verify.

## BitC — verify the implementation by changing the language

The defining bet: Coyotos would be written in **BitC**, a new systems
programming language (with a compiler, **BitCC**) designed by Shapiro to be both
low-level enough for a kernel *and* amenable to formal reasoning — roughly, "C's
control over representation with a semantics you can prove things about." The
plan was to specify the kernel and prove the BitC implementation met the spec.

This is the philosophical fork in the road:

- **Coyotos:** invent a verifiable language, then write a verifiable kernel in
  it. Verification effort is front-loaded into language/compiler design.
- **[seL4](sel4.md):** keep writing the kernel in C, generate it from a
  Haskell prototype, and prove the *existing C* matches the spec with Isabelle/
  HOL. Verification effort is back-loaded into a one-off proof.

seL4's route shipped a complete, machine-checked proof in 2009. Coyotos's route
did not: by **March 2010** the main effort had shifted to the BitC *language*
itself, and the last change to the Coyotos source dates to **June 2010**.
Shapiro had **joined Microsoft in 2009** (working on the Midori/Singularity-
adjacent research line), which effectively ended his bandwidth for the project.
BitC was later acknowledged by Shapiro as having unresolved design problems and
was set aside.

## Status (2026)

Dormant. No releases or commits in well over a decade. Coyotos is studied for
its *ideas* (the IPC redesign, the BitC verification strategy), not run.

## Why it matters to Myrhiza

Two lessons, both negative-space:

1. **Don't make verification depend on inventing infrastructure.** Coyotos
   coupled "verified kernel" to "first finish a new language + compiler," and
   the dependency sank the schedule. Myrhiza's analogue would be coupling a
   correctness goal to an un-shipped tool. Myrhiza instead leans on *existing*
   verified/verifiable substrates (Wasmtime, the Component Model) and on
   determinism-as-a-property
   ([determinism.md](../../specs/2026-05-09-myrhiza-master-design/determinism.md))
   rather than a from-scratch proof of its own kernel — see
   [tcb-and-verification.md](tcb-and-verification.md) and
   [formal-methods](../formal-methods/README.md).
2. **A single maintainer is a single point of failure.** EROS → Coyotos rode on
   Shapiro; when he moved to Microsoft, the line stopped. seL4 survived its
   founders by becoming a Foundation. Capability-OS *ideas* are durable; specific
   *projects* are fragile.

## Sources

- https://en.wikipedia.org/wiki/Coyotos
- https://archiveos.org/coyotos/
- https://www.osnews.com/story/21262/jonathan-shapiro-of-coyotos-bitc-joins-microsoft/
- http://thomas.enix.org/pub/rmll2005/rmll2005-shapiro1.pdf
