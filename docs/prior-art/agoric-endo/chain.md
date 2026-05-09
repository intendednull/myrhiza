**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric — the chain layer (Cosmos SDK + Tendermint + SwingSet)

# Chain integration

Agoric is **a Cosmos SDK chain**. SwingSet — the JavaScript object-capability runtime that gets the most attention from the Endo / Agoric papers — only runs in production *inside* this chain. The chain's constraints (BFT consensus, deterministic block production, IBC, tokenomics) shape what SwingSet has been optimized for and what it has not.

For Myrhiza this matters because: the same SwingSet codebase that we sometimes lift snippets from was built for a setting in which **every kernel transition is replayed by ~150 validators**, not for a P2P swarm where peers may diverge. Read the SwingSet runtime spec with that frame on. See [contracts.md](./contracts.md) for the contract framework that runs *on top of* this chain, and [apps.md](./apps.md) for what's actually deployed.

## Stack

The chain is a fairly standard CometBFT + Cosmos SDK setup with one extra module — `x/swingset` — that hands off to the JavaScript kernel.

| Layer | Component | Notes |
|---|---|---|
| Consensus | CometBFT v0.38.17 (Agoric fork) | Tendermint-style BFT, instant finality |
| App framework | Cosmos SDK v0.50.14 (Agoric fork) | Go, `BeginBlock` / `EndBlock` lifecycle |
| Daemon | `agd` | Go binary, `agoric-upgrade-22b` is the current mainnet release (Oct 2025); `agoric-upgrade-23-rc1` cut 2026-05-06 |
| App-layer module | `x/swingset` | Forwards block transactions into the JS kernel |
| Smart-contract VM | SwingSet | Pure-JS kernel running vats; not WASM |
| Native asset | `ubld` (BLD, "build") | Staking + governance |
| IBC | v8.7.0 with ICS20-1, ICA | Cosmos-native interop |
| CosmWasm | **Disabled** | No Wasm contracts on Agoric — only SwingSet vats |

