**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — Hyperswarm: peer discovery and connection layer

# Hyperswarm

Hyperswarm is the Holepunch stack's answer to the question "two computers, no
account servers, no port forwarding — how do they find and talk to each
other?" It is the load-bearing transport under Keet, Pear app distribution,
and every Hypercore-replicated thing in the Holepunch ecosystem. For Myrhiza
spec authors choosing [Iroh](../iroh/) as the runtime's transport, this is
the reference comparison: the only other P2P stack in this prior-art set that
has shipped to a real consumer-mobile audience and survived contact with
production NATs.

This file covers Hyperswarm's discovery and connection layer. A side-by-side
on the engineering choices vs Iroh lives in [`./transport-comparison.md`](./transport-comparison.md).

## Package layout

The Holepunch transport stack is a small tower of npm packages, all under the
[`holepunchto`](https://github.com/holepunchto) GitHub org, all MIT-licensed
(not Apache-2.0; correcting the brief). Verified versions as of 2026-05-09:

| Package | Latest | Released | Role |
|---|---|---|---|
| [`hyperswarm`](https://github.com/holepunchto/hyperswarm) | 4.17.0 | 2026-02-20 | High-level "join a topic, get connections" API |
| [`hyperdht`](https://github.com/holepunchto/hyperdht) | 6.32.0 | 2026-05-05 | The DHT + holepunching engine |
| [`dht-rpc`](https://github.com/holepunchto/dht-rpc) | 6.27.0 | 2026-05-05 | Generic Kademlia-flavored DHT-RPC primitive |
| [`@hyperswarm/secret-stream`](https://www.npmjs.com/package/@hyperswarm/secret-stream) | 6.9.1 | 2025-10-07 | Noise + libsodium secretstream over a raw stream |
| [`protomux`](https://github.com/mafintosh/protomux) | 3.11.0 | 2026-05-05 | Multi-protocol multiplexer over a framed stream |
| [`blind-relay`](https://github.com/holepunchto/blind-relay) | 1.3.x | (transitive) | TURN-equivalent fallback over Protomux + UDX |
| [`udx-native`](https://github.com/holepunchto/udx-native) | — | — | Native UDP-over-X reliable stream (the actual datagram surface) |

The `hyperswarm` package itself is small — it depends on `hyperdht` and adds
discovery-session bookkeeping (`PeerDiscovery`, retry timers, connection
sets). Most of the interesting machinery lives one layer down in `hyperdht`.

## Two phases of the swarm

Hyperswarm splits the problem into two phases:

1. **Topic discovery.** "Who else cares about this 32-byte topic hash?"
   Answered by the DHT: peers in *server* mode announce themselves on the
   topic; peers in *client* mode query for that topic. The lookup yields a
   list of (public-key, address-hint) pairs.
2. **Connection establishment.** Given a peer's 32-byte ed25519 public key,
   open an end-to-end-encrypted bidirectional stream to them. This is where
   UDP holepunching lives.

The two phases are orthogonal in code: `hyperdht` exposes
`createServer()` / `connect(publicKey)` directly, and `hyperswarm` is just a
discovery-layer convenience over that pair. An app could skip discovery
entirely and dial a known public key.

### Topic semantics

A topic is **any 32-byte hash**. Convention is `sha256(application-defined
string)` — Hypercore-replication topics are derived from the discovery key
of the core, app-defined topics use whatever hash the app picks. Topics are
flat: there is no hierarchy, no wildcards, no subscription patterns. Compare
to libp2p's gossipsub topics (which are strings carrying a delivery
semantics) or Iroh's `iroh-gossip` topics (also flat 32-byte hashes — same
shape, slightly different protocol underneath; see
[`../iroh/gossip.md`](../iroh/gossip.md)).

A peer in server mode for topic *T* publishes a record on the DHT at
`announce(T)`; a peer in client mode does `lookup(T)` and gets back the
recently-announced peer set. Both modes can be combined on the same swarm —
in fact `swarm.join(topic)` defaults to `{ server: true, client: true }`.
The brief asked about gossip-discovered topic membership: it's not gossip in
the gossipsub sense. It's pull-from-DHT-on-refresh, with a
`REFRESH_INTERVAL` of 10 minutes plus 2 minutes of jitter (verified in
`hyperswarm/lib/peer-discovery.js`).

### What's in a topic announce

The DHT-level `ANNOUNCE` command (verified in `hyperdht/lib/constants.js`)
carries the topic hash, the announcing peer's public key, and a small set of
candidate-address hints. Records are signed by the announcer; the DHT layer
can reject records whose signature doesn't match the claimed public key.
Records expire — they're refreshed on the same 10-minute cadence that
`PeerDiscovery._refreshLater` enforces.

## The DHT

`hyperdht` is built on `dht-rpc`, a generic Kademlia-flavored DHT-RPC
primitive maintained by the same author (Mathias Buus). The interesting
parts vs vanilla Kademlia:

- **Customized to carry holepunching coordination.** The 11 commands defined
  in `hyperdht/lib/constants.js` are
  `PEER_HANDSHAKE`, `PEER_HOLEPUNCH`, `FIND_PEER`, `LOOKUP`, `ANNOUNCE`,
  `UNANNOUNCE`, `MUTABLE_PUT`, `MUTABLE_GET`, `IMMUTABLE_PUT`,
  `IMMUTABLE_GET`, `PLUGIN`. The first two are dedicated to the
  holepunch-coordination side channel — DHT nodes act as the
  rendezvous-and-relay-of-introductions during NAT traversal.
- **Mutable + immutable records.** `MUTABLE_PUT/GET` is signed-by-public-key
  records (sequence-numbered, replace-on-higher-seq). `IMMUTABLE_PUT/GET` is
  content-addressed records. These are the building blocks under which
  Hyperdrive bootstrap and Pear's rolling-update mechanism are implemented
  (see [`./pear-runtime.md`](./pear-runtime.md)).
- **Secure routing IDs.** `dht-rpc` v5 introduced "secure routing IDs" — a
  routing-table-position derivation that ties a node's DHT identity to a
  fresh ephemeral key signed under its long-term key, mitigating the cheap
  Sybil eclipse attacks that plagued Kademlia DHTs. The README calls this
  out as a v5-vs-v4 difference.

### Bootstrap nodes — the honest "no servers" picture

The DHT needs to be entered. `hyperdht/lib/constants.js` defines exactly
**three** default bootstrap nodes, hardcoded:

```js
exports.BOOTSTRAP_NODES = global.Pear?.config.dht?.bootstrap || [
  '88.99.3.86@node1.hyperdht.org:49737',
  '142.93.90.113@node2.hyperdht.org:49737',
  '138.68.147.8@node3.hyperdht.org:49737'
]
```

Run by Holepunch ("publicly served on behalf of the commons" per the
hyperdht README). Anyone can run their own bootstrap node — `DHT.bootstrapper(port, host, ...)`
ships in the public API — and a Pear app can override the list via
`Pear.config.dht.bootstrap`. But the default install hits Holepunch
infrastructure on first contact.

The "no central servers" framing in Holepunch marketing is accurate at the
data-plane level: there is no Holepunch-operated server in the path of an
established peer-to-peer connection. It is misleading at the
control-plane level: the DHT's entry points are three Hetzner/DigitalOcean
boxes operated by Holepunch. Once you're in the DHT, the DHT itself
(thousands of peer nodes) is the discovery substrate. Until you're in, you
phoned home. This is the same shape as Iroh's relay defaults — different
mechanism, same operational dependency on the vendor's free infrastructure.
Honest comparison in [`./transport-comparison.md`](./transport-comparison.md).

## Connection establishment: the holepunch dance

Once a peer's public key is known (via the DHT lookup or out-of-band), a
direct connection is attempted. The mechanism, verified in
`hyperdht/lib/connect.js` and `hyperdht/lib/holepuncher.js`:

### Phase 1: NAT introspection

Each side asks **multiple DHT nodes** what address it appears to be coming
from. `hyperdht/lib/nat.js` is an active sampler: it pings DHT nodes
(default minimum 4 samples) and records the (`from`, `to`) pairs the DHT
reports back. From that distribution, the local peer infers its NAT class:

- `OPEN` — same public address regardless of destination.
- `CONSISTENT` — same public mapped port across different remote hosts
  (port-restricted-cone-NAT-equivalent).
- `RANDOM` — different mapped port per destination (symmetric NAT).
- `UNKNOWN` — not enough samples.

This is STUN-like in spirit — discover your NAT mapping by asking a third
party — but it does not run STUN. It uses the DHT itself as the address-
observation channel, so "NAT-detection servers" don't need to exist
separately. Compare to Iroh's QUIC Address Discovery (QAD), which uses a
dedicated QUIC frame to the relay; same idea, three different
implementations across three stacks.

### Phase 2: rendezvous + simultaneous send

The DHT nodes that answered `FIND_PEER` for the dialer also speak
`PEER_HANDSHAKE` and `PEER_HOLEPUNCH` to coordinate the actual punch.
Roughly:

1. Dialer asks the DHT for the destination's known relay nodes (these are
   regular DHT nodes that happen to have recently observed packets from the
   destination, *not* dedicated relay infrastructure — they're routing-table
   neighbors).
2. Dialer sends a `PEER_HANDSHAKE` request *via* one of those DHT nodes,
   carrying the dialer's NAT class and candidate addresses.
3. Destination receives the handshake (still over the DHT path), replies
   with its own NAT class and candidate addresses.
4. Both sides launch `Holepuncher` (`hyperdht/lib/holepuncher.js`) — they
   open up to **256 birthday sockets** (`BIRTHDAY_SOCKETS = 256`) on
   randomized local ports if the remote NAT is `RANDOM`, send low-TTL
   probe datagrams (`HOLEPUNCH_TTL = 5`) to seed mappings, and fire
   simultaneous full-TTL probes from each candidate to each candidate.
5. Whichever (local-port, remote-port) tuple gets a response becomes the
   data-plane path. A `noise-handshake` (Noise-IK pattern, verified in
   `hyperdht/lib/noise-wrap.js`) runs over that path, producing a
   `NoiseSecretStream` (`@hyperswarm/secret-stream`).

The brief said "Noise-XX-flavored." **Correction:** the peer-handshake is
**Noise-IK** (`new NoiseHandshake('IK', ...)` in `noise-wrap.js`),
because the DHT lookup gives the dialer the responder's static public key
up front — IK fits, XX would be over-pessimistic. The transport-level
`@hyperswarm/secret-stream` defaults to XX when used standalone (e.g. over
TCP without prior key knowledge), but inside hyperdht the pattern is IK.

### When holepunching can't work: blind-relay (TURN-equivalent)

The brief stated there are "no central relays like Iroh's DERP servers."
**Correction:** Hyperswarm has a relay layer, called
[`blind-relay`](https://github.com/holepunchto/blind-relay), and it is a
real dependency of `hyperdht` (verified in `hyperdht/package.json`:
`"blind-relay": "^1.3.0"`). Quoting its README:

> Blind relay for UDX over Protomux channels. By acting as a blind relay, a
> host may accept pairing requests from other hosts and relay UDX stream
> messages between them, similar to Traversal Using Relays around NAT
> (TURN).

So Hyperswarm's *full* picture is closer to Iroh's than the marketing
suggests: there is a TURN-equivalent fallback when holepunching fails. The
asymmetry vs Iroh is:

- Iroh's DERP relays are operated **by n0** as the default fallback. Every
  endpoint maintains a persistent connection to its home relay regardless
  of whether direct paths are working. Always-on.
- Hyperswarm's blind-relay is an **opt-in** path. It's invoked via
  `relayThrough` on `connect()` (verified in `hyperdht/lib/connect.js`:
  `const relayThrough = selectRelay(opts.relayThrough || null)`). There is
  no Holepunch-operated default relay fleet for blind-relay; if the app
  wants relayed connectivity, it nominates a peer (or peers) to relay
  through.

In practice this means Keet-style apps either (a) accept that hard-NAT
peers can't connect, or (b) designate community / app-operated relay nodes
the app has out-of-band knowledge of. Iroh chose to take operational
responsibility for relay availability; Holepunch chose to push it onto the
app.

## Connection authentication

The peer's identity is its **ed25519 keypair**. Specifically: the curve
used by the noise handshake is `noise-curve-ed` (Edwards-curve Curve25519
points repurposed for ECDH — same key, both sign-and-encrypt). The 32-byte
public key serves as:

- The DHT routing identifier (Kademlia XOR-distance).
- The connection's authenticated identity — `socket.remotePublicKey` after
  handshake completes.
- The server-listen address — `server.listen(keyPair)` makes the keypair's
  public key the dial target.

There is no PKI, no CA, no transitive trust. If you have peer X's 32-byte
public key, you have everything you need to dial them and verify the
connection terminated at them. Same identity model as Iroh (`EndpointId` =
ed25519 public key). Same model as Hypercore (discovery key = derived from
public key). Same model as much of the post-Bitcoin P2P design space.

The handshake additionally produces a `handshakeHash` — a unique 32-byte
hash representing this specific session. Both sides compute the same value;
applications can use it as a session ID without further coordination.

## Multiplexing: protomux

A `NoiseSecretStream` is a single bidirectional encrypted stream. Real apps
need many concurrent message channels over that stream. Enter
[`protomux`](https://github.com/mafintosh/protomux) — a custom
multi-protocol multiplexer. Each "protocol" registers as a named channel
(`protocol`, optional `id`); each protocol declares typed messages with
compact-encoding codecs. A single secretstream might be carrying Hypercore
replication, Hyperdrive metadata sync, Hyperbee key-range queries, and an
app-specific RPC, all interleaved.

protomux is not QUIC. It does not provide independent stream flow control
or head-of-line-blocking avoidance — it's strictly message-multiplexing
over an ordered byte stream. If one channel's consumer is slow, every
channel on the same secretstream is back-pressured. For Hypercore-shaped
workloads (mostly request/response with bounded message sizes) that's been
fine in practice, per the [`./pear-runtime.md`](./pear-runtime.md) deployment
record. For latency-sensitive concurrent workloads it would be a real
problem; see [`./transport-comparison.md`](./transport-comparison.md) for
the contrast with Iroh's QUIC-native streams.

## Mobile-network behavior

This is where Hyperswarm has the most production-distance signal in the
prior-art set. Keet has shipped on iOS and Android since 2023; the same
discovery+connection layer carries those clients. Documented mobile-specific
behaviors (verified in source):

- **Suspend/resume.** `swarm.suspend()` and `swarm.resume()` are
  first-class API methods (verified in `hyperswarm/README.md`). Suspend
  disconnects all peers, halts server listening, and stops new-peer
  discovery. Resume re-announces topics on the DHT and reconnects pending
  peers. The brief asked how Hyperswarm handles iOS background-mode socket
  reaping — the answer is "explicit hand-off to a kernel-style API." The
  app is responsible for calling suspend/resume around the OS lifecycle
  events; Hyperswarm itself doesn't try to keep sockets alive across
  backgrounding.
- **Network-change refresh.** `server.refresh()` is documented as
  "automatically called on network changes" — when the host's network
  interfaces change, the announcing keypair gets re-published to the DHT.
- **Wake-from-sleep.** `PeerDiscovery._refreshLater` includes an explicit
  `DELAY_GRACE_PERIOD = 30s`: if a scheduled refresh fires more than 30s
  after expected (i.e. the laptop was suspended), it eagerly re-refreshes.
  Same instinct, same magic-number range as Iroh's "schedule a relay
  rebind on suspend-detected" logic.
- **`connectionKeepAlive: 5000ms` default** on hyperdht sockets — heartbeat
  interval to keep NAT mappings warm. Same idea, similar order-of-magnitude
  to QUIC PING-frame keepalive in iroh.

What Hyperswarm doesn't have, that Iroh does: a default relay fleet that
keeps a connection viable through arbitrarily-long NAT-mapping churn.
Hyperswarm's answer is "punch again from the new mapping," which works
when both sides are still reachable but doesn't help when one side has
gone fully quiet behind a CGNAT change.

## Performance shape

No published benchmarks in the Holepunch docs as of 2026-05-09 — the
hyperdht README and docs.pears.com don't quote success rates or latency
numbers. Indicative properties from the source:

- **Discovery latency.** Topic-lookup is a Kademlia query, `O(log N)`
  hops. At Keet's low-tens-of-thousands MAU class (see README) the DHT has
  on the order of tens of thousands of active nodes; a fresh lookup
  typically completes in 1-3 round-trips
  (couple-hundred milliseconds). Stale topic announces (10-minute refresh)
  mean a peer that just came online may not be in the lookup result for up
  to that interval.
- **Connection latency.** Holepunch dance involves multiple DHT roundtrips
  before the simultaneous-send phase. End-to-end "I called `swarm.join`
  to a connection emitted on `swarm.on('connection')`" is dominated by DHT
  query time when the remote is already announced; sub-second when the
  network cooperates, up to several seconds when it doesn't.
- **Bandwidth.** Once direct, bandwidth is whatever UDX gives you over the
  punched UDP path. UDX is a userland reliable-stream protocol (Holepunch's
  alternative to QUIC); throughput is competitive with TCP on uncongested
  networks. Through blind-relay, throughput is bounded by the relay node's
  upstream.

## What's not in the box

- **No TCP fallback today.** The brief said "falling back to TCP if UDP
  fails." **Correction:** the current `hyperdht` does not include a TCP
  fallback path. Older versions (pre-v5) experimented with utp/tcp duals;
  current production is UDP (UDX) plus blind-relay (also UDP-over-Protomux,
  not TCP). If UDP is fully blocked at the network edge, Hyperswarm does
  not connect. Iroh's QUIC-only stack has the same constraint (QUIC is UDP);
  the difference is that Iroh's relay path is HTTPS, which traverses
  HTTPS-only networks that block UDP entirely. Hyperswarm has no equivalent
  HTTPS-tunnel fallback.
- **No browser support out of the box.** `hyperdht` ships a `browser.js`
  but it's a stub — the DHT can't run from a browser context (no UDP).
  Browser integration is via gateway/relay nodes, app-specific.

## Implications for Myrhiza

Myrhiza picks Iroh, not Hyperswarm. The Hyperswarm body of work is still
useful as input to the kernel-network capability spec because it represents
the most mature alternative answer to the same problem and surfaces
constraints worth designing against:

- **The "DHT-as-rendezvous" pattern works at scale.** Keet ships on it;
  Hypercore replication runs on it. If a future Myrhiza spec wanted a
  DHT-style discovery cap (alternative to or alongside Iroh's
  `iroh-discovery`), the hyperdht commands surface — `LOOKUP` /
  `ANNOUNCE` / mutable + immutable records — is a battle-tested template.
- **NAT-class detection should not leak into apps.** Hyperswarm exposes
  `peerInfo` but not raw firewall classification. Apps see "you got a
  connection" or "you didn't." This is the same kernel-cap shape Myrhiza
  should adopt — see [`../iroh/nat-traversal.md`](../iroh/nat-traversal.md)
  for the equivalent argument framed against `Connection::paths()`.
- **Suspend/resume must be a first-class capability.** Hyperswarm's
  explicit `suspend()`/`resume()` is a real lesson: Keet shipped on iOS,
  iOS reaps backgrounded sockets, the only way to survive that cleanly is
  to expose the lifecycle to the application. The Myrhiza kernel cap-shape
  for the network must include lifecycle events; an app that holds open
  network connections must be able to be told "we're going dark, drop your
  state cleanly" by the host.
- **The "no central servers" claim deserves the same scrutiny applied to
  Iroh's relays.** Hyperswarm needs Holepunch's three bootstrap nodes to
  enter the DHT; Iroh needs n0's relay fleet to keep hard-NAT peers
  reachable. Both rhetorical positions ("we run no servers") and ("we run
  some servers, transparently") have honest framings; neither is "no
  infrastructure dependency at all." Myrhiza specs should not pretend
  otherwise. See [`./transport-comparison.md`](./transport-comparison.md)
  for the side-by-side.
- **Determinism boundary is identical.** Topic discovery, NAT class,
  relay-vs-direct, latency to peer — all non-deterministic. None of it can
  cross into `state-apply`. The host-import surface for "open connection
  to peer X" should return only (peer-public-key, encrypted-bytes-stream);
  the kernel handles the rest.

## See also

- [`./pear-runtime.md`](./pear-runtime.md) — the application runtime that
  rides on Hyperswarm.
- [`./transport-comparison.md`](./transport-comparison.md) — direct
  Iroh-vs-Hyperswarm engineering comparison.
- [`../iroh/nat-traversal.md`](../iroh/nat-traversal.md) — Iroh's
  DERP-based NAT-traversal story.
- [`../iroh/transports.md`](../iroh/transports.md) — Iroh's QUIC / noq
  transport substrate.
- [`../holochain/`](../holochain/) — Holochain piggybacks on Iroh's relay
  fleet rather than running its own; useful third data point.
- [`../wasmcloud/`](../wasmcloud/) — NATS-based control plane is a
  centralized-by-design contrast to both Hyperswarm and Iroh.
