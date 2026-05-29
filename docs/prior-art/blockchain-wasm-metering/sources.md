**Date:** 2026-05-29
**Status:** active
**Subject:** Consolidated sources + verification notes for high-risk facts

# Sources & verification notes

Aggregate of URLs cited across the folder, plus explicit notes on which
high-risk facts (versions, dates, IDs) were verified and how, and which claims
could **not** be verified and are flagged for the reviewer.

## Verified facts (with method)

| Fact | Value | How verified |
|---|---|---|
| CosmWasm metering fix upstreamed to Wasmer | **Wasmer crate 4.4.0**, published **2024-10-04** | crates.io API for `wasmer`; matches CosmWasm article's "upstream… version 4.4.0" |
| CosmWasm `cosmwasm-vm` fixed releases for CWA-2024-007/008 | **1.5.8 / 2.0.7 / 2.1.4**, all **2024-09-23** | crates.io API for `cosmwasm-vm` (these exact versions+date present) |
| CWA-2024-004 incomplete gas patch | **2024-08-08**, backported to 1.5/2.0/2.1 series | CosmWasm "incomplete gas patch" article |
| CWA-2024-007 root cause | metering omitted the `if` instruction → undercharging | CosmWasm "Metering is hard" article (verbatim quotes) |
| CWA-2024-008 root cause | panic skips gas report; fixed with checked math | CosmWasm "Metering is hard" article |
| Wasmer 1.0 integrated into CosmWasm | **2020-12-23**; Singlepass used on-chain, Cranelift dev default | CosmWasm "Wasmer 1.0 integrated" article |
| NEAR calibration rule | "per 1 Tgas of execution, ≤ 1 ms wall-clock"; 1 Pgas ≤ 1 s | nearcore gas architecture + estimator docs |
| NEAR `RuntimeConfigStore` | `BTreeMap<ProtocolVersion, Arc<RuntimeConfig>>`; parameters.yaml + per-version diff files (e.g. 53.yaml) | nearcore parameter_definition doc (verbatim quotes) |
| Soroban VM | Wasmi interpreter (~13 KLOC); no JIT | Stellar "Why Doesn't Soroban Use a JIT?" |
| Soroban cost model | linear `y = a + bx` per cost-type, separate CPU + memory, offline-calibrated | Soroban fees/metering docs (verbatim) |
| PolkaVM status | pre-release ("do not use in production"); recompiler x86-64/ARM64 + interpreter | PolkaVM README |
| PolkaVM authorship | Parity Technologies; primary author **Jan Bujak** (`koute`) — *not* on the README | `polkavm-common` Cargo.toml `authors` field; `github.com/koute` profile |
| PVM gas | fixed cost per instruction; deterministic for consensus | JAM PVM docs (verbatim) |
| PVM2 announcement | **2026-05-28**, Polkadot Forum, toward standard RISC-V (RV64E + extensions) | PVM2 forum thread |
| Substrate runtime executor | **Wasmtime (compiled)** is the production default; not instruction-metered (uses weights) | Substrate sc-executor source + PR #3869 + benchmarking docs |
| FROST | **RFC 9591**, Informational, June 2024, IRTF/CFRG | rfc-editor.org / datatracker (cross-check for the gap-analysis spec-hygiene note; not load-bearing here) |

## Could NOT fully verify — flagged for reviewer

- **NEAR "300 TGas → 1 PGas in protocol version 2.11.0"** and the specific
  `wasm_touching_trie_node` repricing numbers: sourced from secondary
  documentation surfaced in search, **not** cross-checked against a primary NEP or
  the nearcore CHANGELOG. The *pattern* (frequent per-protocol-version repricing)
  is well established; treat the exact "2.11.0" mapping and the specific gas
  numbers as **indicative, not confirmed**. ([near.md](near.md) carries the same
  flag inline.)
- **"Free metering" for RISC-V/PVM**: the literal phrase and any quantified
  overhead comparison were **not** found in the primary JAM PVM doc (which states
  fixed-per-instruction gas + asynchronous metering). The "free metering" framing
  comes from secondary blog/forum summaries. Flagged inline in
  [polkavm-jam.md](polkavm-jam.md).
- **JAM mainnet status**: testnet launched Jan 2026; "mainnet targeted early 2026"
  is a *target* from secondary sources. As of 2026-05-29 this folder treats JAM as
  **not** a shipped mainnet metering reference. If a reviewer has primary
  confirmation either way, update [polkavm-jam.md](polkavm-jam.md).
- **Soroban per-protocol-version cost mechanics**: docs confirm cost params are
  "network configurable… updated through network consensus" but did **not** give a
  NEAR-style explicit per-version store mechanism or version numbers. Stated as
  consensus-updatable; the per-version internals are less documented than NEAR's.

## All URLs cited in this folder

- CosmWasm "Metering is hard" — https://medium.com/cosmwasm/metering-is-hard-cosmwasm-security-issues-explained-a797511cd54e
- CosmWasm "The incomplete gas patch" — https://medium.com/cosmwasm/the-incomplete-gas-patch-and-why-it-caused-consensus-failures-173547ef02de
- CosmWasm "Wasmer 1.0 integrated" — https://medium.com/cosmwasm/wasmer-1-0-integrated-into-cosmwasm-2fa87437458c
- NEAR gas architecture — https://near.github.io/nearcore/architecture/gas/index.html
- NEAR parameter estimator — https://near.github.io/nearcore/architecture/gas/estimator.html
- NEAR parameter definitions — https://near.github.io/nearcore/architecture/gas/parameter_definition.html
- NEAR gas (execution fees) — https://docs.near.org/protocol/transactions/gas
- NEAR Nomicon RuntimeConfig — https://nomicon.io/GenesisConfig/RuntimeConfig
- Soroban "Why no JIT?" — https://stellar.org/blog/developers/why-doesnt-soroban-use-a-jit
- Soroban fees/limits/metering — https://developers.stellar.org/docs/learn/fundamentals/fees-resource-limits-metering
- Soroban fee structure / scalability — https://stellar.org/blog/developers/sorobans-fee-structure-contributes-stellar-network-scalability
- Wasmi — https://github.com/wasmi-labs/wasmi
- PolkaVM repo — https://github.com/paritytech/polkavm
- JAM PVM docs — https://jam-docs.onrender.com/basics/pvm
- "Announcing PolkaVM" forum — https://forum.polkadot.network/t/announcing-polkavm-a-new-risc-v-based-vm-for-smart-contracts-and-possibly-more/3811
- "Announcing PVM2" forum — https://forum.polkadot.network/t/announcing-pvm2-reimagining-pvm-towards-standard-risc-v/17748
- PolkaVM authorship (`polkavm-common` Cargo.toml `authors`) — https://docs.rs/crate/polkavm-common/latest/source/Cargo.toml
- ink! "Why RISC-V and PolkaVM" — https://use.ink/docs/v6/background/why-riscv-and-polkavm-for-smart-contracts/
- Substrate sc-executor — https://github.com/paritytech/substrate/blob/master/client/executor/wasm_runtime.rs
- Substrate Wasmtime PR #3869 — https://github.com/paritytech/substrate/pull/3869/
- Substrate benchmarking — https://polkadot.study/tutorials/substrate-in-bits/docs/Benchmarking-substrate-pallet
- Polkadot PVF determinism — https://github.com/paritytech/polkadot/issues/1269
- Wasmtime deterministic execution — https://docs.wasmtime.dev/examples-deterministic-wasm-execution.html
- crates.io: `cosmwasm-vm`, `wasmer`
- RFC 9591 (FROST) — https://www.rfc-editor.org/rfc/rfc9591.html
