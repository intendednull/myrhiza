**Date:** 2026-05-29
**Status:** active
**Subject:** THE decision file — validates / avoid / borrow, tied to Myrhiza's deferred fuel-cost-table spec, DoS-asymmetry risk, and Wasmtime-LTS-bump rule

# Lessons

This is the file the folder exists for. Every lesson ties to a named Myrhiza
decision surface: the **deferred fuel-cost-table child spec**
([`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md)),
the **DoS-asymmetry risk**
([`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md)), and the
rule that a **Wasmtime-LTS bump = a kernel-major fuel recalibration**
([`browser-native.md §14.2`](../../specs/2026-05-09-myrhiza-master-design/browser-native.md)).
Read [framing.md](framing.md) first — the borrow is the metering *discipline*, not
the consensus layer.

## Validates (Myrhiza choices this corpus supports)

- **Instruction-count fuel for `state-apply` is the right primitive for untrusted
  code.** All three contract VMs (CosmWasm, NEAR, Soroban) and JAM meter untrusted
  guest code by instruction count; Substrate's *non*-metered path is reserved for
  *trusted* governance code ([substrate-context.md](substrate-context.md)). Myrhiza
  `state-apply` is untrusted app-author code → metering is correct, weights are not.
  *(Surface: determinism.md §5.3.)*

- **Counting in WASM-instruction space, not native cycles, is mandatory and
  correct.** Every system meters in source-ISA space because native cost is
  non-portable across x86-64/aarch64/riscv64 ([the-determinism-problem.md](the-determinism-problem.md)).
  Wasmtime's `consume_fuel` does this; Myrhiza relies on it correctly.

- **A cost-table change as a major-version event** is the universal rule
  ([recalibration-as-version-bump.md](recalibration-as-version-bump.md)). Myrhiza's
  "Wasmtime-LTS bump = kernel-major recalibration" matches NEAR/Soroban/CosmWasm/
  JAM exactly. *(Surface: browser-native.md §14.2, risks.md §19.)*

- **Separate CPU and memory budgets.** Soroban meters both independently
  ([soroban.md](soroban.md)); Myrhiza's fuel budget + 64 MB memory cap is the same
  structure and is validated. *(Surface: determinism.md §5.3.)*

- **Pinning the engine/compiler config is load-bearing precisely because Myrhiza
  chose a JIT.** Soroban *avoided* this entire problem by using an interpreter
  ([soroban.md](soroban.md)); since Myrhiza uses Cranelift, the opt-level/strategy/
  feature-flag pins ([`determinism.md §5.2`](../../specs/2026-05-09-myrhiza-master-design/determinism.md))
  are the price of that choice and are correctly mandatory.

- **Banning floats outright** is the strongest float-nondeterminism mitigation in
  the corpus — stronger than the canonicalization others use
  ([incident-corpus.md](incident-corpus.md)). *(Surface: determinism.md §5.2.)*

## Avoid (specific pitfalls + Myrhiza mitigation)

- **Stale precompiled-artifact cache carrying an old cost table → divergence.**
  The CosmWasm Aug-2024 halt (CWA-2024-004, [cosmwasm.md](cosmwasm.md)). Myrhiza's
  `precompile_component` image cache is the same hazard.
  **Mitigation (concrete, do this):** include the fuel-table / kernel version in
  the precompiled-artifact cache key, so a recalibration invalidates every cached
  image. This is the single most concrete code-level requirement the incident
  corpus forces. *(Surface: risks.md §19 precompile caching; determinism.md §5.3.)*

- **Metering injection that misses a control-flow construct → undercharging.**
  CWA-2024-007's missed `if` ([incident-corpus.md](incident-corpus.md)). Myrhiza
  cannot audit Wasmtime's injection but inherits its correctness.
  **Mitigation:** treat Wasmtime fuel-metering correctness as a TCB dependency;
  track upstream advisories; the byte-level lint that already defends the feature
  pins ([`determinism.md §5.2`](../../specs/2026-05-09-myrhiza-master-design/determinism.md))
  is the right posture to extend.

