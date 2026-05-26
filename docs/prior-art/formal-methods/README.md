**Date:** 2026-05-22
**Status:** active
**Subject:** Formal-methods tooling survey — TLA+ / Apalache / PlusCal / Loom / Kani / Shuttle, with Coq-Rocq / Lean / SPIN / Dafny for context

# Formal methods (tooling survey)

This folder is a **tooling survey**, not a single-project deep dive. The subject is the lightweight formal-verification stack a Rust P2P runtime can realistically adopt: protocol-level specification with **TLA+** (plus its model-checkers TLC and **Apalache**, and its PlusCal frontend), Rust-level concurrency exploration with **Loom** and its alternative **Shuttle**, and Rust-level bounded model checking with **Kani**. Heavier proof-assistant tooling — **Rocq** (the renamed Coq) and **Lean 4** — and the older imperative model checker **SPIN**, plus **Dafny** as a contrast point, are covered for context, not as adoption targets.

The Myrhiza-shaped question is **"what is the cheapest formal-methods investment that pays back on a Rust P2P runtime?"**, not "how do we prove the runtime correct end-to-end." Be honest: nothing in this folder verifies an implementation against its spec automatically. The pay-off lives at two specific seams — *protocol design* (TLA+) and *concurrent-code bug-detection* (Loom / Kani) — and stops there.

## Why this folder exists

Two existing prior-art open-problems files explicitly nominate this stack:

- [`prior-art/holochain/open-problems.md §10`](../holochain/open-problems.md) — *"the core state machines (state-apply ordering, component link integrity, capability-token check) are bounded enough to specify in TLA+ from day one, and Loom is essentially free to adopt for any Rust runtime."*
- [`prior-art/spritely-ocapn/open-problems.md §8`](../spritely-ocapn/open-problems.md) — *"Spec the wire and the vat-state state machine in TLA+ before 1.0. Use Loom on the Rust runtime from day one."*

This folder turns those one-line recommendations into a concrete reading + adoption plan and surfaces the realistic limits.

## Key facts

