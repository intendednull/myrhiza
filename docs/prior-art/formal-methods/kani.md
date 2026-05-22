**Date:** 2026-05-22
**Status:** active
**Subject:** Kani — Rust bounded model checker

# Kani

[Kani](https://github.com/model-checking/kani) is a Rust bounded model checker. You write a `#[kani::proof]` function that calls real Rust code on `kani::any()` symbolic inputs and asserts a property; Kani translates the Rust → MIR → Goto-C → SMT and asks the [CBMC](https://github.com/diffblue/cbmc) bounded model checker whether *any* input could make the assertion fail. If yes, you get a counterexample input.

Kani was AWS-incubated (originated inside Amazon, now stewarded under the [model-checking](https://github.com/model-checking) GitHub organization rather than the `awslabs` org — note this is a deliberate distancing, not abandonment; AWS engineers remain the primary contributors). Current release **0.67.0** on crates.io 2026-01-16; license **Apache-2.0 OR MIT**; CBMC as the back-end SMT-frontend; primary supported SMT solvers are bitwuzla, cvc5, and Z3 (Kani 0.65+).

## What Kani does

The shape:

```rust
#[kani::proof]
fn capability_check_never_panics() {
    let cap: u64 = kani::any();
    let token: Token = kani::any();
    // Real production function, called on symbolic inputs.
    let _ = verify_capability(cap, token);
}

#[kani::proof]
#[kani::unwind(10)]
fn capability_check_rejects_revoked() {
    let token: Token = kani::any();
    kani::assume(token.is_revoked());
    assert!(!verify_capability(token.capability, token).is_ok());
}
```

Kani lifts the function into Goto-C, encodes "for all inputs satisfying `kani::assume`, the assertion holds" as an SMT formula, and proves or finds a counterexample. The output is concrete: "for `token = Token { capability: 0xDEADBEEF, expiry: 0, revoked: true }`, `verify_capability` returned `Ok(())`."

Kani is **bit-precise** — it models integer arithmetic with the actual bitwidths, catches overflow/underflow, models `unsafe` Rust including raw pointer arithmetic, and detects undefined behavior. The bit-precision is what distinguishes it from deductive verifiers like Prusti (which model integers as mathematical, not machine, integers).

## Tradeoffs

**Strengths.**

- **Verifies real Rust code.** Not a model of the code, not a translation to a different language — the actual function, called on symbolic inputs. The spec-vs-implementation gap that haunts TLA+ doesn't exist here.
- **Handles `unsafe`.** This is the load-bearing capability. Kani found bugs in Firecracker's rate limiter and VirtIO stack — both `unsafe`-heavy paths — that were "intractable to traditional methods" (AWS's framing). [Blog post](https://model-checking.github.io/kani-verifier-blog/2023/08/31/using-kani-to-validate-security-boundaries-in-aws-firecracker.html).
- **Function contracts.** Kani supports `#[kani::requires]` / `#[kani::ensures]` annotations, letting you compose proofs — verify a function's contract, then assume the contract when verifying its callers. This avoids re-verifying the whole call tree.
- **Loop invariants and SMT-solver choice.** Kani 0.65 added support for choosing between bitwuzla, cvc5, and Z3; Kani 0.66 added loop invariant support for `while let` loops. The toolchain is actively maturing.
- **CBMC heritage.** CBMC is 20+ years of bounded-model-checking R&D applied to C. Kani inherits that maturity.

**Limitations.**

- **Bounded.** Kani verifies up to a finite unwind depth. A function with an unbounded loop requires either a loop invariant annotation (which Kani then checks separately) or an `#[kani::unwind(N)]` cap. Beyond N, behavior is not verified.
- **State-space blow-up is real.** Symbolic verification with bit-precise reasoning is expensive. Kani proofs of complex functions (parsers, codecs, anything iterating over a non-trivial data structure) routinely take minutes to hours, and some don't terminate within practical budgets.
- **Doesn't scale to whole-system properties.** Kani is per-function. "Does the runtime converge?" is not a Kani question. "Does this specific bit-packing decoder ever panic on malformed input?" is.
- **`async` support is incomplete.** Kani has growing support for `async` Rust but historically has lagged. Async-heavy code paths are still better tested with [Loom](loom.md) or via separating sync state-machine logic that Kani can prove from the async runtime.
- **Tooling surface is rougher than `cargo test`.** Kani ships as a `cargo` subcommand (`cargo kani`) and reuses CBMC under the hood, but the error messages and proof debugging experience are noticeably less polished than the surrounding Rust ecosystem. Counterexamples are concrete and useful; the *infrastructure* of running and re-running proofs is rougher.

## Kani vs Loom vs TLA+

These three are not competitors — they live at different layers:

| Layer | Tool | What it verifies |
|---|---|---|
| Protocol | [TLA+](tla.md) | Design-level state-machine properties (safety, liveness, convergence) |
| Concurrent code | [Loom](loom.md) | Memory-model bug detection for *interleavings* of synchronization primitives |
| Sequential code | Kani | Bit-precise properties of individual functions on arbitrary inputs |

A team that's serious about correctness on a Rust P2P runtime adopts all three for the parts that suit each tool. AWS does exactly this: TLA+ at protocol design, Kani at the function level (Firecracker, S3 internals), and randomized/permutation testing (the Shuttle and Loom lineage) for concurrency.

## Adjacent Rust verification tools

Worth knowing about, not part of Myrhiza's adoption path:

- **[Prusti](https://github.com/viperproject/prusti-dev)** — ETH Zurich, deductive verifier via Viper. Strong on functional correctness with contracts; weaker on `unsafe`. Models integers as mathematical, not bit-precise.
- **[Creusot](https://github.com/creusot-rs/creusot)** — French academic (INRIA) project, deductive verifier using prophecy variables to model Rust's mutable references. Rust-only annotations, no separation logic. Supports more borrowing patterns than Prusti.
- **[Verus](https://github.com/verus-lang/verus)** — Microsoft Research, SMT-backed verifier with a Rust-syntax superset. Designed for systems programmers.

All three are deductive (contract-based, theorem-proving-shaped); Kani is the only major Rust bounded model checker. The Rust Formal Methods Interest Group (rust-formal-methods.github.io) maintains a fuller landscape.

## Implications for Myrhiza

Kani's sweet spot for Myrhiza is **bit-level invariants on specific functions in the kernel surface**:

- **Capability-token decoders.** The "given a serialized capability token, does the decoder either reject it or produce a valid in-memory representation" property. Kani-shaped.
- **Wire-format parsers.** Same shape — any deserializer that touches untrusted bytes is a prime Kani target. Firecracker's VirtIO parser is the production analogue.
- **Bit-packed state representations** (e.g. a state-digest encoding) — Kani's bit-precision is its strength here, where Prusti/Creusot would lose precision.
- **The capability-token verification function itself.** Kani can prove "no revoked token passes verification" as a contract.

What *not* to put in Kani: anything async-heavy, anything that requires reasoning about the whole runtime's behavior, anything where the function-level invariant isn't well-defined. For those, TLA+ at the spec layer and Loom at the concurrency layer are the right tools.

A reasonable first investment: **Kani-prove the capability-token decoder and the wire-format parser before either ships to production.** Each is a focused function-level question that Kani is designed for, the bugs it would find are catastrophic, and the cost is "one `cargo kani` invocation in CI."

## Sources

- Kani: https://github.com/model-checking/kani — Apache-2.0 OR MIT, 0.67.0 (2026-01-16)
- Kani docs: https://model-checking.github.io/kani/
- Kani crates.io: https://crates.io/crates/kani-verifier
- Kani Firecracker case study: https://model-checking.github.io/kani-verifier-blog/2023/08/31/using-kani-to-validate-security-boundaries-in-aws-firecracker.html
- Kani Firecracker example: https://model-checking.github.io/kani-verifier-blog/2022/07/13/using-the-kani-rust-verifier-on-a-firecracker-example.html
- Kani vs other tools: https://model-checking.github.io/kani/tool-comparison.html
- CBMC: https://github.com/diffblue/cbmc
- Rust Formal Methods Interest Group: https://rust-formal-methods.github.io/
- "Surveying the Rust Verification Landscape" (Le Blanc, 2024): https://arxiv.org/abs/2410.01981
- Prusti project paper: https://pm.inf.ethz.ch/publications/AstrauskasBilyFialaGrannanMathejaMuellerPoliSummers22.pdf
- Creusot paper (Denis et al., 2022): https://www.researchgate.net/publication/364287862_Creusot_A_Foundry_for_the_Deductive_Verification_of_Rust_Programs
