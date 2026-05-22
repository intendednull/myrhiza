**Date:** 2026-05-22
**Status:** active
**Subject:** What formal methods structurally don't solve — the honest gap

# Open problems (what formal methods don't solve)

Formal methods are powerful for **protocol design** (TLA+) and **concurrent-code bug detection** (Loom/Kani). They are not a correctness guarantee. This file enumerates the gaps that survive even after a serious formal-methods investment, so Myrhiza specs and marketing don't oversell what's been done.

## 1. Spec-implementation gap

The biggest open problem: **TLA+ verifies the spec, not the code.** You can have a perfect TLA+ spec of a protocol and a wildly buggy implementation, and TLA+ catches none of the implementation bugs. The bridge between spec and code is:

- Code review
- Property-based / fuzz tests
- Manual translation from spec to code
- (At AWS, sometimes) a separate Kani or P verification of the code

Bridging the gap automatically is **possible but expensive** — the technique is *refinement proof*, where you prove the implementation refines (simulates) the spec. CompCert does this for the C compiler in Rocq. The cost is ~10 person-years per nontrivial system. Out of scope for Myrhiza.

What this means for Myrhiza practice: when a TLA+ spec ships, document explicitly *which part* of the system the spec covers and *which part* relies on code review. Don't let "we have a TLA+ spec" creep into "this is verified."

## 2. Liveness is harder than safety

Most TLA+ specs in practice check only **safety** properties — "no bad state is reachable." **Liveness** properties — "every request eventually gets a response", "the system eventually converges" — require *fairness assumptions* and dramatically expand the search space.

Fairness is itself slippery: "weak fairness" (an action enabled forever is eventually taken) is the common default, but real distributed systems often need finer-grained fairness (specific actors being scheduled, specific channels not being starved). Specifying fairness wrong gives you a spec that satisfies the wrong liveness.

For Myrhiza: assume the TLA+ spec covers safety. Liveness will mostly be argued informally. This is a real gap; AWS handles it the same way.

## 3. Bounded-model checking is bounded

TLC bounds the state space by enumeration; Apalache bounds by trace depth; Kani bounds by unwind depth. **None of them verify behavior beyond the bound.** A bug that requires 12 protocol steps to trigger, when your bound is 10, is invisible.

The standard mitigation: argue *informally* that the bound is sufficient — bugs of greater depth are vanishingly unlikely to differ from bugs of bound-depth, and the bound is large enough to capture the interesting interleavings. This is a real argument, not handwaving, but it is *informal*. It can be wrong.

A more rigorous mitigation: prove an *inductive invariant* — an invariant that, if true in any state, remains true after any transition. Inductive invariants verify unbounded behavior. Apalache supports inductive-invariant checking; TLC does not. Most real specs do not bother — TLC + informal bound-argument is the standard practice.

## 4. Memory model is incomplete

[Loom](loom.md)'s own README acknowledges: it doesn't fully implement C11. Known limitations around SeqCst and load buffering. Bugs that depend on the unmodeled regions of the memory model are invisible to Loom.

The C11 memory model itself has known surprises — "thin-air reads" are a live academic question with no clean resolution. Rust inherits C11; Rust inherits the surprises.

What this means: Loom passing is *not* a proof of correct concurrent behavior on real hardware. It is strong evidence — much stronger than `cargo test` — but not a proof.

## 5. Async and Loom are an uneasy fit

Loom permutes thread schedules. Real Rust uses `tokio::spawn` heavily. Loom's affordances for async are limited; the Tokio team's `cfg(loom)` tests reconstruct a custom executor to make Loom and async cooperate. This pattern is reusable but non-trivial.

The honest framing: Loom is excellent for testing **the synchronization primitives that async runtimes build on**. It is weaker for testing **arbitrary async application code**. For Myrhiza, this means kernel primitives get Loom tests; high-level integration tests get Shuttle or standard property-based testing.

## 6. No tool covers WASM-guest behavior

All the tools in this folder run on the *host*. WASM components are guests. The host runtime's behavior is testable; the inside of a WASM guest is not — at least not by these tools. (There exist WASM-native verification efforts — the Wasm-R3 paper, Spectec, the Wasm reference interpreter — but none are integrated into the host-side stack.)

What this means: Myrhiza's `state-apply` components, when implemented as WASM, are *not* covered by Kani or Loom on the host. The host's invocation of the WASM is covered. The WASM's own logic is the component author's responsibility — they can use whatever language-specific verification they like inside the component.

