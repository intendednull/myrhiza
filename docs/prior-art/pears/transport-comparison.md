**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — Hyperswarm vs Iroh: direct transport comparison

# Transport comparison: Hyperswarm vs Iroh

Myrhiza commits to [Iroh](../iroh/) as the runtime's transport. Hyperswarm
is the most mature competing answer in the same problem space and the only
other P2P transport in this prior-art set with a real consumer-mobile
deployment behind it (Keet, see [`./pear-runtime.md`](./pear-runtime.md)).
This file is a side-by-side on the engineering choices, focused on
discovery, NAT traversal, wire protocol, and mobile-production reality.

For Hyperswarm internals see [`./hyperswarm.md`](./hyperswarm.md). For
Iroh internals see [`../iroh/nat-traversal.md`](../iroh/nat-traversal.md)
and [`../iroh/transports.md`](../iroh/transports.md).

## Snapshot

| Axis | Hyperswarm | Iroh |
|---|---|---|
| Language | JavaScript (also Bare-runtime) | Rust |
| License | MIT | Apache-2.0 |
| Identity | ed25519 keypair | ed25519 keypair (`EndpointId`) |
| Discovery | `hyperdht` Kademlia DHT, custom commands | `iroh-discovery`: pkarr DNS + mainline DHT + local mDNS |
| NAT detection | DHT nodes as STUN-equivalent | QUIC Address Discovery (QAD) frame to relay |
| Holepunching | Coordinated via DHT side channel | Coordinated via QUIC NAT-traversal frames over relay |
| Relay fallback | `blind-relay` (TURN-equivalent), opt-in, no default fleet | DERP relays (HTTP/WebSocket), always-on, default fleet by n0 |
| Wire protocol | UDX over UDP + custom `protomux` framing | QUIC (noq, fork of Quinn) |
| Stream multiplexing | `protomux` (no per-channel flow control) | QUIC streams (independent flow control, no head-of-line blocking) |
| Encryption | Noise-IK + libsodium secretstream | TLS 1.3 (QUIC's built-in) over Noise-XX-equivalent identity |
| Browser story | None (DHT can't run in browser) | Alpha since 0.32, WebTransport-backed |
| Mobile production | Keet on iOS + Android, low-tens-of-thousands MAU class (see README honest-scale disclosure) | Delta Chat on Android, smaller iOS apps; younger here |
| Spec stability | No published wire spec; behavior frozen by Holepunch | No published wire spec; QUIC NAT-traversal IETF draft expired 2024-09 |

Both are fast-moving codebases with a single dominant maintainer
organization and no IETF-standardized wire format. Treat both as
engineering substrates, not as protocols you can reimplement against a
spec.

## Discovery

Both stacks discover peers via DHT. They differ in the fallback story.

### Hyperswarm

Single mechanism: `hyperdht`. A Kademlia-flavored DHT with topic-announce
and topic-lookup commands (`ANNOUNCE` / `LOOKUP`, verified in
`hyperdht/lib/constants.js`). Three Holepunch-operated bootstrap nodes are
the entry points (`node1`/`node2`/`node3.hyperdht.org` at port 49737).
Once you're in the DHT, you query for the 32-byte topic hash and get back
recently-announcing peers. There is no DNS-based fallback, no mDNS path,
no PEX (peer-exchange) layer above the DHT. The DHT is the discovery.

### Iroh

Multiple, layered mechanisms in `iroh-discovery`:

- **pkarr** — public-key-addressable records over DNS. An endpoint
  publishes its current relay/direct addresses to a pkarr server (default
  `dns.iroh.link`, n0-operated; community pkarr servers exist). Peers
  resolve `_iroh.<endpoint-id>.dns.iroh.link` over DNS-over-HTTPS. This is
  the dominant resolution path in production: cheap, cached, works in
  restrictive networks that allow DNS but block raw UDP.
- **mainline DHT** — Iroh announces and looks up endpoint records on the
  BitTorrent mainline DHT, a much larger DHT than hyperdht (millions of
  active nodes vs Hyperswarm's tens of thousands). Robust because it has
  no single owner; slow because it has no single owner.
- **mDNS** — local-network discovery for same-LAN peers. Hyperswarm has no
  built-in mDNS path (the local-LAN case for Hypercore is handled via the
  DHT — both peers are still public-Internet nodes that happen to also be
  reachable on the LAN).

The asymmetry: Iroh has bet on multiple-path redundancy (DNS + DHT + mDNS
fall through to one another); Hyperswarm has bet on one robust DHT.

## NAT traversal

This is the load-bearing comparison.

### Hyperswarm: punch-and-pray, with optional TURN

Phase 1 (NAT introspection): the local peer pings DHT nodes (default 4+
samples) and observes the (`from`, `to`) addresses they report. From the
distribution, classify the local NAT as `OPEN` / `CONSISTENT` /
`RANDOM` / `UNKNOWN`. Same idea as STUN; uses the DHT itself as the
observation channel.

Phase 2 (rendezvous): dialer asks DHT nodes that know the destination to
relay a `PEER_HANDSHAKE` and `PEER_HOLEPUNCH` exchange. Both sides learn
each other's NAT class and candidate addresses.

Phase 3 (simultaneous send): both peers fire low-TTL probe packets to seed
NAT mappings, then full-TTL probes from each candidate to each candidate.
Up to 256 birthday-attack sockets when one side is symmetric. Whichever
(local-port, remote-port) tuple gets a response wins; Noise-IK runs over
that path.

If holepunching fails: `blind-relay` is available as a TURN-equivalent
fallback (verified: `hyperdht/package.json` depends on `blind-relay@^1.3.0`),
but **there is no default Holepunch-operated relay fleet**. Apps either
designate community-run relays via `relayThrough`, accept that hard-NAT
peers can't connect, or retry holepunching with more aggressive
parameters. Keet's behavior in CGNAT environments is "occasionally fails
to deliver messages until the network changes."

### Iroh: relay-first, holepunch in the background

Iroh's flow inverts the priority. Every endpoint maintains a persistent
WebSocket to a "home" DERP relay (default fleet: 4 n0-operated relays).
When you `Endpoint::connect(peer-id)`:

1. The connection opens **through the relay first** — `connect()` returns a
   usable `Connection` as soon as the relay path validates. Application
   code can start sending data immediately.
2. In parallel, multipath QUIC tries to validate a direct path. NAT-traversal
   frames (`REACH_OUT`, `PATH_CHALLENGE`/`PATH_RESPONSE`, drawn from the
   expired `draft-seemann-quic-nat-traversal-02`) travel inside the QUIC
   connection itself, over the relay path, to coordinate candidate-address
   exchange.
3. If a direct path validates, the multipath stack prefers it; the relay
   keeps a control-plane heartbeat but stops carrying data.
4. If no direct path validates (symmetric CGNAT, restrictive corporate
   firewalls, browser peers), the relay continues to be the data plane,
   indefinitely.

NAT introspection is **QUIC Address Discovery (QAD)** — a special QUIC
frame asks the relay "what address did you observe me at?" Same effect as
STUN, no separate STUN service.

### The asymmetry

Iroh accepts that some networks are fundamentally unrelayable (symmetric
NAT to symmetric NAT, browsers, locked-down corporate) and provides a
relay path that always works at the cost of higher latency and bounded
throughput. Hyperswarm bets that with enough holepunching tries — birthday
sockets, randomized TTLs, retry storms — you can punch most networks, and
that the rest are an app-level problem.

Both bets are partially correct. Iroh's bet means even users on the most
hostile networks can transact, but every connection goes through n0
infrastructure for at least its early lifetime. Hyperswarm's bet means
infrastructure cost is genuinely zero in the data plane, but users on
hostile networks see "couldn't connect" intermittent failures with no
clear remediation other than "switch networks."

For Myrhiza's mobile target (consumer phones, often on cellular CGNAT),
Iroh's bet is the safer engineering call. For Keet-shaped use cases
(message delivery is asynchronous-tolerant, the app retries on the next
network change), Hyperswarm's bet has been good enough.

## Wire protocol

### Hyperswarm

Custom binary protocol. The transport is **UDX** (Holepunch's reliable-stream
over UDP — their alternative to QUIC; userland congestion control,
out-of-order delivery, custom retransmit). On top of UDX runs a
`NoiseSecretStream` (Noise-IK handshake + libsodium secretstream encryption).
On top of secretstream, **`protomux`** multiplexes multiple message-oriented
protocols.

protomux is not a stream multiplexer in the QUIC sense. It does **not**
provide per-channel flow control or head-of-line-blocking avoidance.
Channels share the underlying stream's send queue: a slow consumer on
channel A back-pressures channels B, C, D. For Hypercore-shaped workloads
(small request/response messages, bounded queue depths) this is fine. For
heterogeneous workloads (a large blob transfer concurrent with low-latency
RPC) it would be a real problem.

### Iroh

QUIC, via [noq](https://github.com/n0-computer/noq) (formerly
`iroh-quinn`, a soft fork of Quinn). QUIC gives:

- **Independent stream flow control.** Each stream has its own credit;
  slow consumers don't back-pressure other streams.
- **TLS 1.3 native.** Encryption is QUIC-native, not bolted on; the
  handshake is interleaved with stream open.
- **Multipath as of 0.96.** One connection can hold multiple network paths
  simultaneously (Wi-Fi + cellular). Path migration on network change is
  graceful.
- **0-RTT connection resumption.** When peers have spoken before, the
  second connection is materially faster.

The cost: noq is young and divergent from upstream Quinn; iroh has shipped
correctness regressions in its multipath implementation as recently as
0.98 (see [`../iroh/transports.md`](../iroh/transports.md)).

### The asymmetry

Iroh inherits HTTP/3-class transport features (multiplexing, multipath,
0-RTT, congestion control) for free by sitting on QUIC. Hyperswarm rolled
its own (UDX + protomux + secretstream) and has the simpler primitives but
fewer features. Each layer of the Hyperswarm stack is small and
inspectable; the QUIC stack is enormous and full of security-sensitive
state machines. There's a genuine tradeoff in engineering surface area —
Hyperswarm's stack you can read end-to-end in a weekend, noq you cannot.

## Connection security

Both stacks authenticate connections against an ed25519 keypair. Both use
a Noise-flavored handshake.

- **Hyperswarm.** Noise-IK pattern (verified in
  `hyperdht/lib/noise-wrap.js`: `new NoiseHandshake('IK', ...)`), curve
  `noise-curve-ed`. Once handshake completes, payload is encrypted via
  libsodium's `secretstream` primitive. The handshake produces a
  `handshakeHash` both sides can use as a session ID without further
  coordination.
- **Iroh.** TLS 1.3 over QUIC, with the cert presenting the endpoint's
  ed25519 public key. The handshake is QUIC-native; ALPN selection
  identifies the application protocol on top.

Both are end-to-end-encrypted between exactly the two endpoints. Neither
relays in either stack can read the inner traffic — `blind-relay` is
explicitly named because it's blind (UDX stream messages pass through
ciphertext-opaque); DERP relays forward ciphertext-opaque QUIC datagrams.

For Myrhiza's threat model both are equivalent. For interop, neither is.

## Mobile production

This is where the rubber meets the road and where Hyperswarm currently
has more data.

### Hyperswarm / Keet

Keet has shipped on iOS and Android since 2023. Userbase is in the hundreds
of thousands (Holepunch hasn't published exact numbers; this is an
order-of-magnitude estimate from app-store rankings and public statements).
Three years of production on this stack means:

- iOS background-mode socket reaping has been engineered around. The
  `swarm.suspend()` / `swarm.resume()` API surface is the lesson learned.
- CGNAT failure modes are characterized and gracefully degraded — Keet's
  in-app behavior on a hard NAT is "messages will deliver when the network
  cooperates," not "the app stops working."
- Battery-life impact has been tuned. `connectionKeepAlive` defaults to
  5000ms; `randomPunchInterval` defaults to 20000ms. These are not
  arbitrary; they're tuned numbers from real-device telemetry.

The downside: Keet itself has no published engineering blog cataloguing
these tradeoffs, so much of the lesson is encoded only in the source's
default constants and the API shapes (suspend/resume, refresh on
network change). For a Myrhiza spec author, this means much of the Keet
lesson is recoverable only by reading hyperdht source directly.

### Iroh / Delta Chat and friends

Iroh is younger in mobile production. Delta Chat on Android uses iroh as
its transport for the IROH-only mode (alongside SMTP/IMAP); a few smaller
iOS P2P apps ship iroh via the [iroh-ffi](https://github.com/n0-computer/iroh-ffi)
bindings. The user-base is materially smaller than Keet's.

Iroh has invested heavily in the relay-as-mobile-fallback story
specifically because mobile is hostile, and the engineering bets show:

- Multipath QUIC means cellular ↔ Wi-Fi handover is graceful (post-0.98
  fix).
- DERP relays mean even on the most hostile cellular CGNAT, peers can
  always reach each other via the relay path. Keet's "occasionally
  fails to deliver until the network changes" mode does not happen with
  iroh — it just runs slower through the relay.

But: production-distance is shorter. iroh has shipped correctness
regressions in multipath holepunching as recently as 0.98 (April 2026).
Keet has been live for three years on Hyperswarm; iroh-ffi-based mobile
apps have been live for under a year. The engineering bet on iroh is
"the architecture is right, the implementation will mature." Hyperswarm's
position is "the architecture is more limited but the implementation is
mature."

## The "no servers" rhetoric vs reality

Both stacks describe themselves as serverless P2P. Both are true at the
data-plane level. Both are misleading at the control-plane level.

| | Hyperswarm | Iroh |
|---|---|---|
| Default-install dependency | 3 hardcoded bootstrap nodes (`*.hyperdht.org`) | 4 default DERP relays (n0-operated) + pkarr DNS server (`dns.iroh.link`) |
| Operator | Holepunch | n0 |
| Runs in data path? | No (bootstrap is one-shot to enter DHT) | Yes (relay carries data when no direct path) |
| Self-hosted alternative? | `DHT.bootstrapper(port, host)`; override via `Pear.config.dht.bootstrap` | `iroh-relay` is a library; `RelayMap` configurable per-Endpoint; pkarr is open protocol |
| Auth/access-control? | None — anyone can bootstrap, anyone can relay through any other peer | DERP relays support optional auth tokens since 0.98 |
| Censorship resistance? | DHT itself has thousands of nodes once entered; bootstrap nodes are the choke point | Relay fleet is the choke point (data plane); pkarr DNS is the choke point (discovery) |

Both projects have honest framings available — "we run some infrastructure,
transparently, you can self-host" — and both have marketing framings that
are less honest. Myrhiza specs should not amplify either framing.

The deepest difference: **Iroh runs servers in the data path; Hyperswarm
runs servers only in the control path** (for the bootstrap moment).
Iroh's relays carry traffic when holepunching fails. Hyperswarm's
bootstrap nodes are touched on first contact and during DHT-routing-table
rebuilds, but not during steady-state communication. From a censorship-
or trust-pressure perspective, this matters: an adversary who can pressure
Holepunch to take down their bootstrap nodes hurts new-peer entry but
doesn't immediately affect existing peer-to-peer connections; an adversary
who can pressure n0 to take down DERP relays may break ongoing user
sessions on hard NATs.

For Myrhiza, choosing Iroh means inheriting that pressure surface. The
kernel-network-cap spec should let the deploying organization swap the
relay map at deploy time (Iroh's API supports this; the kernel just needs
to expose it).

## Implications for Myrhiza

Myrhiza picks Iroh. The Hyperswarm comparison validates and informs the
choice:

- **Iroh's relay-first stance is the right engineering call for a runtime
  that cares about always-works-eventually semantics.** The Keet experience
  shows pure holepunching CAN ship to consumer iOS/Android at low-tens-of-
  thousands MAU class, but with persistent tail-latency for hard NATs that
  shows up as "this app is flaky on cell networks." Myrhiza's host imports
  should not expose apps to that tail. DERP fills the gap.
- **The "phase-1 + phase-2 holepunch" pattern is well-understood.** Both
  stacks implement it; it works; the differences are in the rendezvous
  channel (DHT for Hyperswarm, relay-WebSocket for Iroh) and the
  timing-orchestration (DHT-coordinated PEER_HOLEPUNCH for Hyperswarm,
  in-QUIC NAT-traversal frames for Iroh). Myrhiza doesn't need to design
  this pattern; it needs to consume Iroh's implementation cleanly.
- **Suspend/resume must be a first-class capability.** Hyperswarm's
  `swarm.suspend()`/`swarm.resume()` is a hard-won lesson from iOS
  background-mode socket reaping. Iroh's API is less explicit here —
  you can `endpoint.close()` and rebuild — but the kernel cap-shape for
  Myrhiza should expose lifecycle events (`going-background`,
  `coming-foreground`) regardless of which transport is underneath. Apps
  that hold network state need to be notified.
- **NAT-class detection is kernel-private.** Hyperswarm doesn't expose
  raw NAT class to apps; Iroh has `Connection::paths()` which technically
  could leak relay-vs-direct status. Both should be hidden from the
  WASM-app surface. See [`../iroh/nat-traversal.md`](../iroh/nat-traversal.md)
  for the equivalent argument framed against Iroh's API.
- **The relay fleet is operationally trusted, not cryptographically
  trusted.** Same shape in both stacks. The kernel-network-cap spec
  should make the relay map (or DHT bootstrap list, in the unlikely event
  Myrhiza grows a hyperdht-style discovery cap) a kernel-policy parameter,
  not an app-tunable knob. Apps should not be able to choose their own
  relay; that's an operator decision.
- **Determinism boundary is identical for both.** Topic discovery, NAT
  class, relay-vs-direct, latency, path migration — none of it can leak
  into `state-apply`. The host-import surface for "open connection to peer
  X" returns (peer-public-key, encrypted-bytes-stream); the kernel handles
  everything else. This is true regardless of which transport Myrhiza
  picks; Iroh just makes it cleaner because the hidden state is more
  comprehensively encapsulated by the QUIC connection abstraction.

## See also

- [`./hyperswarm.md`](./hyperswarm.md) — full Hyperswarm internals.
- [`./pear-runtime.md`](./pear-runtime.md) — the application runtime
  riding on Hyperswarm.
- [`../iroh/nat-traversal.md`](../iroh/nat-traversal.md) — Iroh's
  DERP-based NAT-traversal mechanism.
- [`../iroh/transports.md`](../iroh/transports.md) — Iroh's QUIC / noq
  transport substrate.
- [`../iroh/architecture.md`](../iroh/architecture.md) — Iroh's `Endpoint`
  API and the surface Myrhiza will sit on.
- [`../holochain/`](../holochain/) — Holochain piggybacks on n0's relay
  fleet rather than running its own; the explicit "we trust n0
  operationally" deployment.
- [`../wasmcloud/`](../wasmcloud/) — NATS-based control plane is a
  centralized-by-design contrast to both Hyperswarm and Iroh.
