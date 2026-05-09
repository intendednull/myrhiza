# Glossary

Agoric / Endo / SwingSet-specific terms and the Agoric-flavored variants of generic ocap concepts. Generic distributed-systems terms (CRDT, gossip, NAT, QUIC) are deferred to other glossaries. Generic ocap terms (capability, attenuation, sturdyref) cross-reference [`../spritely-ocapn/glossary.md`](../spritely-ocapn/glossary.md) where they're more thoroughly covered.

## Endo / Hardened JS

- **SES** (Secure ECMAScript) — a hardened-JavaScript shim that runs `lockdown()` to freeze ECMAScript primordials and remove sources of ambient authority. Standalone npm package `ses`. Currently `2.0.0` (2026-04-17). TC39 Stage 1 as `proposal-ses`, with active follow-on work in `proposal-compartments`.
- **`lockdown()`** — the SES initialization call that freezes `Object`, `Array`, `Function`, `Promise`, etc., taming them to be safely shared across mutually-suspicious code. Idempotent; once called per realm, primordials are immutable. Configurable via *taming options* (e.g. `errorTaming`, `legacyRegeneratorRuntimeTaming`).
- **Compartment** — an SES isolation unit sharing intrinsics with the outer realm but with its own globals, module loader, and code surface. Cheaper than a `Realm`. The unit of mutual-suspicion code-loading.
- **`harden()`** — recursive transitive-freeze on an object graph. Stops at "boundary" objects (other compartments, presence refs). The `Far()` and `Exo()` constructors call `harden()` automatically.
- **`Far(interfaceName, methods)`** — the convention for marking an object as a remote-callable presence. Sets a brand and freezes; the marshal layer recognizes Far objects when serializing across vat boundaries.
- **`Exo`** — newer, structurally-typed analog of `Far` with built-in interface-shape validation. Stands for "exotic object" in the JS-host-object sense.
- **Pass-style** — Endo's classification of values for marshal: `data` (passed by value, deep-cloned across boundaries), `presence` (live ref to an object in another vat), `promise` (a future value). Marshal's serialization is driven by pass-style.
- **`@endo/marshal`** — the serialization layer that encodes JS values into a JSON-extensible format (CapTP slots), with capability slots replaced by indexes into a `slots[]` array.
- **smallcaps** — the newer, more-compact marshal encoding. Distinct from the original "ocaps" encoding. Both still in production for backwards compat.
- **`@endo/captp`** — Endo's CapTP implementation. The over-the-wire ocap protocol. Currently `4.5.0` (2026-02-26).
- **`@endo/eventual-send` / `E()`** — eventual-send: `E(remoteRef).method(args)` returns a promise for the result. Pipelining: `E(E(x).y()).z()` does not require a network round trip per `E()` — the runtime forwards.
- **`HandledPromise`** — the underlying machinery for `E()`. A promise that is handled (eventual-send-aware) versus a vanilla `Promise` that resolves locally.
- **`@endo/compartment-mapper`** — the bundler-input layer. Reads a `package.json` graph and produces a compartment-per-package map (which package can `import` what).
- **`@endo/bundle-source`** — the bundler. Produces an `endoZipBase64` bundle with hash `b1-<sha512(compartment-map)>` (the hash is over the *manifest*, not the raw bytes — robust to whitespace/encoding drift).
- **bundlecap / bundle-installation** — the SwingSet abstraction over a bundle. A *bundlecap* is an authority-handle to a bundle; the bundle bytes are stored elsewhere and addressed by hash.
- **`harden`-target / `Far`-target / `Exo`-target** — three styles of the same underlying "object marked for capability use" pattern.
- **Stabilize / Non-trapping** — TC39 proposals (Stage 1 as of 2026-05-09) that would let SES stop runaway code on memory-allocation paths via `[[NonTrappingProxyHandler]]` and `Stabilize()`. Both pre-shipping.

## SwingSet kernel

