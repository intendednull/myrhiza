**Date:** 2026-05-09
**Status:** draft

# Myrhiza — master design

The runtime spec. What we are building, what its shape is, what we
have committed to, what we have explicitly deferred, and how the v1
acceptance proof works.

This folder is the canonical reference for any spec, plan, report,
or implementation work that touches the runtime. Child files refine
specific subsystems — they cite each other and conform to this
README's vision + scope. When a child changes, sibling files review
for impact.

## 1. Vision and scope

Myrhiza is a **P2P app runtime**. A small kernel hosts typed,
capability-mediated, content-addressed apps. Apps are bundles of
WebAssembly components. The kernel owns identity, peer protocol,
event/DAG primitives, the component loader, and the capability
arbiter. Everything else — chat, wikis, kanban, polls, whatever
someone builds in two years — is an app.

**The novel commitment**: peers are infrastructure. Storage,
replication, sync, replay buffering, snapshot custody — these are
work performed by participants, not deployed services. As more peers
participate in an app, the app's maintenance capacity grows. No
infrastructure deploy required to scale.

**The security commitment**: capabilities are the only way components
reach beyond their own memory. Apps cannot touch private keys, the
network, persistent storage, or other apps' state directly — every
operation is mediated by a kernel-arbitrated capability. WASM
execution is non-negotiable on every backend (native, browser,
mobile); compiling apps to native code for performance is explicitly
rejected.

**What this is not**:

- Not a chat client. Chat is one app among many.
- Not a plugin framework for a host application. Apps are the
  product; the kernel is the substrate.
- Not a CRDT library. Apps may use CRDTs internally; the kernel
  stays generic.
- Not a service to deploy. Peers are the runtime.

**What's novel and what's borrowed**: the "peers as infrastructure"
framing has been claimed by Holochain and Pears; it is not novel on
its own. Myrhiza's distinct combination is: WCM-typed components +
capability-secure host surface + no-CRDT-in-kernel + author-bounded-
scale-at-v1 + event-log-replay convergence with TUTTI-shaped drift
detection. No prior project has shipped this combination. Honest
positioning: not a new pitch, a new combination.

**On "production-validated" claims**: when this spec cites Agoric
and Willow as precedents, note that Agoric is a Cosmos blockchain
(consensus-given event ordering, validator-class hardware) and
Willow is currently a hundreds-of-users-shape chat product. Neither
has stress-tested event-log replay as P2P infrastructure for write-
heavy public-read apps at scale. The master spec borrows the
substrate shape with awareness that scale validation is a v2+
obligation. See [convergence.md](convergence.md) §4.5 +
[risks.md](risks.md) for explicit scaling acknowledgment.

## 2. The three-tier architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │                       KERNEL                             │
   │  Identity. Peer protocol. Event/DAG primitives.          │
   │  Component loader. Capability arbiter. Crypto            │
   │  primitives. Narrow native imports.                      │
   └──────────────────────────────────────────────────────────┘
                              ▲
                              │ host imports (WIT-typed)
                              │
   ┌──────────────────────────────────────────────────────────┐
   │              MODULES  (myrhiza-* WASM components)        │
   │  Cross-cutting concerns reusable across apps:            │
   │  - Participation: social-graph, tit-for-tat, ...         │
   │  - Permission: rbac, governance, invite-chain, ...       │
   │  - Crypto: mls, channel-key, double-ratchet, ...         │
   │  - State helpers: snapshot-cache, log-prune,             │
   │    crdt-{automerge,yjs,loro}, ...                        │
   │  - Identity: multi-device, behavior, ...                 │
   │  - UI: components, theme-tokens, accessibility, ...      │
   └──────────────────────────────────────────────────────────┘
                              ▲
                              │ component imports / wac composition
                              │
   ┌──────────────────────────────────────────────────────────┐
   │                         APPS                             │
   │  counter, poll, chat, kanban, wiki, etc.                 │
   │  Compose modules + add app-specific state-apply +        │
   │  state-propose + interaction + behavior components.      │
   └──────────────────────────────────────────────────────────┘
