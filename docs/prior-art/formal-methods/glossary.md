**Date:** 2026-05-22
**Status:** active
**Subject:** Glossary — formal-methods terms used in this folder

# Glossary

Terms a Myrhiza reader will see across [`tla.md`](tla.md), [`loom.md`](loom.md), [`kani.md`](kani.md), [`adoption.md`](adoption.md), [`comparisons.md`](comparisons.md), [`open-problems.md`](open-problems.md). Definitions are scoped to the meanings used in this folder; broader formal-methods terminology varies by sub-community.

## Algorithms and techniques

**Bounded model checking.** Verification by exploring states or traces up to a finite depth (number of states, trace length, loop unwinds). [Kani](kani.md) and [Apalache](tla.md) (in symbolic mode) both bounded-model-check. The bound is the limit of the guarantee: a property holds *up to* the bound. Compare with *unbounded* verification (theorem proving, inductive invariants).

**CBMC** (C Bounded Model Checker). Mature bounded model checker for C/C++; Kani's backend. Originated at CMU, maintained by Diffblue. 20+ years of research backing.

**CDSChecker.** The 2013 OOPSLA paper (Norris & Demsky) that Loom is an adaptation of — model-checking concurrent data structures under the C/C++ memory model.

**DPOR (Dynamic Partial Order Reduction).** Search-space pruning technique for concurrency model-checkers. Avoids re-exploring equivalent interleavings. Loom uses DPOR; without it the interleaving space explodes combinatorially.

**Explicit-state model checking.** Enumerate states one-by-one, hashing each to detect revisits. [TLC](tla.md) is explicit-state. Fast on small bounded models, dies on large state spaces.

**Fairness.** An assumption that certain enabled actions are eventually taken. Required for *liveness* properties. Weak fairness (default in TLA+): an action enabled forever is eventually taken. Strong fairness: an action enabled infinitely often is eventually taken.

**Inductive invariant.** An invariant `I` such that `Init ⇒ I` and `I ∧ Next ⇒ I'`. If `I` is inductive, it holds in all reachable states — verified by checking one step, not the whole space. Apalache supports inductive-invariant checking; TLC does not directly.

**Liveness property.** "Eventually P" — something good will happen. Examples: every request eventually answered; system eventually converges. Vastly more expensive than safety to check; requires fairness.

**Loom (the technique, not the crate).** Permutation testing — running a test under all valid interleavings of synchronization operations under the C11 memory model. The crate [`loom`](loom.md) implements this for Rust.

