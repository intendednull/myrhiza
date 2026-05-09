# Architecture

```
         +------------------------------------------+
         |             CONDUCTOR (Rust)             |
         |  - keystore (lair)                       |
         |  - app management, admin & app websocket |
         |  - Kitsune2 networking + iroh transport  |
         |  +-----------+   +-----------+   ...     |
         |  |  CELL A   |   |  CELL B   |           |
         |  | DNA hash  |   | DNA hash  |           |
         |  | + agent   |   | + agent   |           |
         |  | source    |   | source    |           |
         |  | chain     |   | chain     |           |
         |  | wasmer VM |   | wasmer VM |           |
         |  +-----------+   +-----------+           |
         +------------------------------------------+
                           ^
                  websocket| (UI/JS client)
                           v
                     +------------+
                     | Tauri / web|
                     +------------+
```

A **conductor** is the long-running Rust process — it owns the keystore, hosts a wasmer instance per cell, exposes admin/app websocket APIs, and runs networking ([build/dnas](https://developer.holochain.org/build/dnas/)). A **cell** is a `(DNA hash, agent pubkey)` pair: the same DNA installed under two agents creates two cells. A **DNA** is the bundle of zomes that defines one peer-to-peer network; same DNA hash = same network ([build/dnas](https://developer.holochain.org/build/dnas/)). A **zome** is one WASM module exporting a set of functions.

## Integrity vs coordinator zomes

Since 0.1, zomes are split into two flavors ([Dev Pulse 121](https://blog.holochain.org/integrity-and-coordination-part-ways/), [build/zomes](https://developer.holochain.org/build/zomes/)):

- **Integrity zomes** define entry/link types and validation callbacks. They are hashed into the DNA hash, so any change forks the network. They use the smaller `hdi` crate (deterministic subset).
- **Coordinator zomes** hold all the imperative logic: zome calls, init callbacks, signal emitters, remote calls, scheduler hooks. They depend on the full `hdk`. Crucially, **coordinator zomes can be swapped at runtime without forking the network** — this is the upgrade story.

This split is the architectural cornerstone. Validation must be deterministic (every authority must reach the same verdict). Imperative app logic must be free to use clocks, RNG, network. Holochain enforces the separation at the zome boundary; Myrhiza expresses the same thing at the WIT-interface boundary via component profiles (`state-apply` vs `state-propose`/`interaction`/`behavior`).

## Source chain → DHT op flow

Every action a cell takes is appended to its **source chain**, a hash-linked, agent-signed local log. Public actions then produce **DHT operations** that get gossiped to a "neighborhood" of peers whose storage arc covers the action's hash ([concepts/4_dht](https://developer.holochain.org/concepts/4_dht/)).

```
   author commits action -> source chain -> produces DHT ops
                                            (StoreEntry, RegisterAgentActivity, etc.)
                                                       |
                                                       v
                                       neighborhood authorities
                                       (peers whose storage arc
                                        covers basis_hash)
                                                       |
                                                       v
                              run integrity validation callback
                                                       |
                              valid? -> store + gossip onward
                            invalid? -> publish warrant against author
```

A single commit fans out into multiple op types — `StoreEntry`, `StoreRecord`, `RegisterAgentActivity`, `RegisterUpdate`, `RegisterDelete`, link ops — each routed to a different basis hash and validated by a different authority set. This decomposition lets different parts of one logical action be authoritatively verified by different overlapping neighborhoods.

The networking layer is **Kitsune2** (data/gossip layer) over **iroh** (transport, default since 0.6.1) or **tx5** (WebRTC transport, the 0.5 default). See [`networking.md`](networking.md) for the gossip protocol and history.

## Sources

- [Build Guide — DNAs](https://developer.holochain.org/build/dnas/)
- [Build Guide — Zomes](https://developer.holochain.org/build/zomes/)
- [Concepts — DHT](https://developer.holochain.org/concepts/4_dht/)
- [Dev Pulse 121: Integrity and Coordination Part Ways](https://blog.holochain.org/integrity-and-coordination-part-ways/)
