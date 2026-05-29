**Date:** 2026-05-29
**Status:** active
**Subject:** System-specific terms for blockchain WASM/VM metering

# Glossary

System-specific terms used across this folder. For Myrhiza's own vocabulary
(fuel, `state-apply`, `HeadsSummary`, profiles) see the master spec; for WASM
Component Model terms see [`../wasm-component-model/glossary.md`](../wasm-component-model/glossary.md).

- **Gas** — the abstract, machine-independent unit of computation a consensus VM
  charges per operation. The blockchain analogue of Myrhiza's *fuel*. Counted in
  source-ISA (WASM/RISC-V) instruction space, never native cycles.

- **Cost table / gas table / fuel-cost-table** — the mapping from operations to
  gas/fuel cost. Changing it is a protocol-version event in every system here.

- **Metering middleware (Wasmer)** — a module-rewriting pass that injects
  gas-decrement instructions into a WASM module before compilation. The locus of
  CosmWasm's CWA-2024-007 bug.

- **Singlepass (Wasmer)** — a single-pass, non-optimizing WASM compiler. Fast to
  compile, predictable codegen; CosmWasm's **on-chain** backend.

- **Cranelift** — the optimizing code generator used by both Wasmtime (Myrhiza)
  and as Wasmer's development-default backend. A JIT-style compiler.

- **JIT bomb** — a malicious input crafted to make a JIT compiler consume
  excessive time/memory during compilation. One of Soroban's reasons to use an
  interpreter.

- **Wasmi** — a pure WASM interpreter (~13 KLOC) used by Soroban; metering is a
  simple per-instruction counter increment.

- **1 Tgas = 1 ms** — NEAR's calibration rule: 1 teragas of execution must take
  at most 1 millisecond on reference validator hardware. Defines the physical
  meaning of a gas unit.

- **ICount / `icount`** — instruction counting via QEMU emulation; one of NEAR's
  two calibration metrics, reproducible across machines.

- **`RuntimeConfig` / `RuntimeConfigStore` (NEAR)** — the per-protocol-version gas
  parameter set, and the `BTreeMap<ProtocolVersion, RuntimeConfig>` that maps each
  protocol version to its complete config. The model for "recalibration is a
  version bump."

- **Protocol version (NEAR)** — the agreed blockchain-protocol version for an
  epoch, advanced by validator vote; the key that selects which `RuntimeConfig`
  applies. Myrhiza has **no global analogue** (per-author DAG, no epoch).

- **`send_sir` / `send_not_sir` / `execution` (NEAR)** — the three cost components
  of an action: send-same-account, send-cross-account, and execution. Fee-market
  machinery; *not* borrowed by Myrhiza.

- **Cost type / meta-instruction (Soroban)** — a host operation of known
  complexity, costed via a linear model `y = a + bx` separately for CPU and memory.

- **Budget (Soroban)** — the per-resource allowance (CPU instructions, memory
  bytes) for a contract invocation; exceeding it terminates execution.

- **PVM (Polkadot Virtual Machine)** — the RISC-V-based VM JAM uses, built on
  PolkaVM; replaces Substrate's Wasmtime-based executor. Assigns a fixed gas cost
  per RISC-V instruction.

- **PVM2** — an in-progress redesign (announced 2026-05-28) moving PVM's encoding
  toward standard RISC-V (RV64E + standard extensions). The PVM ISA is not frozen.

- **JAM (Join-Accumulate Machine)** — Polkadot's planned successor protocol
  architecture; on testnet as of early 2026, not mainnet.

- **"Free metering"** — informal claim that RISC-V per-instruction gas has
  near-zero overhead vs. WASM's per-block fuel check. Marketing-adjacent; not a
  measured figure in the primary docs reviewed (see [polkavm-jam.md](polkavm-jam.md)).

- **PVF (Parachain Validation Function)** — parachain logic re-executed by
  Polkadot relay validators; the Wasmtime-based path JAM replaces with RISC-V.

- **Weight (Substrate)** — an offline-benchmarked constant cost for an extrinsic,
  baked into the (trusted) runtime. Bounds work *without* runtime instruction
  metering. The trusted-code alternative to metering; closed to Myrhiza.

- **Consensus failure / chain halt / fork** — what happens when nodes disagree on
  computed gas: the blockchain analogue of a Myrhiza convergence divergence.

## Sources

- Per-file `## Sources` sections in this folder.
- NEAR gas docs — https://near.github.io/nearcore/architecture/gas/index.html
- Soroban metering — https://developers.stellar.org/docs/learn/fundamentals/fees-resource-limits-metering
- JAM PVM — https://jam-docs.onrender.com/basics/pvm
