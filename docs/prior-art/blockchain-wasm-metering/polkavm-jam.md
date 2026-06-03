**Date:** 2026-05-29
**Status:** active
**Subject:** PolkaVM / JAM — the RISC-V escape from WASM-control-flow metering; "free metering" examined; current pre-production status

# PolkaVM / JAM

PolkaVM is Parity's RISC-V-based virtual machine; the JAM ("Join-Accumulate
Machine") protocol — Polkadot's planned successor architecture — adopts it as the
**PVM** (Polkadot Virtual Machine), **replacing the Wasmtime-based executor**
Substrate uses today ([substrate-context.md](substrate-context.md)). It is the
one system here that answered "WASM metering is hard" by **leaving WASM**.

## Status (verify before relying — it is pre-production)

- **PolkaVM is explicitly pre-release.** Its README states, verbatim: "This
  project is still unfinished and is a very heavy work-in-progress! Do not use it
  in production!" Treat any performance or metering claim as a design goal, not a
  shipped guarantee.
- **Attribution.** Developed by **Parity Technologies**; primary author **Jan
  Bujak** (GitHub handle `koute`). This is *not* on the README — it is sourced
  from the `polkavm-common` crate's `authors` field (`Jan Bujak <jan@parity.io>`,
  `Parity Technologies <admin@parity.io>`) and Bujak's Parity-affiliated GitHub
  profile at `github.com/koute` (which pins `paritytech/polkavm`).
- **JAM is on testnet, not mainnet.** The JAM testnet launched **January 2026**
  and supports multiple execution environments including RISC-V. Multiple clients
  reportedly reached conformance milestones through 2025; **mainnet was targeted
  for early 2026** but JAM was not, as of this writing (2026-05-29), a shipped
  mainnet. Do not cite JAM as a *production* metering reference; cite it as the
  best-documented *design* argument for RISC-V metering.
- **PVM2** (announced on the Polkadot Forum **2026-05-28**, by user "sorpaas") is
  an in-progress redesign moving PVM's instruction encoding toward **standard
  RISC-V** ("a short differential from RV64E" plus unmodified standard extensions
  `m, c, zbb, zba, zbs, zicond, zicclsm`). This is *active churn* — the ISA is
  not frozen. Flag for any future reader: the PVM instruction set is moving.

## The metering thesis: RISC-V is cleaner to meter than WASM

The JAM/PVM argument for RISC-V over WASM rests substantially on **metering**:

- **"PVM defines a fixed cost for each instruction (aka _gas_)."** Each program
  gets "an upper limit of gas and can run at most until that limit is reached."
  Because RISC-V is a flat, regular instruction stream (no nested structured
  control flow like WASM's `block`/`loop`/`if`/`br_table`), assigning a fixed cost
  per instruction is straightforward.
- PolkaVM advertises **"high performance asynchronous gas metering"** that aims to
  be "cheap, deterministic, and reasonably accurate" — though, per its own README,
  this "remains a design goal rather than a completed feature."
- The often-repeated **"free metering"** framing (RISC-V metering has near-zero
  overhead vs. WASM's per-block fuel check) is a *claim*, attributed to the
  RISC-V design rationale. **Flag:** the primary JAM PVM doc reviewed states
  fixed-per-instruction gas and asynchronous metering but **did not contain the
  literal phrase "free metering" nor a quantified overhead comparison** — that
  framing comes from secondary summaries (forum/blog). Treat "free metering" as
  marketing-adjacent shorthand, not a measured number.

### Why this is the most pointed lesson for Myrhiza

CosmWasm's headline metering bug (CWA-2024-007, [cosmwasm.md](cosmwasm.md)) was a
**missed branch instruction (`if`)** — a bug that exists *because* WASM has rich
structured control flow that metering injection must enumerate correctly. JAM's
entire argument is: **WASM's control-flow structure is a metering liability.**
Myrhiza cannot leave WASM (it is a hard dependency — WASM Component Model +
Wasmtime), so Myrhiza **inherits exactly the metering-injection-correctness risk
that PVM was designed to escape.** The mitigation: Myrhiza does not hand-roll the
injection — it relies on Wasmtime's upstream `consume_fuel`, which is the
analogue of trusting Wasmer's (post-fix) middleware. The lesson is to **track
Wasmtime fuel-metering correctness as a TCB concern**, because a missed-`if`-class
bug in Wasmtime would hit Myrhiza the same way it hit CosmWasm. See
[lessons.md](lessons.md) and [open-problems.md](open-problems.md).

## Determinism & consensus framing (borrow boundary)

PVM "is designed to be fully deterministic" — same input/code → same output — and
JAM uses this for consensus: "all validators that interpret a service must arrive
at the same outcome," with **slashing** for validators that diverge. That is the
consensus layer Myrhiza does **not** borrow ([framing.md](framing.md)): Myrhiza
has no slashing and no validator quorum. What transfers is only the determinism
requirement itself, which Myrhiza already states for `state-apply`.

## RISC-V → native transpilation (the portability angle)

A secondary RISC-V selling point relevant to cross-HW gas identity: RISC-V
"easily transpiles to x86/x64/ARM," and PolkaVM ships a **native recompiler for
x86-64 and ARM64** plus an **interpreted fallback**. The gas count is in RISC-V
instruction space (portable) regardless of which backend runs — the same
"meter in source-ISA space, not native space" invariant as every other system
here ([the-determinism-problem.md](the-determinism-problem.md)).

## Sources

- PolkaVM repo (status, async gas metering, recompiler/interpreter backends) — https://github.com/paritytech/polkavm
- PolkaVM authorship (`polkavm-common` Cargo.toml `authors`; `github.com/koute`) — https://docs.rs/crate/polkavm-common/latest/source/Cargo.toml
- JAM PVM docs (fixed per-instruction gas, determinism) — https://jam-docs.onrender.com/basics/pvm
- "Announcing PolkaVM" (Polkadot Forum) — https://forum.polkadot.network/t/announcing-polkavm-a-new-risc-v-based-vm-for-smart-contracts-and-possibly-more/3811
- "Announcing PVM2: Reimagining PVM towards standard RISC-V" (2026-05-28) — https://forum.polkadot.network/t/announcing-pvm2-reimagining-pvm-towards-standard-risc-v/17748
- ink! "Why RISC-V and PolkaVM for Smart Contracts?" — https://use.ink/docs/v6/background/why-riscv-and-polkavm-for-smart-contracts/