- **Any exit path that bypasses fuel accounting → free computation.** CWA-2024-008's
  panic-skips-gas-report.
  **Mitigation:** charge fuel on every `state-apply` exit (return, trap, host-side
  error/panic); pair with the existing resource-handle-revocation-on-any-path rule
  ([`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md)).

- **Asserting "N fuel ≈ M instructions" without measuring it.** The spec asserts
  10M fuel ≈ 10^6 typical instructions
  ([`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md))
  — that is exactly the kind of claim NEAR *measures* with a reproducible metric.
  **Mitigation:** calibrate against a wall-clock budget on reference HW (NEAR's
  1-Tgas=1-ms) and verify with QEMU `icount` rather than asserting. *(Surface:
  the deferred fuel-cost-table child spec.)*

- **Host-call costs that ignore host CPU → DoS asymmetry.** Instruction metering
  alone cannot stop a cheap-to-call, expensive-to-run host function from draining
  host CPU ([open-problems.md](open-problems.md) §1).
  **Mitigation:** Myrhiza already pins per-host-call fuel costs
  ([`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md));
  make them *CPU-proportional and offline-calibrated*, Soroban `a + b*input_size`
  style. *(Surface: risks.md §19 DoS-asymmetry — this is the direct mitigation.)*

- **Don't hardcode the cost table or read it ad hoc.** NEAR's explicit rule:
  "Never hard-code parameter values. Never look them up in a different way"
  ([near.md](near.md)). **Mitigation:** route fuel costs through a single
  versioned table object selected by kernel/fuel-table version, never inline
  constants scattered across the kernel.

## Borrow (primitives worth studying before writing the child spec)

- **NEAR's `RuntimeConfigStore` shape** — `BTreeMap<version, CostTable>`, keep all
  historical tables, select per-execution, immutable per version. Adapt it to
  *kernel version* (carried in `HeadsSummary`) since Myrhiza has no global epoch
  ([recalibration-as-version-bump.md](recalibration-as-version-bump.md)). Study the
  *discipline*; redesign the *selection key*.

- **NEAR's calibration methodology** — two metrics (wall-clock Time + QEMU ICount),
  take the worse, round up, human review, target 1-Tgas=1-ms. Directly applicable
  to setting Myrhiza's fuel budget and host-call costs ([near.md](near.md)).

- **Soroban's linear cost model per cost-type** (`y = a + bx`, offline-fitted) —
  the right shape for per-host-call fuel costs ([soroban.md](soroban.md)).

- **Soroban's "outer loops in WASM, inner loops in native host functions"** — the
  architecture that makes interpreter/metered execution affordable; Myrhiza's
  deterministic helper set already follows it ([soroban.md](soroban.md)).

- **The interpreter fallback (Wasmi/Soroban; Wasmtime Pulley)** — keep as the
  contingency if Cranelift fuel-determinism proves unstable across a bump
  ([soroban.md](soroban.md), [open-problems.md](open-problems.md) §3).

- **JAM's diagnosis that WASM control flow is a metering liability** — not a path
  Myrhiza can take (WASM is a hard dep), but it correctly identifies *why* the
  `if`-class bug exists and validates treating fuel as a TCB concern
  ([polkavm-jam.md](polkavm-jam.md)).

## Recommendation matrix (for the deferred fuel-cost-table child spec)

| Decision | Corpus says | Do for Myrhiza |
|---|---|---|
| Meter untrusted `state-apply`? | yes (all contract VMs) | yes — fuel, not weights |
| Count native or WASM cost? | WASM (all) | WASM (`consume_fuel`) |
| CPU + memory separate budgets? | yes (Soroban) | yes (fuel + 64 MB cap) |
| Cost-table change = version bump? | yes (all) | yes (kernel-major) |
| Cost-table cutover coordination | global epoch vote | **no analogue** — peer-by-peer rollout window, skew-flagged |
| Precompiled cache key includes table version? | learned the hard way (CosmWasm) | **yes — mandatory** |
| Calibrate or assert costs? | calibrate (NEAR/Soroban) | calibrate, measure the 10M claim |
| Host-call cost model | CPU-proportional, linear, offline (Soroban) | `a + b*n`, offline-calibrated |
| Charge fuel on every exit path? | yes (CWA-2024-008) | yes |

## Sources

- All sibling files in this folder (CosmWasm, NEAR, Soroban, PolkaVM-JAM, Substrate, incident-corpus, recalibration).
- Myrhiza spec: determinism.md §5.2/§5.3, risks.md §19, browser-native.md §14.2.
- Companion: [`../wasm-component-model/wasmtime.md`](../wasm-component-model/wasmtime.md) (fuel vs epoch).
