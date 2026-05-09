**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric / Endo / SwingSet — production-hardened ocap + deterministic-replay JavaScript runtime, with a Cosmos chain attached

# Agoric / Endo / SwingSet

Three interlocking projects from one team:

- **Endo** — the hardened-JavaScript layer. SES (`lockdown()`-frozen primordials), compartments, marshal/pass-styles, CapTP, eventual-send, the bundle-source toolchain. Used standalone by MetaMask Snaps and LavaMoat; the most production-deployed ocap-JS surface in existence.
- **SwingSet** — the kernel-and-vats deterministic runtime. Vats are single-threaded JS event loops; the kernel mediates all I/O; transcript-driven replay rebuilds vat state byte-for-byte after restart, upgrade, or validator catch-up.
- **Agoric chain** — Cosmos SDK + CometBFT + SwingSet, where the SwingSet kernel is the application layer. Mainnet-1 launched 2022-10-27. Tokens BLD (governance/validation) and IST (collateral-backed stablecoin, sunset 2025-06-30).

Spritely Goblins ([`../spritely-ocapn/`](../spritely-ocapn/)) is the research-grade ocap sibling — same E lineage, same OCapN co-design, different language and deployment context. Agoric is the production-hardened cousin: it has shipped a deterministic-replay JS runtime to validators since 2022, and Endo has shipped to MetaMask Snaps users at the same scale. **The vat-snapshot-and-replay engineering is the most direct production analog to Myrhiza's `state-apply` purity requirement we have access to.**

## Key facts

| Fact | Value |
|---|---|
| Lineage | E (Mark Miller, 1997) → Caja (Google, 2007–2014, archived 2021-01-31) → SES-shim → Agoric (2018) |
| Steward | Agoric Systems Operating Company (Delaware C-corp, incorporated 2018-03-16) |
| Founders | Dean Tribble (CEO), Mark Miller (Chief Scientist), Brian Warner, Bill Tulloh |
| Foundation | No separately incorporated Agoric Foundation; governance via on-chain BLD votes + DCF + Engineering Council |
| Funding | Seed May 2018 (Polychain, Naval Ravikant, Zcash Co); Nov 2021 token sale $15.6M @ $0.40/BLD; ~$52.25M cumulative across rounds |
| License | Apache-2.0 across both `Agoric/agoric-sdk` and `endojs/endo` monorepos |
| Mainnet-0 | 2021-11-01 (validator-bootstrap; no user contracts) |
| Mainnet-1 | 2022-10-27 (Zoe + first contracts live) |
| Inter Protocol | Sunset 2025-06-30; TVL at sunset ~$103K (DefiLlama) |
| Current chain release | `agoric-upgrade-22b` on mainnet (Oct 2025); `agoric-upgrade-23-rc1` published 2026-05-06 |
| Current SwingSet | `@agoric/swingset-vat 0.33.0` (npm publish 2026-04-08, registry-modified 2026-05-07); `@agoric/swing-store 0.10.0`; `xsnap 0.15.0` |
| Current Endo | `ses 2.0.0` (2026-04-17); `@endo/captp 4.5.0`; `@endo/marshal 1.9.1`; `@endo/compartment-mapper 2.1.0`; `@endo/bundle-source 4.3.0` |
| JS engine | XS (Moddable), no JIT, deterministic, snapshot-friendly, integrated via `xsnap` worker |
| Persistence backend | SQLite (`better-sqlite3`); migrated from LMDB in `@agoric/swing-store` 0.9.0 (2023-05-19) |
| Cosmos integration | Cosmos SDK v0.50.14, CometBFT v0.38.17, IBC v8.7.0 (current mainnet `agoric-upgrade-22b`; in-flight `agoric-upgrade-23-rc1` moves to cosmos-sdk v0.53); CosmWasm disabled |
| Largest production deployment | **MetaMask Snaps** (depends on `ses ^1.15.0`, not `@endo/*` directly) — the actual at-scale ocap-JS production validation, not Agoric the chain |
| BLD price (May 2026) | ~99.4% below ATH of $0.7512 |
| TC39 SES status | Stage 1 (`proposal-ses` repo flagged "outdated"); active work is `proposal-compartments` (Stage 1) and a constellation of adjacent proposals |

(All version numbers verified against npm registry + `gh api Agoric/agoric-sdk` + `gh api endojs/endo` on 2026-05-09.)

## Contents

Each file is independent and skimmable standalone. ~17 files, ~3,100+ lines once README/lessons/glossary land.

**Hardened JS (Endo layer)**
- [**Hardened JS**](hardened-js.md) — SES, `lockdown()`, frozen primordials, compartments, `harden()`, the `ses-ava` test stack, real limitations (no CPU/RAM caps, regex censor false-positives, `class` syntax has no clean ocap pattern).
- [**Capabilities**](capabilities.md) — `E()` eventual-send + promise pipelining, three pass-styles (data/presence/promise), CapTP wire opcode table, marshal smallcaps encoding, distributed GC reachable-vs-recognizable distinction, `Far()`/`Exo()`.
- [**Modules and bundling**](modules-and-bundling.md) — compartment-mapper, `@endo/bundle-source` v4 endoZipBase64 with `b1-` SHA-512 hash of compartment-map (not raw bytes), no-dynamic-require rule, bundlecaps vs bundles separation.

