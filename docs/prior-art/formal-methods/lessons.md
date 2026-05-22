**Date:** 2026-05-22
**Status:** active
**Subject:** Lessons for Myrhiza — what to adopt, what to skip, with cost framing

# Lessons for Myrhiza

This is the **decision file**. If a Myrhiza spec or component touches formal methods and you have time to read one file, read this one. Other files in this folder are evidence; this is the synthesis.

The Myrhiza-stance question is **"what is the cheapest formal-methods investment that pays back on a small-team Rust P2P runtime?"**. The answer below assumes a small team, deterministic `state-apply`, capability-mediated host surface, and a wire protocol still under design.

## Validates

These are things Myrhiza's existing design already gets right that formal methods would *reinforce*, not change.

- **Determinism as a load-bearing property.** The CLAUDE.md note "*state-apply components must be pure functions of (prior state, event)*" is exactly the shape TLA+ is designed for. A TLA+ spec of state-apply ordering compiles directly to this assumption. Don't relax determinism for any reason — formal methods amplify the value of it.
- **Capability surface as the only host boundary.** Capabilities are state machines (mint → delegate → use → revoke). A TLA+ spec of capability lifecycle is short (≤150 lines), checks fast, and finds delegation/expiry edge cases by construction. The narrow host-surface design choice is what makes this tractable.
- **Bounded state.** Per [`prior-art/holochain/open-problems.md §10`](../holochain/open-problems.md), Myrhiza's core state machines (state-apply ordering, component-link integrity, capability-token check) are *bounded* — finite participants, finite events per spec — which is exactly the regime TLC and Apalache handle well. Unbounded state machines would push toward TLAPS or Rocq, which is out of budget.
- **Rust as the runtime substrate.** Loom and Kani are Rust-native and cheap. A non-Rust runtime would lose half this folder's recommendations. Sticking with Rust pays back on formal methods specifically.

## Borrow

These are concrete adoption recommendations, ordered by priority and cost.

### Tier 1 — adopt before 1.0

These have the highest return-per-hour of any formal-methods investment.

1. **One TLA+ spec of `state-apply` ordering and convergence.** Target: 100–200 lines of PlusCal. Bound: 3 peers, 5 events, 2 event-types. Check under TLC. Validates the determinism assumption and finds ordering bugs before code locks. **Estimated cost: 1–2 weeks for one engineer who has not used TLA+ before**, per Hillel Wayne's [Learn TLA+](https://learntla.com/) ramp-up estimates and AWS's "2–3 weeks to productivity" framing. The bug-find from this spec, if any, is worth the entire investment by itself. See [`tla.md`](tla.md) for the workflow.

2. **One TLA+ spec of capability-token lifecycle.** Mint → delegate → use → revoke. Safety invariants: "no use after revoke", "delegation chain depth bounded", "expired tokens fail check". Smaller and faster than the state-apply spec. **Estimated cost: 3–5 days** once Tier 1.1 has built tooling familiarity. Reuse the same TLC/Apalache infrastructure.

3. **Loom adoption for kernel synchronization primitives.** Cost is essentially zero — add `loom = { version = "0.7", optional = true }` to dev-deps, write `cfg(loom)`-gated tests for every `pub` mutable surface in the kernel (capability table, broker queue, event log append, persistence). **Estimated cost: ~1 day per primitive**, ongoing. See [`loom.md`](loom.md).

4. **Kani-verify the capability-token decoder and the wire-format parser.** These are the two highest-impact targets: catastrophic if buggy, both touch untrusted input, both function-shaped. Kani is built for this. **Estimated cost: 2–4 days per function**, including learning curve. See [`kani.md`](kani.md).

### Tier 2 — adopt opportunistically

5. **TLA+ spec of the wire protocol** once the wire shape is stable. Same pattern as Tier 1.1, applied to the gossip/replication layer. Hold off until the wire is converging — too early, you spec a moving target.
6. **Shuttle for end-to-end multi-peer integration tests.** When Loom can't bound the test, Shuttle's PCT mode gives randomized scheduling at scale. Cheap to add once a Loom test has demonstrated need.
7. **Kani contracts (`#[kani::requires]` / `#[kani::ensures]`) on critical kernel functions.** Compositional proofs — verify a function's contract, assume it when verifying callers. Use sparingly; over-annotation costs more than it gains.

### Tier 3 — don't invest unless scope expands

8. **Rocq or Lean 4 proofs of anything.** Out of budget for a small team. Re-evaluate only if Myrhiza scope expands to a verified compiler, verified cryptography, or formal-methods-as-a-product.
9. **TLAPS proofs of TLA+ specs.** Model-checking with TLC is far cheaper and catches the same bugs in practice. TLAPS is the right tool when you need a *machine-checked theorem* (e.g. for academic publication or extreme-assurance certification); not for "we want to ship a P2P runtime."
10. **Whole-runtime verification.** Not a thing. The decomposition above (TLA+ at protocol, Loom at concurrency, Kani at parsers) is the realistic shape; "verify the whole runtime" is a different industry's R&D project.

