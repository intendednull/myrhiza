**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — networking, transport, sync protocols

Willow's transport substrate, gossip topology, blob plane, and sync
protocols. Documents what shipped in `crates/network/` after the
2026-03-29 iroh migration retired libp2p. Companion: [identity.md](identity.md),
[crypto.md](crypto.md), [ui.md](ui.md), [README.md](README.md).

## Iroh as the substrate (today)

Willow replaced libp2p with iroh in the migration spec at
`docs/specs/2026-03-29-iroh-migration-design.md` (status: completed
2026-04-18). The substitution was end-to-end, not adapter-shaped:
peer addresses are now raw 32-byte Ed25519 public keys
(`iroh_base::EndpointId`), not multihashes (`PeerId`). `EndpointId`
is the wire identity, the gossip address, and the lookup key for
permissions / channel keys / profiles. The migration spec lists six
concrete simplifications driving the choice: single `Endpoint` type
replacing six libp2p `NetworkBehaviour`s; built-in hole-punching with
relay fallback; ALPN routing replacing behaviour composition; native
WASM transport with no separate WebSocket adapter; `iroh-blobs` with
BLAKE3-verified streaming replacing a custom chunk protocol; ~150
fewer transitive deps (`docs/specs/2026-03-29-iroh-migration-design.md:19-36`).

Browser viability is the load-bearing piece. Willow's CLAUDE.md states
that "iroh handles native/WASM transport differences internally,"
and most `#[cfg(target_arch = "wasm32")]` gates around networking
that existed under libp2p went away
(`willow CLAUDE.md` § Dual-Target Support). On WASM, an `Endpoint`
runs relay-only; on native, it gets direct QUIC plus relay fallback.
Both are the same type, configured differently.

## Trait surface (`crates/network/src/traits.rs`)

The crate defines four traits so production iroh and an in-memory
test double share semantics:

- **`Network`** (`traits.rs:139-182`) — top-level handle. `id() ->
  EndpointId`, `subscribe(TopicId, bootstrap) -> (Topic, Events)`,
  `unsubscribe`, `blobs() -> &dyn BlobStore`, `relay_status()`,
  `device_online()`, `shutdown()`. The two associated types
  (`Topic`, `Events`) let each impl pick concrete handle / receiver
  shapes.
- **`TopicHandle`** (`traits.rs:67-76`) — sender side of a gossip
  subscription. `broadcast(Bytes)`, `broadcast_neighbors(Bytes)`,
  `neighbors()`. Mirrors `iroh_gossip::GossipSender`.
- **`TopicEvents`** (`traits.rs:84-90`) — receiver side. `next()
  -> Option<Result<GossipEvent>>`, `joined()`. `GossipEvent` =
  `Received { content, sender }` | `NeighborUp` | `NeighborDown`.
- **`BlobStore`** (`traits.rs:98-113`) — content-addressed bytes.
  `add(Bytes) -> BlobHash`, `get(BlobHash) -> Option<Bytes>`,
  `has`, `remove`, `store_size`. `BlobHash([u8; 32])` is BLAKE3.

The trait surface speaks iroh types directly (`EndpointId`,
`TopicId`, `Bytes`). It is not an abstraction layer over a library —
it is a seam for testability. Production: `IrohNetwork` in
`crates/network/src/iroh.rs`. Tests: `MemNetwork` in
`crates/network/src/mem.rs`, gated behind the `test-utils` feature
and never compiled into production binaries
(`crates/network/src/lib.rs:22-23`). `MemHub` (`mem.rs:63-69`) is
the in-process broadcast bus that connects test peers; one hub per
test ensures isolation.

## Topic IDs (`crates/network/src/topics.rs`)

All gossip topics are 32-byte BLAKE3 hashes derived from a
human-readable string: `topic_id(name) = blake3::hash(name.as_bytes())`
(`topics.rs:12-15`). System topics are `_willow_server_ops`,
`_willow_workers`, `_willow_profiles`. Per-channel topics:
`channel_topic(server_id, channel_id) = topic_id(format!("{}/{}",
server_id, channel_id))`. Voice signaling: `voice_topic = topic_id(
format!("{}/{}/voice", ...))`.

The hash-as-topic-id pattern is shared with iroh-gossip natively.
What Willow adds is the deterministic naming convention and the
test-pinned property that name → hash is one-to-one
(`topics.rs:55-88`).

## Sync protocols

Willow has consolidated on a single delta-exchange protocol after
the four 2026-04-24 sync specs landed:

- **`HeadsSummary`-based delta** — `BTreeMap<EndpointId, AuthorHead
  { seq, hash }>` exchanged in `SyncRequestV2 { request_id, heads,
  filter }`. Per-author monotonicity is enforced by the DAG itself
  (`crates/state/src/dag.rs:146-158`), so streaming `seq >
  known_max` ascending delivers a contiguous chain with no gaps and
  no fingerprint negotiation
  (`docs/specs/2026-04-24-negentropy-sync.md:60-95`). Replaces a
  legacy `WireMessage::SyncRequest { state_hash, topic }` that
  could only return "first 500 events from topological sort."
