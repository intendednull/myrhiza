**Date:** 2026-05-29
**Status:** active
**Subject:** Deployed capability/sandbox exemplars — Genode/Sculpt, FreeBSD Capsicum, Fuchsia/Zircon, gVisor

# Deployed exemplars

The pure-capability research lineage ([keykos](keykos.md) → [eros](eros.md) →
[coyotos](coyotos.md) → [sel4](sel4.md)) is the theory. This file is the *what
actually ships* counterweight: four production-or-near-production systems that
took capability/sandbox ideas to real users — and, in three of the four,
**watered the model down to fit a legacy substrate.** The dilution is itself the
lesson.

## Genode / Sculpt OS — the recursive purist that ships

**Genode** is a component-OS framework; **Sculpt OS** is the general-purpose
desktop/phone OS built on it, used daily by its own developers. It is the
nearest thing to a pure-capability OS that a person can run on a laptop in 2026.

- Founded **2008** by **Norman Feske** (Genode Labs; originated as the "Bastei"
  architecture at TU Dresden). License **AGPL-3.0-only + commercial**.
- **Sculpt OS 26.04** released **30 April 2026** — actively shipping.
- Runs on a *choice* of microkernels — **seL4, NOVA, Fiasco.OC**, base-hw, and
  others — decoupling the component framework from the kernel.
- **Recursive system structure:** every program runs in a sandbox granted only
  what it needs; a program can spawn **sub-sandboxes out of its own resources**,
  forming a tree rooted at *core*. Authority and resources flow **strictly
  parent → child** — a child can never hold more than its parent delegated.
- **Parent-mediated delegation + automatic revocation:** capabilities are
  delegated down the tree; when an RPC object is destroyed, the kernel
  invalidates *all* capabilities referring to it, **however far they were
  delegated** — revocation is a structural side-effect, not a separate ledger.

**Relevance to Myrhiza:** Genode's recursive parent-delegated tree is the
strongest living model for Myrhiza's intersection rule. `M_effective = A_ambient ∩
M_required` ([capabilities.md §7.2](../../specs/2026-05-09-myrhiza-master-design/capabilities.md))
*is* "a child can never exceed its parent" expressed for a one-level app→module
relationship. If Myrhiza ever needs module→sub-module nesting, Genode's
n-level recursive structure is the precedent to study — and its "destroy the
object, revoke all derived capabilities" is the same shape as Myrhiza's
"tokens expire when the component instance terminates"
([abi.md §8.5](../../specs/2026-05-09-myrhiza-master-design/abi.md)).

## FreeBSD Capsicum — capabilities retrofitted onto POSIX

Capsicum (Watson, Anderson, Laurie, Kennaway, *Capsicum: practical capabilities
for UNIX*, USENIX Security 2010) is a **hybrid**: it adds a capability discipline
to an OS that was never capability-based.

- **Capability mode:** `cap_enter()` puts a process into a sandbox that **denies
  access to global namespaces** — no `open()` by path, no PID namespace, no
  sysctl by name. The process can act only through file descriptors it already
  holds.
- **Capabilities = refined file descriptors:** `cap_rights_limit()` narrows the
  rights on an fd (e.g. read-only, no-seek). The familiar Unix fd becomes an
  attenuable capability.
- First appeared (experimental) in **FreeBSD 9.0**; compiled in by default from
  **FreeBSD 10.0**. Base-system users include `tcpdump`, `dhclient`, `kdump`.

**The dilution:** Capsicum is "ambient authority everywhere, *except* inside a
cap-mode sandbox." It is opt-in, per-process, and an un-sandboxed process still
has the full ACL/uid model. It proves you *can* bolt the capability discipline
onto a mainstream kernel — and shows the cost: the abstraction is only as strong
as the boundary you remembered to draw. Myrhiza has the opposite default
(capabilities are the *only* surface, ambient authority does not exist), which
is the right side of this trade — Capsicum is the cautionary "retrofit" case.

## Fuchsia / Zircon — handle-objects, no ambient authority

Google's **Zircon** microkernel (under Fuchsia) is a from-scratch
capability-ish design at hyperscaler resourcing.

- User code touches kernel resources **only via handles** to kernel objects.
  Kernel objects themselves carry no security; **rights live on the handle.**
- Two handles to the same object can carry different rights; duplicating a
  handle can **drop** rights but never add them — kernel-enforced attenuation.
- Stated principle: "applications running on Fuchsia have **no ambient
  authority**" — capabilities can be transferred but not forged.

**Relevance to Myrhiza:** Zircon's handle-with-rights is the closest mainstream
analogue to a **WIT resource handle**
([abi.md §8.3](../../specs/2026-05-09-myrhiza-master-design/abi.md)): a
slot-indexed, non-forgeable, rights-bearing reference the holder can attenuate
but not amplify. It is independent evidence that the resource-handle abstraction
Myrhiza inherits from the Component Model is a sound capability primitive, not a
toy. (Caveat: Fuchsia's *product* trajectory has been turbulent; cite it for the
**kernel object/handle model**, not as a thriving end-user platform.)

## gVisor — the application-kernel, a different bet entirely

Google's **gVisor** is included as the *contrasting* isolation philosophy. It is
**not** a capability system; it is an **application kernel**.

- A user-space process (the **Sentry**, written in Go) **reimplements the Linux
  syscall interface from scratch** and intercepts the sandboxed application's
  syscalls. The app's syscalls never reach the host kernel directly.
- Filesystem I/O is brokered to a separate **Gofer** process over a 9P-style
  channel; the Sentry itself is constrained by **seccomp** filters to a small,
  fixed set of host syscalls (≈53 without networking).
- `runsc` is the runtime; Google uses gVisor to sandbox untrusted workloads on
  Google Cloud / Kubernetes.

**Why it's here:** gVisor and the capability lineage solve the same problem —
*safely run untrusted code* — by opposite means. The lineage **removes** the
dangerous surface (no ambient authority, no global namespace). gVisor **keeps**
the surface (a full Linux ABI) but **interposes** a re-implementation in front
of it and shrinks the *host* attack surface behind seccomp. Myrhiza is firmly on
the capability side: it does not emulate a host OS; it exposes a small,
declared, kernel-brokered import set
([abi.md §8](../../specs/2026-05-09-myrhiza-master-design/abi.md)). gVisor is the
reference for "the other school," and a reminder that interposition-on-a-fat-ABI
has a real, ongoing maintenance cost (a whole Linux re-implementation to keep
current) that the small-surface school avoids — compare
[wasm-component-model](../wasm-component-model/README.md), which makes the same
small-typed-surface bet.

## Sources

- https://en.wikipedia.org/wiki/Genode
- https://genode.org/about/index
- https://genode.org/download/sculpt
- https://genode.org/documentation/genode-foundations/20.05/architecture/Recursive_system_structure.html
- https://www.usenix.org/legacy/event/sec10/tech/full_papers/Watson.pdf (Capsicum, USENIX Security 2010)
- https://www.cl.cam.ac.uk/research/security/capsicum/
- https://fuchsia.dev/fuchsia-src/concepts/kernel/handles
- https://fuchsia.dev/fuchsia-src/concepts/principles/secure
- https://gvisor.dev/docs/architecture_guide/security/
- https://gvisor.dev/blog/2019/11/18/gvisor-security-basics-part-1/
