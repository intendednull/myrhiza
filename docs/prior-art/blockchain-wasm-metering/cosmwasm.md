**Date:** 2026-05-29
**Status:** active
**Subject:** CosmWasm — Wasmer Singlepass + metering middleware; the richest WASM-gas incident corpus

# CosmWasm

CosmWasm is the WASM smart-contract platform for the Cosmos SDK ecosystem
(Osmosis, Neutron, Sei, Secret Network, etc.). Contracts are WASM modules; the
host VM is `cosmwasm-vm`, built on **Wasmer**. It is the system in this folder
whose metering bugs are best documented publicly, which is precisely why it is
the most useful: it shows that **getting injected WASM metering right is hard**,
and what goes wrong when you don't.

## The VM stack

- **Wasmer**, not Wasmtime. CosmWasm integrated Wasmer 1.0 on **2020-12-23**.
- Two backends: **Singlepass** (single-pass, non-optimizing compiler) is used
  **on-chain** (production); Cranelift is the development default. Per the
  integration announcement, the split is because "Singlepass does not yet support
  Windows." Both backends "support gas metering and protection from
  undeterministic float operations."
- **Metering is a Wasmer middleware**: it rewrites the WASM module to inject a
  gas-counter decrement, then Singlepass compiles the rewritten module. Gas is
  counted in **WASM-instruction space** (portable), not native cycles.
- Floats are made deterministic by a middleware that rejects/canonicalizes
  nondeterministic float behavior — the same class of concern Myrhiza handles by
  banning floats outright in `state-apply`
  ([`determinism.md §5.2`](../../specs/2026-05-09-myrhiza-master-design/determinism.md)).

## The metering optimization that caused the bugs

Wasmer's metering middleware does **not** inject a decrement before *every*
instruction — that would be too slow. It injects only "before *branching
instructions or branch targets*." The cost of a straight-line basic block is
charged once, up front, at the block's entry. This is the standard, correct
optimization: a basic block has a fixed instruction count, so you can charge for
all of it at the top. **The bug class is: which instructions count as branches?**

## CWA-2024-007 — the missed `if` (the headline metering bug)

Disclosed **2024-09-23** (Medium severity). The metering injection logic
**omitted the `if` instruction**. `if` branches conditionally, so it *is* a
branch — but it was not in the set of instructions that triggered a metering
checkpoint. Consequence, verbatim: when an `if` block never executes, "all the
instructions before it are never metered, leading to a potentially massive
undercharging of gas." An attacker could craft a contract whose real work hides
behind a never-taken `if`, doing near-unbounded computation for almost no gas.

**The fix was upstreamed.** Per CosmWasm: "We communicated our fix for this with
Wasmer before the release, so they could upstream it and release it in version
**4.4.0**." This is the **Wasmer crate** version 4.4.0, published **2024-10-04**
(verified on crates.io). It is **not** a `cosmwasm-vm` version — the corresponding
`cosmwasm-vm` releases were **1.5.8 / 2.0.7 / 2.1.4**, all published
**2024-09-23** (verified on crates.io). *This distinction matters and is easy to
get wrong: the gap-analysis report and the original task both phrase it as
"~4.4.0" — that is the Wasmer crate, confirmed.*

## CWA-2024-008 — panic suppresses the gas report

Also disclosed **2024-09-23** (Medium). A contract could trigger a **panic** in
VM code via unchecked arithmetic when parsing data sections. Because the panic
handler in `libwasmvm` does not return a gas report, an attacker could perform
work and have the gas accounting *not run at all*. Fix: `checked` arithmetic that
returns an error (which *does* charge gas) instead of panicking.

**The general lesson:** metering is only sound if *every* exit path reports gas.
A panic, an early trap, an unhandled error that bypasses the accounting code is a
gas-evasion vector. For Myrhiza, the analogue is: fuel must be charged on *every*
`state-apply` exit path including traps — Wasmtime's `Store::get_fuel` after a
`Trap::OutOfFuel` is well-defined, but any host-call-side panic in the kernel
must not bypass fuel accounting.

## CWA-2024-004 — the incomplete gas patch and the Aug-2024 consensus halt

This is the most instructive incident in the folder, and it is **not a bug in
the metering code — it is a bug in how a metering *change* was rolled out**.

On **2024-08-08** CosmWasm shipped a gas-pricing patch (CWA-2024-004) that
re-priced WASM operations to realign with the "1 Teragas/second" target:
"Increase the gas for breaking operations (loop, br, call, return, …) from 170
to 1610" and reduce other operations from 170 to 115. Backported to the **1.5,
2.0, and 2.1** release series.

It caused **consensus failures within hours**. Root cause: **gas pricing is baked
in at compile time.** When a contract is first uploaded it is compiled once and
the *compiled artifact* (with the gas-injection baked in at the old prices) is
**cached on disk** (`~/.myd/wasm/wasm/cache`). A node that upgraded its binary
but still had old compiled artifacts in cache kept metering at the **old**
prices, while a freshly-syncing node compiled at the **new** prices. Same
transaction, different `gas_used`, **chain fork**. Same-day follow-up patches
forced cache invalidation so all nodes recompiled.

### Borrow boundary — this is Myrhiza's `precompile_component` hazard, exactly

Myrhiza ships **precompiled component images** and peers `Engine::deserialize`
them on first use ([`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md),
[`../wasm-component-model/wasmtime.md`](../wasm-component-model/wasmtime.md)
"Snapshots and pre-initialised images"). A precompiled image bakes in the
fuel-cost behavior **of the Wasmtime version that compiled it**. If a kernel
upgrade changes the fuel table but a peer reuses a cached precompiled artifact
from the old kernel, that peer meters at old costs while a peer that recompiled
meters at new costs — **the CosmWasm halt, reproduced in Myrhiza**. The
mitigation the spec must adopt (and CosmWasm learned the hard way): **the
precompiled-artifact cache key MUST include the fuel-table / kernel version**, so
a recalibration invalidates every cached image. See
[recalibration-as-version-bump.md](recalibration-as-version-bump.md).

## What CosmWasm validates for Myrhiza

- Injected WASM-space metering + a single-pass deterministic compiler is a
  *working* production design (Myrhiza's Wasmtime+`consume_fuel` is the same
  family — [the-determinism-problem.md](the-determinism-problem.md)).
- Float-nondeterminism is real and must be handled at the engine level (Myrhiza
  bans floats; CosmWasm canonicalizes — both valid).

## Sources

- "Metering is hard — CosmWasm security issues explained" (CWA-2024-007/008) — https://medium.com/cosmwasm/metering-is-hard-cosmwasm-security-issues-explained-a797511cd54e
- "The incomplete gas patch and why it caused consensus failures" (CWA-2024-004) — https://medium.com/cosmwasm/the-incomplete-gas-patch-and-why-it-caused-consensus-failures-173547ef02de
- "Wasmer 1.0 integrated into CosmWasm" (2020-12-23) — https://medium.com/cosmwasm/wasmer-1-0-integrated-into-cosmwasm-2fa87437458c
- crates.io: `wasmer` 4.4.0 published 2024-10-04; `cosmwasm-vm` 1.5.8 / 2.0.7 / 2.1.4 published 2024-09-23 (max stable 3.0.7 as of 2026-05-21)
- Myrhiza spec: determinism.md §5.2, risks.md §19
