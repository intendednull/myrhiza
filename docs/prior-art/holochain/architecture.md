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

Since 0.0.144 (June 2022; stabilized in the 0.1 line), zomes are split into two flavors ([Dev Pulse 121](https://blog.holochain.org/integrity-and-coordination-part-ways/), [build/zomes](https://developer.holochain.org/build/zomes/)):

- **Integrity zomes** define entry/link types and validation callbacks. They are hashed into the DNA hash, so any change forks the network. They use the smaller `hdi` crate (deterministic subset).
- **Coordinator zomes** hold all the imperative logic: zome calls, init callbacks, signal emitters, remote calls, scheduler hooks. They depend on the full `hdk`. Crucially, **coordinator zomes can be swapped at runtime without forking the network** — this is the upgrade story. See [`distribution.md` § Coordinator hot-swap via `UpdateCoordinators`](distribution.md#coordinator-hot-swap-via-updatecoordinators) for the actual mechanism, payload shape, and the operational gaps.

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

The networking layer is **Kitsune2** (data/gossip layer) over **iroh** (transport, added in 0.6.0, default in the 0.6.1-rc line) or **tx5** (WebRTC transport, the 0.5 default). See [`networking.md`](networking.md) for the gossip protocol and history.

## The ribosome and Wasmer integration

The **ribosome** is the abstraction that sits between the conductor and a cell's WASM module. It's a Rust trait (`RibosomeT`, with `RealRibosome` as the concrete implementation backed by [`holochain_wasmer_host`](https://github.com/holochain/holochain-wasmer)) responsible for:

1. Compiling and caching WASM modules. `holochain_wasmer_host::module::ModuleCache` keeps compiled artifacts in memory and on disk so the same `.wasm` doesn't recompile per call ([holochain-wasmer README](https://github.com/holochain/holochain-wasmer)).
2. Building a Wasmer `Imports` object that exposes all host functions to the guest.
3. Marshalling arguments and results across the WASM boundary using a pointer/length scheme over a `u64` return value.
4. Dispatching invocations: `init`, `validate`, `genesis_self_check`, `post_commit`, `recv_remote_signal`, scheduler callbacks, and arbitrary zome functions exposed via `#[hdk_extern]`.

The ribosome trait exists so the conductor can swap in test/mock ribosomes (e.g. a `MockRibosome` for unit testing zome behaviour without WASM compilation), and so future engines (e.g. Wasmtime) could be slotted in alongside Wasmer.

### WASM ABI

Guests are pure Wasmer modules with `extern "C"` exports. The ABI is symmetric in both directions: arguments and returns are MessagePack-serialized payloads passed as `(GuestPtr, Len)` pairs packed into a `u64`. The guest exports `__hc__allocate_1` so the host can reserve memory inside the guest before copying serialized bytes; the guest leaks return buffers and the host frees them ([ON-WASM.md](https://github.com/holochain/holochain/blob/develop/crates/hdk/ON-WASM.md), [holochain-wasmer](https://github.com/holochain/holochain-wasmer)).

The HDK/HDI macro layer (`#[hdk_extern]`, `host_call!`, `holochain_externs!`) generates the boilerplate. `host_call!` synchronously serializes args, calls the imported host function, blocks until it returns, and deserializes the response — guests cannot re-enter the host while a host call is in flight (a Wasmer constraint, [holochain-wasmer](https://github.com/holochain/holochain-wasmer)).

### Serialization: `holochain_serialized_bytes`

The wire format is **MessagePack** wrapped in a `SerializedBytes` newtype (`#[repr(transparent)]` over `Vec<u8>`) that enforces a single canonical round-trip via `TryFrom`. The msgpack profile preserves field names (struct-tagged, not positional), which costs bytes but makes payloads forward/backward compatible across struct field reordering ([holochain_serialized_bytes docs](https://docs.rs/holochain_serialized_bytes/latest/holochain_serialized_bytes/struct.SerializedBytes.html), [holochain-serialization repo](https://github.com/holochain/holochain-serialization)). Custom encodings (anything non-msgpack) must go through `UnsafeBytes`. No formal schema-version negotiation; compatibility is informal and breaking changes happen at minor releases.

## DHT op types in full

A single source-chain commit fans out into multiple DHT ops, each going to a different basis hash and validated by a different authority set. From [build/dht-operations](https://developer.holochain.org/build/dht-operations/):

| Op | Produced by | Basis hash | Authority |
|---|---|---|---|
| `RegisterAgentActivity` | every action | author's `AgentPubKey` | agent activity authority (peers near the author's key) |
| `StoreRecord` | every action | action hash | peers near the action hash |
| `StoreEntry` | `Create`/`Update` of public entry | entry hash | peers near the entry hash |
| `RegisterUpdate` | `Update` | original entry & action hash | peers near the original entry/action |
| `RegisterDelete` | `Delete` | original entry & action hash | peers near the original entry/action |
| `RegisterCreateLink` | `CreateLink` | link base address | peers near the base |
| `RegisterDeleteLink` | `DeleteLink` | original link base + action hash | peers near base/action |
| `WarrantOp` | system, on validation failure | the bad-actor's `AgentPubKey` | agent activity authority |

Private entries skip `StoreEntry` (the entry stays on the author's chain). The `DhtOp` enum at the type level is just `ChainOp(Box<ChainOp>)` and `WarrantOp(Box<WarrantOp>)` ([DhtOp docs](https://docs.rs/holochain_types/latest/holochain_types/dht_op/enum.DhtOp.html)).

## Source chain commit pipeline

```
zome fn invoked
    -> HDK call (e.g. create_entry)
       -> host produces an Action (Create, Update, Delete, CreateLink, ...)
          -> action hashed into ActionHash
             -> lair signs the hash, producing SignedActionHashed
                -> appended to source chain (sqlite, agent-local)
                   -> cascade derives DhtOps from the action
                      -> publish: ops gossiped to authorities for each basis
                         -> authority runs integrity validate(op) callback
                            -> Valid     -> store + return ValidationReceipt
                            -> Unresolved -> park in validation limbo, retry
                            -> Invalid    -> publish Warrant against author
```

`Record = (SignedActionHashed, Option<Entry>)` is the canonical pair returned from `get`. `Action` is an enum: `Dna`, `AgentValidationPkg`, `Create`, `Update`, `Delete`, `CreateLink`, `DeleteLink`, `OpenChain`, `CloseChain`, `InitZomesComplete` ([concepts/3_source_chain](https://developer.holochain.org/concepts/3_source_chain/)). Every action carries `prev_action`, `author`, `timestamp`, and a sequence number — except `Dna` which has none of those, being position 0.

## Lair keystore

[Lair](https://github.com/holochain/lair) is the secret-management process. Conductors never hold private keys directly — they call into lair over an IPC connection (Unix domain socket on Linux/macOS, named pipe on Windows; recently lair can also be linked in-process for bundled apps). Lair stores **seeds** in an encrypted SQLite database (sqlcipher), from which Ed25519 (signing) and X25519 (encryption) keypairs are derived on demand. Components ([Least Authority audit](https://leastauthority.com/blog/audit-of-holochain-lair-keystore/)):

- `sodoken` — tokio-safe wrappers over libsodium, providing memory-protected secret buffers.
- `hc_seed_bundle` — encrypted, passphrase-locked seed export format.
- `lair_keystore_api` — server/client for IPC.
- `lair_keystore` — the executable + sqlcipher backend.

The boundary lair enforces: **private keys never leave the keystore**. All signing, encrypting, decrypting happens inside lair; conductors get back ciphertexts/signatures, never raw keys. A compromised conductor process or zome cannot exfiltrate the agent identity even if it gains arbitrary memory access — lair lives in a separate address space (in IPC mode).

### Lair IPC wire protocol

The protocol over the Unix domain socket / Windows named pipe is **not** msgpack-RPC and **not** Noise — it's a custom libsodium-based handshake plus an authenticated-encryption stream carrying msgpack-serialized request/response objects. From [`lair_keystore_api/src/sodium_secretstream.rs`](https://github.com/holochain/lair/blob/main/crates/lair_keystore_api/src/sodium_secretstream.rs):

**Connection URL.** `unix:///path/to/socket?k=<base64-server-pub-key>` on Linux/macOS, `named-pipe:\\.\pipe\<name>?k=<base64-server-pub-key>` on Windows. The server's X25519 public key is **embedded in the URL** (`k=` query param) — the conductor pins server identity by URL, no PKI involved.

**Handshake** (server-authenticated, no client identity):

1. Client generates ephemeral X25519 cbox + kx keypairs.
2. Client sends `xsalsa_seal(eph_cbox_pub || eph_kx_pub)` (96 bytes) to the server using the server's public key from the URL — only the holder of the matching server private key can decrypt.
3. Server decrypts, generates its own ephemeral kx pubkey, seals it back to the client's ephemeral cbox key (64 bytes).
4. Both sides derive `(rx_key, tx_key)` from `crypto_kx::client_session_keys` over the ephemeral kx pubkeys.
5. Each direction initializes a libsodium **`secretstream`** (XChaCha20-Poly1305) keyed by its tx key, sending a 24-byte header.

**Framing.** A trivial 2-byte little-endian length prefix per encrypted record, with a hard `MAX_FRAME = 8 KiB` cap (oversized messages return `FrameOverflow`). All framing happens *outside* the cipher; libsodium's secretstream provides the AEAD over each frame.

**Payload encoding.** Inside the encrypted stream: `rmp_serde::Serializer::with_struct_map()` — MessagePack with named struct fields (same dialect as `mr_bundle`). Every message is a `LairApiEnum` variant: `Hello`, `Unlock`, `NewSeed`, `SignByPubKey`, `GetEntry`, etc. Requests carry an `Arc<str>` `msg_id` (nanoid) for response correlation; responses echo it back.

**Authentication flow** (after the handshake):

1. `LairApiReqHello { nonce: 32 random bytes }` → server returns `LairApiResHello { name, version, server_pub_key, hello_sig }` where `hello_sig` is an Ed25519 signature over the nonce by the *same* key that the URL pinned. The client verifies and rejects on mismatch.
2. `LairApiReqUnlock { passphrase: SecretData }` → the passphrase is argon2id-hashed against the database key; the connection moves to "unlocked" state and may now issue `SignByPubKey`, `NewSeed`, etc.

Two failure modes worth noting: (a) version-mismatch is silent unless the client opts in via `exact_client_server_version_match`; (b) the 8 KiB frame cap means any payload larger than ~8 KB needs application-level chunking — relevant for batch operations that rmp_serde-encode to anything substantial.

## Conductor admin vs app websocket APIs

The conductor exposes two distinct websocket interfaces ([AdminRequest docs](https://docs.rs/holochain/latest/holochain/conductor/api/enum.AdminRequest.html)):

**Admin API** (`AdminRequest` — 29 variants). Trusted, typically only a local management UI talks to it: `InstallApp`, `UninstallApp`, `EnableApp`/`DisableApp`, `ListApps`, `ListCellIds`, `ListDnas`, `GenerateAgentPubKey`, `AttachAppInterface`, `GraftRecords`, `UpdateCoordinators`, `GrantZomeCallCapability`, `RevokeZomeCallCapability`, `IssueAppAuthenticationToken`, `DumpState`, `DumpFullState`, `DumpNetworkStats`, `StorageInfo`, etc. No per-call signing; trust is bound to the socket itself (loopback + local OS process).

**App API.** Per-app interface, one per attached port. Callers must first obtain an *app authentication token* via `IssueAppAuthenticationToken` on the admin socket, then connect with `(token, signing_key)`. Every zome call is signed by the client signing key and presents a capability secret matching a `ZomeCallCapGrant` on the cell's source chain — the conductor mediates the cap-token check before invoking the ribosome ([holochain_client docs](https://docs.rs/holochain_client/latest/holochain_client/struct.AppWebsocket.html)). Both APIs use the same envelope: msgpack-serialized request/response with a request id.

## Sources

- [Build Guide — DNAs](https://developer.holochain.org/build/dnas/)
- [Build Guide — Zomes](https://developer.holochain.org/build/zomes/)
- [Build Guide — DHT Operations](https://developer.holochain.org/build/dht-operations/)
- [Build Guide — Connecting a Front End](https://developer.holochain.org/build/connecting-a-front-end/)
- [Concepts — DHT](https://developer.holochain.org/concepts/4_dht/)
- [Concepts — Source Chain](https://developer.holochain.org/concepts/3_source_chain/)
- [Dev Pulse 121: Integrity and Coordination Part Ways](https://blog.holochain.org/integrity-and-coordination-part-ways/)
- [holochain-wasmer repo](https://github.com/holochain/holochain-wasmer)
- [holochain ON-WASM.md](https://github.com/holochain/holochain/blob/develop/crates/hdk/ON-WASM.md)
- [holochain_serialized_bytes docs](https://docs.rs/holochain_serialized_bytes/latest/holochain_serialized_bytes/struct.SerializedBytes.html)
- [DhtOp enum docs](https://docs.rs/holochain_types/latest/holochain_types/dht_op/enum.DhtOp.html)
- [AdminRequest docs](https://docs.rs/holochain/latest/holochain/conductor/api/enum.AdminRequest.html)
- [holochain_client AppWebsocket](https://docs.rs/holochain_client/latest/holochain_client/struct.AppWebsocket.html)
- [Lair repository](https://github.com/holochain/lair)
- [Lair Keystore audit (Least Authority)](https://leastauthority.com/blog/audit-of-holochain-lair-keystore/)
