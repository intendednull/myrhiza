**Date:** 2026-05-29
**Status:** active
**Subject:** Blockchain WASM/VM metering (CosmWasm / NEAR / Soroban / PolkaVM-JAM) — how consensus VMs make instruction-count gas a hard, cross-validator-identical invariant, and the metering-bug incident corpus

# Blockchain WASM metering prior art

Reference folder for the four production (and one near-production) systems that
have shipped, debugged, *and version-migrated* native-WASM/VM instruction-count
gas as a hard consensus invariant: **CosmWasm** (Wasmer Singlepass), **NEAR**
(custom WASM runtime + parameter tables), **Soroban / Stellar** (Wasmi
interpreter), and **PolkaVM / JAM** (RISC-V PVM, Polkadot's successor to its
Wasmtime-based executor). **Substrate's Wasmtime runtime** is covered for
context as the *counter*-example — it runs WASM in consensus but does **not**
instruction-meter it.

These are consensus VMs. **Myrhiza is not a blockchain.** The borrow is the
metering *discipline* — determinism of cost across heterogeneous hardware, and
"recalibration is a protocol-version bump" — not the consensus layer. The borrow
boundary is spelled out in [framing.md](framing.md) and in every
"Borrow boundary" callout. Read those before importing any conclusion.

This is the **external-experience companion** to
[`../wasm-component-model/wasmtime.md`](../wasm-component-model/wasmtime.md)'s
fuel-vs-epoch section: that file documents what Wasmtime's `consume_fuel` gives
Myrhiza mechanically; this folder documents what four shipping systems learned
operating instruction-count gas in anger.

## Key facts at a glance

| System | Engine / VM | Metering mechanism | Cross-HW gas identity strategy | Recalibration = version bump? |
|---|---|---|---|---|
| **CosmWasm** | Wasmer **Singlepass** (compiler, not JIT-tiered) | Metering middleware injects fuel decrements at branch points; deterministic float middleware | Singlepass codegen is deterministic; gas counted in WASM-instruction space, not wall-clock | Yes — gas-table change is consensus-breaking; caused the Aug-2024 halt (see [incident-corpus.md](incident-corpus.md)) |
| **NEAR** | Custom WASM runtime (was Wasmer; finite-wasm-based) | Gas cost parameters per host/WASM op; `RuntimeConfigStore` keyed by protocol version | Params calibrated to "1 Tgas = 1 ms" on reference HW; QEMU `icount` as one of two metrics | **Yes, structurally** — params are a `BTreeMap<ProtocolVersion, RuntimeConfig>`; a change *is* a new protocol version |
| **Soroban** | **Wasmi** interpreter (~13 KLOC) | Linear cost model `y = a + bx` per cost-type, separate CPU + memory meters; offline-calibrated | Interpreter has no JIT optimization paths to diverge; counts instructions directly | Yes — cost params are network-config / ledger settings updated by consensus |
| **PolkaVM / JAM** | RISC-V **PVM** (recompiler x86-64/ARM64 + interpreter) | Fixed gas cost per instruction; "asynchronous gas metering" | RISC-V chosen partly *because* metering is cheaper/cleaner than WASM's | Pre-production; JAM Gray Paper pins instruction costs as protocol constants |
| **Substrate (context)** | **Wasmtime** (compiled, pooling) | **No instruction metering** — offline-benchmarked *weights* | N/A — weight is a HW-measured constant baked into the runtime | Weight re-benchmark = runtime upgrade (not VM-level metering at all) |

Version/date provenance is in each file's `## Sources` and consolidated in
[sources.md](sources.md). High-risk facts (versions, dates, bug IDs) were
verified against crates.io / GitHub / RFC-Editor — see the research notes in
[incident-corpus.md](incident-corpus.md) and [sources.md](sources.md).

## How to use

Canonical reading order:

1. **[framing.md](framing.md)** — the borrow boundary. Consensus VM vs. P2P
   convergence runtime: what transfers, what does not. Read first.
2. **[the-determinism-problem.md](the-determinism-problem.md)** — the shared
   problem all four solve: make "how much computation" byte-identical across
   x86-64 / aarch64 / riscv64. JIT-divergence, the two solution families
   (interpreter vs. deterministic-codegen vs. RISC-V).
3. **[cosmwasm.md](cosmwasm.md)** — Wasmer Singlepass + metering middleware; the
   richest incident corpus.
4. **[near.md](near.md)** — the most fully-documented
   *recalibration-as-protocol-version* mechanism (`RuntimeConfigStore`), the
   "1 Tgas = 1 ms" calibration rule.
5. **[soroban.md](soroban.md)** — the conservative-engineer case for an
   interpreter; CPU + memory dual meter; "why not a JIT."
