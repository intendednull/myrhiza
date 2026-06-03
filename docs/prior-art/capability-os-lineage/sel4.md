**Date:** 2026-05-29
**Status:** active
**Subject:** seL4 — the verified capability microkernel; what is proved, on which architectures, the cost, the live frontier

# seL4 (the live anchor)

seL4 is the capability microkernel whose C implementation carries a
**machine-checked proof of functional correctness** — the first such proof for a
complete, general-purpose OS kernel. It is the *live* member of this lineage:
governed by the seL4 Foundation, with verification still actively extending in
2025–2026. Where KeyKOS/EROS/Coyotos supply the *ideas*, seL4 is the system that
*shipped the proof and still ships*.

## What it is

A third-generation **L4-family** microkernel. The kernel does almost nothing
itself: IPC (synchronous endpoints, no kernel-buffered message queues), threads,
address spaces, interrupts, and capability bookkeeping. Everything else —
drivers, filesystems, networking — runs as unprivileged user components, reached
only through capabilities. The TCB is the kernel and nothing more (see
[tcb-and-verification.md](tcb-and-verification.md)).

The capability model is the lineage's, made concrete:

- **CSpace / CNodes.** Each thread's capabilities live in a kernel-protected
  capability space; user code names a capability by an index, never by a forged
  pointer (the partitioned representation from
  [capability-model.md](capability-model.md)).
- **Untyped memory + Retype.** All kernel objects are carved from *untyped*
  memory by an explicit `Retype` operation. There is no hidden kernel heap;
  memory is an explicit, accountable, capability-controlled resource. This is
  what makes the kernel's resource use bounded and verifiable.
- **Capability Derivation Tree (CDT).** The kernel tracks the lineage of every
  capability so that `Revoke` can recursively invalidate a capability and all
  capabilities derived from it — the explicit revocation primitive the
  bearer-token world struggles to match (see
  [capability-tokens/open-problems.md](../capability-tokens/open-problems.md)).
- **Rights + badges.** Capabilities carry rights bits and an optional badge;
  minting can only narrow authority — kernel-enforced attenuation.

## Provenance, governance, license

- Developed by the **Trustworthy Systems** group, originally **NICTA**, then
  **Data61 / CSIRO** (Australia). Lead figures: **Gerwin Klein** (verification),
  **Gernot Heiser** (microkernel), **June Andronick**.
- The first verification was published at **SOSP 2009** (Klein et al.,
  *seL4: Formal Verification of an OS Kernel*); inducted into the **ACM SIGOPS
  Hall of Fame in 2019**.
- The **seL4 Foundation** was established with the **Linux Foundation** hosting
  it, announced **7 April 2020**, to give the project a neutral home and survive
  its founders.
- **License (mixed family):** kernel-level code is **GPL-2.0** (chosen, per the
  project, to encourage reciprocity of commercial investment); user-level code
  is **BSD-2-Clause**, with a syscall exception so user code that calls kernel
  services by normal system calls is *not* derivative of the GPL kernel. This
  folder is kernel-focused, so "seL4 is GPL-2.0" is shorthand for the kernel.

## What is actually proved (and on what)

The proofs are a *stack*, each layer assuming the one below:

| Property | What it means | Status |
|---|---|---|
| **Functional correctness** | C implementation refines the abstract spec; kernel never crashes / does nothing undefined | Arm32, **x86-64**, **RISC-V64**, **AArch64** (AArch64 brought on par per recent work) |
| **Integrity** | Kernel cannot write where the policy forbids | Proved |
| **Confidentiality / information flow** | First source-level information-flow (noninterference) proof for a general-purpose OS kernel | Proved (Arm) |
| **Binary / translation correctness** | The compiled *binary* refines the C — closing the "trust the compiler" gap | ARMv7 and RISC-V64 |
| **WCET** | Verified upper bounds on operation latency | Single-core ARM (ARM11) |

The functional-correctness chain reaches more architectures than the
information-flow and binary proofs; do not assume "verified" means "all
properties on all architectures." Read the verification matrix per property.

