# Glossary

Holochain-specific terms used throughout this prior-art doc. Generic P2P / WASM / crypto terms not listed.

- **Cell** — a running `(DNA hash, agent pubkey)` instance inside a conductor. The active unit of execution.
- **Capability grant** — a `ZomeCallCapGrant` entry on the agent's source chain authorizing some caller (unrestricted/transferable/assigned) to invoke specific zome functions.
- **Conductor** — the Rust process that hosts cells, manages the keystore (`lair`), runs networking (Kitsune2 + iroh/tx5), and exposes admin/app websocket APIs.
- **Coordinator zome** — a zome that holds imperative app logic, can be swapped without changing the DNA hash and thus without forking the network.
- **Countersigning** — framework-supported protocol for two or more agents to atomically commit the same entry to all their source chains within a time window.
- **DHT** — the validating distributed hash table that stores public actions. "Validating" because every op runs through validation callbacks before being stored or gossiped onward.
- **DNA** — the immutable bundle of integrity zomes (and a coordinator zome list) that defines one peer-to-peer network. Identified by hash. Same hash = same network.
- **DPKI / DeepKey** — the deprecated cross-app identity system; removed in 0.6.
- **Gossip neighborhood** — the set of peers whose declared storage arcs overlap with a given DHT address. Gossip happens within and between overlapping neighborhoods.
- **hApp** — Holochain application. A bundle of one or more DNAs plus a manifest, optionally with bundled UI.
- **HDK** — Holochain Development Kit. The Rust crate guest WASM imports to talk to the host. Split into `hdi` (deterministic, integrity-only) and `hdk` (full, coordinator-only).
- **HDI** — Holochain Deterministic Integrity. The deterministic subset of HDK; what integrity zomes use.
- **Integrity zome** — a zome whose code is part of the DNA hash; defines entry/link types and validation. Cannot be hot-swapped.
- **Kitsune2** — Holochain's current gossip / DHT layer (replaces kitsune1). Not wire-compatible with kitsune1.
- **`lair`** — Holochain's keystore daemon.
- **Membrane proof** — an app-defined credential checked at cell installation; the app's gate for who can join its DHT.
- **Source chain** — an agent's personal hash-linked, signed, append-only log of every action they've authored in a given cell.
- **Storage arc** — a peer's declared range of DHT addresses for which it commits to store and validate every op. Full arc = "I store everything"; partial arcs are sharding.
- **tx5** — Holochain's WebRTC transport layer (default through 0.5, replaced by iroh as default in 0.6.1).
- **Validation callback** — a pure deterministic function in an integrity zome that takes a DHT op and returns Valid / Invalid / UnresolvedDependencies.
- **Warrant** — a signed cryptographic proof that an agent authored an invalid op. Gossiped network-wide; recipients block the warranted author.
- **Zome** — one WebAssembly module exporting a defined set of functions. "Integrity zome" defines data + validation; "coordinator zome" defines imperative logic.

## Source

- [Holochain official glossary](https://developer.holochain.org/resources/glossary/) — authoritative; consult when in doubt.
