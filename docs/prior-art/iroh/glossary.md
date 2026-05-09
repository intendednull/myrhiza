# Glossary

Iroh-specific terms and the iroh-flavored variants of generic P2P concepts. Generic distributed-systems terms (gossip, eventual consistency, NAT, QUIC) are deferred to other glossaries.

## Core API

- **`Endpoint`** — the long-running object that owns sockets, the keypair, the connection registry, and the relay client. One per process is the expected shape. Construction is via builder; configured with a relay map, discovery service, and ALPN list. The kernel-of-the-application analog of a Holochain `Conductor`.
- **`Connection`** — a QUIC connection to a peer, multiplexing many streams. Returned by `Endpoint::connect` once any path (relay or direct) completes the handshake. Path upgrades happen post-handshake and are visible via `Connection::paths()`.
- **`SendStream` / `RecvStream`** — standard QUIC unidirectional and bidirectional stream halves. No framing on top — applications layer their own (length-prefix, postcard, etc.).
- **`Router`** — registers per-ALPN `ProtocolHandler` instances on an `Endpoint`. Spawned via `Router::builder(endpoint).accept(alpn, handler).spawn()`.
- **`ProtocolHandler`** — trait an application implements to handle incoming connections for one ALPN.

## Identity

- **`NodeId`** — legacy name for the 32-byte Ed25519 public key that identifies a peer. Renamed `EndpointId` in iroh 0.94 ([0.94 release notes](https://www.iroh.computer/blog/iroh-0-94-0-the-endpoint-takeover)). Older docs and many production deployments still use `NodeId`.
- **`EndpointId`** — current name for the same concept. The public key *is* the identity — there is no separate certificate authority. TLS certs presented during the QUIC handshake are self-signed by this key.
- **`EndpointAddr`** — the address bundle: `(EndpointId, Vec<TransportAddr>)`. A peer is named by its `EndpointId`; the addresses are advisory hints.
- **`TransportAddr`** — currently `Udp(SocketAddr) | Relay(RelayUrl)`. The enum is shaped to admit future transports (custom-transports API, 0.97).
- **`EndpointTicket`** — base32-encoded serialized `EndpointAddr`. The "share this URL to dial me" primitive. Tickets are addresses, not authority — possession only enables a connection attempt.

## Discovery

- **pkarr** — public-key addressable records over the Mainline DHT. Endpoints publish signed DNS-like records to a public DHT; peers query by `EndpointId`. The default discovery mechanism for n0-preset endpoints; opt-in for self-hosted setups.
- **n0 DNS server** — a centralized DNS-over-HTTPS service operated by Number 0 that mirrors pkarr records. Lower-latency than Mainline DHT but adds a centralization point.
- **mDNS discovery** — local-network discovery via multicast DNS; useful for LAN-only applications.

## Networking

- **ALPN** (Application-Layer Protocol Negotiation) — standard TLS extension for protocol selection during handshake. Iroh's only multiplexing primitive: one endpoint registers N protocol-name byte-strings; the client passes one to `connect`; mismatch refuses the handshake. Convention is `b"<name>/<version>"` but nothing enforces it.
- **Relay** (formerly DERP-derived) — a server that forwards encrypted packets between two NATted peers when direct connectivity fails. The relay sees source/destination NodeIDs and traffic patterns but cannot read content. n0 operates four default relays; `iroh-relay` is the self-hostable binary.
- **`RelayUrl`** — the address of a specific relay server.
- **Hole-punching** — UDP packet exchange via STUN-like address discovery to establish a direct path between two NATted peers, bypassing the relay. iroh's mechanism uses QUIC NAT-traversal frames (`PATH_CHALLENGE`, `REACH_OUT`) that piggyback on an active connection.
- **QAD** (QUIC Address Discovery) — replaces STUN since iroh 0.32. A QUIC-native mechanism for a peer to discover its public address as observed by a server.
- **Multipath QUIC** — landed in iroh 0.96 (Jan 2026). Allows a single `Connection` to use multiple network paths concurrently (relay + direct, Wi-Fi + cellular). Tracks the IETF QUIC multipath draft.
- **`noq`** — iroh's QUIC stack as of 0.97 (Mar 2026). Originally a fork of [Quinn](https://github.com/quinn-rs/quinn), graduated to a separate top-level project. Divergent fork — not a thin wrapper. The fork is what enables iroh-specific QUIC extensions (multipath, NAT-traversal) without upstream churn.

## Data plane

- **`iroh-blobs`** — content-addressed blob transfer. BLAKE3 hashes, Bao verified streaming, HashSeq for collections, range-based requests. No built-in discovery — peers exchange blob hashes out-of-band. Tag-based GC.
- **BLAKE3** — the hash function used for content addressing. A 32-byte hash with a tree structure that enables verified streaming.
- **Bao** — the verified-streaming transfer protocol that exploits BLAKE3's tree structure. A receiver can verify each chunk as it arrives without waiting for the full transfer.
- **`HashSeq`** — a collection abstraction: a sequence of BLAKE3 hashes, itself BLAKE3-hashed. The way iroh-blobs represents directory-shaped content.
- **`BlobTicket`** — the iroh-blobs analog of `EndpointTicket`: encodes `(EndpointAddr, Hash, Format)` so a recipient can dial the host and request the blob in one step.
- **`iroh-docs`** — multi-author eventually-consistent KV. `NamespaceId` (the document) and `AuthorId` (the writer) are dual ed25519 keys. Last-writer-wins by `(timestamp, AuthorId)`. Sync via range-based set reconciliation (RBSR).
- **`iroh-gossip`** — topic-based pub/sub overlay. HyParView (membership) + Plumtree (broadcast) per Leitão et al. 2007. Best-effort delivery; no auth, no spam control.
- **`iroh-willow`** — the [Willow protocol](https://willowprotocol.org/) implementation. Namespace + subspace + path + payload + timestamp data model with prefix pruning, Meadowcap capabilities, and 3d range-based set reconciliation. Currently stalled on `iroh = 0.34` (March 2025), not officially deprecated.

## Willow protocol terminology

- **Namespace** — top-level data scope, identified by a public key. Roughly equivalent to a "document" in iroh-docs.
- **Subspace** — author scope within a namespace. Writes are signed by the subspace key.
- **Path** — a hierarchical key within a subspace. Like a filesystem path.
- **Payload** — the value at a `(namespace, subspace, path, timestamp)` coordinate.
- **Meadowcap** — Willow's capability system for delegating write authority over namespace/subspace/path ranges.
- **3d RBSR** — range-based set reconciliation across the three dimensions (subspace, path, time). The sync algorithm.
- **Confidential Sync** — Willow's encrypted-sync mode (formerly WGPS, "Willow General-Purpose Sync"). Renamed in October 2025.

## Crates and tools

- **`iroh`** — core library (Endpoint, Connection, dial/accept, hole-punching, relay client).
- **`iroh-base`** — shared primitives (Hash, key types, RelayUrl, EndpointTicket).
- **`iroh-relay`** — relay server binary + client protocol module.
- **`iroh-dns-server`** — the DNS server backing n0's endpoint-ID discovery service.
- **`iroh-net-report`** *(no longer a workspace member)* — standalone NAT-class / reachability / RTT probe utility published as a separate crate. Some older write-ups list it as part of the iroh workspace; as of the 0.90+ workspace shape it is not.
- **`iroh-net`** *(legacy)* — pre-0.29 separate crate, folded into `iroh` in the "Net is the new iroh" rename (Dec 2024); workspace further consolidated in the 0.90 "Canary Series" reorg (Jun 2025).
- **`iroh-ffi`** *(unmaintained for production)* — UniFFI bindings for iOS / Android. README self-declares "reference example only" since Feb 2025; GitHub `archived` flag is *not* set as of May 2026 but the repo has not had functional updates. No successor shipped before 1.0.0-rc.0.
- **`iroh-c-ffi`, `iroh-js`** — paid commercial FFI offerings from Number 0.
- **`iroh-doctor`** — diagnostic tool for debugging connectivity / hole-punching.
- **`sendme`** — single-binary file transfer utility built on iroh-blobs.
- **`dumbpipe`** — n0's hello-world demo: pipe stdio across iroh.
- **`patchbay`** — Linux network-namespace simulator used by iroh's integration tests.
- **`chuck`** — iroh's continuous-perf-on-main runner; reports to perf.iroh.computer.
- **`n0-future`** — utility async crate from Number 0.
- **`n0-error`** — error-handling crate from Number 0.

## Project / governance

- **Number 0** (n0) — the private company that develops iroh. Funded "partly venture capital and partly founder-backed"; no publicly disclosed funding rounds.
- **beetle** — the original repo (an IPFS rewrite); archived in late 2023 after the pivot to iroh-as-toolkit in Feb 2023. Last GitHub push was 2023-11-22; the formal archive event isn't directly exposed by the GitHub API.
- **n0-spec** — *does not exist as of May 2026.* Iroh is a single-implementation protocol; the relay wire format lives in `iroh-relay/src/protos/relay.rs`. The 1.0 roadmap commits to publishing specs but 1.0.0-rc.0 shipped without them.

## Sources

- [iroh on docs.rs](https://docs.rs/iroh/latest/iroh/)
- [iroh-blobs on docs.rs](https://docs.rs/iroh-blobs/latest/iroh_blobs/)
- [Willow protocol](https://willowprotocol.org/)
- [Quinn QUIC implementation](https://github.com/quinn-rs/quinn)
- [iroh 0.94.0 — The Endpoint Takeover](https://www.iroh.computer/blog/iroh-0-94-0-the-endpoint-takeover)
- [iroh 0.96.0 — The QUIC Multipaths to 1.0](https://www.iroh.computer/blog/iroh-0-96-0-the-quic-multipaths-to-1-0)
- [iroh 0.97.0 — Custom transports & noq](https://www.iroh.computer/blog/iroh-0-97-0-custom-transports-and-noq)
- [QAD: STUN to QUIC Address Discovery](https://www.iroh.computer/blog/qad)
- [pkarr](https://github.com/pubky/pkarr)
