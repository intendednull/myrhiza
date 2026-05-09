**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/Endo/SwingSet — structural open problems and what Myrhiza inherits

# Open Problems

What does Agoric/Endo/SwingSet, after seven years of production engineering, structurally not solve? And which of those problems would Myrhiza inherit if we copy their patterns versus solve differently? This file lists eight categories. Each is one paragraph plus a "Myrhiza disposition" note: do we inherit, partially-inherit, sidestep, or solve differently.

The spec authors should treat this as a checklist when designing Myrhiza primitives. If a problem is on this list and the runtime spec does not have a story for it, that's a gap.

Cross-references: `../spritely-ocapn/open-problems.md` shares many of these (it has the ocap-discipline ones and the OCapN-cross-impl ones); `../iroh/open-problems.md` covers the transport-layer ones.

## 1. WASM is missing from the equation

Agoric chose JavaScript, not WASM. That is a path-not-taken whose costs and benefits Agoric has been paying for since 2018. JS gave them a familiar developer pool, mature tooling, and TC39 leverage. It cost them: cross-language guests (only JS in vats), JIT-vs-interpreter perf tradeoffs (forced to XS for determinism), and the SES-shaped tax of bolting confinement onto a language that was never designed for it. The Agoric team has occasionally floated "WASM in vats" ([agoric-sdk#1127](https://github.com/Agoric/agoric-sdk/issues/1127)) but it has not landed. The structural reason: vat upgrade, snapshot, and replay are all designed around the JS object graph; switching to WASM would mean re-architecting the kernel.

**Myrhiza disposition:** **solve differently.** WASM Component Model is our primary engine. We get cross-language guests, structural determinism, smaller engine surface, and isolation-by-construction. We pay a different cost: WIT / Component Model is younger and tooling is less mature. Worth it.

## 2. GC determinism is hard, and the fix is engine-cost

[agoric-sdk#2615 ("sufficiently-deterministic GC")](https://github.com/Agoric/agoric-sdk/issues/2615) is closed but the underlying problem is structural: V8 / SpiderMonkey GC timing is non-deterministic, which means you cannot use them on-chain because validators must reach the same answer about *when* a finalizer fires. Agoric's solution is to forbid those engines on chain — chain vats run only XS, which has more predictable GC. Solo (off-chain) vats can use V8/Node and accept eventual consistency, with `WeakRef` + `FinalizationRegistry` deferred to crank boundaries. The cost is real: XS is slower than V8; the chain runtime is decoupled from the off-chain runtime in subtle ways; and the "sufficiently deterministic" bar requires careful per-engine review.

**Myrhiza disposition:** **partially sidestep.** WASM linear memory has no GC by construction. Component Model can include GC types (the WASM GC proposal exists), but for state-apply components we should be wary: anything that introduces non-deterministic destructor timing reintroduces this exact problem. Spec rule of thumb: state-apply components must not rely on weakrefs / finalizers; references-as-capabilities must be explicitly held or explicitly dropped. This is a `state-apply` constraint, not a runtime-wide ban.

## 3. Distributed GC across vats and machines is research-active

CapTP includes drop messages — when a remote holds a reference and lets it go, the holder sends `drop` to let the exporter free. In a partitioned or unreliable network, drop messages can be lost, leaving the exporter unable to reclaim. This is the classic distributed GC problem (Birrell, Nelson, Owicki, 1990s). Goblins, Cap'n Proto, and Endo CapTP each have answers, none fully satisfying. Cross-machine three-vat handoffs make it worse: vat A grants a reference vat B holds, B introduces it to C; now A must know about the path A→B→C to GC correctly. Practical implementations heuristically retain references and rely on session lifecycles to bound leaks.

**Myrhiza disposition:** **partially inherit.** If we do peer-to-peer ocap reference passing (likely, via OCapN/CapTP), we inherit this. Mitigation: lean on the OCapN spec rather than reinventing; ensure session-bounded retention is the default; reference handoffs across more than two peers should be rare and explicit in the API.

## 4. Vat upgrade is improving but not fully solved

Vat upgrade — replacing a vat's code while preserving its identity, exports, and persistent state — is a hard problem because external references must continue to resolve to "the same vault" even after the vault implementation changes. Agoric has shipped a usable upgrade mechanism ([agoric-sdk#1848](https://github.com/Agoric/agoric-sdk/issues/1848), [#5811](https://github.com/Agoric/agoric-sdk/issues/5811)) but the model is still maturing: kernel-mediated identity preservation, durable kinds vs ephemeral, baggage migration, etc. Rolling upgrades across a live mainnet of validators add another layer (chain governance must coordinate the upgrade; bad upgrades cannot be cheaply rolled back).

**Myrhiza disposition:** **inherit, with a different surface.** Component upgrade is a real problem regardless of the engine. WASM Component Model gives us versioned worlds and can express "instance A and instance B export the same world" — which is a building block for upgrade — but the durable-state migration story (taking the prior state, mapping to the new component's state shape) is on us to design. This is a load-bearing spec topic; treat it as not-yet-solved.

## 5. Cross-implementation OCapN is still draft

OCapN ([ocapn.org](https://ocapn.org/), [github.com/ocapn/ocapn](https://github.com/ocapn/ocapn)) intends to make CapTP an interoperable wire protocol across Agoric, Spritely Goblins, Cap'n Proto, and future implementations. As of May 2026 the specification is a draft. There are demo-grade interop examples; there is not production-grade cross-impl interop. Wire-format harmonization (Capdata vs Cap'n Proto schema vs Goblins serialization), session lifecycle, and three-party introduction details all remain in flight.

**Myrhiza disposition:** **track upstream; commit late.** Bind to OCapN if/when it stabilizes; until then, define our own ocap wire protocol with an explicit migration path to OCapN. Don't ship a hand-rolled non-OCapN protocol that would later require a flag-day migration. Documenting "we will adopt OCapN when stable" in the master spec is itself the design decision.

## 6. Browser SwingSet is not a thing

`agoric-sdk` runtime is Node-only. Hardened JS / SES runs in browsers (it has to — MetaMask Snaps run in a browser-extension context). But the *vat host* — the SwingSet kernel that runs vats with persistence and cross-vat eventual send — only runs in Node. There is no in-browser vat host. The architectural reason: vat persistence relies on Node fs APIs and on xsnap subprocesses, neither of which is browser-compatible. Endo's `endo-bundle` and the compartments work make vat-shaped code *runnable* in a browser, but not *hosted*.

**Myrhiza disposition:** **solve differently.** Myrhiza is P2P-runtime; the host can be any platform that runs WASM. Browser-host (via wasmtime-wasi or similar bindings adapted for browser) is in scope; mobile-host is in scope. We should not inherit Agoric's "Node only" assumption. Make it explicit in the master spec that Myrhiza's host is platform-agnostic by design.

## 7. Hardened JS is a sandbox, not a determinism guarantee

This one is subtle and easy to get wrong. SES makes JS *safe to share* — it freezes primordials, removes ambient authority, isolates compartments. SES does **not** make JS *deterministic*. JS has many sources of non-determinism: GC timing, JIT timing, `Math.random`, `Date.now`, integer-precision in legacy code. Agoric gets determinism not from SES but from a separate discipline: removing access to non-deterministic APIs (`Math.random`, `Date.now`, network, fs) at the compartment-creation boundary, and choosing XS as the engine to bound JIT-related variance. SES is necessary; it is not sufficient. ([Per the Agoric platform docs](https://docs.agoric.com/platform/), the determinism story explicitly involves XS engine choice + compartment endowments curation, not lockdown alone.)

**Myrhiza disposition:** **inherit the discipline, not the implementation.** WASM Component Model gives us the equivalent of "SES freezes primordials" for free — there are no shared mutable primordials. But we still have to curate which kernel/WASI imports are exposed to state-apply components. Clock, randomness, network, fs: all must be brokered or absent. Document this in the master spec as "deterministic-imports policy." This is the load-bearing spec rule that future Myrhiza authors will most often want to bend; the spec must say no.

## 8. Determinism vs. performance is a real engineering tradeoff

Closely related to (2) and (7), but worth its own paragraph. Agoric ran the experiment: when you commit to deterministic execution, you give up JIT, you give up V8, you give up fast crypto via off-chain workers, you accept slower contracts. For Inter Protocol's vault throughput, that was fine. For "TPS leader" use cases (DEXes, NFT minting, gaming), it is not. Agoric's pivot to Orchestration is partly a tacit acknowledgment: rather than try to be the high-throughput chain, be the secure cross-chain coordinator that talks to higher-throughput chains.

**Myrhiza disposition:** **own the tradeoff explicitly.** WASM execution is faster than XS-interpreted JS but slower than V8-JIT JS. We are not choosing JIT, so we are not in the same regime as a JIT-ed runtime. State-apply must be deterministic; pre-check uses the same code path; state-propose has more latitude (kernel re-checks anyway). For interaction and behavior components, we can relax determinism. Spec the four profiles' performance regimes clearly so app authors don't expect Solana-class throughput from a state-apply component.

## 9. Single-engine lock-in

Agoric's chain commits to XS for vat execution. If XS has a bug, every chain vat has the bug; if the XS team stops maintaining XS, Agoric inherits the maintenance burden. Moddable is a small company; XS is a small project. This is a real concentration risk that nothing in the architecture mitigates. Agoric's bet is that XS plus Agoric internal patches plus TC39 representation is enough to keep XS healthy.

**Myrhiza disposition:** **partially inherit, partially mitigate.** WASM has multiple production runtimes (wasmtime, wasmer, V8, JSC, SpiderMonkey, Bytecode Alliance ecosystem). We pick a primary (likely wasmtime or wasmi for embedded targets) but the WASM bytecode is portable across engines, so engine-switch cost is finite. We are not as locked in as Agoric is to XS. Still, "what if our chosen engine breaks" is a real spec-level concern; build component bundles such that re-running them on a different engine is testable.

## 10. Bridge / orchestration is not a substitute for native primitives

Agoric's 2024 Orchestration pivot is conceptually a wrapper over IBC: vats can construct cross-chain workflows that hold capabilities to remote-chain accounts and assets. This is genuinely useful — Fast USDC works because of it. But it does not give Agoric apps the *primitives* of the remote chains. Calling an Ethereum DEX from an Agoric vat is still a foreign-call; the DEX semantics are not in the Agoric type system. Cross-chain orchestration is plumbing, not composition.

**Myrhiza disposition:** **avoid this trap.** P2P apps in Myrhiza compose locally (via state-apply purity and event sourcing) and only escape to the network for sync. We do not have a "remote chain" to orchestrate against; we have peers running the same component bundles. Composition is in our type system because it's all our type system. This is one of the core advantages of the P2P-runtime model over the chain-runtime model; don't accidentally lose it by adopting Agoric-style "remote service" patterns where a peer-local pattern would do.

## Implications for Myrhiza

To summarize the dispositions:

| Open problem | Myrhiza disposition |
|---|---|
| 1. WASM not in the equation | Solve differently — Component Model first |
| 2. GC determinism | Partially sidestep — no GC in linear memory; constrain `state-apply` |
| 3. Distributed GC across machines | Partially inherit — adopt OCapN session-bounded retention |
| 4. Vat (component) upgrade | Inherit — load-bearing open spec topic for Myrhiza too |
| 5. Cross-impl OCapN draft | Track upstream; commit late |
| 6. Browser host | Solve differently — host platform-agnostic by design |
| 7. Hardened-sandbox ≠ determinism | Inherit the discipline; spec deterministic-imports policy |
| 8. Determinism vs. perf | Own explicitly; spec per profile |
| 9. Single-engine lock-in | Partially mitigate — WASM portability gives us slack |
| 10. Orchestration ≠ composition | Avoid — peer-local composition is a P2P advantage |

The four marked "inherit" or "load-bearing": **vat/component upgrade, deterministic-imports policy, distributed-GC over OCapN, and per-profile performance regimes.** These should each have a section in the master Myrhiza spec or a dedicated design note in `docs/specs/`. Without those, we are pretending we have answers we don't have.

## Sources

- https://github.com/Agoric/agoric-sdk/issues/511 — heap snapshots / vat replay overhead
- https://github.com/Agoric/agoric-sdk/issues/1127 — vat-container options (XS, Worker, WASM)
- https://github.com/Agoric/agoric-sdk/issues/1848 — kernel API for upgrading vats
- https://github.com/Agoric/agoric-sdk/issues/2615 — sufficiently-deterministic GC
- https://github.com/Agoric/agoric-sdk/issues/5811 — tentative API to upgrade static vats
- https://docs.agoric.com/platform/ — Agoric platform overview
- https://www.moddable.com/hardening-xs — Moddable XS hardening
- https://github.com/ocapn/ocapn — OCapN draft spec
- https://ocapn.org/ — OCapN pre-standardization group
- https://hardenedjs.org/ — Hardened JS / SES
- https://github.com/tc39/proposal-compartments — Compartments proposal (Stage 1)
- ../spritely-ocapn/open-problems.md
- ../iroh/open-problems.md
