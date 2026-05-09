**Date:** 2026-05-09
**Status:** active
**Subject:** Lessons from CRDT prior art for Myrhiza state-apply convergence

# Lessons for Myrhiza

The decision-relevant synthesis. The other files in this folder are evidence; this file is what we take away.

Format: validates / avoid / borrow / open questions, then a recommendation matrix.

## Validates

CRDT prior art **confirms** these Myrhiza design bets:

- **Deterministic merge across peers is achievable in production.** Three production-grade open-source libraries (Automerge, Yjs, Loro) demonstrate cross-peer convergence without a coordinator. Myrhiza's `state-apply` purity requirement is not an unrealistic ask — it is a well-established pattern with implementations.
- **Per-replica unique IDs are the universal identity primitive.** Automerge's 16-byte actor IDs, Yjs's 53-bit `clientID`, Loro's `u64` peer IDs — every library separates "who made the change" from "what the change is." Myrhiza's peer-pubkey-as-identity model maps cleanly.
- **DAG-of-changes encoding wins for distributed history.** Automerge's SHA-256-hashed change graph and Loro's op-log-as-DAG are the dominant pattern. Yjs deviates (uses Lamport-ordered linear log) and pays for it in awareness-protocol complexity. Myrhiza's event-log shape should be DAG, not linear.
- **Tombstones beat physical delete.** All three libraries keep tombstones for deleted elements to preserve causal context. Myrhiza `state-apply` should be tombstone-aware: a "removed" state isn't a missing state, it's a marked one.
- **Separation of CRDT state from ephemeral state is correct.** Yjs's design call to put cursor/presence in `awarenessProtocol` *outside* the CRDT — and to GC it on disconnect rather than persist it — is the right pattern. Myrhiza should similarly separate `state-apply`-converged state from per-session ephemera.
- **Per-container algorithm choice (Loro)** is principled. Different state shapes (text, list, map, counter, tree) want different merge semantics; forcing one algorithm across all shapes (Yjs/Automerge approach) is a compromise. Myrhiza `state-apply` components should be free to pick merge semantics per state shape.

## Avoid

CRDT prior art shows where the **easy mistakes** are:

- **Don't conflate "deterministic merge" with "byte-identical save output."** Automerge guarantees identical *head hashes* across peers but `save()` byte-output varies because of chunk-packing freedom (per [automerge.md](automerge.md)). Yjs apply-side is deterministic but generate-side has random `clientID`s. Loro byte-parity across language bindings is unverified (per [loro.md](loro.md)). Myrhiza determinism checks must compare on **content hashes**, not raw byte-equality of the serialized blob.
- **Don't expect a CRDT library to handle authority.** CRDTs converge regardless of *who* made the change — they cannot reject a malicious actor's update. Myrhiza `state-apply` MUST validate authority **before** the merge step, never inside it. This is a structural gap in CRDT designs (per [open-problems.md](open-problems.md)).
- **Don't expect a CRDT library to enforce invariants.** The bank-account-balance-must-be-positive problem cannot be solved within any of the three libraries. If Myrhiza state requires invariants across operations, those must live in `state-apply`'s **pre-check** (the kernel-dry-run path), not in the merge.
- **Don't expect a CRDT library to handle schema migration.** Adding a field to a `Y.Map` or `LoroMap` while old-schema replicas still exist is unsolved across all three libraries. If Myrhiza apps will evolve their state shape, `state-apply` needs an explicit schema-version + migration pathway. Cambria (Ink & Switch) is research, not production.
- **Don't lock in on one CRDT library expecting interop later.** Yjs/Automerge/Loro on-wire formats are mutually incompatible. There is no canonical CRDT exchange format. The library choice is **one-way**.
- **Don't pick `yjs` for Rust state-apply.** `yjs` is JavaScript. The Rust entry point is `yrs` (separate org `y-crdt/y-crdt`), which is a community reimplementation. Yjs and yrs aim at binary protocol compatibility; this is not the same as ABI compatibility. Treat yrs as the actual library, not "Yjs in Rust."
- **Don't assume Loro is production-ready at scale.** Loro has zero verified named at-scale production users (per [ecosystem.md](ecosystem.md)) and bus factor 1 (~81% of commits from one founder). It is shipping research-grade. Don't soft-pedal this if recommending Loro.
- **Don't pick Yjs governance posture for a load-bearing dependency.** Yjs is a single-maintainer project with ~22K stars and millions of downstream users. Kevin Jahns is paid through GitHub Sponsors. The "bus factor 1" risk is structural — for Myrhiza-as-runtime, picking yrs means inheriting that risk (yrs has its own maintainers but ultimately tracks Yjs design decisions).

## Borrow

Specific patterns Myrhiza `state-apply` design should **steal**:

