**Date:** 2026-05-29
**Status:** active
**Subject:** Capability-based OS lineage — KeyKOS → EROS → Coyotos → seL4, plus the deployed exemplars (Genode/Sculpt, Capsicum, Fuchsia/Zircon, gVisor)

# Capability-based OS lineage

The kernel/OS-side object-capability tradition: a forty-year line of operating
systems where **all authority is conveyed by unforgeable, kernel-mediated
capabilities** and the kernel is the single broker of every privileged
operation. This is the direct intellectual root of Myrhiza's "capabilities are
the only host surface" axiom — the OS-design companion to the *language*-side
ocap folders ([spritely-ocapn](../spritely-ocapn/README.md),
[agoric-endo](../agoric-endo/README.md)) and the WASM-platform folders
([wasmcloud](../wasmcloud/README.md), [spin](../spin/README.md),
[wasm-component-model](../wasm-component-model/README.md)), which only cite this
lineage as ancestry.

**seL4 is the live anchor** — a capability microkernel with a machine-checked
proof of functional correctness, hosted under the Linux-Foundation-incubated
seL4 Foundation, with verification still actively extending in 2025–2026.
**KeyKOS / EROS / Coyotos are historical-but-load-bearing** — the Hardy/Shapiro
lineage that established confinement, the constructor/factory, and persistence-
plus-capabilities. They are durable grounding, not churn: cite them for the
*ideas*, cite seL4 (and the exemplars) for *what ships*.

## Key facts

| System | Era | Provenance | Status (2026) | What it contributed |
|---|---|---|---|---|
| [**KeyKOS**](keykos.md) | mid-1970s–1980s | Tymshare (as GNOSIS) → Key Logic; Norman Hardy | Historical; production on IBM S/370 from 1983 | Pure caps + persistent single-level store + system-wide checkpoint; the **factory** (confinement) |
| [**EROS**](eros.md) | 1991–2005 | UPenn → Johns Hopkins; Jonathan Shapiro | Research; ended 2005 → CapROS, Coyotos | Clean-room KeyKOS; the **constructor**; *formally verified confinement* (2000) |
| [**Coyotos**](coyotos.md) | 2005–~2010 | The EROS Group; Shapiro | Dormant since ~2010 | Fix EROS's synchronous-IPC flaws; aimed at full verification via BitC |
| [**seL4**](sel4.md) | 2006– | NICTA/Data61/CSIRO → seL4 Foundation; Klein, Heiser, Andronick | **Active**; verified on Arm/x64/RISC-V64 | First machine-checked functional-correctness proof of a general-purpose OS kernel |
| [**Exemplars**](exemplars.md) | 2010– | Genode, FreeBSD, Google ×2 | Genode/Sculpt **ships 2026**; Capsicum, Zircon, gVisor in production | Recursive delegation; cap-mode on POSIX; handle-objects; application-kernel |

## Contents

Each file is independent and skimmable standalone.

**The lineage (historical-but-load-bearing)**
- [**keykos.md**](keykos.md) — pure capabilities, persistent single-level store, checkpoint, the factory.
- [**eros.md**](eros.md) — clean-room KeyKOS; the constructor; the 2000 confinement proof.
- [**coyotos.md**](coyotos.md) — the EROS successor that aimed at full verification and went dormant.

**The live anchor**
- [**sel4.md**](sel4.md) — verified capability microkernel; what is proved, on which architectures, the cost, the active frontier.

**Deployed exemplars (one file)**
- [**exemplars.md**](exemplars.md) — Genode/Sculpt OS (recursive parent-delegated authority), FreeBSD Capsicum, Fuchsia/Zircon handle-objects, gVisor application-kernel.

**Cross-cutting theory**
- [**capability-model.md**](capability-model.md) — what "unforgeable kernel-mediated authority" actually means: designation = authority, no ambient authority, the confused deputy.
- [**confinement-and-take-grant.md**](confinement-and-take-grant.md) — Lampson's confinement (1973), Lipton–Snyder take-grant (1977), the *-property debate, and why confinement is the load-bearing theorem of this lineage.
- [**tcb-and-verification.md**](tcb-and-verification.md) — TCB minimization as the through-line; what verification can and cannot promise.

