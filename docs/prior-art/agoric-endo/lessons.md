**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric / Endo / SwingSet — design lessons for Myrhiza (validates / avoid / borrow)

# Lessons for Myrhiza

The consult-this-when-designing file. Synthesis from the rest of the corpus, framed as actionable design statements.

Agoric/Endo/SwingSet is the most production-hardened ocap + deterministic-replay JS runtime in existence. The runtime has shipped to mainnet validators since 2022-10-27; Endo has shipped to MetaMask Snaps users at scale. Where Agoric's design choices have been validated by years of consensus-critical operation, we should treat them as load-bearing references. Where they have failed, retreated, or sunset (Inter Protocol, the JS-on-chain perf budget, the cross-implementation OCapN draft), we should treat the failure mode as a constraint to design *around*, not a tax to pay.

This file structures lessons as **Validates / Avoid / Borrow** tables. See the [framing disclosure in `README.md`](./README.md#how-to-use-this-prior-art-doc): "Validates" entries are claims about us dressed as observations about Agoric — weight them with skepticism. "Avoid" and the load-bearing items in "Borrow" are the higher-leverage content.

## Validates

Things Agoric/Endo's production experience confirms about choices Myrhiza has already made or is leaning toward.

| Myrhiza choice | What Agoric validates |
|---|---|
| **State-apply must be a pure function of (prior state, event).** | SwingSet's transcript-driven vat replay is exactly this discipline, shipped to consensus-critical validators for ~3.5 years. The discipline is real, the discipline is engineerable, and the discipline is auditable in production. |
| **Capabilities are the only host surface.** | MetaMask Snaps + Endo run untrusted third-party code on hundreds of millions of users with no shared-memory bugs and no escape-from-compartment exploits at scale. Ocap discipline plus a careful host scales. |
| **Determinism is a load-bearing property, not a quirk.** | Agoric chose XS over V8 specifically because cross-validator determinism is non-negotiable. They built `xsnap` to wrap it. They explicitly removed `Math.random` and `Date.now` from vat surfaces. Our `state-apply` purity is the same shape of decision. |
| **Bundles as content-addressed identity.** | `@endo/bundle-source` v4's `b1-<sha512(compartment-map)>` bundle hash is the closest-existing-art for a deterministic, hash-addressed, recursively-verifiable code bundle in production. Validates our app-as-WASM-bundle plan. |
| **Cross-implementation protocol is worth co-designing with neighbors.** | Agoric is co-author of OCapN with Spritely, MetaMask, Cap'n Proto. The fact that the four reference-impl teams talk to each other is itself evidence that this is the correct shape of cross-system spec work. |
| **Pre-1.0 monorepos with semver-strict release notes are tractable to depend on.** | `agoric-sdk` and `endo` are both pre-1.0 monorepos with monthly+ release cadence. They are usable in production by sufficiently-disciplined consumers (MetaMask, Agoric chain itself). Iroh follows the same pattern — see [`../iroh/distribution.md`](../iroh/distribution.md). The pattern works. |

**Skepticism check on this section** (per the framing disclosure): every entry above is a Myrhiza decision we *want* validated. Agoric's success at any of these is partial evidence, not proof. The MetaMask Snaps at-scale validation is the strongest item; it is the only one with a population-of-millions sample size. The others are 1–2 consensus-critical chain validators' worth of evidence and should be treated as such.

## Avoid

Things Agoric/Endo did that did not work or should not be replicated in Myrhiza's design space.

| Anti-pattern | Why we avoid |
|---|---|
| **Fusing a flagship application into the runtime narrative.** | Inter Protocol sunset 2025-06-30, TVL ~$103K. The chain's identity got entangled with one CDP-stablecoin product; when the product failed market-fit, the chain narrative had to be rewritten ("Orchestration pivot"). Myrhiza must remain agnostic to whichever apps win. **No flagship app appears in the Myrhiza master spec.** |
| **JS as the kernel substrate.** | XS is excellent and `xsnap` is impressive engineering, but choosing JS forced Agoric to import the *entire JS evolution surface* — every TC39 stage, every Node compat issue, every npm-supply-chain risk. WASM Component Model gives us a smaller, schema-defined surface that's deterministic-by-construction; Agoric is paying the cost of having picked the bigger language. |
| **Single-engine lock-in.** | SwingSet is XS-specific. There is no second JS engine that can replay an XS transcript bit-for-bit. Myrhiza specs must require *any* spec-conforming WASM engine to produce identical state-apply outputs. Determinism by *spec*, not by *engine*. |
| **Permission-prompt UX as the cap-grant primitive for end users.** | MetaMask Snaps permission-prompt fatigue is real (see [`critiques.md`](./critiques.md)). Users habituate, click through, and the cap discipline degrades to a checkbox. Myrhiza must design cap discovery + grant for human users without inheriting the prompt-storm UX. |
| **Single-entity chain governance.** | No separately incorporated Agoric Foundation; governance flows through one company plus DCF plus on-chain BLD votes which are dominated by insiders. When the company pivoted (Orchestration), the chain pivoted. Myrhiza is not a chain, but the lesson generalizes: do not put the spec, the steward entity, and the deploy fleet under one corporate roof. |
| **Bridge / orchestration as composition.** | Agoric's 2024–2026 Orchestration pivot frames cross-chain bridging as the thing app developers want. From the inside (vats), each remote chain looks like an actor you `E()`-call. From the outside, it's still N+1 chains with N+1 trust assumptions. **In Myrhiza, peer-local composition is a P2P advantage** — apps share a single peer-symmetric runtime; "bridges" are the failure mode we don't have. |
| **Snapshots as part of consensus.** | Agoric chose explicitly *not* to consensus on XS heap snapshots (`agoric-sdk#5227`); transcripts are the consensus primitive, snapshots are local cache. **Borrow this exact decision** — for Myrhiza, the equivalent is: peers consensus on the event log + state-apply outputs, *not* on internal component memory layouts. |
| **`Math.random()` / `Date.now()` removal as the only determinism primitive.** | Agoric removes these from the vat side via lockdown. WASM doesn't have them as ambient builtins to begin with. But the deeper trap is that "removing the obvious sources" is not the same as proving determinism — vats can still loop forever, allocate forever, or starve. Myrhiza's `state-apply` purity must be enforced *at the verification step*, not by ablating builtins. |

## Borrow

Specific design choices from Agoric/Endo that Myrhiza should adopt with attribution.

| Borrow | Where to apply |
|---|---|
| **Transcript-driven replay shape.** | Every `state-apply` invocation is logged with `(input event, prior state hash, output state hash)`. Replay = re-run inputs, verify hashes match. This is exactly SwingSet's vat-transcript shape, narrowed to pure functions. See [`determinism.md`](./determinism.md) and [`persistence.md`](./persistence.md). |
| **Snapshot-as-local-cache, never-as-consensus.** | Per `agoric-sdk#5227`. Cross-peer convergence is over events + state-apply outputs. Component instance memory layouts are local. Peer-A and Peer-B can disagree on snapshot bytes and still agree on state. |
| **Computron-style deterministic metering.** | `DEFAULT_CRANK_METERING_LIMIT = 1e8` per delivery, accumulated, predictable. Myrhiza needs an analogous "fuel" budget for `state-apply` invocations — both for DoS protection and for cross-peer "did we reach the same termination point?" determinism. WASM has built-in fuel/epoch metering primitives; use them. |
| **The `baggage` upgrade convention.** | When a vat is upgraded to new code with the same identity, what survives is whatever was stored under the `baggage` key in the vat-store. Everything else is reset. This gives a clean upgrade contract without committing to "all heap survives." Myrhiza component upgrade should adopt the same shape: explicit `baggage`-style persistent collection, everything else discarded. |
| **`bringOutYourDead` ceremony for distributed GC.** | The kernel periodically asks each vat "tell me what you can drop." This is the synchronization point for cross-vat GC. Myrhiza's distributed GC across peers needs the same shape: a periodic, deterministic, kernel-driven liveness scan, not opportunistic per-message ref-count chatter. |
| **`b1-<sha512>` hash addressing on bundles, hashing the *compartment-map*, not raw bytes.** | The Endo bundle hash is over the structured manifest, which is robust to whitespace/encoding differences in source. Myrhiza bundles should hash a normalized manifest of the WASM components + WIT interfaces + asset blobs, not the zip bytes. (See [`modules-and-bundling.md`](./modules-and-bundling.md) for the precise scheme.) |
| **Three pass-styles.** | Endo's data / presence / promise classification is the right partition for cap-bearing values. Myrhiza's WIT-equivalent should distinguish "passed by value (data)", "passed as live ref (presence)", "passed as a future (promise)". See [`capabilities.md`](./capabilities.md). |
| **OCapN co-design as the cross-peer protocol.** | Track upstream OCapN (Spritely + Agoric + MetaMask + Cap'n Proto are co-designing). Don't fork. Don't invent a private protocol. Commit to OCapN late — when the spec stabilizes — but commit to it. See [`comparisons.md`](./comparisons.md). |
| **Pin-policy: lock-step across umbrella.** | If Myrhiza ever depends on `@endo/lockdown` or similar (e.g. for an ahead-of-WASM JS escape hatch in browser shim), pin all `@endo/*` packages together. Cross-package mismatches are real, often delayed-binding. Same lesson as Iroh's `iroh-blobs ↔ iroh` mismatch. |
| **Honest, auto-generated changelogs (`git-cliff` shape).** | Both `agoric-sdk` and `endojs/endo` ship auto-generated, structured changelogs with `[**breaking**]` markers. Mechanical, complete, no marketing prose. Myrhiza adopt the same tooling once we have multiple crates to coordinate. |

## Open questions Myrhiza specs need to answer

These came up repeatedly in the deep-dive and don't have a clean Validates/Avoid/Borrow disposition.

1. **What is the WASM-Component-Model equivalent of `bringOutYourDead`?** WASM has reference-typed (gc) types in some engines; Component Model has resource handles. Spec what GC sync looks like across peers.

2. **What is our `baggage` analog?** A persistent, content-addressed key-value handle that survives `state-apply`-instance-replacement-with-new-bundle? Or do we forbid in-place upgrade entirely and require an event-replay migration?

3. **What's our cross-peer determinism contract?** Is it bit-identical state-apply outputs, or hash-equal? What hash? At what granularity (per-event, per-block, per-N-events)?

4. **Browser-side state-apply: does it exist?** Agoric does not run SwingSet in the browser — it's Node-only. We have a stronger commitment to browser-side participation. State-apply in `wasm32-unknown-unknown` with no JIT and no `Date.now()` is the design problem.

5. **OCapN binding timing.** The protocol is pre-1.0. Spritely ships the reference impl. Agoric co-designs but does not deploy. When do we commit?

6. **Permission-grant UX.** MetaMask Snaps' approach (per-snap permission prompts with persistent grants) does not generalize well. Myrhiza needs a better answer for human cap-grant UX. This is a UX spec, not a runtime spec, but it must exist before app developers can ship.

## Sources

This file synthesizes the rest of the corpus. Source URLs are cited inline in the per-subsystem files. Highest-leverage source files:

- [`./determinism.md`](./determinism.md) — XS, computrons, four documented mainnet incidents
- [`./persistence.md`](./persistence.md) — transcript replay, baggage, swing-store SQLite
- [`./capabilities.md`](./capabilities.md) — pass-styles, CapTP, distributed GC
- [`./modules-and-bundling.md`](./modules-and-bundling.md) — bundle hashing scheme
- [`./critiques.md`](./critiques.md) — Inter Protocol sunset verbatim, MetaMask permission-prompt fatigue
- [`./open-problems.md`](./open-problems.md) — 10 unresolved items with Myrhiza disposition
- [`../spritely-ocapn/lessons.md`](../spritely-ocapn/lessons.md) — sibling ocap-lineage lessons
