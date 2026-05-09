**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — transport substrate (QUIC, noq, custom transports, browser viability)

Iroh has always run over QUIC. What's changed across the 0.95 → 1.0 line is *which* QUIC implementation, how many paths run concurrently, and what counts as a "transport" at all. See [`./architecture.md`](./architecture.md) for the `Endpoint` API; see [`./nat-traversal.md`](./nat-traversal.md) for how QUIC frames carry the hole-punching coordination.

## QUIC implementation: quinn → noq

For most of iroh's history, the QUIC layer was [Quinn](https://github.com/quinn-rs/quinn), maintained as the soft fork [`iroh-quinn`](https://crates.io/crates/iroh-quinn). The fork existed because iroh kept needing changes Quinn couldn't accommodate — most importantly multipath and a way for QUIC frames to carry NAT-traversal coordination.

In 0.96 (2026-01-28, [iroh 0.96.0 — The QUIC Multipaths to 1.0](https://www.iroh.computer/blog/iroh-0-96-0-the-quic-multipaths-to-1-0)) the fork started carrying real divergence — multipath integrated into the QUIC connection state machine, and NAT-traversal frames as first-class transport concepts rather than a side channel. In 0.97 (2026-03-16, [iroh 0.97.0 — Custom Transports & noq](https://www.iroh.computer/blog/iroh-0-97-0-custom-transports-and-noq)) the fork was renamed and graduated into its own project: **noq**, "number 0 QUIC." The migration from `iroh_quinn` was mechanical — a search-and-replace to `noq` and `noq_proto` — but the projects have separate maintainers from 0.97 onward. The standalone announcement landed three days later in [noq, noq, who's there?](https://www.iroh.computer/blog/noq-announcement) (2026-03-19).

What noq is, honestly:

- **A divergent fork, not a wrapper.** The codebase started from Quinn but has its own direction. Per the announcement: "the problems we're solving are specific enough that a separate codebase, with collaboration where our interests overlap, is the honest path forward."
- **Young.** The 0.18 release referenced by iroh 0.98 ([iroh 0.98.0 release notes](https://github.com/n0-computer/iroh/releases)) fixed a real correctness bug — "holepunching frames no longer get stuck behind stream data" — which says the multipath/NAT-traversal scheduler was still being tuned in production through April 2026.
- **Bound to iroh's needs.** noq adds multipath and NAT traversal because iroh wants them; a generic QUIC user picking noq over Quinn buys those features and inherits noq's release cadence.

Going forward you depend on `noq` (and `noq_proto` for the sans-IO codec layer) wherever you'd previously have depended on Quinn types through iroh's re-exports.

## Multipath QUIC

Multipath landed in 0.96 ([iroh 0.96.0](https://www.iroh.computer/blog/iroh-0-96-0-the-quic-multipaths-to-1-0)). One QUIC connection can hold multiple network paths simultaneously — the canonical case is "I'm on Wi-Fi and on cellular at the same time, both routes are validated, the connection picks the best." Internally each path has its own path identifier, congestion controller, and `PATH_CHALLENGE` validation; iroh selects among them and exposes the selection plus the underlying set via `Connection::paths()`, a watcher that fires on open/close/select-change.

The user-visible API is intentionally small: applications don't pin a path, the stack picks one. Two real limitations as of 0.96:

1. **Holepunching is not re-triggered when network conditions change in most circumstances.** If your direct path drops because you switched networks, traffic falls back to the relay and stays there. This was the regression that 0.98 ([iroh 0.98.0 — Getting back to traversing NATs](https://github.com/n0-computer/iroh/releases)) was specifically released to fix; the fix landed alongside `noq@0.18`.
2. **Multipath APIs are early.** The 0.96 announcement explicitly invites users to request additional functionality — "the implementation is still young."

The mobile use case (cellular ↔ Wi-Fi handover without dropping the connection) is the obvious payoff. As of 0.98 it works; before 0.98 the path would stall. Treat it as "real but recently-stabilized" and budget for behavioral surprises on flaky networks.

## Custom transports (experimental)

0.97 introduced an experimental hook to plug non-UDP transports into the same `Endpoint` ([iroh 0.97.0](https://www.iroh.computer/blog/iroh-0-97-0-custom-transports-and-noq)):

```rust
Endpoint::builder(presets::N0)
    .add_custom_transport(my_transport)
    .bind()
    .await?;
```

The transport implements traits for low-level packet send/recv with a minimum 1200-byte packet size (QUIC's standard minimum-MTU floor). Use cases the announcement gives: Bluetooth, custom WebRTC, embedded link layers. The honest framing from the same post: **"unstable and will remain so for some time even after iroh 1.0."** A second associated change — "Endpoint no longer makes any best-effort to close connections gracefully on drop" — means custom-transport users (and everyone else) must call `endpoint.close()` explicitly.

For Myrhiza, custom transports are the door we'd use to slot in any non-UDP substrate (e.g. an in-process loopback for tests, or a Bluetooth path on mobile). It is not a stable surface and committing specs against the API today is premature; the *concept* — that iroh allows transport pluggability below the QUIC layer — is the load-bearing fact.

## WebTransport / browser viability

There is no WebTransport-backed iroh transport. Browser support, alpha since 0.32 (2025-02-04, [iroh 0.32.0 — Browser alpha, QAD, and n0-future](https://www.iroh.computer/blog/iroh-0-32-0-browser-alpha-qad-and-n0-future)), works by:

- compiling iroh to `wasm32-unknown-unknown` with `wasm-bindgen`,
- speaking to relay servers over **WebSocket** (the iroh-relay protocol is HTTP/HTTPS-upgrade-capable; see [`./nat-traversal.md`](./nat-traversal.md)),
- running in **relay-only mode** — direct connections and hole-punching are disabled because browsers can't open raw UDP.

What this means: a browser-side iroh peer is reachable by `EndpointId`, has the same end-to-end encryption properties, but every byte goes through a relay. There is no path-upgrade story. Several features don't compile to wasm — `metrics`, `test-utils`, `discovery-local-network`, `discovery-pkarr-dht`. `wasm32-wasi` is not supported as of 0.33 ([iroh 0.33.0](https://www.iroh.computer/blog/iroh-0-33-0-browsers-and-discovery-and-0-rtt-oh-my)).

Browser support is real but constrained: it's "a peer that always uses the relay," not "a peer that opens direct paths via WebTransport." If/when a WebTransport-backed transport ships, it would plug in via the custom-transports API above. As of 2026-05-08 it has not.

## Stability and breaking-change cadence

Iroh runs on a roughly monthly minor-version cadence, with most minor releases breaking some API. Looking at the 0.95 → 1.0-rc.0 stretch ([release list](https://github.com/n0-computer/iroh/releases)):

| Release | Date | Headline change | Breaking? |
|---|---|---|---|
| 0.95.0 **(yanked)** / 0.95.1 | 2025-11-05 | Endpoint presets, RelayMap mutability, error-handling overhaul, new relay implementation | Yes |
| 0.96.0 | 2026-01-28 | Multipath, EndpointHooks, qlog, holepunch metrics | Yes |
| 0.97.0 | 2026-03-16 | noq graduation, custom transports, embeddable relay | Yes |
| 0.98.0 | 2026-04-17 | Pluggable crypto backends, NAT-traversal regression fix | Yes |
| 1.0.0-rc.0 | 2026-05-07 | Path observation API redesign, relay auth tokens | Yes |

The roadmap ([iroh.computer/roadmap](https://www.iroh.computer/roadmap)) targets 1.0 in Q1 2026 and the rc dropped on 2026-05-07; the public roadmap does not state a stability commitment for the post-1.0 API. Realistic posture: **expect at least one breaking change per minor release through 1.x**, even after 1.0 ships, until a SemVer policy is published. Pin the iroh version in `Cargo.toml`, do not float against `*` or `^`.

## Implications for Myrhiza

The Myrhiza host import for "open a peer connection" should be defined in terms iroh exposes today and is unlikely to break: `(EndpointId, alpn)` → `Connection`, with `Connection` reduced to "open bidirectional stream / open unidirectional stream / accept stream / paths()-watcher." That's stable across the 0.95–1.0-rc churn and corresponds to standard QUIC concepts that are unlikely to vanish. Anything below that — multipath internals, custom-transport hooks, the noq path-scheduler — is fluid; do not expose it through capabilities. When iroh's API churns we want to absorb the churn at the kernel boundary, not propagate it to every WASM bundle.

## Sources

- [iroh release list](https://github.com/n0-computer/iroh/releases)
- [iroh 0.96.0 — The QUIC Multipaths to 1.0](https://www.iroh.computer/blog/iroh-0-96-0-the-quic-multipaths-to-1-0)
- [iroh 0.97.0 — Custom Transports & noq](https://www.iroh.computer/blog/iroh-0-97-0-custom-transports-and-noq)
- [noq, noq, who's there?](https://www.iroh.computer/blog/noq-announcement)
- [iroh 0.32.0 — Browser alpha, QAD, and n0-future](https://www.iroh.computer/blog/iroh-0-32-0-browser-alpha-qad-and-n0-future)
- [iroh 0.33.0 — Browsers and Discovery and 0-RTT, oh my!](https://www.iroh.computer/blog/iroh-0-33-0-browsers-and-discovery-and-0-rtt-oh-my)
- [iroh roadmap](https://www.iroh.computer/roadmap)
- [Quinn QUIC implementation](https://github.com/quinn-rs/quinn)
- [iroh-quinn on crates.io](https://crates.io/crates/iroh-quinn)