One detail in the information-flow row is load-bearing for Myrhiza's
covert-channel story: getting the noninterference proof required **replacing
seL4's scheduler with a fixed partition scheduler**, because the original
scheduler's decisions leaked information across isolated domains via timing
(Murray et al., *seL4: from General Purpose to a Proof of Information Flow
Enforcement*, IEEE S&P 2013). Even the verified kernel had to neuter its
scheduler to get the property — concrete evidence that covert channels live in
the *scheduler*, not the capability model (see
[open-problems.md §1](open-problems.md)).

## The cost, stated honestly

- Kernel source: on the order of **~10,000 lines of C** (the 2009 paper quoted
  8,700 C + ~600 assembler for the original; it has grown since).
- Proof: **~500,000 lines of Isabelle/HOL** — a roughly **50:1
  proof-to-code ratio**. (Trustworthy Systems separately describes "nearly 1
  million lines of proof *steps*"; the two numbers measure different things —
  source lines vs. tactic steps.)
- The team's own development-cost estimate has been quoted around **$400 per
  line of code** — expensive, but reported as *cheaper* than achieving
  comparable assurance by traditional high-assurance engineering.

The lesson is not "verification is free." It is "for a kernel small enough, a
one-time ~50:1 proof investment buys a permanent guarantee, and the verifiability
*forcing function* keeps the kernel small."

## The live frontier (2025–2026)

- **MCS (Mixed-Criticality Scheduling)** — a reworked scheduling model with time
  budgets — is in mainline; verifying its extensions is ongoing.
- **seL4 Microkit** — a higher-level, opinionated SDK/abstraction over raw seL4
  (static component system); the 2.0 line landed with **2.0.0 (2025-03-06)** and
  **2.0.1 (2025-03-21)**, with verification work underway. Later releases have
  since shipped (**2.1.0, 2025-11-26**; **2.2.0, 2026-03-31**) — check the
  release page for current; treat any version cited here as a snapshot.
- **DARPA PROVERS** funds increasing verification automation and scope; the
  annual **seL4 Summit** tracks the moving target.

This is why seL4 is framed as the *anchor*, not a static reference: cite it for
the proof methodology and the verified-kernel existence proof, but check the
current verification matrix before relying on any specific property/architecture
pairing.

## Why it matters to Myrhiza

seL4 is the proof-of-concept for Myrhiza's whole posture: **a small,
capability-mediated TCB can be made trustworthy, and the smallness is *because*
of the capability discipline, not in spite of it.** Two specific borrowings:

1. **TCB minimization as a design driver.** Myrhiza's kernel is the only thing
   apps must trust ([abi.md §8.4](../../specs/2026-05-09-myrhiza-master-design/abi.md));
   every host import is an ABI change to that TCB
   ([CLAUDE.md], "capabilities are the only host surface"). seL4 says: keep that
   surface small enough to reason about, and treat each addition as a cost.
2. **Explicit, kernel-tracked revocation (the CDT) over best-effort
   bearer-token revocation.** Myrhiza's resource handles and submit-tokens are
   kernel-issued and kernel-revoked at instance teardown
   ([abi.md §8.5](../../specs/2026-05-09-myrhiza-master-design/abi.md)) — the same
   "the issuer can always revoke because it tracks derivation" shape, which the
   token-format folder shows is the hard part.

The non-borrowing: Myrhiza does **not** attempt an seL4-grade functional-
correctness proof of its own kernel. Its trust story rests on a small TCB plus
*determinism* and *sandboxing* of guest code, not a from-scratch Isabelle proof
— see [tcb-and-verification.md](tcb-and-verification.md) and
[formal-methods](../formal-methods/README.md).

## Sources

- https://en.wikipedia.org/wiki/SeL4
- https://trustworthy.systems/projects/OLD/seL4-verification/
- https://cgi.cse.unsw.edu.au/~kleing/papers/sosp09.html (Klein et al., SOSP 2009)
- https://www.linuxfoundation.org/press/press-release/sel4-microkernel-optimized-for-security-gets-support-of-linux-foundation
- https://riscv.org/blog/sel4-is-verified-on-risc-v/
- https://trustworthy.systems/projects/microkit/
- https://github.com/seL4/microkit/releases
- https://sel4.systems/Summit/2025/abstracts2025.html
- https://www.ieee-security.org/TC/SP2013/papers/4977a415.pdf (Murray et al., *seL4: from General Purpose to a Proof of Information Flow Enforcement*, IEEE S&P 2013)