6. **[polkavm-jam.md](polkavm-jam.md)** — the RISC-V escape hatch; "free
   metering" claim examined honestly; current pre-production status.
7. **[substrate-context.md](substrate-context.md)** — the counter-example:
   consensus WASM *without* instruction metering (benchmarked weights). Why
   that path is closed to Myrhiza.
8. **[recalibration-as-version-bump.md](recalibration-as-version-bump.md)** —
   the cross-system synthesis of the single most load-bearing pattern for
   Myrhiza: a cost-table change is a major-version event.
9. **[incident-corpus.md](incident-corpus.md)** — every metering bug we could
   verify, with IDs/versions/dates. "Metering is hard" is the thesis.
10. **[open-problems.md](open-problems.md)** — what instruction-count metering
    structurally does NOT solve.
11. **[lessons.md](lessons.md)** — *the decision file*: validates / avoid /
    borrow, tied to Myrhiza spec sections.
12. **[glossary.md](glossary.md)** — system-specific terms.

If you only have time for two files: **lessons.md** + **near.md**.

## Why this folder exists

Myrhiza's master spec defers a **fuel-cost-table child spec**
([`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md))
and pins a rule that a **Wasmtime-LTS bump is a kernel-major fuel
recalibration**
([`browser-native.md §14.2`](../../specs/2026-05-09-myrhiza-master-design/browser-native.md),
[`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md)). It also
flags a **DoS-asymmetry risk** — host calls (`host.hash`,
`host.verify-signature`) cost host CPU disproportionate to their WASM
instruction count
([`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md)). Those
three decisions have *exactly* one good source of operational evidence: the
consensus VMs that have already lived this. This folder is that evidence.

## Framing disclosure

These docs are written from **Myrhiza's current design stance** —
capability-mediated host access, P2P-only (no global consensus), the WASM
Component Model on Wasmtime, and `state-apply` as event-log replay over a
per-author Merkle DAG. The "Borrow boundary" callouts and every lesson read each
system **through that lens**. This is **not a neutral catalog** of blockchain VM
metering: facts are verified and reported honestly, but the *selection* and
*framing* of what matters are governed by Myrhiza's decision surfaces (the
deferred fuel-cost-table child spec, the DoS-asymmetry risk, the
Wasmtime-LTS-bump rule). A reader evaluating these systems on their own terms —
or for a different host design — should treat the framing as Myrhiza-specific and
re-derive relevance.

Concretely: these systems meter gas to **price** computation for a fee market
and to **bound** validator work under Byzantine load; Myrhiza meters fuel only
to **bound** `state-apply` so that fuel-exhaustion is a convergent outcome.
Myrhiza has no fee market and no Byzantine-validator quorum. Keep that asymmetry
in view — see [framing.md](framing.md). Do not import the fee-market machinery;
import the determinism discipline.

**Soft-pedal caution.** None of these systems is a Myrhiza hard dependency (the
hard deps are iroh, the WASM Component Model + Wasmtime, and jco). But this folder
documents failure modes Myrhiza genuinely *inherits* through its Wasmtime
dependency — chiefly the injected-metering-correctness risk (CWA-2024-007's missed
`if`) and the stale-precompiled-cache divergence (CWA-2024-004). Because the
corpus is read in service of validating Myrhiza's design, there is a standing
incentive to frame these inherited problems as "handled" or "mitigated" rather
than open. They are **not** closed: Wasmtime fuel-metering correctness is an
unaudited TCB dependency Myrhiza cannot eliminate, and the cache-key mitigation is
a *recommendation the spec has not yet adopted*. [open-problems.md](open-problems.md)
and [lessons.md](lessons.md) state these as live, not solved — read them as the
honest counterweight to any "validated" framing elsewhere in the folder.

## Sources

- CosmWasm "Metering is hard" — https://medium.com/cosmwasm/metering-is-hard-cosmwasm-security-issues-explained-a797511cd54e
- CosmWasm "The incomplete gas patch" — https://medium.com/cosmwasm/the-incomplete-gas-patch-and-why-it-caused-consensus-failures-173547ef02de
- NEAR gas architecture — https://near.github.io/nearcore/architecture/gas/index.html
- NEAR parameter definitions — https://near.github.io/nearcore/architecture/gas/parameter_definition.html
- Soroban "Why Doesn't Soroban Use a JIT?" — https://stellar.org/blog/developers/why-doesnt-soroban-use-a-jit
- Soroban fees, resource limits, metering — https://developers.stellar.org/docs/learn/fundamentals/fees-resource-limits-metering
- PolkaVM repo — https://github.com/paritytech/polkavm
- JAM PVM docs — https://jam-docs.onrender.com/basics/pvm
- crates.io: `cosmwasm-vm` (max stable 3.0.7), `wasmer` (4.4.0 published 2024-10-04)
