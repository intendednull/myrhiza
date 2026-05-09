**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — lessons for Myrhiza (validates / avoid / borrow)

# Lessons for Myrhiza

This is the consult-this-when-designing file. The other iroh prior-art files are evidence; this file is decisions.

Iroh is a **load-bearing dependency** Myrhiza will likely commit to as its P2P transport substrate, not a peer system we learn from at arm's length. That changes the lens: "validates" is what iroh's existence + experience supports about our design, "avoid" is what iroh's pre-1.0 churn warns us about, and "borrow" is what we will actually adopt — whether by depending directly or by mirroring API shape.

## Validates

These Myrhiza design choices are confirmed by iroh's experience:

- **Pubkey-as-transport-identity (Ed25519, no DID layer at the transport).** Iroh's `EndpointId` is a 32-byte Ed25519 public key; no certificate authority, no DID resolver, no L1 chain. The TLS cert presented during the QUIC handshake is self-signed by that key. This is what a P2P transport identity should look like. See [`identity.md`](identity.md), [`architecture.md`](architecture.md).
- **ALPN-as-protocol-multiplexing.** Iroh leans on QUIC's standard ALPN for protocol selection rather than inventing a multistream-style negotiation. One endpoint registers N protocol names; the client passes one to `connect`; the TLS handshake refuses on no-match. This composes cleanly with Myrhiza's per-app capability surface — each app gets a kernel-allocated ALPN namespace, the kernel owns the Router. See [`architecture.md`](architecture.md).
- **Single-`Endpoint`-per-process as the API surface.** The kernel wants exactly one transport object per host; iroh's API agrees. Builder-configured at boot, mutable post-bind for relay/ALPN registration, multi-tenant via `Router`. See [`architecture.md`](architecture.md).
- **Relay-with-direct-upgrade as the only realistic NAT story.** Pure peer-to-peer is a fantasy for the NATted internet. Iroh's "open relay path immediately, race direct candidates, upgrade when reachable" is the right shape. See [`nat-traversal.md`](nat-traversal.md).
- **Consensus is impossible at the transport layer.** The team's [Consensus is Impossible](https://www.iroh.computer/blog/consensus-is-impossible) post is a load-bearing fact for Myrhiza's `state-apply` profile: cross-peer convergence is asymptotic, not synchronous. State-apply must be a deterministic function of `(prior state, event)` *because* there is no consensus to lean on. See [`open-problems.md`](open-problems.md).
- **Content-addressing as the bundle-distribution primitive.** BLAKE3 + Bao verified streaming is the right shape for shipping WASM component bundles peer-to-peer: integrity is checked while streaming, partial transfers resume, cache hits are free. See [`blobs.md`](blobs.md).
- **Range-based set reconciliation for state sync.** Iroh-docs (and Willow) both bet on RBSR for converging multi-author state. The math is sound, the perf is good, and it composes with content addressing. See [`docs.md`](docs.md), [`willow.md`](willow.md).
- **Custom transports as a first-class extension point.** iroh 0.97 added a `Transport` trait that lets Tor and Veilid be plugged in alongside QUIC. Mirrors the netlayer pattern Spritely uses. Validates Myrhiza's plan to keep transport pluggable. See [`transports.md`](transports.md).

## Avoid

Pitfalls iroh's pre-1.0 churn reveals — and how Myrhiza compensates.

