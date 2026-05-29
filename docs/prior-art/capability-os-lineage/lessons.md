**Date:** 2026-05-29
**Status:** active
**Subject:** Lessons for Myrhiza — validates / avoid / borrow from the capability-OS lineage

# Lessons for Myrhiza

The consult-this-when-designing file. The other files in this folder are
evidence; this one is decisions, tied to the Myrhiza surfaces named in the task:
the four-layer capability gating model
([capabilities.md §7](../../specs/2026-05-09-myrhiza-master-design/capabilities.md),
`M_effective = A_ambient ∩ M_required`), the kernel-is-the-call-broker TCB boundary
([abi.md §8](../../specs/2026-05-09-myrhiza-master-design/abi.md)), and the
"capabilities are the only host surface" axiom.

## Validates

These Myrhiza choices are confirmed by forty years of capability-OS experience:

- **"Capabilities are the only host surface" is a *proven* OS-design posture,
  not an experiment.** KeyKOS ran a production mainframe OS on pure capabilities
  ([keykos.md](keykos.md)); seL4 carries a machine-checked proof of a pure-
  capability kernel ([sel4.md](sel4.md)); Fuchsia/Zircon ships the same
  no-ambient-authority model at Google scale ([exemplars.md](exemplars.md)).
  Myrhiza is on solid, well-trodden ground at
  [abi.md §8.4](../../specs/2026-05-09-myrhiza-master-design/abi.md).
- **The kernel-as-call-broker is the correct TCB shape.** Every system here puts
  *all* privileged mediation in a small kernel and pushes everything else into
  unprivileged, capability-confined components. Myrhiza's kernel owning I/O /
  keys / network / storage, with apps reaching it only through declared imports,
  is the microkernel TCB-minimization argument applied to a WASM host
  ([tcb-and-verification.md](tcb-and-verification.md)).
