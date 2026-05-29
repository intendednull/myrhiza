**Date:** 2026-05-29
**Status:** active
**Subject:** Glossary — capability-OS-lineage-specific terms

# Glossary

System-specific vocabulary for this folder. For the language-side ocap terms
(vat, sturdyref, swissnum, netlayer) see
[spritely-ocapn/glossary.md](../spritely-ocapn/glossary.md); for wire-token terms
see [capability-tokens](../capability-tokens/README.md).

- **Capability** — an unforgeable, transferable token that *designates* an
  object and *carries the authority* to operate on it. Holding it is the
  permission; you cannot forge one, only receive it. See
  [capability-model.md](capability-model.md).

- **Ambient authority** — authority a program has by virtue of *who it is* (its
  uid, its identity) rather than *what it was handed*. Capability systems abolish
  it. Fuchsia: "no ambient authority."

- **Designation = authority** — the defining capability property: naming an
  object and being permitted to use it are the same act, fused into one
  reference. Eliminates the name-resolution gap where confused-deputy bugs hide.

- **Confused deputy** — a privileged program tricked into misusing its *own*
  ambient authority on a caller's behalf. Named by Norman Hardy (1988). The bug
  capabilities were "invented to prevent."

- **Confinement** — the property that a program cannot leak its authority (or
  data) to a party not authorized to receive it. Lampson, 1973. The load-bearing
  theorem of this lineage; see [confinement-and-take-grant.md](confinement-and-take-grant.md).

- **Covert channel** — an unauthorized information path (timing, resource
  contention, power) that defeats confinement without violating the explicit
  authority model. The reason confinement is hard.

- **Factory** (KeyKOS) — a mechanism that builds an object and can *certify it is
  confined* to a prospective client, enabling mutually-suspicious composition.
  Ancestor of the EROS constructor. See [keykos.md](keykos.md).

- **Constructor** (EROS) — the sharpened factory: a trusted builder that attests
  a new process holds no leaking capability beyond authorized "holes," so a
  client can verify confinement before trusting it. See [eros.md](eros.md).

- **Single-level store** — a uniform persistent address space with no
  memory-vs-disk distinction; the disk backs the whole object world. KeyKOS's
  persistence substrate. See [keykos.md](keykos.md).

- **Checkpoint** (KeyKOS) — periodic, asynchronous, system-wide snapshot of all
  object state; on restart the system resumes mid-computation. Orthogonal
  persistence in practice.

- **Take-grant** — Lipton & Snyder's (1977) graph model of authority
  propagation via *take* and *grant* rights; makes "can A ever acquire right r
  over B?" decidable in linear time. See
  [confinement-and-take-grant.md](confinement-and-take-grant.md).

- **\*-property** (star-property) — the Bell–LaPadula "no write-down" rule
  forbidding high-secrecy data from flowing into low-secrecy containers. Long
  argued (Boebert) to be unenforceable on capability machines; EROS's 2000 proof
  showed it *is* enforceable given a confinement mechanism.

- **TCB (Trusted Computing Base)** — the code whose correctness everything else
  depends on. The lineage's goal is to minimize it. See
  [tcb-and-verification.md](tcb-and-verification.md).

- **Functional correctness** (seL4) — a machine-checked proof that the C
  implementation refines the abstract specification (never crashes / never does
  anything undefined relative to the spec). Says nothing about timing or covert
  channels. See [sel4.md](sel4.md).

- **CSpace / CNode** (seL4) — the kernel-protected capability space of a thread;
  user code names a capability by an index into it, never by a forgeable
  pointer.

- **Untyped memory + Retype** (seL4) — all kernel objects are explicitly carved
  from capability-controlled "untyped" memory via a `Retype` operation; no hidden
  kernel heap. Makes resource use accountable and bounded.

- **CDT (Capability Derivation Tree)** (seL4) — kernel-maintained record of each
  capability's lineage, enabling recursive `Revoke` of a capability and all
  capabilities derived from it. The local, complete revocation primitive.

- **Badge** (seL4) — an issuer-set tag on a capability used to distinguish/
  identify minted capabilities; minting can only narrow authority.

- **Recursive system structure** (Genode) — the component tree rooted at *core*,
  where each component runs in a sandbox and can spawn sub-sandboxes from its own
  resources; authority flows strictly parent → child. See
  [exemplars.md](exemplars.md).

- **Capability mode** (Capsicum) — a per-process FreeBSD sandbox (`cap_enter()`)
  that denies access to global namespaces; the process acts only through
  file-descriptor capabilities it already holds.

- **Handle** (Zircon) — a process-local, rights-bearing reference to a kernel
  object; rights live on the handle, not the object; duplication can drop but not
  add rights. The closest mainstream analogue to a WIT resource handle.

- **Application kernel** (gVisor) — a user-space re-implementation of a host
  syscall interface (the Sentry) that interposes on a sandboxed app's syscalls;
  the *opposite* school to capabilities (interpose on a fat ABI rather than
  remove the ambient surface). See [exemplars.md](exemplars.md).

- **Orthogonal persistence** — persistence as a property of the substrate, not
  an opt-in save/load API. KeyKOS's single-level store + checkpoint is the
  canonical example.

## Sources

- https://en.wikipedia.org/wiki/Capability-based_security
- https://en.wikipedia.org/wiki/KeyKOS
- https://en.wikipedia.org/wiki/EROS_(microkernel)
- https://en.wikipedia.org/wiki/SeL4
- https://fuchsia.dev/fuchsia-src/concepts/kernel/handles
- https://www.usenix.org/legacy/event/sec10/tech/full_papers/Watson.pdf