| Pitfall | Source | Mitigation |
|---|---|---|
| **API churn is constant pre-1.0.** `NodeId → EndpointId` (0.94), `iroh-net` fold-in (0.29, the "Net is the new iroh" rename), Quinn fork → noq graduation (0.97), tickets-moved-to-`iroh-base` (0.95). Every minor is breaking. | [`history.md`](history.md), [`distribution.md`](distribution.md) | Anchor Myrhiza specs against the *concepts* (a peer keypair-identified endpoint, ALPN-mux'd connection) and import iroh's current names as terminology, not as load-bearing API surface. Vendor-pin iroh in `Cargo.toml` and bump deliberately. |
| **No published wire spec.** Iroh is a single-implementation protocol; the relay wire format lives in `iroh-relay/src/protos/relay.rs`. The 1.0 roadmap promises specs but 1.0.0-rc.0 shipped without them. | [`open-problems.md`](open-problems.md), [`tooling.md`](tooling.md) | Myrhiza writes its own wire spec for everything that crosses a peer boundary, even where Myrhiza embeds iroh wholesale. Track when iroh's wire spec ships and re-evaluate single-implementation risk then. |
| **iroh-ffi is unmaintained for production.** README self-declares "reference example only" since Feb 2025; GitHub `archived` flag is not set but the repo has not had functional updates. Production iOS/Android paths are paid (`iroh-c-ffi`) or third-party (`iroh-js` is dormant since Dec 2023). | [`mobile-and-wasm.md`](mobile-and-wasm.md), [`apps.md`](apps.md) | Don't bet Myrhiza's mobile story on iroh-ffi. Plan to either pay for iroh's commercial bindings, write our own UniFFI / wasm-bindgen layer, or use Component Model + jco for the browser path. |
| **No `iroh` CLI; tooling is fragmented.** `sendme`, `dumbpipe`, `iroh-doctor` live in separate repos at separate version trains. The user-facing experience is "which binary do I install today?" | [`tooling.md`](tooling.md) | Ship one Myrhiza CLI as the canonical operator surface; do not assume users will assemble n0's tools. |
| **Relay metadata is privileged.** Relays see *which NodeID is talking to which NodeID, when, how much*. Subpoena, coercion, timing-correlation attacks are all in scope. | [`open-problems.md`](open-problems.md), [`critiques.md`](critiques.md) | Document that the default Myrhiza deployment is **not censorship-resistant**. For state-level threat models, route apps over iroh-on-Tor or iroh-on-Veilid via the custom-transport API. Treat metadata patterns (alice-talks-to-bob-every-10s) as app-layer concerns; consider padding/cover traffic if a Myrhiza app needs metadata privacy. |
| **NodeID = identity is a category error.** A NodeID is a per-device transport key. Lose the device, lose the NodeID. The team has explored FROST split-key recovery but not shipped it. | [`identity.md`](identity.md), [`open-problems.md`](open-problems.md) | Separate **NodeID** (transport key, iroh-managed) from **PrincipalID** (application identity, Myrhiza-managed, multi-device, recoverable). PrincipalID lives at the layer above iroh. |
| **No discovery primitive.** Iroh resolves NodeIDs to addresses (pkarr-on-Mainline-DHT); it does not solve "two strangers find each other" in the social sense. | [`open-problems.md`](open-problems.md) | Decide explicitly whether Myrhiza ships a discovery primitive (DHT, gossip overlay, federated index, tag registry) or delegates to apps. Even Holochain ships bootstrap servers — plan for an analogous bootstrap layer. |
| **Sybil resistance is none, by design.** Anyone can spin up arbitrarily many NodeIDs. Iroh has no membership gate, no proof-of-personhood, no resource cost. | [`open-problems.md`](open-problems.md) | Document explicitly that Myrhiza has no global Sybil floor. Per-app membership proofs (capability-token-gated, social-graph-attested, fee-paying) — the policy lives at the layer above. |
| **Relay infrastructure costs money long-term.** Four n0-operated default relays today, rate-limited for development. Production posture is "pay n0 for dedicated relays or self-host." | [`open-problems.md`](open-problems.md) | Decide explicitly: Myrhiza-operated relay fleet, app-operated, or third-party (Iroh Services / Cloudflare / future relay marketplace). Document cost in the ops spec. Make relay choice swappable per-app to avoid chokepoints. |
| **Performance benchmarks vs alternatives are absent.** No published head-to-head with libp2p, Hypercore, gRPC, NATS. Internal benchmarks exist (QAD, BLAKE3); cross-implementation comparisons do not. | [`open-problems.md`](open-problems.md), [`testing.md`](testing.md) | Bench Myrhiza against gRPC, Cap'n Proto, libp2p ping-pong, Hypercore from MVP. Connection setup time (relay-fronted, hairpin), memory per long-lived `Connection`, throughput per `Endpoint`. Publish numbers. |
| **No determinism claims, anywhere.** Iroh tests run real `tokio` + real wall clocks + real networks. No `madsim`, no shipping loom tests, no fuzz targets. | [`testing.md`](testing.md) | The kernel must enforce determinism above the iroh layer — iroh's API surface (timestamps, connection paths, packet ordering) is non-deterministic by construction. State-apply components see only the deterministic event payload, never the underlying transport state. |
| **iroh-willow is stalled.** Stuck on `iroh = 0.34` while the ecosystem moved to `1.0-rc`. Functionally abandoned, officially not deprecated. | [`willow.md`](willow.md) | Don't bet Myrhiza's state-sync layer on iroh-willow as it stands today. Either contribute to unstall it, embed the Willow protocol directly, or use iroh-docs for now. |
| **n0 is a private company with undisclosed funding.** No Crunchbase, no SEC Form D. Stewardship if n0 fails is unspecified — there is no foundation backstop. | [`governance.md`](governance.md) | Treat the company-as-steward risk as a real consideration. Apache-2.0/MIT means we can fork; the question is whether anyone will if n0 disappears. Track financial signals (hiring, blog cadence, customer announcements) as proxy health metrics. |

## Borrow

Concrete subsystems we'll either depend on directly or mirror in Myrhiza's design.

1. **The `Endpoint` API as the kernel's transport surface.** One `Endpoint` per host, owned by the kernel. Apps get capability handles into `Endpoint::accept(alpn)`, `Endpoint::connect(addr, alpn)`, and stream open/accept — never the raw socket, never the keypair, never the discovery service. The API maps cleanly onto a kernel-mediated capability boundary. See [`architecture.md`](architecture.md).
2. **`Router` + `ProtocolHandler` for ALPN-namespaced multi-tenant dispatch.** The kernel allocates ALPN bytes to apps; each app's component registers a handler; the kernel proxies streams across the WASM boundary. Iroh's `Router` is exactly this shape. See [`architecture.md`](architecture.md).
3. **`EndpointTicket` as the canonical address-sharing format.** Base32-encoded `(EndpointId, Vec<TransportAddr>)`. The right primitive for "paste this in chat to dial me." Use as Myrhiza's external peer-introduction format. See [`architecture.md`](architecture.md).
4. **BLAKE3 + Bao verified streaming for app-bundle distribution.** Content-addressed component bundles with verified streaming and resumable transfer. iroh-blobs gives this for free. See [`blobs.md`](blobs.md).
5. **Custom transports API for pluggable netlayers.** Myrhiza apps with stricter threat models (Tor, Veilid, libp2p, mesh) plug in via iroh's `Transport` trait. The kernel chooses per-app or per-host. See [`transports.md`](transports.md).
6. **DERP-derived relay protocol + self-hostable `iroh-relay`.** When Myrhiza eventually operates its own relay fleet, `iroh-relay` is the binary; the protocol is HTTP/HTTPS+WebSocket-based which composes with standard ops practice (load balancers, TLS termination, geo-DNS). See [`nat-traversal.md`](nat-traversal.md).
7. **QAD (QUIC Address Discovery) instead of STUN.** Iroh replaced STUN with QAD in 0.32; the QUIC NAT-traversal extension is a draft IETF spec but iroh ships it today. Adopt — there is no reason to keep STUN. See [`nat-traversal.md`](nat-traversal.md).
8. **Pkarr-on-Mainline-DHT for endpoint discovery.** The default n0 path uses pkarr (signed DNS records) over the Mainline DHT for `EndpointId → EndpointAddr` resolution. Reusable as a substrate for Myrhiza's own discovery primitive (or as the lowest-layer anchor underneath one). See [`identity.md`](identity.md).
9. **Range-based set reconciliation for replicated state.** RBSR (Meyer 2022) is the algorithm iroh-docs uses and Willow normalizes. Worth direct study even if Myrhiza doesn't adopt iroh-docs as-is. See [`docs.md`](docs.md), [`willow.md`](willow.md).
10. **Network simulation via Linux netns (`patchbay`).** Iroh's integration tests drive real NAT/loss/mobility scenarios via Linux network namespaces. The right shape for Myrhiza's own end-to-end test infra above the iroh layer. See [`testing.md`](testing.md).

## How to use this file

When designing a Myrhiza feature that touches the network or persistence layer:

1. Find the row in **Avoid** that names a pitfall close to your design. Read the linked subsystem file for the full evidence.
2. Find the entry in **Borrow** that names a primitive close to what you're designing. Confirm the current iroh version still exposes that primitive — pre-1.0 churn means a stale spec citation will rot quickly. Use [`distribution.md`](distribution.md) to check.
3. Promote any decision into a Myrhiza spec under `docs/specs/`. This file captures what we learn from prior art, not our own decisions.

When iroh ships a breaking minor (a frequent event pre-1.0), update this file's date in [`README.md`](README.md) and the affected subsystem files. Specifically, treat 1.0.0 final, the wire-spec publication, and the `iroh-willow` unstall as discrete events that warrant a folder refresh.
