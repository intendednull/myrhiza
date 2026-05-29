**Date:** 2026-05-29
**Status:** active
**Subject:** What instruction-count metering structurally does NOT solve

# Open problems

Instruction-count metering is a deterministic *bound on guest computation*. It is
also narrow. This file records what it does **not** solve —
both for the consensus VMs and, by inheritance, for Myrhiza.

## 1. It bounds WASM-instruction count, not host CPU time

Counting WASM instructions says nothing about how much *host* CPU a host call
burns. `host.verify-signature` is one host call (cheap in instruction terms) that
does an Ed25519 verify (expensive in CPU terms). An attacker who can call
expensive host functions cheaply drains host CPU disproportionate to the fuel
spent — the **DoS-asymmetry** Myrhiza names in
[`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md). The fix
(charge host calls a CPU-proportional cost, Soroban-style — [soroban.md](soroban.md))
is a *patch on top of* instruction metering, not something instruction metering
gives you. Every system here meters host calls separately for exactly this reason.

## 2. It does not make the cost table *correct*, only *consistent*

Metering guarantees every node computes the *same* gas number. It does **not**
guarantee that number reflects real cost. A miscalibrated table (too cheap) is a
DoS hole even when perfectly deterministic — CWA-2024-004 ([cosmwasm.md](cosmwasm.md))
was a *consistency* fix to a *correctness* problem, and the fix itself caused a
halt. Calibration is a separate, ongoing, error-prone activity (NEAR's estimator,
Soroban's offline fitting). Metering is necessary but not sufficient.

## 3. It does not survive a JIT/compiler version change for free

A cost table calibrated against Wasmtime vN may be wrong for vN+1 — Cranelift's
codegen and fuel-injection can shift between majors
([`../wasm-component-model/wasmtime.md`](../wasm-component-model/wasmtime.md)
open-problems; [near.md](near.md) keys around this). So metering does not give
you a *stable* notion of cost across your own toolchain upgrades; it forces a
recalibration discipline ([recalibration-as-version-bump.md](recalibration-as-version-bump.md)).
For Myrhiza specifically: pre-check ↔ apply equivalence depends on both running
the same Wasmtime version's fuel behavior.

## 4. The injection points are a correctness surface that can be wrong

CWA-2024-007's missed `if` proves the metering *implementation* itself is a bug
surface. WASM's structured control flow makes correct injection non-trivial; JAM
left WASM partly to escape this ([polkavm-jam.md](polkavm-jam.md)). Myrhiza
inherits Wasmtime's injection correctness as a TCB dependency it cannot audit
away — only track.

## 5. No accounting on bypassed exit paths unless you build it

CWA-2024-008: a panic skipped the gas report. Metering only counts what flows
through the accounting code; any error path that bypasses it leaks free
computation. This is an implementation invariant ("charge on every exit"), not
something metering enforces by construction.

## 6. It says nothing about memory, I/O, or storage cost

Instruction count ignores RAM, disk, and bandwidth. Soroban meters memory as a
*separate* budget ([soroban.md](soroban.md)); NEAR has distinct storage-cost
parameters; Myrhiza has a separate 64 MB memory cap and a 1 MB payload limit
([`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md)).
A single CPU-fuel meter is structurally incomplete for any non-CPU resource.

## 7. No global-epoch coordination for the cutover (Myrhiza-specific)

The consensus VMs get a hard, coordinated cost-table cutover for free from their
consensus layer (a protocol-version vote per epoch). Myrhiza's per-author DAG has
**no global epoch**, so a recalibration propagates peer-by-peer with a genuine
convergence-divergence window during rollout
([recalibration-as-version-bump.md](recalibration-as-version-bump.md),
[framing.md](framing.md)). Instruction metering does not solve *when* everyone
switches tables; in a blockchain, consensus does; in Myrhiza, nothing does — it
is a tolerated window.

## 8. Pre-check / dry-run shares the budget — a soft DoS remains

Myrhiza runs pre-check as the same WASM function as `state-apply` in dry-run,
sharing the per-event fuel budget
([`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md)).
A malicious event with deep validation logic makes downstream peers pay that
validation cost too. The spec flags this as an open question (separate pre-check
fuel). The consensus VMs do not have a direct analogue (they have one execution,
not a propose/re-check split), so the corpus offers limited guidance here — it is
a Myrhiza-original surface.

## Sources

- CosmWasm incidents — https://medium.com/cosmwasm/metering-is-hard-cosmwasm-security-issues-explained-a797511cd54e and https://medium.com/cosmwasm/the-incomplete-gas-patch-and-why-it-caused-consensus-failures-173547ef02de
- Soroban metering — https://developers.stellar.org/docs/learn/fundamentals/fees-resource-limits-metering
- NEAR estimator — https://near.github.io/nearcore/architecture/gas/estimator.html
- Myrhiza spec: determinism.md §5.2/§5.3, risks.md §19