**PCT (Probabilistic Concurrency Testing).** Burckhardt et al., ASPLOS 2010. Randomized scheduler with *lower-bound* probability guarantees of finding bugs at a given preemption depth. [Shuttle](loom.md#shuttle)'s primary algorithm.

**Refinement.** A relation between a high-level spec and a lower-level implementation where every implementation behavior is allowed by the spec. Proving refinement bridges the spec-implementation gap. Expensive — CompCert is the canonical example.

**Safety property.** "Bad thing doesn't happen" — invariant that holds in every reachable state. Cheaper to check than liveness; the bulk of TLA+ specs check safety only.

**State-space explosion.** The combinatorial blow-up in number of reachable states as concurrent actors and shared variables increase. The dominant cost of explicit-state model checking. Mitigations: smaller bounds, symmetry reduction, DPOR.

**Symbolic model checking.** Encode the verification problem as SMT, dispatch to a solver (Z3 typically). [Apalache](tla.md), [Kani](kani.md) (via CBMC) are symbolic. Handles unbounded data more gracefully than explicit-state; rejects specs/code that doesn't fit the supported subset.

## Languages and tools

**Apalache.** Symbolic model checker for [TLA+](tla.md), from Informal Systems. Apache-2.0. Current: v0.57.0 (2026-04-24). Uses Z3.

**CBMC.** See above.

**Coq.** Renamed [Rocq](comparisons.md#rocq-formerly-coq) effective 2025-03-12.

**Creusot.** Rust deductive verifier using prophecy variables to model mutable references. INRIA. See [`kani.md`](kani.md#adjacent-rust-verification-tools).

**Dafny.** Microsoft Research / Amazon verification-aware language. Compiles to C#/Java/Go/Python. See [`comparisons.md`](comparisons.md#dafny).

**F\*.** INRIA + Microsoft Research dependently-typed verification language. Source of HACL\* cryptography library. See [`comparisons.md`](comparisons.md#other-adjacent).

**Goto-C.** CBMC's intermediate representation. Kani lowers Rust → MIR → Goto-C → SMT.

**Iris.** Higher-order concurrent separation logic, primarily a Rocq library. Used by RustBelt to formalize Rust's type system.

**Kani.** Rust bounded model checker, AWS-origin, now under model-checking org. Apache-2.0 OR MIT. Current: 0.67.0 (2026-01-16). See [`kani.md`](kani.md).

**KEVM.** K Framework formal semantics of the EVM. See [`adoption.md`](adoption.md#ethereum-and-beyond).

**Lean 4.** Dependently-typed proof assistant and programming language. Apache-2.0. See [`comparisons.md`](comparisons.md#lean-4).

**Loom.** Tokio Rust concurrency permutation tester. MIT. Current: 0.7.2 (2024-04-23). See [`loom.md`](loom.md).

**Mathlib.** The largest formalized-mathematics library, written in Lean 4.

**P.** Microsoft Research formal modeling language, adopted at AWS for the S3 strong-consistency rollout. State-machine-oriented. Out of scope for Myrhiza adoption but worth knowing about. See [`adoption.md`](adoption.md#aws-the-canonical-adoption-story).

**PlusCal.** Algorithmic frontend for TLA+ that translates to TLA+. See [`tla.md`](tla.md).

**Promela.** SPIN's specification language. See [`comparisons.md`](comparisons.md#spin).

**Prusti.** Rust deductive verifier, ETH Zurich, Viper backend. See [`kani.md`](kani.md#adjacent-rust-verification-tools).

**Quint.** Engineer-friendly DSL that compiles to TLA+ and uses Apalache as primary backend. From Informal Systems. See [`tla.md`](tla.md#quint--engineer-friendly-dsl).

**Rocq.** Renamed [Coq](comparisons.md#rocq-formerly-coq), rename effective 2025-03-12 with Rocq 9.0.0. LGPL-2.1. Current: 9.2.0 (2026-03-27). The flagship dependently-typed proof assistant.

**Shuttle.** AWS Labs Rust concurrency tester via randomized scheduling + PCT. Apache-2.0. Current: 0.9.1 (2026-04-21). See [`loom.md#shuttle`](loom.md#shuttle).

**SPIN.** Bell Labs / Holzmann imperative model checker. Promela specification language. See [`comparisons.md`](comparisons.md#spin).

**TLA+.** Lamport's specification language. See [`tla.md`](tla.md). MIT (tools). TLA+ Foundation under Linux Foundation since 2023-04-21.

**TLAPS.** TLA+ Proof System. Theorem-prover-style proofs of TLA+ theorems, backed by Isabelle/HOL, Z3, others. Less used than TLC/Apalache.

**TLA+ Toolbox.** Lamport's Eclipse-based TLA+ IDE. Currently **unmaintained** per the repo README. Recommended replacement: VS Code extension `tlaplus/vscode-tlaplus`.

**TLC.** TLA+'s explicit-state model checker. Ships in `tlaplus/tlaplus`. Java-based. MIT.

**Verus.** Microsoft Research SMT-backed Rust verifier. See [`comparisons.md`](comparisons.md#other-adjacent).

**Z3.** Microsoft Research SMT solver, the dominant backend for nearly all the symbolic tools in this folder (Apalache, Kani, Dafny, Prusti, Creusot). MIT-licensed.

## Concepts

**Bit-precise reasoning.** Modeling integers at their actual machine bitwidths, catching overflow/underflow/truncation. [Kani](kani.md) is bit-precise; Prusti is not (models integers as mathematical).

**C11 memory model.** The C11 standard's specification of allowed reorderings and visibility of memory operations across threads. Rust inherits this. [Loom](loom.md) verifies against the C11 model.

**Counterexample.** A concrete execution that violates a property — the output a model checker produces when it finds a bug. Useful because it's actionable: "for this specific sequence of events, the system reaches a bad state."

**Deductive verification.** Contract-based verification: annotate functions with pre/post-conditions; the verifier proves the contracts via theorem-prover-style reasoning. Prusti, Creusot, Dafny are deductive. Compare with *model checking* (state-space exploration).

**Dependently-typed.** A type system where types can depend on values, enabling correctness properties to be expressed in the type system. Rocq, Lean, F\* are dependently-typed. Enables more proofs but at higher annotation cost.

**Model checker.** A tool that explores the state space of a system (concrete or symbolic) to verify properties. [TLC](tla.md), Apalache, Loom, Kani, SPIN are model checkers. Compare with *theorem provers* (Rocq, Lean) which do proof checking rather than space exploration.

**Sound.** A verification tool is *sound* if it never reports "no bug" when a bug exists in the modeled universe. [Loom](loom.md) is sound (within the C11 model + bounds); [Shuttle](loom.md#shuttle) is not (randomized; bugs can be missed).

**Theorem prover.** A tool for constructing machine-checked proofs of theorems. Rocq, Lean, F\*, Isabelle/HOL. Higher assurance than model-checking; vastly higher cost per proof.

**Unsound.** Not sound. A passing run does not prove correctness. Shuttle is unsound by design (the soundness-vs-scalability trade-off).

## Sources

- Files in this folder
- Lamport, *Specifying Systems* (2002), the canonical TLA+ reference
- Holzmann, *The SPIN Model Checker* (Addison-Wesley 2003)
- Burckhardt et al., "A Randomized Scheduler with Probabilistic Guarantees of Finding Bugs", ASPLOS 2010: https://dl.acm.org/doi/10.1145/1736020.1736040
- Norris & Demsky, "CDSChecker", OOPSLA 2013
- Newcombe et al., CACM 2015 (AWS): https://cacm.acm.org/research/how-amazon-web-services-uses-formal-methods/
