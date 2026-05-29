**Date:** 2026-05-29
**Status:** active
**Subject:** The shared problem — making "how much computation" byte-identical across x86-64 / aarch64 / riscv64

# The determinism problem all four systems solve

Every system here faces the same root requirement: two nodes running different
hardware and (often) different compiler vintages must agree, **bit-for-bit**, on
how much gas a given execution consumed. If node A says a transaction used
1,000,000 gas and node B says 1,000,001, the block is invalid on one and valid
on the other — the chain **forks**. This is the same failure Myrhiza calls a
convergence divergence: peer A applies an event, peer B traps on fuel
exhaustion, and their state digests no longer match.

The hard part is that the *natural* unit of "how much computation" — wall-clock
time, CPU cycles, retired instructions — is **not** portable. It depends on the
microarchitecture, the JIT's optimization decisions, the host load. So every
system meters in an **abstract, machine-independent unit**: counts of *WASM* (or
RISC-V) instructions, weighted by a fixed cost table, never counts of *native*
instructions or nanoseconds.

## Why native execution is the enemy

A JIT (Wasmtime's Cranelift, Wasmer's Cranelift/LLVM backends) compiles one WASM
instruction into a *variable* number of native instructions, and the count
**differs across targets and compiler versions**:

- The same `i32.add` may fuse with a neighbor on x86-64 and not on aarch64.
- Constant folding can elide an instruction on one opt-level and not another —
  Myrhiza's own spec pins Cranelift opt-level to `Speed` for exactly this reason
  ([`determinism.md §5.2`](../../specs/2026-05-09-myrhiza-master-design/determinism.md)).
- A new compiler version may change register allocation, shifting where a trap
  lands.

So metering **must not** count native work. It counts work in the *source* ISA's
units. This is the invariant that ties the whole folder together, and it is the
single most important transferable fact for Myrhiza: **fuel is counted in
WASM-instruction space, which is portable; it must never leak native-execution
cost into the count.** Wasmtime's `consume_fuel` already does this — Cranelift
inserts the fuel-decrement in WASM-instruction terms before codegen
([`../wasm-component-model/wasmtime.md`](../wasm-component-model/wasmtime.md)).

## The three solution families

### 1. Interpreter (Soroban / Wasmi)

Run a pure WASM interpreter and increment a counter per instruction dispatched.
No JIT means no optimization paths to diverge. Soroban's argument, verbatim:
"Given the complex nature of JITs and unexpected optimization paths it's hard to
ensure stability or portability of JIT-based metering." The interpreter's
per-instruction dispatch overhead is *already paid*, so adding a counter is
nearly free. Cost: slow execution (mitigated by pushing inner loops into native
host functions). See [soroban.md](soroban.md).

### 2. Deterministic compiler + injected metering (CosmWasm / NEAR / Substrate-PVF)

Use a compiler whose codegen is itself deterministic across targets at the
*WASM-semantics* level, and **inject** the gas-decrement instructions into the
WASM *before* compiling. CosmWasm uses Wasmer **Singlepass** (a single-pass,
non-optimizing compiler — predictable, fast to compile, deterministic) plus a
**metering middleware** that rewrites the module to decrement gas at branch
points. The metering is in WASM space; the native code differs per host but the
gas count does not. This is the family Myrhiza is in (Wasmtime + `consume_fuel`).
See [cosmwasm.md](cosmwasm.md). **The injection points are where the bugs live**
(see [incident-corpus.md](incident-corpus.md)).

### 3. Change the ISA (PolkaVM / JAM)

Abandon WASM for **RISC-V**, on the thesis that RISC-V's flat, regular
instruction set makes per-instruction fixed-cost gas cleaner than WASM's
structured control flow (blocks, `if`, `loop`, `br_table`) — the exact structure
that made CosmWasm's metering injection miss the `if` instruction
([incident-corpus.md](incident-corpus.md) CWA-2024-007). RISC-V also transpiles
cheaply to x86-64/ARM64. See [polkavm-jam.md](polkavm-jam.md). **Borrow boundary:**
Myrhiza's ISA choice (WASM Component Model) is fixed by hard dependency; the
RISC-V option is interesting as evidence that *WASM's control-flow structure is a
metering liability*, not as a path Myrhiza can take.

## The counter-example: don't meter at all (Substrate runtime)

Substrate runs its *runtime* under **Wasmtime (compiled)** and does **not**
instruction-meter it for consensus. Instead it bounds work with **weights** —
offline-benchmarked constants ("this extrinsic costs N picoseconds on reference
HW") that are part of the runtime code. The runtime author is *trusted* (it is
on-chain governance-upgraded WASM, not arbitrary user code), so a non-metered JIT
is acceptable. See [substrate-context.md](substrate-context.md). **This path is
closed to Myrhiza**: `state-apply` is arbitrary app-author code, not trusted
governance code, so it must be metered, not benchmarked.

## Where each lands for Myrhiza

Myrhiza is family #2 (Wasmtime + injected fuel) by hard dependency. The lessons
that matter most therefore come from CosmWasm (same family, richest bug corpus)
and NEAR (the recalibration mechanism). Soroban's interpreter is the
conservative runner-up Myrhiza explicitly does not take (Wasmtime's Pulley
interpreter exists but is not the `state-apply` default — see
[`determinism.md §5.2`](../../specs/2026-05-09-myrhiza-master-design/determinism.md)
codegen-strategy pin). See [lessons.md](lessons.md).

## Sources

- Soroban "Why Doesn't Soroban Use a JIT?" — https://stellar.org/blog/developers/why-doesnt-soroban-use-a-jit
- CosmWasm "Metering is hard" — https://medium.com/cosmwasm/metering-is-hard-cosmwasm-security-issues-explained-a797511cd54e
- JAM PVM docs — https://jam-docs.onrender.com/basics/pvm
- Wasmtime deterministic execution — https://docs.wasmtime.dev/examples-deterministic-wasm-execution.html
- Companion: [`../wasm-component-model/wasmtime.md`](../wasm-component-model/wasmtime.md)