This is a real gap, and the canonical mitigation is the **bandit-mode determinism check**: the kernel runs `state-apply` in dry-run mode and compares the result to the proposer's claim. The dry-run check catches *behavior* mismatches, not *implementation* bugs in the WASM — but it's the strongest signal available.

## 7. Adoption rot

A TLA+ spec that nobody maintains is worse than no spec — it's a false signal of correctness. As the implementation drifts from the spec, the spec rots. AWS's CACM 2025 paper notes the discipline cost explicitly: keeping specs in sync with code is *more* expensive than writing them in the first place.

What this means for Myrhiza: budget for spec maintenance. A reasonable policy: **specs that ship must have a named owner, and the owner must approve any change to the corresponding code path**. Otherwise, mark the spec `[archived]` honestly.

## 8. No formal verification of the verifiers

TLC is written in Java; Apalache is Scala backed by Z3; Kani uses CBMC which uses Z3 / bitwuzla / cvc5. None of these tools are themselves verified. Bugs in TLC have been found (and fixed); bugs in Z3 have been found (and fixed). A spec that TLC says is safe might still have a bug, if TLC has a bug.

In practice this is a tiny effect — these tools have been used on hundreds of real specs and the major bugs are caught. But it is a real caveat for extreme-assurance claims. The TLAPS proofs Rocq/Lean produce are checked by smaller, audited *kernels*; that is the design intent of dependently-typed proof assistants. Whether the kernel-check matters for your use case depends on the assurance ceiling.

## 9. Formal methods do not prevent operational outages

The 2017 Cloudflare Cloudbleed bug was a buffer overrun in the HTML parser. No amount of TLA+ would have caught it (wrong layer); Kani could have if applied; Loom couldn't (no concurrency). Most production outages are *not* in the protocol-design layer that TLA+ covers — they are in code, config, dependencies, the operations layer.

Formal methods are a *high-leverage* defense against a *specific* class of bugs (design-time concurrency/distributed-system bugs, function-level invariant violations). They are not a defense against operational incompetence, config drift, dependency vulns, or human error. Don't market them as one.

## 10. The proof-assistant gap

Rocq, Lean 4, and TLAPS can prove things model-checkers can't — unbounded behavior, dependently-typed invariants, mechanically-verified theorems. But the cost is 10-100x higher per line of proof.

Myrhiza is choosing the *cheap subset* of formal methods (model-checking only). This is the right choice for the team size. Be honest that it is a choice — the assurance ceiling is lower than what the proof-assistant subset offers. If Myrhiza later wants higher assurance (verified cryptography, verified kernel, anything safety-critical), the proof-assistant tier comes back into scope.

## Implications for Myrhiza

These open problems do *not* invalidate the [`lessons.md`](lessons.md) recommendations. The recommendations are still the right investment — they just don't claim what they don't claim.

Specifically:

- **Document the spec/implementation distinction prominently.** When a TLA+ spec ships, document the exact scope ("this models the `state-apply` ordering between peers; the Rust implementation of `state-apply` is *not* verified to refine this spec; correspondence is maintained by code review and property tests").
- **Document the bound on every model-checked property.** "TLC verified under a 3-peer, 5-event bound" is honest. "Verified" is not.
- **Document the assurance tier in marketing copy.** "Uses formal methods" is honest. "Provably correct" is not.
- **Plan for spec rot.** Every spec needs a named owner and a review trigger.

The combination of "we have done formal-methods work + we are honest about what it covers" is far more valuable than "we have done formal-methods work + we claim it covers everything." The first earns trust; the second loses it the moment anyone reads carefully.

## Sources

- "How Amazon Web Services Uses Formal Methods", CACM 2015: https://cacm.acm.org/research/how-amazon-web-services-uses-formal-methods/
- "Systems Correctness Practices at AWS", CACM/Queue 2024–2025: https://queue.acm.org/detail.cfm?id=3712057
- Loom limitations (README): https://github.com/tokio-rs/loom#testing-from-async-fns
- CompCert (refinement proof example): https://compcert.org/
- C11 thin-air reads (academic): https://dl.acm.org/doi/10.1145/3093333.3009850
- Cloudflare Cloudbleed: https://blog.cloudflare.com/incident-report-on-memory-leak-caused-by-cloudflare-parser-bug/
- [`prior-art/holochain/open-problems.md §10`](../holochain/open-problems.md) — same gap, framed from another corpus
- [`prior-art/spritely-ocapn/open-problems.md §8`](../spritely-ocapn/open-problems.md) — same gap, framed from another corpus
