**Date:** 2026-05-22
**Status:** active
**Subject:** Context tools — Rocq (formerly Coq), Lean 4, SPIN, Dafny, Prusti, Creusot

# Comparisons / context tools

These tools are not the recommended Myrhiza adoption path but appear adjacent to anything calling itself "formal methods." This file gives each enough scope to recognize *what* it is and *why* the [Myrhiza-stance lessons](lessons.md) treat it as context rather than a load-bearing tool. Heavier proof-assistant work (Rocq, Lean) and the older specification-language family (SPIN, Dafny) belong here.

## Rocq (formerly Coq)

[Rocq](https://github.com/rocq-prover/rocq) — formerly **Coq** — is an interactive theorem prover and the most-deployed dependently-typed proof assistant. The Coq → Rocq rename was announced **2023-10-11** and became effective with the **Rocq 9.0.0 release on 2025-03-12**; the current version is **9.2.0** (2026-03-27). The rationale, per the team's announcement, was the "Coq" name's unfortunate connotation in English which had become an actual recruiting obstacle. The rename was the major project-governance event of the last decade for the tool — backward-compatible (the OPAM repo still ships `coq-*` packages alongside `rocq-*` ones), but identity-changing.

License: **LGPL-2.1**. Original development at INRIA (France), now under the Rocq Consortium. Implemented in **OCaml**. The standard library is at [rocq-prover/stdlib](https://github.com/rocq-prover/stdlib).

What Rocq does: machine-checked proofs of theorems in a higher-order dependently-typed logic (the Calculus of Inductive Constructions). The two flagship outcomes are **CompCert** (verified C compiler — Xavier Leroy and colleagues, INRIA) and **certified verification** of cryptographic primitives (the [Fiat Cryptography](https://github.com/mit-plv/fiat-crypto) project, which has shipped verified ECC implementations into Chrome and BoringSSL).

Why it's context, not adoption-path, for Myrhiza: writing Rocq proofs is **expensive**. CompCert took ~10 person-years. The cost-per-line-of-proof is two-to-three orders of magnitude higher than the cost-per-line-of-Rust. The bug-find leverage is unmatched (when a Rocq proof says "no", the property *holds*, not "I couldn't find a counterexample"), but the adoption cost is far outside a small-team P2P runtime's budget. Rocq is the right tool for a verified compiler or a verified cryptographic kernel; it is the wrong tool for "our state machine should converge."

**Note for future readers:** if Myrhiza ever ships a verified-cryptography or verified-compiler subcomponent, Rocq (and its sister Lean 4) come back into scope.

## Lean 4

[Lean 4](https://github.com/leanprover/lean4) is the second mainstream dependently-typed proof assistant. Originally Microsoft Research (Leonardo de Moura, the same author behind Z3), now stewarded by the **Lean Focused Research Organization (Lean FRO)** with continuing involvement from Carnegie Mellon and Microsoft. Current stable: **4.29.1** (2026-04-14); **4.30.0-rc2** is in pre-release as of 2026-04-16. License: **Apache-2.0**. Implemented in Lean itself (with a C/C++ runtime).

The flagship artifact is **[mathlib](https://github.com/leanprover-community/mathlib4)**, the largest formalized mathematics library in the world — over a million lines, ~85 contributors. Mathlib follows monthly Lean releases. The recent expansion of Lean 4 has been driven heavily by the mathematics community (the Fields Medalist Terence Tao publicly mentors Lean projects), but software-verification use cases are growing — verified compilers, verified cryptography, and a verified subset of the Lean compiler itself.

For systems verification specifically, Lean 4's tooling is younger than Rocq's but improving fast — the Lean FRO is well-funded by industry sponsors (AWS is a sponsor; the [Leansec](https://leanprover.zulipchat.com/) community discusses real production verification work).

Why it's context, not adoption-path: same shape as Rocq — the cost-per-proof is far outside a small-team runtime's budget. Mention Lean for completeness; do not invest in it for Myrhiza unless the scope expands dramatically.

## SPIN

[SPIN](https://spinroot.com/) is the older, imperative model checker. Originated at Bell Labs in 1980 under **Gerard Holzmann**; verified systems are described in **Promela** (Process Meta Language). Free for academic + non-commercial use; commercial licenses available. Continues to receive maintenance and is used in safety-critical academic and aerospace settings.

SPIN predates TLA+ by ~15 years. TLA+ partially supplanted SPIN in industry adoption because (a) TLA+'s declarative spec style is closer to mathematical reasoning than Promela's imperative one, and (b) Lamport's 2002 *Specifying Systems* book and AWS's 2015 paper drove TLA+'s industry presence in a way SPIN never enjoyed.

For Myrhiza, SPIN is context only. If Myrhiza had a hard-real-time or aerospace-grade safety requirement, SPIN might re-enter consideration — neither applies. Note for the curious: the [Paxos in SPIN](https://arxiv.org/abs/1408.5962) paper is a useful side-by-side comparison with the equivalent TLA+ specs.

## Dafny

[Dafny](https://github.com/dafny-lang/dafny) is Microsoft Research's verification-aware language: a Java/C#-shaped surface with built-in support for pre/post-conditions, loop invariants, and SMT-backed verification (via Z3). Dafny is **Amazon-stewarded since ~2022** following Rustan Leino's move to AWS; license is MIT.

Dafny sits between Kani (bounded model checking of an existing language) and Rocq/Lean (heavy proof assistants for new code): you write *new* Dafny code, the compiler verifies the assertions at compile time. The output compiles to C#, Java, Go, or Python.

The flagship deployment is **Dafny-EVM** ([`Consensys/evm-dafny`](https://github.com/Consensys/evm-dafny), Cassez et al., *Formal Methods 2023*) — a verified and executable Ethereum Virtual Machine specification, used as a reference implementation for cross-checking against go-ethereum and other EVMs.

Why it's context for Myrhiza: Dafny requires writing *new* code in Dafny. Myrhiza's surface is Rust + WASM components; rewriting any Myrhiza component in Dafny would be a paradigm shift, not a tool adoption. Mention Dafny when discussing AWS's broader formal-methods toolkit; do not adopt it.

## Prusti and Creusot

Both are deductive verifiers for Rust, sharing a layer with Kani but a different style:

- **[Prusti](https://github.com/viperproject/prusti-dev)** — ETH Zurich, uses Viper as the backend separation-logic verifier. Annotation-style: `#[requires(...)]` / `#[ensures(...)]`. Stronger on safe Rust + functional correctness, weaker on `unsafe`. Integer arithmetic is modeled as mathematical, not bit-precise — a deliberate choice trading some bug-find leverage for proof tractability.
- **[Creusot](https://github.com/creusot-rs/creusot)** — INRIA, uses Why3 as the backend. Models Rust's mutable references via prophecy variables (a technique that exploits Rust's ownership invariants to avoid heap reasoning). Supports more borrowing patterns than Prusti, including reborrowing. Pure-Rust annotations.

Both are **deductive** (contract-based): the verifier proves the function meets its contract via theorem-prover-style reasoning. [Kani](kani.md) is **bounded model checking**: it tries inputs symbolically up to some depth bound.

The honest framing for Myrhiza: deductive verification has higher cost per function but verifies unbounded properties. Kani has lower cost but only verifies up to a depth bound. For the specific case of "this parser never panics on any input," Kani is usually adequate and cheaper. For "this distributed protocol always commits in order under arbitrary failures," neither tool is the right shape — [TLA+](tla.md) is.

Prusti and Creusot are worth knowing about. They are not the recommended Myrhiza first investment.

## Other adjacent

- **[Verus](https://github.com/verus-lang/verus)** — Microsoft Research, SMT-backed Rust verifier with first-class support for `unsafe` and concurrency. Newer than Prusti/Creusot, actively developed.
- **[F\*](https://www.fstar-lang.org/)** — INRIA + Microsoft Research dependently-typed verification language. Source of HACL\*, the cryptography library used in Mozilla NSS and Linux kernel. Extracts to OCaml, F#, C, WASM. Out of scope for Myrhiza but worth knowing about for the verified-cryptography angle.
- **[Iris](https://iris-project.org/)** — Higher-order concurrent separation logic, primarily a Rocq library. Academic-grade tool used for ground-up concurrent-program verification (including [RustBelt](https://plv.mpi-sws.org/rustbelt/), the formal model of Rust's type system).

## When this folder is the wrong reading

The folder is shaped for a small-team Rust P2P runtime. If your scenario is:

- **Verified cryptography from scratch** — go read about F\*, libcrux, HACL\*, and the Cryspen/Inria work. Out of scope here.
- **Verified compilers / safety-critical avionics** — Rocq + CompCert + SPIN are the canonical stack. Out of scope here.
- **Formalized mathematics** — Lean + mathlib. Out of scope here.

This folder treats those as "things we have heard of" rather than "things we should adopt." If Myrhiza's scope changes to overlap one of them, the folder needs a second sweep — see the framing-disclosure note in [`README.md`](README.md).

## Sources

- Rocq Prover: https://github.com/rocq-prover/rocq — LGPL-2.1, 9.2.0 (2026-03-27)
- Rocq Prover 9.0 release notes: https://rocq-prover.org/releases/9.0.0
- Coq→Rocq rename announcement (HN 2023): https://news.ycombinator.com/item?id=38779480
- Coq→Rocq HN follow-up (2024): https://news.ycombinator.com/item?id=41180007
- CompCert: https://compcert.org/
- Fiat Cryptography: https://github.com/mit-plv/fiat-crypto
- Lean 4: https://github.com/leanprover/lean4 — Apache-2.0, 4.29.1 (2026-04-14)
- Mathlib 4: https://github.com/leanprover-community/mathlib4
- Lean FRO: https://lean-fro.org/
- SPIN model checker: https://spinroot.com/
- SPIN Wikipedia: https://en.wikipedia.org/wiki/SPIN_model_checker
- Paxos in SPIN paper: https://arxiv.org/abs/1408.5962
- Dafny: https://github.com/dafny-lang/dafny
- Dafny-EVM: https://github.com/Consensys/evm-dafny ; paper https://arxiv.org/abs/2303.00152
- Prusti: https://github.com/viperproject/prusti-dev
- Creusot: https://github.com/creusot-rs/creusot
- Verus: https://github.com/verus-lang/verus
- F*: https://www.fstar-lang.org/
- Iris / RustBelt: https://iris-project.org/ ; https://plv.mpi-sws.org/rustbelt/
- Rust Formal Methods Interest Group: https://rust-formal-methods.github.io/
