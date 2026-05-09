**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — relay infrastructure, hole-punching, and NAT traversal

The pitch for iroh from a deployment standpoint is "your peers can reach each other through residential NATs without you running anything." That's delivered by two cooperating mechanisms: a fleet of **relay servers** that every endpoint stays loosely connected to, and a **hole-punching protocol** that uses those relays as a side channel to coordinate direct UDP paths. See [`./architecture.md`](./architecture.md) for the `Endpoint` API; see [`./transports.md`](./transports.md) for the QUIC layer that carries the NAT-traversal frames.

## Relay servers

The `iroh-relay` crate ([docs.rs/iroh-relay](https://docs.rs/iroh-relay/latest/iroh_relay/)) implements a server "a revised version of the Designated Encrypted Relay for Packets (DERP) protocol written by Tailscale." The relay is **not** a TURN server and not a STUN server — it speaks a custom wire protocol over **HTTP/HTTPS** with a WebSocket upgrade (`tokio-websockets` is the relay's WS dependency).

What a relay does:

1. **Long-lived endpoint registration.** Every iroh endpoint maintains a persistent connection to its "home" relay (chosen by latency from a configured `RelayMap`). The relay knows "endpoint `EndpointId` is currently online and reachable via *this* socket." The endpoint is *not* unwrapping any payload; the inner traffic is end-to-end encrypted by QUIC/TLS.
2. **Encrypted packet forwarding.** When peer A wants to talk to peer B but no direct path is established, A sends QUIC packets to A's home relay addressed to B's `EndpointId`; the relay forwards over the network of relays to B's home relay, which delivers to B. The relay sees ciphertext only.
3. **Hole-punching coordination.** The same relay channel carries small NAT-traversal control frames so A and B can exchange candidate addresses (more below).

The n0 organization runs a default fleet — four public relays at the time of writing (US x2, EU, Asia), referenced via the `presets::N0` builder ([Endpoint docs](https://docs.rs/iroh/latest/iroh/endpoint/struct.Endpoint.html)). Holochain 0.6 piggybacks on this same fleet rather than running its own ([upgrade-holochain-0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)). Self-hosting is a supported path: 0.97 added an embeddable relay server (`iroh-relay` is a library, not just a binary) ([iroh 0.97.0](https://www.iroh.computer/blog/iroh-0-97-0-custom-transports-and-noq)), and 0.98 added relay authentication tokens for permissioned fleets.

Honest framing: the "n0 default relays" are operationally a centralization point. They're not authoritative — they cannot read your traffic, cannot impersonate peers, cannot prevent two peers from connecting if a direct path works — but they *can* be a liveness dependency. If all four relays are unreachable and you've never opened a direct path, two peers cannot find each other.

## Hole-punching mechanism

Iroh's hole-punching is **not** STUN-based. STUN gave way to QUIC Address Discovery (QAD) in 0.32 ([iroh 0.32.0](https://www.iroh.computer/blog/iroh-0-32-0-browser-alpha-qad-and-n0-future)) — instead of running a separate STUN server, an iroh peer learns its public-mapped address by sending a special QUIC frame to the relay and reading what address the relay observed it from. Same effect, one fewer service to run.

Above that, the mechanism for *coordinating* a direct path is QUIC NAT Traversal, drawn from `draft-seemann-quic-nat-traversal` ([IETF datatracker](https://datatracker.ietf.org/doc/draft-seemann-quic-nat-traversal/)). The IETF draft itself is short-lived: `-02` posted 2024-03-03 and **expired 2024-09-05** with no later revision. The status as of 2026-05-08 is "expired Internet-Draft, no IETF stream assignment, no successor draft posted publicly." Iroh ships an implementation of the expired draft's mechanics anyway, with its own evolution layered on top in noq. Combined with iroh's general absence of a published wire-format spec ([open-problems.md §9](./open-problems.md)), this is a real spec-stability hazard for Myrhiza: there is no upstream standard against which to pin our own NAT-traversal interop guarantees.

The mechanics, post 0.96 ([iroh 0.96.0](https://www.iroh.computer/blog/iroh-0-96-0-the-quic-multipaths-to-1-0)):

1. Both peers gather **candidate addresses** — local interface addresses plus the QAD-discovered public-mapped address.
2. The dialer sends `REACH_OUT` frames (renamed from `PUNCH_ME_NOW`) over the relay path, listing its candidate addresses.
3. Each peer sends `PATH_CHALLENGE` frames at every candidate address it received, on a new QUIC path identifier.
4. Whichever path validates (gets a `PATH_RESPONSE`) becomes a usable direct path; multipath logic (see [`./transports.md`](./transports.md)) then prefers it over the relay.

Because the punching frames travel inside the QUIC connection rather than over a side protocol, NAT mapping state stays warm throughout — there's no race between "STUN said my address is X" and "I haven't sent a packet from X recently enough."

## Fallback semantics: when does the relay get used vs direct?

A connection always opens through the relay first — the dialer doesn't wait for hole-punching to succeed before returning a `Connection`. Then the multipath stack races direct-path validation in the background. Three steady states:

- **Direct path validated, relay idle.** Best case. The relay still sees keepalives from each peer to track liveness, but the data plane is direct.
- **Direct path failed, relay is the data plane.** Symmetric NATs that won't be punched, restrictive corporate firewalls, browser peers (see [`./transports.md`](./transports.md) — browsers always relay). Throughput is limited by relay capacity and adds two extra hops.
- **Direct path was working, then network changed.** Pre-0.98: stalled — holepunching wasn't re-triggered, the connection silently degraded to the relay or just hung. Post-0.98 ([iroh 0.98.0 release notes](https://github.com/n0-computer/iroh/releases)): "holepunching after network changes restarts correctly again." This was a real correctness regression introduced with multipath in 0.96 and only fully fixed five months later in 0.98.

There is no automatic refusal-to-use-relay mode. If you want "direct or fail," that's an application-level policy you enforce by inspecting `Connection::paths()` and tearing down the connection if no direct path validates within your deadline.

## Mobile-network reliability

Cellular CGNAT (carrier-grade NAT) is the hostile environment iroh has invested most heavily in. Two things matter:

- **Symmetric NAT mappings.** Many cellular carriers use symmetric NAT, which is the case standard hole-punching does *not* solve — the public port observed at peer A's relay differs from the port that would be used to reach peer B, so PATH_CHALLENGE frames go to the wrong port. Iroh's behavior here is no different from any other UDP-hole-punching stack: connection still works via the relay, direct paths often don't.
- **Network handover (Wi-Fi ↔ cellular).** This is the multipath payoff. Pre-0.96 a handover dropped the connection. 0.96 introduced multipath but with the regression noted above. 0.98 fixed the regression. As of 0.98 the connection survives handover and renegotiates a direct path on the new interface — but treat this as recently-stabilized and budget for tail-of-distribution surprises.

Empirically the hole-punching success rate iroh quotes in its own materials is on the order of high-90s for residential NAT and noticeably lower for cellular CGNAT; there is no published benchmark page I can cite a hard number from as of 2026-05-08.

## QUIC NAT-traversal extension — IETF status

For a Myrhiza spec author committing against this: the underlying draft (`draft-seemann-quic-nat-traversal-02`) is expired. The mechanism iroh uses is real and works in production, but it is **not** a ratified standard. Other implementations are sparse; you should not assume iroh's NAT-traversal mechanism interoperates with any non-iroh QUIC stack today. If standardization restarts and lands a different wire format, noq will follow it, and you'll get one of those "breaking change in a minor release" updates.

## Implications for Myrhiza

### Determinism boundary

P2P transport is non-deterministic by construction — packet timing, path selection, relay-vs-direct, NAT churn. None of this can leak into `state-apply`. The kernel must serve the network capability such that an app's `behavior` or `interaction` component can drive the network, but the events fed to `state-apply` carry no transport metadata — no path, no latency, no relay-vs-direct flag — only the verified payload and the peer `EndpointId`. That's the seam.

### Concrete cap-shape recommendations

For the host import that exposes iroh to a WASM bundle, the surface area is materially smaller than iroh's own API. Concrete recommendations for the kernel network capability:

- **Hide `Connection::paths()` from apps.** The path watcher exposes relay-vs-direct, multipath identifiers, and live RTT — all of which are non-deterministic and useful only to a `behavior` or `interaction` component (and even there, rarely). If an app legitimately needs to know "am I on a direct path?" expose a coarsened query (`is_direct() -> bool`) on a per-`behavior` cap, never as raw watcher state.
- **Don't expose relay-control APIs to apps.** `Endpoint::insert_relay` / `Endpoint::remove_relay` are kernel-policy operations. An app that could choose its own relay could exfiltrate metadata or bypass policy. Keep these on the kernel side, possibly tunable via an admin app holding a privileged cap.
- **Path-change events do not cross the WASM boundary directly.** If a `behavior` component needs reconnect notifications (e.g. to flush a buffer on Wi-Fi-to-cellular handover), serve them as discrete kernel-emitted events with no transport-state payload — `path-changed { peer: EndpointId }` rather than `path-changed { from: Direct(addr), to: Relay(url) }`. The latter leaks non-deterministic state into the host-import surface.
- **Custom-transport selection is per-app or per-peer kernel policy.** A Myrhiza app with a stricter threat model (Tor-only, no relay, etc.) communicates that intent via capability shape — e.g. "this app's network cap forbids relay paths" — not by configuring iroh directly. The kernel translates intent into the iroh `Transport` plug-in choice.
- **Connection establishment is fail-closed if iroh isn't bound.** "Open connection to peer X with ALPN Y" returns an error, never blocks indefinitely, when no `Endpoint` exists or no relay is reachable. Apps must handle this; tests must cover it.

### Operational-trust boundary

The relay is the operationally-trusted-but-not-cryptographically-trusted infrastructure: the kernel decides which relay map to use, apps inherit it. iroh's `Endpoint::insert_relay` / `Endpoint::remove_relay` make that policy live-tunable from the kernel without restarts, which is the right shape.

### Liveness assumptions in specs

A Myrhiza node that depends on the n0 default relays inherits n0's operational reliability. Specs that assume "peer reachability" should be explicit about whether they tolerate "all configured relays unreachable and no warm direct path" — this is a spec question, not a code question. Answer it in writing before shipping. The 0.96-introduced / 0.98-fixed multipath holepunching regression is a useful concrete example: a real correctness bug that lived in production for five months and would have silently degraded any deployment relying on automatic post-handover reconnect during that window.

## Sources

- [iroh-relay on docs.rs](https://docs.rs/iroh-relay/latest/iroh_relay/)
- [iroh 0.32.0 — Browser alpha, QAD, and n0-future](https://www.iroh.computer/blog/iroh-0-32-0-browser-alpha-qad-and-n0-future)
- [iroh 0.96.0 — The QUIC Multipaths to 1.0](https://www.iroh.computer/blog/iroh-0-96-0-the-quic-multipaths-to-1-0)
- [iroh 0.97.0 — Custom Transports & noq](https://www.iroh.computer/blog/iroh-0-97-0-custom-transports-and-noq)
- [iroh release list](https://github.com/n0-computer/iroh/releases)
- [draft-seemann-quic-nat-traversal (IETF)](https://datatracker.ietf.org/doc/draft-seemann-quic-nat-traversal/)
- [Endpoint API on docs.rs](https://docs.rs/iroh/latest/iroh/endpoint/struct.Endpoint.html)
- [Holochain 0.6 upgrade notes](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)
