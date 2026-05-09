**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — endpoint API and core architecture

Iroh is a Rust P2P stack from [Number 0](https://www.iroh.computer/) (n0). The headline product is "dial a peer by its public key, get an authenticated bidirectional QUIC connection" — every other feature in the workspace is built around that primitive. As of `1.0.0-rc.0` (released 2026-05-07, [release list](https://github.com/n0-computer/iroh/releases)) the central object is the `Endpoint`, and the fundamental unit of trust is a 32-byte Ed25519 public key called an `EndpointId` (renamed from `NodeId` in 0.94, [iroh 0.94.0 — The Endpoint Takeover](https://www.iroh.computer/blog/iroh-0-94-0-the-endpoint-takeover)).

## Crate split

The workspace ([n0-computer/iroh](https://github.com/n0-computer/iroh)) ships five crates as of 0.98 / 1.0-rc.0 (verified against `Cargo.toml` on `main`, 2026-05-08):

| Crate | Role |
|---|---|
| `iroh` | Core library — `Endpoint`, `Connection`, dial / accept loop, hole-punching, relay client |
| `iroh-base` | Shared primitives — `Hash`, key types, `RelayUrl`, `EndpointTicket` (moved here in 0.95) |
| `iroh-dns` | Client-side DNS resolver for endpoint-ID discovery |
| `iroh-dns-server` | DNS server backing the n0 endpoint-ID discovery service |
| `iroh-relay` | Relay server implementation + client protocol module ([docs.rs/iroh-relay](https://docs.rs/iroh-relay/latest/iroh_relay/)) |

There is **no** `iroh-net` crate as of 0.98. Earlier iroh shipped a separate `iroh-net` containing what is now in `iroh`; it was folded into the top-level crate in **0.29 (Dec 2024)** — the "Net is the new iroh" rename ([0.29 blog post](https://www.iroh.computer/blog/iroh-0-29-net-is-the-new-iroh)) — and the workspace was further consolidated during the 0.90 "Canary Series" reorg in mid-2025. If you read older docs that talk about `iroh_net::MagicEndpoint`, that's now `iroh::Endpoint`. The standalone NAT-class-probe `iroh-net-report` published earlier — and referenced in some older write-ups — is no longer part of this workspace. Iroh's QUIC dependency was its own fork of [Quinn](https://github.com/quinn-rs/quinn); that fork has now graduated to [`noq`](https://www.iroh.computer/blog/noq-announcement), a separate top-level project — see [`./transports.md`](./transports.md).

## The `Endpoint`

The `Endpoint` is what `Conductor` is to Holochain — the long-running object that owns sockets, keys, and the connection registry. One `Endpoint` per process is the expected shape; multiple are allowed but rarely useful. Construction goes through a builder ([Endpoint docs](https://docs.rs/iroh/latest/iroh/endpoint/struct.Endpoint.html)):

```rust
let endpoint = Endpoint::builder(presets::N0)
    .alpns(vec![MY_ALPN.to_vec()])
    .bind()
    .await?;
```

`presets::N0` configures the n0-operated relay map and discovery (DNS + pkarr). `Endpoint::empty_builder(RelayMode::Disabled)` is the offline-friendly variant; you wire in your own relay map and discovery service from there. The `Endpoint::insert_relay` / `Endpoint::remove_relay` calls let you mutate the relay map after binding (added in 0.94).

What the endpoint owns:

- **Identity.** An Ed25519 keypair. The public key *is* the `EndpointId`; there is no separate certificate authority. TLS certs presented during the QUIC handshake are self-signed by that key.
- **Sockets.** One IPv4 and one IPv6 UDP socket by default, plus a long-lived TCP/TLS connection to each home relay.
- **Discovery.** A pluggable trait that maps `EndpointId → EndpointAddr` (the address bundle: relay URL plus zero-or-more direct UDP socket addresses). The default n0 preset uses pkarr-over-DNS — endpoints publish signed records into the n0 DNS server, peers query by ID.
- **ALPN registry.** The list of protocol names this endpoint will accept on incoming connections. Mutable post-bind via `set_alpns`.

## Connection lifecycle

```
Dialer                                         Acceptor
  |                                                |
  |-- Endpoint::connect(EndpointAddr, alpn) ----   |
  |   1. Resolve EndpointAddr (discovery)          |
  |   2. Open relay path immediately               |
  |   3. Probe direct UDP candidates in parallel   |
  |   4. QUIC handshake (TLS 1.3 + ALPN)           |
  |      ALPN must match a registered handler -----|
  |   5. NAT-traversal frames piggyback (see       |
  |      ./nat-traversal.md) to upgrade to direct  |
  |      path when reachable                       |
  |                                                |
  |==== Connection (multiplexed QUIC streams) =====|
```

`Endpoint::connect(addr, alpn)` returns a `Connection` once any path (relay or direct) completes the QUIC handshake; the path-upgrade machinery then races, and `Connection::paths()` exposes a watcher of the currently selected path ([iroh 0.96.0](https://www.iroh.computer/blog/iroh-0-96-0-the-quic-multipaths-to-1-0)). Streams are standard QUIC streams: `connection.open_bi().await?` for a bidirectional pair (`SendStream`, `RecvStream`), `open_uni`, `accept_bi`, `accept_uni`. There is no message framing on top — applications layer their own (length-prefix or postcard or whatever).

## ALPN-based protocol multiplexing

Iroh's only multiplexing primitive is QUIC's standard ALPN. One endpoint registers N protocol names (`b"my-app/v1"`, `b"iroh-blobs/0"`, …); the client passes an ALPN to `connect`; the TLS handshake refuses if the server didn't register that name. This is identical to how HTTP/2 vs HTTP/3 vs QUIC-over-anything-else negotiate.

The accept side is `Endpoint::accept().await` returning an `Incoming`, which yields `(Connection, alpn)`. The convention — codified in the `iroh::protocol` module — is a `ProtocolHandler` trait per ALPN, registered on a `Router`:

```rust
let router = Router::builder(endpoint)
    .accept(MY_ALPN, my_handler)
    .accept(BLOBS_ALPN, blobs_handler)
    .spawn();
```

Two practical consequences:

1. ALPN bytes are an unversioned namespace that you, the application author, own. n0's convention is `b"<name>/<version>"`, but nothing enforces it. Bumping the version requires deciding whether the old endpoint accepts both during a rollout.
2. There is no in-band capability negotiation beyond ALPN match/no-match. If a peer needs feature flags, they go in your application protocol, not in the iroh handshake.

## EndpointAddr and tickets

`EndpointAddr` is `(EndpointId, Vec<TransportAddr>)` where `TransportAddr` is currently `Udp(SocketAddr) | Relay(RelayUrl)` (enum-shaped to admit future transports — see [`./transports.md`](./transports.md) on the custom-transports work). An `EndpointTicket` is the serialized form, base32-encoded for human sharing — the iroh equivalent of "here's a URL you can dial." Tickets are not tokens, they're addresses; possession of one does not grant any permission, only the ability to attempt a connection.

## Error model

Iroh ships its error types via [`n0-error`](https://crates.io/crates/n0-error), an n0-internal error-handling crate that produces `snafu`-style errors with explicit context. The shapes a Myrhiza host calling `Endpoint::connect` will actually see:

- **`ConnectError`** — connection-establishment failures: unresolvable `EndpointId`, no relay reachable, TLS handshake refused (typically ALPN mismatch), QUIC handshake failure. Generally retryable but the underlying cause matters — ALPN mismatch is permanent for a given pair of registered ALPNs, no-relay-reachable is transient.
- **`ConnectionError`** — failures on a live `Connection`: peer-initiated close, idle timeout, application-level reset, transport reset (path failure with no fallback). The 0.95 error-handling overhaul tightened these into non-exhaustive enums; downstream code must accept that variants will be added in future minors.
- **Per-stream errors** — `WriteError` and `ReadError` on `SendStream` / `RecvStream` mirror QUIC's stream semantics: peer reset with arbitrary reset code, connection-level closure visible as a stream error, idle timeout.

Two Myrhiza-relevant gotchas:

1. **Errors carry transport-state metadata.** Default error displays for `ConnectionError` may include relay URL, path-id, even peer-observed addresses. The kernel should sanitize errors before they cross the WASM boundary into apps — apps see "connection failed" plus an opaque retry hint, not a free dump of network state. Otherwise a malicious app exfiltrates topology by triggering errors and reading them.
2. **Retryability is not a typed property.** Iroh does not encode "retry vs don't retry" in the error type; the host must apply its own classification (and re-classify when iroh's variants change). For state-apply purity this is a feature: retry policy lives in `behavior` components, not `state-apply`.

## Implications for Myrhiza

The kernel's network capability sits at `Endpoint`-level, not below it. A capability handle hands an app the ability to (a) accept on a specific ALPN, (b) dial out by `EndpointId`, and (c) read/write streams — never the raw socket, never the keypair, never the discovery service. That maps cleanly onto a single shared `Endpoint` per host with per-app `Router` registrations: the kernel owns the `Endpoint`, decides which ALPNs each WASM bundle gets to register, and proxies streams across the WASM boundary. The `EndpointId` is the only naming primitive the runtime needs to commit to permanently — it's a 32-byte Ed25519 public key with no embedded version, so a future protocol change won't require renaming peers. The renaming churn iroh did in 0.94 (Node→Endpoint) is a useful warning: anchor Myrhiza specs against the *concept* (a peer keypair-identified endpoint) and import iroh's current names as terminology, not as load-bearing API surface.

## Sources

- [iroh GitHub repository](https://github.com/n0-computer/iroh)
- [iroh release list](https://github.com/n0-computer/iroh/releases)
- [iroh 0.94.0 — The Endpoint Takeover](https://www.iroh.computer/blog/iroh-0-94-0-the-endpoint-takeover)
- [iroh 0.96.0 — The QUIC Multipaths to 1.0](https://www.iroh.computer/blog/iroh-0-96-0-the-quic-multipaths-to-1-0)
- [Endpoint API on docs.rs](https://docs.rs/iroh/latest/iroh/endpoint/struct.Endpoint.html)
- [iroh-relay on docs.rs](https://docs.rs/iroh-relay/latest/iroh_relay/)
- [Number 0 (iroh.computer)](https://www.iroh.computer/)
- [Quinn QUIC implementation](https://github.com/quinn-rs/quinn)