- **Bloom-filter-based sync diff (Automerge).** ~10 bits per commit, 7 hash probes, identifies "changes the other peer doesn't have" in one round-trip without full enumeration. Applicable to Myrhiza event log replication regardless of CRDT-or-not.
- **State-vector sync round-trip (Yjs).** Peer A sends `encodeStateVector()`; peer B sends back delta. Single RT. Clean primitive worth replicating. The state-vector shape is `clientID → clock`.
- **Change graph as content-addressed DAG (Automerge).** SHA-256 hashing of (parents, content, actor) gives free deduplication, free integrity, free "do we already have this change?" Applicable to Myrhiza event log even without CRDT semantics.
- **Peritext span-based marks (Automerge / Loro `crdt-richtext`).** If any Myrhiza app surface ever exposes rich text, Peritext is the only published correct algorithm with formal interleaving guarantees over formatting marks. Don't reinvent.
- **Move-tree (Kleppmann 2021, used by Loro).** If Myrhiza state shapes ever include moveable trees (file system, hierarchical task list, scene graph), this is the algorithm with formal proof (Isabelle/HOL-verified). Don't reinvent.
- **Eg-walker insight (Kleppmann 2024).** "CRDT or not?" is a false dichotomy. The real primitive is *deterministic replicated data structure with merge semantics*. Eg-walker outperforms CRDTs on size + speed by computing local OT-like state from the change DAG rather than maintaining the CRDT in memory. Myrhiza `state-apply` is a deterministic replicated data structure by definition; it is not constrained to use a CRDT.
- **Critical Version + shallow snapshot (Loro).** Truncating change history before a pinned point so the working set stays bounded. Applicable to Myrhiza event log retention policy.

## Open questions

Myrhiza spec authors should address, with this corpus loaded:

- **Is `state-apply` a CRDT-style merge or a single-author-event-application?** A CRDT merge takes two diverged states and produces a converged state. A single-author-event-application takes one state and one event and produces the next state. These are different design points with different implications for what the kernel's `state-apply` ABI looks like. The corpus suggests CRDT-style for collaborative state and event-application for sequential state — Myrhiza could conceivably support both.
- **If CRDT-style, which library?** See recommendation matrix below.
- **Where does the authority layer sit?** The kernel has to validate "this peer is allowed to make this change" *before* `state-apply` merges it. The CRDT prior art is mute on this; Myrhiza must design it.
- **What's the migration story?** None of the three libraries solve schema evolution. Myrhiza apps will outlive their initial schema; the runtime needs a migration pathway.
- **WASM Component Model wrapping.** None of the three libraries ship as Component Model `.wasm` artifacts. They ship raw WASM modules with `wasm-bindgen`. Myrhiza will need to author the WIT for whichever library it picks (or for the chosen non-CRDT replicated-data-structure design).

## Recommendation matrix for Myrhiza

If Myrhiza decides to commit to one of these libraries today:

| If you want… | Choose | Reason | Risk |
|---|---|---|---|
| **Most production-hardening + healthy team** | Automerge (Rust crate `automerge`) | 7+ years, Ink & Switch + Kleppmann + 3 full-time, NLnet + GoodNotes funding, MIT, RGA + Peritext | Document-size growth (mitigated in 3.x but historically the weakest characteristic); 240 bytes/char in early Automerge benchmarks |
| **Largest editor-binding ecosystem (rich-text apps)** | yrs (Rust port of Yjs) | YATA proven; ProseMirror/Quill/CodeMirror/Monaco/Slate/Tiptap bindings; ~22K-star upstream | Yjs upstream is bus-factor 1 (Kevin Jahns); yrs tracks Yjs design decisions; v14 RC migration not yet stable |
| **Rust-native + WASM-native + per-container algorithm choice + Fugue text** | Loro | Rust-from-day-one, Fugue (only formally-non-interleaving text CRDT), Moveable Tree, time travel + shallow snapshots | No verified named at-scale production users; bus-factor 1 (~81% commits Zixuan Chen); WASM bundle ~1MB raw vs Yjs's 69KB |
| **Best determinism story** | Loro for byte-format stability commitment, Automerge for hash-equality across peers | Loro publicly committed to wire-format stability at 1.0; Automerge guarantees identical head hashes | Loro byte-parity across language bindings unverified; Automerge `save()` byte-output varies |
| **Maximum ecosystem safety (avoid bus-factor-1)** | Automerge | Healthiest stewardship of the three; Ink & Switch + Kleppmann advisor + 3 full-time + named funding | Smaller ecosystem than Yjs |
| **A non-CRDT alternative to consider** | Eg-walker / diamond-types | Kleppmann's 2024 EuroSys paper claims better performance than any CRDT for text; Joseph Gentle's diamond-types is the production code | Pre-stable; Loro's op-walking adopts it but no library has shipped it as the primary public API yet |

## Recommended posture for the runtime spec

A defensible default given current state:

1. **Don't bake a CRDT into the kernel.** `state-apply` should be Component-Model-typed pure-function-of-`(prior, event)`. The choice of CRDT (or non-CRDT) lives **in the application** state-apply component, not in the kernel.
2. **Provide a deterministic-helper-set capability** that includes content-hash, deterministic-set-merge, and deterministic-list-ordering helpers. CRDT libraries can be authored as user-space components consuming these.
3. **Author one reference state-apply component** demonstrating per-Myrhiza convention. Recommend Automerge for the reference component because it has the healthiest stewardship and the most production-hardening, even if other apps pick yrs or Loro.
4. **Document the WIT contract for "CRDT-as-state-apply"** — what the kernel guarantees about call ordering, snapshot semantics, and what user-space state-apply components are responsible for. This is the spec gap the corpus illuminates.

## Sources

This file synthesizes from sibling files. Primary sources cited per sibling:

- [automerge.md](automerge.md), [yjs.md](yjs.md), [loro.md](loro.md) — per-library deep dives
- [crdt-theory.md](crdt-theory.md), [history.md](history.md) — academic foundations
- [comparisons.md](comparisons.md) — head-to-head analysis
- [open-problems.md](open-problems.md) — what no CRDT solves
- [critiques.md](critiques.md) — Kleppmann on Eg-walker, Boodman/Rocicorp on server-authority, Haverbeke on OT-vs-CRDTs
- [ecosystem.md](ecosystem.md), [governance.md](governance.md) — adoption + bus-factor analysis