- **Manifest intersection is take-grant confinement, computed statically.**
  `M_effective = A_ambient ∩ M_required` is "a child can never exceed its parent"
  (Genode's recursive tree) made into a link-time check. Because Myrhiza forbids
  ambient authority *and* forbids amplification, "can module M reach capability
  c?" is decidable by manifest inspection — the take-grant tractability property
  ([confinement-and-take-grant.md](confinement-and-take-grant.md)).
- **Per-call gating is a textbook confused-deputy defense.** Re-checking the
  *calling* component's manifest on high-value ops
  ([capabilities.md §7.3](../../specs/2026-05-09-myrhiza-master-design/capabilities.md))
  is exactly Hardy's 1988 fix: authority travels with the invoker, so a deputy
  can never apply more than the caller held ([capability-model.md](capability-model.md)).
- **Non-forgeable resource handles are a sound capability primitive.** WIT
  resource handles are the slot-indexed, rights-bearing, attenuable-not-
  amplifiable reference that Zircon handles and seL4 CSpace slots independently
  validate ([exemplars.md](exemplars.md), [capability-model.md](capability-model.md)).
- **Confirm confinement *before* trusting third-party code.** The KeyKOS factory
  / EROS constructor establish the discipline: verify a component is confined
  before handing it anything sensitive. Myrhiza's link-time intersection check is
  a static instance of the same posture ([eros.md](eros.md)).

## Avoid

| Pitfall | Source | Myrhiza mitigation |
|---|---|---|
| **Treating a verified/sound protection model as immunizing the layer beside it.** EROS *proved* its confinement model, then shipped an exploitable **synchronous-IPC** flaw next to it. | [eros.md](eros.md) | The protection check (intersection) and the *interaction protocol* (the async ABI) are separate concerns; harden both. Myrhiza's submit-and-poll async ABI ([abi.md §8.5](../../specs/2026-05-09-myrhiza-master-design/abi.md)) already declines EROS's sync-IPC blocking trap — keep them decoupled in review. |
| **Coupling a correctness goal to un-shipped infrastructure.** Coyotos tied "verified kernel" to "first invent BitC"; the dependency sank it. | [coyotos.md](coyotos.md) | Don't gate Myrhiza correctness on a tool that doesn't exist yet. Lean on shipped substrates (Wasmtime, Component Model) and determinism-as-checkable-property, not a from-scratch proof. |
| **Single-maintainer / single-company projects die.** GNOSIS→KeyKOS (Key Logic closed, 1991); EROS→Coyotos (Shapiro → Microsoft, 2009). Only seL4 survived, via a Foundation. | [keykos.md](keykos.md), [coyotos.md](coyotos.md) | Treat the *protocol/spec* as the durable artifact, not any one implementation or person — the same lesson [spritely-ocapn/lessons.md](../spritely-ocapn/lessons.md) draws. Govern for succession. |
| **Believing the gate closes covert channels.** No capability model stops timing/resource side channels; the literature has said so since 1973. | [confinement-and-take-grant.md](confinement-and-take-grant.md), [open-problems.md](open-problems.md) | Keep covert/timing channels explicitly out of scope (the spec already does — [determinism.md §5.1](../../specs/2026-05-09-myrhiza-master-design/determinism.md) side-channel scope clarification declines capability-gate dispatch timing and cache-timing leaks). Don't market the gate as more than de-jure authority control. |
| **Retrofit / opt-in capability boundaries.** Capsicum is "ambient authority everywhere *except* inside a sandbox you remembered to enter." The boundary is only as strong as the one you drew. | [exemplars.md](exemplars.md) | Myrhiza's default is the opposite (no ambient authority *anywhere*; capabilities are the only surface). Preserve that default; resist any "escape hatch" host import that reintroduces ambient power. |
| **Letting the host-import surface grow unchecked.** Verification cost and audit difficulty scale super-linearly with TCB size; a fat surface forecloses the verification option permanently. | [tcb-and-verification.md](tcb-and-verification.md) | Treat every new host import as a TCB/ABI change with a justification cost ([abi.md §8](../../specs/2026-05-09-myrhiza-master-design/abi.md)). Keep the surface small enough to audit. |
| **Pretending capabilities imply identity or discovery.** Capability systems are introduce-then-invoke; they name objects, not principals, and don't find strangers. | [open-problems.md](open-problems.md) | Layer discovery/identity explicitly (see [did-methods](../did-methods/README.md), [sybil-resistance](../sybil-resistance/README.md)); don't assume the capability layer provides them. |

## Borrow

Primitives and constructions worth studying directly:

- **seL4's Capability Derivation Tree + explicit Revoke.** The kernel tracks
  every capability's lineage so revocation is recursive and complete *locally*.
  Myrhiza's "tokens expire at instance teardown" is the same idea; study the CDT
  if handle delegation ever needs to nest or outlive its creator. See
  [sel4.md](sel4.md).
- **seL4's Untyped-memory + Retype accounting.** All kernel objects are carved
  from explicit, capability-controlled untyped memory — no hidden heap. If
  Myrhiza ever needs hard, accountable per-component resource bounds (beyond the
  outstanding-token cap in [abi.md §8.5](../../specs/2026-05-09-myrhiza-master-design/abi.md)),
  this is the model for "resources are themselves capabilities."
- **Genode's recursive parent-delegated tree + destroy-revokes-derived.** The
  n-level generalization of Myrhiza's one-level app→module intersection. The
  precedent to study before adding module→sub-module nesting, and the cleanest
  "structural revocation as a side effect of teardown" design. See
  [exemplars.md](exemplars.md).
- **The EROS constructor as a "prove-confined-before-trust" pattern.** If
  Myrhiza ever supports dynamically-loaded modules whose confinement must be
  attested at runtime (not just link time), the constructor is the reference
  design. See [eros.md](eros.md).
- **seL4's verification *methodology* (refinement: abstract spec → executable
  spec → C → binary), not its scope.** Myrhiza won't prove its whole kernel, but
  the layered-refinement discipline is worth borrowing for the *determinism*
  argument: specify `state-apply` semantics, then show the implementation
  refines them. Pair with [formal-methods](../formal-methods/README.md).

## The one-paragraph version

This lineage *validates* Myrhiza's core bet — capabilities-only, kernel-as-
broker, small TCB — more strongly than any other prior-art folder, because these
systems *ran in production and got proved correct* doing exactly that. The
*borrowing* is mostly conceptual (the model is already in the Component Model);
the *avoiding* is where the value concentrates: don't let a sound protection
model lull you about the layer beside it (EROS), don't couple correctness to
un-built tools (Coyotos), don't let the host surface grow (the verification
cliff), and never claim the gate closes covert channels (Lampson, since 1973).

## Sources

- https://en.wikipedia.org/wiki/KeyKOS
- https://en.wikipedia.org/wiki/EROS_(microkernel)
- https://en.wikipedia.org/wiki/Coyotos
- https://en.wikipedia.org/wiki/SeL4
- https://en.wikipedia.org/wiki/Genode
- https://dl.acm.org/doi/10.1145/54289.871709 (Hardy, *The Confused Deputy*, 1988)
- https://flint.cs.yale.edu/cs428/doc/eros-verify.pdf (Shapiro & Weber, IEEE S&P 2000)
