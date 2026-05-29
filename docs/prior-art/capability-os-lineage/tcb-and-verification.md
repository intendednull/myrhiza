**Date:** 2026-05-29
**Status:** active
**Subject:** TCB minimization as the through-line; what verification can and cannot promise

# TCB minimization and verification

The single thread running through this whole lineage is **Trusted Computing Base
(TCB) minimization**: shrink the code everything else must trust until it is
small enough to *reason about* — and, eventually, to *prove correct*. Capability
discipline and TCB minimization are mutually reinforcing: removing ambient
authority forces functionality out of the kernel into unprivileged components,
which shrinks the TCB, which makes verification feasible, which is only worth
attempting because the TCB is small.

## The argument, stated plainly

- In a monolithic OS, the TCB is the whole kernel — millions of lines, every
  driver, the whole syscall surface. Any bug anywhere is a potential total
  compromise.
- A **microkernel** moves drivers, filesystems, and networking into
  unprivileged user components reached only through capabilities. The TCB
  collapses to the kernel: IPC, scheduling, address spaces, capability
  bookkeeping. [seL4](sel4.md) is ~10,000 lines of C.
- A bug in a user component now compromises only what *that component's
  capabilities* reach — confinement ([confinement-and-take-grant.md](confinement-and-take-grant.md))
  bounds the blast radius.

This is the same move Myrhiza makes one layer up. The kernel owns I/O, keys,
network, and storage; apps are WASM components that can touch nothing directly.
The host-import set *is* the TCB surface, and the spec's axiom —
**"capabilities are the only host surface"** ([CLAUDE.md]) — is exactly "keep the
TCB boundary narrow and explicit." Each new host import is an ABI change to that
TCB and is treated as a deliberate cost
([abi.md §8](../../specs/2026-05-09-myrhiza-master-design/abi.md)).

## What verification promises — and the assumptions underneath

seL4's functional-correctness proof is the gold standard, but its *power comes
from being explicit about what it does not cover.* The proof shows the C
implementation refines the abstract spec. It assumes the correctness of:

- the **C compiler** (closed on ARMv7/RISC-V64 by the separate *binary
  correctness* / translation-validation proof, which removes this assumption on
  those targets);
- the small amount of **hand-written assembly** and the **boot code**;
- the **hardware** behaving as modeled — including, crucially, the parts the
  model abstracts away: caches, TLBs, speculative execution, and **timing**.

That last bullet is the durable caveat. A functional-correctness proof says
nothing about **covert timing channels** (Lampson, 1973) or microarchitectural
side channels (Spectre-class). "Proved correct" means "matches the spec," and the
spec is a functional one. The team's own current work on "strengthening
assumptions involving hardware behaviour" is precisely about chipping at this
boundary. **Verification is a contract whose value equals the honesty of its
assumption list.**

## The verification cliff

[Coyotos](coyotos.md) tried to reach full verification by inventing a verifiable
language (BitC) first; it never shipped. seL4 reached it by keeping C and paying
a one-time ~50:1 proof cost. The cliff is real:

- Verification scales with TCB size super-linearly. ~10K lines took ~500K lines
  of proof and person-decades. Doubling the kernel does **not** double the proof
  cost.
- Therefore the *only* affordable path to a verified system is to make the thing
  you verify *tiny first*. Capability discipline is the tool that makes it tiny.

## Where Myrhiza sits

Myrhiza is **not** pursuing an seL4-grade proof of its own kernel, and that is a
deliberate, defensible choice given the cliff:

- Its trust story rests on **(a)** a small, explicit capability TCB (the
  borrowed seL4 posture), **(b)** running guest code in an existing,
  widely-audited sandbox (Wasmtime / the Component Model — see
  [wasm-component-model](../wasm-component-model/README.md)), and **(c)**
  **determinism** of `state-apply` as a *checkable* property rather than a proven
  one ([determinism.md](../../specs/2026-05-09-myrhiza-master-design/determinism.md)).
- Where Myrhiza *does* reach for formal methods, it is targeted and tool-based —
  see [formal-methods](../formal-methods/README.md) (Kani, Loom, TLA+) — not a
  monolithic whole-kernel refinement proof.

The lesson seL4 hands Myrhiza is not "go verify your kernel." It is: **keep the
TCB small enough that you *could*, and treat the host-import surface as the
thing whose growth you must justify.** A small TCB is valuable even un-proved —
it is auditable, it bounds blast radius, and it leaves the verification door
open. A large one forecloses the option permanently.

## Sources

- https://en.wikipedia.org/wiki/SeL4
- https://trustworthy.systems/projects/OLD/seL4-verification/
- https://sel4.systems/About/seL4-whitepaper.pdf
- https://dl.acm.org/doi/10.1145/362375.362389 (Lampson, CACM 1973 — covert channels)