- **`HistorySyncComplete` EOSE signal** — explicit
  `WireMessage::HistorySyncComplete { topic_id, last_event_hash,
  stream_generation }` so the joining client knows when backfill
  is done per-topic per-provider
  (`docs/specs/2026-04-24-history-sync-eose.md:46-80`). Direct
  borrow from Nostr NIP-01's EOSE marker.
- **pkarr-based relay discovery** — iroh's pkarr (signed DNS
  packets over BitTorrent mainline DHT) gives `EndpointId →
  addresses`; the `/.well-known/willow` capability doc gives
  `EndpointId → role/capabilities`; the `SyncProvider` grant
  in the event DAG gives `EndpointId → trust`. Three independent
  layers, composed by the client at dial time
  (`docs/specs/2026-04-24-outbox-relay-discovery.md:62-72`). The
  spec explicitly *rejected* a Nostr-NIP-65-style replaceable
  `RelayList` event because it would have introduced replaceable-kind
  semantics into a single-pass `apply_event`.
- **Relay capability doc** — NIP-11-style `/.well-known/willow`
  JSON sidecar served on the same port as the relay handshake.
  Lets clients pick wire version, surface
  degraded/full state, and filter directories pre-connection
  (`docs/specs/2026-04-24-relay-capability-doc.md:10-66`).

## Relay = dumb topic-bridge

In current Willow, the relay is a minimal proxy: a public TCP
port (default 3340) that dispatches `/bootstrap-id` and the
upcoming `/.well-known/willow` and forwards everything else to a
loopback iroh-relay instance. The migration spec calls out the
security improvement: under libp2p the relay saw all GossipSub
traffic in plaintext; under iroh-relay it is pure encrypted
packet forwarding and "cannot read message content"
(`docs/specs/2026-03-29-iroh-migration-design.md:158-170`). Bootstrap
participation is split into a separate process / role.

## Lift-into-Myrhiza notes

iroh is already a Myrhiza load-bearing dependency
(see `prior-art/iroh/`); the question is what *trait shape* the
kernel exposes around it. Willow's `Network` / `TopicHandle` /
`TopicEvents` / `BlobStore` is a credible starting point — it
keeps test-double substitution fast (`MemNetwork` + `MemHub` ran
the multi-peer client tests today) without leaking abstraction
between iroh and apps. Lift the trait shape; expose it through
the kernel as a host-imported capability, not as a re-exported
public API.

`HeadsSummary`-based sync directly applies. Myrhiza apps will
have per-author DAGs and need delta exchange between peers; the
spec's "no fingerprint negotiation needed because per-author
monotonicity is structural" is a property the kernel inherits
for free as long as it preserves the per-author chain invariant.

Relay-as-dumb-topic-bridge is a Myrhiza commitment per CLAUDE.md
("Relays are gossip-driven, not state-driven"). PR #636 surfaces
one constraint Willow has not solved: app-driven topic-ID
rotation (the epoch-rotation spec relies on rotated topic IDs
being unpredictable to non-members for unlinkability) requires
the relay to follow rotations *without* publishing the next ID
on a public channel. PR #636 commits only to "the kernel is not
in this loop" and defers the protocol shape to a relay-and-rotation
child spec; the in-flight Willow epoch-rotation work
(`docs/specs/2026-04-24-epoch-key-rotation.md`) has to land in the
new shape because the relay no longer runs app code under the
runtime model (`PR #636 lines 538-550`). This is unsolved on
both sides.

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)

## Sources

- Willow repo: `/mnt/storage/projects/willow`
- `crates/network/src/lib.rs` — module exports
- `crates/network/src/traits.rs` — `Network`, `TopicHandle`, `TopicEvents`, `BlobStore`, `RelayStatus`, `BlobHash`, `GossipEvent`
- `crates/network/src/iroh.rs` — production `IrohNetwork`, `Config`, `IrohBlobStore`
- `crates/network/src/mem.rs` — `MemNetwork`, `MemHub`, `MemTopicHandle`, `MemTopicEvents`, `MemBlobStore` (test-utils only)
- `crates/network/src/topics.rs` — BLAKE3 topic-ID derivation, system topic constants
- `docs/specs/2026-03-29-iroh-migration-design.md` — full migration rationale and crate-by-crate diff
- `docs/specs/2026-04-24-negentropy-sync.md` — `HeadsSummary` delta protocol consolidation
- `docs/specs/2026-04-24-history-sync-eose.md` — `HistorySyncComplete` EOSE signal
- `docs/specs/2026-04-24-outbox-relay-discovery.md` — pkarr + capability doc + `SyncProvider` composition
- `docs/specs/2026-04-24-relay-capability-doc.md` — NIP-11-style sidecar
- PR #636 §"Relays are gossip-driven, not state-driven" (lines 535-550)
- `willow CLAUDE.md` § Dual-Target Support, § Architecture Notes