## Avoid

Anti-patterns to avoid, learned from adjacent prior-art and from formal-methods adoption stories.

- **Don't oversell what's verified.** A TLA+ spec verifies the *design*. The code can still be wrong. Documentation that says "this protocol is formally verified" without qualifying *what* is misleading and erodes trust. AWS's CACM papers are scrupulous about this; copy their framing.
- **Don't pretend Loom proves correctness.** Loom proves no bug exists *in the bounded model under the C11 memory model*. Real Rust has many more bugs than that — logic errors, missing cases, panics from upstream crates. Loom is one layer of defense; it is not a correctness guarantee.
- **Don't adopt TLA+ everywhere.** The bug-find curve is convex — the first spec finds many bugs, the tenth spec finds few. AWS uses TLA+ on perhaps a dozen of their hundreds of services. One excellent spec is far more valuable than five mediocre ones.
- **Don't write specs only one person can read.** A TLA+ spec that lives in one engineer's head and rots when they leave is worse than no spec — it's a false signal of safety. Specs need maintainership. Budget for it.
- **Don't ship a custom DSL that "compiles to TLA+".** Multiple teams have built such DSLs (Quint is a *general-purpose* one and is fine; in-house Myrhiza-specific ones are not). The maintenance cost dwarfs the writing-ergonomics gain.
- **Don't conflate the three layers.** Treating TLA+, Loom, and Kani as alternatives ("we picked TLA+, so we don't need Loom") is the single most common adoption mistake. They are *layered* — protocol, memory model, function. Each catches bugs the others don't.
- **Don't skip Kani because "Rust's type system catches everything."** Rust's type system catches *some* things. `unsafe` paths, integer arithmetic, parser logic, decoder state machines — all outside the type system's reach. Kani catches what the type system misses.
- **Don't trust formal-methods marketing.** "Provably correct" claims in marketing copy usually mean "we have a TLA+ spec of one component." Check the actual scope. (Apply this to Myrhiza's own future marketing.)

## The convex-bug-find-curve recommendation

The most actionable framing: **adopt formal methods at the *most concentrated* points of risk first.**

Concentrated risk in a P2P runtime is:

1. **State-apply ordering** — multi-peer convergence depends on it.
2. **Capability lifecycle** — security depends on it.
3. **Wire-format parsing** — anything untrusted bytes touch.
4. **Kernel concurrency primitives** — corruption silently breaks everything else.

For each: pick the *cheapest* tool that catches the bug shape. TLA+ for (1) and (2). Kani for (3). Loom for (4). Don't pick a heavier tool than the bug shape requires.

After Tier 1 ships, evaluate. The convexity means the marginal value of Tier 2 should be measured against actual bug-finds in Tier 1. If Tier 1 found 5 design bugs, Tier 2 is worth pursuing. If Tier 1 found 0, either the spec was wrong (likely) or the design is solid enough that Tier 2 has lower expected value.

## Where this folder ends and other prior-art begins

- For the *content* of state-apply ordering — what is the state machine, what are the events — see [`prior-art/willow/`](../willow/) and the Myrhiza master spec.
- For the *wire format* — what bytes go on the network — see [`prior-art/iroh/`](../iroh/), [`prior-art/willow/`](../willow/).
- For the *capability semantics* — what is mint/delegate/use/revoke shaped like — see [`prior-art/spritely-ocapn/`](../spritely-ocapn/), [`prior-art/agoric-endo/`](../agoric-endo/).

This folder answers the *tooling* question. The *what-to-spec* questions live in those folders.

## Sources

- [`tla.md`](tla.md), [`loom.md`](loom.md), [`kani.md`](kani.md), [`adoption.md`](adoption.md), [`comparisons.md`](comparisons.md), [`open-problems.md`](open-problems.md) — synthesized here
- [`prior-art/holochain/open-problems.md §10`](../holochain/open-problems.md) — the originating "gap to not repeat" framing
- [`prior-art/spritely-ocapn/open-problems.md §8`](../spritely-ocapn/open-problems.md) — TLA+ + Loom before 1.0 recommendation
- "How Amazon Web Services Uses Formal Methods", CACM 2015: https://cacm.acm.org/research/how-amazon-web-services-uses-formal-methods/
- "Systems Correctness Practices at AWS", CACM/Queue 2024–2025: https://queue.acm.org/detail.cfm?id=3712057
- Hillel Wayne's Learn TLA+: https://learntla.com/
