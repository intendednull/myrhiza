**Date:** 2026-05-29
**Status:** active
**Subject:** Substrate (context) — consensus WASM execution WITHOUT instruction metering; why the weights path is closed to Myrhiza

# Substrate (context)

Substrate is the framework underneath Polkadot's current relay chain and most of
its parachains. It is in this folder as the **counter-example**: it runs WASM in
consensus on **Wasmtime**, yet it does **not** instruction-meter that WASM. It
bounds work a completely different way — **offline-benchmarked weights** — and
that difference is exactly why its approach is *closed* to Myrhiza. Understanding
why sharpens why Myrhiza *must* meter.

## Two distinct WASM execution contexts in the Polkadot stack

It is easy to conflate these; they have different metering postures.

1. **The runtime** (the chain's state-transition logic, an upgradeable WASM blob).
   Executed by `sc-executor` using **Wasmtime (compiled)** with the
   `PoolingCopyOnWrite` instantiation strategy — this is the production default;
   "the runtime is run with wasmtime in all production cases" and "wasmi is way
   too slow for production use." Wasmi survives only as a sandboxing fallback and
   for some contract execution. **This WASM is NOT instruction-metered.**
2. **PVF — Parachain Validation Functions** (parachain logic re-executed by relay
   validators). Also Wasmtime-based today; **this** is the path JAM/PVM replaces
   with RISC-V ([polkavm-jam.md](polkavm-jam.md)).

## Why the runtime isn't metered: it's trusted code + benchmarked weights

The Substrate runtime author is **trusted**. The runtime is governance-upgraded
on-chain WASM, not arbitrary user-submitted code. So instead of metering every
instruction at execution time, Substrate **benchmarks** each extrinsic *offline*
on reference hardware and bakes the result in as a **weight** — a constant
("`pallet::call` costs N picoseconds") asserted by the pallet's `#[weight = ...]`
annotation. A block is bounded by summing the *declared* weights of its
extrinsics against a block weight limit; the VM itself does no per-instruction
accounting during consensus.

This is the same primitive NEAR uses to *calibrate* gas (offline benchmark on
reference HW, [near.md](near.md)) — but Substrate uses the benchmark **as** the
runtime bound, whereas NEAR/Soroban/CosmWasm use the benchmark only to *set the
table* and then meter at runtime. The difference is whether you trust the code:

| | Bounds work by | Trusts the code? |
|---|---|---|
| Substrate runtime | declared weights (no runtime metering) | **Yes** — governance-upgraded |
| NEAR / Soroban / CosmWasm contracts | runtime instruction metering | **No** — user-submitted |
| JAM PVFs | runtime RISC-V gas metering | **No** — parachain-submitted |

## Borrow boundary — why this path is closed to Myrhiza

Myrhiza `state-apply` is **arbitrary app-author code**, not trusted governance
code. The weight model relies on the author honestly declaring (and the
benchmark honestly capturing) the worst-case cost — which only works when the
author cannot be adversarial. A malicious Myrhiza app author would simply declare
a low weight and run an unbounded loop. Therefore:

- Myrhiza **must** instruction-meter (fuel), exactly as the contract-VMs do, and
  **cannot** use Substrate-style declared weights.
- The trust boundary is the deciding factor: "weights for trusted code, metering
  for untrusted code." Myrhiza's `state-apply` is untrusted, so it meters.

This also clarifies a subtle point about Myrhiza's spec: the per-host-call fuel
costs ([`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md))
are a **hybrid** — the *host* implementations of `host.hash`, `host.verify-signature`
etc. are trusted kernel code (Substrate-weight-style: charge a benchmarked flat
cost), while the *WASM* around them is untrusted and instruction-metered
(contract-VM-style). Myrhiza correctly uses weights for the trusted host calls
and metering for the untrusted guest — the same trust-based split, applied at the
host-call boundary. That hybrid is exactly right and is *validated* by the
Substrate-vs-contracts contrast.

## One determinism footnote from Substrate worth keeping

The Substrate/Wasmtime determinism work surfaces the same hazard Myrhiza's spec
already pins: **relaxed-SIMD differs across x86-64 and aarch64**, mitigated by
`Config::relaxed_simd_deterministic` (or, as Myrhiza does, disabling SIMD/relaxed-
SIMD for `state-apply` entirely —
[`determinism.md §5.2`](../../specs/2026-05-09-myrhiza-master-design/determinism.md),
[`../wasm-component-model/wasmtime.md`](../wasm-component-model/wasmtime.md)). The
Polkadot PVF-determinism issue tracker is a real-world catalogue of the
cross-arch WASM-determinism footguns Myrhiza's engine-config pins defend against.

## Sources

- Substrate `sc-executor` (Wasmtime production default) — https://github.com/paritytech/substrate/blob/master/client/executor/wasm_runtime.rs
- "Integrate Wasmtime for runtime execution" (PR #3869) — https://github.com/paritytech/substrate/pull/3869/
- Substrate benchmarking / weights — https://polkadot.study/tutorials/substrate-in-bits/docs/Benchmarking-substrate-pallet
- Polkadot PVF determinism issue — https://github.com/paritytech/polkadot/issues/1269
- Wasmtime deterministic execution — https://docs.wasmtime.dev/examples-deterministic-wasm-execution.html
- Myrhiza spec: determinism.md §5.2/§5.3
