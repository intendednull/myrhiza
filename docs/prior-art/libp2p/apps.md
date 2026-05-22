**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — production users (Ethereum, Filecoin, IPFS, Polkadot, Status, Drand)

# Apps + production users

libp2p is one of the most-deployed P2P stacks in production. The Ethereum consensus layer alone (5 client implementations, ~1M+ validator keys, ~10k+ full beacon nodes) is arguably the largest live P2P network ever shipped. Filecoin and IPFS are the original Protocol Labs flagships. Polkadot's substrate-network ships on rust-libp2p. Status Network's Nimbus + Waku ship on nim-libp2p.

This file enumerates the production deployments to ground the "libp2p is at-scale" claim, and walks one app end-to-end as a worked example.

## Tier 1: at-scale production

### Ethereum consensus layer (the largest deployment)

The **Ethereum beacon chain + post-Merge mainnet** is libp2p's flagship deployment by node count + economic value. Five major client implementations all use libp2p:

| Client | Language | libp2p impl | Steward |
|---|---|---|---|
| **Prysm** | Go | go-libp2p | Offchain Labs (formerly Prysmatic Labs) |
| **Lighthouse** | Rust | rust-libp2p + custom | Sigma Prime |
| **Teku** | Java | jvm-libp2p | ConsenSys |
| **Nimbus** | Nim | nim-libp2p | Status Research & Development |
| **Lodestar** | TypeScript | js-libp2p + `@chainsafe/libp2p-gossipsub` | ChainSafe |

The Eth2 P2P spec (called "the consensus layer p2p spec" inside Ethereum) builds on libp2p with Eth2-specific layers:

- **Gossipsub** carries blocks, attestations, sync committee messages, and slashing reports — the high-rate fan-out workload (~1.5 MB attestation messages, ~30/slot, ~12-second slots).
- **req/resp** uses libp2p streams with snappy-compressed SSZ-encoded payloads for direct peer-to-peer block requests and sync.
- **Discovery v5** (a libp2p-adjacent DHT-like protocol, not standard libp2p Kademlia) for peer discovery. The Eth team specifically *did not* use libp2p's Kademlia because Discovery v5 was already in use from Eth1 and the team chose continuity over adoption.

Eth2 production scale (approximate, mid-2025):

- ~1M+ active validator keys (each validator is a logical entity; multiple validators per physical node is common).
- ~10k+ beacon-chain full nodes.
- ~5–15 message broadcasts per slot across multiple gossipsub topics.
- gossipsub v1.1 universally; v1.2 IDONTWANT rolling out unevenly.
- ~$50B+ ETH staked.

This deployment is the most-attacked, most-instrumented gossipsub deployment in existence. The peer-scoring parameters were tuned against this workload; the spec text reflects production learnings.

### Filecoin