```

**Kernel** is the privileged layer. Owns secrets, brokers all I/O,
arbitrates every cross-component call. Compiles to native (Wasmtime
host) or browser (jco-shimmed JS+wasm host).

**Modules** are reusable WASM components encapsulating cross-cutting
concerns. They look like apps to the kernel — same WASM Component
Model, same manifest format, same distribution channel — but are
designed to be pulled in by other apps as dependencies. App authors
declare module deps in their `manifest.toml`; the kernel intersects
capability declarations and links at instantiation time.

**Apps** are user-facing bundles. They compose zero or more modules
and add their app-specific component code (state-apply, state-propose,
interaction, behavior).

The tier separation is conceptual, not enforced — modules and apps
are mechanically the same shape (WASM components with manifest +
signature). The distinction is intent: modules are designed for
reuse; apps are designed for end users.

### 2.1 Why three tiers

**Why modules and not just apps**: cross-cutting concerns
(participation enforcement, RBAC, MLS, snapshot management) recur
across many apps. Without modules, every app reinvents these
patterns. With modules, each pattern is authored once, audited
once, distributed once.

**Why modules and not kernel features**: cross-cutting concerns
evolve faster than kernel ABI. Pinning MLS in the kernel locks
Myrhiza to one MLS implementation; pinning RBAC in the kernel
locks one permission model. Modules let the ecosystem evolve
without breaking kernel ABI.

**Why kernel and not just modules**: identity custody, capability
arbitration, deterministic state-apply replay, network plumbing,
content addressing — these need privileged access to native
resources (private keys, sockets, filesystem). They cannot be
modules without breaking the sandbox model.

The three tiers correspond to three trust boundaries: kernel is
trusted absolutely; modules are sandboxed but typically authored
or audited by the project; apps are sandboxed and may come from
anywhere.

## Reading order

For new contributors:

1. **This README** — vision, scope, three-tier overview.
2. **[architecture.md](architecture.md)** — four component profiles
   (state-apply / state-propose / interaction / behavior); the
   normative host import surface table.
3. **[convergence.md](convergence.md)** — event-log replay paradigm,
   topo-sort, sync protocol (HeadsSummary), pre-check unification,
   author equivocation, scaling future direction, topic identity,
   drift detection.
4. **[determinism.md](determinism.md)** — deterministic helper set,
   denied imports, fuel + resource limits, state-digest format
   pinning.
5. **[identity.md](identity.md)** — IdentityScope primitive, use cases,
   deferred items.
6. **[capabilities.md](capabilities.md)** — four-layer gating model.
7. **[abi.md](abi.md)** — Full Component Model decision, composition,
   submit-and-poll for async surfaces.
8. **[crypto.md](crypto.md)** — primitive crypto host imports,
   MLS-as-module direction.
9. **[distribution.md](distribution.md)** — manifest schema, signing,
   revocation, kernel binary trust root.
10. **[networking.md](networking.md)** — iroh transport, relays,
    topic membership.
11. **[maintenance.md](maintenance.md)** — no-worker-class framing,
    maintenance modules, social-graph Sybil resistance.
12. **[ui.md](ui.md)** — UI app contract, kernel-controlled UI surface.
13. **[browser-native.md](browser-native.md)** — dual-stack v1.
14. **[mvp.md](mvp.md)** — acceptance criteria, counter+poll demo,
    workspace shape.
15. **[migration.md](migration.md)** — Willow → Myrhiza path.
16. **[future.md](future.md)** — named-but-deferred items.
17. **[tradeoffs.md](tradeoffs.md)** — decision matrix with runners-up.
18. **[risks.md](risks.md)** — open questions / accepted risks.
19. **[implementation.md](implementation.md)** — critical-path outline
    handed off to writing-plans.
20. **[sources.md](sources.md)** — references.

## Cross-section anchor convention

Section numbering is preserved within each child file. References
across child files use `[file.md](file.md) §N.M` format (e.g.
"see [convergence.md](convergence.md) §4.5"). Within a single child
file, section refs may use bare `§N.M`.

## Sources

See [sources.md](sources.md).
