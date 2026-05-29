**Date:** 2026-05-29
**Status:** active
**Subject:** The metering-bug incident corpus — every verified metering failure, with IDs, versions, dates

# Incident corpus: "metering is hard"

The thesis of this folder, and the title of CosmWasm's own writeup, is that
**instruction-count metering is hard to get right.** This file is the evidence:
the metering failures we could verify, each with its identifier, affected
versions, dates, root cause, and the lesson for Myrhiza. The point is not to
disparage these projects — it is to show that a *production, audited, consensus-
critical* metering implementation still shipped these bugs, so Myrhiza must
treat its own fuel surface as a live TCB concern, not a solved problem.

Provenance: CosmWasm incidents are from CosmWasm's own published security
writeups; version/date facts cross-checked against crates.io and RFC-Editor.
Where a fact could not be primary-sourced it is flagged inline.

## CWA-2024-007 — undercharged gas via missed `if` instruction

- **System:** CosmWasm (`cosmwasm-vm` on Wasmer Singlepass).
- **Severity:** Medium. **Disclosed:** 2024-09-23.
- **Affected:** `cosmwasm-vm` < 1.5.8 / < 2.0.7 / < 2.1.4. **Fixed:** those
  releases (2024-09-23); the underlying Wasmer fix shipped in the **Wasmer crate
  4.4.0** (2024-10-04, verified crates.io).
- **Root cause:** the metering middleware injects gas checkpoints only "before
  branching instructions or branch targets," but the **`if` instruction was
  omitted** from that set. When an `if` block never executes, "all the
  instructions before it are never metered, leading to a potentially massive
  undercharging of gas."
- **Lesson for Myrhiza:** WASM's structured control flow means metering must
  enumerate *every* control-flow construct correctly. Myrhiza delegates this to
  Wasmtime's `consume_fuel`; a missed-construct bug **in Wasmtime** would hit
  Myrhiza identically. Track upstream Wasmtime fuel-metering correctness. This is
  also the bug JAM/PVM cites to justify leaving WASM ([polkavm-jam.md](polkavm-jam.md)).

## CWA-2024-008 — gas report suppressed by panic

- **System:** CosmWasm (`libwasmvm`). **Severity:** Medium. **Disclosed:**
  2024-09-23. **Fixed:** same releases as CWA-2024-007.
- **Root cause:** a contract could trigger a **panic** via unchecked arithmetic
  when parsing data sections. The panic handler does **not return a gas report**,
  so work was done with gas accounting bypassed entirely. Fix: `checked`
  arithmetic returning an error (which charges gas) instead of panicking.
- **Lesson for Myrhiza:** fuel must be charged on **every** exit path — normal
  return, trap, *and* any host-side error. A kernel host-call panic that bypasses
  fuel accounting is the same vector. Myrhiza's spec already requires resource-
  handle revocation on "any path (normal exit, fuel exhaustion, trap, fatal error,
  operator-initiated kill)" ([`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md));
  the fuel-accounting equivalent must hold too.

## CWA-2024-004 — incomplete gas patch → consensus halt (the artifact-cache class)

- **System:** CosmWasm (chains running it: e.g. via `wasmd`). **Disclosed/patched:**
  2024-08-08. **Affected/fixed series:** backported to **1.5, 2.0, 2.1**.
- **What happened:** a gas-repricing patch ("Increase gas for breaking operations
  loop, br, call, return… from 170 to 1610"; reduce others 170→115, to hit the
  1-Teragas/second target) caused **consensus failures within hours**.
- **Root cause:** gas pricing is **baked into the compiled artifact at compile
  time**, and compiled contracts are **cached on disk**. Upgraded nodes that still
  held old cached artifacts metered at old prices; freshly-syncing nodes metered
  at new prices → different `gas_used` for the same tx → **fork**. Same-day
  follow-up patches forced cache invalidation.
- **Lesson for Myrhiza (the load-bearing one):** Myrhiza's `precompile_component`
  image cache is the same hazard. **The fuel-table version MUST be part of the
  precompiled-artifact cache key.** See
  [recalibration-as-version-bump.md](recalibration-as-version-bump.md) and the
  [cosmwasm.md](cosmwasm.md) borrow-boundary callout.

## The float-nondeterminism class (handled, not an incident — but a near-miss category)

Across all WASM consensus VMs, **nondeterministic float behavior** (NaN payload
propagation, some int↔float conversions, relaxed-SIMD) is a recurring divergence
*category* that each system neutralizes pre-emptively: CosmWasm via a deterministic-
float middleware; Wasmtime via NaN canonicalization + disabling relaxed-SIMD
([substrate-context.md](substrate-context.md)); Myrhiza by **banning floats
outright** in `state-apply`
([`determinism.md §5.2`](../../specs/2026-05-09-myrhiza-master-design/determinism.md)).
Myrhiza's ban is the strongest mitigation in the corpus — it removes the category
rather than canonicalizing it. Listed here because it is the same "two honest
nodes compute different bits" failure mode as a metering bug, on a different axis.

## Pattern across the corpus

| Bug class | Mechanism | Myrhiza exposure | Myrhiza mitigation |
|---|---|---|---|
| Missed control-flow construct | injection misses `if`/branch | inherited from Wasmtime `consume_fuel` | track upstream; treat fuel as TCB |
| Bypassed accounting on error path | panic skips gas report | host-call panic skips fuel | charge fuel on every exit path |
| Stale artifact cache | precompiled blob baked old costs | `precompile_component` cache | fuel-table version in cache key |
| Float nondeterminism | NaN/SIMD differ cross-arch | any float in `state-apply` | floats banned outright |

## Sources

- "Metering is hard — CosmWasm security issues explained" (CWA-2024-007/008) — https://medium.com/cosmwasm/metering-is-hard-cosmwasm-security-issues-explained-a797511cd54e
- "The incomplete gas patch and why it caused consensus failures" (CWA-2024-004) — https://medium.com/cosmwasm/the-incomplete-gas-patch-and-why-it-caused-consensus-failures-173547ef02de
- crates.io: `wasmer` 4.4.0 (2024-10-04); `cosmwasm-vm` 1.5.8 / 2.0.7 / 2.1.4 (2024-09-23)
- Myrhiza spec: determinism.md §5.2, risks.md §19
