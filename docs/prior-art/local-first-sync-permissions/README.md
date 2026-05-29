**Date:** 2026-05-29
**Status:** active
**Subject:** Local-first sync permissions — engine/product-level authority models (Jazz/cojson, Zero, Triplit, PowerSync, ElectricSQL); the product-layer answer to the CRDT authority gap

# Local-first sync permissions

How shipped local-first **products** decide who is allowed to write and read —
the layer *above* the CRDT libraries. [../crdts/](../crdts/) covers the merge
algorithms; this folder covers **authority**. It exists because
[../crdts/open-problems.md](../crdts/open-problems.md) §2–§3 states the
library-layer gap bluntly — **"CRDTs converge, then violate"**: no CRDT library
enforces *who* may write or *whether the converged result is legal*. The products
here are the product-layer answers, and they split into two paradigms.

This is a **survey** folder (multiple systems), like [../crdts/](../crdts/). It
serves two named Myrhiza decision surfaces: the **`myrhiza-permission-*` module
ABI** and the **`state-apply` authority verdict**
([../../specs/2026-05-09-myrhiza-master-design/convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.4).

## Key facts at a glance

| Product | Authority paradigm | Identity | Enforcement point | License (verified) | Latest (verified) | Maturity |
|---|---|---|---|---|---|---|
| **Jazz / cojson** | **Peer-verified** (Myrhiza-shaped) | self-sovereign keypair | every peer (`determineValidTransactions`) | MIT | `cojson` 0.20.18; Jazz 2.0 **alpha** | unfunded; **no named at-scale users verified** |
| **Zero** (Rocicorp) | Trusted middlebox | JWT (your server validates) | server (mutators; RLS rules **deprecated**) | Apache-2.0 | `@rocicorp/zero` 1.5.0 (1.0 stable shipped) | small co.; total-raised unverified |
| **Triplit** (Aspen Cloud → Supabase) | Trusted middlebox | JWT claims | server (schema rules) | AGPL-3.0-only | `@triplit/client` 1.0.50 | YC W21; **acquired by Supabase 2025-10-08** (stays OSS) |
| **PowerSync** (JourneyApps) | Trusted middlebox | JWT claims | service (Sync Rules) + backend (writes) | FSL-1.1-ALv2 (→ Apache-2.0) | service 1.21.x line | industrial-scale (in the *trusted-server* paradigm) |
| **ElectricSQL** | None (auth is external proxy) | external | trusted proxy in front of engine | Apache-2.0 core | v1.x (v1.1 Aug 2025) | shipping; comparator only |

The single most-cited fact, kept canonical here and cross-referenced elsewhere:
**Jazz/cojson is the rarer peer-verified model and the closest shipped analogue to
Myrhiza; the other four are trusted-middlebox and Myrhiza-rejected.**

## Reading order

1. **[paradigm-contrast.md](paradigm-contrast.md)** — the spine. Peer-verified vs
   trusted-middlebox; the cojson→Myrhiza mapping table; which library-layer gap
   this fills. **Start here.**
2. **[jazz-cojson.md](jazz-cojson.md)** — the Myrhiza-shaped model in detail:
   transactions, groups/roles, `determineValidTransactions`, encryption-based
   reads, key-rotation revocation.
3. **[maturity-and-the-2.0-pivot.md](maturity-and-the-2.0-pivot.md)** — **read
   before citing Jazz as mature.** The 2.0 alpha reintroduces a trusted-server
   tier; honest maturity scorecard.
4. **[zero-rocicorp.md](zero-rocicorp.md)**, **[triplit.md](triplit.md)**,
   **[powersync.md](powersync.md)** — the three trusted-middlebox products.
5. **[electricsql-comparator.md](electricsql-comparator.md)** — comparator: a sync
   engine that deliberately has no permission model.
6. **[open-problems.md](open-problems.md)** — what even peer-verified authority
   does NOT solve (global invariants, equivocation, revocation races, history
   growth, metadata privacy, declarative-DSL limits, encrypted-state migration).
7. **[lessons.md](lessons.md)** — *the decision file*. Validates / avoid / borrow,
   tied to the `myrhiza-permission-*` ABI and the `state-apply` verdict.
8. **[glossary.md](glossary.md)** — terms.

If you only have time for two files: **[paradigm-contrast.md](paradigm-contrast.md)
+ [lessons.md](lessons.md)**.

## Relationship to neighbor folders

- **[../crdts/](../crdts/)** — the library layer. This folder is the explicit
  product-layer answer to its [open-problems.md](../crdts/open-problems.md) §2–§3.
- **[../mls/](../mls/)** — group key management / revocation (TreeKEM); the
  cryptographic machinery behind "rotate the read key on revoke."
- **[../capability-tokens/](../capability-tokens/)** — capability-as-token; Electric's
  gatekeeper and cojson invites are token-shaped grants.
- **[../holochain/](../holochain/)**, **[../at-protocol/](../at-protocol/)**,
  **[../willow/](../willow/)** — other per-author signed-log systems Myrhiza draws on.

## Glossary stub

Full terms in [glossary.md](glossary.md). The load-bearing four:
**peer-verified authority** (every peer re-derives the verdict; no trusted
server), **trusted-middlebox authority** (a trusted server adjudicates),
**CoValue** (cojson's unit of collaborative state), **`determineValidTransactions`**
(cojson's per-peer authority function — the analogue of Myrhiza's `state-apply`
verdict).

## Framing disclosure

These docs are written from **Myrhiza's current design stance**, not as a neutral
catalog. That stance is specific: **capability-mediated** host access, **P2P-only**
(no trusted middlebox), the **Component Model on Wasmtime** as the execution
substrate, and **event-log-replay `state-apply`** as the authority surface. The
per-product "contrast / borrow" sections and all of [lessons.md](lessons.md) read
each system *through that lens* — they ask "how does this map onto / diverge from
the peer-verified runtime Myrhiza is building," not "what is the best local-first
permission model in the abstract." A reader optimizing for a different stance
(e.g. accepting a trusted server) would weigh these systems differently.

Honest counterweight, surfaced throughout: **trusted-middlebox is the mainstream
shipped choice precisely because it is easier** (a server can run real
transactions, see plaintext, express ad-hoc rules), and even Jazz — the one
aligned product — is drifting toward a trusted tier in 2.0
([maturity-and-the-2.0-pivot.md](maturity-and-the-2.0-pivot.md)). Peer-verified
authority is the deliberate, harder, trust-minimizing path, not the popular one.
Jazz's deployment scale is unverified; do not overstate it.

**Load-bearing-target caveat — read this before trusting the optimism.** Myrhiza
adopts the cojson/peer-verified shape, so this corpus has a structural incentive
to *soft-pedal the problems Myrhiza would inherit by doing so*. Guard against it:
the open problems Myrhiza takes on with this shape are real and only partially
mitigated — global/cross-object invariants (the bank-account problem survives the
product layer), forward-only racy revocation, monotonic authority-history growth
with no coordinated GC, metadata privacy the relay still sees, declarative-DSL
expressiveness walls, and schema migration over encrypted/on-log state. They are
catalogued without spin in [open-problems.md](open-problems.md); if any of those
reads as "handled," re-check it there. The fact that the *one* product sharing
Myrhiza's shape is itself retreating toward a trusted tier under exactly these
pressures ([maturity-and-the-2.0-pivot.md](maturity-and-the-2.0-pivot.md)) is the
most important disconfirming evidence in the folder — do not let the "validates"
framing bury it.

## Sources

Per-file `## Sources` sections list the URLs cited there. Top-level anchors:

- Jazz / cojson: <https://github.com/garden-co/jazz>, <https://jazz.tools>, <https://registry.npmjs.org/cojson>
- Zero: <https://zero.rocicorp.dev/docs>
- Triplit: <https://www.triplit.dev/docs>, <https://github.com/aspen-cloud/triplit>
- PowerSync: <https://docs.powersync.com>, <https://github.com/powersync-ja/powersync-service>
- ElectricSQL: <https://electric-sql.com/docs>
- Library-layer gap this fills: [../crdts/open-problems.md](../crdts/open-problems.md)