| Tool | Layer | Lang / surface | License | Current version (2026-05-22) | Steward |
|---|---|---|---|---|---|
| [TLA+ Tools](https://github.com/tlaplus/tlaplus) (TLC, PlusCal translator, Toolbox) | Protocol spec | TLA+ / PlusCal | MIT | v1.8.0 "Clarke release" (2026-05-18) | [TLA+ Foundation](https://foundation.tlapl.us/) (Linux Foundation, est. 2023-04-21) |
| [Apalache](https://github.com/apalache-mc/apalache) | Symbolic TLA+ checker | TLA+ subset | Apache-2.0 | v0.57.0 (2026-04-24) | [Informal Systems](https://informal.systems/) |
| [Quint](https://github.com/informalsystems/quint) | Engineer-friendly TLA frontend | Quint | Apache-2.0 | active (Apalache backend) | Informal Systems |
| [Loom](https://github.com/tokio-rs/loom) | Rust concurrency permutation | Rust crate | MIT | 0.7.2 (2024-04-23, crates.io stable) | [Tokio](https://tokio.rs/) (Carl Lerche et al.) |
| [Shuttle](https://github.com/awslabs/shuttle) | Rust randomized concurrency | Rust crate | Apache-2.0 | 0.9.1 (2026-04-21, crates.io) / v0.9.0 tagged 2026-04-20 | AWS Labs |
| [Kani](https://github.com/model-checking/kani) | Rust bounded model checker | Rust + CBMC | Apache-2.0 OR MIT | 0.67.0 (2026-01-16) | AWS-origin; org `model-checking` |
| [Rocq](https://github.com/rocq-prover/rocq) (formerly Coq) | Proof assistant | Gallina / OCaml | LGPL-2.1 | 9.2.0 (2026-03-27); rename effective 9.0.0 (2025-03-12) | Inria + Rocq Consortium |
| [Lean 4](https://github.com/leanprover/lean4) | Proof assistant / lang | Lean | Apache-2.0 | 4.29.1 (2026-04-14 stable); 4.30.0-rc2 (2026-04-16) | Lean FRO (Microsoft Research + CMU origin) |
| [SPIN](https://spinroot.com/) | Promela model checker | Promela | (legacy, free for academic + non-commercial) | maintained; predates TLA+ | Gerard Holzmann / Bell Labs lineage |
| [Dafny](https://github.com/dafny-lang/dafny) | Verification-aware language | Dafny | MIT | active | Amazon-stewarded (post-MSR) |

The three load-bearing rows are TLA+/Apalache, Loom, and Kani. Everything else is **context**.

## Contents

- [`tla.md`](tla.md) — TLA+, TLC, PlusCal, Apalache, Quint, TLAPS. Protocol-level specification + model-checking. Includes the AWS adoption story.
- [`loom.md`](loom.md) — Tokio Loom + AWS Shuttle. Rust concurrency-bug detection via interleaving exploration.
- [`kani.md`](kani.md) — AWS-origin Rust bounded model checker. Verifies Rust functions over symbolic inputs via CBMC.
- [`adoption.md`](adoption.md) — Who uses what, with concrete deployments (AWS, MongoDB, CockroachDB, Tokio, Firecracker). Honest about pervasiveness — admired widely, used narrowly.
- [`comparisons.md`](comparisons.md) — Context tools: Rocq (renamed Coq, 2025), Lean 4, SPIN, Dafny, Prusti, Creusot. What they do, why they're context rather than adoption targets here.
- [`lessons.md`](lessons.md) — **The decision file.** Validates / avoid / borrow for Myrhiza.
- [`open-problems.md`](open-problems.md) — What formal methods structurally don't solve. The honest gap between "model verified" and "code correct."
- [`glossary.md`](glossary.md) — Terms (TLC vs Apalache, DPOR vs PCT, bounded vs unbounded, etc.)

## Reading order

1. [`lessons.md`](lessons.md) — start here if you're deciding whether to adopt any of this for a Myrhiza spec or component.
2. [`tla.md`](tla.md) — the protocol-design payoff. Read alongside the [AWS CACM 2015](https://cacm.acm.org/research/how-amazon-web-services-uses-formal-methods/) and [CACM 2025](https://cacm.acm.org/practice/systems-correctness-practices-at-amazon-web-services/) papers.
3. [`loom.md`](loom.md) — read before writing any non-trivial concurrent Rust in a state-apply or kernel-broker path.
4. [`kani.md`](kani.md) — read if you have a function-level invariant you actually want to prove (capability-token check, parser, codec).
5. [`adoption.md`](adoption.md) — read for honest priors on how much each tool costs in practice.
6. [`open-problems.md`](open-problems.md) — read after any of the above, to recalibrate expectations.
7. [`comparisons.md`](comparisons.md), [`glossary.md`](glossary.md) — reference only.

## How to use

**Framing disclosure.** These docs are written from a Myrhiza-stance (Rust P2P runtime, deterministic `state-apply`, capability-mediated host surface) — the *Implications for Myrhiza* sub-sections frame each tool's choices through that lens. A reader auditing whether to *adopt* the stack should weigh the corpus as a learn-from-X-into-Myrhiza-stance artifact, not a neutral catalog. In particular, this folder is biased toward the **lightweight subset** of formal methods (model-checking, not theorem proving) because that is what a small team can realistically apply; readers evaluating whether to invest in heavier verification (Rocq / Lean / TLAPS proofs) should treat the "context" framing in [`comparisons.md`](comparisons.md) as a possibly-too-quick dismissal.

The corpus also reflects the **dependency-side bias** noted in the `using-prior-art` skill: load-bearing tools we will commit to (TLA+, Loom, Kani) get the deepest treatment; reference-only tools get a paragraph. If Myrhiza later decides to invest in proof-assistant-grade verification, this folder will need a second sweep.

## Cross-references

- [`prior-art/holochain/open-problems.md §10`](../holochain/open-problems.md) — Holochain has zero formal verification; "gap to not repeat" framing.
- [`prior-art/spritely-ocapn/open-problems.md §8`](../spritely-ocapn/open-problems.md) — recommends TLA+ + Loom before 1.0.
- [`prior-art/iroh/`](../iroh/) — load-bearing transport dependency, no formal spec of its own protocols (separate gap).
- [`prior-art/willow/`](../willow/) — internal-ancestor runtime; lessons here apply directly to its state-apply paths.

## Sources

- TLA+ Tools releases: https://github.com/tlaplus/tlaplus/releases
- Apalache releases: https://github.com/apalache-mc/apalache/releases
- Kani releases + crates.io: https://github.com/model-checking/kani/releases ; https://crates.io/crates/kani-verifier
- Loom crates.io: https://crates.io/crates/loom
- Shuttle crates.io + GitHub: https://crates.io/crates/shuttle ; https://github.com/awslabs/shuttle/releases
- Rocq Prover 9.0/9.2 releases: https://rocq-prover.org/releases/9.0.0 ; https://github.com/rocq-prover/rocq
- Lean 4 releases: https://github.com/leanprover/lean4/releases
- TLA+ Foundation launch: https://www.linuxfoundation.org/press/linux-foundation-launches-tlafoundation (2023-04-21)
- Lamport's TLA+ home: https://lamport.azurewebsites.net/tla/tla.html
- Learn TLA+: https://learntla.com/
- AWS CACM 2015: https://cacm.acm.org/research/how-amazon-web-services-uses-formal-methods/
- AWS CACM 2025: https://cacm.acm.org/practice/systems-correctness-practices-at-amazon-web-services/