**Project lens**
- [**open-problems.md**](open-problems.md) — what this lineage structurally does *not* solve (covert channels, revocation at scale, naming, the verification cliff).
- [**lessons.md**](lessons.md) — **the consult-this-when-designing file.** validates / avoid / borrow for Myrhiza.
- [**glossary.md**](glossary.md) — factory, constructor, untyped memory, CDT, sturdyref vs handle, etc.

## Canonical reading order

1. [capability-model.md](capability-model.md) — the shared vocabulary.
2. [keykos.md](keykos.md) → [eros.md](eros.md) → [coyotos.md](coyotos.md) — the historical line.
3. [sel4.md](sel4.md) — the live anchor and what verification bought.
4. [confinement-and-take-grant.md](confinement-and-take-grant.md) — the theory that ties it together.
5. [exemplars.md](exemplars.md) — what shipped, and how the ideas got watered down to ship.
6. [lessons.md](lessons.md) — the Myrhiza decision surface.

## How to use this prior-art doc

Designing anything that touches the four-layer capability gating model
([capabilities.md §7](../../specs/2026-05-09-myrhiza-master-design/capabilities.md),
`M_effective = A_ambient ∩ M_required`), the kernel-is-the-call-broker TCB boundary
([abi.md §8](../../specs/2026-05-09-myrhiza-master-design/abi.md)), or any
revisiting of the capabilities-are-the-only-host-surface axiom? Start with
[lessons.md](lessons.md), then drop into [capability-model.md](capability-model.md)
and [confinement-and-take-grant.md](confinement-and-take-grant.md) for the theory
under the spec.

## Framing disclosure

These docs are written from **Myrhiza's current design stance**, not as a neutral
catalog. That stance is specific: **capability-mediated** (capabilities are the
only host surface), **P2P-only** (no central server; per-author Merkle event
DAGs), **Component-Model-on-Wasmtime** for the guest sandbox, and
**event-log-replay `state-apply`** as the determinism substrate. Every "why it
matters to Myrhiza" section and all of [lessons.md](lessons.md) reads the
capability-OS lineage *through that lens* — asking what Myrhiza should borrow,
validate, or avoid — rather than asking whether capabilities are the right
primitive at all. A reader auditing that axiom itself should read
[open-problems.md](open-problems.md) first: covert channels, revocation at scale,
and the verification cliff are where the paradigm's own literature is most honest
about its limits.

Because this lineage is a **load-bearing dependency** for Myrhiza's host-surface
axiom (not a competitor being surveyed), there is a built-in incentive to
soft-pedal the problems Myrhiza would *inherit* by adopting it — the IPC/gate
performance tax ([open-problems.md §7](open-problems.md)), the covert-channel
gap Myrhiza cannot close at the capability layer
([open-problems.md §1](open-problems.md)), and the cross-peer revocation problem
that the clean in-kernel CDT story does *not* solve once authority leaves one
machine ([open-problems.md §3](open-problems.md)). Those are surfaced explicitly
in [open-problems.md](open-problems.md) and the **Avoid** table in
[lessons.md](lessons.md) precisely to counter that pull; treat any place this
folder sounds uncritically confident as a prompt to re-check against the spec's
own scoped-out list, not as settled fact.

## Glossary stub

- **Capability** — an unforgeable, transferable token that *designates* an
  object and *carries the authority* to use it. Full list in
  [glossary.md](glossary.md).
- **Ambient authority** — authority a program has by virtue of *who it is*
  rather than *what it was handed*; the thing capability systems abolish.
- **Confinement** — the property that a program cannot leak its authority to a
  party that was not authorized to receive it.
- **TCB** — Trusted Computing Base: the code whose correctness everything else
  depends on. The lineage's central goal is to make it small.

## Sources

- https://en.wikipedia.org/wiki/KeyKOS
- https://en.wikipedia.org/wiki/EROS_(microkernel)
- https://en.wikipedia.org/wiki/SeL4
- https://en.wikipedia.org/wiki/Genode
- https://sel4.systems/
- https://genode.org/download/sculpt