Source: [chain registry agoric/chain.json](https://raw.githubusercontent.com/cosmos/chain-registry/master/agoric/chain.json) (chain ID `agoric-3`).

The `cosmic-swingset` package in agoric-sdk is the bridge: a Go `x/swingset` module on the Cosmos side, paired with a Node.js process on the JS side, communicating over an internal protocol. Cosmos SDK calls `BeginBlock`, `DeliverTx`, `EndBlock` as usual; on the SwingSet side these turn into messages enqueued for the kernel to crank. ([cosmic-swingset README](https://github.com/Agoric/agoric-sdk/blob/master/packages/cosmic-swingset/README.md))

## Mainnet phases

Agoric has been deliberate about *not* turning everything on at once. The phases are real and worth knowing:

- **mainnet-0** — launched **November 1, 2021**. BLD goes live as a Cosmos-SDK staking token. **No smart contracts**. Validators stake; governance exists; the chain produces blocks. ([Mainnet Phase 0 launch announcement](https://agoric.com/blog/announcements/mainnet-phase-0-launch/))
- **mainnet-1** — launched **October 27, 2022**. SwingSet kernel turned on; first JS smart contract (the Inter Protocol PSM) deployed via governance core-eval. ([Mainnet-1 milestone post](https://agoric.com/blog/announcements/agoric-composable-smart-contract-framework-reaches-mainnet-1-milestone/))
- **mainnet-2** — incremental rollout of more Inter Protocol features (vaults, auctions). No discrete launch event; phases blurred together.
- **mainnet-3** — *intended* phase that would open permissionless contract deployment. As of the docs reviewed (May 2026), this phase has not arrived; contracts are still installed via governance only. See [coreeval docs](https://docs.agoric.com/guides/coreeval/).

Practical consequence: every contract on Agoric mainnet was installed by **passing a governance proposal**. There is no `eth_sendTransaction` for "deploy a new contract." The friction is intentional — and it explains a lot about [why deployments are sparse](./apps.md).

## Block production with SwingSet

A Tendermint block on Agoric has the usual Cosmos lifecycle plus a SwingSet "run policy":

1. `BeginBlock` — Cosmos SDK modules run their begin-block hooks (`x/staking`, `x/distribution`, etc.).
2. `DeliverTx` — Each transaction in the block is dispatched to its module. Transactions targeting `x/swingset` (e.g., `MsgInstallBundle`, `MsgWalletAction`, `MsgCoreEval`) get queued as kernel inputs.
3. `EndBlock` — `x/swingset` runs the SwingSet kernel for some number of "cranks." A crank is one delivery to one vat. The kernel keeps cranking until it either runs out of work or hits a per-block compute limit.
4. App hash is computed over both Cosmos state and a deterministic digest of SwingSet state. Validators must agree on both.

The compute limit is set by the on-chain parameter `max_computrons_per_block` in `x/swingset`. Computrons are a synthetic unit Agoric uses to meter JavaScript execution cost. If the kernel can't drain its run-queue inside that budget, leftover work spills to the next block. This is what "SwingSet ticks per block" actually means — there is no fixed number; it's a budget. ([cosmic-swingset issue #3752 on swingset metering+fee parameters](https://github.com/Agoric/agoric-sdk/issues/3752))

**Determinism requirement.** The kernel must be a pure function of its prior state and the ordered transactions in the block. Validators replay the same JS code and must reach the same hash. This is a hard constraint that ripples through every design decision — including why Hardened JS exists (see [contracts.md](./contracts.md)).

## vstorage

`vstorage` is the chain's key-value store, write-only from the JS side and read-only from the outside. It exists because Cosmos chains traditionally expose state via custom RPC endpoints, and Agoric did not want every contract to need a Go-side adapter.

- A vat receives a `chainStorage` capability. It writes to a path (e.g., `published.vaultFactory.governance`).
- Off-chain consumers query via `agd query vstorage data <path>` or via gRPC. They can also subscribe to the `published.*` subtree through the `@agoric/casting` library, which handles the marshalling protocol.
- vstorage values are JSON strings produced by `@endo/marshal`. Consumers that want to reconstruct objects (e.g., turn a board ID back into a brand reference) need the matching marshal logic.

This is essentially an event-log / state-publication pattern, not a query interface. Contracts publish; clients tail. ([vstorage docs](https://docs.agoric.com/guides/getting-started/contract-rpc.html))

## IBC

Agoric ships IBC v8.7.0 with ICS20-1 (fungible token transfer) and Interchain Accounts. Two specifics worth flagging:

- **Dynamic IBC ("dIBC").** Smart contracts inside SwingSet can open new IBC ports and channels at runtime. On most Cosmos chains, channels are bound to fixed module names baked into the chain binary; on Agoric, a vat asks the kernel for a port and gets a JS object capability. Used by Inter Protocol's PSM and by the orchestration vat.
- **Orchestration.** `@agoric/orchestration` (latest 0.2.0, July 2025) is a higher-level framework letting a contract say "send X to chain Y via ICA, then on completion run callback Z." It papers over the packet-acknowledgment dance.

For Myrhiza this is not directly relevant — we are not a Cosmos chain — but it's the analog of a peer reaching across to another peer, and the design pattern of "open a port, get a capability, drive the protocol from JS" is one we may want to mirror for our P2P transports.

## Validator topology

BLD is the staking token. The active validator set is governed by the standard Cosmos `x/staking` module parameters; Agoric uses the typical 100–150 validator cap.

Honest read: BLD market cap in May 2026 is **~$2.8M** (CoinGecko: ~$0.004/BLD, ~696M circulating). That is below the level at which a Cosmos chain can claim meaningful security from market value alone — economic security is a fraction of that, given partial bond ratios. The chain is functionally secured by the validator set's reputational / operational alignment more than by stake-slashing economics.

This does not mean the chain is unsafe — Cosmos chains far smaller than this run for years without consensus failures — but it does mean Agoric is not a high-value-attack target by 2026 standards. We should read SwingSet's design choices in that light: the chain has not been stress-tested by attackers in the way Ethereum has.

## The `agoric` CLI

Two distinct binaries, often confused:

- **`agd`** (Go) — the chain daemon. Validators run it; users use it for `agd tx ...` and `agd query ...`. Standard Cosmos-SDK fare.
- **`agoric`** (Node.js, npm package `agoric` — latest 0.22.0 published April 2026) — the **developer CLI**. Wraps `agd` with conveniences for contract development: `agoric init`, `agoric start` (local chain), `agoric run` (compile and submit a deploy script), `agoric follow` (tail vstorage with marshal decoding), `agoric wallet`.

A typical contract deployment to mainnet (paraphrased from the [getting-started docs](https://docs.agoric.com/guides/getting-started/)):

```
1. agoric run deploy-script.js          # bundles contract source, produces a hash
2. agd tx bank send ...                 # collect IST + BLD for fees + governance deposit
3. agd tx swingset install-bundle ...   # uploads the bundle to chain storage
4. agd tx gov submit-proposal swingset-core-eval ...  # proposal that core-evals the install
5. wait for the vote, then the proposal executes
```

`agoric deploy` (the older command) is being deprecated in favour of `agoric run` against a Cosmos node. The friction is real — the docs explicitly note that "permissionless contract installation with Zoe is limited to development environments" until mainnet-3.

## Implications for Myrhiza

1. **SwingSet's design is BFT-consensus-shaped.** Determinism is non-negotiable for them because every validator replays. We need the same property for state-apply, but for a different reason (cross-peer convergence). The constraint is the same; the threat model is different. Useful: their answer to *how* you make a JS-style runtime deterministic — Hardened JS — is directly portable in spirit even though our VM is WASM.
2. **vstorage is a publish-only state surface.** That pattern (vat writes to a path, off-chain tails it) maps cleanly onto our event-log story. No query language, no transactions — just append + subscribe. Worth borrowing as a UX pattern for app-to-host state surfacing.
3. **Don't copy the deployment friction.** Agoric's "every contract is a governance proposal" model is a chain-of-trust workaround for a permissionless chain that has not finished its rollout. We are not a chain. App installation in Myrhiza should be peer-local, not network-consensus-dependent. Where Agoric needs governance to authorize a new contract, we just need the device owner to authorize a new app.
4. **Block-time semantics inform metering, not architecture.** The computron meter is interesting as evidence that JS execution can be deterministically metered at all. We want the same property for our WASM components (fuel-based metering is well-trodden in wasmtime). Their lesson: don't try to meter wall-clock; meter abstract instructions.
5. **IBC is the wrong mental model for our transports.** It works for Agoric because all the chains are BFT. P2P transport correctness in our setting comes from CRDT / event-log convergence, not packet acks.

## Sources

- [Agoric Mainnet Phase 0 launch (Nov 1, 2021)](https://agoric.com/blog/announcements/mainnet-phase-0-launch/)
- [Agoric Mainnet-1 milestone announcement (Oct 27, 2022)](https://agoric.com/blog/announcements/agoric-composable-smart-contract-framework-reaches-mainnet-1-milestone/)
- [Cosmos chain-registry: agoric/chain.json](https://raw.githubusercontent.com/cosmos/chain-registry/master/agoric/chain.json)
- [agoric-sdk cosmic-swingset README](https://github.com/Agoric/agoric-sdk/blob/master/packages/cosmic-swingset/README.md)
- [agoric-sdk telemetry README](https://github.com/Agoric/agoric-sdk/blob/master/packages/cosmic-swingset/README-telemetry.md)
- [Cosmos governance for swingset metering+fee params (issue #3752)](https://github.com/Agoric/agoric-sdk/issues/3752)
- [Agoric Platform overview](https://docs.agoric.com/guides/platform/)
- [vstorage / contract RPC docs](https://docs.agoric.com/guides/getting-started/contract-rpc.html)
- [Core-eval docs (governance-mediated contract install)](https://docs.agoric.com/guides/coreeval/)
- [agoric-sdk releases (verified via `gh api repos/Agoric/agoric-sdk/releases`)](https://github.com/Agoric/agoric-sdk/releases)
- [BLD market data, CoinGecko](https://www.coingecko.com/en/coins/agoric)
