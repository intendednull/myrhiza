**Date:** 2026-05-29
**Status:** active
**Subject:** NEAR — gas parameters as protocol-version-keyed config; the "1 Tgas = 1 ms" calibration rule

# NEAR

NEAR's gas system is the most fully-documented reference for the single pattern
Myrhiza needs most: **recalibration is structurally a protocol-version event**,
not a patch. Where CosmWasm learned this through a halt
([cosmwasm.md](cosmwasm.md) CWA-2024-004), NEAR designed for it from the start by
keying its entire gas cost table to the protocol version.

## The calibration rule: 1 Tgas = 1 ms

NEAR's gas unit has a defined physical meaning: **"per 1 Tgas of execution, we
spend no more than 1 ms wall-clock time"** on minimum-required validator
hardware. Equivalently, a chunk filled with 1 Pgas (10^15 gas) must apply in at
most 1 second. Every gas parameter is set so that the **largest realistic input**
still respects this boundary. This is the answer to the question Myrhiza's
deferred fuel-cost-table child spec must answer: *what does a fuel unit mean?*
NEAR's answer is "a fixed slice of reference-hardware wall time," chosen so the
budget bounds real work, not abstract instruction counts in a vacuum. (Borrow
boundary: NEAR needs this to bound a *block*; Myrhiza needs it to size the 10M
`state-apply` budget — [`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md)
asserts 10M ≈ 10^6 typical instructions, exactly this kind of calibration claim,
and it should be *measured*, not asserted.)

## How parameters are estimated

The **runtime parameter estimator** (a binary in `nearcore`) validates parameters
against the 1-ms-per-Tgas rule using **two metrics**:

1. **Time** — wall-clock on standardized cloud hardware. Realistic but
   HW/environment-sensitive.
2. **ICount** — instruction counting via **QEMU emulation**. Consistent across
   machines but less representative of real performance.

Final values are typically set as the **higher** of the two metrics, **rounded
up**, then **manually sanity-checked by several reviewers**. The conservatism
(take the worse number, round up, human review) is itself a lesson: a gas table
that is *too cheap* is a DoS hole; *too expensive* merely wastes budget. NEAR
errs toward expensive.

### Borrow boundary — the ICount/QEMU trick is directly relevant

Myrhiza must justify its per-host-call fuel costs
([`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md):
`host.hash` = n*5, `host.verify-signature` = 5000, etc.). NEAR's ICount-via-QEMU
metric is a reproducible way to measure "how much work does this host operation
do" independent of the measuring machine — exactly the data the
fuel-cost-table child spec needs to set host-call costs proportional to host CPU
(the [`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md)
DoS-asymmetry mitigation). This is a concrete, borrowable methodology.

## Recalibration as a protocol version: `RuntimeConfigStore`

This is the mechanism. NEAR stores gas parameters as:

- A **base** file: `runtime_configs/parameters.yaml`.
- **Per-protocol-version diff** files (e.g. `53.yaml` overrides specific
  parameters for protocol version 53) — diffs, not full copies.
- A generated snapshot `runtime_configs/parameters.snap` recording every
  resolved value.

At runtime this resolves to a **`RuntimeConfigStore`**: "a sparse map from
protocol versions to complete runtime configurations
(`BTreeMap<ProtocolVersion, Arc<RuntimeConfig>>`)." A single `nearcore` binary
therefore carries **every** historical gas table and selects the right one per
chunk via `store.get_config(protocol_version)`. The protocol version for an epoch
is agreed by validator vote.

The implementation discipline is explicit and worth quoting: **"Never hard-code
parameter values. Never look them up in a different way."** Functions that depend
on gas parameters take a `&RuntimeConfig` argument (which implicitly fixes the
protocol version) and read it **fresh from the store per chunk**, never cached.

### Why this is the pattern Myrhiza wants — and where it breaks

The shape is exactly the Myrhiza rule "a Wasmtime-LTS bump = a kernel-major fuel
recalibration." NEAR shows the *clean* version: keep all cost tables, key them by
version, select per-execution, never hardcode. **But NEAR has a global protocol
version that advances by epoch under validator vote — Myrhiza does not.**
Myrhiza's per-author DAG has no global block height to key a config to. So
Myrhiza cannot adopt `RuntimeConfigStore` literally; it must carry the kernel
fuel-table version in `HeadsSummary` and treat skew as a flagged convergence
event ([`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md)).
The *discipline* (one cost table per version, never silently mutate, never
hardcode) transfers; the *coordination mechanism* (global epoch vote) does not.
This is the central tension developed in
[recalibration-as-version-bump.md](recalibration-as-version-bump.md).

## Concrete recalibration examples (recalibration is routine, not rare)

NEAR re-prices gas regularly via protocol upgrades — evidence that a metering
runtime must *expect* recalibration:

- The contract gas limit was raised from **300 TGas to 1 PGas** in a protocol
  upgrade (reported for protocol version 2.11.0).
- In that upgrade `wasm_touching_trie_node` dropped from 16,101,955,926 to
  2,280,000,000 gas, and a compute-cost split was introduced for
  `wasm_read_cached_trie_node`.

The exact protocol-version numbers above come from secondary documentation; the
*pattern* (parameters change at protocol-version boundaries, frequently) is the
load-bearing fact, and it is well established. **Flag:** the specific
"2.11.0" mapping was not cross-checked against a primary NEP — treat the version
number as indicative.

## Sources

- NEAR gas architecture overview — https://near.github.io/nearcore/architecture/gas/index.html
- NEAR runtime parameter estimator — https://near.github.io/nearcore/architecture/gas/estimator.html
- NEAR parameter definitions (`RuntimeConfigStore`, parameters.yaml, diff files) — https://near.github.io/nearcore/architecture/gas/parameter_definition.html
- NEAR gas (execution fees) docs — https://docs.near.org/protocol/transactions/gas
- NEAR Nomicon RuntimeConfig — https://nomicon.io/GenesisConfig/RuntimeConfig
- Myrhiza spec: determinism.md §5.3, risks.md §19, browser-native.md §14.2