- **Vat** — a single-threaded JS event loop with persistent identity. Roughly: a Unix process, with the kernel as the OS. Vats run on top of `xsnap`, an XS worker process. Vats can be *static* (built into the kernel bundle, started at chain genesis) or *dynamic* (spawned at runtime via `vatAdmin.createVat`).
- **`buildRootObject`** — the entry point a vat author writes. Returns the `rootObject`, the cap-bearing object exposed to other vats.
- **`xsnap`** — the worker process wrapping XS. SwingSet spawns one xsnap per vat. Communication with the kernel is over a length-prefixed pipe; deliveries and syscalls are JSON messages. `xsnap 0.15.0` is the current npm release.
- **XS** — the Moddable JavaScript engine SwingSet uses. No JIT; small, deterministic, snapshot-friendly. Distinct from V8 / SpiderMonkey / JSC. Designed originally for IoT.
- **Kernel** — the SwingSet kernel proper. Orchestrates vats, manages the c-list, schedules deliveries, applies syscalls, persists state. Lives in `@agoric/swingset-vat`.
- **Device** — a kernel-side component that mediates I/O between vats and the host. Examples: timer device, mailbox device (sends bytes to other machines), bridge device (Cosmos chain bridge), bundle device. Devices are how vats reach anything not other vats.
- **c-list** (capability list) — the kernel-maintained per-vat translation table from vrefs (vat-side refs) to krefs (kernel-side refs). Every cap a vat holds appears in its c-list.
- **kref / vref / oref** — kref = kernel-side reference (e.g. `ko42`); vref = vat-side reference (e.g. `o+12`); oref = object reference more generally. The kernel translates between them at the c-list boundary.
- **Run-queue** — the kernel's pending-deliveries list. Deliveries are serialized — the kernel processes one at a time per cycle.
- **Delivery** — a message from the kernel into a vat. Three types: `message` (a method call on a vat-side object), `notify` (a promise resolution), `bringOutYourDead` (GC sync).
- **Syscall** — a call from a vat to the kernel. Examples: `send` (call a method on a remote object), `subscribe` (await a promise), `resolve` (resolve a promise), `vatstoreGet/Set` (persistent kv access), `exit` (terminate the vat).
- **Crank** — one delivery + the syscalls it triggers, executed atomically. The unit of metering.
- **Computron** — the unit of CPU-cost in SwingSet metering. `DEFAULT_CRANK_METERING_LIMIT = 1e8` computrons per crank. A computron is roughly one bytecode op; the costs are tuned per-op so cross-validator agreement is exact.
- **Meter** — a budget object. A vat is associated with a Meter; each crank deducts computrons; running out terminates the vat.
- **Transcript** — the append-only log of every delivery into a vat. Together with the latest snapshot, the transcript fully determines vat state via replay.
- **Snapshot** — an XS heap snapshot of a vat. Periodic. Hashed for integrity but **not part of consensus** (per `agoric-sdk#5227`); replay across validators uses the *transcript*, not the snapshot bytes.
- **Span** — a slice of the transcript between two snapshots. Replay loads the latest snapshot, then re-applies the span on top.
- **Incarnation** — a generation of a vat between upgrades. Vat upgrade increments the incarnation; baggage carries forward, transcript begins fresh.
- **`bringOutYourDead`** — periodic kernel-driven GC ceremony. Kernel asks each vat "what can you drop?", vat reports liveness, kernel reconciles distributed GC.
- **Baggage** — the persistent collection (a `MapStore`) preserved across vat upgrades. Convention: contract authors stash everything they want to survive upgrade in the root vat's baggage.
- **`@agoric/swing-store`** — the persistence backend. SQLite-based (since `0.9.0`, 2023-05-19); previously LMDB. One DB per chain node.
- **swingset-runner** — Agoric's testing/profiling harness for running SwingSet kernels outside the chain.

## Capabilities and CapTP

- **CapTP** (Capability Transport Protocol) — the over-the-wire ocap protocol Endo implements. Originated in E (1997). Co-designed today via OCapN with Spritely / MetaMask / Cap'n Proto. See [`captp-and-network.md`](./captp-and-network.md) and [`../spritely-ocapn/captp-and-ocapn.md`](../spritely-ocapn/captp-and-ocapn.md).
- **OCapN** (Object Capabilities Network) — the cross-implementation CapTP-shaped network protocol. Pre-1.0. Spritely Goblins ships the reference impl; Endo ships `@endo/ocapn` (1.0.0 on npm but marked experimental); Agoric the chain has not committed to deploying it yet.
- **slots[]** — the marshal encoding's array of capability references. Wire format embeds `{"@qclass":"slot","index":N}` and the receiver looks up the cap in a parallel slots array.
- **Forward** — a CapTP resolution that promises were resolved to a remote presence (vs Reject or Resolve to data). Pipelining target.
- **Drop / Retire** — CapTP messages signaling that a capability is no longer reachable (drop) or no longer recognizable (retire). Distributed GC primitives.
- **Reachable vs recognizable** — Endo's distinction in distributed GC: reachable = "can use" (live), recognizable = "can compare for equality" (weak). Different lifecycles.
- **Comms vat** — a special vat in each SwingSet machine that brokers all off-machine traffic. Other vats `E()` remote refs through the comms vat without knowing where they live.
- **Mailbox device** — a kernel device that owns the byte-level wire to other machines. The comms vat speaks CapTP; the mailbox device gets the bytes onto IBC/HTTP/etc.

## Agoric chain

