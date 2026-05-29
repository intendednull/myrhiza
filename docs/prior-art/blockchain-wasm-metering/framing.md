**Date:** 2026-05-29
**Status:** active
**Subject:** Framing — the borrow boundary between consensus-VM gas metering and Myrhiza's convergence fuel

# Framing: what transfers, what does not

All four systems in this folder are **consensus VMs**. They meter computation to
do two things Myrhiza does *not* do:

1. **Price** computation for a fee market (gas → fee → block-space auction).
2. **Bound** validator work so a Byzantine block producer cannot grief honest
   validators with an unbounded-cost transaction.

Myrhiza is a **P2P convergence runtime**. It meters fuel to do exactly *one*
thing:

- **Bound** `state-apply` so that running out of fuel is a *convergent* outcome
  — every peer that applies the same event traps at the same instruction, so
  state stays identical across peers
  ([`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md)).

That asymmetry is the whole borrow boundary. Keep it in view in every file.

## What transfers (the discipline)

| Borrowable | Where it lands in Myrhiza |
|---|---|
| **Cost-of-an-instruction must be identical across x86-64 / aarch64 / riscv64**, or two honest nodes diverge | `state-apply` fuel exhaustion must converge; the cross-peer agreement check ([`mvp.md §15.1`](../../specs/2026-05-09-myrhiza-master-design/mvp.md)) fails if it doesn't |
| **A cost-table change is a protocol-version event**, never a silent patch | The Wasmtime-LTS-bump = kernel-major rule ([`browser-native.md §14.2`](../../specs/2026-05-09-myrhiza-master-design/browser-native.md), [`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md)) |
| **Host-function cost must reflect host CPU, not WASM-instruction count** | The DoS-asymmetry mitigation ([`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md)); Myrhiza already pins per-host-call fuel costs ([`determinism.md §5.3`](../../specs/2026-05-09-myrhiza-master-design/determinism.md)) |
| **Compiled-artifact caches can carry a stale cost table across a "fix"** | Myrhiza's `Engine::precompile_component` caching ([`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md)) is the same hazard as CosmWasm's Aug-2024 halt ([incident-corpus.md](incident-corpus.md)) |
| **Calibrate gas to a wall-clock budget on reference HW** | The "what does 10M fuel units *mean*" question the deferred child spec must answer ([near.md](near.md) §"1 Tgas = 1 ms") |

## What does NOT transfer (the consensus layer)

- **Fee markets / gas pricing economics.** Myrhiza has no auction for block
  space, no gas price, no validator rewards. Anything in these systems about
  *pricing* (vs. *bounding*) is out of scope. Soroban's elaborate fee structure,
  NEAR's `send_sir`/`send_not_sir`/`execution` three-cost split — ignore the fee
  parts, study the determinism parts.
- **Byzantine-quorum threat model.** These VMs assume a malicious *block
  producer* who must not be able to make honest validators do unbounded work
  *and still be paid*. Myrhiza's threat model is a malicious *app author* whose
  event every peer optionally applies. The mitigation overlaps (bound the work)
  but the economics differ entirely.
- **Slashing / on-chain penalties.** JAM slashes validators that diverge; NEAR
  and Soroban reject the block. Myrhiza has no slashing — divergence is a drift
  signal ([`convergence.md §4.7`](../../specs/2026-05-09-myrhiza-master-design/convergence.md)),
  not a punishable offense.
- **Global total order.** Consensus VMs execute in a single agreed sequence per
  block. Myrhiza executes over a per-author Merkle DAG with a deterministic
  topo-sort ([`convergence.md §4.1`](../../specs/2026-05-09-myrhiza-master-design/convergence.md));
  there is no global block height to key a `RuntimeConfig` to. This is the one
  place NEAR's mechanism does *not* port cleanly — see
  [recalibration-as-version-bump.md](recalibration-as-version-bump.md).

## The one structural mismatch to flag loudly

NEAR keys its `RuntimeConfig` to a **protocol version**, which advances by
**epoch** under validator vote. Myrhiza has **no global epoch** — every author's
chain advances independently. So Myrhiza cannot say "from block N, the new gas
table applies." The Myrhiza analogue must be a **kernel version** carried in
`HeadsSummary`
([`risks.md §19`](../../specs/2026-05-09-myrhiza-master-design/risks.md) already
sketches this), with kernel-version-skew treated as a flagged convergence event.
That is a genuinely harder problem than the blockchain version because there is
no consensus moment to coordinate the cutover. See
[recalibration-as-version-bump.md](recalibration-as-version-bump.md) for the full
treatment.

## Sources

- Myrhiza master spec — `docs/specs/2026-05-09-myrhiza-master-design/` (determinism.md §5.3, risks.md §19, browser-native.md §14.2, convergence.md §4.1/§4.7, mvp.md §15.1)
- Companion: [`../wasm-component-model/wasmtime.md`](../wasm-component-model/wasmtime.md) (fuel vs epoch)