**SwingSet (kernel + vats)**
- [**Architecture**](architecture.md) — kernel + vats + devices, c-list per vat, krefs/vrefs/orefs, run-queue, kernel state in SQLite, "everything is a vat" (comms, timer, vatAdmin).
- [**Vat model**](vat-model.md) — lifecycle, `buildRootObject`, delivery types (`message`/`notify`/`bringOutYourDead`), syscall API, transcript, static vs dynamic vats.
- [**CapTP and network**](captp-and-network.md) — comms vat, mailbox device, off-machine routing, OCapN co-design (Agoric is co-author but has not committed to deploying it).

**Determinism and persistence (load-bearing for Myrhiza)**
- [**Determinism**](determinism.md) — XS engine, lockdown-removed primordials, computron metering (`DEFAULT_CRANK_METERING_LIMIT = 1e8`), GC determinism by boundary, the four documented mainnet replay/snapshot incidents (`#7829` gcc-9, `#4297` "banana", `#4911`, `#5901`).
- [**Persistence**](persistence.md) — transcript-driven replay, XS heap snapshots, spans + incarnations, vat upgrade with the baggage convention, swing-store SQLite backend, the snapshot-as-cache stance (snapshots are NOT part of consensus; per `#5227`).

**Chain, contracts, apps, distribution**
- [**Chain**](chain.md) — Cosmos SDK + CometBFT, `agd` daemon, mainnet phase timeline, vstorage, IBC + dIBC.
- [**Contracts**](contracts.md) — Zoe (offer-safety, payout-liveness invariants), ERTP (issuers/brands/mints/purses/payments), the ocap-vs-Solidity thesis.
- [**Apps**](apps.md) — Inter Protocol (sunset), Fast USDC, MetaMask Snaps (the actual production validation), LavaMoat at ConsenSys.
- [**Distribution**](distribution.md) — two-monorepo split, npm release cadence, `agoric-upgrade-N` chain releases, cross-package compatibility constraints.

**Project lens**
- [**History**](history.md) — AMIX (1988) → E (1997) → Caja (2007–2014) → Agoric (2018) → mainnet-1 (2022) → orchestration pivot (2024) → Inter Protocol sunset (2025).
- [**Governance**](governance.md) — corporate entity, funding, BLD tokenomics, no separate foundation, single-entity governance risk.
- [**Comparisons**](comparisons.md) — vs Spritely Goblins / OCapN, vs Cap'n Proto + Cap'n Web, vs Solidity/EVM, vs CosmWasm, vs Iroh (orthogonal layer), vs browser hardened-JS / Realms / Workers.
- [**Critiques**](critiques.md) — Inter Protocol sunset (verbatim DCF self-critique), vat-replay overhead, SES library compatibility friction (incl. the `legacyRegeneratorRuntimeTaming: 'unsafe-ignore'` opt-out), MetaMask Snaps permission-prompt fatigue.
- [**Open problems**](open-problems.md) — 10 entries with Myrhiza disposition: WASM not in equation, GC determinism, distributed GC across machines, vat/component upgrade, OCapN cross-impl draft, browser SwingSet, hardened-sandbox is not a determinism guarantee, determinism vs performance, single-engine lock-in, bridge/orchestration is not composition.

**Reference**
- [**Lessons for Myrhiza**](lessons.md) — validates / avoid / borrow — **the consult-this-when-designing file.**
- [**Glossary**](glossary.md) — vat, kref/vref/oref, c-list, computron, baggage, Zoe, ERTP, IST, BLD, smallcaps, Far, Exo, sturdyref, lockdown, compartment, etc.

## Recommended reading order

For a Myrhiza spec author working on `state-apply` semantics or component upgrade: read [**lessons.md**](lessons.md) first, then [**determinism.md**](determinism.md) and [**persistence.md**](persistence.md) — these are the load-bearing files for our purity-of-state-apply story. Follow with [**vat-model.md**](vat-model.md) for the lifecycle analog and [**modules-and-bundling.md**](modules-and-bundling.md) for the bundle-distribution analog.

For a spec author working on the capability layer: [**lessons.md**](lessons.md), then [**capabilities.md**](capabilities.md) and [**hardened-js.md**](hardened-js.md), then [**comparisons.md**](comparisons.md) (vs Spritely Goblins / OCapN, vs Cap'n Proto).

For anyone evaluating "should we adopt OCapN as our cross-peer protocol": read [**captp-and-network.md**](captp-and-network.md) here and [`../spritely-ocapn/captp-and-ocapn.md`](../spritely-ocapn/captp-and-ocapn.md) together. They cover the same protocol from the two reference-impl teams.

## How to use this prior-art doc

This corpus is reference for future Myrhiza spec writing. Pin numbers and dates accurate as of the **Date:** in this README; bump the date when meaningful churn happens upstream (new mainnet upgrade, new SES major, etc.).

**Framing disclosure.** These docs are written from a Component-Model-as-foundation, P2P, capability-mediated-host-imports stance — most "Implications for Myrhiza" sub-sections frame Agoric's choices through that lens. Agoric chose JS, single-process Cosmos chain, and consensus-bound determinism; we choose WASM, peer-symmetric P2P, and per-peer state-apply. Future readers auditing whether *that choice* is itself right should weigh the corpus accordingly: it's a learn-from-Agoric-into-Myrhiza artifact, not a neutral catalog. The Spritely and Holochain prior-art folders carry the same disclosure for the same reason; the Iroh folder carries an additional load-bearing-dependency disclosure.

Unlike Iroh, Agoric/Endo is **not a library Myrhiza will hard-bake against** — Myrhiza is WASM, Endo is JS. The lessons we take are *design lessons* (vat-replay shape, computron metering, baggage upgrade convention, ocap pass-styles, marshal smallcaps), not API commitments.
