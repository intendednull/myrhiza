**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — glossary of stack-specific terms

# Glossary

System-specific terms used across this folder. Cross-references to the file where the term is treated in depth.

## Project + organization

- **Holepunch (Inc)** — the company behind the stack. Mexico legal entity `Tether Data S.A. de C.V.`. Founded ~2021; public launch 2022-07-25 by Mathias Buus + Paolo Ardoino + Andrew Osheroff. Tether-funded. See [commercial.md](commercial.md), [governance.md](governance.md).
- **`holepunchto`** — the GitHub organization. Created 2021-03-25; 617 public repos as of 2026-05-09. See [governance.md](governance.md).
- **Dat Project** — the 2013-founded predecessor. Hypercore + Hyperdrive originated under Dat (Max Ogden, Karissa McKelvey, Mathias Buus). Holepunch absorbed the codebase ~2020–2021. See [history.md](history.md).
- **Tether** — the stablecoin issuer. Primary funder of Holepunch (~$10M committed initial + up to $50–100M follow-on per third-party crypto press; primary-source figure not verified). Paolo Ardoino is CEO of both Tether and Holepunch CSO. See [commercial.md](commercial.md).

## Runtime layer

- **Pear runtime** — the application runtime. Distributes apps via Hyperdrive, runs them on Bare. Apache-2.0; 241 stars; created 2024-02-03. Installed via `npx pear` (NOT `pear-cli` — that's stale at 2.5.9 from 2022). See [pear-runtime.md](pear-runtime.md).
- **Bare** — Holepunch's embeddable JavaScript runtime. Built on libjs + libuv. Apache-2.0; 1,072 stars; npm `bare@1.28.5`. Tier 1 mobile platforms. C embedding API: `bare_setup`, `bare_load`, `bare_run`, `bare_teardown`, `bare_suspend`, `bare_resume`. See [bare-runtime.md](bare-runtime.md).
- **`pear://` link** — the URL scheme for addressing Pear apps. Format `pear://<base32-encoded-key>`. The key is a Hyperdrive public key; the link both identifies and authorizes installation. See [pear-runtime.md](pear-runtime.md).
- **Production bootstrap key** — `pear://gd4n8itmfs6x7tzioj6jtxexiu4x4ijiu3grxdjwkbtkczw5dwho` — the well-known Pear runtime key for production bootstrap. See [pear-runtime.md](pear-runtime.md).
- **`pear stage`** / **`pear seed`** / **`pear release`** — CLI workflow for publishing a Pear app: stage to a local Hyperdrive, seed it onto the network, release the link. See [pear-runtime.md](pear-runtime.md), [apps.md](apps.md).

## Data layer (Hypercore stack)

- **Hypercore** — append-only signed log. Each block hash-chained back through a Merkle tree. Author identified by ed25519 keypair (the Hypercore "key"). Sparse-replicated. **MIT-licensed.** Current `11.30.1` (2026-05-06). See [hypercore-stack.md](hypercore-stack.md).
- **Hypercore Protocol v11** — the current major. Introduced **RocksDB-backed storage** (replacing the v10-era `.metadata` / `.tree` / `.bitfield` / `.data` file layout). Migrating from v10 → v11 requires re-encoding. See [hypercore-stack.md](hypercore-stack.md), [history.md](history.md).
- **`corestore`** — the package that manages multiple Hypercores in a single directory. `7.9.2`. See [hypercore-stack.md](hypercore-stack.md).
- **Hyperdrive** — filesystem semantics on Hypercore. v13 layout: a metadata-Hyperbee on one Hypercore plus a Hyperblobs-on-Hypercore for content. (v12-and-earlier used a different two-Hypercore layout.) **Apache-2.0** (Holepunch-era relicense; not MIT despite Dat-era origin); `13.3.2` (2026-03-27). See [hypercore-stack.md](hypercore-stack.md).
- **Hyperblobs** — content-blob storage layered on Hypercore. Used by Hyperdrive v13. See [hypercore-stack.md](hypercore-stack.md).
- **Hyperbee** — sorted key-value B-tree on Hypercore. Get / put / range query. Each insert appends a tree-node block. `2.27.3`. See [hypercore-stack.md](hypercore-stack.md).
- **Autobase** — multi-writer linearization on Hypercore. Multiple writers' Hypercores get merged into a single deterministic view. README explicitly avoids "CRDT" terminology — calls it "multiwriter data structure + event sourcing pattern." **Apache-2.0**. `7.28.0` (2026-05-05). See [hypercore-stack.md](hypercore-stack.md), [data-model.md](data-model.md).
- **View** — in Autobase, the application-defined materialization of the linearized writer-log into a Hyperbee or Hyperdrive. Re-derived on each new merge. The view is where conflict resolution actually happens. See [data-model.md](data-model.md).
- **Encrypted core** — a Hypercore where the public key alone doesn't decrypt block content. Additional symmetric `encryptionKey` required. Used by Keet for E2E. (Note: in v11 the `encryptionKey` parameter was deprecated; check current encryption story.) See [hypercore-stack.md](hypercore-stack.md), [keet-and-apps.md](keet-and-apps.md).
- **Tombstone** — a metadata-Hypercore append that marks a Hyperdrive entry deleted. Append-only logs do not support physical deletion; tombstones are the convention. See [data-model.md](data-model.md).

## Network layer

- **Hyperswarm** — peer discovery + connection layer. DHT topic discovery + UDP holepunching. **MIT-licensed** (NOT Apache-2.0). `4.17.0` (2026-02-20); 1,261 stars. See [hyperswarm.md](hyperswarm.md), [transport-comparison.md](transport-comparison.md).
- **HyperDHT** — the DHT layer beneath Hyperswarm. Kademlia-flavored. MIT; `6.32.0` (2026-05-05); 391 stars. See [hyperswarm.md](hyperswarm.md).
- **Bootstrap nodes** — three hardcoded entries in `hyperdht/lib/constants.js`: `node1.hyperdht.org`, `node2.hyperdht.org`, `node3.hyperdht.org`, all on port `49737`. Holepunch-operated. See [hyperswarm.md](hyperswarm.md), [critiques.md](critiques.md).
- **Topic** — a 32-byte hash. Peers join a topic to find each other; topic membership is gossip-discovered. Application-defined topic content. See [hyperswarm.md](hyperswarm.md).
- **Noise-IK** — the Noise Protocol Framework variant Hyperswarm uses for the DHT-layer handshake (NOT Noise-XX as some older docs/marketing say). Authenticates initiator's static key against responder's. See [hyperswarm.md](hyperswarm.md).
- **UDX** — Holepunch's UDP-based transport, used by Hyperswarm/HyperDHT. Reliable bidirectional channel over UDP. No TCP fallback in current `hyperdht`. See [hyperswarm.md](hyperswarm.md).
- **`protomux`** — the multiplexer running over a UDX or TCP connection. Allows multiple application-level protocols to share one underlying connection. `3.11.0`. See [hyperswarm.md](hyperswarm.md).
- **`@hyperswarm/secret-stream`** — encrypted stream layer, `6.9.1`. See [hyperswarm.md](hyperswarm.md).
- **Holepunching (the protocol)** — STUN-flavored phase-1 (each peer learns its NAT-mapped public address from a third party in the DHT) followed by phase-2 simultaneous UDP-send to punch through. Hyperswarm uses `BIRTHDAY_SOCKETS=256`, `HOLEPUNCH_TTL=5`, `randomPunchInterval=20000`, `_minSamples=4` as tunables. See [hyperswarm.md](hyperswarm.md).
- **`blind-relay`** — TURN-equivalent dependency of `hyperdht`. Opt-in via `relayThrough`; **no default Holepunch-operated fleet**. Fundamentally different from Iroh's always-available DERP fleet. See [hyperswarm.md](hyperswarm.md), [transport-comparison.md](transport-comparison.md).
- **`dht-rpc`** — lower-level RPC primitive over the DHT, `6.27.0`. Used by HyperDHT internals. See [hyperswarm.md](hyperswarm.md).

## Apps

- **Keet** — Holepunch's flagship messenger app. **Closed-source**; iOS bundle id `io.keet.app`; v4.14.0 (2026-04-29). React Native + Expo + Bare-embedded. Voice/video via WebRTC + Hyperswarm signaling (inferred from CallKit + new-call-engine changelog evidence). 24-word seed identity. ~99 iOS ratings (4.59 stars), ~1K Android ratings, ~690K Android lifetime downloads. See [keet-and-apps.md](keet-and-apps.md).
- **Room key** — Keet's access control unit. Joining a room = exchanging the room key. The room key is also the symmetric key for the encrypted Hypercores backing the room. See [keet-and-apps.md](keet-and-apps.md).
- **24-word seed** — Keet's user identity. Locally generated; the user copies/restores it across devices. No server-side recovery. See [keet-and-apps.md](keet-and-apps.md).
- **PearPass** — second showcase Pear app. Built by **Tether Data** (a Tether-affiliated entity, separate from Holepunch). App Store id `6752954830`. See [apps.md](apps.md), [commercial.md](commercial.md).
- **Hyperbeam** — terminal-sharing tool. 539 stars under `holepunchto`. See [apps.md](apps.md).
- **Hypershell / Hyperssh / Hypertele / Drives** — other Holepunch tools shown in the Pears showcase. See [apps.md](apps.md).

## Cross-substrate (for comparison with neighbor folders)

- **DERP** (Iroh) — Iroh's always-available HTTP relay fleet for NAT-traversal fallback. Hyperswarm has no equivalent. Both have *some* infrastructure (DHT bootstrap for Hyperswarm, DERP for Iroh) but differ on whether servers run in the data path (Iroh) or only the control path (Hyperswarm). See [transport-comparison.md](transport-comparison.md), [`../iroh/transports.md`](../iroh/transports.md).
- **Source chain** (Holochain) — Holochain's per-agent append-only signed log. Direct analog of Hypercore. Both append-only, signed, content-addressed; differ in that Holochain re-validates every entry through a deterministic WASM zome and Hypercore does not. See [`../holochain/`](../holochain/).
- **Vat** (Agoric) — single-threaded, sandboxed compute unit. Closer to a Pear-app process than to a Hypercore. Vat snapshots have no Hypercore analog; Hypercore replays from the log instead. See [`../agoric-endo/vat-model.md`](../agoric-endo/vat-model.md).
- **Component** (WASM CM) — typed-import WASM unit. Pear apps are JS, not components. The `pear://` link distribution model is conceptually similar to OCI-as-WASM-registry; the addressability is the same shape (hash of the artifact). See [`../wasm-component-model/`](../wasm-component-model/).

## Sources

- Pears docs: https://docs.pears.com/
- Hypercore protocol upgrades: https://github.com/holepunchto/hypercore/blob/main/UPGRADE.md
- HyperDHT bootstrap nodes: https://github.com/holepunchto/hyperdht/blob/main/lib/constants.js
- Bare embedding API: https://github.com/holepunchto/bare/blob/main/include/bare.h