[Filecoin](https://filecoin.io/), Protocol Labs' incentivized data-storage network, ships on go-libp2p (Lotus, the reference Filecoin client). Filecoin was the **original at-scale stress test** for libp2p — gossipsub v1.1's peer scoring was largely shipped in response to Filecoin's 2020-21 mainnet attacks.

Components:

- **Lotus** (go-libp2p) — block + message propagation via gossipsub; chain sync via libp2p streams.
- **Forest** (rust-libp2p) — ChainSafe's Rust Filecoin client.
- **Boost** (Go) — storage-provider deals layer.

Filecoin uses libp2p for:

- Block + message gossip (gossipsub).
- Storage-deal protocol (libp2p streams, custom protocols).
- Retrieval-deal protocol (libp2p streams, with custom hooks into IPFS bitswap).
- Drand-randomness propagation.

### IPFS / kubo

kubo (formerly go-ipfs) is the reference IPFS implementation. Every public IPFS gateway, every Brave Web3 user, every Cloudflare IPFS gateway runs on go-libp2p underneath.

IPFS uses libp2p for:

- **Bitswap** — block exchange protocol over libp2p streams.
- **Kademlia DHT** — provider records for `cid → peer` lookup.
- **mDNS** — local-network IPFS discovery.
- **Gossipsub** — for IPNS over pubsub and other ambient pub/sub use cases.

IPFS's public DHT (the libp2p Kademlia DHT in `Mode::Server`) has 25k–50k peers in steady state. The provider-lookup performance issues at this scale drove the development of:

- **Accelerated DHT client** ([blog](https://blog.ipfs.io/2023-09-13-accelerated-dht-client/)) — caches more aggressively, parallelises better.
- **Hydra Booster** ([archived 2025](https://github.com/libp2p/hydra-booster)) — Protocol Labs' "indexer" peers that maintained massive routing tables to short-circuit lookups. Archived after re-architecture.

### Polkadot / Substrate

The Polkadot ecosystem (Substrate framework, Polkadot relay chain, dozens of parachains) ships on **rust-libp2p**. This is the original use case Parity built rust-libp2p for — before Protocol Labs adopted it as the canonical Rust impl.

Substrate uses libp2p for:

- Block sync (libp2p streams).
- Transaction gossip (gossipsub, recently — switched from a custom protocol in ~2022).
- Validator-set discovery via Kademlia.
- Off-chain worker messaging.

KAGOME (cpp-libp2p) is the C++ Polkadot client. The libp2p interop is real cross-impl: Substrate (rust) talks to KAGOME (cpp) on the same network.

### Waku / Status Network

[Waku](https://waku.org/) (formerly Whisper, before that the Eth1 Whisper protocol) is Status' privacy-preserving messaging network. nim-libp2p + Go bindings. Used by the Status messenger app, Logos, and various Vac research projects.

Waku uses libp2p for:

- Gossipsub-based relay (with privacy extensions — store nodes, light push protocol).
- Custom Waku-protocol streams over libp2p.

### Drand

[Drand](https://drand.love/) — distributed randomness beacon. Filecoin's chain randomness comes from Drand. Drand nodes form a libp2p network and produce a verifiable beacon every 30 seconds.

### Codex

[Codex](https://codex.storage/) — Status' incentivized data-availability network (Filecoin-shape but with formal verification). nim-libp2p. Pre-launch as of 2026-05.

## Tier 2: shipping, smaller scale

- **Berty** (decentralized messenger, go-libp2p) — formerly Riot, now privacy-first iOS/Android.
- **OrbitDB** (eventually-consistent database on IPFS, js-libp2p) — research-grade but real users.
- **Subspace / Autonomys** (rust-libp2p, blockchain network).
- **Storj** (uses parts of libp2p for inter-node messaging — moved away from full libp2p stack ~2021).
- **Helia** (the modern js-libp2p-based IPFS replacement for `js-ipfs`).
- **Lit Protocol** (decentralized signing network, uses libp2p subset).

## Tier 3: notable but not at-scale

- **Hyperspace** (Holepunch-adjacent research project, archived).
- Various academic + research deployments (Subspace, Iroh's former network during the IPFS-era).
- Bluesky AT Protocol does *not* use libp2p — they built their own federation protocol.

## Worked example: kubo IPFS publishing a CID end-to-end

This walks one operation through the full libp2p stack as the canonical worked example.

**Scenario:** Alice runs kubo, adds a file (`ipfs add hello.txt`), and Bob (running kubo elsewhere) requests it (`ipfs cat <cid>`).

### Stage 1: Alice publishes

1. **`ipfs add hello.txt`** — kubo computes the file's CID (= multihash of content).
2. **Block stored locally** — the block is in Alice's blockstore.
3. **Provider record added to DHT** — Alice's kubo announces "I have CID X" to the 20 (k=20) closest peers in the IPFS DHT. The DHT operation:
   - `FIND_NODE(target=sha256(CID))` to locate the closest peers (~3–5 RTT hops through the DHT).
   - `ADD_PROVIDER(CID, alice.peer_id, alice.multiaddrs)` to each of the 20 closest peers.
   - Each receiving peer stores the record with a 48h TTL.
   - Alice will republish every 22h to keep the record alive.

Wire-level: each DHT operation is a libp2p stream over QUIC (or TCP, depending on connectivity). The stream carries protobuf-encoded `/ipfs/kad/1.0.0` messages.

### Stage 2: Bob requests

1. **`ipfs cat <cid>`** — Bob's kubo looks up the CID.
2. **Cache check** — not in local blockstore; need to fetch.
3. **DHT provider lookup** — Bob's kubo runs `GET_PROVIDERS(CID)`:
   - `FIND_NODE(target=sha256(CID))` to find DHT peers near the CID.
   - α=10 concurrent queries at each hop.
   - On reaching a peer storing the provider record, receive `{providers: [{alice.peer_id, alice.multiaddrs}], closer_peers: [...]}`.
   - Total lookup latency: 5–60 seconds in practice. Iroh's "IPFS performance is not great" critique lives here.
4. **Connect to Alice** — Bob's kubo dials Alice's multiaddr.
   - If Alice has a public IP: direct QUIC connection. ~1 RTT to handshake.
   - If Alice is behind NAT: dial a relay first, attempt DCUtR hole-punch (70% success), fall back to relay if it fails.
5. **Bitswap session** — Bob's kubo opens a `/ipfs/bitswap/1.2.0` stream to Alice. Sends `WANT_HAVE(CID)`, receives `HAVE`, sends `WANT_BLOCK(CID)`, receives the block.
6. **Block stored + Bob now provides** — Bob's kubo stores the block locally and registers itself as a provider in the DHT (closing the loop — popular content has many providers).

### Stage 3: Bob streams the file output

`ipfs cat` returns the block contents to stdout. The block's content is the file's bytes (for small files); larger files are split into multiple blocks linked by a UnixFS DAG, and the lookup recursively fetches each block (with parallel-fetch pipelining).

### What this demonstrates

- **Discovery cost is real.** The 5–60s lookup is a substantial cost per cold-fetch. Iroh's design avoids this by requiring out-of-band PeerId exchange — but at the cost of "you must know the peer to fetch from them."
- **NAT traversal is critical.** Most kubo users are residential, behind NAT. The DCUtR hole-punching success rate (~70%) determines whether end-to-end direct connection is possible; the relay fallback covers the rest.
- **The libp2p stack composes naturally for this workflow.** Discovery (DHT) + connectivity (DCUtR + Circuit Relay) + transport (QUIC) + application protocol (Bitswap) all stack cleanly. The cost is the protocol-upgrade negotiation overhead documented in [`architecture.md`](architecture.md).

## Implications for Myrhiza

- **The "libp2p is at scale" claim is correct.** Ethereum consensus + Filecoin + IPFS together are unambiguously the largest live P2P deployment. Any "libp2p is research-grade" framing is wrong.
- **Eth2's choice not to use libp2p Kademlia (using Discovery v5 instead) is a useful signal.** When a major user picks a libp2p-adjacent-but-not-libp2p discovery protocol, the lesson is: Kademlia is good but not strictly required; the underlying primitive (signed peer-record DHT) is what matters. Myrhiza's pkarr-on-Mainline-DHT inheritance is the same pattern as Discovery v5 — DHT-of-signed-records, distinct from Kademlia-the-protocol.
- **The DHT performance cliff is the recurring critique.** Myrhiza should plan for this if we ever ship a content-addressed discovery layer — see [`discovery.md`](discovery.md) §"Implications".
- **The cross-implementation interop discipline is rare and valuable.** Myrhiza is unlikely to ever have 5 implementations, but if we do, libp2p's test-plans CI is the model. See [`implementations.md`](implementations.md) §"Interop testing".
- **The worked-example walkthrough (kubo IPFS) is the right shape for Myrhiza spec authors to internalise.** A future Myrhiza spec's "how does an app actually exchange data?" section should be this concrete — multiple RTTs, named protocol versions, explicit failure modes.

## Sources

- [Ethereum consensus layer P2P spec](https://github.com/ethereum/consensus-specs/blob/dev/specs/phase0/p2p-interface.md)
- [Ethereum beacon node count (beaconcha.in)](https://beaconcha.in/) — for verifying validator + node counts at time of writing
- [Filecoin Lotus](https://github.com/filecoin-project/lotus) — go-libp2p user
- [IPFS kubo](https://github.com/ipfs/kubo) — go-libp2p reference user
- [Polkadot / Substrate](https://github.com/paritytech/polkadot-sdk)
- [KAGOME — C++ Polkadot](https://github.com/qdrvm/kagome)
- [Status / Nimbus](https://github.com/status-im/nimbus-eth2)
- [Waku](https://github.com/waku-org/nwaku)
- [Drand](https://github.com/drand/drand)
- [Helia](https://github.com/ipfs/helia)
- [Lodestar (Eth2 in TypeScript)](https://github.com/ChainSafe/lodestar)
- [Lighthouse (Eth2 in Rust)](https://github.com/sigp/lighthouse)
- [Teku (Eth2 in Java)](https://github.com/Consensys/teku)
- [Prysm (Eth2 in Go)](https://github.com/prysmaticlabs/prysm)
- [Accelerated DHT client blog](https://blog.ipfs.io/2023-09-13-accelerated-dht-client/)
- [iroh — IPFS performance critique](https://news.ycombinator.com/item?id=39033100)
