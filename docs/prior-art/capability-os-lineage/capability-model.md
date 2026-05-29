**Date:** 2026-05-29
**Status:** active
**Subject:** The object-capability model — designation = authority, no ambient authority, the confused deputy

# The capability model (OS-side)

Every system in this lineage rests on one idea: **a capability is an unforgeable
token that both *names* (designates) an object and *carries the authority* to
operate on it.** You cannot hold a capability without the right to use it, and
you cannot manufacture one — you can only receive it from someone who already
has it. Authority flows only by explicit hand-off.

This is the OS-side statement of the same discipline the language-side folders
document for in-process objects (see
[spritely-ocapn/capabilities.md](../spritely-ocapn/capabilities.md),
[agoric-endo/capabilities.md](../agoric-endo/capabilities.md)) and the
token-format folder documents for the wire
([capability-tokens](../capability-tokens/README.md)). Same principle, three
enforcement substrates: a programming-language object graph, a serialized
bearer/PKI token, and — here — a kernel-maintained table.

## The two defining properties

1. **Designation is authority.** There is no separate "name a file, then ask
   may-I." Holding the reference *is* the permission. This collapses the
   name-resolution step where most access-control bugs hide.
2. **No ambient authority.** A process can do nothing by virtue of *who it is*
   — only by virtue of *what capabilities it was handed*. There is no global
   filesystem namespace to `open()`, no `getuid()` that confers power. Fuchsia
   states this most plainly: "applications running on Fuchsia have no ambient
   authority" (fuchsia.dev, Secure principle).

Contrast the dominant alternative, the **ACL / identity model** (Unix uid,
Windows ACLs): authority is ambient (a process acts with its user's full
rights), and access decisions consult a list keyed on identity at the moment of
use. That gap between *who you are* and *what you're doing for whom* is exactly
the **confused-deputy** hole.

## The confused deputy

Norman Hardy named it in *The Confused Deputy (or why capabilities might have
been invented)*, ACM SIGOPS Operating Systems Review, Vol. 22 No. 4 (October
1988). A compiler is given access to a billing file for its own accounting. A
user invokes it and names the billing file as the *output* path. The compiler,
acting with its own ambient authority, dutifully overwrites the billing file —
"confused" into wielding a privilege on behalf of a caller who should not have
had it.

The capability fix: the compiler should write only through a capability the
*caller* supplied. Authority then travels with the request, so the deputy can
never apply more authority than the invoker actually held. Designation and
authority are fused, so there is no ambiguous name to confuse.

This is not a museum piece. The pattern recurs in every system that hands a
privileged component a request plus an ambiently-held credential — SSRF,
CSRF, and the 2025-era "confused-deputy" prompt-injection class against AI
agents are the same bug. Myrhiza's per-call gating
([capabilities.md §7.3](../../specs/2026-05-09-myrhiza-master-design/capabilities.md))
is a direct confused-deputy defense: it re-checks the **calling component's**
manifest, so an interaction component cannot ask a more-privileged UI module to
escalate on its behalf.

## How capabilities are represented

The lineage uses three physical encodings, all preserving unforgeability:

- **Partitioned / segregated** (KeyKOS, EROS, seL4, Zircon): capabilities live
  in kernel-protected memory the process cannot write directly; the process
  holds an *index* (a slot number / handle) into its capability table. The
  kernel dereferences. This is the dominant OS approach because it gives
  unforgeability "for free" from memory protection.
- **Sparse / password** (rare in OS kernels, common on the wire): the
  capability is a large unguessable bit string; unforgeability comes from the
  improbability of guessing. This is what [sturdyrefs](../spritely-ocapn/glossary.md)
  and [capability tokens](../capability-tokens/README.md) use across a network.
- **Tagged** (historic capability hardware, e.g. IBM System/38, CHERI today):
  the hardware tags pointer words so they cannot be forged by arithmetic.

Myrhiza's host surface is the partitioned variant: **WIT resource handles** are
slot-indexed, kernel-arbitrated references the component cannot forge
([abi.md §8.3](../../specs/2026-05-09-myrhiza-master-design/abi.md),
[capabilities.md §7.4](../../specs/2026-05-09-myrhiza-master-design/capabilities.md)),
and the submit-and-poll request-tokens are HMAC-tagged so they sit in the sparse
family ([abi.md §8.5](../../specs/2026-05-09-myrhiza-master-design/abi.md)).

## Where the trust boundary actually sits (kernel-side, not language-side)

Representation is not the whole story; the more important question is *who
enforces unforgeability* — which is what separates this folder's tradition from
the language-side ocap folders. The split is:

- **Language-side enforcement** ([spritely-ocapn](../spritely-ocapn/README.md),
  [agoric-endo](../agoric-endo/README.md)): the capability is an in-process
  object reference, and unforgeability rests on the *language runtime's* memory
  safety and on membranes inside one address space. If the guest can break out
  of the language (native code, a VM escape), the object graph stops protecting
  anything. The enforcement and the guest live in the **same** trust domain.
- **Kernel/OS-side enforcement** (this folder): unforgeability rests on a
  *separate, more-privileged* component — a kernel with its own protection
  domain — that the guest cannot reach into even with arbitrary code execution.
  The capability table is on the *other side* of a hardware/sandbox boundary.

This is the load-bearing distinction for Myrhiza's "capabilities are the only
host surface" axiom. **Myrhiza's enforcement is kernel-side, this folder's
lineage.** The trust boundary is the WASM sandbox plus the Myrhiza kernel: a
component reaches the host only through declared imports the kernel arbitrates
([abi.md §8](../../specs/2026-05-09-myrhiza-master-design/abi.md)), and a guest
that goes fully hostile inside its sandbox still cannot forge a WIT resource
handle, because the handle table lives in the kernel, not in guest memory. Any
language-side membrane a guest runs *inside itself* is advisory in-guest
structure, not the load-bearing boundary — useful for intra-app composition, but
not what stops a malicious component. That is why this OS lineage, not the
language-ocap lineage, is the direct ancestor of the host-surface axiom.

## Attenuation and delegation

Because authority is a value, you can hand out a *weaker* version: a read-only
view, a single-method facet, a time-boxed proxy. This is **attenuation**, and it
is the constructive half of least-privilege — you grant exactly what is needed
by composing a reduced capability, not by configuring a policy. Zircon makes
this concrete: duplicating a handle can *drop* rights but never add them
(fuchsia.dev, Handles). seL4's capabilities carry rights bits and a *badge*; a
minted capability can only narrow. Myrhiza's manifest intersection
`M_effective = A_ambient ∩ M_required` is attenuation expressed declaratively at link
time rather than by hand-crafting a proxy.

## Sources

- https://dl.acm.org/doi/10.1145/54289.871709 (Hardy, *The Confused Deputy*, 1988)
- http://cap-lore.com/CapTheory/ConfusedDeputy.html
- https://fuchsia.dev/fuchsia-src/concepts/principles/secure
- https://fuchsia.dev/fuchsia-src/concepts/kernel/handles
- https://en.wikipedia.org/wiki/Capability-based_security
