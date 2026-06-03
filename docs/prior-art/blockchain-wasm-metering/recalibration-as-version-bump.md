**Date:** 2026-05-29
**Status:** active
**Subject:** Cross-system synthesis — a cost-table change is a major-version event; the no-global-epoch problem for Myrhiza

# Recalibration as a version bump

This is the synthesis file for the single most load-bearing pattern in the
folder for Myrhiza. It collects how each system treats a change to its
cost/gas/fuel table, then maps the pattern onto Myrhiza's specific constraint:
**Myrhiza has no global epoch at which to coordinate the cutover.**

## The universal rule across all four systems

> A change to the metering cost table is a **protocol-version event**, never a
> silent patch — and every node must agree on *which* cost table applies to a
> given execution.

| System | Cost-table change is… | How "which table" is selected |
|---|---|---|
| **NEAR** | a new protocol version | `RuntimeConfigStore`: `BTreeMap<ProtocolVersion, RuntimeConfig>`; protocol version per epoch by validator vote ([near.md](near.md)) |
| **Soroban** | a network-config / ledger-settings change by consensus | network-configurable cost params, updated by CAP/upgrade ([soroban.md](soroban.md)) |
| **CosmWasm** | a consensus-breaking chain upgrade | binary version + **cache invalidation** (learned via the Aug-2024 halt, [cosmwasm.md](cosmwasm.md)) |
| **JAM/PVM** | a protocol constant in the Gray Paper | fixed per-instruction costs are protocol constants ([polkavm-jam.md](polkavm-jam.md)) |

NEAR is the cleanest mechanism: **keep every historical cost table, key it by
version, select per-execution, never hardcode, never mutate in place.**

## The two failure modes recalibration must avoid

Both are demonstrated in the corpus, both apply to Myrhiza:

1. **Silent divergence (the live-mutation failure).** If node A meters at the new
   table and node B at the old, they disagree on the result → fork. NEAR's
   `RuntimeConfigStore` ("never hard-code… read fresh per chunk") and Soroban's
   consensus-gated params both exist to make the active table an explicit,
   versioned, agreed value rather than ambient client state.
2. **Stale-cache divergence (the artifact-cache failure).** Even with the *source*
   table updated, a node can keep metering at old costs if it reuses a
   **precompiled/cached artifact** that baked in the old costs. This is exactly
   the CosmWasm CWA-2024-004 halt ([cosmwasm.md](cosmwasm.md)): the compiled
   contract cache survived the binary upgrade. The fix: invalidate the cache when
   the cost table changes.

## Mapping onto Myrhiza

Myrhiza's spec already states the rule in NEAR-compatible language:

- **"A Wasmtime-LTS bump = a kernel-major fuel recalibration"**
  ([`browser-native.md §14.2`](../../specs/2026-05-09-myrhiza-master-design/browser-native.md),
  [`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md)): "LTS
  bump is a kernel MAJOR version bump (convergence-breaking)." Cranelift fuel cost
  tables "may shift between Wasmtime majors" — the same per-version cost-table drift
  NEAR keys around.
- **Kernel announces its fuel-table version in `HeadsSummary`**
  ([`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md)); skew
  surfaces an "upgrade recommended for convergence guarantee" warning and is
  tagged by drift detection
  ([`convergence.md §4.7`](../../specs/2026-05-09-myrhiza-master-design/convergence.md))
  as `kernel-version-skew`, not generic drift.

So Myrhiza has the **discipline** (cost-table change = version bump, announce the
version) and the **detection** (skew flagging). What it does **not** have, and
cannot have, is NEAR's coordination mechanism.

### The no-global-epoch problem (the genuinely harder bit)

NEAR can say "from epoch E / protocol version V, the new table applies" because
all validators vote on a single global protocol version that advances in lockstep.
Myrhiza's events live on **per-author Merkle DAGs** with a deterministic topo-sort
([`convergence.md §4.1`](../../specs/2026-05-09-myrhiza-master-design/convergence.md))
and **no global block height or epoch**. There is no consensus moment to declare
"all peers switch to fuel-table v2 now." Consequences:

- A recalibration cannot be a *coordinated cutover*; it is a kernel binary upgrade
  that propagates **peer-by-peer at each peer's own pace**.
- During the rollout window, old-kernel and new-kernel peers **coexist** and may
  disagree on fuel exhaustion for the same event. The spec accepts this as a
  bounded, flagged divergence (skew warning + drift tag), not a hard fork — there
  is no slashing and no single canonical chain to fork.
- This means Myrhiza's recalibration is **softer** than a blockchain's (no
  instantaneous global break) but also **less clean** (a genuine convergence-
  divergence window exists until the network finishes upgrading). The blockchains
  get a hard cutover for free from consensus; Myrhiza trades that for an upgrade
  window it must tolerate.

### Concrete recommendations the corpus supports for the deferred child spec

For the fuel-cost-table child spec
([`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md)):

1. **Version the fuel table explicitly** and carry the version in `HeadsSummary`
   (spec already does). Treat the table as immutable per version (NEAR
   discipline) — never patch a released table in place.
2. **Include the fuel-table version in the `precompile_component` cache key** so a
   recalibration invalidates cached images. This is the direct CosmWasm-halt
   mitigation and is the *one concrete code-level requirement* the incident corpus
   forces ([cosmwasm.md](cosmwasm.md) borrow-boundary callout).
3. **Calibrate the table to a wall-clock budget on reference HW** (NEAR's
   1-Tgas=1-ms rule) and **measure** the "10M fuel ≈ 10^6 instructions" claim
   rather than assert it — use a reproducible metric (NEAR's QEMU `icount`) so the
   numbers are defensible.
4. **Make host-call costs reflect host CPU** (the [`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md)
   DoS-asymmetry mitigation), using Soroban-style `a + b*input_size` linear models
   per host call, calibrated offline.

## Sources

- NEAR parameter definitions / `RuntimeConfigStore` — https://near.github.io/nearcore/architecture/gas/parameter_definition.html
- CosmWasm "The incomplete gas patch" — https://medium.com/cosmwasm/the-incomplete-gas-patch-and-why-it-caused-consensus-failures-173547ef02de
- Soroban fees/metering — https://developers.stellar.org/docs/learn/fundamentals/fees-resource-limits-metering
- Myrhiza spec: determinism.md §5.3, browser-native.md §14.2, risks.md §19, convergence.md §4.1/§4.7