- **`agd`** — the Cosmos SDK daemon binary. Runs CometBFT, Cosmos modules, and the SwingSet bridge.
- **agoric-cli (`agoric`)** — the developer-facing CLI for deploying contracts.
- **cosmic-swingset** — the bridge module wiring the Cosmos SDK app block lifecycle to SwingSet kernel cranks.
- **vstorage** — the on-chain key-value store SwingSet vats can read from and (via the chain bridge) publish to. How off-chain consumers query chain state.
- **agoric-upgrade-N** — the chain's release tag scheme. Each `agoric-upgrade-N` is a coordinated chain upgrade requiring 2/3+ validator participation. `agoric-upgrade-22b` is current on mainnet (Oct 2025); `agoric-upgrade-23-rc1` published 2026-05-06.
- **CometBFT** — the BFT consensus layer (formerly Tendermint). Currently v0.38.17 in agoric-sdk.
- **IBC** — Inter-Blockchain Communication protocol. Cosmos's standard cross-chain messaging.
- **dIBC** — "dynamic IBC" — Agoric's vat-side wrapper exposing IBC connections as live ocap refs.
- **ICA** — Interchain Accounts. A Cosmos primitive for one chain to control an account on another chain.
- **CCTP** — Circle's Cross-Chain Transfer Protocol. Used by Fast USDC.

## Contracts and tokens

- **Zoe** — Agoric's smart-contract framework. A vat that hosts contract instances. Enforces *offer safety* (proposer either gets what they wanted or their deposit back) and *payout liveness* (any settlement eventually pays out).
- **ERTP** (Electronic Rights Transfer Protocol) — Agoric's primitive for digital assets. Issuers, brands, mints, purses, payments. Pre-dates Zoe and is more general-purpose.
- **Issuer / Brand / Mint / Purse / Payment** — the ERTP type taxonomy. Brand = identity of an asset class; Issuer = authority over the brand; Mint = permission to create units; Purse = persistent holding; Payment = transferable bearer instrument.
- **BLD** — Agoric's validator/governance token. ~1B initial supply. ATH $0.7512 (Oct 2021); ~99.4% below ATH as of May 2026.
- **IST** — Agoric's collateral-backed stablecoin (sunset 2025-06-30). Was minted via the PSM (Parity Stability Module) against USDC/USDT, and via vaults against ATOM/stATOM/etc.
- **Inter Protocol** — the umbrella application that issued IST. TVL at sunset ~$103K. Sunsetted by DCF + Agoric Engineering Council; on-chain vote 2025-04-28 → 2025-05-01; final wind-down 2025-06-30.
- **Fast USDC** — Agoric's flagship 2024–2025 application. Sub-minute USDC bridging from Ethereum + L2s to Cosmos via Circle CCTP and Agoric Orchestration. The post-Inter-Protocol identity.
- **DCF** — Decentralized Capital Fund. Operates governance proposals on Agoric chain.
- **Agoric Engineering Council** — internal Agoric governance body for technical proposals.

## Project / governance

- **Agoric Systems Operating Company** — the Delaware C-corp incorporated 2018-03-16. The legal entity behind Agoric the chain and Endo the monorepo.
- **Agoric Foundation** — does *not* exist as a separately-incorporated entity (verified, 2026-05-09). Governance flows through the operating company plus DCF plus on-chain BLD votes.
- **Number 0** vs. Agoric — distinct entities. Number 0 is iroh's steward; Agoric is Agoric's. Sometimes confused because both are pre-token-cycle small companies in the P2P space.
- **Agorics, Inc.** — Tribble's *earlier* company (acquired by Microsoft, 1990s). Distinct from modern Agoric. Search engines occasionally surface "Microsoft acquired Agoric"; this conflation is wrong. Modern Agoric Systems Operating Company has not been acquired.

## Sources

- [`@agoric/swingset-vat` on npm](https://www.npmjs.com/package/@agoric/swingset-vat)
- [`@agoric/swing-store` on npm](https://www.npmjs.com/package/@agoric/swing-store)
- [`ses` on npm](https://www.npmjs.com/package/ses)
- [`@endo/marshal`, `@endo/captp`, `@endo/bundle-source`, `@endo/compartment-mapper` on npm](https://www.npmjs.com/~agoric)
- [agoric-sdk SwingSet README](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/README.md)
- [Endo monorepo](https://github.com/endojs/endo)
- [Agoric chain registry entry](https://raw.githubusercontent.com/cosmos/chain-registry/master/agoric/chain.json)
- [TC39 proposal-ses](https://github.com/tc39/proposal-ses)
- [TC39 proposal-compartments](https://github.com/tc39/proposal-compartments)
- [Inter Protocol sunset proposal](https://community.agoric.com/t/sunset-inter-protocol-and-begin-wind-down-process/787)
- [Spritely glossary (sibling ocap terms)](../spritely-ocapn/glossary.md)
